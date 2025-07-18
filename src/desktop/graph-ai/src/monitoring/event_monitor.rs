//! Event monitoring for behavioral learning
//! 
//! Monitors system events from various sources including Wayland,
//! D-Bus, filesystem, and browser to capture user behavior patterns.

use crate::AIError;
use crate::monitoring::{RawEvent, EventSource, MonitoringConfig};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use parking_lot::RwLock;
use tokio::sync::mpsc;
use tokio::time::interval;
use std::time::Duration;
use chrono::{DateTime, Utc};
use log::{info, debug};
use std::collections::HashMap;
use std::path::Path;
use std::fs;
use std::process::Command;

/// Event monitor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMonitorConfig {
    /// Enable event monitoring
    pub enabled: bool,
    /// Monitor Wayland events
    pub wayland_events: bool,
    /// Monitor D-Bus events
    pub dbus_events: bool,
    /// Monitor filesystem events
    pub filesystem_events: bool,
    /// Monitor browser events
    pub browser_events: bool,
    /// Monitor application events
    pub application_events: bool,
    /// Monitor system events
    pub system_events: bool,
    /// Event buffer size
    pub buffer_size: usize,
    /// Event processing interval (milliseconds)
    pub processing_interval: u64,
    /// Maximum events per second
    pub max_events_per_second: u32,
    /// Filesystem watch directories
    pub watch_directories: Vec<String>,
    /// Excluded file patterns
    pub excluded_patterns: Vec<String>,
    /// D-Bus services to monitor
    pub dbus_services: Vec<String>,
    /// Browser extensions to monitor
    pub browser_extensions: Vec<String>,
}

impl Default for EventMonitorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            wayland_events: true,
            dbus_events: true,
            filesystem_events: true,
            browser_events: true,
            application_events: true,
            system_events: true,
            buffer_size: 1000,
            processing_interval: 100, // 100ms
            max_events_per_second: 1000,
            watch_directories: vec![
                std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string()),
                "/usr/share/applications".to_string(),
                "/var/log".to_string(),
            ],
            excluded_patterns: vec![
                "*.tmp".to_string(),
                "*.cache".to_string(),
                "*.log".to_string(),
                "*/.git/*".to_string(),
                "*/node_modules/*".to_string(),
            ],
            dbus_services: vec![
                "org.freedesktop.Notifications".to_string(),
                "org.gnome.Shell".to_string(),
                "org.kde.plasmashell".to_string(),
                "org.freedesktop.portal.Desktop".to_string(),
            ],
            browser_extensions: vec![
                "chrome-extension://".to_string(),
                "moz-extension://".to_string(),
            ],
        }
    }
}

/// Event monitor statistics
#[derive(Debug, Default, Clone)]
pub struct EventMonitorStats {
    /// Total events processed
    pub total_events: u64,
    /// Events by source
    pub events_by_source: HashMap<String, u64>,
    /// Events per second (current)
    pub events_per_second: f64,
    /// Events dropped due to rate limiting
    pub dropped_events: u64,
    /// Last event timestamp
    pub last_event: Option<DateTime<Utc>>,
    /// Monitor start time
    pub started_at: DateTime<Utc>,
    /// Active monitors
    pub active_monitors: Vec<String>,
}

/// Event monitor implementation
pub struct EventMonitor {
    /// Configuration
    config: Arc<RwLock<EventMonitorConfig>>,
    /// Event sender
    event_sender: mpsc::UnboundedSender<RawEvent>,
    /// Statistics
    stats: Arc<RwLock<EventMonitorStats>>,
    /// Monitor handles
    monitor_handles: Arc<RwLock<Vec<tokio::task::JoinHandle<()>>>>,
    /// Rate limiter state
    rate_limiter: Arc<RwLock<RateLimiter>>,
    /// Current sampling rate
    sampling_rate: Arc<RwLock<u32>>,
}

/// Rate limiter for event monitoring
#[derive(Debug)]
struct RateLimiter {
    /// Events in current window
    events_in_window: u32,
    /// Window start time
    window_start: DateTime<Utc>,
    /// Window duration (seconds)
    window_duration: u64,
    /// Maximum events per window
    max_events_per_window: u32,
}

impl RateLimiter {
    fn new(max_events_per_second: u32) -> Self {
        Self {
            events_in_window: 0,
            window_start: Utc::now(),
            window_duration: 1,
            max_events_per_window: max_events_per_second,
        }
    }
    
    fn should_allow_event(&mut self) -> bool {
        let now = Utc::now();
        
        // Reset window if expired
        if (now - self.window_start).num_seconds() >= self.window_duration as i64 {
            self.events_in_window = 0;
            self.window_start = now;
        }
        
        // Check if we can allow this event
        if self.events_in_window < self.max_events_per_window {
            self.events_in_window += 1;
            true
        } else {
            false
        }
    }
}

impl EventMonitor {
    /// Create a new event monitor
    pub async fn new(
        config: MonitoringConfig,
        event_sender: mpsc::UnboundedSender<RawEvent>,
    ) -> Result<Self, AIError> {
        let event_config = EventMonitorConfig::default(); // Use default for now
        
        let monitor = Self {
            config: Arc::new(RwLock::new(event_config.clone())),
            event_sender,
            stats: Arc::new(RwLock::new(EventMonitorStats {
                started_at: Utc::now(),
                ..Default::default()
            })),
            monitor_handles: Arc::new(RwLock::new(Vec::new())),
            rate_limiter: Arc::new(RwLock::new(RateLimiter::new(event_config.max_events_per_second))),
            sampling_rate: Arc::new(RwLock::new(config.sampling_rate)),
        };
        
        info!("Event monitor initialized");
        Ok(monitor)
    }
    
    /// Start event monitoring
    pub async fn start(&self) -> Result<(), AIError> {
        let config = self.config.read().clone();
        
        if !config.enabled {
            return Ok(());
        }
        
        // Start different event monitors based on configuration
        if config.wayland_events {
            self.start_wayland_monitor().await?;
        }
        
        if config.dbus_events {
            self.start_dbus_monitor().await?;
        }
        
        if config.filesystem_events {
            self.start_filesystem_monitor().await?;
        }
        
        if config.browser_events {
            self.start_browser_monitor().await?;
        }
        
        if config.application_events {
            self.start_application_monitor().await?;
        }
        
        if config.system_events {
            self.start_system_monitor().await?;
        }
        
        // Update stats
        {
            let mut stats = self.stats.write();
            stats.active_monitors.clear();
            if config.wayland_events { stats.active_monitors.push("wayland".to_string()); }
            if config.dbus_events { stats.active_monitors.push("dbus".to_string()); }
            if config.filesystem_events { stats.active_monitors.push("filesystem".to_string()); }
            if config.browser_events { stats.active_monitors.push("browser".to_string()); }
            if config.application_events { stats.active_monitors.push("application".to_string()); }
            if config.system_events { stats.active_monitors.push("system".to_string()); }
        }
        
        info!("Event monitoring started with {} active monitors", self.stats.read().active_monitors.len());
        Ok(())
    }
    
    /// Stop event monitoring
    pub async fn stop(&self) -> Result<(), AIError> {
        // Cancel all monitor tasks
        for handle in self.monitor_handles.write().drain(..) {
            handle.abort();
        }
        
        // Clear active monitors
        self.stats.write().active_monitors.clear();
        
        info!("Event monitoring stopped");
        Ok(())
    }
    
    /// Update configuration
    pub async fn update_config(&self, new_config: MonitoringConfig) -> Result<(), AIError> {
        *self.sampling_rate.write() = new_config.sampling_rate;
        
        // Update rate limiter
        let mut rate_limiter = self.rate_limiter.write();
        rate_limiter.max_events_per_window = new_config.sampling_rate;
        
        info!("Event monitor configuration updated");
        Ok(())
    }
    
    /// Set sampling rate
    pub async fn set_sampling_rate(&self, rate: u32) -> Result<(), AIError> {
        *self.sampling_rate.write() = rate;
        
        // Update rate limiter
        let mut rate_limiter = self.rate_limiter.write();
        rate_limiter.max_events_per_window = rate;
        
        debug!("Event monitor sampling rate set to {} Hz", rate);
        Ok(())
    }
    
    /// Get monitoring statistics
    pub fn get_stats(&self) -> EventMonitorStats {
        self.stats.read().clone()
    }
    
    /// Send an event with rate limiting
    fn send_event(&self, event: RawEvent) {
        // Check rate limiter
        let should_send = self.rate_limiter.write().should_allow_event();
        
        if should_send {
            if let Err(_) = self.event_sender.send(event.clone()) {
                log::warn!("Failed to send event - channel closed");
                return;
            }
            
            // Update statistics
            let mut stats = self.stats.write();
            stats.total_events += 1;
            stats.last_event = Some(event.timestamp);
            
            let source_key = format!("{:?}", event.source);
            stats.events_by_source.entry(source_key).and_modify(|e| *e += 1).or_insert(1);
            
            // Calculate events per second
            let duration = Utc::now() - stats.started_at;
            if duration.num_seconds() > 0 {
                stats.events_per_second = stats.total_events as f64 / duration.num_seconds() as f64;
            }
        } else {
            self.stats.write().dropped_events += 1;
        }
    }
    
    /// Start Wayland event monitoring
    async fn start_wayland_monitor(&self) -> Result<(), AIError> {
        let event_sender = self.event_sender.clone();
        let stats = self.stats.clone();
        
        let handle = tokio::spawn(async move {
            info!("Starting Wayland event monitor");
            
            // Monitor Wayland socket for events
            let mut interval = interval(Duration::from_millis(100));
            
            loop {
                interval.tick().await;
                
                // Check for window focus changes
                if let Ok(output) = Command::new("wlr-randr").output() {
                    if let Ok(output_str) = String::from_utf8(output.stdout) {
                        if !output_str.is_empty() {
                            let event = RawEvent {
                                source: EventSource::Wayland,
                                timestamp: Utc::now(),
                                data: serde_json::json!({
                                    "event_type": "display_info",
                                    "output": output_str,
                                }),
                                metadata: serde_json::json!({
                                    "monitor": "wayland"
                                }),
                            };
                            
                            if let Err(_) = event_sender.send(event) {
                                break;
                            }
                        }
                    }
                }
                
                // Check for window list changes
                if let Ok(output) = Command::new("swaymsg").args(&["-t", "get_tree"]).output() {
                    if let Ok(output_str) = String::from_utf8(output.stdout) {
                        if !output_str.is_empty() {
                            let event = RawEvent {
                                source: EventSource::Wayland,
                                timestamp: Utc::now(),
                                data: serde_json::json!({
                                    "event_type": "window_tree",
                                    "tree": output_str,
                                }),
                                metadata: serde_json::json!({
                                    "monitor": "wayland"
                                }),
                            };
                            
                            if let Err(_) = event_sender.send(event) {
                                break;
                            }
                        }
                    }
                }
            }
            
            info!("Wayland event monitor stopped");
        });
        
        self.monitor_handles.write().push(handle);
        Ok(())
    }
    
    /// Start D-Bus event monitoring
    async fn start_dbus_monitor(&self) -> Result<(), AIError> {
        let event_sender = self.event_sender.clone();
        let config = self.config.clone();
        
        let handle = tokio::spawn(async move {
            info!("Starting D-Bus event monitor");
            
            // Monitor D-Bus messages
            let mut interval = interval(Duration::from_millis(200));
            
            loop {
                interval.tick().await;
                
                // Monitor notifications
                if let Ok(output) = Command::new("dbus-monitor")
                    .args(&["--session", "--monitor", "interface=org.freedesktop.Notifications"])
                    .output()
                {
                    if let Ok(output_str) = String::from_utf8(output.stdout) {
                        if !output_str.is_empty() {
                            let event = RawEvent {
                                source: EventSource::DBus,
                                timestamp: Utc::now(),
                                data: serde_json::json!({
                                    "event_type": "notification",
                                    "message": output_str,
                                }),
                                metadata: serde_json::json!({
                                    "monitor": "dbus",
                                    "service": "org.freedesktop.Notifications"
                                }),
                            };
                            
                            if let Err(_) = event_sender.send(event) {
                                break;
                            }
                        }
                    }
                }
            }
            
            info!("D-Bus event monitor stopped");
        });
        
        self.monitor_handles.write().push(handle);
        Ok(())
    }
    
    /// Start filesystem event monitoring
    async fn start_filesystem_monitor(&self) -> Result<(), AIError> {
        let event_sender = self.event_sender.clone();
        let config = self.config.clone();
        
        let handle = tokio::spawn(async move {
            info!("Starting filesystem event monitor");
            
            // Use inotify to watch filesystem events
            let mut interval = interval(Duration::from_millis(500));
            
            loop {
                interval.tick().await;
                
                // Watch home directory for changes
                let home_dir = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
                
                if let Ok(output) = Command::new("inotifywait")
                    .args(&["-r", "-e", "modify,create,delete", &home_dir])
                    .output()
                {
                    if let Ok(output_str) = String::from_utf8(output.stdout) {
                        if !output_str.is_empty() {
                            let event = RawEvent {
                                source: EventSource::FileSystem,
                                timestamp: Utc::now(),
                                data: serde_json::json!({
                                    "event_type": "file_change",
                                    "path": home_dir,
                                    "change": output_str,
                                }),
                                metadata: serde_json::json!({
                                    "monitor": "filesystem"
                                }),
                            };
                            
                            if let Err(_) = event_sender.send(event) {
                                break;
                            }
                        }
                    }
                }
            }
            
            info!("Filesystem event monitor stopped");
        });
        
        self.monitor_handles.write().push(handle);
        Ok(())
    }
    
    /// Start browser event monitoring
    async fn start_browser_monitor(&self) -> Result<(), AIError> {
        let event_sender = self.event_sender.clone();
        
        let handle = tokio::spawn(async move {
            info!("Starting browser event monitor");
            
            // Monitor browser history and bookmarks
            let mut interval = interval(Duration::from_secs(10));
            
            loop {
                interval.tick().await;
                
                // Check Firefox history
                let firefox_profile = format!("{}/.mozilla/firefox", std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string()));
                if Path::new(&firefox_profile).exists() {
                    let event = RawEvent {
                        source: EventSource::Browser,
                        timestamp: Utc::now(),
                        data: serde_json::json!({
                            "event_type": "browser_activity",
                            "browser": "firefox",
                            "activity": "profile_check",
                        }),
                        metadata: serde_json::json!({
                            "monitor": "browser"
                        }),
                    };
                    
                    if let Err(_) = event_sender.send(event) {
                        break;
                    }
                }
                
                // Check Chrome history
                let chrome_profile = format!("{}/.config/google-chrome", std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string()));
                if Path::new(&chrome_profile).exists() {
                    let event = RawEvent {
                        source: EventSource::Browser,
                        timestamp: Utc::now(),
                        data: serde_json::json!({
                            "event_type": "browser_activity",
                            "browser": "chrome",
                            "activity": "profile_check",
                        }),
                        metadata: serde_json::json!({
                            "monitor": "browser"
                        }),
                    };
                    
                    if let Err(_) = event_sender.send(event) {
                        break;
                    }
                }
            }
            
            info!("Browser event monitor stopped");
        });
        
        self.monitor_handles.write().push(handle);
        Ok(())
    }
    
    /// Start application event monitoring
    async fn start_application_monitor(&self) -> Result<(), AIError> {
        let event_sender = self.event_sender.clone();
        
        let handle = tokio::spawn(async move {
            info!("Starting application event monitor");
            
            // Monitor running applications
            let mut interval = interval(Duration::from_secs(5));
            
            loop {
                interval.tick().await;
                
                // Get running processes
                if let Ok(output) = Command::new("ps").args(&["aux"]).output() {
                    if let Ok(output_str) = String::from_utf8(output.stdout) {
                        let lines: Vec<&str> = output_str.lines().collect();
                        
                        let event = RawEvent {
                            source: EventSource::Application,
                            timestamp: Utc::now(),
                            data: serde_json::json!({
                                "event_type": "process_list",
                                "process_count": lines.len(),
                                "sample_processes": lines.iter().take(10).collect::<Vec<_>>(),
                            }),
                            metadata: serde_json::json!({
                                "monitor": "application"
                            }),
                        };
                        
                        if let Err(_) = event_sender.send(event) {
                            break;
                        }
                    }
                }
            }
            
            info!("Application event monitor stopped");
        });
        
        self.monitor_handles.write().push(handle);
        Ok(())
    }
    
    /// Start system event monitoring
    async fn start_system_monitor(&self) -> Result<(), AIError> {
        let event_sender = self.event_sender.clone();
        
        let handle = tokio::spawn(async move {
            info!("Starting system event monitor");
            
            // Monitor system resources and events
            let mut interval = interval(Duration::from_secs(30));
            
            loop {
                interval.tick().await;
                
                // Check system load
                if let Ok(loadavg) = fs::read_to_string("/proc/loadavg") {
                    let event = RawEvent {
                        source: EventSource::System,
                        timestamp: Utc::now(),
                        data: serde_json::json!({
                            "event_type": "system_load",
                            "loadavg": loadavg.trim(),
                        }),
                        metadata: serde_json::json!({
                            "monitor": "system"
                        }),
                    };
                    
                    if let Err(_) = event_sender.send(event) {
                        break;
                    }
                }
                
                // Check memory usage
                if let Ok(meminfo) = fs::read_to_string("/proc/meminfo") {
                    let event = RawEvent {
                        source: EventSource::System,
                        timestamp: Utc::now(),
                        data: serde_json::json!({
                            "event_type": "memory_info",
                            "meminfo": meminfo.lines().take(5).collect::<Vec<_>>(),
                        }),
                        metadata: serde_json::json!({
                            "monitor": "system"
                        }),
                    };
                    
                    if let Err(_) = event_sender.send(event) {
                        break;
                    }
                }
            }
            
            info!("System event monitor stopped");
        });
        
        self.monitor_handles.write().push(handle);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;
    
    #[tokio::test]
    async fn test_event_monitor_creation() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let config = MonitoringConfig::default();
        
        let monitor = EventMonitor::new(config, tx).await.unwrap();
        
        let stats = monitor.get_stats();
        assert_eq!(stats.total_events, 0);
        assert!(stats.active_monitors.is_empty());
    }
    
    #[test]
    fn test_rate_limiter() {
        let mut limiter = RateLimiter::new(10);
        
        // Should allow events up to limit
        for _ in 0..10 {
            assert!(limiter.should_allow_event());
        }
        
        // Should reject events beyond limit
        assert!(!limiter.should_allow_event());
    }
    
    #[test]
    fn test_event_monitor_config_default() {
        let config = EventMonitorConfig::default();
        assert!(config.enabled);
        assert!(config.wayland_events);
        assert!(config.dbus_events);
        assert_eq!(config.buffer_size, 1000);
    }
}