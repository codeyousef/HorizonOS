//! Automation and RPA integration for HorizonOS
//! 
//! This module provides comprehensive automation capabilities including:
//! - n8n workflow orchestration
//! - Browser automation with Playwright
//! - UI automation with ydotool
//! - Workflow pattern recognition and optimization

pub mod n8n;
pub mod browser;
pub mod ui;
pub mod workflows;
pub mod scheduler;

use crate::AIError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use chrono::{DateTime, Utc};
use tokio::sync::mpsc;
use uuid::Uuid;
use log::{info, warn, error, debug};

/// Automation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationConfig {
    /// Enable automation features
    pub enabled: bool,
    /// n8n server configuration
    pub n8n: n8n::N8nConfig,
    /// Browser automation configuration
    pub browser: browser::BrowserConfig,
    /// UI automation configuration
    pub ui: ui::UIConfig,
    /// Workflow scheduler configuration
    pub scheduler: scheduler::SchedulerConfig,
    /// Maximum concurrent workflows
    pub max_concurrent_workflows: usize,
    /// Automation timeout in seconds
    pub default_timeout: u64,
    /// Enable workflow suggestions
    pub enable_suggestions: bool,
    /// Workflow execution history retention (days)
    pub history_retention_days: u32,
}

impl Default for AutomationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            n8n: n8n::N8nConfig::default(),
            browser: browser::BrowserConfig::default(),
            ui: ui::UIConfig::default(),
            scheduler: scheduler::SchedulerConfig::default(),
            max_concurrent_workflows: 5,
            default_timeout: 300, // 5 minutes
            enable_suggestions: true,
            history_retention_days: 30,
        }
    }
}

/// Automation execution context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationContext {
    /// Execution ID
    pub execution_id: String,
    /// Workflow ID
    pub workflow_id: String,
    /// User ID
    pub user_id: String,
    /// Execution start time
    pub started_at: DateTime<Utc>,
    /// Current step
    pub current_step: usize,
    /// Execution variables
    pub variables: HashMap<String, serde_json::Value>,
    /// Execution metadata
    pub metadata: HashMap<String, String>,
}

/// Automation execution status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExecutionStatus {
    /// Execution is pending
    Pending,
    /// Execution is running
    Running,
    /// Execution completed successfully
    Success,
    /// Execution failed
    Failed,
    /// Execution was cancelled
    Cancelled,
    /// Execution timed out
    Timeout,
}

/// Automation execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Execution ID
    pub execution_id: String,
    /// Workflow ID
    pub workflow_id: String,
    /// Execution status
    pub status: ExecutionStatus,
    /// Start time
    pub started_at: DateTime<Utc>,
    /// End time
    pub ended_at: Option<DateTime<Utc>>,
    /// Execution duration
    pub duration: Option<std::time::Duration>,
    /// Result data
    pub result: Option<serde_json::Value>,
    /// Error message if failed
    pub error: Option<String>,
    /// Execution steps
    pub steps: Vec<ExecutionStep>,
    /// Execution logs
    pub logs: Vec<ExecutionLog>,
}

/// Individual execution step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStep {
    /// Step ID
    pub step_id: String,
    /// Step name
    pub name: String,
    /// Step type
    pub step_type: String,
    /// Step status
    pub status: ExecutionStatus,
    /// Start time
    pub started_at: DateTime<Utc>,
    /// End time
    pub ended_at: Option<DateTime<Utc>>,
    /// Step input
    pub input: Option<serde_json::Value>,
    /// Step output
    pub output: Option<serde_json::Value>,
    /// Step error
    pub error: Option<String>,
}

/// Execution log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionLog {
    /// Log timestamp
    pub timestamp: DateTime<Utc>,
    /// Log level
    pub level: LogLevel,
    /// Log message
    pub message: String,
    /// Step ID (if applicable)
    pub step_id: Option<String>,
    /// Additional context
    pub context: Option<serde_json::Value>,
}

/// Log levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// Automation manager that coordinates all automation systems
pub struct AutomationManager {
    /// Configuration
    config: Arc<RwLock<AutomationConfig>>,
    /// n8n integration
    n8n: Arc<n8n::N8nIntegration>,
    /// Browser automation
    browser: Arc<browser::BrowserAutomation>,
    /// UI automation
    ui: Arc<ui::UIAutomation>,
    /// Workflow scheduler
    scheduler: Arc<scheduler::WorkflowScheduler>,
    /// Active executions
    active_executions: Arc<RwLock<HashMap<String, AutomationContext>>>,
    /// Execution results
    execution_results: Arc<RwLock<HashMap<String, ExecutionResult>>>,
    /// Execution channel
    execution_tx: mpsc::UnboundedSender<ExecutionResult>,
    /// Statistics
    stats: Arc<RwLock<AutomationStats>>,
}

/// Automation statistics
#[derive(Debug, Default)]
pub struct AutomationStats {
    /// Total executions
    total_executions: u64,
    /// Successful executions
    successful_executions: u64,
    /// Failed executions
    failed_executions: u64,
    /// Average execution time
    avg_execution_time: f64,
    /// Active workflows
    active_workflows: u64,
    /// Workflows by type
    workflows_by_type: HashMap<String, u64>,
    /// Last execution time
    last_execution: Option<DateTime<Utc>>,
}

impl AutomationManager {
    /// Create a new automation manager
    pub async fn new(config: AutomationConfig) -> Result<Self, AIError> {
        let config = Arc::new(RwLock::new(config.clone()));
        
        // Initialize components
        let n8n = Arc::new(n8n::N8nIntegration::new(config.read().n8n.clone()).await?);
        let browser = Arc::new(browser::BrowserAutomation::new(config.read().browser.clone()).await?);
        let ui = Arc::new(ui::UIAutomation::new(config.read().ui.clone()).await?);
        let scheduler = Arc::new(scheduler::WorkflowScheduler::new(config.read().scheduler.clone()).await?);
        
        // Create execution channel
        let (execution_tx, mut execution_rx) = mpsc::unbounded_channel();
        
        let manager = Self {
            config: config.clone(),
            n8n,
            browser,
            ui,
            scheduler,
            active_executions: Arc::new(RwLock::new(HashMap::new())),
            execution_results: Arc::new(RwLock::new(HashMap::new())),
            execution_tx,
            stats: Arc::new(RwLock::new(AutomationStats::default())),
        };
        
        // Start execution result processor
        let execution_results = manager.execution_results.clone();
        let stats = manager.stats.clone();
        tokio::spawn(async move {
            while let Some(result) = execution_rx.recv().await {
                // Store execution result
                execution_results.write().insert(result.execution_id.clone(), result.clone());
                
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
                
                stats.last_execution = Some(result.ended_at.unwrap_or_else(Utc::now));
            }
        });
        
        info!("Automation manager initialized");
        Ok(manager)
    }
    
    /// Start automation services
    pub async fn start(&self) -> Result<(), AIError> {
        if !self.config.read().enabled {
            return Ok(());
        }
        
        // Start all automation services
        self.n8n.start().await?;
        self.browser.start().await?;
        self.ui.start().await?;
        self.scheduler.start().await?;
        
        info!("Automation services started");
        Ok(())
    }
    
    /// Stop automation services
    pub async fn stop(&self) -> Result<(), AIError> {
        // Stop all automation services
        self.n8n.stop().await?;
        self.browser.stop().await?;
        self.ui.stop().await?;
        self.scheduler.stop().await?;
        
        info!("Automation services stopped");
        Ok(())
    }
    
    /// Execute a workflow
    pub async fn execute_workflow(
        &self,
        workflow_id: &str,
        input: Option<serde_json::Value>,
        user_id: &str,
    ) -> Result<String, AIError> {
        let execution_id = Uuid::new_v4().to_string();
        
        let context = AutomationContext {
            execution_id: execution_id.clone(),
            workflow_id: workflow_id.to_string(),
            user_id: user_id.to_string(),
            started_at: Utc::now(),
            current_step: 0,
            variables: HashMap::new(),
            metadata: HashMap::new(),
        };
        
        // Store context
        self.active_executions.write().insert(execution_id.clone(), context.clone());
        
        // Start execution
        let execution_tx = self.execution_tx.clone();
        let n8n = self.n8n.clone();
        let active_executions = self.active_executions.clone();
        let execution_id_clone = execution_id.clone();
        let workflow_id_clone = workflow_id.to_string();
        
        tokio::spawn(async move {
            let result = match n8n.execute_workflow(&workflow_id_clone, input, context).await {
                Ok(result) => result,
                Err(e) => ExecutionResult {
                    execution_id: execution_id_clone.clone(),
                    workflow_id: workflow_id_clone.clone(),
                    status: ExecutionStatus::Failed,
                    started_at: Utc::now(),
                    ended_at: Some(Utc::now()),
                    duration: Some(std::time::Duration::from_secs(0)),
                    result: None,
                    error: Some(e.to_string()),
                    steps: Vec::new(),
                    logs: Vec::new(),
                },
            };
            
            // Remove from active executions
            active_executions.write().remove(&execution_id_clone);
            
            // Send result
            let _ = execution_tx.send(result);
        });
        
        Ok(execution_id)
    }
    
    /// Get execution status
    pub fn get_execution_status(&self, execution_id: &str) -> Option<ExecutionStatus> {
        // Check active executions first
        if self.active_executions.read().contains_key(execution_id) {
            return Some(ExecutionStatus::Running);
        }
        
        // Check completed executions
        self.execution_results.read()
            .get(execution_id)
            .map(|result| result.status.clone())
    }
    
    /// Get execution result
    pub fn get_execution_result(&self, execution_id: &str) -> Option<ExecutionResult> {
        self.execution_results.read()
            .get(execution_id)
            .cloned()
    }
    
    /// List active executions
    pub fn list_active_executions(&self) -> Vec<AutomationContext> {
        self.active_executions.read().values().cloned().collect()
    }
    
    /// Cancel execution
    pub async fn cancel_execution(&self, execution_id: &str) -> Result<(), AIError> {
        if let Some(context) = self.active_executions.write().remove(execution_id) {
            // Try to cancel in n8n
            self.n8n.cancel_execution(execution_id).await?;
            
            // Create cancellation result
            let result = ExecutionResult {
                execution_id: execution_id.to_string(),
                workflow_id: context.workflow_id,
                status: ExecutionStatus::Cancelled,
                started_at: context.started_at,
                ended_at: Some(Utc::now()),
                duration: Some(Utc::now().signed_duration_since(context.started_at).to_std().unwrap_or_default()),
                result: None,
                error: Some("Execution cancelled by user".to_string()),
                steps: Vec::new(),
                logs: Vec::new(),
            };
            
            // Send result
            let _ = self.execution_tx.send(result);
        }
        
        Ok(())
    }
    
    /// Get automation statistics
    pub fn get_stats(&self) -> AutomationStats {
        self.stats.read().clone()
    }
    
    /// Update configuration
    pub async fn update_config(&self, new_config: AutomationConfig) -> Result<(), AIError> {
        // Update configuration
        *self.config.write() = new_config.clone();
        
        // Update component configurations
        self.n8n.update_config(new_config.n8n).await?;
        self.browser.update_config(new_config.browser).await?;
        self.ui.update_config(new_config.ui).await?;
        self.scheduler.update_config(new_config.scheduler).await?;
        
        info!("Automation configuration updated");
        Ok(())
    }
    
    /// Get available workflows
    pub async fn list_workflows(&self) -> Result<Vec<workflows::WorkflowDefinition>, AIError> {
        self.n8n.list_workflows().await
    }
    
    /// Create a new workflow
    pub async fn create_workflow(
        &self,
        workflow: workflows::WorkflowDefinition,
    ) -> Result<String, AIError> {
        self.n8n.create_workflow(workflow).await
    }
    
    /// Update an existing workflow
    pub async fn update_workflow(
        &self,
        workflow_id: &str,
        workflow: workflows::WorkflowDefinition,
    ) -> Result<(), AIError> {
        self.n8n.update_workflow(workflow_id, workflow).await
    }
    
    /// Delete a workflow
    pub async fn delete_workflow(&self, workflow_id: &str) -> Result<(), AIError> {
        self.n8n.delete_workflow(workflow_id).await
    }
    
    /// Schedule a workflow for execution
    pub async fn schedule_workflow(
        &self,
        workflow_id: &str,
        schedule: scheduler::Schedule,
        user_id: &str,
    ) -> Result<String, AIError> {
        self.scheduler.schedule_workflow(workflow_id, schedule, user_id).await
    }
    
    /// Health check for automation services
    pub async fn health_check(&self) -> Result<AutomationHealth, AIError> {
        let n8n_health = self.n8n.health_check().await?;
        let browser_health = self.browser.health_check().await?;
        let ui_health = self.ui.health_check().await?;
        let scheduler_health = self.scheduler.health_check().await?;
        
        let overall_healthy = n8n_health && browser_health && ui_health && scheduler_health;
        
        Ok(AutomationHealth {
            healthy: overall_healthy,
            n8n_healthy: n8n_health,
            browser_healthy: browser_health,
            ui_healthy: ui_health,
            scheduler_healthy: scheduler_health,
            active_executions: self.active_executions.read().len(),
            last_check: Utc::now(),
        })
    }
}

/// Automation health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationHealth {
    /// Overall health status
    pub healthy: bool,
    /// n8n service health
    pub n8n_healthy: bool,
    /// Browser automation health
    pub browser_healthy: bool,
    /// UI automation health
    pub ui_healthy: bool,
    /// Scheduler health
    pub scheduler_healthy: bool,
    /// Number of active executions
    pub active_executions: usize,
    /// Last health check time
    pub last_check: DateTime<Utc>,
}

impl Clone for AutomationStats {
    fn clone(&self) -> Self {
        Self {
            total_executions: self.total_executions,
            successful_executions: self.successful_executions,
            failed_executions: self.failed_executions,
            avg_execution_time: self.avg_execution_time,
            active_workflows: self.active_workflows,
            workflows_by_type: self.workflows_by_type.clone(),
            last_execution: self.last_execution,
        }
    }
}

// Re-export common types
pub use workflows::WorkflowDefinition;
pub use scheduler::Schedule;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_automation_config_default() {
        let config = AutomationConfig::default();
        assert!(config.enabled);
        assert_eq!(config.max_concurrent_workflows, 5);
        assert_eq!(config.default_timeout, 300);
    }
    
    #[test]
    fn test_execution_context_creation() {
        let context = AutomationContext {
            execution_id: "test-exec-123".to_string(),
            workflow_id: "test-workflow".to_string(),
            user_id: "test-user".to_string(),
            started_at: Utc::now(),
            current_step: 0,
            variables: HashMap::new(),
            metadata: HashMap::new(),
        };
        
        assert_eq!(context.execution_id, "test-exec-123");
        assert_eq!(context.workflow_id, "test-workflow");
        assert_eq!(context.current_step, 0);
    }
    
    #[test]
    fn test_execution_status_transitions() {
        let statuses = vec![
            ExecutionStatus::Pending,
            ExecutionStatus::Running,
            ExecutionStatus::Success,
            ExecutionStatus::Failed,
            ExecutionStatus::Cancelled,
            ExecutionStatus::Timeout,
        ];
        
        for status in statuses {
            let serialized = serde_json::to_string(&status).unwrap();
            let deserialized: ExecutionStatus = serde_json::from_str(&serialized).unwrap();
            assert_eq!(status, deserialized);
        }
    }
}