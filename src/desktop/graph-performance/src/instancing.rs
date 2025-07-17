//! GPU instancing optimization for batch rendering

use horizonos_graph_engine::SceneId;
use crate::lod::LodLevel;
use std::collections::HashMap;
use wgpu::Buffer;
use bytemuck::{Pod, Zeroable};

/// GPU instancing system for efficient batch rendering
pub struct InstancingSystem {
    /// Instance buffers for different object types
    instance_buffers: HashMap<InstanceType, InstanceBuffer>,
    /// Batch configurations
    batch_configs: HashMap<InstanceType, BatchConfig>,
    /// Instance data staging area
    staging_data: HashMap<InstanceType, Vec<InstanceData>>,
    /// Enable instancing optimization
    enabled: bool,
}

/// Types of objects that can be instanced
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InstanceType {
    /// Node instances
    Node(NodeInstanceType),
    /// Edge instances
    Edge(EdgeInstanceType),
    /// Particle instances
    Particle,
    /// UI element instances
    UiElement,
}

/// Node instance types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeInstanceType {
    /// Application nodes
    Application,
    /// File nodes
    File,
    /// Person nodes
    Person,
    /// Task nodes
    Task,
    /// Device nodes
    Device,
    /// AI agent nodes
    AIAgent,
    /// Concept nodes
    Concept,
    /// System nodes
    System,
}

/// Edge instance types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EdgeInstanceType {
    /// Data flow edges
    DataFlow,
    /// Dependency edges
    Dependency,
    /// Relationship edges
    Relationship,
    /// Hierarchy edges
    Hierarchy,
}

/// Instance buffer for a specific object type
pub struct InstanceBuffer {
    /// GPU buffer for instance data
    buffer: Option<Buffer>,
    /// Current buffer capacity
    capacity: u32,
    /// Current instance count
    count: u32,
    /// Buffer usage pattern
    usage_pattern: BufferUsagePattern,
}

/// Instance data for GPU rendering
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct InstanceData {
    /// World transformation matrix (4x4)
    pub transform: [[f32; 4]; 4],
    /// Color (RGBA)
    pub color: [f32; 4],
    /// Scale factor
    pub scale: f32,
    /// LOD level
    pub lod_level: u32,
    /// Instance flags
    pub flags: u32,
    /// Reserved for future use
    pub reserved: u32,
}

/// Batch configuration for instance rendering
#[derive(Debug, Clone)]
pub struct BatchConfig {
    /// Maximum instances per batch
    pub max_instances: u32,
    /// Buffer growth strategy
    pub growth_strategy: BufferGrowthStrategy,
    /// Update frequency
    pub update_frequency: UpdateFrequency,
    /// Enable frustum culling for instances
    pub enable_culling: bool,
    /// Enable LOD for instances
    pub enable_lod: bool,
}

/// Buffer growth strategy
#[derive(Debug, Clone, Copy)]
pub enum BufferGrowthStrategy {
    /// Double buffer size when full
    Double,
    /// Increase by fixed amount
    Fixed(u32),
    /// Increase by percentage
    Percentage(f32),
}

/// Update frequency for instance buffers
#[derive(Debug, Clone, Copy)]
pub enum UpdateFrequency {
    /// Update every frame
    EveryFrame,
    /// Update every N frames
    EveryNFrames(u32),
    /// Update only when data changes
    OnChange,
    /// Update based on distance to camera
    DistanceBased,
}

/// Buffer usage pattern for optimization
#[derive(Debug, Clone, Copy)]
pub enum BufferUsagePattern {
    /// Data changes frequently
    Dynamic,
    /// Data changes occasionally
    Static,
    /// Data never changes after creation
    Immutable,
}

/// Instancing statistics
#[derive(Debug, Default)]
pub struct InstancingStats {
    /// Total instances rendered
    pub total_instances: u32,
    /// Number of draw calls saved
    pub draw_calls_saved: u32,
    /// GPU memory used for instancing (bytes)
    pub gpu_memory_used: u32,
    /// Buffer update count this frame
    pub buffer_updates: u32,
    /// Instances culled
    pub instances_culled: u32,
}

impl InstancingSystem {
    /// Create a new instancing system
    pub fn new() -> Self {
        Self {
            instance_buffers: HashMap::new(),
            batch_configs: Self::default_configs(),
            staging_data: HashMap::new(),
            enabled: true,
        }
    }
    
    /// Update instancing system
    pub fn update(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        if !self.enabled {
            return;
        }
        
        // Update all instance buffers
        let staging_data = std::mem::take(&mut self.staging_data);
        for (instance_type, staging) in &staging_data {
            self.update_instance_buffer(*instance_type, staging, device, queue);
        }
        
        // Clear staging data
        self.staging_data.clear();
    }
    
    /// Add instance data for rendering
    pub fn add_instance(&mut self, instance_type: InstanceType, data: InstanceData) {
        self.staging_data.entry(instance_type).or_insert_with(Vec::new).push(data);
    }
    
    /// Add multiple instances
    pub fn add_instances(&mut self, instance_type: InstanceType, data: Vec<InstanceData>) {
        self.staging_data.entry(instance_type).or_insert_with(Vec::new).extend(data);
    }
    
    /// Update instance buffer for a specific type
    fn update_instance_buffer(
        &mut self,
        instance_type: InstanceType,
        data: &[InstanceData],
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        let buffer = self.instance_buffers.entry(instance_type).or_insert_with(|| {
            InstanceBuffer::new(instance_type, device)
        });
        
        buffer.update(data, device, queue);
    }
    
    /// Get instance buffer for rendering
    pub fn get_instance_buffer(&self, instance_type: InstanceType) -> Option<&Buffer> {
        self.instance_buffers.get(&instance_type)?.buffer.as_ref()
    }
    
    /// Get instance count for a type
    pub fn get_instance_count(&self, instance_type: InstanceType) -> u32 {
        self.instance_buffers.get(&instance_type).map(|b| b.count).unwrap_or(0)
    }
    
    /// Configure batching for an instance type
    pub fn configure_batch(&mut self, instance_type: InstanceType, config: BatchConfig) {
        self.batch_configs.insert(instance_type, config);
    }
    
    /// Create instance data from node information
    pub fn create_node_instance(&self, node_id: SceneId, position: [f32; 3], scale: f32, color: [f32; 4], lod: LodLevel) -> InstanceData {
        // Create transformation matrix
        let transform = [
            [scale, 0.0, 0.0, position[0]],
            [0.0, scale, 0.0, position[1]],
            [0.0, 0.0, scale, position[2]],
            [0.0, 0.0, 0.0, 1.0],
        ];
        
        let lod_level = match lod {
            LodLevel::High => 0,
            LodLevel::Medium => 1,
            LodLevel::Low => 2,
            LodLevel::VeryLow => 3,
            LodLevel::Culled => 4,
        };
        
        InstanceData {
            transform,
            color,
            scale,
            lod_level,
            flags: 0,
            reserved: 0,
        }
    }
    
    /// Create instance data from edge information
    pub fn create_edge_instance(&self, start_pos: [f32; 3], end_pos: [f32; 3], thickness: f32, color: [f32; 4]) -> InstanceData {
        // Calculate edge transformation (simplified)
        let mid_point = [
            (start_pos[0] + end_pos[0]) * 0.5,
            (start_pos[1] + end_pos[1]) * 0.5,
            (start_pos[2] + end_pos[2]) * 0.5,
        ];
        
        let length = ((end_pos[0] - start_pos[0]).powi(2) +
                     (end_pos[1] - start_pos[1]).powi(2) +
                     (end_pos[2] - start_pos[2]).powi(2)).sqrt();
        
        let transform = [
            [thickness, 0.0, 0.0, mid_point[0]],
            [0.0, thickness, 0.0, mid_point[1]],
            [0.0, 0.0, length, mid_point[2]],
            [0.0, 0.0, 0.0, 1.0],
        ];
        
        InstanceData {
            transform,
            color,
            scale: 1.0,
            lod_level: 0,
            flags: 0,
            reserved: 0,
        }
    }
    
    /// Batch instances by type and LOD
    pub fn batch_instances(&self, instances: &[InstanceData]) -> HashMap<LodLevel, Vec<InstanceData>> {
        let mut batches = HashMap::new();
        
        for instance in instances {
            let lod = match instance.lod_level {
                0 => LodLevel::High,
                1 => LodLevel::Medium,
                2 => LodLevel::Low,
                3 => LodLevel::VeryLow,
                _ => LodLevel::Culled,
            };
            
            batches.entry(lod).or_insert_with(Vec::new).push(*instance);
        }
        
        batches
    }
    
    /// Get instancing statistics
    pub fn get_stats(&self) -> InstancingStats {
        let mut stats = InstancingStats::default();
        
        for buffer in self.instance_buffers.values() {
            stats.total_instances += buffer.count;
            stats.gpu_memory_used += buffer.capacity * std::mem::size_of::<InstanceData>() as u32;
        }
        
        // Estimate draw calls saved (rough approximation)
        stats.draw_calls_saved = stats.total_instances.saturating_sub(self.instance_buffers.len() as u32);
        
        stats
    }
    
    /// Enable or disable instancing
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
    
    /// Clear all instance data
    pub fn clear(&mut self) {
        self.staging_data.clear();
        for buffer in self.instance_buffers.values_mut() {
            buffer.count = 0;
        }
    }
    
    /// Get default batch configurations
    fn default_configs() -> HashMap<InstanceType, BatchConfig> {
        let mut configs = HashMap::new();
        
        // Node configurations
        for node_type in [
            NodeInstanceType::Application,
            NodeInstanceType::File,
            NodeInstanceType::Person,
            NodeInstanceType::Task,
            NodeInstanceType::Device,
            NodeInstanceType::AIAgent,
            NodeInstanceType::Concept,
            NodeInstanceType::System,
        ] {
            configs.insert(
                InstanceType::Node(node_type),
                BatchConfig {
                    max_instances: 1000,
                    growth_strategy: BufferGrowthStrategy::Double,
                    update_frequency: UpdateFrequency::EveryFrame,
                    enable_culling: true,
                    enable_lod: true,
                },
            );
        }
        
        // Edge configurations
        for edge_type in [
            EdgeInstanceType::DataFlow,
            EdgeInstanceType::Dependency,
            EdgeInstanceType::Relationship,
            EdgeInstanceType::Hierarchy,
        ] {
            configs.insert(
                InstanceType::Edge(edge_type),
                BatchConfig {
                    max_instances: 2000,
                    growth_strategy: BufferGrowthStrategy::Double,
                    update_frequency: UpdateFrequency::EveryFrame,
                    enable_culling: true,
                    enable_lod: true,
                },
            );
        }
        
        // Particle configuration
        configs.insert(
            InstanceType::Particle,
            BatchConfig {
                max_instances: 10000,
                growth_strategy: BufferGrowthStrategy::Percentage(1.5),
                update_frequency: UpdateFrequency::EveryFrame,
                enable_culling: true,
                enable_lod: false,
            },
        );
        
        // UI element configuration
        configs.insert(
            InstanceType::UiElement,
            BatchConfig {
                max_instances: 500,
                growth_strategy: BufferGrowthStrategy::Fixed(100),
                update_frequency: UpdateFrequency::OnChange,
                enable_culling: false,
                enable_lod: false,
            },
        );
        
        configs
    }
}

impl InstanceBuffer {
    /// Create a new instance buffer
    pub fn new(instance_type: InstanceType, device: &wgpu::Device) -> Self {
        let initial_capacity = match instance_type {
            InstanceType::Node(_) => 100,
            InstanceType::Edge(_) => 200,
            InstanceType::Particle => 1000,
            InstanceType::UiElement => 50,
        };
        
        Self {
            buffer: None,
            capacity: initial_capacity,
            count: 0,
            usage_pattern: BufferUsagePattern::Dynamic,
        }
    }
    
    /// Update buffer with new data
    pub fn update(&mut self, data: &[InstanceData], device: &wgpu::Device, queue: &wgpu::Queue) {
        self.count = data.len() as u32;
        
        // Check if we need to resize buffer
        if self.count > self.capacity || self.buffer.is_none() {
            self.resize_buffer(self.count.max(self.capacity), device);
        }
        
        // Update buffer data
        if let Some(ref buffer) = self.buffer {
            let data_bytes = bytemuck::cast_slice(data);
            queue.write_buffer(buffer, 0, data_bytes);
        }
    }
    
    /// Resize buffer to new capacity
    fn resize_buffer(&mut self, new_capacity: u32, device: &wgpu::Device) {
        self.capacity = new_capacity;
        
        let buffer_size = (new_capacity as u64) * std::mem::size_of::<InstanceData>() as u64;
        
        self.buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Instance Buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
    }
}

impl Default for InstancingSystem {
    fn default() -> Self {
        Self::new()
    }
}