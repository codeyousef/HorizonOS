//! Performance metrics collection and monitoring

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Performance metrics collector
pub struct PerformanceMetrics {
    /// Frame time history
    frame_times: VecDeque<f32>,
    /// FPS history
    fps_history: VecDeque<f32>,
    /// Frame counter
    frame_count: u64,
    /// Last frame time
    last_frame_time: Instant,
    /// Average calculation window size
    window_size: usize,
    /// Performance statistics
    stats: PerformanceStats,
}

/// Performance statistics
#[derive(Debug, Clone)]
pub struct PerformanceStats {
    /// Current FPS
    pub current_fps: f32,
    /// Average FPS over window
    pub average_fps: f32,
    /// Minimum FPS in window
    pub min_fps: f32,
    /// Maximum FPS in window
    pub max_fps: f32,
    /// Current frame time (ms)
    pub current_frame_time: f32,
    /// Average frame time over window
    pub average_frame_time: f32,
    /// Minimum frame time in window
    pub min_frame_time: f32,
    /// Maximum frame time in window
    pub max_frame_time: f32,
    /// Frame time variance
    pub frame_time_variance: f32,
    /// Total frames rendered
    pub total_frames: u64,
    /// Performance trend
    pub trend: PerformanceTrend,
}

/// Performance trend indicator
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PerformanceTrend {
    /// Performance is improving
    Improving,
    /// Performance is stable
    Stable,
    /// Performance is degrading
    Degrading,
    /// Not enough data to determine trend
    Unknown,
}

/// GPU metrics (placeholder for future implementation)
#[derive(Debug, Clone)]
pub struct GpuMetrics {
    /// GPU utilization percentage
    pub utilization: f32,
    /// GPU memory usage (MB)
    pub memory_used: f32,
    /// GPU memory total (MB)
    pub memory_total: f32,
    /// GPU temperature (Celsius)
    pub temperature: f32,
    /// GPU power usage (Watts)
    pub power_usage: f32,
}

/// Memory metrics
#[derive(Debug, Clone)]
pub struct MemoryMetrics {
    /// System memory used (MB)
    pub system_used: f32,
    /// System memory total (MB)
    pub system_total: f32,
    /// Application memory used (MB)
    pub app_used: f32,
    /// GPU memory used (MB)
    pub gpu_used: f32,
    /// GPU memory total (MB)
    pub gpu_total: f32,
}

/// Rendering statistics
#[derive(Debug, Clone)]
pub struct RenderStats {
    /// Vertices rendered this frame
    pub vertices_rendered: u32,
    /// Triangles rendered this frame
    pub triangles_rendered: u32,
    /// Draw calls this frame
    pub draw_calls: u32,
    /// Nodes rendered this frame
    pub nodes_rendered: u32,
    /// Edges rendered this frame
    pub edges_rendered: u32,
    /// Nodes culled this frame
    pub nodes_culled: u32,
    /// LOD level distribution
    pub lod_distribution: [u32; 5], // [High, Medium, Low, VeryLow, Culled]
}

impl PerformanceMetrics {
    /// Create a new performance metrics collector
    pub fn new() -> Self {
        Self::with_window_size(60) // 1 second at 60fps
    }
    
    /// Create with custom window size
    pub fn with_window_size(window_size: usize) -> Self {
        Self {
            frame_times: VecDeque::with_capacity(window_size),
            fps_history: VecDeque::with_capacity(window_size),
            frame_count: 0,
            last_frame_time: Instant::now(),
            window_size,
            stats: PerformanceStats::default(),
        }
    }
    
    /// Update metrics with frame timing
    pub fn update(&mut self, delta_time: Duration) {
        let now = Instant::now();
        let frame_time_ms = delta_time.as_secs_f32() * 1000.0;
        
        // Calculate FPS
        let fps = if frame_time_ms > 0.0 {
            1000.0 / frame_time_ms
        } else {
            0.0
        };
        
        // Add to history
        self.frame_times.push_back(frame_time_ms);
        self.fps_history.push_back(fps);
        
        // Maintain window size
        if self.frame_times.len() > self.window_size {
            self.frame_times.pop_front();
        }
        if self.fps_history.len() > self.window_size {
            self.fps_history.pop_front();
        }
        
        // Update frame count
        self.frame_count += 1;
        self.last_frame_time = now;
        
        // Calculate statistics
        self.calculate_stats();
    }
    
    /// Calculate performance statistics
    fn calculate_stats(&mut self) {
        if self.frame_times.is_empty() {
            return;
        }
        
        // Current values
        self.stats.current_frame_time = *self.frame_times.back().unwrap();
        self.stats.current_fps = *self.fps_history.back().unwrap();
        
        // Calculate averages
        let frame_time_sum: f32 = self.frame_times.iter().sum();
        let fps_sum: f32 = self.fps_history.iter().sum();
        
        self.stats.average_frame_time = frame_time_sum / self.frame_times.len() as f32;
        self.stats.average_fps = fps_sum / self.fps_history.len() as f32;
        
        // Calculate min/max
        self.stats.min_frame_time = self.frame_times.iter().fold(f32::INFINITY, |a, &b| a.min(b));
        self.stats.max_frame_time = self.frame_times.iter().fold(0.0, |a, &b| a.max(b));
        self.stats.min_fps = self.fps_history.iter().fold(f32::INFINITY, |a, &b| a.min(b));
        self.stats.max_fps = self.fps_history.iter().fold(0.0, |a, &b| a.max(b));
        
        // Calculate variance
        let frame_time_mean = self.stats.average_frame_time;
        let variance_sum: f32 = self.frame_times.iter()
            .map(|&time| (time - frame_time_mean).powi(2))
            .sum();
        self.stats.frame_time_variance = variance_sum / self.frame_times.len() as f32;
        
        // Update total frames
        self.stats.total_frames = self.frame_count;
        
        // Calculate trend
        self.stats.trend = self.calculate_trend();
    }
    
    /// Calculate performance trend
    fn calculate_trend(&self) -> PerformanceTrend {
        if self.fps_history.len() < 10 {
            return PerformanceTrend::Unknown;
        }
        
        // Compare first and last thirds of the window
        let third_size = self.fps_history.len() / 3;
        let first_third: f32 = self.fps_history.iter().take(third_size).sum::<f32>() / third_size as f32;
        let last_third: f32 = self.fps_history.iter().rev().take(third_size).sum::<f32>() / third_size as f32;
        
        let difference = last_third - first_third;
        let threshold = self.stats.average_fps * 0.05; // 5% threshold
        
        if difference > threshold {
            PerformanceTrend::Improving
        } else if difference < -threshold {
            PerformanceTrend::Degrading
        } else {
            PerformanceTrend::Stable
        }
    }
    
    /// Get current FPS
    pub fn current_fps(&self) -> f32 {
        self.stats.current_fps
    }
    
    /// Get current frame time in milliseconds
    pub fn current_frame_time(&self) -> f32 {
        self.stats.current_frame_time
    }
    
    /// Get average FPS
    pub fn average_fps(&self) -> f32 {
        self.stats.average_fps
    }
    
    /// Get average frame time
    pub fn average_frame_time(&self) -> f32 {
        self.stats.average_frame_time
    }
    
    /// Get performance statistics
    pub fn get_stats(&self) -> &PerformanceStats {
        &self.stats
    }
    
    /// Check if performance is stable
    pub fn is_stable(&self) -> bool {
        self.stats.trend == PerformanceTrend::Stable && 
        self.stats.frame_time_variance < 2.0 // Low variance threshold
    }
    
    /// Check if performance is good
    pub fn is_performance_good(&self, target_fps: f32) -> bool {
        self.stats.current_fps >= target_fps * 0.9 &&
        self.stats.frame_time_variance < 5.0
    }
    
    /// Get frame time history for plotting
    pub fn get_frame_time_history(&self) -> Vec<f32> {
        self.frame_times.iter().copied().collect()
    }
    
    /// Get FPS history for plotting
    pub fn get_fps_history(&self) -> Vec<f32> {
        self.fps_history.iter().copied().collect()
    }
    
    /// Reset metrics
    pub fn reset(&mut self) {
        self.frame_times.clear();
        self.fps_history.clear();
        self.frame_count = 0;
        self.last_frame_time = Instant::now();
        self.stats = PerformanceStats::default();
    }
    
    /// Get performance summary as string
    pub fn get_summary(&self) -> String {
        format!(
            "FPS: {:.1} (avg: {:.1}, min: {:.1}, max: {:.1}) | Frame Time: {:.2}ms (avg: {:.2}ms) | Trend: {:?}",
            self.stats.current_fps,
            self.stats.average_fps,
            self.stats.min_fps,
            self.stats.max_fps,
            self.stats.current_frame_time,
            self.stats.average_frame_time,
            self.stats.trend
        )
    }
}

impl Default for PerformanceStats {
    fn default() -> Self {
        Self {
            current_fps: 0.0,
            average_fps: 0.0,
            min_fps: 0.0,
            max_fps: 0.0,
            current_frame_time: 0.0,
            average_frame_time: 0.0,
            min_frame_time: 0.0,
            max_frame_time: 0.0,
            frame_time_variance: 0.0,
            total_frames: 0,
            trend: PerformanceTrend::Unknown,
        }
    }
}

impl Default for GpuMetrics {
    fn default() -> Self {
        Self {
            utilization: 0.0,
            memory_used: 0.0,
            memory_total: 0.0,
            temperature: 0.0,
            power_usage: 0.0,
        }
    }
}

impl Default for MemoryMetrics {
    fn default() -> Self {
        Self {
            system_used: 0.0,
            system_total: 0.0,
            app_used: 0.0,
            gpu_used: 0.0,
            gpu_total: 0.0,
        }
    }
}

impl Default for RenderStats {
    fn default() -> Self {
        Self {
            vertices_rendered: 0,
            triangles_rendered: 0,
            draw_calls: 0,
            nodes_rendered: 0,
            edges_rendered: 0,
            nodes_culled: 0,
            lod_distribution: [0; 5],
        }
    }
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self::new()
    }
}