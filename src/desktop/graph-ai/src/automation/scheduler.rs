//! Workflow scheduler for automation
//! 
//! This module provides scheduling capabilities for automation workflows,
//! including cron-like scheduling, interval-based scheduling, and event-driven scheduling.

use crate::AIError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use chrono::{DateTime, Utc, TimeZone};
use tokio::time::{sleep, Duration};
use tokio::sync::mpsc;
use log::{info, warn, error, debug};
use cron::Schedule as CronSchedule;
use std::str::FromStr;

/// Scheduler configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    /// Enable scheduler
    pub enabled: bool,
    /// Maximum concurrent scheduled workflows
    pub max_concurrent_workflows: usize,
    /// Check interval for schedules (seconds)
    pub check_interval: u64,
    /// Enable persistent scheduling
    pub persistent: bool,
    /// Schedule storage file
    pub schedule_file: Option<String>,
    /// Default timezone for schedules
    pub default_timezone: String,
    /// Enable schedule notifications
    pub enable_notifications: bool,
    /// Notification webhook URL
    pub notification_webhook: Option<String>,
    /// Schedule history retention (days)
    pub history_retention_days: u32,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_concurrent_workflows: 10,
            check_interval: 60, // Check every minute
            persistent: true,
            schedule_file: Some("/tmp/horizonos/schedules.json".to_string()),
            default_timezone: "UTC".to_string(),
            enable_notifications: false,
            notification_webhook: None,
            history_retention_days: 30,
        }
    }
}

/// Schedule definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schedule {
    /// Schedule ID
    pub id: String,
    /// Schedule name
    pub name: String,
    /// Schedule description
    pub description: Option<String>,
    /// Schedule type
    pub schedule_type: ScheduleType,
    /// Workflow ID to execute
    pub workflow_id: String,
    /// User ID who created the schedule
    pub user_id: String,
    /// Schedule enabled status
    pub enabled: bool,
    /// Schedule created time
    pub created_at: DateTime<Utc>,
    /// Schedule updated time
    pub updated_at: DateTime<Utc>,
    /// Next execution time
    pub next_execution: Option<DateTime<Utc>>,
    /// Last execution time
    pub last_execution: Option<DateTime<Utc>>,
    /// Execution count
    pub execution_count: u64,
    /// Maximum executions (None for unlimited)
    pub max_executions: Option<u64>,
    /// Schedule configuration
    pub config: ScheduleConfig,
    /// Schedule metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Schedule types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScheduleType {
    /// One-time execution at specific time
    Once,
    /// Recurring execution with interval
    Interval,
    /// Cron-based scheduling
    Cron,
    /// Event-based scheduling
    Event,
    /// Manual execution only
    Manual,
}

/// Schedule configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleConfig {
    /// Cron expression (for Cron type)
    pub cron_expression: Option<String>,
    /// Interval in seconds (for Interval type)
    pub interval_seconds: Option<u64>,
    /// Specific execution time (for Once type)
    pub execution_time: Option<DateTime<Utc>>,
    /// Event type (for Event type)
    pub event_type: Option<String>,
    /// Event filter (for Event type)
    pub event_filter: Option<serde_json::Value>,
    /// Timezone for schedule
    pub timezone: Option<String>,
    /// Execution window (start and end times)
    pub execution_window: Option<(String, String)>,
    /// Days of week (0 = Sunday, 6 = Saturday)
    pub days_of_week: Option<Vec<u8>>,
    /// Execution timeout (seconds)
    pub timeout: Option<u64>,
    /// Retry configuration
    pub retry_config: Option<RetryConfig>,
}

/// Retry configuration for scheduled executions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum retry attempts
    pub max_attempts: u32,
    /// Delay between retries (seconds)
    pub delay_seconds: u64,
    /// Exponential backoff factor
    pub backoff_factor: f64,
    /// Maximum delay between retries
    pub max_delay_seconds: u64,
}

/// Scheduled execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledExecutionResult {
    /// Schedule ID
    pub schedule_id: String,
    /// Execution ID
    pub execution_id: String,
    /// Workflow ID
    pub workflow_id: String,
    /// Execution status
    pub status: ExecutionStatus,
    /// Scheduled time
    pub scheduled_at: DateTime<Utc>,
    /// Actual execution time
    pub executed_at: DateTime<Utc>,
    /// Execution completion time
    pub completed_at: Option<DateTime<Utc>>,
    /// Execution duration
    pub duration: Option<std::time::Duration>,
    /// Execution result
    pub result: Option<serde_json::Value>,
    /// Error message
    pub error: Option<String>,
    /// Retry attempts
    pub retry_attempts: u32,
}

/// Execution status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExecutionStatus {
    /// Scheduled for execution
    Scheduled,
    /// Currently executing
    Running,
    /// Completed successfully
    Success,
    /// Failed
    Failed,
    /// Cancelled
    Cancelled,
    /// Timed out
    Timeout,
    /// Retrying
    Retrying,
}

/// Workflow scheduler
pub struct WorkflowScheduler {
    /// Configuration
    config: Arc<RwLock<SchedulerConfig>>,
    /// Active schedules
    schedules: Arc<RwLock<HashMap<String, Schedule>>>,
    /// Execution history
    execution_history: Arc<RwLock<Vec<ScheduledExecutionResult>>>,
    /// Scheduler task handle
    scheduler_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    /// Execution sender
    execution_sender: mpsc::UnboundedSender<ScheduledExecutionResult>,
    /// Statistics
    stats: Arc<RwLock<SchedulerStats>>,
}

/// Scheduler statistics
#[derive(Debug, Default)]
pub struct SchedulerStats {
    /// Total schedules created
    total_schedules: u64,
    /// Active schedules
    active_schedules: u64,
    /// Total executions
    total_executions: u64,
    /// Successful executions
    successful_executions: u64,
    /// Failed executions
    failed_executions: u64,
    /// Average execution time
    avg_execution_time: f64,
    /// Last execution time
    last_execution: Option<DateTime<Utc>>,
    /// Schedules by type
    schedules_by_type: HashMap<String, u64>,
}

impl WorkflowScheduler {
    /// Create a new workflow scheduler
    pub async fn new(config: SchedulerConfig) -> Result<Self, AIError> {
        let (execution_sender, mut execution_receiver) = mpsc::unbounded_channel();
        
        let scheduler = Self {
            config: Arc::new(RwLock::new(config.clone())),
            schedules: Arc::new(RwLock::new(HashMap::new())),
            execution_history: Arc::new(RwLock::new(Vec::new())),
            scheduler_handle: Arc::new(RwLock::new(None)),
            execution_sender,
            stats: Arc::new(RwLock::new(SchedulerStats::default())),
        };
        
        // Load schedules from file if persistent
        if config.persistent {
            scheduler.load_schedules().await?;
        }
        
        // Start execution result processor
        let execution_history = scheduler.execution_history.clone();
        let stats = scheduler.stats.clone();
        tokio::spawn(async move {
            while let Some(result) = execution_receiver.recv().await {
                // Store execution result
                execution_history.write().push(result.clone());
                
                // Update statistics
                let mut stats = stats.write();
                stats.total_executions += 1;
                
                match result.status {
                    ExecutionStatus::Success => stats.successful_executions += 1,
                    ExecutionStatus::Failed | ExecutionStatus::Timeout => stats.failed_executions += 1,
                    _ => {}
                }
                
                if let Some(duration) = result.duration {
                    stats.avg_execution_time = (stats.avg_execution_time * (stats.total_executions - 1) as f64 + duration.as_secs_f64()) / stats.total_executions as f64;
                }
                
                stats.last_execution = Some(result.executed_at);
            }
        });
        
        info!("Workflow scheduler initialized");
        Ok(scheduler)
    }
    
    /// Start the scheduler
    pub async fn start(&self) -> Result<(), AIError> {
        if !self.config.read().enabled {
            return Ok(());
        }
        
        if self.scheduler_handle.read().is_some() {
            return Ok(());
        }
        
        let config = self.config.clone();
        let schedules = self.schedules.clone();
        let execution_sender = self.execution_sender.clone();
        
        *self.scheduler_handle.write() = Some(tokio::spawn(async move {
            Self::scheduler_loop(config, schedules, execution_sender).await;
        }));
        
        info!("Workflow scheduler started");
        Ok(())
    }
    
    /// Stop the scheduler
    pub async fn stop(&self) -> Result<(), AIError> {
        if let Some(handle) = self.scheduler_handle.write().take() {
            handle.abort();
        }
        
        // Save schedules if persistent
        if self.config.read().persistent {
            self.save_schedules().await?;
        }
        
        info!("Workflow scheduler stopped");
        Ok(())
    }
    
    /// Schedule a workflow
    pub async fn schedule_workflow(
        &self,
        workflow_id: &str,
        schedule: Schedule,
        user_id: &str,
    ) -> Result<String, AIError> {
        let mut schedule = schedule;
        schedule.workflow_id = workflow_id.to_string();
        schedule.user_id = user_id.to_string();
        schedule.id = uuid::Uuid::new_v4().to_string();
        schedule.created_at = Utc::now();
        schedule.updated_at = Utc::now();
        
        // Calculate next execution time
        schedule.next_execution = self.calculate_next_execution(&schedule)?;
        
        // Validate schedule
        self.validate_schedule(&schedule)?;
        
        // Store schedule
        let schedule_id = schedule.id.clone();
        self.schedules.write().insert(schedule_id.clone(), schedule.clone());
        
        // Update statistics
        let mut stats = self.stats.write();
        stats.total_schedules += 1;
        stats.active_schedules += 1;
        
        let schedule_type_key = format!("{:?}", schedule.schedule_type);
        stats.schedules_by_type.entry(schedule_type_key).and_modify(|e| *e += 1).or_insert(1);
        
        // Save schedules if persistent
        if self.config.read().persistent {
            self.save_schedules().await?;
        }
        
        info!("Workflow scheduled: {} -> {}", schedule_id, workflow_id);
        Ok(schedule_id)
    }
    
    /// Update a schedule
    pub async fn update_schedule(&self, schedule_id: &str, mut schedule: Schedule) -> Result<(), AIError> {
        if !self.schedules.read().contains_key(schedule_id) {
            return Err(AIError::Configuration(format!("Schedule not found: {}", schedule_id)));
        }
        
        schedule.id = schedule_id.to_string();
        schedule.updated_at = Utc::now();
        
        // Calculate next execution time
        schedule.next_execution = self.calculate_next_execution(&schedule)?;
        
        // Validate schedule
        self.validate_schedule(&schedule)?;
        
        // Update schedule
        self.schedules.write().insert(schedule_id.to_string(), schedule);
        
        // Save schedules if persistent
        if self.config.read().persistent {
            self.save_schedules().await?;
        }
        
        info!("Schedule updated: {}", schedule_id);
        Ok(())
    }
    
    /// Delete a schedule
    pub async fn delete_schedule(&self, schedule_id: &str) -> Result<(), AIError> {
        if self.schedules.write().remove(schedule_id).is_some() {
            // Update statistics
            let mut stats = self.stats.write();
            stats.active_schedules = stats.active_schedules.saturating_sub(1);
            
            // Save schedules if persistent
            if self.config.read().persistent {
                self.save_schedules().await?;
            }
            
            info!("Schedule deleted: {}", schedule_id);
            Ok(())
        } else {
            Err(AIError::Configuration(format!("Schedule not found: {}", schedule_id)))
        }
    }
    
    /// Get a schedule
    pub fn get_schedule(&self, schedule_id: &str) -> Option<Schedule> {
        self.schedules.read().get(schedule_id).cloned()
    }
    
    /// List all schedules
    pub fn list_schedules(&self) -> Vec<Schedule> {
        self.schedules.read().values().cloned().collect()
    }
    
    /// Get execution history
    pub fn get_execution_history(&self, schedule_id: Option<&str>) -> Vec<ScheduledExecutionResult> {
        let history = self.execution_history.read();
        
        if let Some(schedule_id) = schedule_id {
            history.iter()
                .filter(|r| r.schedule_id == schedule_id)
                .cloned()
                .collect()
        } else {
            history.clone()
        }
    }
    
    /// Update configuration
    pub async fn update_config(&self, new_config: SchedulerConfig) -> Result<(), AIError> {
        *self.config.write() = new_config;
        info!("Scheduler configuration updated");
        Ok(())
    }
    
    /// Health check
    pub async fn health_check(&self) -> Result<bool, AIError> {
        Ok(self.scheduler_handle.read().is_some())
    }
    
    /// Get scheduler statistics
    pub fn get_stats(&self) -> SchedulerStats {
        self.stats.read().clone()
    }
    
    /// Main scheduler loop
    async fn scheduler_loop(
        config: Arc<RwLock<SchedulerConfig>>,
        schedules: Arc<RwLock<HashMap<String, Schedule>>>,
        execution_sender: mpsc::UnboundedSender<ScheduledExecutionResult>,
    ) {
        let mut interval = tokio::time::interval(Duration::from_secs(config.read().check_interval));
        
        info!("Scheduler loop started");
        
        loop {
            interval.tick().await;
            
            if !config.read().enabled {
                continue;
            }
            
            let now = Utc::now();
            let mut schedules_to_execute = Vec::new();
            
            // Find schedules that need to be executed
            {
                let mut schedules = schedules.write();
                for (schedule_id, schedule) in schedules.iter_mut() {
                    if !schedule.enabled {
                        continue;
                    }
                    
                    if let Some(next_execution) = schedule.next_execution {
                        if next_execution <= now {
                            // Check if maximum executions reached
                            if let Some(max_executions) = schedule.max_executions {
                                if schedule.execution_count >= max_executions {
                                    schedule.enabled = false;
                                    continue;
                                }
                            }
                            
                            schedules_to_execute.push(schedule.clone());
                            
                            // Update execution count and calculate next execution
                            schedule.execution_count += 1;
                            schedule.last_execution = Some(now);
                            
                            // Calculate next execution time for recurring schedules
                            if matches!(schedule.schedule_type, ScheduleType::Interval | ScheduleType::Cron) {
                                schedule.next_execution = Self::calculate_next_execution_static(schedule).ok().flatten();
                            } else {
                                schedule.next_execution = None;
                                schedule.enabled = false; // Disable one-time schedules
                            }
                        }
                    }
                }
            }
            
            // Execute scheduled workflows
            for schedule in schedules_to_execute {
                debug!("Executing scheduled workflow: {} -> {}", schedule.id, schedule.workflow_id);
                
                // Create execution result
                let execution_result = ScheduledExecutionResult {
                    schedule_id: schedule.id.clone(),
                    execution_id: uuid::Uuid::new_v4().to_string(),
                    workflow_id: schedule.workflow_id.clone(),
                    status: ExecutionStatus::Scheduled,
                    scheduled_at: schedule.next_execution.unwrap_or(now),
                    executed_at: now,
                    completed_at: None,
                    duration: None,
                    result: None,
                    error: None,
                    retry_attempts: 0,
                };
                
                // Send execution result
                if let Err(e) = execution_sender.send(execution_result) {
                    error!("Failed to send execution result: {}", e);
                }
                
                // TODO: Actually execute the workflow
                // This would integrate with the workflow execution system
            }
        }
    }
    
    /// Calculate next execution time for a schedule
    fn calculate_next_execution(&self, schedule: &Schedule) -> Result<Option<DateTime<Utc>>, AIError> {
        Self::calculate_next_execution_static(schedule)
    }
    
    /// Calculate next execution time (static version)
    fn calculate_next_execution_static(schedule: &Schedule) -> Result<Option<DateTime<Utc>>, AIError> {
        let now = Utc::now();
        
        match schedule.schedule_type {
            ScheduleType::Once => {
                if let Some(execution_time) = schedule.config.execution_time {
                    if execution_time > now {
                        Ok(Some(execution_time))
                    } else {
                        Ok(None) // Already passed
                    }
                } else {
                    Err(AIError::Configuration("Execution time not specified for one-time schedule".to_string()))
                }
            }
            ScheduleType::Interval => {
                if let Some(interval_seconds) = schedule.config.interval_seconds {
                    Ok(Some(now + chrono::Duration::seconds(interval_seconds as i64)))
                } else {
                    Err(AIError::Configuration("Interval not specified for interval schedule".to_string()))
                }
            }
            ScheduleType::Cron => {
                if let Some(cron_expression) = &schedule.config.cron_expression {
                    let schedule_cron = CronSchedule::from_str(cron_expression)
                        .map_err(|e| AIError::Configuration(format!("Invalid cron expression: {}", e)))?;
                    
                    if let Some(next) = schedule_cron.upcoming(Utc).next() {
                        Ok(Some(next))
                    } else {
                        Ok(None)
                    }
                } else {
                    Err(AIError::Configuration("Cron expression not specified for cron schedule".to_string()))
                }
            }
            ScheduleType::Manual => Ok(None),
            ScheduleType::Event => {
                // Event-based schedules don't have predictable next execution times
                Ok(None)
            }
        }
    }
    
    /// Validate a schedule
    fn validate_schedule(&self, schedule: &Schedule) -> Result<(), AIError> {
        if schedule.workflow_id.is_empty() {
            return Err(AIError::Configuration("Workflow ID cannot be empty".to_string()));
        }
        
        if schedule.user_id.is_empty() {
            return Err(AIError::Configuration("User ID cannot be empty".to_string()));
        }
        
        // Validate schedule type specific configuration
        match schedule.schedule_type {
            ScheduleType::Once => {
                if schedule.config.execution_time.is_none() {
                    return Err(AIError::Configuration("Execution time required for one-time schedule".to_string()));
                }
            }
            ScheduleType::Interval => {
                if schedule.config.interval_seconds.is_none() {
                    return Err(AIError::Configuration("Interval required for interval schedule".to_string()));
                }
            }
            ScheduleType::Cron => {
                if let Some(cron_expression) = &schedule.config.cron_expression {
                    CronSchedule::from_str(cron_expression)
                        .map_err(|e| AIError::Configuration(format!("Invalid cron expression: {}", e)))?;
                } else {
                    return Err(AIError::Configuration("Cron expression required for cron schedule".to_string()));
                }
            }
            ScheduleType::Event => {
                if schedule.config.event_type.is_none() {
                    return Err(AIError::Configuration("Event type required for event schedule".to_string()));
                }
            }
            ScheduleType::Manual => {
                // No additional validation needed for manual schedules
            }
        }
        
        Ok(())
    }
    
    /// Load schedules from file
    async fn load_schedules(&self) -> Result<(), AIError> {
        let config = self.config.read();
        
        if let Some(schedule_file) = &config.schedule_file {
            if tokio::fs::metadata(schedule_file).await.is_ok() {
                let content = tokio::fs::read_to_string(schedule_file).await
                    .map_err(|e| AIError::Configuration(format!("Failed to read schedule file: {}", e)))?;
                
                let schedules: Vec<Schedule> = serde_json::from_str(&content)
                    .map_err(|e| AIError::Configuration(format!("Failed to parse schedule file: {}", e)))?;
                
                let mut schedule_map = self.schedules.write();
                for schedule in schedules {
                    schedule_map.insert(schedule.id.clone(), schedule);
                }
                
                info!("Loaded {} schedules from file", schedule_map.len());
            }
        }
        
        Ok(())
    }
    
    /// Save schedules to file
    async fn save_schedules(&self) -> Result<(), AIError> {
        let config = self.config.read();
        
        if let Some(schedule_file) = &config.schedule_file {
            let schedules: Vec<Schedule> = self.schedules.read().values().cloned().collect();
            
            let content = serde_json::to_string_pretty(&schedules)
                .map_err(|e| AIError::Configuration(format!("Failed to serialize schedules: {}", e)))?;
            
            // Create directory if it doesn't exist
            if let Some(parent) = std::path::Path::new(schedule_file).parent() {
                tokio::fs::create_dir_all(parent).await
                    .map_err(|e| AIError::Configuration(format!("Failed to create schedule directory: {}", e)))?;
            }
            
            tokio::fs::write(schedule_file, content).await
                .map_err(|e| AIError::Configuration(format!("Failed to write schedule file: {}", e)))?;
            
            debug!("Saved {} schedules to file", schedules.len());
        }
        
        Ok(())
    }
}

impl Clone for SchedulerStats {
    fn clone(&self) -> Self {
        Self {
            total_schedules: self.total_schedules,
            active_schedules: self.active_schedules,
            total_executions: self.total_executions,
            successful_executions: self.successful_executions,
            failed_executions: self.failed_executions,
            avg_execution_time: self.avg_execution_time,
            last_execution: self.last_execution,
            schedules_by_type: self.schedules_by_type.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    
    #[test]
    fn test_scheduler_config_default() {
        let config = SchedulerConfig::default();
        assert!(config.enabled);
        assert_eq!(config.max_concurrent_workflows, 10);
        assert_eq!(config.check_interval, 60);
        assert!(config.persistent);
        assert_eq!(config.default_timezone, "UTC");
    }
    
    #[test]
    fn test_schedule_serialization() {
        let schedule = Schedule {
            id: "test-schedule".to_string(),
            name: "Test Schedule".to_string(),
            description: Some("A test schedule".to_string()),
            schedule_type: ScheduleType::Interval,
            workflow_id: "test-workflow".to_string(),
            user_id: "test-user".to_string(),
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            next_execution: Some(Utc::now() + chrono::Duration::hours(1)),
            last_execution: None,
            execution_count: 0,
            max_executions: None,
            config: ScheduleConfig {
                cron_expression: None,
                interval_seconds: Some(3600),
                execution_time: None,
                event_type: None,
                event_filter: None,
                timezone: None,
                execution_window: None,
                days_of_week: None,
                timeout: None,
                retry_config: None,
            },
            metadata: HashMap::new(),
        };
        
        let serialized = serde_json::to_string(&schedule).unwrap();
        let deserialized: Schedule = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(schedule.id, deserialized.id);
        assert_eq!(schedule.name, deserialized.name);
        assert_eq!(schedule.workflow_id, deserialized.workflow_id);
        assert!(matches!(deserialized.schedule_type, ScheduleType::Interval));
    }
    
    #[test]
    fn test_cron_schedule_validation() {
        let schedule = Schedule {
            id: "test-cron".to_string(),
            name: "Test Cron".to_string(),
            description: None,
            schedule_type: ScheduleType::Cron,
            workflow_id: "test-workflow".to_string(),
            user_id: "test-user".to_string(),
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            next_execution: None,
            last_execution: None,
            execution_count: 0,
            max_executions: None,
            config: ScheduleConfig {
                cron_expression: Some("0 0 0 * * *".to_string()), // Daily at midnight
                interval_seconds: None,
                execution_time: None,
                event_type: None,
                event_filter: None,
                timezone: None,
                execution_window: None,
                days_of_week: None,
                timeout: None,
                retry_config: None,
            },
            metadata: HashMap::new(),
        };
        
        // Test cron expression parsing (5-field format: sec min hour day month)
        let cron_schedule = CronSchedule::from_str("0 0 0 * * *").unwrap();
        assert!(cron_schedule.upcoming(Utc).next().is_some());
    }
    
    #[test]
    fn test_execution_status_transitions() {
        let statuses = vec![
            ExecutionStatus::Scheduled,
            ExecutionStatus::Running,
            ExecutionStatus::Success,
            ExecutionStatus::Failed,
            ExecutionStatus::Cancelled,
            ExecutionStatus::Timeout,
            ExecutionStatus::Retrying,
        ];
        
        for status in statuses {
            let serialized = serde_json::to_string(&status).unwrap();
            let deserialized: ExecutionStatus = serde_json::from_str(&serialized).unwrap();
            assert_eq!(status, deserialized);
        }
    }
    
    #[test]
    fn test_schedule_config_options() {
        let config = ScheduleConfig {
            cron_expression: Some("0 0 */6 * * *".to_string()),
            interval_seconds: Some(3600),
            execution_time: Some(Utc.with_ymd_and_hms(2025, 1, 1, 12, 0, 0).unwrap()),
            event_type: Some("file_changed".to_string()),
            event_filter: Some(serde_json::json!({"path": "/tmp/test"})),
            timezone: Some("America/New_York".to_string()),
            execution_window: Some(("09:00".to_string(), "17:00".to_string())),
            days_of_week: Some(vec![1, 2, 3, 4, 5]), // Monday to Friday
            timeout: Some(300),
            retry_config: Some(RetryConfig {
                max_attempts: 3,
                delay_seconds: 60,
                backoff_factor: 2.0,
                max_delay_seconds: 600,
            }),
        };
        
        assert_eq!(config.cron_expression, Some("0 0 */6 * * *".to_string()));
        assert_eq!(config.interval_seconds, Some(3600));
        assert_eq!(config.timezone, Some("America/New_York".to_string()));
        assert_eq!(config.days_of_week, Some(vec![1, 2, 3, 4, 5]));
        assert_eq!(config.timeout, Some(300));
        assert!(config.retry_config.is_some());
    }
}