//! Adaptive quality rendering system that adjusts based on performance metrics

use crate::{PerformanceMetrics, PerformanceTargets};
use std::time::{Duration, Instant};

/// Adaptive quality rendering system
pub struct AdaptiveQualitySystem {
    /// Current render quality settings
    current_quality: RenderQuality,
    /// Quality adjustment history
    adjustment_history: Vec<QualityAdjustment>,
    /// Settings for adaptive behavior
    settings: AdaptiveSettings,
    /// Last adjustment time
    last_adjustment: Instant,
}

/// Render quality configuration
#[derive(Debug, Clone)]
pub struct RenderQuality {
    /// Overall quality level
    pub level: QualityLevel,
    /// Render resolution scale (0.1 to 1.0)
    pub resolution_scale: f32,
    /// Shadow quality
    pub shadow_quality: ShadowQuality,
    /// Anti-aliasing setting
    pub anti_aliasing: AntiAliasing,
    /// Particle density (0.0 to 1.0)
    pub particle_density: f32,
    /// Animation quality
    pub animation_quality: AnimationQuality,
    /// Post-processing effects
    pub post_processing: PostProcessing,
    /// Texture quality
    pub texture_quality: TextureQuality,
}

/// Overall quality levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QualityLevel {
    /// Ultra high quality
    Ultra,
    /// High quality
    High,
    /// Medium quality
    Medium,
    /// Low quality
    Low,
    /// Performance mode
    Performance,
}

/// Shadow quality settings
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShadowQuality {
    /// No shadows
    Off,
    /// Low quality shadows
    Low,
    /// Medium quality shadows
    Medium,
    /// High quality shadows
    High,
    /// Ultra quality shadows with soft shadows
    Ultra,
}

/// Anti-aliasing settings
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AntiAliasing {
    /// No anti-aliasing
    Off,
    /// Fast Approximate Anti-Aliasing
    FXAA,
    /// Multi-Sample Anti-Aliasing 2x
    MSAA2x,
    /// Multi-Sample Anti-Aliasing 4x
    MSAA4x,
    /// Multi-Sample Anti-Aliasing 8x
    MSAA8x,
    /// Temporal Anti-Aliasing
    TAA,
}

/// Animation quality settings
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationQuality {
    /// No animations
    Off,
    /// Basic animations only
    Basic,
    /// Standard animations
    Standard,
    /// High quality animations
    High,
    /// Ultra smooth animations
    Ultra,
}

/// Post-processing effect settings
#[derive(Debug, Clone)]
pub struct PostProcessing {
    /// Bloom effect intensity
    pub bloom_intensity: f32,
    /// Screen-space ambient occlusion
    pub ssao_enabled: bool,
    /// Screen-space reflections
    pub ssr_enabled: bool,
    /// Motion blur
    pub motion_blur: bool,
    /// Depth of field
    pub depth_of_field: bool,
    /// Color grading
    pub color_grading: bool,
}

/// Texture quality settings
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureQuality {
    /// Very low resolution textures
    VeryLow,
    /// Low resolution textures
    Low,
    /// Medium resolution textures
    Medium,
    /// High resolution textures
    High,
    /// Ultra high resolution textures
    Ultra,
}

/// Quality adjustment record
#[derive(Debug, Clone)]
pub struct QualityAdjustment {
    /// Time when adjustment was made
    pub timestamp: Instant,
    /// Previous quality level
    pub from_quality: QualityLevel,
    /// New quality level
    pub to_quality: QualityLevel,
    /// Reason for adjustment
    pub reason: AdjustmentReason,
    /// Performance metrics at time of adjustment
    pub metrics_snapshot: PerformanceSnapshot,
}

/// Reason for quality adjustment
#[derive(Debug, Clone)]
pub enum AdjustmentReason {
    /// FPS dropped below target
    LowFrameRate { current_fps: f32, target_fps: f32 },
    /// Frame time exceeded threshold
    HighFrameTime { current_ms: f32, target_ms: f32 },
    /// Memory usage too high
    HighMemoryUsage { current_mb: f32, threshold_mb: f32 },
    /// GPU utilization too high
    HighGpuUsage { current_percent: f32, threshold_percent: f32 },
    /// Performance improved, can increase quality
    PerformanceImproved,
    /// Manual adjustment
    Manual,
}

/// Performance snapshot for analysis
#[derive(Debug, Clone)]
pub struct PerformanceSnapshot {
    /// Frame rate at time of snapshot
    pub fps: f32,
    /// Frame time at time of snapshot
    pub frame_time_ms: f32,
    /// Memory usage at time of snapshot
    pub memory_mb: f32,
    /// GPU usage at time of snapshot
    pub gpu_usage_percent: f32,
}

/// Settings for adaptive quality behavior
#[derive(Debug, Clone)]
pub struct AdaptiveSettings {
    /// Enable adaptive quality adjustment
    pub enabled: bool,
    /// Minimum time between adjustments
    pub adjustment_cooldown: Duration,
    /// How aggressively to adjust quality (0.0 to 1.0)
    pub aggressiveness: f32,
    /// Hysteresis factor to prevent quality oscillation
    pub hysteresis_factor: f32,
    /// Minimum quality level (won't go below this)
    pub min_quality_level: QualityLevel,
    /// Maximum quality level (won't go above this)
    pub max_quality_level: QualityLevel,
}

impl AdaptiveQualitySystem {
    /// Create a new adaptive quality system
    pub fn new() -> Self {
        Self {
            current_quality: RenderQuality::default(),
            adjustment_history: Vec::new(),
            settings: AdaptiveSettings::default(),
            last_adjustment: Instant::now(),
        }
    }
    
    /// Update adaptive quality based on performance metrics
    pub fn update(&mut self, metrics: &PerformanceMetrics, targets: &PerformanceTargets) {
        if !self.settings.enabled {
            return;
        }
        
        // Check if enough time has passed since last adjustment
        let now = Instant::now();
        if now.duration_since(self.last_adjustment) < self.settings.adjustment_cooldown {
            return;
        }
        
        // Analyze current performance
        let performance_ratio = self.analyze_performance(metrics, targets);
        
        // Determine if quality adjustment is needed
        if let Some(new_quality_level) = self.determine_quality_adjustment(performance_ratio, metrics, targets) {
            self.adjust_quality(new_quality_level, metrics, targets);
            self.last_adjustment = now;
        }
    }
    
    /// Analyze current performance and return ratio (1.0 = meeting targets)
    fn analyze_performance(&self, metrics: &PerformanceMetrics, targets: &PerformanceTargets) -> f32 {
        let fps_ratio = metrics.current_fps() / targets.target_fps;
        let frame_time_ratio = targets.max_frame_time / metrics.current_frame_time();
        
        // Use the worse of the two ratios
        fps_ratio.min(frame_time_ratio)
    }
    
    /// Determine if quality should be adjusted
    fn determine_quality_adjustment(&self, performance_ratio: f32, metrics: &PerformanceMetrics, targets: &PerformanceTargets) -> Option<QualityLevel> {
        let current_level = self.current_quality.level;
        
        // Apply hysteresis to prevent oscillation
        let lower_threshold = 0.85 - (self.settings.hysteresis_factor * 0.1);
        let upper_threshold = 1.15 + (self.settings.hysteresis_factor * 0.1);
        
        if performance_ratio < lower_threshold {
            // Performance is poor, decrease quality
            self.get_lower_quality_level(current_level)
        } else if performance_ratio > upper_threshold {
            // Performance is good, try to increase quality
            self.get_higher_quality_level(current_level)
        } else {
            // Performance is acceptable, no change needed
            None
        }
    }
    
    /// Get the next lower quality level
    fn get_lower_quality_level(&self, current: QualityLevel) -> Option<QualityLevel> {
        let new_level = match current {
            QualityLevel::Ultra => QualityLevel::High,
            QualityLevel::High => QualityLevel::Medium,
            QualityLevel::Medium => QualityLevel::Low,
            QualityLevel::Low => QualityLevel::Performance,
            QualityLevel::Performance => return None, // Already at lowest
        };
        
        if new_level as u8 >= self.settings.min_quality_level as u8 {
            Some(new_level)
        } else {
            None
        }
    }
    
    /// Get the next higher quality level
    fn get_higher_quality_level(&self, current: QualityLevel) -> Option<QualityLevel> {
        let new_level = match current {
            QualityLevel::Performance => QualityLevel::Low,
            QualityLevel::Low => QualityLevel::Medium,
            QualityLevel::Medium => QualityLevel::High,
            QualityLevel::High => QualityLevel::Ultra,
            QualityLevel::Ultra => return None, // Already at highest
        };
        
        if new_level as u8 <= self.settings.max_quality_level as u8 {
            Some(new_level)
        } else {
            None
        }
    }
    
    /// Adjust quality to new level
    fn adjust_quality(&mut self, new_level: QualityLevel, metrics: &PerformanceMetrics, targets: &PerformanceTargets) {
        let old_level = self.current_quality.level;
        
        // Apply quality settings for new level
        self.apply_quality_preset(new_level);
        
        // Determine reason for adjustment
        let reason = self.determine_adjustment_reason(metrics, targets);
        
        // Record adjustment
        let adjustment = QualityAdjustment {
            timestamp: Instant::now(),
            from_quality: old_level,
            to_quality: new_level,
            reason,
            metrics_snapshot: PerformanceSnapshot {
                fps: metrics.current_fps(),
                frame_time_ms: metrics.current_frame_time(),
                memory_mb: 0.0, // Would be filled from memory metrics
                gpu_usage_percent: 0.0, // Would be filled from GPU metrics
            },
        };
        
        self.adjustment_history.push(adjustment);
        
        // Limit history size
        if self.adjustment_history.len() > 100 {
            self.adjustment_history.remove(0);
        }
        
        log::info!("Quality adjusted from {:?} to {:?}", old_level, new_level);
    }
    
    /// Apply quality preset for a given level
    fn apply_quality_preset(&mut self, level: QualityLevel) {
        self.current_quality = match level {
            QualityLevel::Ultra => RenderQuality {
                level,
                resolution_scale: 1.0,
                shadow_quality: ShadowQuality::Ultra,
                anti_aliasing: AntiAliasing::TAA,
                particle_density: 1.0,
                animation_quality: AnimationQuality::Ultra,
                post_processing: PostProcessing {
                    bloom_intensity: 1.0,
                    ssao_enabled: true,
                    ssr_enabled: true,
                    motion_blur: true,
                    depth_of_field: true,
                    color_grading: true,
                },
                texture_quality: TextureQuality::Ultra,
            },
            QualityLevel::High => RenderQuality {
                level,
                resolution_scale: 1.0,
                shadow_quality: ShadowQuality::High,
                anti_aliasing: AntiAliasing::MSAA4x,
                particle_density: 0.8,
                animation_quality: AnimationQuality::High,
                post_processing: PostProcessing {
                    bloom_intensity: 0.8,
                    ssao_enabled: true,
                    ssr_enabled: false,
                    motion_blur: true,
                    depth_of_field: false,
                    color_grading: true,
                },
                texture_quality: TextureQuality::High,
            },
            QualityLevel::Medium => RenderQuality {
                level,
                resolution_scale: 0.85,
                shadow_quality: ShadowQuality::Medium,
                anti_aliasing: AntiAliasing::MSAA2x,
                particle_density: 0.6,
                animation_quality: AnimationQuality::Standard,
                post_processing: PostProcessing {
                    bloom_intensity: 0.5,
                    ssao_enabled: true,
                    ssr_enabled: false,
                    motion_blur: false,
                    depth_of_field: false,
                    color_grading: false,
                },
                texture_quality: TextureQuality::Medium,
            },
            QualityLevel::Low => RenderQuality {
                level,
                resolution_scale: 0.7,
                shadow_quality: ShadowQuality::Low,
                anti_aliasing: AntiAliasing::FXAA,
                particle_density: 0.4,
                animation_quality: AnimationQuality::Basic,
                post_processing: PostProcessing {
                    bloom_intensity: 0.0,
                    ssao_enabled: false,
                    ssr_enabled: false,
                    motion_blur: false,
                    depth_of_field: false,
                    color_grading: false,
                },
                texture_quality: TextureQuality::Low,
            },
            QualityLevel::Performance => RenderQuality {
                level,
                resolution_scale: 0.5,
                shadow_quality: ShadowQuality::Off,
                anti_aliasing: AntiAliasing::Off,
                particle_density: 0.1,
                animation_quality: AnimationQuality::Off,
                post_processing: PostProcessing {
                    bloom_intensity: 0.0,
                    ssao_enabled: false,
                    ssr_enabled: false,
                    motion_blur: false,
                    depth_of_field: false,
                    color_grading: false,
                },
                texture_quality: TextureQuality::VeryLow,
            },
        };
    }
    
    /// Determine reason for quality adjustment
    fn determine_adjustment_reason(&self, metrics: &PerformanceMetrics, targets: &PerformanceTargets) -> AdjustmentReason {
        if metrics.current_fps() < targets.target_fps * 0.9 {
            AdjustmentReason::LowFrameRate {
                current_fps: metrics.current_fps(),
                target_fps: targets.target_fps,
            }
        } else if metrics.current_frame_time() > targets.max_frame_time * 1.1 {
            AdjustmentReason::HighFrameTime {
                current_ms: metrics.current_frame_time(),
                target_ms: targets.max_frame_time,
            }
        } else {
            AdjustmentReason::PerformanceImproved
        }
    }
    
    /// Get current quality settings
    pub fn current_quality(&self) -> &RenderQuality {
        &self.current_quality
    }
    
    /// Set quality level manually
    pub fn set_quality_level(&mut self, level: QualityLevel) {
        self.apply_quality_preset(level);
        
        let adjustment = QualityAdjustment {
            timestamp: Instant::now(),
            from_quality: self.current_quality.level,
            to_quality: level,
            reason: AdjustmentReason::Manual,
            metrics_snapshot: PerformanceSnapshot {
                fps: 0.0,
                frame_time_ms: 0.0,
                memory_mb: 0.0,
                gpu_usage_percent: 0.0,
            },
        };
        
        self.adjustment_history.push(adjustment);
        self.last_adjustment = Instant::now();
    }
    
    /// Get adjustment history
    pub fn get_adjustment_history(&self) -> &[QualityAdjustment] {
        &self.adjustment_history
    }
    
    /// Configure adaptive settings
    pub fn configure(&mut self, settings: AdaptiveSettings) {
        self.settings = settings;
    }
    
    /// Get current adaptive settings
    pub fn get_settings(&self) -> &AdaptiveSettings {
        &self.settings
    }
}

impl Default for RenderQuality {
    fn default() -> Self {
        Self {
            level: QualityLevel::High,
            resolution_scale: 1.0,
            shadow_quality: ShadowQuality::High,
            anti_aliasing: AntiAliasing::MSAA4x,
            particle_density: 0.8,
            animation_quality: AnimationQuality::High,
            post_processing: PostProcessing::default(),
            texture_quality: TextureQuality::High,
        }
    }
}

impl Default for PostProcessing {
    fn default() -> Self {
        Self {
            bloom_intensity: 0.5,
            ssao_enabled: true,
            ssr_enabled: false,
            motion_blur: false,
            depth_of_field: false,
            color_grading: true,
        }
    }
}

impl Default for AdaptiveSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            adjustment_cooldown: Duration::from_secs(3),
            aggressiveness: 0.5,
            hysteresis_factor: 0.2,
            min_quality_level: QualityLevel::Performance,
            max_quality_level: QualityLevel::Ultra,
        }
    }
}

impl Default for AdaptiveQualitySystem {
    fn default() -> Self {
        Self::new()
    }
}