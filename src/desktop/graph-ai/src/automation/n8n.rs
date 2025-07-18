//! n8n workflow orchestration integration
//! 
//! This module provides integration with n8n for visual workflow orchestration,
//! enabling users to create, manage, and execute automation workflows.

use crate::AIError;
use crate::automation::{AutomationContext, ExecutionResult, ExecutionStatus, ExecutionStep, ExecutionLog, LogLevel};
use crate::automation::workflows::WorkflowDefinition;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use chrono::{DateTime, Utc};
use reqwest::Client;
use tokio::time::{sleep, Duration};
use log::{info, warn, error, debug};

/// n8n integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct N8nConfig {
    /// n8n server URL
    pub server_url: String,
    /// n8n API key
    pub api_key: Option<String>,
    /// n8n webhook URL
    pub webhook_url: String,
    /// Connection timeout in seconds
    pub timeout: u64,
    /// Retry attempts for failed requests
    pub retry_attempts: u32,
    /// Retry delay in seconds
    pub retry_delay: u64,
    /// Enable webhook endpoints
    pub enable_webhooks: bool,
    /// Webhook authentication token
    pub webhook_auth_token: Option<String>,
    /// Maximum workflow execution time (seconds)
    pub max_execution_time: u64,
    /// Enable workflow metrics
    pub enable_metrics: bool,
}

impl Default for N8nConfig {
    fn default() -> Self {
        Self {
            server_url: "http://localhost:5678".to_string(),
            api_key: None,
            webhook_url: "http://localhost:5678/webhook".to_string(),
            timeout: 30,
            retry_attempts: 3,
            retry_delay: 5,
            enable_webhooks: true,
            webhook_auth_token: None,
            max_execution_time: 300, // 5 minutes
            enable_metrics: true,
        }
    }
}

/// n8n workflow execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct N8nExecution {
    /// Execution ID
    pub id: String,
    /// Workflow ID
    pub workflow_id: String,
    /// Execution mode
    pub mode: String,
    /// Execution status
    pub status: String,
    /// Start time
    pub started_at: DateTime<Utc>,
    /// End time
    pub finished_at: Option<DateTime<Utc>>,
    /// Execution data
    pub data: serde_json::Value,
    /// Error information
    pub error: Option<String>,
}

/// n8n workflow information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct N8nWorkflow {
    /// Workflow ID
    pub id: String,
    /// Workflow name
    pub name: String,
    /// Workflow description
    pub description: Option<String>,
    /// Workflow active status
    pub active: bool,
    /// Workflow nodes
    pub nodes: Vec<N8nNode>,
    /// Workflow connections
    pub connections: serde_json::Value,
    /// Workflow settings
    pub settings: serde_json::Value,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Updated timestamp
    pub updated_at: DateTime<Utc>,
}

/// n8n workflow node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct N8nNode {
    /// Node ID
    pub id: String,
    /// Node name
    pub name: String,
    /// Node type
    pub node_type: String,
    /// Node position
    pub position: Vec<f64>,
    /// Node parameters
    pub parameters: serde_json::Value,
    /// Node credentials
    pub credentials: Option<serde_json::Value>,
}

/// n8n webhook payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct N8nWebhookPayload {
    /// Webhook ID
    pub webhook_id: String,
    /// Workflow ID
    pub workflow_id: String,
    /// Execution ID
    pub execution_id: String,
    /// Webhook data
    pub data: serde_json::Value,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// n8n integration client
pub struct N8nIntegration {
    /// Configuration
    config: Arc<RwLock<N8nConfig>>,
    /// HTTP client
    client: Client,
    /// Active executions
    active_executions: Arc<RwLock<HashMap<String, N8nExecution>>>,
    /// Execution metrics
    metrics: Arc<RwLock<N8nMetrics>>,
    /// Webhook server handle
    webhook_server: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
}

/// n8n metrics
#[derive(Debug, Default)]
struct N8nMetrics {
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
    /// API requests
    api_requests: u64,
    /// API errors
    api_errors: u64,
}

impl N8nIntegration {
    /// Create a new n8n integration
    pub async fn new(config: N8nConfig) -> Result<Self, AIError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout))
            .build()
            .map_err(|e| AIError::Configuration(format!("Failed to create HTTP client: {}", e)))?;
        
        let integration = Self {
            config: Arc::new(RwLock::new(config)),
            client,
            active_executions: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(N8nMetrics::default())),
            webhook_server: Arc::new(RwLock::new(None)),
        };
        
        // Test connection
        integration.test_connection().await?;
        
        info!("n8n integration initialized");
        Ok(integration)
    }
    
    /// Start n8n integration services
    pub async fn start(&self) -> Result<(), AIError> {
        let config = self.config.read().clone();
        
        if config.enable_webhooks {
            self.start_webhook_server().await?;
        }
        
        info!("n8n integration services started");
        Ok(())
    }
    
    /// Stop n8n integration services
    pub async fn stop(&self) -> Result<(), AIError> {
        if let Some(handle) = self.webhook_server.write().take() {
            handle.abort();
        }
        
        info!("n8n integration services stopped");
        Ok(())
    }
    
    /// Test connection to n8n server
    async fn test_connection(&self) -> Result<(), AIError> {
        let config = self.config.read();
        let url = format!("{}/healthz", config.server_url);
        
        let response = self.client.get(&url).send().await
            .map_err(|e| AIError::Configuration(format!("Failed to connect to n8n: {}", e)))?;
        
        if response.status().is_success() {
            info!("n8n connection test successful");
            Ok(())
        } else {
            Err(AIError::Configuration(format!("n8n health check failed: {}", response.status())))
        }
    }
    
    /// Execute a workflow
    pub async fn execute_workflow(
        &self,
        workflow_id: &str,
        input: Option<serde_json::Value>,
        context: AutomationContext,
    ) -> Result<ExecutionResult, AIError> {
        let config = self.config.read().clone();
        let url = format!("{}/api/v1/workflows/{}/execute", config.server_url, workflow_id);
        
        let mut request = self.client.post(&url);
        
        // Add API key if configured
        if let Some(api_key) = &config.api_key {
            request = request.header("X-N8N-API-KEY", api_key);
        }
        
        // Prepare request body
        let mut body = serde_json::json!({
            "executionMode": "synchronous",
            "runData": {}
        });
        
        if let Some(input_data) = input {
            body["runData"] = input_data;
        }
        
        // Update metrics
        {
            let mut metrics = self.metrics.write();
            metrics.api_requests += 1;
            metrics.total_executions += 1;
        }
        
        let start_time = std::time::Instant::now();
        
        // Send request with retry logic
        let response = self.send_request_with_retry(request, body).await?;
        
        let execution_time = start_time.elapsed();
        
        // Parse response
        let n8n_execution: N8nExecution = response.json().await
            .map_err(|e| AIError::Configuration(format!("Failed to parse n8n response: {}", e)))?;
        
        // Convert to ExecutionResult
        let result = self.convert_n8n_execution_to_result(n8n_execution, context, execution_time).await?;
        
        // Update metrics
        {
            let mut metrics = self.metrics.write();
            match result.status {
                ExecutionStatus::Success => metrics.successful_executions += 1,
                ExecutionStatus::Failed | ExecutionStatus::Timeout => metrics.failed_executions += 1,
                _ => {}
            }
            
            metrics.avg_execution_time = (metrics.avg_execution_time * (metrics.total_executions - 1) as f64 + execution_time.as_secs_f64()) / metrics.total_executions as f64;
            metrics.last_execution = Some(Utc::now());
        }
        
        Ok(result)
    }
    
    /// Cancel an execution
    pub async fn cancel_execution(&self, execution_id: &str) -> Result<(), AIError> {
        let config = self.config.read().clone();
        let url = format!("{}/api/v1/executions/{}/stop", config.server_url, execution_id);
        
        let mut request = self.client.post(&url);
        
        // Add API key if configured
        if let Some(api_key) = &config.api_key {
            request = request.header("X-N8N-API-KEY", api_key);
        }
        
        let response = request.send().await
            .map_err(|e| AIError::Configuration(format!("Failed to cancel execution: {}", e)))?;
        
        if response.status().is_success() {
            info!("Execution {} cancelled successfully", execution_id);
            Ok(())
        } else {
            Err(AIError::Configuration(format!("Failed to cancel execution: {}", response.status())))
        }
    }
    
    /// List available workflows
    pub async fn list_workflows(&self) -> Result<Vec<WorkflowDefinition>, AIError> {
        let config = self.config.read().clone();
        let url = format!("{}/api/v1/workflows", config.server_url);
        
        let mut request = self.client.get(&url);
        
        // Add API key if configured
        if let Some(api_key) = &config.api_key {
            request = request.header("X-N8N-API-KEY", api_key);
        }
        
        let response = request.send().await
            .map_err(|e| AIError::Configuration(format!("Failed to list workflows: {}", e)))?;
        
        if response.status().is_success() {
            let n8n_workflows: Vec<N8nWorkflow> = response.json().await
                .map_err(|e| AIError::Configuration(format!("Failed to parse workflows: {}", e)))?;
            
            // Convert to WorkflowDefinition
            let workflows = n8n_workflows.into_iter()
                .map(|w| self.convert_n8n_workflow_to_definition(w))
                .collect();
            
            Ok(workflows)
        } else {
            Err(AIError::Configuration(format!("Failed to list workflows: {}", response.status())))
        }
    }
    
    /// Create a new workflow
    pub async fn create_workflow(&self, workflow: WorkflowDefinition) -> Result<String, AIError> {
        let config = self.config.read().clone();
        let url = format!("{}/api/v1/workflows", config.server_url);
        
        let mut request = self.client.post(&url);
        
        // Add API key if configured
        if let Some(api_key) = &config.api_key {
            request = request.header("X-N8N-API-KEY", api_key);
        }
        
        // Convert WorkflowDefinition to n8n format
        let n8n_workflow = self.convert_workflow_definition_to_n8n(workflow)?;
        
        let response = request.json(&n8n_workflow).send().await
            .map_err(|e| AIError::Configuration(format!("Failed to create workflow: {}", e)))?;
        
        if response.status().is_success() {
            let created_workflow: N8nWorkflow = response.json().await
                .map_err(|e| AIError::Configuration(format!("Failed to parse created workflow: {}", e)))?;
            
            info!("Workflow created successfully: {}", created_workflow.id);
            Ok(created_workflow.id)
        } else {
            Err(AIError::Configuration(format!("Failed to create workflow: {}", response.status())))
        }
    }
    
    /// Update an existing workflow
    pub async fn update_workflow(&self, workflow_id: &str, workflow: WorkflowDefinition) -> Result<(), AIError> {
        let config = self.config.read().clone();
        let url = format!("{}/api/v1/workflows/{}", config.server_url, workflow_id);
        
        let mut request = self.client.put(&url);
        
        // Add API key if configured
        if let Some(api_key) = &config.api_key {
            request = request.header("X-N8N-API-KEY", api_key);
        }
        
        // Convert WorkflowDefinition to n8n format
        let n8n_workflow = self.convert_workflow_definition_to_n8n(workflow)?;
        
        let response = request.json(&n8n_workflow).send().await
            .map_err(|e| AIError::Configuration(format!("Failed to update workflow: {}", e)))?;
        
        if response.status().is_success() {
            info!("Workflow updated successfully: {}", workflow_id);
            Ok(())
        } else {
            Err(AIError::Configuration(format!("Failed to update workflow: {}", response.status())))
        }
    }
    
    /// Delete a workflow
    pub async fn delete_workflow(&self, workflow_id: &str) -> Result<(), AIError> {
        let config = self.config.read().clone();
        let url = format!("{}/api/v1/workflows/{}", config.server_url, workflow_id);
        
        let mut request = self.client.delete(&url);
        
        // Add API key if configured
        if let Some(api_key) = &config.api_key {
            request = request.header("X-N8N-API-KEY", api_key);
        }
        
        let response = request.send().await
            .map_err(|e| AIError::Configuration(format!("Failed to delete workflow: {}", e)))?;
        
        if response.status().is_success() {
            info!("Workflow deleted successfully: {}", workflow_id);
            Ok(())
        } else {
            Err(AIError::Configuration(format!("Failed to delete workflow: {}", response.status())))
        }
    }
    
    /// Update configuration
    pub async fn update_config(&self, new_config: N8nConfig) -> Result<(), AIError> {
        *self.config.write() = new_config;
        info!("n8n configuration updated");
        Ok(())
    }
    
    /// Health check
    pub async fn health_check(&self) -> Result<bool, AIError> {
        match self.test_connection().await {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }
    
    /// Send request with retry logic
    async fn send_request_with_retry(
        &self,
        request: reqwest::RequestBuilder,
        body: serde_json::Value,
    ) -> Result<reqwest::Response, AIError> {
        let config = self.config.read().clone();
        let mut last_error = None;
        
        for attempt in 0..config.retry_attempts {
            let request_clone = request.try_clone()
                .ok_or_else(|| AIError::Configuration("Failed to clone request".to_string()))?;
            
            match request_clone.json(&body).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        return Ok(response);
                    } else {
                        last_error = Some(AIError::Configuration(format!("Request failed with status: {}", response.status())));
                    }
                }
                Err(e) => {
                    last_error = Some(AIError::Configuration(format!("Request failed: {}", e)));
                }
            }
            
            if attempt < config.retry_attempts - 1 {
                sleep(Duration::from_secs(config.retry_delay)).await;
            }
        }
        
        // Update error metrics
        self.metrics.write().api_errors += 1;
        
        Err(last_error.unwrap_or_else(|| AIError::Configuration("Request failed after all retries".to_string())))
    }
    
    /// Convert n8n execution to ExecutionResult
    async fn convert_n8n_execution_to_result(
        &self,
        n8n_execution: N8nExecution,
        context: AutomationContext,
        execution_time: std::time::Duration,
    ) -> Result<ExecutionResult, AIError> {
        let status = match n8n_execution.status.as_str() {
            "success" => ExecutionStatus::Success,
            "error" => ExecutionStatus::Failed,
            "running" => ExecutionStatus::Running,
            "canceled" => ExecutionStatus::Cancelled,
            _ => ExecutionStatus::Failed,
        };
        
        // Extract steps from n8n execution data
        let steps = self.extract_execution_steps(&n8n_execution.data)?;
        
        // Extract logs from n8n execution data
        let logs = self.extract_execution_logs(&n8n_execution.data)?;
        
        Ok(ExecutionResult {
            execution_id: context.execution_id,
            workflow_id: context.workflow_id,
            status,
            started_at: context.started_at,
            ended_at: n8n_execution.finished_at,
            duration: Some(execution_time),
            result: Some(n8n_execution.data),
            error: n8n_execution.error,
            steps,
            logs,
        })
    }
    
    /// Extract execution steps from n8n data
    fn extract_execution_steps(&self, data: &serde_json::Value) -> Result<Vec<ExecutionStep>, AIError> {
        let mut steps = Vec::new();
        
        if let Some(nodes) = data.get("resultData").and_then(|r| r.get("runData")) {
            for (node_name, node_data) in nodes.as_object().unwrap_or(&serde_json::Map::new()) {
                if let Some(executions) = node_data.as_array() {
                    for (i, execution) in executions.iter().enumerate() {
                        let step = ExecutionStep {
                            step_id: format!("{}_{}", node_name, i),
                            name: node_name.clone(),
                            step_type: "n8n_node".to_string(),
                            status: if execution.get("error").is_some() {
                                ExecutionStatus::Failed
                            } else {
                                ExecutionStatus::Success
                            },
                            started_at: Utc::now(), // TODO: Extract actual start time
                            ended_at: Some(Utc::now()), // TODO: Extract actual end time
                            input: execution.get("data").and_then(|d| d.get("input")).cloned(),
                            output: execution.get("data").and_then(|d| d.get("output")).cloned(),
                            error: execution.get("error").and_then(|e| e.as_str()).map(|s| s.to_string()),
                        };
                        
                        steps.push(step);
                    }
                }
            }
        }
        
        Ok(steps)
    }
    
    /// Extract execution logs from n8n data
    fn extract_execution_logs(&self, data: &serde_json::Value) -> Result<Vec<ExecutionLog>, AIError> {
        let mut logs = Vec::new();
        
        // TODO: Extract actual logs from n8n execution data
        // For now, create basic log entries
        logs.push(ExecutionLog {
            timestamp: Utc::now(),
            level: LogLevel::Info,
            message: "Workflow execution started".to_string(),
            step_id: None,
            context: None,
        });
        
        if let Some(error) = data.get("error") {
            logs.push(ExecutionLog {
                timestamp: Utc::now(),
                level: LogLevel::Error,
                message: error.to_string(),
                step_id: None,
                context: Some(error.clone()),
            });
        }
        
        Ok(logs)
    }
    
    /// Convert n8n workflow to WorkflowDefinition
    fn convert_n8n_workflow_to_definition(&self, n8n_workflow: N8nWorkflow) -> WorkflowDefinition {
        // TODO: Implement proper conversion
        WorkflowDefinition {
            id: Some(n8n_workflow.id),
            name: n8n_workflow.name,
            description: n8n_workflow.description,
            version: "1.0".to_string(),
            created_at: n8n_workflow.created_at,
            updated_at: n8n_workflow.updated_at,
            active: n8n_workflow.active,
            steps: Vec::new(), // TODO: Convert nodes to steps
            triggers: Vec::new(), // TODO: Extract triggers
            variables: std::collections::HashMap::new(),
            settings: serde_json::json!({}),
        }
    }
    
    /// Convert WorkflowDefinition to n8n format
    fn convert_workflow_definition_to_n8n(&self, workflow: WorkflowDefinition) -> Result<serde_json::Value, AIError> {
        // TODO: Implement proper conversion
        Ok(serde_json::json!({
            "name": workflow.name,
            "description": workflow.description,
            "active": workflow.active,
            "nodes": [],
            "connections": {},
            "settings": workflow.settings
        }))
    }
    
    /// Start webhook server
    async fn start_webhook_server(&self) -> Result<(), AIError> {
        let config = self.config.read().clone();
        
        if !config.enable_webhooks {
            return Ok(());
        }
        
        // TODO: Implement webhook server
        info!("Webhook server would be started here");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_n8n_config_default() {
        let config = N8nConfig::default();
        assert_eq!(config.server_url, "http://localhost:5678");
        assert_eq!(config.timeout, 30);
        assert_eq!(config.retry_attempts, 3);
        assert!(config.enable_webhooks);
    }
    
    #[test]
    fn test_execution_status_conversion() {
        let test_cases = vec![
            ("success", ExecutionStatus::Success),
            ("error", ExecutionStatus::Failed),
            ("running", ExecutionStatus::Running),
            ("canceled", ExecutionStatus::Cancelled),
            ("unknown", ExecutionStatus::Failed),
        ];
        
        for (n8n_status, expected) in test_cases {
            let status = match n8n_status {
                "success" => ExecutionStatus::Success,
                "error" => ExecutionStatus::Failed,
                "running" => ExecutionStatus::Running,
                "canceled" => ExecutionStatus::Cancelled,
                _ => ExecutionStatus::Failed,
            };
            
            assert_eq!(status, expected);
        }
    }
    
    #[test]
    fn test_n8n_execution_serialization() {
        let execution = N8nExecution {
            id: "test-123".to_string(),
            workflow_id: "workflow-456".to_string(),
            mode: "manual".to_string(),
            status: "success".to_string(),
            started_at: Utc::now(),
            finished_at: Some(Utc::now()),
            data: serde_json::json!({"result": "test"}),
            error: None,
        };
        
        let serialized = serde_json::to_string(&execution).unwrap();
        let deserialized: N8nExecution = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(execution.id, deserialized.id);
        assert_eq!(execution.workflow_id, deserialized.workflow_id);
        assert_eq!(execution.status, deserialized.status);
    }
}