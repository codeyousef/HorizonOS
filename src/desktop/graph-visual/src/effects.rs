//! Visual effects system including particles, glow, and shadows

use std::collections::HashMap;
use std::time::{Duration, Instant};
use nalgebra::{Point3, Vector3};

/// Particle system for visual feedback
pub struct ParticleSystem {
    /// Active particles
    particles: Vec<Particle>,
    /// Particle emitters
    emitters: HashMap<String, ParticleEmitter>,
    /// GPU buffer for particle data
    particle_buffer: Option<wgpu::Buffer>,
    /// Maximum particles
    max_particles: usize,
}

impl ParticleSystem {
    /// Create new particle system
    pub fn new(max_particles: usize) -> Self {
        Self {
            particles: Vec::with_capacity(max_particles),
            emitters: HashMap::new(),
            particle_buffer: None,
            max_particles,
        }
    }
    
    /// Initialize GPU resources
    pub fn init_gpu_resources(&mut self, device: &wgpu::Device) {
        let buffer_size = self.max_particles * std::mem::size_of::<ParticleGPUData>();
        
        self.particle_buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Particle Buffer"),
            size: buffer_size as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
    }
    
    /// Add particle emitter
    pub fn add_emitter(&mut self, name: String, emitter: ParticleEmitter) {
        self.emitters.insert(name, emitter);
    }
    
    /// Emit particles from named emitter
    pub fn emit(&mut self, emitter_name: &str, position: Point3<f32>, count: u32) {
        if let Some(emitter) = self.emitters.get(emitter_name) {
            let particles_to_add = count.min((self.max_particles - self.particles.len()) as u32);
            
            for _ in 0..particles_to_add {
                self.particles.push(emitter.create_particle(position));
            }
        }
    }
    
    /// Update particle system
    pub fn update(&mut self, delta_time: f32) {
        // Update existing particles
        self.particles.retain_mut(|particle| {
            particle.update(delta_time);
            particle.lifetime > 0.0
        });
        
        // Update continuous emitters
        for (name, emitter) in &mut self.emitters {
            if emitter.continuous && emitter.last_emission.elapsed() >= emitter.emission_interval {
                if let Some(pos) = emitter.position {
                    let count = emitter.particles_per_emission;
                    let particles_to_add = count.min((self.max_particles - self.particles.len()) as u32);
                    
                    for _ in 0..particles_to_add {
                        self.particles.push(emitter.create_particle(pos));
                    }
                    
                    emitter.last_emission = Instant::now();
                }
            }
        }
    }
    
    /// Update GPU buffer with particle data
    pub fn update_gpu_buffer(&self, queue: &wgpu::Queue) {
        if let Some(buffer) = &self.particle_buffer {
            let gpu_data: Vec<ParticleGPUData> = self.particles
                .iter()
                .map(|p| p.to_gpu_data())
                .collect();
            
            queue.write_buffer(buffer, 0, bytemuck::cast_slice(&gpu_data));
        }
    }
    
    /// Get particle count
    pub fn particle_count(&self) -> usize {
        self.particles.len()
    }
}

/// Individual particle
#[derive(Debug, Clone)]
struct Particle {
    /// Position
    position: Point3<f32>,
    /// Velocity
    velocity: Vector3<f32>,
    /// Color with alpha
    color: [f32; 4],
    /// Size
    size: f32,
    /// Remaining lifetime in seconds
    lifetime: f32,
    /// Initial lifetime
    initial_lifetime: f32,
}

impl Particle {
    /// Update particle physics
    fn update(&mut self, delta_time: f32) {
        // Update position
        self.position += self.velocity * delta_time;
        
        // Apply gravity
        self.velocity.y -= 9.8 * delta_time * 0.1; // Scaled gravity
        
        // Update lifetime
        self.lifetime -= delta_time;
        
        // Fade out based on lifetime
        let life_ratio = self.lifetime / self.initial_lifetime;
        self.color[3] = life_ratio.max(0.0);
        
        // Shrink size
        self.size *= 0.99;
    }
    
    /// Convert to GPU data
    fn to_gpu_data(&self) -> ParticleGPUData {
        ParticleGPUData {
            position: [self.position.x, self.position.y, self.position.z, 0.0],
            color: self.color,
            size: self.size,
            _padding: [0.0; 3],
        }
    }
}

/// Particle data for GPU
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct ParticleGPUData {
    position: [f32; 4],
    color: [f32; 4],
    size: f32,
    _padding: [f32; 3],
}

/// Particle emitter configuration
#[derive(Debug, Clone)]
pub struct ParticleEmitter {
    /// Emitter position
    position: Option<Point3<f32>>,
    /// Base velocity
    base_velocity: Vector3<f32>,
    /// Velocity randomness
    velocity_variance: f32,
    /// Base color
    base_color: [f32; 4],
    /// Color variance
    color_variance: f32,
    /// Base size
    base_size: f32,
    /// Size variance
    size_variance: f32,
    /// Particle lifetime
    lifetime: f32,
    /// Continuous emission
    continuous: bool,
    /// Particles per emission
    particles_per_emission: u32,
    /// Emission interval
    emission_interval: Duration,
    /// Last emission time
    last_emission: Instant,
}

impl ParticleEmitter {
    /// Create node creation emitter
    pub fn node_creation() -> Self {
        Self {
            position: None,
            base_velocity: Vector3::new(0.0, 2.0, 0.0),
            velocity_variance: 1.0,
            base_color: [0.3, 0.7, 1.0, 1.0],
            color_variance: 0.2,
            base_size: 5.0,
            size_variance: 2.0,
            lifetime: 1.0,
            continuous: false,
            particles_per_emission: 20,
            emission_interval: Duration::from_millis(100),
            last_emission: Instant::now(),
        }
    }
    
    /// Create edge connection emitter
    pub fn edge_connection() -> Self {
        Self {
            position: None,
            base_velocity: Vector3::new(0.0, 0.0, 0.0),
            velocity_variance: 0.5,
            base_color: [0.8, 0.8, 0.3, 1.0],
            color_variance: 0.1,
            base_size: 3.0,
            size_variance: 1.0,
            lifetime: 0.5,
            continuous: false,
            particles_per_emission: 10,
            emission_interval: Duration::from_millis(50),
            last_emission: Instant::now(),
        }
    }
    
    /// Create particle from emitter
    fn create_particle(&self, position: Point3<f32>) -> Particle {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        
        // Randomize velocity
        let velocity = self.base_velocity + Vector3::new(
            rng.gen_range(-self.velocity_variance..self.velocity_variance),
            rng.gen_range(-self.velocity_variance..self.velocity_variance),
            rng.gen_range(-self.velocity_variance..self.velocity_variance),
        );
        
        // Randomize color
        let mut color = self.base_color;
        for i in 0..3 {
            color[i] += rng.gen_range(-self.color_variance..self.color_variance);
            color[i] = color[i].clamp(0.0, 1.0);
        }
        
        // Randomize size
        let size = self.base_size + rng.gen_range(-self.size_variance..self.size_variance);
        
        Particle {
            position,
            velocity,
            color,
            size: size.max(0.1),
            lifetime: self.lifetime,
            initial_lifetime: self.lifetime,
        }
    }
}

/// Glow effect renderer
pub struct GlowEffect {
    /// Glow render targets
    blur_targets: Vec<wgpu::Texture>,
    /// Blur shader pipeline
    blur_pipeline: Option<wgpu::RenderPipeline>,
    /// Combine shader pipeline
    combine_pipeline: Option<wgpu::RenderPipeline>,
}

impl GlowEffect {
    /// Create new glow effect renderer
    pub fn new() -> Self {
        Self {
            blur_targets: Vec::new(),
            blur_pipeline: None,
            combine_pipeline: None,
        }
    }
    
    /// Initialize GPU resources
    pub fn init_gpu_resources(
        &mut self,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) {
        // Create blur targets at different resolutions
        let resolutions = vec![
            (width / 2, height / 2),
            (width / 4, height / 4),
            (width / 8, height / 8),
        ];
        
        self.blur_targets = resolutions
            .into_iter()
            .map(|(w, h)| {
                device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("Glow Blur Target"),
                    size: wgpu::Extent3d {
                        width: w,
                        height: h,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                })
            })
            .collect();
        
        // TODO: Create blur and combine pipelines
    }
    
    /// Apply glow effect
    pub fn apply_glow(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        source_view: &wgpu::TextureView,
        target_view: &wgpu::TextureView,
        intensity: f32,
    ) {
        // TODO: Implement multi-pass blur and combine
        // 1. Extract bright areas
        // 2. Downsample and blur
        // 3. Combine with original
    }
}

/// Shadow renderer
pub struct ShadowRenderer {
    /// Shadow map texture
    shadow_map: Option<wgpu::Texture>,
    /// Shadow render pipeline
    shadow_pipeline: Option<wgpu::RenderPipeline>,
    /// Shadow map size
    shadow_map_size: u32,
}

impl ShadowRenderer {
    /// Create new shadow renderer
    pub fn new(shadow_map_size: u32) -> Self {
        Self {
            shadow_map: None,
            shadow_pipeline: None,
            shadow_map_size,
        }
    }
    
    /// Initialize GPU resources
    pub fn init_gpu_resources(&mut self, device: &wgpu::Device) {
        // Create shadow map texture
        self.shadow_map = Some(device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Shadow Map"),
            size: wgpu::Extent3d {
                width: self.shadow_map_size,
                height: self.shadow_map_size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        }));
        
        // TODO: Create shadow mapping pipeline
    }
    
    /// Render shadow map
    pub fn render_shadow_map(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        light_position: Point3<f32>,
        objects: &[ShadowCaster],
    ) {
        // TODO: Implement shadow map rendering
        // 1. Set up light view/projection matrices
        // 2. Render scene depth from light's perspective
    }
}

/// Object that casts shadows
pub struct ShadowCaster {
    /// World transform
    pub transform: nalgebra::Matrix4<f32>,
    /// Vertex buffer
    pub vertex_buffer: wgpu::Buffer,
    /// Vertex count
    pub vertex_count: u32,
}

/// Soft boundary effect for clusters
pub struct SoftBoundaryEffect {
    /// Distance field texture
    distance_field: Option<wgpu::Texture>,
    /// SDF generation pipeline
    sdf_pipeline: Option<wgpu::ComputePipeline>,
    /// Boundary render pipeline
    boundary_pipeline: Option<wgpu::RenderPipeline>,
}

impl SoftBoundaryEffect {
    /// Create new soft boundary effect
    pub fn new() -> Self {
        Self {
            distance_field: None,
            sdf_pipeline: None,
            boundary_pipeline: None,
        }
    }
    
    /// Generate signed distance field for cluster
    pub fn generate_sdf(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        cluster_points: &[Point3<f32>],
        resolution: u32,
    ) {
        // TODO: Implement SDF generation
        // 1. Upload cluster points to GPU
        // 2. Run compute shader to generate distance field
        // 3. Store in distance field texture
    }
    
    /// Render soft boundary
    pub fn render_boundary(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        boundary_color: [f32; 4],
        softness: f32,
    ) {
        // TODO: Implement soft boundary rendering
        // 1. Sample distance field
        // 2. Apply smoothstep for soft edges
        // 3. Render with transparency
    }
}

/// Depth-based rendering manager
pub struct DepthManager {
    /// Z-index assignments
    z_indices: HashMap<u64, f32>,
    /// Next available Z-index
    next_z: f32,
    /// Z-index step
    z_step: f32,
}

impl DepthManager {
    /// Create new depth manager
    pub fn new() -> Self {
        Self {
            z_indices: HashMap::new(),
            next_z: 0.0,
            z_step: 0.001,
        }
    }
    
    /// Assign Z-index to object
    pub fn assign_z_index(&mut self, object_id: u64) -> f32 {
        if let Some(&z) = self.z_indices.get(&object_id) {
            z
        } else {
            let z = self.next_z;
            self.z_indices.insert(object_id, z);
            self.next_z += self.z_step;
            z
        }
    }
    
    /// Bring object to front
    pub fn bring_to_front(&mut self, object_id: u64) {
        let z = self.next_z;
        self.z_indices.insert(object_id, z);
        self.next_z += self.z_step;
    }
    
    /// Send object to back
    pub fn send_to_back(&mut self, object_id: u64) {
        let min_z = self.z_indices.values().min_by(|a, b| a.partial_cmp(b).unwrap()).copied().unwrap_or(0.0);
        self.z_indices.insert(object_id, min_z - self.z_step);
    }
    
    /// Get Z-index for object
    pub fn get_z_index(&self, object_id: u64) -> Option<f32> {
        self.z_indices.get(&object_id).copied()
    }
    
    /// Clear all Z-indices
    pub fn clear(&mut self) {
        self.z_indices.clear();
        self.next_z = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_particle_system() {
        let mut system = ParticleSystem::new(1000);
        
        // Add emitter
        system.add_emitter("test".to_string(), ParticleEmitter::node_creation());
        
        // Emit particles
        system.emit("test", Point3::new(0.0, 0.0, 0.0), 10);
        assert_eq!(system.particle_count(), 10);
        
        // Update system
        system.update(0.016); // 60 FPS
        assert!(system.particle_count() <= 10);
    }
    
    #[test]
    fn test_depth_manager() {
        let mut depth = DepthManager::new();
        
        // Assign Z-indices
        let z1 = depth.assign_z_index(1);
        let z2 = depth.assign_z_index(2);
        assert!(z2 > z1);
        
        // Bring to front
        depth.bring_to_front(1);
        let z1_new = depth.get_z_index(1).unwrap();
        assert!(z1_new > z2);
        
        // Send to back
        depth.send_to_back(2);
        let z2_new = depth.get_z_index(2).unwrap();
        assert!(z2_new < z1);
    }
}