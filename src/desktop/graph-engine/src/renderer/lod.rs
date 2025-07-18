//! Level of Detail (LOD) system for graph rendering optimization
//!
//! This system manages different quality levels for rendering nodes and edges
//! based on their distance from the camera and screen-space size.

use crate::{Camera, GraphEngineError};
use super::primitives::{SphereVertex, EdgeVertex, generate_sphere};
use nalgebra::Point3;
use std::collections::HashMap;
use wgpu::{Device, Buffer};
use wgpu::util::DeviceExt;

/// Level of Detail levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LodLevel {
    /// Highest quality - full detail
    High = 0,
    /// Medium quality - reduced detail
    Medium = 1,
    /// Low quality - minimal detail
    Low = 2,
    /// Culled - not rendered
    Culled = 3,
}

/// LOD configuration parameters
#[derive(Debug, Clone)]
pub struct LodConfig {
    /// Distance thresholds for LOD levels
    pub distance_thresholds: [f32; 3],
    /// Screen-space size thresholds (in pixels)
    pub screen_size_thresholds: [f32; 3],
    /// Enable frustum culling
    pub frustum_culling: bool,
    /// Maximum nodes to render per frame
    pub max_nodes_per_frame: usize,
    /// Maximum edges to render per frame
    pub max_edges_per_frame: usize,
    /// Quality degradation for performance
    pub performance_scaling: f32,
}

impl Default for LodConfig {
    fn default() -> Self {
        Self {
            distance_thresholds: [50.0, 200.0, 500.0],
            screen_size_thresholds: [100.0, 50.0, 20.0],
            frustum_culling: true,
            max_nodes_per_frame: 5000,
            max_edges_per_frame: 10000,
            performance_scaling: 1.0,
        }
    }
}

/// LOD-specific geometry data
#[derive(Debug)]
pub struct LodGeometry {
    /// Vertices for this LOD level
    pub vertices: Vec<SphereVertex>,
    /// Indices for this LOD level
    pub indices: Vec<u32>,
    /// Vertex buffer
    pub vertex_buffer: Buffer,
    /// Index buffer
    pub index_buffer: Buffer,
    /// Index count
    pub index_count: u32,
}

/// LOD system manager
pub struct LodManager {
    /// Configuration
    config: LodConfig,
    /// Node geometry for different LOD levels
    node_geometries: HashMap<LodLevel, LodGeometry>,
    /// Edge geometry for different LOD levels
    edge_geometries: HashMap<LodLevel, LodGeometry>,
    /// Visibility cache
    visibility_cache: HashMap<u64, LodLevel>,
    /// Performance tracking
    performance_tracker: PerformanceTracker,
}

/// Performance tracking for adaptive LOD
#[derive(Debug)]
struct PerformanceTracker {
    /// Frame time history (last 60 frames)
    frame_times: Vec<f32>,
    /// Current frame index
    frame_index: usize,
    /// Target frame time (16.67ms for 60fps)
    target_frame_time: f32,
    /// Performance scaling factor
    scaling_factor: f32,
}

impl Default for PerformanceTracker {
    fn default() -> Self {
        Self {
            frame_times: vec![16.67; 60],
            frame_index: 0,
            target_frame_time: 16.67,
            scaling_factor: 1.0,
        }
    }
}

impl LodManager {
    /// Create a new LOD manager
    pub fn new(device: &Device, config: LodConfig) -> Result<Self, GraphEngineError> {
        let mut node_geometries = HashMap::new();
        let mut edge_geometries = HashMap::new();
        
        // Generate node geometry for each LOD level
        for lod in [LodLevel::High, LodLevel::Medium, LodLevel::Low] {
            let subdivisions = match lod {
                LodLevel::High => 32,    // High quality sphere
                LodLevel::Medium => 16,  // Medium quality sphere
                LodLevel::Low => 8,      // Low quality sphere
                LodLevel::Culled => continue,
            };
            
            let (vertices, indices) = generate_sphere(subdivisions);
            let indices: Vec<u32> = indices.into_iter().map(|i| i as u32).collect();
            let index_count = indices.len() as u32;
            
            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("Node LOD {:?} Vertex Buffer", lod)),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
            
            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("Node LOD {:?} Index Buffer", lod)),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            });
            
            node_geometries.insert(lod, LodGeometry {
                vertices,
                indices,
                vertex_buffer,
                index_buffer,
                index_count,
            });
        }
        
        // Generate edge geometry for each LOD level
        for lod in [LodLevel::High, LodLevel::Medium, LodLevel::Low] {
            let segments = match lod {
                LodLevel::High => 32,    // Smooth curves
                LodLevel::Medium => 16,  // Medium curves
                LodLevel::Low => 4,      // Simple lines
                LodLevel::Culled => continue,
            };
            
            let (vertices, indices) = Self::generate_edge_geometry(segments);
            let index_count = indices.len() as u32;
            
            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("Edge LOD {:?} Vertex Buffer", lod)),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
            
            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("Edge LOD {:?} Index Buffer", lod)),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            });
            
            edge_geometries.insert(lod, LodGeometry {
                vertices: vertices.into_iter().map(|v| SphereVertex {
                    position: v.position,
                    normal: [0.0, 0.0, 1.0],
                    uv: [0.0, 0.0],
                }).collect(),
                indices,
                vertex_buffer,
                index_buffer,
                index_count,
            });
        }
        
        Ok(Self {
            config,
            node_geometries,
            edge_geometries,
            visibility_cache: HashMap::new(),
            performance_tracker: PerformanceTracker::default(),
        })
    }
    
    /// Calculate LOD level for a node
    pub fn calculate_node_lod(&self, node_position: Point3<f32>, camera: &Camera) -> LodLevel {
        let distance = (node_position - camera.position).magnitude();
        
        // Check distance thresholds
        let distance_lod = if distance < self.config.distance_thresholds[0] {
            LodLevel::High
        } else if distance < self.config.distance_thresholds[1] {
            LodLevel::Medium
        } else if distance < self.config.distance_thresholds[2] {
            LodLevel::Low
        } else {
            LodLevel::Culled
        };
        
        // Check screen-space size (approximate)
        let screen_size = self.estimate_screen_size(node_position, camera, 50.0); // Assume 50 unit node radius
        let screen_size_lod = if screen_size > self.config.screen_size_thresholds[0] {
            LodLevel::High
        } else if screen_size > self.config.screen_size_thresholds[1] {
            LodLevel::Medium
        } else if screen_size > self.config.screen_size_thresholds[2] {
            LodLevel::Low
        } else {
            LodLevel::Culled
        };
        
        // Apply performance scaling
        let performance_scaled_lod = self.apply_performance_scaling(distance_lod.min(screen_size_lod));
        
        // Apply frustum culling
        if self.config.frustum_culling && !self.is_in_frustum(node_position, camera) {
            return LodLevel::Culled;
        }
        
        performance_scaled_lod
    }
    
    /// Calculate LOD level for an edge
    pub fn calculate_edge_lod(&self, start_pos: Point3<f32>, end_pos: Point3<f32>, camera: &Camera) -> LodLevel {
        // Use midpoint for distance calculation
        let midpoint = Point3::from((start_pos.coords + end_pos.coords) / 2.0);
        let distance = (midpoint - camera.position).magnitude();
        
        // Check distance thresholds (edges use same thresholds as nodes)
        let distance_lod = if distance < self.config.distance_thresholds[0] {
            LodLevel::High
        } else if distance < self.config.distance_thresholds[1] {
            LodLevel::Medium
        } else if distance < self.config.distance_thresholds[2] {
            LodLevel::Low
        } else {
            LodLevel::Culled
        };
        
        // Edge length consideration
        let edge_length = (end_pos - start_pos).magnitude();
        let edge_screen_size = self.estimate_screen_size(midpoint, camera, edge_length);
        
        let screen_size_lod = if edge_screen_size > self.config.screen_size_thresholds[0] {
            LodLevel::High
        } else if edge_screen_size > self.config.screen_size_thresholds[1] {
            LodLevel::Medium
        } else if edge_screen_size > self.config.screen_size_thresholds[2] {
            LodLevel::Low
        } else {
            LodLevel::Culled
        };
        
        let performance_scaled_lod = self.apply_performance_scaling(distance_lod.min(screen_size_lod));
        
        // Apply frustum culling for both endpoints
        if self.config.frustum_culling && 
           !self.is_in_frustum(start_pos, camera) && 
           !self.is_in_frustum(end_pos, camera) {
            return LodLevel::Culled;
        }
        
        performance_scaled_lod
    }
    
    /// Get node geometry for a specific LOD level
    pub fn get_node_geometry(&self, lod: LodLevel) -> Option<&LodGeometry> {
        self.node_geometries.get(&lod)
    }
    
    /// Get edge geometry for a specific LOD level
    pub fn get_edge_geometry(&self, lod: LodLevel) -> Option<&LodGeometry> {
        self.edge_geometries.get(&lod)
    }
    
    /// Update performance tracking
    pub fn update_performance(&mut self, frame_time: f32) {
        self.performance_tracker.frame_times[self.performance_tracker.frame_index] = frame_time;
        self.performance_tracker.frame_index = (self.performance_tracker.frame_index + 1) % 60;
        
        // Calculate average frame time
        let avg_frame_time = self.performance_tracker.frame_times.iter().sum::<f32>() / 60.0;
        
        // Adjust scaling factor based on performance
        if avg_frame_time > self.performance_tracker.target_frame_time * 1.2 {
            // Performance is poor, reduce quality
            self.performance_tracker.scaling_factor = (self.performance_tracker.scaling_factor - 0.05).max(0.1);
        } else if avg_frame_time < self.performance_tracker.target_frame_time * 0.8 {
            // Performance is good, increase quality
            self.performance_tracker.scaling_factor = (self.performance_tracker.scaling_factor + 0.02).min(1.0);
        }
        
        // Update config based on performance
        self.config.performance_scaling = self.performance_tracker.scaling_factor;
    }
    
    /// Generate edge geometry for a given segment count
    fn generate_edge_geometry(segments: usize) -> (Vec<EdgeVertex>, Vec<u32>) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        
        // Generate a simple line with width
        let width = 0.02; // Edge width in world units
        
        for i in 0..=segments {
            let t = i as f32 / segments as f32;
            
            // Create vertices for the edge quad
            vertices.push(EdgeVertex {
                position: [t, -width, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
                thickness: width,
                _padding: [0.0, 0.0, 0.0],
            });
            vertices.push(EdgeVertex {
                position: [t, width, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
                thickness: width,
                _padding: [0.0, 0.0, 0.0],
            });
            
            // Create indices for the quad
            if i < segments {
                let base = (i * 2) as u32;
                indices.extend_from_slice(&[
                    base, base + 1, base + 2,
                    base + 1, base + 3, base + 2,
                ]);
            }
        }
        
        (vertices, indices)
    }
    
    /// Estimate screen-space size of an object
    fn estimate_screen_size(&self, position: Point3<f32>, camera: &Camera, object_radius: f32) -> f32 {
        let distance = (position - camera.position).magnitude();
        let fov = camera.fov;
        
        // Approximate screen-space size calculation
        let screen_size = 2.0 * object_radius * (fov / 2.0).tan() / distance;
        screen_size * 800.0 // Assume 800 pixel viewport for calculation
    }
    
    /// Check if position is within camera frustum
    fn is_in_frustum(&self, position: Point3<f32>, camera: &Camera) -> bool {
        // Simplified frustum culling - use view-projection matrix
        let view_proj = camera.view_projection_matrix();
        let world_pos = nalgebra::Vector4::new(position.x, position.y, position.z, 1.0);
        let clip_pos = view_proj * world_pos;
        
        // Check if within normalized device coordinates
        let w = clip_pos.w;
        clip_pos.x.abs() <= w && clip_pos.y.abs() <= w && clip_pos.z >= 0.0 && clip_pos.z <= w
    }
    
    /// Apply performance scaling to LOD level
    fn apply_performance_scaling(&self, base_lod: LodLevel) -> LodLevel {
        if self.config.performance_scaling < 0.3 {
            // Very poor performance - use lowest quality
            match base_lod {
                LodLevel::High => LodLevel::Low,
                LodLevel::Medium => LodLevel::Low,
                LodLevel::Low => LodLevel::Low,
                LodLevel::Culled => LodLevel::Culled,
            }
        } else if self.config.performance_scaling < 0.7 {
            // Poor performance - reduce quality by one level
            match base_lod {
                LodLevel::High => LodLevel::Medium,
                LodLevel::Medium => LodLevel::Low,
                LodLevel::Low => LodLevel::Low,
                LodLevel::Culled => LodLevel::Culled,
            }
        } else {
            // Good performance - use base LOD
            base_lod
        }
    }
    
    /// Get LOD statistics
    pub fn get_statistics(&self) -> LodStatistics {
        let mut stats = LodStatistics::default();
        
        for (_, level) in &self.visibility_cache {
            match level {
                LodLevel::High => stats.high_count += 1,
                LodLevel::Medium => stats.medium_count += 1,
                LodLevel::Low => stats.low_count += 1,
                LodLevel::Culled => stats.culled_count += 1,
            }
        }
        
        stats.total_objects = self.visibility_cache.len();
        stats.performance_scaling = self.config.performance_scaling;
        stats.average_frame_time = self.performance_tracker.frame_times.iter().sum::<f32>() / 60.0;
        
        stats
    }
    
    /// Update configuration
    pub fn update_config(&mut self, config: LodConfig) {
        self.config = config;
    }
    
    /// Clear visibility cache
    pub fn clear_cache(&mut self) {
        self.visibility_cache.clear();
    }
}

/// LOD statistics for debugging and monitoring
#[derive(Debug, Default)]
pub struct LodStatistics {
    /// Total objects processed
    pub total_objects: usize,
    /// High LOD count
    pub high_count: usize,
    /// Medium LOD count
    pub medium_count: usize,
    /// Low LOD count
    pub low_count: usize,
    /// Culled objects count
    pub culled_count: usize,
    /// Performance scaling factor
    pub performance_scaling: f32,
    /// Average frame time
    pub average_frame_time: f32,
}

impl LodStatistics {
    /// Get LOD distribution as percentages
    pub fn get_distribution(&self) -> (f32, f32, f32, f32) {
        if self.total_objects == 0 {
            return (0.0, 0.0, 0.0, 0.0);
        }
        
        let total = self.total_objects as f32;
        (
            self.high_count as f32 / total * 100.0,
            self.medium_count as f32 / total * 100.0,
            self.low_count as f32 / total * 100.0,
            self.culled_count as f32 / total * 100.0,
        )
    }
}

impl LodLevel {
    /// Get quality multiplier for this LOD level
    pub fn quality_multiplier(&self) -> f32 {
        match self {
            LodLevel::High => 1.0,
            LodLevel::Medium => 0.6,
            LodLevel::Low => 0.3,
            LodLevel::Culled => 0.0,
        }
    }
    
    /// Get minimum value for comparison
    pub fn min(self, other: LodLevel) -> LodLevel {
        match (self as u8).min(other as u8) {
            0 => LodLevel::High,
            1 => LodLevel::Medium,
            2 => LodLevel::Low,
            _ => LodLevel::Culled,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_lod_level_min() {
        assert_eq!(LodLevel::High.min(LodLevel::Medium), LodLevel::High);
        assert_eq!(LodLevel::Medium.min(LodLevel::Low), LodLevel::Medium);
        assert_eq!(LodLevel::Low.min(LodLevel::Culled), LodLevel::Low);
    }
    
    #[test]
    fn test_quality_multiplier() {
        assert_eq!(LodLevel::High.quality_multiplier(), 1.0);
        assert_eq!(LodLevel::Medium.quality_multiplier(), 0.6);
        assert_eq!(LodLevel::Low.quality_multiplier(), 0.3);
        assert_eq!(LodLevel::Culled.quality_multiplier(), 0.0);
    }
    
    #[test]
    fn test_default_config() {
        let config = LodConfig::default();
        assert_eq!(config.distance_thresholds, [50.0, 200.0, 500.0]);
        assert_eq!(config.screen_size_thresholds, [100.0, 50.0, 20.0]);
        assert!(config.frustum_culling);
    }
}