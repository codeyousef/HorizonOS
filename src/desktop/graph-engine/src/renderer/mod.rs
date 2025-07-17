//! WebGPU-based renderer for the graph desktop

pub mod shaders;
pub mod primitives;
pub mod pipelines;
pub mod lod;
pub mod edge_content;

use crate::{Scene, Camera, GraphEngineError};
use std::sync::Arc;
use wgpu::{Device, Queue, Surface, SurfaceConfiguration};
use winit::window::Window;

/// Main renderer for the graph desktop
pub struct Renderer {
    device: Arc<Device>,
    queue: Arc<Queue>,
    surface_config: SurfaceConfiguration,
    
    // Render pipelines
    node_pipeline: pipelines::NodePipeline,
    edge_pipeline: pipelines::EdgePipeline,
    
    // Depth buffer
    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
    
    // LOD system
    lod_manager: lod::LodManager,
    
    // Edge content analysis
    edge_content_analyzer: edge_content::EdgeContentAnalyzer,
    
    // Performance monitoring
    frame_count: u64,
    last_frame_time: std::time::Instant,
}

// Re-export pipelines and LOD
pub use pipelines::{NodePipeline, EdgePipeline};
pub use lod::{LodManager, LodConfig, LodLevel, LodStatistics};

impl Renderer {
    /// Create a new renderer
    pub async fn new(
        device: Arc<Device>,
        queue: Arc<Queue>,
        surface: &Surface<'static>,
        window: &Window,
        adapter: &wgpu::Adapter,
    ) -> Result<Self, GraphEngineError> {
        let window_size = window.inner_size();
        
        // Configure the surface
        let surface_caps = surface.get_capabilities(adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);
            
        let surface_config = SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: window_size.width,
            height: window_size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        
        surface.configure(&device, &surface_config);
        
        // Create depth buffer
        let (depth_texture, depth_view) = Self::create_depth_texture(&device, &surface_config);
        
        // Create render pipelines
        let node_pipeline = pipelines::NodePipeline::new(&device, surface_format).await?;
        let edge_pipeline = pipelines::EdgePipeline::new(&device, surface_format).await?;
        
        // Create LOD manager
        let lod_config = lod::LodConfig::default();
        let lod_manager = lod::LodManager::new(&device, lod_config)?;
        
        // Create edge content analyzer
        let edge_content_analyzer = edge_content::EdgeContentAnalyzer::new(device.clone())?;
        
        Ok(Renderer {
            device,
            queue,
            surface_config,
            node_pipeline,
            edge_pipeline,
            depth_texture,
            depth_view,
            lod_manager,
            edge_content_analyzer,
            frame_count: 0,
            last_frame_time: std::time::Instant::now(),
        })
    }
    
    /// Render a frame with LOD optimization
    pub fn render(&mut self, surface: &Surface, scene: &Scene, camera: &Camera) -> Result<(), GraphEngineError> {
        let frame_start = std::time::Instant::now();
        let output = surface
            .get_current_texture()
            .map_err(|e| GraphEngineError::RenderError(format!("Surface error: {:?}", e)))?;
            
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Graph Render Encoder"),
        });
        
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Graph Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.02,
                            g: 0.02,
                            b: 0.05,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            
            // Render edges first (behind nodes)
            self.edge_pipeline.render(&mut render_pass, &self.queue, scene, camera)?;
            
            // Analyze edge content for semantic relationships
            self.edge_content_analyzer.analyze_edges(scene, camera)?;
            
            // Render nodes
            self.node_pipeline.render(&mut render_pass, &self.queue, scene, camera)?;
        }
        
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        
        // Update performance counters and LOD system
        self.frame_count += 1;
        let now = std::time::Instant::now();
        let frame_time = frame_start.elapsed().as_secs_f32() * 1000.0; // Convert to milliseconds
        
        self.lod_manager.update_performance(frame_time);
        
        if now.duration_since(self.last_frame_time).as_secs() >= 1 {
            let stats = self.lod_manager.get_statistics();
            log::debug!("FPS: {}, LOD Stats: High:{} Med:{} Low:{} Culled:{}, Perf:{:.2}", 
                       self.frame_count, stats.high_count, stats.medium_count, 
                       stats.low_count, stats.culled_count, stats.performance_scaling);
            self.frame_count = 0;
            self.last_frame_time = now;
        }
        
        Ok(())
    }
    
    /// Resize the renderer
    pub fn resize(&mut self, surface: &Surface, new_size: winit::dpi::PhysicalSize<u32>) -> Result<(), GraphEngineError> {
        if new_size.width > 0 && new_size.height > 0 {
            self.surface_config.width = new_size.width;
            self.surface_config.height = new_size.height;
            surface.configure(&self.device, &self.surface_config);
            
            // Recreate depth buffer
            let (depth_texture, depth_view) = Self::create_depth_texture(&self.device, &self.surface_config);
            self.depth_texture = depth_texture;
            self.depth_view = depth_view;
        }
        Ok(())
    }
    
    /// Get current window size
    pub fn window_size(&self) -> (u32, u32) {
        (self.surface_config.width, self.surface_config.height)
    }
    
    /// Create depth texture and view
    fn create_depth_texture(device: &Device, config: &SurfaceConfiguration) -> (wgpu::Texture, wgpu::TextureView) {
        let size = wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        };
        
        let desc = wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        
        let texture = device.create_texture(&desc);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        
        (texture, view)
    }
    
    /// Get LOD statistics
    pub fn get_lod_statistics(&self) -> lod::LodStatistics {
        self.lod_manager.get_statistics()
    }
    
    /// Update LOD configuration
    pub fn update_lod_config(&mut self, config: lod::LodConfig) {
        self.lod_manager.update_config(config);
    }
    
    /// Get LOD level for a node position
    pub fn get_node_lod(&self, node_position: nalgebra::Point3<f32>, camera: &Camera) -> lod::LodLevel {
        self.lod_manager.calculate_node_lod(node_position, camera)
    }
    
    /// Get LOD level for an edge
    pub fn get_edge_lod(&self, start_pos: nalgebra::Point3<f32>, end_pos: nalgebra::Point3<f32>, camera: &Camera) -> lod::LodLevel {
        self.lod_manager.calculate_edge_lod(start_pos, end_pos, camera)
    }
    
    /// Clear LOD cache
    pub fn clear_lod_cache(&mut self) {
        self.lod_manager.clear_cache();
    }
    
    /// Get edge content analysis results
    pub fn get_edge_content_analysis(&self) -> edge_content::EdgeContentAnalysis {
        self.edge_content_analyzer.get_analysis()
    }
    
    /// Get semantic relationship strength for an edge
    pub fn get_edge_semantic_strength(&self, edge_id: crate::SceneId) -> f32 {
        self.edge_content_analyzer.get_semantic_strength(edge_id)
    }
    
    /// Update edge content analysis settings
    pub fn update_edge_content_config(&mut self, config: edge_content::EdgeContentConfig) {
        self.edge_content_analyzer.update_config(config);
    }
}