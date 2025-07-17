//! Shader programs for rendering nodes and edges

/// Vertex shader for rendering spherical nodes
pub const NODE_VERTEX_SHADER: &str = r#"
struct CameraUniform {
    view_proj: mat4x4<f32>,
    view_pos: vec3<f32>,
};

struct InstanceInput {
    @location(5) position: vec3<f32>,
    @location(6) color: vec4<f32>,
    @location(7) radius: f32,
    @location(8) selected: f32,
};

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
    @location(4) selected: f32,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@vertex
fn vs_main(
    vertex: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;
    
    // Scale and translate vertex position
    let world_position = vertex.position * instance.radius + instance.position;
    
    out.clip_position = camera.view_proj * vec4<f32>(world_position, 1.0);
    out.world_position = world_position;
    out.world_normal = vertex.normal;
    out.uv = vertex.uv;
    out.color = instance.color;
    out.selected = instance.selected;
    
    return out;
}
"#;

/// Fragment shader for rendering spherical nodes
pub const NODE_FRAGMENT_SHADER: &str = r#"
struct CameraUniform {
    view_proj: mat4x4<f32>,
    view_pos: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
    @location(4) selected: f32,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Normalize the normal vector
    let normal = normalize(in.world_normal);
    
    // Simple directional lighting
    let light_dir = normalize(vec3<f32>(1.0, 1.0, 1.0));
    let light_intensity = max(dot(normal, light_dir), 0.1);
    
    // View direction for specular highlighting
    let view_dir = normalize(camera.view_pos - in.world_position);
    let reflect_dir = reflect(-light_dir, normal);
    let specular = pow(max(dot(view_dir, reflect_dir), 0.0), 32.0) * 0.5;
    
    // Base color with lighting
    var final_color = in.color.rgb * light_intensity + vec3<f32>(specular);
    
    // Selection highlight
    if in.selected > 0.5 {
        final_color = mix(final_color, vec3<f32>(1.0, 0.8, 0.2), 0.3);
    }
    
    return vec4<f32>(final_color, in.color.a);
}
"#;

/// Vertex shader for rendering edges as lines
pub const EDGE_VERTEX_SHADER: &str = r#"
struct CameraUniform {
    view_proj: mat4x4<f32>,
    view_pos: vec3<f32>,
};

struct EdgeVertex {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) thickness: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) thickness: f32,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@vertex
fn vs_main(vertex: EdgeVertex) -> VertexOutput {
    var out: VertexOutput;
    
    out.clip_position = camera.view_proj * vec4<f32>(vertex.position, 1.0);
    out.color = vertex.color;
    out.thickness = vertex.thickness;
    
    return out;
}
"#;

/// Fragment shader for rendering edges
pub const EDGE_FRAGMENT_SHADER: &str = r#"
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) thickness: f32,
};

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Simple edge rendering with thickness-based alpha
    let alpha = in.color.a * (in.thickness * 0.5 + 0.5);
    return vec4<f32>(in.color.rgb, alpha);
}
"#;

/// Compute shader for physics simulation
pub const PHYSICS_COMPUTE_SHADER: &str = r#"
struct PhysicsNode {
    position: vec3<f32>,
    velocity: vec3<f32>,
    force: vec3<f32>,
    mass: f32,
    radius: f32,
    fixed: f32,
};

struct PhysicsSettings {
    damping: f32,
    time_step: f32,
    max_velocity: f32,
    node_count: u32,
};

@group(0) @binding(0)
var<storage, read_write> nodes: array<PhysicsNode>;

@group(0) @binding(1)
var<uniform> settings: PhysicsSettings;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    
    if index >= settings.node_count {
        return;
    }
    
    var node = nodes[index];
    
    // Skip fixed nodes
    if node.fixed > 0.5 {
        return;
    }
    
    // Integrate forces
    let acceleration = node.force / node.mass;
    node.velocity = node.velocity + acceleration * settings.time_step;
    
    // Apply damping
    node.velocity = node.velocity * (1.0 - settings.damping * settings.time_step);
    
    // Clamp velocity
    let speed = length(node.velocity);
    if speed > settings.max_velocity {
        node.velocity = node.velocity / speed * settings.max_velocity;
    }
    
    // Update position
    node.position = node.position + node.velocity * settings.time_step;
    
    // Clear forces for next frame
    node.force = vec3<f32>(0.0, 0.0, 0.0);
    
    // Write back
    nodes[index] = node;
}
"#;

/// Utility function to create a shader module
pub fn create_shader_module(device: &wgpu::Device, source: &str, label: &str) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    })
}