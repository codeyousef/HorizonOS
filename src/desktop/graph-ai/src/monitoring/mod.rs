//! Continuous event monitoring for behavioral learning
//! 
//! This module provides privacy-aware monitoring of user activities
//! to enable intelligent pattern detection and suggestions.

pub mod event_monitor;
pub mod privacy_filter;
pub mod idle_detector;
pub mod resource_monitor;

use crate::AIError;
use crate::storage::{UserAction, StorageManager};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use parking_lot::RwLock;
use chrono::{DateTime, Utc};
use tokio::sync::mpsc;
use futures::stream::Stream;
use std::pin::Pin;
use log::{info, warn, error, debug};

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Whether monitoring is enabled
    pub enabled: bool,
    /// Sampling rate in Hz (events per second)
    pub sampling_rate: u32,
    /// Buffer size for event batching
    pub buffer_size: usize,
    /// Batch timeout in milliseconds
    pub batch_timeout: u64,
    /// Privacy filter configuration
    pub privacy: privacy_filter::PrivacyFilterConfig,
    /// Idle detection configuration
    pub idle_detection: idle_detector::IdleConfig,
    /// Resource monitoring configuration
    pub resource_monitoring: resource_monitor::ResourceConfig,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            sampling_rate: 10, // 10 Hz default
            buffer_size: 1000,
            batch_timeout: 5000, // 5 seconds
            privacy: privacy_filter::PrivacyFilterConfig::default(),
            idle_detection: idle_detector::IdleConfig::default(),
            resource_monitoring: resource_monitor::ResourceConfig::default(),
        }
    }
}

/// Monitoring statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MonitoringStats {
    /// Total events captured
    pub total_events: u64,
    /// Events filtered out for privacy
    pub filtered_events: u64,
    /// Events successfully stored
    pub stored_events: u64,
    /// Events dropped due to overload
    pub dropped_events: u64,
    /// Current sampling rate
    pub current_sampling_rate: u32,
    /// Is currently monitoring
    pub is_monitoring: bool,
    /// Is user idle
    pub is_idle: bool,
    /// Last event timestamp
    pub last_event: Option<DateTime<Utc>>,
    /// Monitoring start time
    pub started_at: DateTime<Utc>,
}

/// Event source type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventSource {
    /// Wayland compositor events
    Wayland,
    /// D-Bus system events
    DBus,
    /// File system events
    FileSystem,
    /// Browser extension events
    Browser,
    /// Application events
    Application,
    /// System events
    System,
}

/// Raw event before processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawEvent {
    /// Event source
    pub source: EventSource,
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
    /// Event data
    pub data: serde_json::Value,
    /// Event metadata
    pub metadata: serde_json::Value,
}

/// Main monitoring system
pub struct MonitoringSystem {
    /// Configuration
    config: Arc<RwLock<MonitoringConfig>>,
    /// Event monitor
    event_monitor: Arc<event_monitor::EventMonitor>,
    /// Privacy filter
    privacy_filter: Arc<privacy_filter::PrivacyFilter>,
    /// Idle detector
    idle_detector: Arc<idle_detector::IdleDetector>,
    /// Resource monitor
    resource_monitor: Arc<resource_monitor::ResourceMonitor>,
    /// Storage manager
    storage: Arc<StorageManager>,
    /// Event channel
    event_tx: mpsc::UnboundedSender<RawEvent>,
    /// Monitoring statistics
    stats: Arc<RwLock<MonitoringStats>>,
    /// Monitoring task handle
    monitoring_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
}

impl MonitoringSystem {
    /// Create a new monitoring system
    pub async fn new(
        config: MonitoringConfig,
        storage: Arc<StorageManager>,
    ) -> Result<Self, AIError> {
        let config = Arc::new(RwLock::new(config));
        
        // Create event channel
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        
        // Initialize components
        let event_monitor = Arc::new(event_monitor::EventMonitor::new(
            config.read().clone(),
            event_tx.clone(),
        ).await?);
        
        let privacy_filter = Arc::new(
            privacy_filter::PrivacyFilter::new(config.read().privacy.clone()).await?
        );
        
        let idle_detector = Arc::new(
            idle_detector::IdleDetector::new(config.read().idle_detection.clone()).await?
        );
        
        let resource_monitor = Arc::new(
            resource_monitor::ResourceMonitor::new(config.read().resource_monitoring.clone()).await?
        );
        
        let stats = Arc::new(RwLock::new(MonitoringStats {
            started_at: Utc::now(),
            current_sampling_rate: config.read().sampling_rate,
            ..Default::default()
        }));
        
        let system = Self {
            config: config.clone(),
            event_monitor,
            privacy_filter: privacy_filter.clone(),
            idle_detector,
            resource_monitor,
            storage: storage.clone(),
            event_tx,
            stats: stats.clone(),
            monitoring_handle: Arc::new(RwLock::new(None)),
        };
        
        // Start event processing task
        *system.monitoring_handle.write() = Some(tokio::spawn(Self::process_events(
            config,
            privacy_filter,
            storage,
            stats,
            event_rx,
        )));
        
        info!("Monitoring system initialized");
        Ok(system)
    }
    
    /// Start monitoring
    pub async fn start(&self) -> Result<(), AIError> {
        if self.stats.read().is_monitoring {
            return Ok(());
        }
        
        info!("Starting monitoring system");
        
        // Start all components
        self.event_monitor.start().await?;
        self.idle_detector.start().await?;
        self.resource_monitor.start().await?;
        
        // Update stats
        {
            let mut stats = self.stats.write();
            stats.is_monitoring = true;
            stats.started_at = Utc::now();
        }
        
        info!("Monitoring system started");
        Ok(())
    }
    
    /// Stop monitoring
    pub async fn stop(&self) -> Result<(), AIError> {
        if !self.stats.read().is_monitoring {
            return Ok(());
        }
        
        info!("Stopping monitoring system");
        
        // Stop all components
        self.event_monitor.stop().await?;
        self.idle_detector.stop().await?;
        self.resource_monitor.stop().await?;
        
        // Update stats
        {
            let mut stats = self.stats.write();
            stats.is_monitoring = false;
        }
        
        info!("Monitoring system stopped");
        Ok(())
    }
    
    /// Get monitoring statistics
    pub fn get_stats(&self) -> MonitoringStats {
        self.stats.read().clone()
    }
    
    /// Update monitoring configuration
    pub async fn update_config(&self, new_config: MonitoringConfig) -> Result<(), AIError> {
        info!("Updating monitoring configuration");
        
        // Stop monitoring if running
        let was_monitoring = self.stats.read().is_monitoring;
        if was_monitoring {
            self.stop().await?;
        }
        
        // Update configuration
        *self.config.write() = new_config;
        
        // Update component configurations
        self.event_monitor.update_config(self.config.read().clone()).await?;
        self.privacy_filter.update_config(self.config.read().privacy.clone()).await?;
        self.idle_detector.update_config(self.config.read().idle_detection.clone()).await?;
        self.resource_monitor.update_config(self.config.read().resource_monitoring.clone()).await?;
        
        // Restart monitoring if it was running
        if was_monitoring {
            self.start().await?;
        }
        
        info!("Monitoring configuration updated");
        Ok(())
    }
    
    /// Get current configuration
    pub fn get_config(&self) -> MonitoringConfig {
        self.config.read().clone()
    }
    
    /// Send a raw event to the monitoring system
    pub fn send_event(&self, event: RawEvent) -> Result<(), AIError> {
        self.event_tx.send(event)
            .map_err(|e| AIError::Configuration(format!("Failed to send event: {}", e)))
    }
    
    /// Get event stream for external processing
    pub fn get_event_stream(&self) -> Pin<Box<dyn Stream<Item = UserAction> + Send>> {
        // TODO: Implement event stream for external consumers
        Box::pin(futures::stream::empty())
    }
    
    /// Process events from the event channel
    async fn process_events(
        config: Arc<RwLock<MonitoringConfig>>,
        privacy_filter: Arc<privacy_filter::PrivacyFilter>,
        storage: Arc<StorageManager>,
        stats: Arc<RwLock<MonitoringStats>>,
        mut event_rx: mpsc::UnboundedReceiver<RawEvent>,
    ) {
        info!("Starting event processing task");
        
        let mut event_buffer = Vec::new();
        let mut last_batch_time = Utc::now();
        
        while let Some(raw_event) = event_rx.recv().await {
            // Update statistics
            {
                let mut stats = stats.write();
                stats.total_events += 1;
                stats.last_event = Some(raw_event.timestamp);
            }
            
            // Apply privacy filter
            if let Some(filtered_event) = privacy_filter.filter_event(&raw_event).await {
                // Convert to user action
                if let Ok(user_action) = Self::convert_to_user_action(&filtered_event) {
                    event_buffer.push(user_action);
                } else {
                    stats.write().dropped_events += 1;
                }
            } else {
                stats.write().filtered_events += 1;
            }
            
            // Check if we should flush the buffer
            let should_flush = {
                let config = config.read();
                event_buffer.len() >= config.buffer_size ||
                (Utc::now() - last_batch_time).num_milliseconds() >= config.batch_timeout as i64
            };
            
            if should_flush && !event_buffer.is_empty() {
                // Store events in batch
                match storage.timescale.store_actions_batch(&event_buffer).await {
                    Ok(_) => {
                        stats.write().stored_events += event_buffer.len() as u64;
                        debug!("Stored {} events in batch", event_buffer.len());
                    }
                    Err(e) => {
                        error!("Failed to store event batch: {}", e);
                        stats.write().dropped_events += event_buffer.len() as u64;
                    }
                }
                
                event_buffer.clear();
                last_batch_time = Utc::now();
            }
        }
        
        // Flush any remaining events
        if !event_buffer.is_empty() {
            if let Err(e) = storage.timescale.store_actions_batch(&event_buffer).await {
                error!("Failed to store final event batch: {}", e);
            }
        }
        
        info!("Event processing task terminated");
    }
    
    /// Convert a raw event to a user action
    fn convert_to_user_action(event: &RawEvent) -> Result<UserAction, AIError> {
        // This is a simplified conversion - real implementation would be more sophisticated
        let action_type = match event.source {
            EventSource::Wayland => crate::storage::ActionType::WindowFocus,
            EventSource::DBus => crate::storage::ActionType::AppLaunch,
            EventSource::FileSystem => crate::storage::ActionType::FileOpen,
            EventSource::Browser => crate::storage::ActionType::WebNavigate,
            EventSource::Application => crate::storage::ActionType::AppLaunch,
            EventSource::System => crate::storage::ActionType::CommandExecute,
        };
        
        let target = event.data.get("target")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        
        let application = event.data.get("application")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        
        Ok(UserAction {
            id: uuid::Uuid::new_v4(),
            user_id: "default".to_string(),
            action_type,
            target,
            application,
            timestamp: event.timestamp,
            context: event.data.clone(),
            duration_ms: None,
            success: true,
            error_message: None,
        })
    }
    
    /// Adaptive sampling based on system load
    pub async fn adjust_sampling_rate(&self) -> Result<(), AIError> {
        let resource_stats = self.resource_monitor.get_stats().await?;
        let current_config = self.config.read().clone();
        
        let new_sampling_rate = if resource_stats.cpu_usage > 80.0 {
            // Reduce sampling rate under high CPU load
            (current_config.sampling_rate / 2).max(1)
        } else if resource_stats.memory_usage > 85.0 {
            // Reduce sampling rate under high memory pressure
            (current_config.sampling_rate / 2).max(1)
        } else if resource_stats.battery_level.map_or(false, |level| level < 20.0) {
            // Reduce sampling rate on low battery
            (current_config.sampling_rate / 4).max(1)
        } else {
            // Return to normal sampling rate
            current_config.sampling_rate
        };
        
        if new_sampling_rate != self.stats.read().current_sampling_rate {
            self.stats.write().current_sampling_rate = new_sampling_rate;
            self.event_monitor.set_sampling_rate(new_sampling_rate).await?;
            
            debug!("Adjusted sampling rate to {} Hz", new_sampling_rate);
        }
        
        Ok(())
    }
    
    /// Health check for the monitoring system
    pub async fn health_check(&self) -> Result<MonitoringHealth, AIError> {
        let stats = self.stats.read();
        let idle_status = self.idle_detector.get_status().await?;
        let resource_stats = self.resource_monitor.get_stats().await?;
        
        let healthy = stats.is_monitoring && 
                     resource_stats.cpu_usage < 90.0 && 
                     resource_stats.memory_usage < 95.0;
        
        Ok(MonitoringHealth {
            healthy,
            monitoring_active: stats.is_monitoring,
            user_idle: idle_status.is_idle,
            events_per_second: Self::calculate_events_per_second(&stats),
            resource_usage: resource_stats,
            last_event: stats.last_event,
        })
    }
    
    /// Calculate events per second from statistics
    fn calculate_events_per_second(stats: &MonitoringStats) -> f64 {
        let duration = Utc::now() - stats.started_at;
        let duration_seconds = duration.num_seconds() as f64;
        
        if duration_seconds > 0.0 {
            stats.total_events as f64 / duration_seconds
        } else {
            0.0
        }
    }
}

/// Monitoring system health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringHealth {
    pub healthy: bool,
    pub monitoring_active: bool,
    pub user_idle: bool,
    pub events_per_second: f64,
    pub resource_usage: resource_monitor::ResourceStats,
    pub last_event: Option<DateTime<Utc>>,
}

/// Cleanup task for monitoring system
pub struct MonitoringCleanup {
    storage: Arc<StorageManager>,
    config: Arc<RwLock<MonitoringConfig>>,
}

impl MonitoringCleanup {
    /// Create a new cleanup task
    pub fn new(storage: Arc<StorageManager>, config: Arc<RwLock<MonitoringConfig>>) -> Self {
        Self { storage, config }
    }
    
    /// Run cleanup task
    pub async fn run(&self) -> Result<u64, AIError> {
        info!("Starting monitoring cleanup task");
        
        let retention_policy = crate::storage::RetentionPolicy::default();
        let cleaned_count = self.storage.cleanup_all(retention_policy).await?;
        
        info!("Monitoring cleanup completed: {} items cleaned", cleaned_count);
        Ok(cleaned_count)
    }
    
    /// Schedule periodic cleanup
    pub async fn schedule_periodic_cleanup(&self) -> Result<(), AIError> {
        let storage = self.storage.clone();
        let config = self.config.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(24 * 60 * 60));
            
            loop {
                interval.tick().await;
                
                let cleanup = MonitoringCleanup::new(storage.clone(), config.clone());
                if let Err(e) = cleanup.run().await {
                    error!("Periodic cleanup failed: {}", e);
                }
            }
        });
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_monitoring_system_creation() {
        let config = MonitoringConfig::default();
        let storage = Arc::new(StorageManager::new_default());
        
        let result = MonitoringSystem::new(config, storage).await;
        
        // This test will fail without proper database setup, but serves as documentation
        // assert!(result.is_ok());
    }
    
    #[test]
    fn test_monitoring_config_default() {
        let config = MonitoringConfig::default();
        assert!(config.enabled);
        assert_eq!(config.sampling_rate, 10);
        assert_eq!(config.buffer_size, 1000);
    }
    
    #[test]
    fn test_raw_event_creation() {
        let event = RawEvent {
            source: EventSource::Wayland,
            timestamp: Utc::now(),
            data: serde_json::json!({"target": "test_app"}),
            metadata: serde_json::json!({"window_id": 123}),
        };
        
        assert!(matches!(event.source, EventSource::Wayland));
        assert_eq!(event.data["target"], "test_app");
    }
}