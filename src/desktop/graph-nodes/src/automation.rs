//! Automation node implementation for scripts, workflows, and automated tasks

use crate::{
    GraphNode, NodeVisualData, NodeAction, NodeActionResult, NodeActionType, NodeError, NodeExportData
};
use horizonos_graph_engine::{SceneNode, SceneId, NodeMetadata};
use horizonos_graph_engine::scene::{NodeType, AutomationType, AutomationStatus};
use nalgebra::Vector3;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

/// Automation node representing scripts, workflows, and automated processes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationNode {
    /// Node ID
    pub id: SceneId,
    /// Automation name
    pub name: String,
    /// Type of automation
    pub automation_type: AutomationType,
    /// Current status
    pub status: AutomationStatus,
    /// Script or workflow content
    pub content: String,
    /// Programming language or workflow engine
    pub language: String,
    /// Execution parameters
    pub parameters: HashMap<String, String>,
    /// Environment variables
    pub environment: HashMap<String, String>,
    /// Trigger conditions
    pub triggers: Vec<AutomationTrigger>,
    /// Execution schedule
    pub schedule: Option<AutomationSchedule>,
    /// Last execution time
    pub last_run: Option<DateTime<Utc>>,
    /// Next scheduled run
    pub next_run: Option<DateTime<Utc>>,
    /// Execution count
    pub run_count: u64,
    /// Success count
    pub success_count: u64,
    /// Failure count
    pub failure_count: u64,
    /// Average execution time in milliseconds
    pub avg_execution_time: u64,
    /// Last execution result
    pub last_result: Option<ExecutionResult>,
    /// Dependencies (other automations that must run first)
    pub dependencies: Vec<SceneId>,
    /// Timeout in seconds
    pub timeout: u64,
    /// Retry configuration
    pub retry_config: RetryConfig,
    /// Notification settings
    pub notifications: NotificationConfig,
    /// Logging configuration
    pub logging: LoggingConfig,
    /// Associated files or resources
    pub resources: Vec<String>,
    /// Documentation or help text
    pub documentation: Option<String>,
    /// Version information
    pub version: String,
    /// Author information
    pub author: Option<String>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Node metadata
    pub metadata: NodeMetadata,
    /// Visual data for rendering
    pub visual_data: NodeVisualData,
}

/// Trigger condition for automation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationTrigger {
    /// Trigger type
    pub trigger_type: TriggerType,
    /// Trigger condition
    pub condition: String,
    /// Enabled state
    pub enabled: bool,
}

/// Types of automation triggers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TriggerType {
    /// File system event
    FileSystem { path: String, event: String },
    /// Time-based trigger
    Time { cron: String },
    /// HTTP webhook
    Webhook { endpoint: String },
    /// System event
    System { event: String },
    /// Database event
    Database { query: String },
    /// API call
    Api { endpoint: String, method: String },
    /// Custom trigger
    Custom { name: String },
}

/// Automation schedule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationSchedule {
    /// Cron expression
    pub cron: String,
    /// Timezone
    pub timezone: String,
    /// Enabled state
    pub enabled: bool,
    /// Maximum concurrent executions
    pub max_concurrent: u32,
    /// Overlap policy
    pub overlap_policy: OverlapPolicy,
}

/// Policy for handling overlapping executions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OverlapPolicy {
    /// Allow overlapping executions
    Allow,
    /// Skip new execution if one is running
    Skip,
    /// Terminate running execution and start new one
    Terminate,
    /// Queue new execution
    Queue,
}

/// Execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Success status
    pub success: bool,
    /// Exit code
    pub exit_code: Option<i32>,
    /// Execution time in milliseconds
    pub execution_time: u64,
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// Start time
    pub start_time: DateTime<Utc>,
    /// End time
    pub end_time: DateTime<Utc>,
    /// Error message (if failed)
    pub error: Option<String>,
}

/// Retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Enable retries
    pub enabled: bool,
    /// Maximum retry attempts
    pub max_attempts: u32,
    /// Retry delay in seconds
    pub delay: u64,
    /// Exponential backoff
    pub exponential_backoff: bool,
    /// Backoff multiplier
    pub backoff_multiplier: f32,
}

/// Notification configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    /// Notify on success
    pub on_success: bool,
    /// Notify on failure
    pub on_failure: bool,
    /// Notification methods
    pub methods: Vec<NotificationMethod>,
}

/// Notification method
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationMethod {
    /// Email notification
    Email { address: String },
    /// Slack notification
    Slack { channel: String },
    /// Discord notification
    Discord { webhook: String },
    /// System notification
    System,
    /// Custom webhook
    Webhook { url: String },
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Enable logging
    pub enabled: bool,
    /// Log level
    pub level: LogLevel,
    /// Log file path
    pub file_path: Option<String>,
    /// Rotate logs
    pub rotate: bool,
    /// Maximum log file size in bytes
    pub max_size: u64,
    /// Maximum number of log files
    pub max_files: u32,
}

/// Log levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl AutomationNode {
    /// Create a new automation node
    pub fn new(id: SceneId, name: String, automation_type: AutomationType, content: String, language: String) -> Self {
        // Set appropriate color based on automation type
        let color = match automation_type {
            AutomationType::Script => [0.4, 0.8, 0.4, 1.0],      // Green
            AutomationType::Workflow => [0.2, 0.6, 0.9, 1.0],    // Blue
            AutomationType::Trigger => [0.9, 0.6, 0.2, 1.0],     // Orange
            AutomationType::Schedule => [0.7, 0.3, 0.9, 1.0],    // Purple
            AutomationType::Rule => [0.9, 0.3, 0.3, 1.0],        // Red
            AutomationType::Macro => [0.3, 0.9, 0.7, 1.0],       // Teal
            AutomationType::Service => [0.6, 0.6, 0.6, 1.0],     // Gray
            AutomationType::Integration => [0.9, 0.7, 0.3, 1.0], // Yellow
        };
        
        let mut visual_data = NodeVisualData::default();
        visual_data.color = color;
        visual_data.radius = 1.3;
        visual_data.icon = Some(Self::get_icon_for_type(&automation_type));
        
        let mut metadata = NodeMetadata::default();
        metadata.description = Some(format!("{:?} automation", automation_type));
        metadata.tags = vec!["automation".to_string(), format!("{:?}", automation_type).to_lowercase()];
        
        Self {
            id,
            name,
            automation_type,
            status: AutomationStatus::Inactive,
            content,
            language,
            parameters: HashMap::new(),
            environment: HashMap::new(),
            triggers: Vec::new(),
            schedule: None,
            last_run: None,
            next_run: None,
            run_count: 0,
            success_count: 0,
            failure_count: 0,
            avg_execution_time: 0,
            last_result: None,
            dependencies: Vec::new(),
            timeout: 300, // 5 minutes default
            retry_config: RetryConfig::default(),
            notifications: NotificationConfig::default(),
            logging: LoggingConfig::default(),
            resources: Vec::new(),
            documentation: None,
            version: "1.0.0".to_string(),
            author: None,
            tags: Vec::new(),
            metadata,
            visual_data,
        }
    }
    
    /// Get icon name for automation type
    fn get_icon_for_type(automation_type: &AutomationType) -> String {
        match automation_type {
            AutomationType::Script => "code".to_string(),
            AutomationType::Workflow => "flow".to_string(),
            AutomationType::Trigger => "zap".to_string(),
            AutomationType::Schedule => "clock".to_string(),
            AutomationType::Rule => "shield".to_string(),
            AutomationType::Macro => "repeat".to_string(),
            AutomationType::Service => "server".to_string(),
            AutomationType::Integration => "link".to_string(),
        }
    }
    
    /// Execute the automation
    pub fn execute(&mut self) -> Result<(), NodeError> {
        self.status = AutomationStatus::Running;
        self.run_count += 1;
        self.last_run = Some(Utc::now());
        
        // Update visual indicator
        self.visual_data.glow = true;
        self.visual_data.color[3] = 0.8; // Slightly transparent when running
        
        // TODO: Implement actual execution logic based on automation type
        // For now, simulate execution
        let execution_time = 1000; // 1 second
        
        let result = ExecutionResult {
            success: true,
            exit_code: Some(0),
            execution_time,
            stdout: "Automation executed successfully".to_string(),
            stderr: String::new(),
            start_time: Utc::now(),
            end_time: Utc::now(),
            error: None,
        };
        
        self.finish_execution(result)?;
        
        Ok(())
    }
    
    /// Finish execution with result
    fn finish_execution(&mut self, result: ExecutionResult) -> Result<(), NodeError> {
        let success = result.success;
        let execution_time = result.execution_time;
        
        self.last_result = Some(result);
        
        if success {
            self.success_count += 1;
            self.status = AutomationStatus::Active;
        } else {
            self.failure_count += 1;
            self.status = AutomationStatus::Failed;
        }
        
        // Update average execution time
        if self.run_count > 0 {
            self.avg_execution_time = (self.avg_execution_time * (self.run_count - 1) + execution_time) / self.run_count;
        }
        
        // Reset visual indicators
        self.visual_data.glow = false;
        self.visual_data.color[3] = 1.0;
        
        // Update status color
        match self.status {
            AutomationStatus::Active => {
                self.visual_data.badge = Some("check".to_string());
            }
            AutomationStatus::Failed => {
                self.visual_data.badge = Some("x".to_string());
            }
            _ => {
                self.visual_data.badge = None;
            }
        }
        
        self.last_run = Some(Utc::now());
        
        Ok(())
    }
    
    /// Pause the automation
    pub fn pause(&mut self) -> Result<(), NodeError> {
        match self.status {
            AutomationStatus::Active | AutomationStatus::Running => {
                self.status = AutomationStatus::Paused;
                self.visual_data.badge = Some("pause".to_string());
                Ok(())
            }
            _ => Err(NodeError::InvalidAction {
                action: NodeAction::Custom {
                    action_type: "pause".to_string(),
                    parameters: HashMap::new(),
                },
            }),
        }
    }
    
    /// Resume the automation
    pub fn resume(&mut self) -> Result<(), NodeError> {
        match self.status {
            AutomationStatus::Paused => {
                self.status = AutomationStatus::Active;
                self.visual_data.badge = None;
                Ok(())
            }
            _ => Err(NodeError::InvalidAction {
                action: NodeAction::Custom {
                    action_type: "resume".to_string(),
                    parameters: HashMap::new(),
                },
            }),
        }
    }
    
    /// Stop the automation
    pub fn stop(&mut self) -> Result<(), NodeError> {
        self.status = AutomationStatus::Inactive;
        self.visual_data.badge = None;
        self.visual_data.glow = false;
        Ok(())
    }
    
    /// Add a trigger
    pub fn add_trigger(&mut self, trigger: AutomationTrigger) {
        self.triggers.push(trigger);
    }
    
    /// Remove a trigger
    pub fn remove_trigger(&mut self, index: usize) -> Result<(), NodeError> {
        if index < self.triggers.len() {
            self.triggers.remove(index);
            Ok(())
        } else {
            Err(NodeError::SystemError {
                message: "Invalid trigger index".to_string(),
            })
        }
    }
    
    /// Set schedule
    pub fn set_schedule(&mut self, schedule: AutomationSchedule) {
        self.schedule = Some(schedule);
        // TODO: Calculate next run time from cron expression
    }
    
    /// Get success rate
    pub fn success_rate(&self) -> f32 {
        if self.run_count > 0 {
            self.success_count as f32 / self.run_count as f32
        } else {
            0.0
        }
    }
    
    /// Get status display
    pub fn status_display(&self) -> String {
        match self.status {
            AutomationStatus::Active => "Active".to_string(),
            AutomationStatus::Inactive => "Inactive".to_string(),
            AutomationStatus::Paused => "Paused".to_string(),
            AutomationStatus::Failed => "Failed".to_string(),
            AutomationStatus::Running => "Running".to_string(),
            AutomationStatus::Scheduled => "Scheduled".to_string(),
        }
    }
}

impl GraphNode for AutomationNode {
    fn id(&self) -> SceneId {
        self.id
    }
    
    fn display_name(&self) -> String {
        self.name.clone()
    }
    
    fn description(&self) -> Option<String> {
        Some(format!("{:?} automation ({})", self.automation_type, self.status_display()))
    }
    
    fn node_type(&self) -> NodeType {
        NodeType::Automation {
            name: self.name.clone(),
            automation_type: self.automation_type.clone(),
            status: self.status.clone(),
        }
    }
    
    fn metadata(&self) -> NodeMetadata {
        self.metadata.clone()
    }
    
    fn visual_data(&self) -> NodeVisualData {
        self.visual_data.clone()
    }
    
    fn update(&mut self, _delta_time: f32) -> Result<(), NodeError> {
        // TODO: Check for triggers and schedule
        Ok(())
    }
    
    fn handle_action(&mut self, action: NodeAction) -> Result<NodeActionResult, NodeError> {
        match action {
            NodeAction::Open => {
                Ok(NodeActionResult::Success {
                    message: Some("Opening automation editor".to_string()),
                })
            }
            NodeAction::Edit => {
                Ok(NodeActionResult::Success {
                    message: Some("Editing automation properties".to_string()),
                })
            }
            NodeAction::Delete => {
                Ok(NodeActionResult::ConfirmationRequired {
                    prompt: format!("Delete automation: {}?", self.name),
                })
            }
            NodeAction::Custom { action_type, parameters } => {
                match action_type.as_str() {
                    "execute" => {
                        self.execute()?;
                        Ok(NodeActionResult::Success {
                            message: Some("Automation executed".to_string()),
                        })
                    }
                    "pause" => {
                        self.pause()?;
                        Ok(NodeActionResult::Success {
                            message: Some("Automation paused".to_string()),
                        })
                    }
                    "resume" => {
                        self.resume()?;
                        Ok(NodeActionResult::Success {
                            message: Some("Automation resumed".to_string()),
                        })
                    }
                    "stop" => {
                        self.stop()?;
                        Ok(NodeActionResult::Success {
                            message: Some("Automation stopped".to_string()),
                        })
                    }
                    "add_parameter" => {
                        if let (Some(key), Some(value)) = (parameters.get("key"), parameters.get("value")) {
                            self.parameters.insert(key.clone(), value.clone());
                            Ok(NodeActionResult::Success {
                                message: Some(format!("Added parameter: {} = {}", key, value)),
                            })
                        } else {
                            Ok(NodeActionResult::Error {
                                error: "Key and value parameters required".to_string(),
                            })
                        }
                    }
                    "view_logs" => {
                        Ok(NodeActionResult::Success {
                            message: Some("Opening automation logs".to_string()),
                        })
                    }
                    _ => Ok(NodeActionResult::Error {
                        error: format!("Unknown action: {}", action_type),
                    }),
                }
            }
            _ => Ok(NodeActionResult::Error {
                error: "Action not supported for automation nodes".to_string(),
            }),
        }
    }
    
    fn available_actions(&self) -> Vec<NodeActionType> {
        vec![
            NodeActionType::Open,
            NodeActionType::Edit,
            NodeActionType::Delete,
            NodeActionType::Custom("execute".to_string()),
            NodeActionType::Custom("pause".to_string()),
            NodeActionType::Custom("resume".to_string()),
            NodeActionType::Custom("stop".to_string()),
            NodeActionType::Custom("add_parameter".to_string()),
            NodeActionType::Custom("view_logs".to_string()),
        ]
    }
    
    fn export_data(&self) -> Result<NodeExportData, NodeError> {
        Ok(NodeExportData {
            node_type: "Automation".to_string(),
            display_name: self.display_name(),
            description: self.description(),
            visual_data: self.visual_data(),
            metadata: self.metadata.clone(),
            type_specific_data: serde_json::to_value(self)?,
        })
    }
    
    fn to_scene_node(&self) -> SceneNode {
        SceneNode {
            id: self.id,
            position: self.visual_data.position.into(),
            velocity: Vector3::zeros(),
            radius: self.visual_data.radius,
            color: self.visual_data.color,
            node_type: NodeType::Automation {
                name: self.name.clone(),
                automation_type: self.automation_type.clone(),
                status: self.status.clone(),
            },
            metadata: self.metadata.clone(),
            visible: self.visual_data.visible,
            selected: self.visual_data.selected,
        }
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_attempts: 3,
            delay: 5,
            exponential_backoff: true,
            backoff_multiplier: 2.0,
        }
    }
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            on_success: false,
            on_failure: true,
            methods: vec![NotificationMethod::System],
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            level: LogLevel::Info,
            file_path: None,
            rotate: true,
            max_size: 10_000_000, // 10MB
            max_files: 5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_automation_node_creation() {
        let automation = AutomationNode::new(
            1,
            "Test Script".to_string(),
            AutomationType::Script,
            "#!/bin/bash\necho 'Hello World'".to_string(),
            "bash".to_string(),
        );
        
        assert_eq!(automation.id(), 1);
        assert_eq!(automation.name, "Test Script");
        assert_eq!(automation.language, "bash");
        assert!(matches!(automation.status, AutomationStatus::Inactive));
    }
    
    #[test]
    fn test_automation_execution() {
        let mut automation = AutomationNode::new(
            1,
            "Test Script".to_string(),
            AutomationType::Script,
            "echo 'test'".to_string(),
            "bash".to_string(),
        );
        
        assert_eq!(automation.run_count, 0);
        assert_eq!(automation.success_count, 0);
        
        automation.execute().unwrap();
        
        assert_eq!(automation.run_count, 1);
        assert_eq!(automation.success_count, 1);
        assert!(matches!(automation.status, AutomationStatus::Active));
        assert_eq!(automation.success_rate(), 1.0);
    }
    
    #[test]
    fn test_automation_pause_resume() {
        let mut automation = AutomationNode::new(
            1,
            "Test Script".to_string(),
            AutomationType::Script,
            "echo 'test'".to_string(),
            "bash".to_string(),
        );
        
        // Set to active first
        automation.status = AutomationStatus::Active;
        
        automation.pause().unwrap();
        assert!(matches!(automation.status, AutomationStatus::Paused));
        
        automation.resume().unwrap();
        assert!(matches!(automation.status, AutomationStatus::Active));
    }
    
    #[test]
    fn test_automation_triggers() {
        let mut automation = AutomationNode::new(
            1,
            "Test Script".to_string(),
            AutomationType::Script,
            "echo 'test'".to_string(),
            "bash".to_string(),
        );
        
        let trigger = AutomationTrigger {
            trigger_type: TriggerType::Time { cron: "0 0 * * *".to_string() },
            condition: "daily".to_string(),
            enabled: true,
        };
        
        automation.add_trigger(trigger);
        assert_eq!(automation.triggers.len(), 1);
        
        automation.remove_trigger(0).unwrap();
        assert_eq!(automation.triggers.len(), 0);
    }
}