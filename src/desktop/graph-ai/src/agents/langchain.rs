//! LangChain integration for AI agents
//! 
//! This module provides integration with LangChain for building intelligent AI agents
//! that can reason, plan, and execute complex tasks.

use crate::AIError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use chrono::{DateTime, Utc};
use tokio::sync::mpsc;
use log::{info, error, debug};

/// LangChain configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LangChainConfig {
    /// Enable LangChain integration
    pub enabled: bool,
    /// LLM provider configuration
    pub llm_provider: LLMProvider,
    /// Default model for agents
    pub default_model: String,
    /// Maximum tokens per request
    pub max_tokens: u32,
    /// Temperature for generation
    pub temperature: f32,
    /// Top-p for nucleus sampling
    pub top_p: f32,
    /// Agent execution timeout (seconds)
    pub execution_timeout: u64,
    /// Maximum agent execution steps
    pub max_execution_steps: u32,
    /// Enable agent memory
    pub enable_memory: bool,
    /// Memory retention period (days)
    pub memory_retention_days: u32,
    /// Enable tool usage
    pub enable_tools: bool,
    /// Available tools
    pub available_tools: Vec<String>,
    /// Enable multi-agent coordination
    pub enable_multi_agent: bool,
    /// Maximum concurrent agents
    pub max_concurrent_agents: usize,
}

impl Default for LangChainConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            llm_provider: LLMProvider::Ollama,
            default_model: "llama3.2:latest".to_string(),
            max_tokens: 4096,
            temperature: 0.7,
            top_p: 0.9,
            execution_timeout: 300,
            max_execution_steps: 50,
            enable_memory: true,
            memory_retention_days: 30,
            enable_tools: true,
            available_tools: vec![
                "web_search".to_string(),
                "file_operations".to_string(),
                "system_commands".to_string(),
                "browser_automation".to_string(),
                "ui_automation".to_string(),
                "workflow_execution".to_string(),
            ],
            enable_multi_agent: true,
            max_concurrent_agents: 10,
        }
    }
}

/// LLM provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LLMProvider {
    /// Local Ollama instance
    Ollama,
    /// OpenAI API (if configured)
    OpenAI,
    /// Anthropic Claude (if configured)
    Anthropic,
    /// Custom provider
    Custom(String),
}

/// Agent type definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentType {
    /// General purpose conversational agent
    Conversational,
    /// Task-specific automation agent
    Automation,
    /// Research and analysis agent
    Research,
    /// Code generation and review agent
    Code,
    /// System administration agent
    System,
    /// Creative content generation agent
    Creative,
    /// Data analysis agent
    DataAnalysis,
    /// Planning and coordination agent
    Planning,
    /// Custom agent type
    Custom(String),
}

/// Agent definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    /// Agent ID
    pub id: String,
    /// Agent name
    pub name: String,
    /// Agent description
    pub description: Option<String>,
    /// Agent type
    pub agent_type: AgentType,
    /// Agent status
    pub status: AgentStatus,
    /// Agent capabilities
    pub capabilities: Vec<AgentCapability>,
    /// Agent configuration
    pub config: AgentConfig,
    /// Agent memory
    pub memory: AgentMemory,
    /// Agent metrics
    pub metrics: AgentMetrics,
    /// Creation time
    pub created_at: DateTime<Utc>,
    /// Last activity time
    pub last_activity: DateTime<Utc>,
    /// Agent metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Agent status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentStatus {
    /// Agent is idle
    Idle,
    /// Agent is thinking/planning
    Thinking,
    /// Agent is executing a task
    Executing,
    /// Agent is waiting for input
    Waiting,
    /// Agent is paused
    Paused,
    /// Agent encountered an error
    Error,
    /// Agent is disabled
    Disabled,
}

/// Agent capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCapability {
    /// Capability name
    pub name: String,
    /// Capability description
    pub description: String,
    /// Capability enabled
    pub enabled: bool,
    /// Capability configuration
    pub config: serde_json::Value,
}

/// Agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Model to use
    pub model: String,
    /// System prompt
    pub system_prompt: String,
    /// Temperature
    pub temperature: f32,
    /// Max tokens
    pub max_tokens: u32,
    /// Enable tools
    pub enable_tools: bool,
    /// Available tools
    pub tools: Vec<String>,
    /// Tool execution timeout
    pub tool_timeout: u64,
    /// Max execution steps
    pub max_steps: u32,
    /// Enable memory
    pub enable_memory: bool,
    /// Memory configuration
    pub memory_config: MemoryConfig,
}

/// Memory configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Memory type
    pub memory_type: MemoryType,
    /// Maximum memory entries
    pub max_entries: usize,
    /// Memory retention period (hours)
    pub retention_hours: u32,
    /// Enable semantic similarity
    pub enable_similarity: bool,
    /// Similarity threshold
    pub similarity_threshold: f32,
}

/// Memory type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum MemoryType {
    /// Short-term conversation memory
    Conversation,
    /// Long-term episodic memory
    Episodic,
    /// Semantic knowledge memory
    Semantic,
    /// Working memory for current task
    Working,
}

/// Agent memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMemory {
    /// Conversation history
    pub conversation: Vec<MemoryEntry>,
    /// Episodic memories
    pub episodic: Vec<MemoryEntry>,
    /// Semantic knowledge
    pub semantic: Vec<MemoryEntry>,
    /// Working memory
    pub working: Vec<MemoryEntry>,
    /// Memory index for fast retrieval
    pub index: HashMap<String, Vec<String>>,
}

impl Default for AgentMemory {
    fn default() -> Self {
        Self {
            conversation: Vec::new(),
            episodic: Vec::new(),
            semantic: Vec::new(),
            working: Vec::new(),
            index: HashMap::new(),
        }
    }
}

/// Memory entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// Entry ID
    pub id: String,
    /// Entry content
    pub content: String,
    /// Entry type
    pub entry_type: MemoryType,
    /// Entry timestamp
    pub timestamp: DateTime<Utc>,
    /// Entry metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Entry importance score
    pub importance: f32,
    /// Entry embedding vector (for similarity search)
    pub embedding: Option<Vec<f32>>,
}

/// Agent metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetrics {
    /// Total tasks completed
    pub tasks_completed: u64,
    /// Total tasks failed
    pub tasks_failed: u64,
    /// Average task completion time
    pub avg_completion_time: f64,
    /// Total tokens used
    pub total_tokens_used: u64,
    /// Tool usage count
    pub tool_usage_count: HashMap<String, u64>,
    /// Error count by type
    pub error_count: HashMap<String, u64>,
    /// Last task completion time
    pub last_task_completion: Option<DateTime<Utc>>,
}

impl Default for AgentMetrics {
    fn default() -> Self {
        Self {
            tasks_completed: 0,
            tasks_failed: 0,
            avg_completion_time: 0.0,
            total_tokens_used: 0,
            tool_usage_count: HashMap::new(),
            error_count: HashMap::new(),
            last_task_completion: None,
        }
    }
}

/// Agent task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTask {
    /// Task ID
    pub id: String,
    /// Task description
    pub description: String,
    /// Task type
    pub task_type: TaskType,
    /// Task priority
    pub priority: TaskPriority,
    /// Task status
    pub status: TaskStatus,
    /// Task input
    pub input: serde_json::Value,
    /// Task output
    pub output: Option<serde_json::Value>,
    /// Task error
    pub error: Option<String>,
    /// Task assigned agent
    pub assigned_agent: String,
    /// Task creation time
    pub created_at: DateTime<Utc>,
    /// Task start time
    pub started_at: Option<DateTime<Utc>>,
    /// Task completion time
    pub completed_at: Option<DateTime<Utc>>,
    /// Task metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Task type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskType {
    /// Question answering
    Question,
    /// Code generation
    CodeGeneration,
    /// Data analysis
    DataAnalysis,
    /// Web automation
    WebAutomation,
    /// System administration
    SystemAdmin,
    /// File operations
    FileOperations,
    /// Research task
    Research,
    /// Creative task
    Creative,
    /// Planning task
    Planning,
    /// Custom task
    Custom(String),
}

/// Task priority
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    /// Low priority
    Low,
    /// Normal priority
    Normal,
    /// High priority
    High,
    /// Critical priority
    Critical,
}

/// Task status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    /// Task is queued
    Queued,
    /// Task is in progress
    InProgress,
    /// Task is completed
    Completed,
    /// Task failed
    Failed,
    /// Task was cancelled
    Cancelled,
    /// Task is paused
    Paused,
}

/// Agent execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentExecutionResult {
    /// Task ID
    pub task_id: String,
    /// Agent ID
    pub agent_id: String,
    /// Execution success
    pub success: bool,
    /// Execution result
    pub result: Option<serde_json::Value>,
    /// Execution error
    pub error: Option<String>,
    /// Execution steps
    pub steps: Vec<ExecutionStep>,
    /// Tokens used
    pub tokens_used: u64,
    /// Execution time
    pub execution_time: std::time::Duration,
    /// Tools used
    pub tools_used: Vec<String>,
    /// Memory updates
    pub memory_updates: Vec<MemoryEntry>,
}

/// Execution step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStep {
    /// Step ID
    pub id: String,
    /// Step type
    pub step_type: StepType,
    /// Step input
    pub input: serde_json::Value,
    /// Step output
    pub output: Option<serde_json::Value>,
    /// Step error
    pub error: Option<String>,
    /// Step timestamp
    pub timestamp: DateTime<Utc>,
    /// Step duration
    pub duration: std::time::Duration,
}

/// Execution step type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepType {
    /// Thinking/reasoning step
    Thinking,
    /// Tool usage step
    ToolUsage,
    /// Memory retrieval step
    MemoryRetrieval,
    /// Memory storage step
    MemoryStorage,
    /// Agent communication step
    Communication,
    /// Final response step
    Response,
}

/// LangChain integration manager
pub struct LangChainManager {
    /// Configuration
    config: Arc<RwLock<LangChainConfig>>,
    /// Active agents
    agents: Arc<RwLock<HashMap<String, Agent>>>,
    /// Task queue
    task_queue: Arc<RwLock<Vec<AgentTask>>>,
    /// Task executor
    task_executor: Option<tokio::task::JoinHandle<()>>,
    /// Task result sender
    result_sender: mpsc::UnboundedSender<AgentExecutionResult>,
    /// Manager statistics
    stats: Arc<RwLock<LangChainStats>>,
}

/// LangChain statistics
#[derive(Debug, Default)]
pub struct LangChainStats {
    /// Total agents created
    total_agents: u64,
    /// Active agents
    active_agents: u64,
    /// Total tasks processed
    total_tasks: u64,
    /// Successful tasks
    successful_tasks: u64,
    /// Failed tasks
    failed_tasks: u64,
    /// Average task completion time
    avg_task_time: f64,
    /// Total tokens used
    total_tokens_used: u64,
    /// Last task completion
    last_task_completion: Option<DateTime<Utc>>,
}

impl LangChainManager {
    /// Create a new LangChain manager
    pub async fn new(config: LangChainConfig) -> Result<Self, AIError> {
        let (result_sender, mut result_receiver) = mpsc::unbounded_channel();
        
        let manager = Self {
            config: Arc::new(RwLock::new(config.clone())),
            agents: Arc::new(RwLock::new(HashMap::new())),
            task_queue: Arc::new(RwLock::new(Vec::new())),
            task_executor: None,
            result_sender,
            stats: Arc::new(RwLock::new(LangChainStats::default())),
        };
        
        // Start result processor
        let stats = manager.stats.clone();
        tokio::spawn(async move {
            while let Some(result) = result_receiver.recv().await {
                // Update statistics
                let mut stats = stats.write();
                stats.total_tasks += 1;
                
                if result.success {
                    stats.successful_tasks += 1;
                } else {
                    stats.failed_tasks += 1;
                }
                
                stats.avg_task_time = (stats.avg_task_time * (stats.total_tasks - 1) as f64 + result.execution_time.as_secs_f64()) / stats.total_tasks as f64;
                stats.total_tokens_used += result.tokens_used;
                stats.last_task_completion = Some(Utc::now());
                
                debug!("Task {} completed: {}", result.task_id, result.success);
            }
        });
        
        info!("LangChain manager initialized");
        Ok(manager)
    }
    
    /// Start the LangChain manager
    pub async fn start(&mut self) -> Result<(), AIError> {
        if !self.config.read().enabled {
            return Ok(());
        }
        
        if self.task_executor.is_some() {
            return Ok(());
        }
        
        let config = self.config.clone();
        let agents = self.agents.clone();
        let task_queue = self.task_queue.clone();
        let result_sender = self.result_sender.clone();
        
        self.task_executor = Some(tokio::spawn(async move {
            Self::task_execution_loop(config, agents, task_queue, result_sender).await;
        }));
        
        info!("LangChain manager started");
        Ok(())
    }
    
    /// Stop the LangChain manager
    pub async fn stop(&mut self) -> Result<(), AIError> {
        if let Some(handle) = self.task_executor.take() {
            handle.abort();
        }
        
        info!("LangChain manager stopped");
        Ok(())
    }
    
    /// Create a new agent
    pub async fn create_agent(&self, agent_type: AgentType, config: AgentConfig) -> Result<String, AIError> {
        let agent_id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();
        
        let agent = Agent {
            id: agent_id.clone(),
            name: format!("{:?} Agent", agent_type),
            description: None,
            agent_type,
            status: AgentStatus::Idle,
            capabilities: Vec::new(),
            config,
            memory: AgentMemory::default(),
            metrics: AgentMetrics::default(),
            created_at: now,
            last_activity: now,
            metadata: HashMap::new(),
        };
        
        self.agents.write().insert(agent_id.clone(), agent);
        
        // Update statistics
        let mut stats = self.stats.write();
        stats.total_agents += 1;
        stats.active_agents += 1;
        
        info!("Agent created: {}", agent_id);
        Ok(agent_id)
    }
    
    /// Submit a task to an agent
    pub async fn submit_task(&self, agent_id: &str, task: AgentTask) -> Result<String, AIError> {
        if !self.agents.read().contains_key(agent_id) {
            return Err(AIError::Configuration(format!("Agent not found: {}", agent_id)));
        }
        
        let mut task = task;
        task.assigned_agent = agent_id.to_string();
        task.created_at = Utc::now();
        task.status = TaskStatus::Queued;
        
        let task_id = task.id.clone();
        self.task_queue.write().push(task);
        
        info!("Task submitted: {} to agent: {}", task_id, agent_id);
        Ok(task_id)
    }
    
    /// Get agent information
    pub fn get_agent(&self, agent_id: &str) -> Option<Agent> {
        self.agents.read().get(agent_id).cloned()
    }
    
    /// List all agents
    pub fn list_agents(&self) -> Vec<Agent> {
        self.agents.read().values().cloned().collect()
    }
    
    /// Update agent configuration
    pub async fn update_agent_config(&self, agent_id: &str, config: AgentConfig) -> Result<(), AIError> {
        if let Some(agent) = self.agents.write().get_mut(agent_id) {
            agent.config = config;
            agent.last_activity = Utc::now();
            info!("Agent configuration updated: {}", agent_id);
            Ok(())
        } else {
            Err(AIError::Configuration(format!("Agent not found: {}", agent_id)))
        }
    }
    
    /// Delete an agent
    pub async fn delete_agent(&self, agent_id: &str) -> Result<(), AIError> {
        if self.agents.write().remove(agent_id).is_some() {
            // Update statistics
            let mut stats = self.stats.write();
            stats.active_agents = stats.active_agents.saturating_sub(1);
            
            info!("Agent deleted: {}", agent_id);
            Ok(())
        } else {
            Err(AIError::Configuration(format!("Agent not found: {}", agent_id)))
        }
    }
    
    /// Update configuration
    pub async fn update_config(&self, new_config: LangChainConfig) -> Result<(), AIError> {
        *self.config.write() = new_config;
        info!("LangChain configuration updated");
        Ok(())
    }
    
    /// Health check
    pub async fn health_check(&self) -> Result<bool, AIError> {
        Ok(self.task_executor.is_some())
    }
    
    /// Get statistics
    pub fn get_stats(&self) -> LangChainStats {
        self.stats.read().clone()
    }
    
    /// Task execution loop
    async fn task_execution_loop(
        config: Arc<RwLock<LangChainConfig>>,
        agents: Arc<RwLock<HashMap<String, Agent>>>,
        task_queue: Arc<RwLock<Vec<AgentTask>>>,
        result_sender: mpsc::UnboundedSender<AgentExecutionResult>,
    ) {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
        
        info!("Task execution loop started");
        
        loop {
            interval.tick().await;
            
            if !config.read().enabled {
                continue;
            }
            
            // Get next task from queue
            let task = {
                let mut queue = task_queue.write();
                queue.sort_by_key(|t| t.priority);
                queue.pop()
            };
            
            if let Some(mut task) = task {
                task.status = TaskStatus::InProgress;
                task.started_at = Some(Utc::now());
                
                // Execute task
                let result = Self::execute_task(&task, &agents).await;
                
                // Send result
                if let Err(e) = result_sender.send(result) {
                    error!("Failed to send task result: {}", e);
                }
            }
        }
    }
    
    /// Execute a task
    async fn execute_task(
        task: &AgentTask,
        agents: &Arc<RwLock<HashMap<String, Agent>>>,
    ) -> AgentExecutionResult {
        let start_time = std::time::Instant::now();
        
        // TODO: Implement actual task execution using LangChain
        // This would involve:
        // 1. Loading the assigned agent
        // 2. Preparing the agent context
        // 3. Executing the task with the agent
        // 4. Processing the result
        // 5. Updating agent memory
        
        debug!("Executing task: {} with agent: {}", task.id, task.assigned_agent);
        
        // Simulate task execution
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        // Update agent status
        if let Some(agent) = agents.write().get_mut(&task.assigned_agent) {
            agent.status = AgentStatus::Executing;
            agent.last_activity = Utc::now();
        }
        
        AgentExecutionResult {
            task_id: task.id.clone(),
            agent_id: task.assigned_agent.clone(),
            success: true,
            result: Some(serde_json::json!({
                "message": "Task completed successfully",
                "task_type": task.task_type
            })),
            error: None,
            steps: vec![
                ExecutionStep {
                    id: uuid::Uuid::new_v4().to_string(),
                    step_type: StepType::Thinking,
                    input: task.input.clone(),
                    output: Some(serde_json::json!({"thought": "Processing task"})),
                    error: None,
                    timestamp: Utc::now(),
                    duration: std::time::Duration::from_millis(50),
                },
                ExecutionStep {
                    id: uuid::Uuid::new_v4().to_string(),
                    step_type: StepType::Response,
                    input: serde_json::json!({}),
                    output: Some(serde_json::json!({"response": "Task completed"})),
                    error: None,
                    timestamp: Utc::now(),
                    duration: std::time::Duration::from_millis(50),
                },
            ],
            tokens_used: 150,
            execution_time: start_time.elapsed(),
            tools_used: Vec::new(),
            memory_updates: Vec::new(),
        }
    }
}

impl Clone for LangChainStats {
    fn clone(&self) -> Self {
        Self {
            total_agents: self.total_agents,
            active_agents: self.active_agents,
            total_tasks: self.total_tasks,
            successful_tasks: self.successful_tasks,
            failed_tasks: self.failed_tasks,
            avg_task_time: self.avg_task_time,
            total_tokens_used: self.total_tokens_used,
            last_task_completion: self.last_task_completion,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_langchain_config_default() {
        let config = LangChainConfig::default();
        assert!(config.enabled);
        assert_eq!(config.default_model, "llama3.2:latest");
        assert_eq!(config.max_tokens, 4096);
        assert_eq!(config.temperature, 0.7);
        assert!(config.enable_memory);
        assert!(config.enable_tools);
        assert!(config.enable_multi_agent);
    }
    
    #[test]
    fn test_agent_creation() {
        let config = AgentConfig {
            model: "llama3.2:latest".to_string(),
            system_prompt: "You are a helpful assistant".to_string(),
            temperature: 0.7,
            max_tokens: 2048,
            enable_tools: true,
            tools: vec!["web_search".to_string()],
            tool_timeout: 30,
            max_steps: 10,
            enable_memory: true,
            memory_config: MemoryConfig {
                memory_type: MemoryType::Conversation,
                max_entries: 100,
                retention_hours: 24,
                enable_similarity: false,
                similarity_threshold: 0.8,
            },
        };
        
        let agent = Agent {
            id: "test-agent".to_string(),
            name: "Test Agent".to_string(),
            description: None,
            agent_type: AgentType::Conversational,
            status: AgentStatus::Idle,
            capabilities: Vec::new(),
            config,
            memory: AgentMemory::default(),
            metrics: AgentMetrics::default(),
            created_at: Utc::now(),
            last_activity: Utc::now(),
            metadata: HashMap::new(),
        };
        
        assert_eq!(agent.id, "test-agent");
        assert_eq!(agent.name, "Test Agent");
        assert!(matches!(agent.agent_type, AgentType::Conversational));
        assert!(matches!(agent.status, AgentStatus::Idle));
    }
    
    #[test]
    fn test_task_priority_ordering() {
        let priorities = vec![
            TaskPriority::Low,
            TaskPriority::Critical,
            TaskPriority::Normal,
            TaskPriority::High,
        ];
        
        let mut sorted = priorities.clone();
        sorted.sort();
        
        assert_eq!(sorted, vec![
            TaskPriority::Low,
            TaskPriority::Normal,
            TaskPriority::High,
            TaskPriority::Critical,
        ]);
    }
    
    #[test]
    fn test_memory_entry_creation() {
        let entry = MemoryEntry {
            id: "mem-001".to_string(),
            content: "Remember this important information".to_string(),
            entry_type: MemoryType::Episodic,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
            importance: 0.8,
            embedding: None,
        };
        
        assert_eq!(entry.id, "mem-001");
        assert_eq!(entry.content, "Remember this important information");
        assert!(matches!(entry.entry_type, MemoryType::Episodic));
        assert_eq!(entry.importance, 0.8);
    }
    
    #[test]
    fn test_execution_result_serialization() {
        let result = AgentExecutionResult {
            task_id: "task-123".to_string(),
            agent_id: "agent-456".to_string(),
            success: true,
            result: Some(serde_json::json!({"output": "success"})),
            error: None,
            steps: Vec::new(),
            tokens_used: 100,
            execution_time: std::time::Duration::from_secs(5),
            tools_used: vec!["web_search".to_string()],
            memory_updates: Vec::new(),
        };
        
        let serialized = serde_json::to_string(&result).unwrap();
        let deserialized: AgentExecutionResult = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(result.task_id, deserialized.task_id);
        assert_eq!(result.agent_id, deserialized.agent_id);
        assert_eq!(result.success, deserialized.success);
        assert_eq!(result.tokens_used, deserialized.tokens_used);
    }
}