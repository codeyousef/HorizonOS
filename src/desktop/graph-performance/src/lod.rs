//! Level-of-detail (LOD) system for scalable graph rendering

use horizonos_graph_engine::{SceneId, Scene, Camera};
use crate::{PerformanceMetrics, PerformanceTargets};
use nalgebra::Point3;
use std::collections::HashMap;

/// Level-of-detail system
pub struct LodSystem {
    /// LOD configurations for different distance ranges
    lod_configs: Vec<LodConfig>,
    /// Per-node LOD overrides
    node_overrides: HashMap<SceneId, LodLevel>,
    /// Distance calculation cache
    distance_cache: HashMap<SceneId, f32>,
    /// Adaptive LOD settings
    adaptive_settings: AdaptiveLodSettings,
}

/// LOD configuration for a distance range
#[derive(Debug, Clone)]
pub struct LodConfig {
    /// Maximum distance for this LOD level
    pub max_distance: f32,
    /// LOD level to use
    pub level: LodLevel,
    /// Performance weight (higher = more aggressive optimization)
    pub performance_weight: f32,
}

/// LOD levels for rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LodLevel {
    /// Highest quality - full detail
    High,
    /// Medium quality - reduced detail
    Medium,
    /// Low quality - basic representation
    Low,
    /// Very low quality - simplified icons
    VeryLow,
    /// Culled - not rendered at all
    Culled,
}

/// Adaptive LOD settings that adjust based on performance
#[derive(Debug, Clone)]
pub struct AdaptiveLodSettings {
    /// Enable adaptive LOD adjustment
    pub enabled: bool,
    /// Performance threshold for LOD adjustment
    pub performance_threshold: f32,
    /// Distance multiplier when performance is poor
    pub distance_multiplier: f32,
    /// Minimum LOD level regardless of performance
    pub min_lod_level: LodLevel,
    /// Maximum nodes to render at high quality
    pub max_high_quality_nodes: usize,
}

impl LodSystem {
    /// Create a new LOD system
    pub fn new() -> Self {
        Self {
            lod_configs: Self::default_configs(),
            node_overrides: HashMap::new(),
            distance_cache: HashMap::new(),
            adaptive_settings: AdaptiveLodSettings::default(),
        }
    }
    
    /// Update LOD system
    pub fn update(&mut self, camera: &Camera, metrics: &PerformanceMetrics, targets: &PerformanceTargets) {
        // Clear distance cache
        self.distance_cache.clear();
        
        // Adjust LOD settings based on performance
        if self.adaptive_settings.enabled {
            self.adjust_for_performance(metrics, targets);
        }
    }
    
    /// Get LOD level for a specific node
    pub fn get_node_lod(&mut self, node_id: SceneId, camera: &Camera, scene: &Scene) -> LodLevel {
        // Check for manual override
        if let Some(&override_lod) = self.node_overrides.get(&node_id) {
            return override_lod;
        }
        
        // Calculate distance to camera
        let distance = self.calculate_distance(node_id, camera, scene);
        
        // Determine LOD level based on distance and configuration
        self.determine_lod_level(distance)
    }
    
    /// Calculate distance from node to camera
    fn calculate_distance(&mut self, node_id: SceneId, camera: &Camera, scene: &Scene) -> f32 {
        if let Some(&cached_distance) = self.distance_cache.get(&node_id) {
            return cached_distance;
        }
        
        let distance = if let Some(node_pos) = scene.get_node_position(node_id) {
            let camera_pos = camera.position;
            (node_pos - camera_pos).magnitude()
        } else {
            f32::INFINITY
        };
        
        self.distance_cache.insert(node_id, distance);
        distance
    }
    
    /// Determine LOD level based on distance
    fn determine_lod_level(&self, distance: f32) -> LodLevel {
        for config in &self.lod_configs {
            if distance <= config.max_distance {
                return config.level;
            }
        }
        
        LodLevel::Culled
    }
    
    /// Adjust LOD settings based on current performance
    fn adjust_for_performance(&mut self, metrics: &PerformanceMetrics, targets: &PerformanceTargets) {
        let performance_ratio = metrics.current_fps() / targets.target_fps;
        
        if performance_ratio < self.adaptive_settings.performance_threshold {
            // Performance is poor - reduce quality
            let quality_reduction = 1.0 - performance_ratio / self.adaptive_settings.performance_threshold;
            let distance_reduction = 1.0 - quality_reduction * 0.5;
            
            // Adjust distance thresholds
            for config in &mut self.lod_configs {
                config.max_distance *= distance_reduction;
            }
            
            log::debug!("LOD: Reducing quality due to poor performance (FPS: {:.1})", metrics.current_fps());
        } else if performance_ratio > 1.2 {
            // Performance is good - can increase quality
            let quality_increase = (performance_ratio - 1.0) * 0.1;
            let distance_increase = 1.0 + quality_increase;
            
            // Gradually restore distance thresholds
            for config in &mut self.lod_configs {
                config.max_distance = (config.max_distance * distance_increase).min(config.max_distance * 1.5);
            }
            
            log::debug!("LOD: Increasing quality due to good performance (FPS: {:.1})", metrics.current_fps());
        }
    }
    
    /// Set LOD level override for a specific node
    pub fn set_node_override(&mut self, node_id: SceneId, lod_level: LodLevel) {
        self.node_overrides.insert(node_id, lod_level);
    }
    
    /// Remove LOD override for a node
    pub fn remove_node_override(&mut self, node_id: SceneId) {
        self.node_overrides.remove(&node_id);
    }
    
    /// Clear all LOD overrides
    pub fn clear_overrides(&mut self) {
        self.node_overrides.clear();
    }
    
    /// Get nodes that should be rendered at each LOD level
    pub fn categorize_nodes_by_lod(&mut self, nodes: &[SceneId], camera: &Camera, scene: &Scene) -> HashMap<LodLevel, Vec<SceneId>> {
        let mut categorized = HashMap::new();
        
        for &node_id in nodes {
            let lod_level = self.get_node_lod(node_id, camera, scene);
            categorized.entry(lod_level).or_insert_with(Vec::new).push(node_id);
        }
        
        // Limit high-quality nodes if needed
        if let Some(high_quality_nodes) = categorized.get_mut(&LodLevel::High) {
            if high_quality_nodes.len() > self.adaptive_settings.max_high_quality_nodes {
                // Sort by distance and keep only the closest ones
                high_quality_nodes.sort_by(|&a, &b| {
                    let dist_a = self.distance_cache.get(&a).copied().unwrap_or(f32::INFINITY);
                    let dist_b = self.distance_cache.get(&b).copied().unwrap_or(f32::INFINITY);
                    dist_a.partial_cmp(&dist_b).unwrap()
                });
                
                // Move excess nodes to medium quality
                let excess = high_quality_nodes.split_off(self.adaptive_settings.max_high_quality_nodes);
                categorized.entry(LodLevel::Medium).or_insert_with(Vec::new).extend(excess);
            }
        }
        
        categorized
    }
    
    /// Get render complexity for a LOD level
    pub fn get_render_complexity(&self, lod_level: LodLevel) -> RenderComplexity {
        match lod_level {
            LodLevel::High => RenderComplexity {
                vertex_count_multiplier: 1.0,
                texture_resolution_multiplier: 1.0,
                effect_quality: EffectQuality::High,
                enable_shadows: true,
                enable_reflections: true,
                enable_animations: true,
            },
            LodLevel::Medium => RenderComplexity {
                vertex_count_multiplier: 0.7,
                texture_resolution_multiplier: 0.7,
                effect_quality: EffectQuality::Medium,
                enable_shadows: true,
                enable_reflections: false,
                enable_animations: true,
            },
            LodLevel::Low => RenderComplexity {
                vertex_count_multiplier: 0.4,
                texture_resolution_multiplier: 0.5,
                effect_quality: EffectQuality::Low,
                enable_shadows: false,
                enable_reflections: false,
                enable_animations: false,
            },
            LodLevel::VeryLow => RenderComplexity {
                vertex_count_multiplier: 0.1,
                texture_resolution_multiplier: 0.25,
                effect_quality: EffectQuality::None,
                enable_shadows: false,
                enable_reflections: false,
                enable_animations: false,
            },
            LodLevel::Culled => RenderComplexity {
                vertex_count_multiplier: 0.0,
                texture_resolution_multiplier: 0.0,
                effect_quality: EffectQuality::None,
                enable_shadows: false,
                enable_reflections: false,
                enable_animations: false,
            },
        }
    }
    
    /// Get default LOD configurations
    fn default_configs() -> Vec<LodConfig> {
        vec![
            LodConfig {
                max_distance: 50.0,
                level: LodLevel::High,
                performance_weight: 1.0,
            },
            LodConfig {
                max_distance: 150.0,
                level: LodLevel::Medium,
                performance_weight: 0.7,
            },
            LodConfig {
                max_distance: 300.0,
                level: LodLevel::Low,
                performance_weight: 0.4,
            },
            LodConfig {
                max_distance: 500.0,
                level: LodLevel::VeryLow,
                performance_weight: 0.1,
            },
        ]
    }
    
    /// Configure LOD distances
    pub fn set_lod_distances(&mut self, high: f32, medium: f32, low: f32, very_low: f32) {
        self.lod_configs = vec![
            LodConfig { max_distance: high, level: LodLevel::High, performance_weight: 1.0 },
            LodConfig { max_distance: medium, level: LodLevel::Medium, performance_weight: 0.7 },
            LodConfig { max_distance: low, level: LodLevel::Low, performance_weight: 0.4 },
            LodConfig { max_distance: very_low, level: LodLevel::VeryLow, performance_weight: 0.1 },
        ];
    }
    
    /// Get current LOD settings
    pub fn get_settings(&self) -> &AdaptiveLodSettings {
        &self.adaptive_settings
    }
    
    /// Configure adaptive LOD settings
    pub fn configure_adaptive(&mut self, settings: AdaptiveLodSettings) {
        self.adaptive_settings = settings;
    }
}

/// Render complexity settings for different LOD levels
#[derive(Debug, Clone)]
pub struct RenderComplexity {
    /// Multiplier for vertex count
    pub vertex_count_multiplier: f32,
    /// Multiplier for texture resolution
    pub texture_resolution_multiplier: f32,
    /// Quality level for effects
    pub effect_quality: EffectQuality,
    /// Whether to render shadows
    pub enable_shadows: bool,
    /// Whether to render reflections
    pub enable_reflections: bool,
    /// Whether to render animations
    pub enable_animations: bool,
}

/// Effect quality levels
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EffectQuality {
    None,
    Low,
    Medium,
    High,
}

impl Default for AdaptiveLodSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            performance_threshold: 0.8, // Adjust when FPS drops below 80% of target
            distance_multiplier: 0.7,
            min_lod_level: LodLevel::VeryLow,
            max_high_quality_nodes: 50,
        }
    }
}

impl Default for LodSystem {
    fn default() -> Self {
        Self::new()
    }
}