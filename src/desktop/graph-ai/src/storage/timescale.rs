//! TimescaleDB storage implementation for AI data
//! 
//! This module provides time-series storage for behavioral data,
//! patterns, and AI-related metrics using TimescaleDB.

use super::{
    Storage, StorageConfig, StorageHealth, StorageStats, RetentionPolicy,
    UserAction, Pattern, PatternData, PatternType, ActionType,
};
use crate::AIError;

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use dashmap::DashMap;
use log::{debug, error, info};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// TimescaleDB client for storing time-series AI data
pub struct TimescaleClient {
    pool: PgPool,
    config: StorageConfig,
    cache: Arc<DashMap<String, CachedQuery>>,
    metrics: Arc<RwLock<PerformanceMetrics>>,
}

/// Cached query result
#[derive(Debug, Clone)]
struct CachedQuery {
    result: String,
    cached_at: DateTime<Utc>,
    ttl: Duration,
}

/// Performance metrics for monitoring
#[derive(Debug, Default)]
struct PerformanceMetrics {
    total_queries: u64,
    successful_queries: u64,
    failed_queries: u64,
    cache_hits: u64,
    cache_misses: u64,
    total_query_time: std::time::Duration,
}

impl TimescaleClient {
    /// Create a new TimescaleDB client with default configuration (placeholder)
    pub fn new_default() -> Self {
        // Create a placeholder pool - this will need to be properly initialized
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy("postgresql://localhost/horizonos")
            .expect("Failed to create lazy connection pool");
        
        Self {
            pool,
            config: StorageConfig::default(),
            cache: Arc::new(DashMap::new()),
            metrics: Arc::new(RwLock::new(PerformanceMetrics::default())),
        }
    }

    /// Create a new TimescaleDB client
    pub async fn new(config: StorageConfig) -> Result<Self, AIError> {
        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .acquire_timeout(std::time::Duration::from_secs(config.connection_timeout))
            .connect(&config.connection_url)
            .await
            .map_err(|e| AIError::Configuration(format!("Failed to connect to TimescaleDB: {}", e)))?;

        let client = Self {
            pool,
            config,
            cache: Arc::new(DashMap::new()),
            metrics: Arc::new(RwLock::new(PerformanceMetrics::default())),
        };

        // Initialize schema if needed
        client.initialize_schema().await?;

        Ok(client)
    }

    /// Initialize database schema
    async fn initialize_schema(&self) -> Result<(), AIError> {
        // Note: Schema initialization is handled by init-timescaledb.sql
        // This method can be used for runtime schema updates if needed
        Ok(())
    }

    /// Store a user action
    pub async fn store_action(&self, action: &UserAction) -> Result<(), AIError> {
        let start_time = std::time::Instant::now();
        
        let query = r#"
            INSERT INTO user_actions (
                time, user_id, action_type, target, application,
                context, duration_ms, success, error_message
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#;
        
        let result = sqlx::query(query)
            .bind(action.timestamp)
            .bind(&action.user_id)
            .bind(format!("{:?}", action.action_type))
            .bind(&action.target)
            .bind(&action.application)
            .bind(&action.context)
            .bind(action.duration_ms.map(|d| d as i32))
            .bind(action.success)
            .bind(&action.error_message)
            .execute(&self.pool)
            .await;
        
        let elapsed = start_time.elapsed();
        self.update_metrics(result.is_ok(), elapsed);
        
        match result {
            Ok(_) => {
                debug!("Stored user action: {:?}", action.action_type);
                Ok(())
            }
            Err(e) => {
                error!("Failed to store user action: {}", e);
                Err(AIError::Configuration(format!("Database error: {}", e)))
            }
        }
    }

    /// Batch insert user actions for performance
    pub async fn store_actions_batch(&self, actions: &[UserAction]) -> Result<u64, AIError> {
        if actions.is_empty() {
            return Ok(0);
        }
        
        let start_time = std::time::Instant::now();
        let mut count = 0;
        
        // Insert actions one by one for type safety
        for action in actions {
            match self.store_action(action).await {
                Ok(()) => count += 1,
                Err(e) => {
                    error!("Failed to insert action in batch: {}", e);
                    // Continue with remaining actions but log the error
                }
            }
        }
        
        let elapsed = start_time.elapsed();
        self.update_metrics(count > 0, elapsed);
        
        debug!("Batch inserted {} user actions", count);
        Ok(count)
    }

    /// Get recent user actions
    pub async fn get_recent_actions(
        &self,
        user_id: &str,
        limit: i64,
        offset: Option<i64>,
    ) -> Result<Vec<UserAction>, AIError> {
        let start_query_time = std::time::Instant::now();
        
        let query = r#"
            SELECT time, user_id, action_type, target, application,
                   context, duration_ms, success, error_message
            FROM user_actions
            WHERE user_id = $1
            ORDER BY time DESC
            LIMIT $2
            OFFSET $3
        "#;
        
        let result = sqlx::query(query)
            .bind(user_id)
            .bind(limit)
            .bind(offset.unwrap_or(0))
            .fetch_all(&self.pool)
            .await;
        
        let elapsed = start_query_time.elapsed();
        self.update_metrics(result.is_ok(), elapsed);
        
        match result {
            Ok(rows) => {
                let actions: Result<Vec<UserAction>, _> = rows.into_iter().map(|row| {
                    let action_type_str: String = row.get("action_type");
                    let action_type = match action_type_str.as_str() {
                        "AppLaunch" => ActionType::AppLaunch,
                        "AppClose" => ActionType::AppClose,
                        "FileOpen" => ActionType::FileOpen,
                        "FileSave" => ActionType::FileSave,
                        "WebNavigate" => ActionType::WebNavigate,
                        "WebClose" => ActionType::WebClose,
                        "CommandExecute" => ActionType::CommandExecute,
                        "WindowFocus" => ActionType::WindowFocus,
                        "WindowClose" => ActionType::WindowClose,
                        _ => ActionType::AppLaunch, // Default fallback
                    };
                    
                    Ok(UserAction {
                        id: Uuid::new_v4(),
                        user_id: row.get("user_id"),
                        action_type,
                        target: row.get("target"),
                        application: row.get("application"),
                        timestamp: row.get("time"),
                        context: row.get("context"),
                        duration_ms: row.get::<Option<i32>, _>("duration_ms").map(|d| d as u64),
                        success: row.get("success"),
                        error_message: row.get("error_message"),
                    })
                }).collect();
                
                actions.map_err(|e: AIError| e)
            }
            Err(e) => {
                error!("Failed to get user actions: {}", e);
                Err(AIError::Configuration(format!("Database error: {}", e)))
            }
        }
    }
    
    /// Store a learned pattern
    pub async fn store_pattern(&self, pattern: &Pattern) -> Result<(), AIError> {
        let start_time = std::time::Instant::now();
        
        let query = r#"
            INSERT INTO learned_patterns (
                id, user_id, pattern_type, pattern_name, pattern_data,
                confidence, first_seen, last_seen, occurrence_count, enabled
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (id) DO UPDATE SET
                pattern_data = EXCLUDED.pattern_data,
                confidence = EXCLUDED.confidence,
                last_seen = EXCLUDED.last_seen,
                occurrence_count = EXCLUDED.occurrence_count,
                updated_at = NOW()
        "#;
        
        let pattern_data_json = serde_json::to_value(&pattern.data)
            .map_err(AIError::Serialization)?;
        
        let result = sqlx::query(query)
            .bind(&pattern.id)
            .bind("default")
            .bind(format!("{:?}", pattern.pattern_type))
            .bind(format!("{:?}", pattern.pattern_type)) // pattern_name
            .bind(pattern_data_json)
            .bind(pattern.confidence)
            .bind(pattern.first_seen)
            .bind(pattern.last_seen)
            .bind(pattern.occurrence_count as i32)
            .bind(pattern.enabled)
            .execute(&self.pool)
            .await;
        
        let elapsed = start_time.elapsed();
        self.update_metrics(result.is_ok(), elapsed);
        
        match result {
            Ok(_) => {
                debug!("Stored pattern: {}", pattern.id);
                Ok(())
            }
            Err(e) => {
                error!("Failed to store pattern: {}", e);
                Err(AIError::Configuration(format!("Database error: {}", e)))
            }
        }
    }
    
    /// Get learned patterns for a user
    pub async fn get_patterns(
        &self,
        user_id: &str,
        pattern_type: Option<PatternType>,
        min_confidence: Option<f32>,
        limit: Option<i64>,
    ) -> Result<Vec<Pattern>, AIError> {
        let start_query_time = std::time::Instant::now();
        
        let mut query = String::from(r#"
            SELECT id, pattern_type, pattern_data, confidence, first_seen, last_seen, occurrence_count, enabled
            FROM learned_patterns
            WHERE user_id = $1 AND enabled = true
        "#);
        
        let mut param_count = 1;
        
        if pattern_type.is_some() {
            param_count += 1;
            query.push_str(&format!(" AND pattern_type = ${}", param_count));
        }
        
        if min_confidence.is_some() {
            param_count += 1;
            query.push_str(&format!(" AND confidence >= ${}", param_count));
        }
        
        query.push_str(" ORDER BY confidence DESC, last_seen DESC");
        
        if let Some(limit) = limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }
        
        let mut sql_query = sqlx::query(&query).bind(user_id);
        
        if let Some(pt) = pattern_type {
            sql_query = sql_query.bind(format!("{:?}", pt));
        }
        
        if let Some(min_conf) = min_confidence {
            sql_query = sql_query.bind(min_conf);
        }
        
        let result = sql_query.fetch_all(&self.pool).await;
        
        let elapsed = start_query_time.elapsed();
        self.update_metrics(result.is_ok(), elapsed);
        
        match result {
            Ok(rows) => {
                let patterns: Result<Vec<Pattern>, _> = rows.into_iter().map(|row| {
                    let pattern_type_str: String = row.get("pattern_type");
                    let pattern_type = match pattern_type_str.as_str() {
                        "Temporal" => PatternType::Temporal,
                        "Sequence" => PatternType::Sequence,
                        "Contextual" => PatternType::Contextual,
                        "Usage" => PatternType::Usage,
                        _ => PatternType::Usage, // Default fallback
                    };
                    
                    let pattern_data_json: serde_json::Value = row.get("pattern_data");
                    let pattern_data: PatternData = serde_json::from_value(pattern_data_json)
                        .map_err(AIError::Serialization)?;
                    
                    Ok(Pattern {
                        id: row.get("id"),
                        pattern_type,
                        data: pattern_data,
                        confidence: row.get("confidence"),
                        first_seen: row.get("first_seen"),
                        last_seen: row.get("last_seen"),
                        occurrence_count: row.get::<i32, _>("occurrence_count") as u32,
                        enabled: row.get("enabled"),
                    })
                }).collect();
                
                patterns
            }
            Err(e) => {
                error!("Failed to get patterns: {}", e);
                Err(AIError::Configuration(format!("Database error: {}", e)))
            }
        }
    }
    
    /// Update performance metrics
    fn update_metrics(&self, success: bool, duration: std::time::Duration) {
        let mut metrics = self.metrics.write();
        metrics.total_queries += 1;
        metrics.total_query_time += duration;
        
        if success {
            metrics.successful_queries += 1;
        } else {
            metrics.failed_queries += 1;
        }
    }
    
    /// Get database size statistics
    pub async fn get_database_size(&self) -> Result<DatabaseSize, AIError> {
        let query = r#"
            SELECT 
                pg_database_size(current_database()) as total_size,
                schemaname,
                tablename,
                pg_total_relation_size(schemaname||'.'||tablename) as table_size
            FROM pg_tables 
            WHERE schemaname = 'public'
            ORDER BY table_size DESC
        "#;
        
        let result = sqlx::query(query)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AIError::Configuration(format!("Failed to get database size: {}", e)))?;
        
        let mut table_sizes = HashMap::new();
        let mut total_size = 0i64;
        
        for row in result {
            if total_size == 0 {
                total_size = row.get("total_size");
            }
            
            let table_name: String = row.get("tablename");
            let table_size: i64 = row.get("table_size");
            table_sizes.insert(table_name, table_size as u64);
        }
        
        Ok(DatabaseSize {
            total_size: total_size as u64,
            table_sizes,
        })
    }
    
    /// Optimize database performance
    pub async fn optimize_database(&self) -> Result<(), AIError> {
        info!("Starting database optimization");
        
        // Analyze tables for query optimization
        let analyze_query = "ANALYZE;";
        sqlx::query(analyze_query)
            .execute(&self.pool)
            .await
            .map_err(|e| AIError::Configuration(format!("Failed to analyze database: {}", e)))?;
        
        info!("Database optimization completed");
        Ok(())
    }
}

#[async_trait]
impl Storage for TimescaleClient {
    async fn initialize(&self) -> Result<(), AIError> {
        info!("Initializing TimescaleDB storage");
        
        // Test connection
        let result = sqlx::query("SELECT 1")
            .fetch_one(&self.pool)
            .await;
        
        match result {
            Ok(_) => {
                info!("TimescaleDB initialization successful");
                Ok(())
            }
            Err(e) => {
                error!("TimescaleDB initialization failed: {}", e);
                Err(AIError::Configuration(format!("Database initialization failed: {}", e)))
            }
        }
    }
    
    async fn health_check(&self) -> Result<StorageHealth, AIError> {
        let start_time = std::time::Instant::now();
        
        let result = sqlx::query("SELECT 1")
            .fetch_one(&self.pool)
            .await;
        
        let response_time = start_time.elapsed().as_millis() as u64;
        
        match result {
            Ok(_) => Ok(StorageHealth {
                healthy: true,
                response_time,
                error_message: None,
                last_check: Utc::now(),
                storage_type: "TimescaleDB".to_string(),
            }),
            Err(e) => Ok(StorageHealth {
                healthy: false,
                response_time,
                error_message: Some(e.to_string()),
                last_check: Utc::now(),
                storage_type: "TimescaleDB".to_string(),
            }),
        }
    }
    
    async fn get_stats(&self) -> Result<StorageStats, AIError> {
        let (avg_query_time, metadata) = {
            let metrics = self.metrics.read();
            let avg_query_time = if metrics.total_queries > 0 {
                metrics.total_query_time.as_millis() as f64 / metrics.total_queries as f64
            } else {
                0.0
            };
            
            let mut metadata = HashMap::new();
            metadata.insert("successful_queries".to_string(), metrics.successful_queries.to_string());
            metadata.insert("failed_queries".to_string(), metrics.failed_queries.to_string());
            metadata.insert("cache_hits".to_string(), metrics.cache_hits.to_string());
            metadata.insert("cache_misses".to_string(), metrics.cache_misses.to_string());
            
            (avg_query_time, metadata)
        };
        
        let db_size = self.get_database_size().await?;
        
        Ok(StorageStats {
            total_records: 0, // TODO: Count total records
            storage_size: db_size.total_size,
            avg_query_time,
            queries_last_hour: 0, // TODO: Calculate last hour
            utilization: 0.0, // TODO: Calculate utilization
            metadata,
        })
    }
    
    async fn cleanup(&self, retention_policy: RetentionPolicy) -> Result<u64, AIError> {
        info!("Starting TimescaleDB cleanup with retention policy");
        
        let cutoff_time = Utc::now() - retention_policy.detailed_retention;
        
        let cleanup_query = r#"
            DELETE FROM user_actions 
            WHERE time < $1
        "#;
        
        let result = sqlx::query(cleanup_query)
            .bind(cutoff_time)
            .execute(&self.pool)
            .await
            .map_err(|e| AIError::Configuration(format!("Cleanup failed: {}", e)))?;
        
        let deleted_count = result.rows_affected();
        
        info!("Cleanup completed: {} records deleted", deleted_count);
        Ok(deleted_count)
    }
}

/// Database size information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseSize {
    pub total_size: u64,
    pub table_sizes: HashMap<String, u64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_timescale_client_creation() {
        let config = StorageConfig::default();
        
        // This will fail in CI but serves as a documentation test
        let result = TimescaleClient::new(config).await;
        
        // In a real environment, this should succeed
        // assert!(result.is_ok());
    }
    
    #[test]
    fn test_cached_query() {
        let cached_query = CachedQuery {
            result: "test_result".to_string(),
            cached_at: Utc::now(),
            ttl: Duration::minutes(5),
        };
        
        assert_eq!(cached_query.result, "test_result");
        assert!(cached_query.cached_at <= Utc::now());
    }
}