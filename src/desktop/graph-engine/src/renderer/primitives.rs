//! Primitive geometry generators for nodes and edges

use nalgebra::{Point3, Vector3};

/// Vertex data for sphere geometry
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SphereVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

/// Instance data for node rendering
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct NodeInstance {
    pub position: [f32; 3],
    pub color: [f32; 4],
    pub radius: f32,
    pub selected: f32,
    pub _padding: [f32; 2], // Ensure alignment
}

/// Vertex data for edge rendering
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct EdgeVertex {
    pub position: [f32; 3],
    pub color: [f32; 4],
    pub thickness: f32,
    pub _padding: [f32; 3], // Ensure alignment
}

/// Generate sphere geometry with given subdivisions
pub fn generate_sphere(subdivisions: u32) -> (Vec<SphereVertex>, Vec<u16>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    
    // Generate vertices
    for i in 0..=subdivisions {
        let theta = i as f32 * std::f32::consts::PI / subdivisions as f32;
        
        for j in 0..=subdivisions * 2 {
            let phi = j as f32 * 2.0 * std::f32::consts::PI / (subdivisions * 2) as f32;
            
            let x = theta.sin() * phi.cos();
            let y = theta.cos();
            let z = theta.sin() * phi.sin();
            
            let position = [x, y, z];
            let normal = [x, y, z]; // For unit sphere, normal = position
            let uv = [j as f32 / (subdivisions * 2) as f32, i as f32 / subdivisions as f32];
            
            vertices.push(SphereVertex {
                position,
                normal,
                uv,
            });
        }
    }
    
    // Generate indices
    for i in 0..subdivisions {
        for j in 0..subdivisions * 2 {
            let ring_size = subdivisions * 2 + 1;
            
            let current = i * ring_size + j;
            let next = current + ring_size;
            
            // First triangle
            indices.push(current as u16);
            indices.push(next as u16);
            indices.push((current + 1) as u16);
            
            // Second triangle
            indices.push((current + 1) as u16);
            indices.push(next as u16);
            indices.push((next + 1) as u16);
        }
    }
    
    (vertices, indices)
}

/// Generate a simple cube for low-LOD nodes
pub fn generate_cube() -> (Vec<SphereVertex>, Vec<u16>) {
    let vertices = vec![
        // Front face
        SphereVertex { position: [-0.5, -0.5,  0.5], normal: [0.0, 0.0, 1.0], uv: [0.0, 0.0] },
        SphereVertex { position: [ 0.5, -0.5,  0.5], normal: [0.0, 0.0, 1.0], uv: [1.0, 0.0] },
        SphereVertex { position: [ 0.5,  0.5,  0.5], normal: [0.0, 0.0, 1.0], uv: [1.0, 1.0] },
        SphereVertex { position: [-0.5,  0.5,  0.5], normal: [0.0, 0.0, 1.0], uv: [0.0, 1.0] },
        
        // Back face
        SphereVertex { position: [-0.5, -0.5, -0.5], normal: [0.0, 0.0, -1.0], uv: [1.0, 0.0] },
        SphereVertex { position: [-0.5,  0.5, -0.5], normal: [0.0, 0.0, -1.0], uv: [1.0, 1.0] },
        SphereVertex { position: [ 0.5,  0.5, -0.5], normal: [0.0, 0.0, -1.0], uv: [0.0, 1.0] },
        SphereVertex { position: [ 0.5, -0.5, -0.5], normal: [0.0, 0.0, -1.0], uv: [0.0, 0.0] },
        
        // Left face
        SphereVertex { position: [-0.5,  0.5,  0.5], normal: [-1.0, 0.0, 0.0], uv: [0.0, 0.0] },
        SphereVertex { position: [-0.5,  0.5, -0.5], normal: [-1.0, 0.0, 0.0], uv: [1.0, 0.0] },
        SphereVertex { position: [-0.5, -0.5, -0.5], normal: [-1.0, 0.0, 0.0], uv: [1.0, 1.0] },
        SphereVertex { position: [-0.5, -0.5,  0.5], normal: [-1.0, 0.0, 0.0], uv: [0.0, 1.0] },
        
        // Right face
        SphereVertex { position: [ 0.5,  0.5,  0.5], normal: [1.0, 0.0, 0.0], uv: [1.0, 0.0] },
        SphereVertex { position: [ 0.5, -0.5,  0.5], normal: [1.0, 0.0, 0.0], uv: [0.0, 0.0] },
        SphereVertex { position: [ 0.5, -0.5, -0.5], normal: [1.0, 0.0, 0.0], uv: [0.0, 1.0] },
        SphereVertex { position: [ 0.5,  0.5, -0.5], normal: [1.0, 0.0, 0.0], uv: [1.0, 1.0] },
        
        // Top face
        SphereVertex { position: [-0.5,  0.5, -0.5], normal: [0.0, 1.0, 0.0], uv: [0.0, 1.0] },
        SphereVertex { position: [-0.5,  0.5,  0.5], normal: [0.0, 1.0, 0.0], uv: [0.0, 0.0] },
        SphereVertex { position: [ 0.5,  0.5,  0.5], normal: [0.0, 1.0, 0.0], uv: [1.0, 0.0] },
        SphereVertex { position: [ 0.5,  0.5, -0.5], normal: [0.0, 1.0, 0.0], uv: [1.0, 1.0] },
        
        // Bottom face
        SphereVertex { position: [-0.5, -0.5, -0.5], normal: [0.0, -1.0, 0.0], uv: [1.0, 1.0] },
        SphereVertex { position: [ 0.5, -0.5, -0.5], normal: [0.0, -1.0, 0.0], uv: [0.0, 1.0] },
        SphereVertex { position: [ 0.5, -0.5,  0.5], normal: [0.0, -1.0, 0.0], uv: [0.0, 0.0] },
        SphereVertex { position: [-0.5, -0.5,  0.5], normal: [0.0, -1.0, 0.0], uv: [1.0, 0.0] },
    ];
    
    let indices = vec![
        0,  1,  2,   2,  3,  0,   // front
        4,  5,  6,   6,  7,  4,   // back
        8,  9, 10,  10, 11,  8,   // left
        12, 13, 14,  14, 15, 12,  // right
        16, 17, 18,  18, 19, 16,  // top
        20, 21, 22,  22, 23, 20,  // bottom
    ];
    
    (vertices, indices)
}

/// Generate edge geometry as a line between two points
pub fn generate_edge_line(
    start: Point3<f32>,
    end: Point3<f32>,
    color: [f32; 4],
    thickness: f32,
) -> Vec<EdgeVertex> {
    vec![
        EdgeVertex {
            position: [start.x, start.y, start.z],
            color,
            thickness,
            _padding: [0.0; 3],
        },
        EdgeVertex {
            position: [end.x, end.y, end.z],
            color,
            thickness,
            _padding: [0.0; 3],
        },
    ]
}

/// Generate a thick edge as a cylinder between two points
pub fn generate_edge_cylinder(
    start: Point3<f32>,
    end: Point3<f32>,
    _color: [f32; 4],
    radius: f32,
    segments: u32,
) -> (Vec<SphereVertex>, Vec<u16>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    
    let direction = (end - start).normalize();
    let length = (end - start).magnitude();
    
    // Create perpendicular vectors for the cylinder
    let up = if direction.y.abs() < 0.9 {
        Vector3::new(0.0, 1.0, 0.0)
    } else {
        Vector3::new(1.0, 0.0, 0.0)
    };
    
    let right = direction.cross(&up).normalize();
    let forward = right.cross(&direction).normalize();
    
    // Generate vertices for both ends of the cylinder
    for ring in 0..=1 {
        let z = ring as f32 * length;
        let center = start + direction * z;
        
        for i in 0..segments {
            let angle = i as f32 * 2.0 * std::f32::consts::PI / segments as f32;
            let x = angle.cos() * radius;
            let y = angle.sin() * radius;
            
            let offset = right * x + forward * y;
            let position = center + offset;
            let normal = offset.normalize();
            
            vertices.push(SphereVertex {
                position: [position.x, position.y, position.z],
                normal: [normal.x, normal.y, normal.z],
                uv: [i as f32 / segments as f32, ring as f32],
            });
        }
    }
    
    // Generate indices for the cylinder walls
    for i in 0..segments {
        let next = (i + 1) % segments;
        
        // First triangle
        indices.push(i as u16);
        indices.push((segments + i) as u16);
        indices.push(next as u16);
        
        // Second triangle
        indices.push(next as u16);
        indices.push((segments + i) as u16);
        indices.push((segments + next) as u16);
    }
    
    (vertices, indices)
}

impl SphereVertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<SphereVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 6]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

impl NodeInstance {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<NodeInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 7]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32,
                },
            ],
        }
    }
}

impl EdgeVertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<EdgeVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 7]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32,
                },
            ],
        }
    }
}