//! Configuration storage for AI settings
//! 
//! This module provides persistent storage for configuration values
//! with versioning, validation, and backup capabilities.

use super::{Storage, StorageConfig, StorageHealth, StorageStats, RetentionPolicy};
use crate::AIError;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use log::{debug, error, info, warn};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Configuration value with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigValue {
    /// The actual configuration value
    pub value: Value,
    /// When this configuration was created
    pub created_at: DateTime<Utc>,
    /// When this configuration was last updated
    pub updated_at: DateTime<Utc>,
    /// Who last updated this configuration
    pub updated_by: String,
    /// Whether this value is encrypted
    pub encrypted: bool,
    /// Configuration category
    pub category: String,
    /// Optional description
    pub description: Option<String>,
}

/// Configuration statistics
#[derive(Debug, Default)]
pub struct ConfigStats {
    pub reads: u64,
    pub writes: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub total_size: usize,
    pub last_backup: Option<DateTime<Utc>>,
}

/// Configuration storage manager
pub struct ConfigStorage {
    config: StorageConfig,
    config_path: PathBuf,
    cache: Arc<RwLock<HashMap<String, ConfigValue>>>,
    stats: Arc<RwLock<ConfigStats>>,
}

impl ConfigStorage {
    /// Create a new configuration storage with default configuration (placeholder)
    pub fn new_default() -> Self {
        Self {
            config: StorageConfig::default(),
            config_path: dirs::config_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("horizonos")
                .join("ai")
                .join("ai_config.json"),
            cache: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(ConfigStats::default())),
        }
    }

    /// Create a new configuration storage
    pub async fn new(config: StorageConfig) -> Result<Self, AIError> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| AIError::Configuration("Unable to determine config directory".to_string()))?
            .join("horizonos")
            .join("ai");
        
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)
                .map_err(|e| AIError::Configuration(format!("Failed to create config directory: {}", e)))?;
        }
        
        let config_path = config_dir.join("ai_config.json");
        
        let storage = Self {
            config,
            config_path,
            cache: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(ConfigStats::default())),
        };
        
        // Load existing configuration
        storage.load_from_disk().await?;
        
        info!("Configuration storage initialized at: {:?}", storage.config_path);
        Ok(storage)
    }
    
    /// Get a configuration value
    pub fn get<T>(&self, key: &str) -> Option<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let cache = self.cache.read();
        let mut stats = self.stats.write();
        
        stats.reads += 1;
        
        if let Some(config_value) = cache.get(key) {
            stats.cache_hits += 1;
            
            match serde_json::from_value(config_value.value.clone()) {
                Ok(value) => Some(value),
                Err(e) => {
                    error!("Failed to deserialize config value for key {}: {}", key, e);
                    None
                }
            }
        } else {
            stats.cache_misses += 1;
            None
        }
    }
    
    /// Set a configuration value
    pub async fn set<T>(&self, key: &str, value: &T, category: &str, description: Option<String>) -> Result<(), AIError>
    where
        T: Serialize + std::fmt::Debug,
    {
        let json_value = serde_json::to_value(value)
            .map_err(AIError::Serialization)?;
        
        let config_value = ConfigValue {
            value: json_value,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            updated_by: "system".to_string(),
            encrypted: false,
            category: category.to_string(),
            description,
        };
        
        {
            let mut cache = self.cache.write();
            cache.insert(key.to_string(), config_value);
        }
        
        {
            let mut stats = self.stats.write();
            stats.writes += 1;
        }
        
        // Save to disk
        self.save_to_disk().await?;
        
        debug!("Set configuration: {} = {:?}", key, value);
        Ok(())
    }
    
    /// Remove a configuration value
    pub async fn remove(&self, key: &str) -> Result<bool, AIError> {
        let removed = {
            let mut cache = self.cache.write();
            cache.remove(key).is_some()
        };
        
        if removed {
            self.save_to_disk().await?;
            debug!("Removed configuration: {}", key);
        }
        
        Ok(removed)
    }
    
    /// Get all configuration values in a category
    pub fn get_category(&self, category: &str) -> HashMap<String, ConfigValue> {
        let cache = self.cache.read();
        
        cache.iter()
            .filter(|(_, config_value)| config_value.category == category)
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()
    }
    
    /// Get all configuration keys
    pub fn get_all_keys(&self) -> Vec<String> {
        let cache = self.cache.read();
        cache.keys().cloned().collect()
    }
    
    /// Get configuration value with metadata
    pub fn get_with_metadata(&self, key: &str) -> Option<ConfigValue> {
        let cache = self.cache.read();
        cache.get(key).cloned()
    }
    
    /// Update configuration metadata
    pub async fn update_metadata(&self, key: &str, updated_by: &str, description: Option<String>) -> Result<(), AIError> {
        let mut cache = self.cache.write();
        
        if let Some(config_value) = cache.get_mut(key) {
            config_value.updated_at = Utc::now();
            config_value.updated_by = updated_by.to_string();
            if let Some(desc) = description {
                config_value.description = Some(desc);
            }
            
            drop(cache);
            self.save_to_disk().await?;
            
            Ok(())
        } else {
            Err(AIError::Configuration(format!("Configuration key not found: {}", key)))
        }
    }
    
    /// Load configuration from disk
    async fn load_from_disk(&self) -> Result<(), AIError> {
        if !self.config_path.exists() {
            info!("No existing configuration file found, starting with empty configuration");
            return Ok(());
        }
        
        let content = fs::read_to_string(&self.config_path)
            .map_err(|e| AIError::Configuration(format!("Failed to read config file: {}", e)))?;
        
        let config_data: HashMap<String, ConfigValue> = serde_json::from_str(&content)
            .map_err(AIError::Serialization)?;
        
        {
            let mut cache = self.cache.write();
            *cache = config_data;
        }
        
        info!("Loaded {} configuration values from disk", self.cache.read().len());
        Ok(())
    }
    
    /// Save configuration to disk
    async fn save_to_disk(&self) -> Result<(), AIError> {
        let cache = self.cache.read();
        
        let content = serde_json::to_string_pretty(&*cache)
            .map_err(AIError::Serialization)?;
        
        // Create backup of existing file
        if self.config_path.exists() {
            let backup_path = self.config_path.with_extension("json.bak");
            if let Err(e) = fs::copy(&self.config_path, &backup_path) {
                log::warn!("Failed to create backup: {}", e);
            }
        }
        
        fs::write(&self.config_path, content)
            .map_err(|e| AIError::Configuration(format!("Failed to write config file: {}", e)))?;
        
        log::debug!("Saved configuration to disk");
        Ok(())
    }
    
    /// Create a backup of the configuration
    pub async fn backup(&self) -> Result<PathBuf, AIError> {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let backup_path = self.config_path.with_file_name(format!("ai_config_backup_{}.json", timestamp));
        
        fs::copy(&self.config_path, &backup_path)
            .map_err(|e| AIError::Configuration(format!("Failed to create backup: {}", e)))?;
        
        {
            let mut stats = self.stats.write();
            stats.last_backup = Some(Utc::now());
        }
        
        info!("Created configuration backup: {:?}", backup_path);
        Ok(backup_path)
    }
    
    /// Restore configuration from backup
    pub async fn restore(&self, backup_path: &Path) -> Result<(), AIError> {
        if !backup_path.exists() {
            return Err(AIError::Configuration(format!("Backup file not found: {:?}", backup_path)));
        }
        
        fs::copy(backup_path, &self.config_path)
            .map_err(|e| AIError::Configuration(format!("Failed to restore from backup: {}", e)))?;
        
        // Reload from disk
        self.load_from_disk().await?;
        
        info!("Restored configuration from backup: {:?}", backup_path);
        Ok(())
    }
    
    /// Get configuration statistics
    pub fn get_config_stats(&self) -> ConfigStats {
        let stats = self.stats.read();
        ConfigStats {
            reads: stats.reads,
            writes: stats.writes,
            cache_hits: stats.cache_hits,
            cache_misses: stats.cache_misses,
            total_size: stats.total_size,
            last_backup: stats.last_backup,
        }
    }
    
    /// Validate configuration integrity
    pub fn validate(&self) -> Result<ValidationReport, AIError> {
        let cache = self.cache.read();
        let mut report = ValidationReport {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            total_keys: cache.len(),
        };
        
        for (key, config_value) in cache.iter() {
            // Check for required fields
            if config_value.category.is_empty() {
                report.warnings.push(format!("Configuration key '{}' has empty category", key));
            }
            
            // Check for old values
            let age = Utc::now() - config_value.created_at;
            if age > chrono::Duration::days(365) {
                report.warnings.push(format!("Configuration key '{}' is over 1 year old", key));
            }
            
            // Validate JSON structure
            if let Err(e) = serde_json::to_string(&config_value.value) {
                report.errors.push(format!("Configuration key '{}' has invalid JSON: {}", key, e));
                report.valid = false;
            }
        }
        
        Ok(report)
    }
    
    /// Clean up old configuration values
    pub async fn cleanup_old_values(&self, max_age: chrono::Duration) -> Result<u64, AIError> {
        let cutoff = Utc::now() - max_age;
        let mut removed_count = 0;
        
        {
            let mut cache = self.cache.write();
            let keys_to_remove: Vec<String> = cache.iter()
                .filter(|(_, config_value)| config_value.created_at < cutoff)
                .map(|(key, _)| key.clone())
                .collect();
            
            for key in keys_to_remove {
                cache.remove(&key);
                removed_count += 1;
            }
        }
        
        if removed_count > 0 {
            self.save_to_disk().await?;
            info!("Cleaned up {} old configuration values", removed_count);
        }
        
        Ok(removed_count)
    }
}

#[async_trait]
impl Storage for ConfigStorage {
    async fn initialize(&self) -> Result<(), AIError> {
        // Already initialized in new()
        Ok(())
    }
    
    async fn health_check(&self) -> Result<StorageHealth, AIError> {
        let start_time = std::time::Instant::now();
        
        // Check if config file is accessible
        let healthy = self.config_path.parent().map_or(false, |p| p.exists());
        
        let response_time = start_time.elapsed().as_millis() as u64;
        
        Ok(StorageHealth {
            healthy,
            response_time,
            error_message: if healthy { None } else { Some("Config directory not accessible".to_string()) },
            last_check: Utc::now(),
            storage_type: "Configuration".to_string(),
        })
    }
    
    async fn get_stats(&self) -> Result<StorageStats, AIError> {
        let stats = self.stats.read();
        let cache = self.cache.read();
        
        let file_size = if self.config_path.exists() {
            fs::metadata(&self.config_path)
                .map(|m| m.len())
                .unwrap_or(0)
        } else {
            0
        };
        
        let mut metadata = HashMap::new();
        metadata.insert("config_keys".to_string(), cache.len().to_string());
        metadata.insert("cache_hits".to_string(), stats.cache_hits.to_string());
        metadata.insert("cache_misses".to_string(), stats.cache_misses.to_string());
        metadata.insert("config_file_path".to_string(), self.config_path.to_string_lossy().to_string());
        
        Ok(StorageStats {
            total_records: cache.len() as u64,
            storage_size: file_size,
            avg_query_time: 0.05, // Config access is very fast
            queries_last_hour: stats.reads,
            utilization: 0.0, // Not applicable for file storage
            metadata,
        })
    }
    
    async fn cleanup(&self, retention_policy: RetentionPolicy) -> Result<u64, AIError> {
        self.cleanup_old_values(retention_policy.detailed_retention).await
    }
}

/// Configuration validation report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    /// Whether the configuration is valid
    pub valid: bool,
    /// List of errors found
    pub errors: Vec<String>,
    /// List of warnings
    pub warnings: Vec<String>,
    /// Total number of configuration keys
    pub total_keys: usize,
}

impl Clone for ConfigStats {
    fn clone(&self) -> Self {
        Self {
            reads: self.reads,
            writes: self.writes,
            cache_hits: self.cache_hits,
            cache_misses: self.cache_misses,
            total_size: self.total_size,
            last_backup: self.last_backup,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_config_storage_basic_operations() {
        let _temp_dir = TempDir::new().unwrap();
        let storage_config = StorageConfig::default();
        
        let storage = ConfigStorage::new(storage_config).await.unwrap();
        
        // Test set and get
        storage.set("test_key", &"test_value", "test_category", Some("Test description".to_string())).await.unwrap();
        
        let result: Option<String> = storage.get("test_key");
        assert_eq!(result, Some("test_value".to_string()));
        
        // Test get with metadata
        let metadata = storage.get_with_metadata("test_key").unwrap();
        assert_eq!(metadata.category, "test_category");
        assert_eq!(metadata.description, Some("Test description".to_string()));
        
        // Test remove
        assert!(storage.remove("test_key").await.unwrap());
        let result: Option<String> = storage.get("test_key");
        assert!(result.is_none());
    }
    
    #[tokio::test]
    async fn test_config_validation() {
        let _temp_dir = TempDir::new().unwrap();
        let storage_config = StorageConfig::default();
        let storage = ConfigStorage::new(storage_config).await.unwrap();
        
        // Add some test configuration
        storage.set("valid_key", &"valid_value", "test", None).await.unwrap();
        
        let report = storage.validate().unwrap();
        assert!(report.valid);
        assert_eq!(report.total_keys, 1);
    }
}