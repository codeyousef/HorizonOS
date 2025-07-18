//! Storage layer for AI integration
//! 
//! This module provides data persistence and retrieval for:
//! - User behavioral patterns and actions
//! - AI model performance metrics
//! - System configuration and preferences
//! - Workflow definitions and executions

pub mod timescale;
pub mod cache;
pub mod config;

use crate::AIError;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use uuid::Uuid;

/// Common storage traits
#[async_trait::async_trait]
pub trait Storage: Send + Sync {
    /// Initialize the storage system
    async fn initialize(&self) -> Result<(), AIError>;
    
    /// Health check for the storage system
    async fn health_check(&self) -> Result<StorageHealth, AIError>;
    
    /// Get storage statistics
    async fn get_stats(&self) -> Result<StorageStats, AIError>;
    
    /// Cleanup old or expired data
    async fn cleanup(&self, retention_policy: RetentionPolicy) -> Result<u64, AIError>;
}

/// Storage health information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageHealth {
    /// Whether storage is healthy
    pub healthy: bool,
    /// Response time in milliseconds
    pub response_time: u64,
    /// Error message if unhealthy
    pub error_message: Option<String>,
    /// Last health check time
    pub last_check: DateTime<Utc>,
    /// Storage type
    pub storage_type: String,
}

/// Storage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    /// Total records stored
    pub total_records: u64,
    /// Storage size in bytes
    pub storage_size: u64,
    /// Average query time in milliseconds
    pub avg_query_time: f64,
    /// Number of queries in last hour
    pub queries_last_hour: u64,
    /// Storage utilization percentage
    pub utilization: f32,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Data retention policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    /// Retention period for detailed data
    pub detailed_retention: chrono::Duration,
    /// Retention period for aggregated data
    pub aggregated_retention: chrono::Duration,
    /// Maximum storage size before cleanup
    pub max_storage_size: Option<u64>,
    /// Cleanup schedule (cron expression)
    pub cleanup_schedule: Option<String>,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            detailed_retention: chrono::Duration::days(30),
            aggregated_retention: chrono::Duration::days(365),
            max_storage_size: Some(10 * 1024 * 1024 * 1024), // 10GB
            cleanup_schedule: Some("0 2 * * *".to_string()), // Daily at 2 AM
        }
    }
}

/// User action types
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ActionType {
    /// Application launch
    AppLaunch,
    /// Application close
    AppClose,
    /// File open
    FileOpen,
    /// File save
    FileSave,
    /// File close
    FileClose,
    /// Web navigation
    WebNavigate,
    /// Web page close
    WebClose,
    /// Command execution
    CommandExecute,
    /// Window focus change
    WindowFocus,
    /// Window close
    WindowClose,
    /// Text input
    TextInput,
    /// Copy action
    Copy,
    /// Paste action
    Paste,
    /// System event
    SystemEvent,
}

/// User action record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAction {
    /// Unique action ID
    pub id: Uuid,
    /// User ID
    pub user_id: String,
    /// Action type
    pub action_type: ActionType,
    /// Target of the action (e.g., file path, URL, app name)
    pub target: String,
    /// Application that performed the action
    pub application: String,
    /// Timestamp of the action
    pub timestamp: DateTime<Utc>,
    /// Additional context data
    pub context: serde_json::Value,
    /// Duration in milliseconds (if applicable)
    pub duration_ms: Option<u64>,
    /// Whether the action was successful
    pub success: bool,
    /// Error message if failed
    pub error_message: Option<String>,
}

/// Pattern types
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum PatternType {
    /// Time-based patterns (e.g., daily routines)
    Temporal,
    /// Sequence patterns (e.g., workflow steps)
    Sequence,
    /// Context-based patterns
    Contextual,
    /// Usage frequency patterns
    Usage,
}

/// Pattern data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternData {
    /// Actions that make up this pattern
    pub actions: Vec<ActionType>,
    /// Time intervals between actions (in seconds)
    pub intervals: Vec<i64>,
    /// Common contexts
    pub contexts: HashMap<String, serde_json::Value>,
    /// Pattern metadata
    pub metadata: serde_json::Value,
}

/// Learned pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    /// Pattern ID
    pub id: Uuid,
    /// Pattern type
    pub pattern_type: PatternType,
    /// Pattern data
    pub data: PatternData,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// First time this pattern was observed
    pub first_seen: DateTime<Utc>,
    /// Last time this pattern was observed
    pub last_seen: DateTime<Utc>,
    /// Number of times this pattern occurred
    pub occurrence_count: u32,
    /// Whether this pattern is enabled for suggestions
    pub enabled: bool,
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Database connection URL
    pub connection_url: String,
    /// Maximum connections in pool
    pub max_connections: u32,
    /// Connection timeout in seconds
    pub connection_timeout: u64,
    /// Query timeout in seconds
    pub query_timeout: u64,
    /// Enable connection pooling
    pub enable_pooling: bool,
    /// Retention policy
    pub retention_policy: RetentionPolicy,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            connection_url: "postgresql://horizonos:horizonos_ai_secure@localhost:5432/ai_patterns".to_string(),
            max_connections: 20,
            connection_timeout: 30,
            query_timeout: 60,
            enable_pooling: true,
            retention_policy: RetentionPolicy::default(),
        }
    }
}

/// Storage manager that coordinates different storage backends
pub struct StorageManager {
    /// TimescaleDB client for time-series data
    pub timescale: timescale::TimescaleClient,
    /// Cache layer for frequently accessed data
    pub cache: cache::CacheManager,
    /// Configuration storage
    pub config: config::ConfigStorage,
}

impl StorageManager {
    /// Create a new storage manager with default configuration (placeholder)
    pub fn new_default() -> Self {
        Self {
            timescale: timescale::TimescaleClient::new_default(),
            cache: cache::CacheManager::new_default(),
            config: config::ConfigStorage::new_default(),
        }
    }
    
    /// Create a new storage manager
    pub async fn new(config: StorageConfig) -> Result<Self, AIError> {
        let timescale = timescale::TimescaleClient::new(config.clone()).await?;
        let cache = cache::CacheManager::new(cache::CacheConfig::default()).await?;
        let config_storage = config::ConfigStorage::new(config.clone()).await?;
        
        Ok(Self {
            timescale,
            cache,
            config: config_storage,
        })
    }
    
    /// Initialize all storage backends
    pub async fn initialize(&self) -> Result<(), AIError> {
        self.timescale.initialize().await?;
        self.cache.initialize().await?;
        self.config.initialize().await?;
        Ok(())
    }
    
    /// Get overall storage health
    pub async fn health_check(&self) -> Result<Vec<StorageHealth>, AIError> {
        let mut health_checks = Vec::new();
        
        // Check TimescaleDB health
        if let Ok(health) = self.timescale.health_check().await {
            health_checks.push(health);
        }
        
        // Check cache health
        if let Ok(health) = self.cache.health_check().await {
            health_checks.push(health);
        }
        
        // Check config storage health
        if let Ok(health) = self.config.health_check().await {
            health_checks.push(health);
        }
        
        Ok(health_checks)
    }
    
    /// Get combined storage statistics
    pub async fn get_combined_stats(&self) -> Result<StorageStats, AIError> {
        let timescale_stats = self.timescale.get_stats().await?;
        let cache_stats = self.cache.get_stats().await?;
        let config_stats = self.config.get_stats().await?;
        
        Ok(StorageStats {
            total_records: timescale_stats.total_records + cache_stats.total_records + config_stats.total_records,
            storage_size: timescale_stats.storage_size + cache_stats.storage_size + config_stats.storage_size,
            avg_query_time: (timescale_stats.avg_query_time + cache_stats.avg_query_time + config_stats.avg_query_time) / 3.0,
            queries_last_hour: timescale_stats.queries_last_hour + cache_stats.queries_last_hour + config_stats.queries_last_hour,
            utilization: (timescale_stats.utilization + cache_stats.utilization + config_stats.utilization) / 3.0,
            metadata: [
                timescale_stats.metadata,
                cache_stats.metadata,
                config_stats.metadata,
            ].into_iter().flatten().collect(),
        })
    }
    
    /// Run cleanup across all storage backends
    pub async fn cleanup_all(&self, retention_policy: RetentionPolicy) -> Result<u64, AIError> {
        let mut total_cleaned = 0;
        
        total_cleaned += self.timescale.cleanup(retention_policy.clone()).await?;
        total_cleaned += self.cache.cleanup(retention_policy.clone()).await?;
        total_cleaned += self.config.cleanup(retention_policy).await?;
        
        Ok(total_cleaned)
    }
}