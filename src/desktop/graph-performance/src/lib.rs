//! Performance optimization systems for HorizonOS graph desktop
//! 
//! This module provides comprehensive performance optimizations including:
//! - Level-of-detail (LOD) system for scalable rendering
//! - Frustum and occlusion culling for visibility optimization
//! - Adaptive quality rendering based on performance metrics
//! - GPU instancing optimization for batch rendering
//! - Memory pooling and cache management
//! - Performance monitoring and metrics

pub mod lod;
pub mod culling;
pub mod adaptive;
pub mod instancing;
pub mod memory;
pub mod metrics;
pub mod cache;

pub use lod::*;
pub use culling::*;
pub use adaptive::*;
pub use instancing::*;
pub use memory::*;
pub use metrics::*;
pub use cache::*;

use horizonos_graph_engine::{GraphEngine, SceneId, Camera};
use std::time::Instant;

/// Main performance manager that coordinates all optimization systems
pub struct PerformanceManager {
    /// Level-of-detail system
    lod_system: LodSystem,
    /// Culling system for visibility optimization
    culling_system: CullingSystem,
    /// Adaptive quality system
    adaptive_system: AdaptiveQualitySystem,
    /// GPU instancing system
    instancing_system: InstancingSystem,
    /// Memory management system
    memory_system: MemoryManager,
    /// Performance metrics collector
    metrics: PerformanceMetrics,
    /// Render cache system
    cache_system: RenderCache,
    /// Last update time
    last_update: Instant,
    /// Performance targets
    targets: PerformanceTargets,
}

/// Performance targets and thresholds
#[derive(Debug, Clone)]
pub struct PerformanceTargets {
    /// Target frame rate (fps)
    pub target_fps: f32,
    /// Maximum acceptable frame time (ms)
    pub max_frame_time: f32,
    /// Memory usage threshold (MB)
    pub memory_threshold: f32,
    /// GPU usage threshold (%)
    pub gpu_threshold: f32,
}

impl PerformanceManager {
    /// Create a new performance manager
    pub fn new() -> Self {
        Self {
            lod_system: LodSystem::new(),
            culling_system: CullingSystem::new(),
            adaptive_system: AdaptiveQualitySystem::new(),
            instancing_system: InstancingSystem::new(),
            memory_system: MemoryManager::new(),
            metrics: PerformanceMetrics::new(),
            cache_system: RenderCache::new(),
            last_update: Instant::now(),
            targets: PerformanceTargets::default(),
        }
    }
    
    /// Update all performance systems
    pub fn update(&mut self, engine: &mut GraphEngine, camera: &Camera) {
        let now = Instant::now();
        let delta_time = now.duration_since(self.last_update);
        self.last_update = now;
        
        // Collect performance metrics
        self.metrics.update(delta_time);
        
        // Update LOD system based on performance
        self.lod_system.update(camera, &self.metrics, &self.targets);
        
        // Update culling system
        self.culling_system.update(camera, engine.scene());
        
        // Update adaptive quality based on performance
        self.adaptive_system.update(&self.metrics, &self.targets);
        
        // Update memory management
        self.memory_system.update(&self.metrics);
        
        // Update cache system
        self.cache_system.update(&self.metrics);
        
        // Log performance warnings if needed
        self.check_performance_warnings();
    }
    
    /// Get visible nodes after culling
    pub fn get_visible_nodes(&mut self, engine: &GraphEngine, camera: &Camera) -> Vec<SceneId> {
        self.culling_system.get_visible_nodes(engine.scene(), camera)
    }
    
    /// Get LOD level for a node
    pub fn get_node_lod(&mut self, node_id: SceneId, camera: &Camera, engine: &GraphEngine) -> LodLevel {
        self.lod_system.get_node_lod(node_id, camera, engine.scene())
    }
    
    /// Get render quality settings
    pub fn get_render_quality(&self) -> &RenderQuality {
        self.adaptive_system.current_quality()
    }
    
    /// Get performance metrics
    pub fn get_metrics(&self) -> &PerformanceMetrics {
        &self.metrics
    }
    
    /// Configure performance targets
    pub fn set_targets(&mut self, targets: PerformanceTargets) {
        self.targets = targets;
    }
    
    /// Check if performance is meeting targets
    pub fn is_performance_good(&self) -> bool {
        self.metrics.current_fps() >= self.targets.target_fps * 0.9 &&
        self.metrics.current_frame_time() <= self.targets.max_frame_time * 1.1
    }
    
    /// Get memory usage information
    pub fn get_memory_info(&self) -> MemoryInfo {
        self.memory_system.get_info()
    }
    
    /// Force garbage collection
    pub fn force_gc(&mut self) {
        self.memory_system.force_cleanup();
        self.cache_system.force_cleanup();
    }
    
    /// Check for performance warnings
    fn check_performance_warnings(&self) {
        if self.metrics.current_fps() < self.targets.target_fps * 0.7 {
            log::warn!("Performance warning: FPS {} below target {}", 
                self.metrics.current_fps(), self.targets.target_fps);
        }
        
        if self.metrics.current_frame_time() > self.targets.max_frame_time * 1.5 {
            log::warn!("Performance warning: Frame time {:.2}ms above target {:.2}ms",
                self.metrics.current_frame_time(), self.targets.max_frame_time);
        }
        
        let memory_mb = self.memory_system.get_info().used_mb;
        if memory_mb > self.targets.memory_threshold {
            log::warn!("Memory warning: Using {:.1}MB above threshold {:.1}MB",
                memory_mb, self.targets.memory_threshold);
        }
    }
}

impl Default for PerformanceTargets {
    fn default() -> Self {
        Self {
            target_fps: 60.0,
            max_frame_time: 16.67, // ~60fps
            memory_threshold: 2048.0, // 2GB
            gpu_threshold: 80.0,
        }
    }
}

impl Default for PerformanceManager {
    fn default() -> Self {
        Self::new()
    }
}