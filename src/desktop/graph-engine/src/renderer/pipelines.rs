//! Render pipelines for nodes and edges

use crate::{Scene, Camera, GraphEngineError};
use super::primitives::{SphereVertex, NodeInstance, EdgeVertex, generate_sphere};
use super::shaders;
use nalgebra::Matrix4;
use wgpu::{Device, RenderPass, Buffer, BindGroup, RenderPipeline};

/// Camera uniform data for shaders
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
    view_pos: [f32; 3],
    _padding: f32,
}

impl CameraUniform {
    fn new() -> Self {
        Self {
            view_proj: Matrix4::identity().into(),
            view_pos: [0.0; 3],
            _padding: 0.0,
        }
    }
    
    fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = camera.view_projection_matrix().into();
        self.view_pos = [camera.position.x, camera.position.y, camera.position.z];
    }
}

/// Render pipeline for drawing nodes
pub struct NodePipeline {
    render_pipeline: RenderPipeline,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    instance_buffer: Buffer,
    camera_buffer: Buffer,
    camera_bind_group: BindGroup,
    index_count: u32,
    max_instances: usize,
}

impl NodePipeline {
    pub async fn new(device: &Device, surface_format: wgpu::TextureFormat) -> Result<Self, GraphEngineError> {
        // Create shaders
        let vertex_shader = shaders::create_shader_module(device, shaders::NODE_VERTEX_SHADER, "Node Vertex Shader");
        let fragment_shader = shaders::create_shader_module(device, shaders::NODE_FRAGMENT_SHADER, "Node Fragment Shader");
        
        // Generate sphere geometry
        let (vertices, indices) = generate_sphere(16); // Medium quality sphere
        let index_count = indices.len() as u32;
        
        // Create vertex buffer
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Node Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        
        // Create index buffer
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Node Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        
        // Create instance buffer (max 10000 instances)
        let max_instances = 10000;
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Node Instance Buffer"),
            size: (std::mem::size_of::<NodeInstance>() * max_instances) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        // Create camera uniform buffer
        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Camera Buffer"),
            size: std::mem::size_of::<CameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        // Create bind group layout
        let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
            label: Some("Camera Bind Group Layout"),
        });
        
        // Create bind group
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
            ],
            label: Some("Camera Bind Group"),
        });
        
        // Create render pipeline layout
        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Node Render Pipeline Layout"),
            bind_group_layouts: &[&camera_bind_group_layout],
            push_constant_ranges: &[],
        });
        
        // Create render pipeline
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Node Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vertex_shader,
                entry_point: "vs_main",
                buffers: &[SphereVertex::desc(), NodeInstance::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &fragment_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });
        
        Ok(NodePipeline {
            render_pipeline,
            vertex_buffer,
            index_buffer,
            instance_buffer,
            camera_buffer,
            camera_bind_group,
            index_count,
            max_instances,
        })
    }
    
    pub fn render<'a>(
        &'a self,
        render_pass: &mut RenderPass<'a>,
        queue: &wgpu::Queue,
        scene: &Scene,
        camera: &Camera,
    ) -> Result<(), GraphEngineError> {
        self.render_fixed(render_pass, queue, scene, camera)
    }
}

/// Render pipeline for drawing edges
pub struct EdgePipeline {
    render_pipeline: RenderPipeline,
    vertex_buffer: Buffer,
    camera_buffer: Buffer,
    camera_bind_group: BindGroup,
    max_vertices: usize,
}

impl EdgePipeline {
    pub async fn new(device: &Device, surface_format: wgpu::TextureFormat) -> Result<Self, GraphEngineError> {
        // Create shaders
        let vertex_shader = shaders::create_shader_module(device, shaders::EDGE_VERTEX_SHADER, "Edge Vertex Shader");
        let fragment_shader = shaders::create_shader_module(device, shaders::EDGE_FRAGMENT_SHADER, "Edge Fragment Shader");
        
        // Create vertex buffer (max 20000 vertices for 10000 edges)
        let max_vertices = 20000;
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Edge Vertex Buffer"),
            size: (std::mem::size_of::<EdgeVertex>() * max_vertices) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        // Create camera uniform buffer
        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Camera Buffer"),
            size: std::mem::size_of::<CameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        // Create bind group layout
        let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
            label: Some("Camera Bind Group Layout"),
        });
        
        // Create bind group
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
            ],
            label: Some("Camera Bind Group"),
        });
        
        // Create render pipeline layout
        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Edge Render Pipeline Layout"),
            bind_group_layouts: &[&camera_bind_group_layout],
            push_constant_ranges: &[],
        });
        
        // Create render pipeline
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Edge Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vertex_shader,
                entry_point: "vs_main",
                buffers: &[EdgeVertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &fragment_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });
        
        Ok(EdgePipeline {
            render_pipeline,
            vertex_buffer,
            camera_buffer,
            camera_bind_group,
            max_vertices,
        })
    }
    
    pub fn render<'a>(
        &'a self,
        render_pass: &mut RenderPass<'a>,
        queue: &wgpu::Queue,
        scene: &Scene,
        camera: &Camera,
    ) -> Result<(), GraphEngineError> {
        self.render_fixed(render_pass, queue, scene, camera)
    }
}

// Add required trait implementations
use wgpu::util::DeviceExt;

// Fix the write_buffer usage - it should be called on queue, not render_pass
impl NodePipeline {
    pub fn render_fixed<'a>(
        &'a self,
        render_pass: &mut RenderPass<'a>,
        queue: &wgpu::Queue,
        scene: &Scene,
        camera: &Camera,
    ) -> Result<(), GraphEngineError> {
        // Update camera uniform
        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(camera);
        
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[camera_uniform]));
        
        // Collect instance data
        let instances: Vec<NodeInstance> = scene.nodes()
            .filter(|(_, node)| node.visible)
            .take(self.max_instances)
            .map(|(_, node)| NodeInstance {
                position: [node.position.x, node.position.y, node.position.z],
                color: node.color,
                radius: node.radius,
                selected: if node.selected { 1.0 } else { 0.0 },
                _padding: [0.0; 2],
            })
            .collect();
        
        if instances.is_empty() {
            return Ok(());
        }
        
        // Update instance buffer
        queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&instances));
        
        // Set pipeline and render
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..self.index_count, 0, 0..instances.len() as u32);
        
        Ok(())
    }
}

impl EdgePipeline {
    pub fn render_fixed<'a>(
        &'a self,
        render_pass: &mut RenderPass<'a>,
        queue: &wgpu::Queue,
        scene: &Scene,
        camera: &Camera,
    ) -> Result<(), GraphEngineError> {
        // Update camera uniform
        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(camera);
        
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[camera_uniform]));
        
        // Collect edge vertices
        let mut vertices = Vec::new();
        
        for edge in scene.edges().filter(|edge| edge.visible) {
            if let (Some(source_node), Some(target_node)) = 
                (scene.get_node(edge.source), scene.get_node(edge.target)) {
                
                let thickness = match edge.edge_type {
                    crate::EdgeType::Contains => 2.0,
                    crate::EdgeType::DependsOn => 1.5,
                    crate::EdgeType::RelatedTo { similarity } => 1.0 + similarity * 2.0,
                    _ => 1.0,
                };
                
                vertices.push(EdgeVertex {
                    position: [source_node.position.x, source_node.position.y, source_node.position.z],
                    color: edge.color,
                    thickness,
                    _padding: [0.0; 3],
                });
                
                vertices.push(EdgeVertex {
                    position: [target_node.position.x, target_node.position.y, target_node.position.z],
                    color: edge.color,
                    thickness,
                    _padding: [0.0; 3],
                });
            }
        }
        
        if vertices.is_empty() {
            return Ok(());
        }
        
        // Limit to max vertices
        vertices.truncate(self.max_vertices);
        
        // Update vertex buffer
        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices));
        
        // Set pipeline and render
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.draw(0..vertices.len() as u32, 0..1);
        
        Ok(())
    }
}