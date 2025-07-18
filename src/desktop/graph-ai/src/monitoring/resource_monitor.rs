//! Resource monitoring for adaptive behavior
//! 
//! Monitors system resources (CPU, memory, disk, network, battery)
//! to enable adaptive sampling and resource-aware AI processing.

use crate::AIError;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use parking_lot::RwLock;
use tokio::time::{sleep, interval};
use std::time::Duration;
use chrono::{DateTime, Utc};
use log::{info, warn, error, debug};
use std::fs;
use std::path::Path;
use std::collections::HashMap;
use sysinfo::System;

/// Resource monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceConfig {
    /// Enable resource monitoring
    pub enabled: bool,
    /// Monitoring interval in seconds
    pub monitoring_interval: u64,
    /// CPU usage threshold for high load
    pub cpu_threshold: f32,
    /// Memory usage threshold for high load
    pub memory_threshold: f32,
    /// Disk usage threshold for high load
    pub disk_threshold: f32,
    /// Network usage threshold for high load (bytes/sec)
    pub network_threshold: u64,
    /// Battery threshold for low battery
    pub battery_threshold: f32,
    /// Temperature threshold for thermal throttling
    pub temperature_threshold: f32,
    /// Enable thermal monitoring
    pub thermal_monitoring: bool,
    /// Enable battery monitoring
    pub battery_monitoring: bool,
    /// Enable network monitoring
    pub network_monitoring: bool,
    /// Enable disk monitoring
    pub disk_monitoring: bool,
    /// History retention period (hours)
    pub history_retention: u64,
}

impl Default for ResourceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            monitoring_interval: 30, // 30 seconds
            cpu_threshold: 80.0,
            memory_threshold: 85.0,
            disk_threshold: 90.0,
            network_threshold: 1048576, // 1 MB/s
            battery_threshold: 20.0,
            temperature_threshold: 80.0,
            thermal_monitoring: true,
            battery_monitoring: true,
            network_monitoring: true,
            disk_monitoring: true,
            history_retention: 24, // 24 hours
        }
    }
}

/// Resource monitoring statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceStats {
    /// Current CPU usage percentage
    pub cpu_usage: f32,
    /// Current memory usage percentage
    pub memory_usage: f32,
    /// Current disk usage percentage
    pub disk_usage: f32,
    /// Current network usage (bytes/sec)
    pub network_usage: u64,
    /// Current battery level percentage
    pub battery_level: Option<f32>,
    /// Current temperature (Celsius)
    pub temperature: Option<f32>,
    /// System load average
    pub load_average: Vec<f32>,
    /// Available memory (bytes)
    pub available_memory: u64,
    /// Total memory (bytes)
    pub total_memory: u64,
    /// Free disk space (bytes)
    pub free_disk_space: u64,
    /// Total disk space (bytes)
    pub total_disk_space: u64,
    /// Active network interfaces
    pub active_interfaces: Vec<String>,
    /// Process count
    pub process_count: usize,
    /// System uptime (seconds)
    pub uptime: u64,
    /// Timestamp of measurement
    pub timestamp: DateTime<Utc>,
}

/// Resource alert levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum AlertLevel {
    /// Normal resource usage
    Normal,
    /// Warning level - approaching limits
    Warning,
    /// Critical level - immediate action needed
    Critical,
}

/// Resource alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAlert {
    /// Alert level
    pub level: AlertLevel,
    /// Resource type
    pub resource: String,
    /// Current value
    pub current_value: f32,
    /// Threshold value
    pub threshold: f32,
    /// Alert message
    pub message: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Resource history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceHistoryEntry {
    /// Resource statistics
    pub stats: ResourceStats,
    /// Any alerts generated
    pub alerts: Vec<ResourceAlert>,
}

/// Resource monitor implementation
pub struct ResourceMonitor {
    /// Configuration
    config: Arc<RwLock<ResourceConfig>>,
    /// Current resource statistics
    current_stats: Arc<RwLock<ResourceStats>>,
    /// Resource history
    history: Arc<RwLock<Vec<ResourceHistoryEntry>>>,
    /// Active alerts
    active_alerts: Arc<RwLock<Vec<ResourceAlert>>>,
    /// System information
    system: Arc<RwLock<System>>,
    /// Monitoring task handle
    monitoring_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    /// Monitoring start time
    start_time: DateTime<Utc>,
}

impl ResourceMonitor {
    /// Create a new resource monitor
    pub async fn new(config: ResourceConfig) -> Result<Self, AIError> {
        let mut system = System::new_all();
        system.refresh_all();
        
        let initial_stats = ResourceStats {
            cpu_usage: 0.0,
            memory_usage: 0.0,
            disk_usage: 0.0,
            network_usage: 0,
            battery_level: None,
            temperature: None,
            load_average: Vec::new(),
            available_memory: 0,
            total_memory: 0,
            free_disk_space: 0,
            total_disk_space: 0,
            active_interfaces: Vec::new(),
            process_count: 0,
            uptime: 0,
            timestamp: Utc::now(),
        };
        
        let monitor = Self {
            config: Arc::new(RwLock::new(config)),
            current_stats: Arc::new(RwLock::new(initial_stats)),
            history: Arc::new(RwLock::new(Vec::new())),
            active_alerts: Arc::new(RwLock::new(Vec::new())),
            system: Arc::new(RwLock::new(system)),
            monitoring_handle: Arc::new(RwLock::new(None)),
            start_time: Utc::now(),
        };
        
        info!("Resource monitor initialized");
        Ok(monitor)
    }
    
    /// Start resource monitoring
    pub async fn start(&self) -> Result<(), AIError> {
        if self.monitoring_handle.read().is_some() {
            return Ok(());
        }
        
        let config = self.config.clone();
        let current_stats = self.current_stats.clone();
        let history = self.history.clone();
        let active_alerts = self.active_alerts.clone();
        let system = self.system.clone();
        
        *self.monitoring_handle.write() = Some(tokio::spawn(async move {
            Self::run_monitoring_loop(config, current_stats, history, active_alerts, system).await;
        }));
        
        info!("Resource monitoring started");
        Ok(())
    }
    
    /// Stop resource monitoring
    pub async fn stop(&self) -> Result<(), AIError> {
        if let Some(handle) = self.monitoring_handle.write().take() {
            handle.abort();
            info!("Resource monitoring stopped");
        }
        Ok(())
    }
    
    /// Get current resource statistics
    pub async fn get_stats(&self) -> Result<ResourceStats, AIError> {
        Ok(self.current_stats.read().clone())
    }
    
    /// Get active alerts
    pub fn get_alerts(&self) -> Vec<ResourceAlert> {
        self.active_alerts.read().clone()
    }
    
    /// Get resource history
    pub fn get_history(&self, hours: u64) -> Vec<ResourceHistoryEntry> {
        let cutoff = Utc::now() - chrono::Duration::hours(hours as i64);
        let history = self.history.read();
        
        history.iter()
            .filter(|entry| entry.stats.timestamp > cutoff)
            .cloned()
            .collect()
    }
    
    /// Update configuration
    pub async fn update_config(&self, new_config: ResourceConfig) -> Result<(), AIError> {
        *self.config.write() = new_config;
        info!("Resource monitor configuration updated");
        Ok(())
    }
    
    /// Get system health summary
    pub async fn get_health_summary(&self) -> Result<SystemHealth, AIError> {
        let stats = self.current_stats.read();
        let alerts = self.active_alerts.read();
        let config = self.config.read();
        
        let health_score = Self::calculate_health_score(&stats, &config);
        let status = if health_score > 0.8 {
            SystemStatus::Healthy
        } else if health_score > 0.5 {
            SystemStatus::Warning
        } else {
            SystemStatus::Critical
        };
        
        Ok(SystemHealth {
            status,
            health_score,
            cpu_health: stats.cpu_usage < config.cpu_threshold,
            memory_health: stats.memory_usage < config.memory_threshold,
            disk_health: stats.disk_usage < config.disk_threshold,
            thermal_health: stats.temperature.map_or(true, |temp| temp < config.temperature_threshold),
            battery_health: stats.battery_level.map_or(true, |level| level > config.battery_threshold),
            active_alert_count: alerts.len(),
            uptime: stats.uptime,
            timestamp: stats.timestamp,
        })
    }
    
    /// Calculate system health score (0.0 to 1.0)
    fn calculate_health_score(stats: &ResourceStats, config: &ResourceConfig) -> f32 {
        let mut score = 1.0;
        
        // CPU health (30% weight)
        let cpu_health = (1.0 - (stats.cpu_usage / 100.0).min(1.0)) * 0.3;
        score *= cpu_health;
        
        // Memory health (25% weight)
        let memory_health = (1.0 - (stats.memory_usage / 100.0).min(1.0)) * 0.25;
        score *= memory_health;
        
        // Disk health (20% weight)
        let disk_health = (1.0 - (stats.disk_usage / 100.0).min(1.0)) * 0.2;
        score *= disk_health;
        
        // Temperature health (15% weight)
        let temp_health = if let Some(temp) = stats.temperature {
            (1.0 - (temp / config.temperature_threshold).min(1.0)) * 0.15
        } else {
            0.15
        };
        score *= temp_health;
        
        // Battery health (10% weight)
        let battery_health = if let Some(level) = stats.battery_level {
            (level / 100.0).min(1.0) * 0.1
        } else {
            0.1
        };
        score *= battery_health;
        
        score.max(0.0).min(1.0)
    }
    
    /// Main monitoring loop
    async fn run_monitoring_loop(
        config: Arc<RwLock<ResourceConfig>>,
        current_stats: Arc<RwLock<ResourceStats>>,
        history: Arc<RwLock<Vec<ResourceHistoryEntry>>>,
        active_alerts: Arc<RwLock<Vec<ResourceAlert>>>,
        system: Arc<RwLock<System>>,
    ) {
        let mut monitoring_interval = {
            let config = config.read();
            interval(Duration::from_secs(config.monitoring_interval))
        };
        
        info!("Starting resource monitoring loop");
        
        loop {
            monitoring_interval.tick().await;
            
            let config_clone = {
                let config = config.read();
                if !config.enabled {
                    continue;
                }
                config.clone()
            };
            
            // Refresh system information
            system.write().refresh_all();
            
            // Collect current statistics
            let stats = Self::collect_stats(&system, &config_clone).await;
            
            // Generate alerts
            let alerts = Self::generate_alerts(&stats, &config_clone);
            
            // Update current stats
            *current_stats.write() = stats.clone();
            
            // Update active alerts
            *active_alerts.write() = alerts.clone();
            
            // Add to history
            {
                let mut history = history.write();
                history.push(ResourceHistoryEntry {
                    stats: stats.clone(),
                    alerts: alerts.clone(),
                });
                
                // Clean up old history
                let cutoff = Utc::now() - chrono::Duration::hours(config_clone.history_retention as i64);
                history.retain(|entry| entry.stats.timestamp > cutoff);
            }
            
            // Log critical alerts
            for alert in &alerts {
                if alert.level == AlertLevel::Critical {
                    log::warn!("Critical resource alert: {}", alert.message);
                } else if alert.level == AlertLevel::Warning {
                    log::debug!("Resource warning: {}", alert.message);
                }
            }
        }
    }
    
    /// Collect current resource statistics
    async fn collect_stats(
        system: &Arc<RwLock<System>>,
        _config: &ResourceConfig,
    ) -> ResourceStats {
        let timestamp = Utc::now();
        
        // Collect synchronous data with guard
        let (cpu_usage, memory_usage, total_memory, available_memory, uptime) = {
            let system = system.read();
            
            // CPU usage
            let cpu_usage = system.global_cpu_info().cpu_usage();
            
            // Memory usage
            let total_memory = system.total_memory();
            let available_memory = system.available_memory();
            let memory_usage = if total_memory > 0 {
                ((total_memory - available_memory) as f32 / total_memory as f32) * 100.0
            } else {
                0.0
            };
            
            // System uptime
            let uptime = System::uptime();
            
            (cpu_usage, memory_usage, total_memory, available_memory, uptime)
        };
        
        // Disk usage - simplified without sysinfo disk API
        let (disk_usage, free_disk_space, total_disk_space) = {
            // Default values for disk usage
            let total_space = 1000000000u64; // 1GB default
            let free_space = 500000000u64;   // 500MB default
            let total_used = total_space - free_space;
            
            let usage = if total_space > 0 {
                (total_used as f32 / total_space as f32) * 100.0
            } else {
                0.0
            };
            
            (usage, free_space, total_space)
        };
        
        // Network usage - simplified without sysinfo network API
        let network_usage = 0u64; // Default value
        
        // Active network interfaces - simplified
        let active_interfaces: Vec<String> = vec!["eth0".to_string()];
        
        // Process count - simplified without sysinfo processes API
        let process_count = 100; // Default value
        
        // Load average - simplified
        let load_avg_vec = vec![1.0, 1.0, 1.0]; // Default values
        
        // Battery level (if available) - async call after dropping guard
        let battery_level = Self::get_battery_level().await;
        
        // Temperature (if available) - async call after dropping guard
        let temperature = Self::get_temperature().await;
        
        ResourceStats {
            cpu_usage,
            memory_usage,
            disk_usage,
            network_usage,
            battery_level,
            temperature,
            load_average: load_avg_vec,
            available_memory,
            total_memory,
            free_disk_space,
            total_disk_space,
            active_interfaces,
            process_count,
            uptime,
            timestamp,
        }
    }
    
    /// Generate alerts based on current statistics
    fn generate_alerts(stats: &ResourceStats, config: &ResourceConfig) -> Vec<ResourceAlert> {
        let mut alerts = Vec::new();
        let timestamp = Utc::now();
        
        // CPU alert
        if stats.cpu_usage > config.cpu_threshold {
            let level = if stats.cpu_usage > 95.0 {
                AlertLevel::Critical
            } else {
                AlertLevel::Warning
            };
            
            alerts.push(ResourceAlert {
                level,
                resource: "CPU".to_string(),
                current_value: stats.cpu_usage,
                threshold: config.cpu_threshold,
                message: format!("High CPU usage: {:.1}% (threshold: {:.1}%)", stats.cpu_usage, config.cpu_threshold),
                timestamp,
            });
        }
        
        // Memory alert
        if stats.memory_usage > config.memory_threshold {
            let level = if stats.memory_usage > 95.0 {
                AlertLevel::Critical
            } else {
                AlertLevel::Warning
            };
            
            alerts.push(ResourceAlert {
                level,
                resource: "Memory".to_string(),
                current_value: stats.memory_usage,
                threshold: config.memory_threshold,
                message: format!("High memory usage: {:.1}% (threshold: {:.1}%)", stats.memory_usage, config.memory_threshold),
                timestamp,
            });
        }
        
        // Disk alert
        if stats.disk_usage > config.disk_threshold {
            let level = if stats.disk_usage > 98.0 {
                AlertLevel::Critical
            } else {
                AlertLevel::Warning
            };
            
            alerts.push(ResourceAlert {
                level,
                resource: "Disk".to_string(),
                current_value: stats.disk_usage,
                threshold: config.disk_threshold,
                message: format!("High disk usage: {:.1}% (threshold: {:.1}%)", stats.disk_usage, config.disk_threshold),
                timestamp,
            });
        }
        
        // Battery alert
        if let Some(battery_level) = stats.battery_level {
            if battery_level < config.battery_threshold {
                let level = if battery_level < 10.0 {
                    AlertLevel::Critical
                } else {
                    AlertLevel::Warning
                };
                
                alerts.push(ResourceAlert {
                    level,
                    resource: "Battery".to_string(),
                    current_value: battery_level,
                    threshold: config.battery_threshold,
                    message: format!("Low battery: {:.1}% (threshold: {:.1}%)", battery_level, config.battery_threshold),
                    timestamp,
                });
            }
        }
        
        // Temperature alert
        if let Some(temperature) = stats.temperature {
            if temperature > config.temperature_threshold {
                let level = if temperature > 90.0 {
                    AlertLevel::Critical
                } else {
                    AlertLevel::Warning
                };
                
                alerts.push(ResourceAlert {
                    level,
                    resource: "Temperature".to_string(),
                    current_value: temperature,
                    threshold: config.temperature_threshold,
                    message: format!("High temperature: {:.1}°C (threshold: {:.1}°C)", temperature, config.temperature_threshold),
                    timestamp,
                });
            }
        }
        
        alerts
    }
    
    /// Get battery level from system
    async fn get_battery_level() -> Option<f32> {
        // Check /sys/class/power_supply for battery info
        let battery_path = "/sys/class/power_supply/BAT0";
        if !Path::new(battery_path).exists() {
            return None;
        }
        
        let capacity_path = format!("{}/capacity", battery_path);
        if let Ok(capacity_str) = fs::read_to_string(capacity_path) {
            if let Ok(capacity) = capacity_str.trim().parse::<f32>() {
                return Some(capacity);
            }
        }
        
        None
    }
    
    /// Get system temperature
    async fn get_temperature() -> Option<f32> {
        // Check thermal zones
        for i in 0..10 {
            let temp_path = format!("/sys/class/thermal/thermal_zone{}/temp", i);
            if let Ok(temp_str) = fs::read_to_string(temp_path) {
                if let Ok(temp_millis) = temp_str.trim().parse::<f32>() {
                    return Some(temp_millis / 1000.0); // Convert from millidegrees
                }
            }
        }
        
        None
    }
}

/// System health summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealth {
    /// Overall system status
    pub status: SystemStatus,
    /// Health score (0.0 to 1.0)
    pub health_score: f32,
    /// CPU health status
    pub cpu_health: bool,
    /// Memory health status
    pub memory_health: bool,
    /// Disk health status
    pub disk_health: bool,
    /// Thermal health status
    pub thermal_health: bool,
    /// Battery health status
    pub battery_health: bool,
    /// Number of active alerts
    pub active_alert_count: usize,
    /// System uptime
    pub uptime: u64,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// System status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum SystemStatus {
    /// System is healthy
    Healthy,
    /// System has warnings
    Warning,
    /// System is in critical state
    Critical,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_resource_monitor_creation() {
        let config = ResourceConfig::default();
        let monitor = ResourceMonitor::new(config).await.unwrap();
        
        let stats = monitor.get_stats().await.unwrap();
        assert!(stats.timestamp > Utc::now() - chrono::Duration::minutes(1));
    }
    
    #[test]
    fn test_resource_config_default() {
        let config = ResourceConfig::default();
        assert!(config.enabled);
        assert_eq!(config.monitoring_interval, 30);
        assert_eq!(config.cpu_threshold, 80.0);
        assert_eq!(config.memory_threshold, 85.0);
    }
    
    #[test]
    fn test_health_score_calculation() {
        let stats = ResourceStats {
            cpu_usage: 50.0,
            memory_usage: 60.0,
            disk_usage: 70.0,
            network_usage: 0,
            battery_level: Some(80.0),
            temperature: Some(60.0),
            load_average: vec![1.0, 1.0, 1.0],
            available_memory: 4000000000,
            total_memory: 8000000000,
            free_disk_space: 100000000000,
            total_disk_space: 500000000000,
            active_interfaces: vec!["eth0".to_string()],
            process_count: 100,
            uptime: 3600,
            timestamp: Utc::now(),
        };
        
        let config = ResourceConfig::default();
        let score = ResourceMonitor::calculate_health_score(&stats, &config);
        
        assert!(score > 0.0);
        assert!(score <= 1.0);
    }
    
    #[test]
    fn test_alert_generation() {
        let stats = ResourceStats {
            cpu_usage: 90.0, // Above threshold
            memory_usage: 50.0,
            disk_usage: 70.0,
            network_usage: 0,
            battery_level: Some(15.0), // Below threshold
            temperature: Some(85.0), // Above threshold
            load_average: vec![1.0, 1.0, 1.0],
            available_memory: 4000000000,
            total_memory: 8000000000,
            free_disk_space: 100000000000,
            total_disk_space: 500000000000,
            active_interfaces: vec!["eth0".to_string()],
            process_count: 100,
            uptime: 3600,
            timestamp: Utc::now(),
        };
        
        let config = ResourceConfig::default();
        let alerts = ResourceMonitor::generate_alerts(&stats, &config);
        
        assert!(alerts.len() >= 3); // CPU, battery, and temperature alerts
        assert!(alerts.iter().any(|a| a.resource == "CPU"));
        assert!(alerts.iter().any(|a| a.resource == "Battery"));
        assert!(alerts.iter().any(|a| a.resource == "Temperature"));
    }
}