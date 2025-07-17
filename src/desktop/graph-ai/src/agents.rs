//! AI agents for various tasks

use crate::{AIError, ollama::{OllamaClient, GenerateOptions}};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Base trait for AI agents
#[async_trait]
pub trait AIAgent: Send + Sync {
    /// Get the agent's name
    fn name(&self) -> &str;
    
    /// Get the agent's description
    fn description(&self) -> &str;
    
    /// Process a request
    async fn process(&self, request: AgentRequest) -> Result<AgentResponse, AIError>;
    
    /// Check if the agent can handle a specific task
    fn can_handle(&self, task: &str) -> bool;
}

/// Agent request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRequest {
    /// Task to perform
    pub task: String,
    /// Input data
    pub input: serde_json::Value,
    /// Additional context
    pub context: Option<serde_json::Value>,
}

/// Agent response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResponse {
    /// Success status
    pub success: bool,
    /// Result data
    pub result: serde_json::Value,
    /// Error message if failed
    pub error: Option<String>,
}

/// Code assistant agent
pub struct CodeAssistant {
    model: String,
    client: Arc<OllamaClient>,
}

impl CodeAssistant {
    pub fn new(model: String, client: Arc<OllamaClient>) -> Self {
        Self { model, client }
    }
}

#[async_trait]
impl AIAgent for CodeAssistant {
    fn name(&self) -> &str {
        "Code Assistant"
    }
    
    fn description(&self) -> &str {
        "Helps with code generation, refactoring, and debugging"
    }
    
    async fn process(&self, request: AgentRequest) -> Result<AgentResponse, AIError> {
        let prompt = match request.task.as_str() {
            "code" => format!(
                "Generate code based on the following request:\n\n{}\n\nContext: {}",
                request.input.as_str().unwrap_or(""),
                request.context.as_ref().map(|c| c.to_string()).unwrap_or_default()
            ),
            "refactor" => format!(
                "Refactor the following code to improve it:\n\n{}\n\nContext: {}",
                request.input.as_str().unwrap_or(""),
                request.context.as_ref().map(|c| c.to_string()).unwrap_or_default()
            ),
            "debug" => format!(
                "Help debug the following code and identify issues:\n\n{}\n\nContext: {}",
                request.input.as_str().unwrap_or(""),
                request.context.as_ref().map(|c| c.to_string()).unwrap_or_default()
            ),
            "explain" => format!(
                "Explain the following code in detail:\n\n{}\n\nContext: {}",
                request.input.as_str().unwrap_or(""),
                request.context.as_ref().map(|c| c.to_string()).unwrap_or_default()
            ),
            _ => return Err(AIError::UnsupportedOperation(format!("Unknown task: {}", request.task))),
        };
        
        let options = GenerateOptions {
            temperature: Some(0.7),
            top_p: Some(0.9),
            ..Default::default()
        };
        
        match self.client.generate(&self.model, &prompt, Some(options)).await {
            Ok(response) => Ok(AgentResponse {
                success: true,
                result: serde_json::json!({
                    "response": response,
                    "task": request.task,
                }),
                error: None,
            }),
            Err(e) => Ok(AgentResponse {
                success: false,
                result: serde_json::json!({}),
                error: Some(e.to_string()),
            }),
        }
    }
    
    fn can_handle(&self, task: &str) -> bool {
        matches!(task, "code" | "refactor" | "debug" | "explain")
    }
}

/// Document summarizer agent
pub struct DocumentSummarizer {
    model: String,
    client: Arc<OllamaClient>,
}

impl DocumentSummarizer {
    pub fn new(model: String, client: Arc<OllamaClient>) -> Self {
        Self { model, client }
    }
}

#[async_trait]
impl AIAgent for DocumentSummarizer {
    fn name(&self) -> &str {
        "Document Summarizer"
    }
    
    fn description(&self) -> &str {
        "Summarizes documents and extracts key information"
    }
    
    async fn process(&self, request: AgentRequest) -> Result<AgentResponse, AIError> {
        let prompt = format!(
            "Please summarize the following document:\n\n{}\n\nProvide a concise summary highlighting the key points.",
            request.input.as_str().unwrap_or("")
        );
        
        let options = GenerateOptions {
            temperature: Some(0.5),
            top_p: Some(0.9),
            ..Default::default()
        };
        
        match self.client.generate(&self.model, &prompt, Some(options)).await {
            Ok(response) => Ok(AgentResponse {
                success: true,
                result: serde_json::json!({
                    "summary": response,
                    "document_length": request.input.as_str().unwrap_or("").len(),
                }),
                error: None,
            }),
            Err(e) => Ok(AgentResponse {
                success: false,
                result: serde_json::json!({}),
                error: Some(e.to_string()),
            }),
        }
    }
    
    fn can_handle(&self, task: &str) -> bool {
        matches!(task, "summarize" | "extract" | "tldr")
    }
}

/// Workflow automation agent
pub struct WorkflowAgent {
    model: String,
    client: Arc<OllamaClient>,
}

impl WorkflowAgent {
    pub fn new(model: String, client: Arc<OllamaClient>) -> Self {
        Self { model, client }
    }
}

#[async_trait]
impl AIAgent for WorkflowAgent {
    fn name(&self) -> &str {
        "Workflow Automation"
    }
    
    fn description(&self) -> &str {
        "Creates and optimizes workflows based on user patterns"
    }
    
    async fn process(&self, request: AgentRequest) -> Result<AgentResponse, AIError> {
        let prompt = format!(
            "Create an automated workflow for the following task:\n\n{}\n\nProvide step-by-step instructions that can be automated.",
            request.input.as_str().unwrap_or("")
        );
        
        let options = GenerateOptions {
            temperature: Some(0.6),
            top_p: Some(0.9),
            ..Default::default()
        };
        
        match self.client.generate(&self.model, &prompt, Some(options)).await {
            Ok(response) => Ok(AgentResponse {
                success: true,
                result: serde_json::json!({
                    "workflow": response,
                    "task": request.task,
                }),
                error: None,
            }),
            Err(e) => Ok(AgentResponse {
                success: false,
                result: serde_json::json!({}),
                error: Some(e.to_string()),
            }),
        }
    }
    
    fn can_handle(&self, task: &str) -> bool {
        matches!(task, "automate" | "workflow" | "optimize")
    }
}