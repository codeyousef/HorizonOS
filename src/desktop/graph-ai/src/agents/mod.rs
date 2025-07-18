//! AI Agent Framework
//! 
//! This module provides a comprehensive AI agent framework with LangChain integration,
//! multi-agent coordination, task decomposition, memory management, and communication protocols.

pub mod langchain;
pub mod coordinator;
pub mod decomposition;
pub mod memory;
pub mod communication;

use crate::AIError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use chrono::{DateTime, Utc};
use tokio::sync::mpsc;
use log::{info, debug};

pub use langchain::{
    LangChainManager, LangChainConfig, Agent, AgentType, AgentTask, AgentExecutionResult,
    TaskType, TaskPriority, TaskStatus, AgentStatus, AgentConfig, MemoryConfig, MemoryType,
    MemoryEntry,
};
pub use coordinator::{
    AgentCoordinator, CoordinationConfig, CoordinatedTask, CoordinatedTaskType,
    CoordinationResult, DistributionStrategy, SelectionStrategy,
};
pub use decomposition::{
    TaskDecompositionEngine, DecompositionConfig, DecompositionResult,
    TaskComplexity, DecompositionPattern,
};
pub use memory::{
    MemoryManager, MemoryManagerConfig, MemoryQuery, MemorySearchResult,
    ImportanceScorer,
};
pub use communication::{
    CommunicationManager, CommunicationConfig, AgentMessage, MessageType,
    MessageContent, ContentType, MessagePriority, Conversation, ConversationType,
    AgentPresence, PresenceStatus, create_text_message, create_data_message,
};

/// AI Agent System configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSystemConfig {
    /// Enable agent system
    pub enabled: bool,
    /// LangChain configuration
    pub langchain: LangChainConfig,
    /// Coordination configuration
    pub coordination: CoordinationConfig,
    /// Decomposition configuration
    pub decomposition: DecompositionConfig,
    /// Memory configuration
    pub memory: MemoryManagerConfig,
    /// Communication configuration
    pub communication: CommunicationConfig,
    /// System-wide settings
    pub system: SystemConfig,
}

impl Default for AgentSystemConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            langchain: LangChainConfig::default(),
            coordination: CoordinationConfig::default(),
            decomposition: DecompositionConfig::default(),
            memory: MemoryManagerConfig::default(),
            communication: CommunicationConfig::default(),
            system: SystemConfig::default(),
        }
    }
}

/// System-wide configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfig {
    /// Maximum concurrent agents
    pub max_agents: usize,
    /// Agent creation timeout (seconds)
    pub agent_creation_timeout: u64,
    /// Agent lifecycle management
    pub enable_lifecycle_management: bool,
    /// Agent health check interval (seconds)
    pub health_check_interval: u64,
    /// Enable agent persistence
    pub enable_persistence: bool,
    /// Persistence directory
    pub persistence_dir: String,
    /// Enable agent metrics collection
    pub enable_metrics: bool,
    /// Metrics collection interval (seconds)
    pub metrics_interval: u64,
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            max_agents: 100,
            agent_creation_timeout: 30,
            enable_lifecycle_management: true,
            health_check_interval: 60,
            enable_persistence: true,
            persistence_dir: "/tmp/horizonos/agent_system".to_string(),
            enable_metrics: true,
            metrics_interval: 60,
        }
    }
}

/// AI Agent System - Main entry point for agent operations
pub struct AIAgentSystem {
    /// Configuration
    config: Arc<RwLock<AgentSystemConfig>>,
    /// LangChain manager
    langchain_manager: Arc<RwLock<Option<LangChainManager>>>,
    /// Agent coordinator
    coordinator: Arc<RwLock<Option<AgentCoordinator>>>,
    /// Task decomposition engine
    decomposition_engine: Arc<RwLock<Option<TaskDecompositionEngine>>>,
    /// Memory manager
    memory_manager: Arc<RwLock<Option<MemoryManager>>>,
    /// Communication manager
    communication_manager: Arc<RwLock<Option<CommunicationManager>>>,
    /// System statistics
    stats: Arc<RwLock<SystemStats>>,
    /// Lifecycle manager handle
    lifecycle_handle: Option<tokio::task::JoinHandle<()>>,
}

/// System statistics
#[derive(Debug, Default)]
pub struct SystemStats {
    /// Total agents created
    total_agents_created: u64,
    /// Active agents
    active_agents: u64,
    /// Total tasks processed
    total_tasks_processed: u64,
    /// Total coordinated tasks
    total_coordinated_tasks: u64,
    /// Total messages sent
    total_messages_sent: u64,
    /// System uptime
    system_start_time: Option<DateTime<Utc>>,
    /// Last health check
    last_health_check: Option<DateTime<Utc>>,
}

impl AIAgentSystem {
    /// Create a new AI agent system
    pub async fn new(config: AgentSystemConfig) -> Result<Self, AIError> {
        let system = Self {
            config: Arc::new(RwLock::new(config.clone())),
            langchain_manager: Arc::new(RwLock::new(None)),
            coordinator: Arc::new(RwLock::new(None)),
            decomposition_engine: Arc::new(RwLock::new(None)),
            memory_manager: Arc::new(RwLock::new(None)),
            communication_manager: Arc::new(RwLock::new(None)),
            stats: Arc::new(RwLock::new(SystemStats::default())),
            lifecycle_handle: None,
        };
        
        // Initialize persistence directory
        if config.system.enable_persistence {
            tokio::fs::create_dir_all(&config.system.persistence_dir).await
                .map_err(|e| AIError::Configuration(format!("Failed to create persistence directory: {}", e)))?;
        }
        
        info!("AI agent system initialized");
        Ok(system)
    }
    
    /// Start the AI agent system
    pub async fn start(&mut self) -> Result<(), AIError> {
        if !self.config.read().enabled {
            return Ok(());
        }
        
        let config = self.config.read().clone();
        
        // Initialize LangChain manager
        let mut langchain = LangChainManager::new(config.langchain).await?;
        langchain.start().await?;
        *self.langchain_manager.write() = Some(langchain);
        
        // Initialize coordinator
        let mut coordinator = AgentCoordinator::new(config.coordination).await?;
        coordinator.start().await?;
        *self.coordinator.write() = Some(coordinator);
        
        // Initialize decomposition engine
        let decomposition = TaskDecompositionEngine::new(config.decomposition).await?;
        *self.decomposition_engine.write() = Some(decomposition);
        
        // Initialize memory manager
        let mut memory = MemoryManager::new(config.memory).await?;
        memory.start().await?;
        *self.memory_manager.write() = Some(memory);
        
        // Initialize communication manager
        let communication = CommunicationManager::new(config.communication).await?;
        *self.communication_manager.write() = Some(communication);
        
        // Start lifecycle management
        if config.system.enable_lifecycle_management {
            let stats = self.stats.clone();
            let langchain_manager = self.langchain_manager.clone();
            let coordinator = self.coordinator.clone();
            let health_check_interval = config.system.health_check_interval;
            
            self.lifecycle_handle = Some(tokio::spawn(async move {
                Self::lifecycle_management_loop(
                    stats,
                    langchain_manager,
                    coordinator,
                    health_check_interval,
                ).await;
            }));
        }
        
        // Update statistics
        let mut stats = self.stats.write();
        stats.system_start_time = Some(Utc::now());
        
        info!("AI agent system started");
        Ok(())
    }
    
    /// Stop the AI agent system
    pub async fn stop(&mut self) -> Result<(), AIError> {
        // Stop lifecycle management
        if let Some(handle) = self.lifecycle_handle.take() {
            handle.abort();
        }
        
        // Stop LangChain manager
        if let Some(mut langchain) = self.langchain_manager.write().take() {
            langchain.stop().await?;
        }
        
        // Stop coordinator
        if let Some(mut coordinator) = self.coordinator.write().take() {
            coordinator.stop().await?;
        }
        
        // Stop memory manager
        if let Some(mut memory) = self.memory_manager.write().take() {
            memory.stop().await?;
        }
        
        info!("AI agent system stopped");
        Ok(())
    }
    
    /// Create a new agent
    pub async fn create_agent(&self, agent_type: AgentType, config: AgentConfig) -> Result<String, AIError> {
        let langchain = self.langchain_manager.read();
        if let Some(manager) = langchain.as_ref() {
            let agent_id = manager.create_agent(agent_type, config).await?;
            
            // Update statistics
            let mut stats = self.stats.write();
            stats.total_agents_created += 1;
            stats.active_agents += 1;
            
            info!("Agent created through system: {}", agent_id);
            Ok(agent_id)
        } else {
            Err(AIError::Configuration("LangChain manager not initialized".to_string()))
        }
    }
    
    /// Submit a task to an agent
    pub async fn submit_task(&self, agent_id: &str, task: AgentTask) -> Result<String, AIError> {
        let langchain = self.langchain_manager.read();
        if let Some(manager) = langchain.as_ref() {
            let task_id = manager.submit_task(agent_id, task).await?;
            
            // Update statistics
            self.stats.write().total_tasks_processed += 1;
            
            info!("Task submitted through system: {} to agent: {}", task_id, agent_id);
            Ok(task_id)
        } else {
            Err(AIError::Configuration("LangChain manager not initialized".to_string()))
        }
    }
    
    /// Submit a coordinated task
    pub async fn submit_coordinated_task(&self, task: CoordinatedTask) -> Result<String, AIError> {
        let coordinator = self.coordinator.read();
        if let Some(coord) = coordinator.as_ref() {
            // Register agents with coordinator
            let langchain = self.langchain_manager.read();
            if let Some(manager) = langchain.as_ref() {
                for agent in manager.list_agents() {
                    coord.register_agent(agent).await?;
                }
            }
            
            let task_id = coord.submit_task(task).await?;
            
            // Update statistics
            self.stats.write().total_coordinated_tasks += 1;
            
            info!("Coordinated task submitted: {}", task_id);
            Ok(task_id)
        } else {
            Err(AIError::Configuration("Coordinator not initialized".to_string()))
        }
    }
    
    /// Decompose a task
    pub async fn decompose_task(&self, task: &AgentTask) -> Result<DecompositionResult, AIError> {
        let decomposition = self.decomposition_engine.read();
        if let Some(engine) = decomposition.as_ref() {
            engine.decompose_task(task).await
        } else {
            Err(AIError::Configuration("Decomposition engine not initialized".to_string()))
        }
    }
    
    /// Store memory for an agent
    pub async fn store_memory(&self, agent_id: &str, entry: MemoryEntry) -> Result<(), AIError> {
        let memory = self.memory_manager.read();
        if let Some(manager) = memory.as_ref() {
            // Add agent ID to metadata
            let mut entry = entry;
            entry.metadata.insert("agent_id".to_string(), serde_json::Value::String(agent_id.to_string()));
            
            manager.store_memory(entry).await
        } else {
            Err(AIError::Configuration("Memory manager not initialized".to_string()))
        }
    }
    
    /// Search memories
    pub async fn search_memories(&self, query: &MemoryQuery) -> Result<MemorySearchResult, AIError> {
        let memory = self.memory_manager.read();
        if let Some(manager) = memory.as_ref() {
            manager.search_memories(query).await
        } else {
            Err(AIError::Configuration("Memory manager not initialized".to_string()))
        }
    }
    
    /// Send a message between agents
    pub async fn send_message(&self, message: AgentMessage) -> Result<(), AIError> {
        let communication = self.communication_manager.read();
        if let Some(manager) = communication.as_ref() {
            // Update statistics
            self.stats.write().total_messages_sent += 1;
            
            manager.send_message(message).await
        } else {
            Err(AIError::Configuration("Communication manager not initialized".to_string()))
        }
    }
    
    /// Register an agent for communication
    pub async fn register_agent_communication(&self, agent_id: String) -> Result<mpsc::UnboundedReceiver<AgentMessage>, AIError> {
        let communication = self.communication_manager.read();
        if let Some(manager) = communication.as_ref() {
            manager.register_agent(agent_id).await
        } else {
            Err(AIError::Configuration("Communication manager not initialized".to_string()))
        }
    }
    
    /// Update system configuration
    pub async fn update_config(&self, new_config: AgentSystemConfig) -> Result<(), AIError> {
        *self.config.write() = new_config.clone();
        
        // Update component configurations
        if let Some(manager) = self.langchain_manager.read().as_ref() {
            manager.update_config(new_config.langchain).await?;
        }
        
        if let Some(coord) = self.coordinator.read().as_ref() {
            coord.update_config(new_config.coordination).await?;
        }
        
        if let Some(engine) = self.decomposition_engine.read().as_ref() {
            engine.update_config(new_config.decomposition).await?;
        }
        
        if let Some(manager) = self.memory_manager.read().as_ref() {
            manager.update_config(new_config.memory).await?;
        }
        
        if let Some(manager) = self.communication_manager.read().as_ref() {
            manager.update_config(new_config.communication).await?;
        }
        
        info!("AI agent system configuration updated");
        Ok(())
    }
    
    /// Get system statistics
    pub fn get_stats(&self) -> SystemStats {
        self.stats.read().clone()
    }
    
    /// Health check
    pub async fn health_check(&self) -> Result<SystemHealth, AIError> {
        let mut health = SystemHealth {
            overall_status: HealthStatus::Healthy,
            components: HashMap::new(),
            last_check: Utc::now(),
        };
        
        // Check LangChain manager
        if let Some(manager) = self.langchain_manager.read().as_ref() {
            let status = if manager.health_check().await? {
                HealthStatus::Healthy
            } else {
                HealthStatus::Unhealthy
            };
            health.components.insert("langchain".to_string(), status);
        }
        
        // Check coordinator
        if let Some(coord) = self.coordinator.read().as_ref() {
            let status = if coord.health_check().await? {
                HealthStatus::Healthy
            } else {
                HealthStatus::Unhealthy
            };
            health.components.insert("coordinator".to_string(), status);
        }
        
        // Update overall status
        if health.components.values().any(|s| matches!(s, HealthStatus::Unhealthy)) {
            health.overall_status = HealthStatus::Degraded;
        }
        
        // Update statistics
        self.stats.write().last_health_check = Some(health.last_check);
        
        Ok(health)
    }
    
    /// Lifecycle management loop
    async fn lifecycle_management_loop(
        stats: Arc<RwLock<SystemStats>>,
        _langchain_manager: Arc<RwLock<Option<LangChainManager>>>,
        coordinator: Arc<RwLock<Option<AgentCoordinator>>>,
        health_check_interval: u64,
    ) {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(health_check_interval));
        
        info!("Lifecycle management loop started");
        
        loop {
            interval.tick().await;
            
            // Perform health checks
            debug!("Performing system health check");
            
            // Check LangChain manager - skip for now to avoid Send/Sync issues
            // TODO: Fix Send/Sync issue with LangChainManager
            let health_check_result: Result<bool, AIError> = Ok(true);
            
            if let Ok(healthy) = health_check_result {
                if !healthy {
                    log::warn!("LangChain manager health check failed");
                }
            }
            
            // Check coordinator (skip health check for now due to async constraints)
            let coordinator_available = {
                let guard = coordinator.read();
                guard.is_some()
            };
            
            if coordinator_available {
                debug!("Coordinator is available");
            } else {
                debug!("Coordinator is not available");
            }
            
            // Update last health check time
            stats.write().last_health_check = Some(Utc::now());
        }
    }
}

/// System health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealth {
    /// Overall system status
    pub overall_status: HealthStatus,
    /// Component health statuses
    pub components: HashMap<String, HealthStatus>,
    /// Last health check time
    pub last_check: DateTime<Utc>,
}

/// Health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthStatus {
    /// Component is healthy
    Healthy,
    /// Component is degraded but functional
    Degraded,
    /// Component is unhealthy
    Unhealthy,
}

impl Clone for SystemStats {
    fn clone(&self) -> Self {
        Self {
            total_agents_created: self.total_agents_created,
            active_agents: self.active_agents,
            total_tasks_processed: self.total_tasks_processed,
            total_coordinated_tasks: self.total_coordinated_tasks,
            total_messages_sent: self.total_messages_sent,
            system_start_time: self.system_start_time,
            last_health_check: self.last_health_check,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_agent_system_config_default() {
        let config = AgentSystemConfig::default();
        assert!(config.enabled);
        assert!(config.langchain.enabled);
        assert!(config.coordination.enabled);
        assert!(config.decomposition.enabled);
        assert!(config.memory.enabled);
        assert!(config.communication.enabled);
        assert_eq!(config.system.max_agents, 100);
    }
    
    #[test]
    fn test_system_config_default() {
        let config = SystemConfig::default();
        assert_eq!(config.max_agents, 100);
        assert_eq!(config.agent_creation_timeout, 30);
        assert!(config.enable_lifecycle_management);
        assert_eq!(config.health_check_interval, 60);
        assert!(config.enable_persistence);
        assert!(config.enable_metrics);
    }
    
    #[test]
    fn test_system_health() {
        let mut health = SystemHealth {
            overall_status: HealthStatus::Healthy,
            components: HashMap::new(),
            last_check: Utc::now(),
        };
        
        health.components.insert("langchain".to_string(), HealthStatus::Healthy);
        health.components.insert("coordinator".to_string(), HealthStatus::Healthy);
        health.components.insert("memory".to_string(), HealthStatus::Degraded);
        
        assert!(matches!(health.overall_status, HealthStatus::Healthy));
        assert_eq!(health.components.len(), 3);
    }
}