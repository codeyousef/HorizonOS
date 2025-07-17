//! WebGPU-based renderer for the graph desktop

pub mod shaders;
pub mod primitives;
pub mod pipelines;

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
    
    // Performance monitoring
    frame_count: u64,
    last_frame_time: std::time::Instant,
}

// Re-export pipelines
pub use pipelines::{NodePipeline, EdgePipeline};

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
        
        Ok(Renderer {
            device,
            queue,
            surface_config,
            node_pipeline,
            edge_pipeline,
            depth_texture,
            depth_view,
            frame_count: 0,
            last_frame_time: std::time::Instant::now(),
        })
    }
    
    /// Render a frame
    pub fn render(&mut self, surface: &Surface, scene: &Scene, camera: &Camera) -> Result<(), GraphEngineError> {
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
            
            // Render nodes
            self.node_pipeline.render(&mut render_pass, &self.queue, scene, camera)?;
        }
        
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        
        // Update performance counters
        self.frame_count += 1;
        let now = std::time::Instant::now();
        if now.duration_since(self.last_frame_time).as_secs() >= 1 {
            log::debug!("FPS: {}", self.frame_count);
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
}