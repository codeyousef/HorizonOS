//! Idle detection for monitoring system
//! 
//! Detects when the user is idle to adjust monitoring behavior
//! and resource usage accordingly.

use crate::AIError;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use parking_lot::RwLock;
use chrono::{DateTime, Utc, Duration};
use tokio::time::{sleep, interval};
use std::time::Duration as StdDuration;
use log::{info, warn, error, debug};
use std::fs;
use std::path::Path;

/// Idle detection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdleConfig {
    /// Enable idle detection
    pub enabled: bool,
    /// Idle threshold in seconds
    pub idle_threshold: u64,
    /// Check interval in seconds
    pub check_interval: u64,
    /// Reduce sampling rate when idle
    pub reduce_sampling_when_idle: bool,
    /// Idle sampling rate multiplier (e.g., 0.1 = 10% of normal rate)
    pub idle_sampling_multiplier: f32,
    /// Detect mouse movement
    pub detect_mouse: bool,
    /// Detect keyboard activity
    pub detect_keyboard: bool,
    /// Detect audio activity
    pub detect_audio: bool,
    /// Detect network activity
    pub detect_network: bool,
    /// Minimum activity threshold for audio detection
    pub audio_threshold: f32,
    /// Minimum activity threshold for network detection (bytes/sec)
    pub network_threshold: u64,
}

impl Default for IdleConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            idle_threshold: 300, // 5 minutes
            check_interval: 10, // Check every 10 seconds
            reduce_sampling_when_idle: true,
            idle_sampling_multiplier: 0.1,
            detect_mouse: true,
            detect_keyboard: true,
            detect_audio: true,
            detect_network: false,
            audio_threshold: 0.01,
            network_threshold: 1024, // 1 KB/s
        }
    }
}

/// Idle detector status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdleStatus {
    /// Whether the user is currently idle
    pub is_idle: bool,
    /// When the user last became idle
    pub idle_since: Option<DateTime<Utc>>,
    /// When the user was last active
    pub last_activity: DateTime<Utc>,
    /// How long the user has been idle (in seconds)
    pub idle_duration: u64,
    /// Activity sources that broke idle state
    pub activity_sources: Vec<ActivitySource>,
    /// Current activity level (0.0 to 1.0)
    pub activity_level: f32,
}

/// Source of activity that can break idle state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActivitySource {
    /// Mouse movement or clicks
    Mouse,
    /// Keyboard input
    Keyboard,
    /// Audio input/output
    Audio,
    /// Network activity
    Network,
    /// System events
    System,
}

/// Activity measurement
#[derive(Debug, Clone)]
struct ActivityMeasurement {
    /// Mouse position change
    mouse_delta: (i32, i32),
    /// Keyboard events count
    keyboard_events: u32,
    /// Audio level (0.0 to 1.0)
    audio_level: f32,
    /// Network bytes transferred
    network_bytes: u64,
    /// Timestamp of measurement
    timestamp: DateTime<Utc>,
}

/// Idle detector implementation
pub struct IdleDetector {
    /// Configuration
    config: Arc<RwLock<IdleConfig>>,
    /// Current idle status
    status: Arc<RwLock<IdleStatus>>,
    /// Activity history for analysis
    activity_history: Arc<RwLock<Vec<ActivityMeasurement>>>,
    /// Detection task handle
    detection_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    /// Last mouse position
    last_mouse_pos: Arc<RwLock<(i32, i32)>>,
    /// Activity statistics
    stats: Arc<RwLock<IdleStats>>,
}

/// Idle detection statistics
#[derive(Debug, Default)]
pub struct IdleStats {
    /// Total idle periods detected
    idle_periods: u64,
    /// Total time spent idle (seconds)
    total_idle_time: u64,
    /// Total activity detections
    activity_detections: u64,
    /// Average idle duration
    avg_idle_duration: f32,
    /// Activity breakdown by source
    activity_by_source: std::collections::HashMap<String, u64>,
}

impl IdleDetector {
    /// Create a new idle detector
    pub async fn new(config: IdleConfig) -> Result<Self, AIError> {
        let now = Utc::now();
        
        let detector = Self {
            config: Arc::new(RwLock::new(config)),
            status: Arc::new(RwLock::new(IdleStatus {
                is_idle: false,
                idle_since: None,
                last_activity: now,
                idle_duration: 0,
                activity_sources: Vec::new(),
                activity_level: 0.0,
            })),
            activity_history: Arc::new(RwLock::new(Vec::new())),
            detection_handle: Arc::new(RwLock::new(None)),
            last_mouse_pos: Arc::new(RwLock::new((0, 0))),
            stats: Arc::new(RwLock::new(IdleStats::default())),
        };
        
        info!("Idle detector initialized");
        Ok(detector)
    }
    
    /// Start idle detection
    pub async fn start(&self) -> Result<(), AIError> {
        if self.detection_handle.read().is_some() {
            return Ok(());
        }
        
        let config = self.config.clone();
        let status = self.status.clone();
        let activity_history = self.activity_history.clone();
        let last_mouse_pos = self.last_mouse_pos.clone();
        let stats = self.stats.clone();
        
        *self.detection_handle.write() = Some(tokio::spawn(async move {
            Self::run_detection_loop(config, status, activity_history, last_mouse_pos, stats).await;
        }));
        
        info!("Idle detection started");
        Ok(())
    }
    
    /// Stop idle detection
    pub async fn stop(&self) -> Result<(), AIError> {
        if let Some(handle) = self.detection_handle.write().take() {
            handle.abort();
            info!("Idle detection stopped");
        }
        Ok(())
    }
    
    /// Get current idle status
    pub async fn get_status(&self) -> Result<IdleStatus, AIError> {
        Ok(self.status.read().clone())
    }
    
    /// Update configuration
    pub async fn update_config(&self, new_config: IdleConfig) -> Result<(), AIError> {
        *self.config.write() = new_config;
        info!("Idle detector configuration updated");
        Ok(())
    }
    
    /// Force update activity (for external triggers)
    pub fn record_activity(&self, source: ActivitySource) {
        let now = Utc::now();
        let mut status = self.status.write();
        
        // Update last activity
        status.last_activity = now;
        status.activity_sources.push(source.clone());
        
        // If was idle, break idle state
        if status.is_idle {
            status.is_idle = false;
            status.idle_since = None;
            
            // Update statistics
            let mut stats = self.stats.write();
            stats.activity_detections += 1;
            
            let source_key = format!("{:?}", source);
            stats.activity_by_source.entry(source_key).and_modify(|e| *e += 1).or_insert(1);
            
            info!("Activity detected: {:?} - idle state broken", source);
        }
    }
    
    /// Get idle detection statistics
    pub fn get_stats(&self) -> IdleStats {
        self.stats.read().clone()
    }
    
    /// Main detection loop
    async fn run_detection_loop(
        config: Arc<RwLock<IdleConfig>>,
        status: Arc<RwLock<IdleStatus>>,
        activity_history: Arc<RwLock<Vec<ActivityMeasurement>>>,
        last_mouse_pos: Arc<RwLock<(i32, i32)>>,
        stats: Arc<RwLock<IdleStats>>,
    ) {
        let mut check_interval = {
            let config_guard = config.read();
            interval(StdDuration::from_secs(config_guard.check_interval))
        };
        
        info!("Starting idle detection loop");
        
        loop {
            check_interval.tick().await;
            
            let config_clone = {
                let config_guard = config.read();
                if !config_guard.enabled {
                    continue;
                }
                config_guard.clone()
            };
            
            // Measure current activity
            let measurement = Self::measure_activity(&config_clone, &last_mouse_pos).await;
            
            // Analyze activity
            let has_activity = Self::analyze_activity(&measurement, &config_clone);
            
            // Update activity history
            {
                let mut history = activity_history.write();
                history.push(measurement.clone());
                
                // Keep only recent measurements (last hour)
                let cutoff = Utc::now() - Duration::hours(1);
                history.retain(|m| m.timestamp > cutoff);
            }
            
            // Update idle status
            let now = Utc::now();
            let mut status = status.write();
            let idle_threshold = Duration::seconds(config_clone.idle_threshold as i64);
            
            if has_activity {
                // Activity detected
                status.last_activity = now;
                status.activity_level = Self::calculate_activity_level(&measurement);
                
                if status.is_idle {
                    // Breaking idle state
                    status.is_idle = false;
                    
                    if let Some(idle_since) = status.idle_since {
                        let idle_duration = (now - idle_since).num_seconds() as u64;
                        
                        // Update statistics
                        let mut stats = stats.write();
                        stats.total_idle_time += idle_duration;
                        stats.activity_detections += 1;
                        
                        // Update average idle duration
                        if stats.idle_periods > 0 {
                            stats.avg_idle_duration = stats.total_idle_time as f32 / stats.idle_periods as f32;
                        }
                        
                        debug!("Idle state broken after {} seconds", idle_duration);
                    }
                    
                    status.idle_since = None;
                    status.idle_duration = 0;
                }
            } else {
                // No activity detected
                let time_since_activity = now - status.last_activity;
                
                if time_since_activity > idle_threshold && !status.is_idle {
                    // Entering idle state
                    status.is_idle = true;
                    status.idle_since = Some(now);
                    
                    // Update statistics
                    let mut stats = stats.write();
                    stats.idle_periods += 1;
                    
                    info!("User became idle after {} seconds of inactivity", time_since_activity.num_seconds());
                }
                
                if status.is_idle {
                    status.idle_duration = time_since_activity.num_seconds() as u64;
                }
            }
            
            // Clear old activity sources
            status.activity_sources.retain(|_| false); // Clear all for next iteration
        }
    }
    
    /// Measure current system activity
    async fn measure_activity(
        config: &IdleConfig,
        last_mouse_pos: &Arc<RwLock<(i32, i32)>>,
    ) -> ActivityMeasurement {
        let mut measurement = ActivityMeasurement {
            mouse_delta: (0, 0),
            keyboard_events: 0,
            audio_level: 0.0,
            network_bytes: 0,
            timestamp: Utc::now(),
        };
        
        // Mouse activity detection
        if config.detect_mouse {
            if let Ok(mouse_pos) = Self::get_mouse_position() {
                let mut last_pos = last_mouse_pos.write();
                measurement.mouse_delta = (mouse_pos.0 - last_pos.0, mouse_pos.1 - last_pos.1);
                *last_pos = mouse_pos;
            }
        }
        
        // Keyboard activity detection
        if config.detect_keyboard {
            measurement.keyboard_events = Self::get_keyboard_events().await.unwrap_or(0);
        }
        
        // Audio activity detection
        if config.detect_audio {
            measurement.audio_level = Self::get_audio_level().await.unwrap_or(0.0);
        }
        
        // Network activity detection
        if config.detect_network {
            measurement.network_bytes = Self::get_network_activity().await.unwrap_or(0);
        }
        
        measurement
    }
    
    /// Analyze activity measurement to determine if user is active
    fn analyze_activity(measurement: &ActivityMeasurement, config: &IdleConfig) -> bool {
        let mut has_activity = false;
        
        // Check mouse movement
        if config.detect_mouse {
            let mouse_distance = (measurement.mouse_delta.0.pow(2) + measurement.mouse_delta.1.pow(2)) as f32;
            if mouse_distance.sqrt() > 5.0 { // Minimum movement threshold
                has_activity = true;
            }
        }
        
        // Check keyboard activity
        if config.detect_keyboard && measurement.keyboard_events > 0 {
            has_activity = true;
        }
        
        // Check audio activity
        if config.detect_audio && measurement.audio_level > config.audio_threshold {
            has_activity = true;
        }
        
        // Check network activity
        if config.detect_network && measurement.network_bytes > config.network_threshold {
            has_activity = true;
        }
        
        has_activity
    }
    
    /// Calculate activity level from measurement
    fn calculate_activity_level(measurement: &ActivityMeasurement) -> f32 {
        let mut level = 0.0;
        
        // Mouse contribution
        let mouse_distance = (measurement.mouse_delta.0.pow(2) + measurement.mouse_delta.1.pow(2)) as f32;
        level += (mouse_distance.sqrt() / 100.0).min(1.0) * 0.3;
        
        // Keyboard contribution
        level += (measurement.keyboard_events as f32 / 10.0).min(1.0) * 0.3;
        
        // Audio contribution
        level += measurement.audio_level * 0.2;
        
        // Network contribution
        level += (measurement.network_bytes as f32 / 10240.0).min(1.0) * 0.2;
        
        level.min(1.0)
    }
    
    /// Get current mouse position (platform-specific)
    fn get_mouse_position() -> Result<(i32, i32), AIError> {
        // For Linux/Wayland, we would need to use appropriate APIs
        // This is a simplified implementation
        if let Ok(_display) = std::env::var("DISPLAY") {
            // X11 environment - could use xdotool or similar
            // For now, return a dummy position
            Ok((0, 0))
        } else {
            // Wayland environment - more complex to get mouse position
            // Would need to use wlroots or similar
            Ok((0, 0))
        }
    }
    
    /// Get keyboard events count (platform-specific)
    async fn get_keyboard_events() -> Result<u32, AIError> {
        // Check /proc/interrupts for keyboard interrupts
        if let Ok(content) = fs::read_to_string("/proc/interrupts") {
            let keyboard_lines: Vec<&str> = content.lines()
                .filter(|line| line.contains("keyboard") || line.contains("i8042"))
                .collect();
            
            if let Some(line) = keyboard_lines.first() {
                // Parse interrupt count (very simplified)
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() > 1 {
                    if let Ok(count) = parts[1].parse::<u32>() {
                        return Ok(count);
                    }
                }
            }
        }
        
        Ok(0)
    }
    
    /// Get current audio level (platform-specific)
    async fn get_audio_level() -> Result<f32, AIError> {
        // Check PulseAudio or ALSA for audio activity
        // This is a simplified implementation
        if Path::new("/usr/bin/pactl").exists() {
            // PulseAudio is available
            // Could use pactl to get sink info
            Ok(0.0)
        } else if Path::new("/proc/asound").exists() {
            // ALSA is available
            // Could check /proc/asound for audio activity
            Ok(0.0)
        } else {
            Ok(0.0)
        }
    }
    
    /// Get network activity (bytes transferred)
    async fn get_network_activity() -> Result<u64, AIError> {
        // Check /proc/net/dev for network statistics
        if let Ok(content) = fs::read_to_string("/proc/net/dev") {
            let mut total_bytes = 0u64;
            
            for line in content.lines().skip(2) { // Skip header lines
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() > 9 {
                    // RX bytes (column 1) + TX bytes (column 9)
                    if let (Ok(rx), Ok(tx)) = (parts[1].parse::<u64>(), parts[9].parse::<u64>()) {
                        total_bytes += rx + tx;
                    }
                }
            }
            
            Ok(total_bytes)
        } else {
            Ok(0)
        }
    }
}

impl Clone for IdleStats {
    fn clone(&self) -> Self {
        Self {
            idle_periods: self.idle_periods,
            total_idle_time: self.total_idle_time,
            activity_detections: self.activity_detections,
            avg_idle_duration: self.avg_idle_duration,
            activity_by_source: self.activity_by_source.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_idle_detector_creation() {
        let config = IdleConfig::default();
        let detector = IdleDetector::new(config).await.unwrap();
        
        let status = detector.get_status().await.unwrap();
        assert!(!status.is_idle);
        assert!(status.idle_since.is_none());
    }
    
    #[test]
    fn test_idle_config_default() {
        let config = IdleConfig::default();
        assert!(config.enabled);
        assert_eq!(config.idle_threshold, 300);
        assert_eq!(config.check_interval, 10);
    }
    
    #[test]
    fn test_activity_level_calculation() {
        let measurement = ActivityMeasurement {
            mouse_delta: (10, 10),
            keyboard_events: 5,
            audio_level: 0.5,
            network_bytes: 1000,
            timestamp: Utc::now(),
        };
        
        let level = IdleDetector::calculate_activity_level(&measurement);
        assert!(level > 0.0);
        assert!(level <= 1.0);
    }
    
    #[tokio::test]
    async fn test_activity_recording() {
        let config = IdleConfig::default();
        let detector = IdleDetector::new(config).await.unwrap();
        
        // Record some activity
        detector.record_activity(ActivitySource::Mouse);
        
        let status = detector.get_status().await.unwrap();
        assert!(!status.is_idle);
        assert!(!status.activity_sources.is_empty());
    }
}