//! Cache management for AI operations
//! 
//! This module provides in-memory caching with various eviction policies
//! to improve performance for frequently accessed data.

use super::{Storage, StorageHealth, StorageStats, RetentionPolicy};
use crate::AIError;

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use log::{debug, error, info};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Maximum cache size in bytes
    pub max_size: usize,
    /// Maximum number of items in cache
    pub max_items: usize,
    /// Default TTL for cache entries
    pub default_ttl: Duration,
    /// Eviction policy to use
    pub eviction_policy: EvictionPolicy,
    /// Enable cache statistics
    pub enable_stats: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_size: 100 * 1024 * 1024, // 100MB
            max_items: 10000,
            default_ttl: Duration::hours(1),
            eviction_policy: EvictionPolicy::LRU,
            enable_stats: true,
        }
    }
}

/// Cache eviction policies
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum EvictionPolicy {
    /// Least Recently Used
    LRU,
    /// Least Frequently Used
    LFU,
    /// Time To Live based
    TTL,
    /// Random eviction
    Random,
}

/// Cached item with metadata
#[derive(Debug)]
struct CachedItem {
    data: Vec<u8>,
    created_at: DateTime<Utc>,
    last_accessed: DateTime<Utc>,
    access_count: u64,
    ttl: Duration,
    size: usize,
}

/// Cache statistics
#[derive(Debug, Default)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub total_size: usize,
    pub item_count: usize,
    pub hit_ratio: f64,
}

/// In-memory cache manager
pub struct CacheManager {
    cache: Arc<RwLock<HashMap<String, CachedItem>>>,
    config: CacheConfig,
    stats: Arc<RwLock<CacheStats>>,
    access_order: Arc<RwLock<Vec<String>>>,
}

impl CacheManager {
    /// Create a new cache manager with default configuration (placeholder)
    pub fn new_default() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            config: CacheConfig::default(),
            stats: Arc::new(RwLock::new(CacheStats::default())),
            access_order: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Create a new cache manager
    pub async fn new(config: CacheConfig) -> Result<Self, AIError> {
        info!("Initializing cache manager with max size: {} MB", config.max_size / (1024 * 1024));
        
        Ok(Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            config,
            stats: Arc::new(RwLock::new(CacheStats::default())),
            access_order: Arc::new(RwLock::new(Vec::new())),
        })
    }
    
    /// Get an item from cache
    pub fn get<T>(&self, key: &str) -> Option<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let mut cache = self.cache.write();
        let mut stats = self.stats.write();
        
        if let Some(item) = cache.get_mut(key) {
            // Check if item has expired
            if Utc::now() - item.created_at > item.ttl {
                cache.remove(key);
                stats.misses += 1;
                return None;
            }
            
            // Update access statistics
            item.last_accessed = Utc::now();
            item.access_count += 1;
            
            // Update LRU order
            drop(cache);
            drop(stats);
            self.update_lru_order(key);
            
            // Deserialize data
            let cache = self.cache.read();
            if let Some(item) = cache.get(key) {
                match bincode::deserialize(&item.data) {
                    Ok(data) => {
                        let mut stats = self.stats.write();
                        stats.hits += 1;
                        stats.hit_ratio = stats.hits as f64 / (stats.hits + stats.misses) as f64;
                        Some(data)
                    }
                    Err(e) => {
                        error!("Failed to deserialize cached item: {}", e);
                        let mut stats = self.stats.write();
                        stats.misses += 1;
                        None
                    }
                }
            } else {
                None
            }
        } else {
            stats.misses += 1;
            stats.hit_ratio = stats.hits as f64 / (stats.hits + stats.misses) as f64;
            None
        }
    }
    
    /// Put an item in cache
    pub fn put<T>(&self, key: &str, value: &T, ttl: Option<Duration>) -> Result<(), AIError>
    where
        T: Serialize,
    {
        let data = bincode::serialize(value)
            .map_err(|e| AIError::Configuration(format!("Failed to serialize cache item: {}", e)))?;
        
        let size = data.len();
        let ttl = ttl.unwrap_or(self.config.default_ttl);
        
        let item = CachedItem {
            data,
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            access_count: 0,
            ttl,
            size,
        };
        
        let mut cache = self.cache.write();
        let mut stats = self.stats.write();
        
        // Check if we need to evict items
        let new_total_size = stats.total_size + size;
        if new_total_size > self.config.max_size || stats.item_count >= self.config.max_items {
            drop(cache);
            drop(stats);
            self.evict_items(size);
            cache = self.cache.write();
            stats = self.stats.write();
        }
        
        // Insert the item
        cache.insert(key.to_string(), item);
        stats.total_size += size;
        stats.item_count += 1;
        
        // Update LRU order
        drop(cache);
        drop(stats);
        self.update_lru_order(key);
        
        debug!("Cached item: {} (size: {} bytes)", key, size);
        Ok(())
    }
    
    /// Remove an item from cache
    pub fn remove(&self, key: &str) -> bool {
        let mut cache = self.cache.write();
        let mut stats = self.stats.write();
        
        if let Some(item) = cache.remove(key) {
            stats.total_size = stats.total_size.saturating_sub(item.size);
            stats.item_count = stats.item_count.saturating_sub(1);
            
            // Remove from LRU order
            let mut access_order = self.access_order.write();
            access_order.retain(|k| k != key);
            
            true
        } else {
            false
        }
    }
    
    /// Clear all items from cache
    pub fn clear(&self) {
        let mut cache = self.cache.write();
        let mut stats = self.stats.write();
        let mut access_order = self.access_order.write();
        
        cache.clear();
        access_order.clear();
        stats.total_size = 0;
        stats.item_count = 0;
        
        info!("Cache cleared");
    }
    
    /// Get cache statistics
    pub fn get_cache_stats(&self) -> CacheStats {
        let stats = self.stats.read();
        CacheStats {
            hits: stats.hits,
            misses: stats.misses,
            evictions: stats.evictions,
            total_size: stats.total_size,
            item_count: stats.item_count,
            hit_ratio: stats.hit_ratio,
        }
    }
    
    /// Update LRU access order
    fn update_lru_order(&self, key: &str) {
        let mut access_order = self.access_order.write();
        
        // Remove existing entry
        access_order.retain(|k| k != key);
        
        // Add to front (most recently used)
        access_order.insert(0, key.to_string());
    }
    
    /// Evict items to make space
    fn evict_items(&self, needed_space: usize) {
        let mut cache = self.cache.write();
        let mut stats = self.stats.write();
        
        let keys_to_evict = self.select_eviction_candidates(needed_space);
        
        for key in keys_to_evict {
            if let Some(item) = cache.remove(&key) {
                stats.total_size = stats.total_size.saturating_sub(item.size);
                stats.item_count = stats.item_count.saturating_sub(1);
                stats.evictions += 1;
                
                debug!("Evicted cache item: {} (size: {} bytes)", key, item.size);
            }
        }
    }
    
    /// Select candidates for eviction based on policy
    fn select_eviction_candidates(&self, needed_space: usize) -> Vec<String> {
        let cache = self.cache.read();
        let mut candidates = Vec::new();
        let mut freed_space = 0;
        
        match self.config.eviction_policy {
            EvictionPolicy::LRU => {
                let access_order = self.access_order.read();
                
                // Evict from least recently used (end of list)
                for key in access_order.iter().rev() {
                    if let Some(item) = cache.get(key) {
                        candidates.push(key.clone());
                        freed_space += item.size;
                        
                        if freed_space >= needed_space {
                            break;
                        }
                    }
                }
            }
            EvictionPolicy::LFU => {
                // Sort by access count (ascending)
                let mut items: Vec<_> = cache.iter().collect();
                items.sort_by_key(|(_, item)| item.access_count);
                
                for (key, item) in items {
                    candidates.push(key.clone());
                    freed_space += item.size;
                    
                    if freed_space >= needed_space {
                        break;
                    }
                }
            }
            EvictionPolicy::TTL => {
                // Evict expired items first
                let now = Utc::now();
                for (key, item) in cache.iter() {
                    if now - item.created_at > item.ttl {
                        candidates.push(key.clone());
                        freed_space += item.size;
                        
                        if freed_space >= needed_space {
                            break;
                        }
                    }
                }
            }
            EvictionPolicy::Random => {
                // Random eviction
                use rand::seq::SliceRandom;
                let mut rng = rand::thread_rng();
                let mut keys: Vec<_> = cache.keys().cloned().collect();
                keys.shuffle(&mut rng);
                
                for key in keys {
                    if let Some(item) = cache.get(&key) {
                        candidates.push(key);
                        freed_space += item.size;
                        
                        if freed_space >= needed_space {
                            break;
                        }
                    }
                }
            }
        }
        
        candidates
    }
    
    /// Clean up expired items
    pub fn cleanup_expired(&self) -> u64 {
        let mut cache = self.cache.write();
        let mut stats = self.stats.write();
        let mut access_order = self.access_order.write();
        
        let now = Utc::now();
        let mut expired_keys = Vec::new();
        
        for (key, item) in cache.iter() {
            if now - item.created_at > item.ttl {
                expired_keys.push(key.clone());
            }
        }
        
        let expired_count = expired_keys.len() as u64;
        
        for key in expired_keys {
            if let Some(item) = cache.remove(&key) {
                stats.total_size = stats.total_size.saturating_sub(item.size);
                stats.item_count = stats.item_count.saturating_sub(1);
                access_order.retain(|k| k != &key);
            }
        }
        
        if expired_count > 0 {
            info!("Cleaned up {} expired cache items", expired_count);
        }
        
        expired_count
    }
    
    /// Get semantic hash for content-based caching
    pub fn semantic_hash<T: Hash>(content: &T) -> String {
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
    
    /// Cache with semantic similarity checking
    pub fn get_similar<T>(&self, content: &T, _similarity_threshold: f64) -> Option<T>
    where
        T: for<'de> Deserialize<'de> + Hash + Clone,
    {
        let content_hash = Self::semantic_hash(content);
        
        // First try exact match
        if let Some(result) = self.get::<T>(&content_hash) {
            return Some(result);
        }
        
        // TODO: Implement semantic similarity search
        // This would require more sophisticated similarity algorithms
        None
    }
    
    /// Put with semantic hashing
    pub fn put_semantic<T>(&self, content: &T, value: &T, ttl: Option<Duration>) -> Result<(), AIError>
    where
        T: Serialize + Hash,
    {
        let content_hash = Self::semantic_hash(content);
        self.put(&content_hash, value, ttl)
    }
}

#[async_trait]
impl Storage for CacheManager {
    async fn initialize(&self) -> Result<(), AIError> {
        info!("Cache manager initialized successfully");
        Ok(())
    }
    
    async fn health_check(&self) -> Result<StorageHealth, AIError> {
        let start_time = std::time::Instant::now();
        
        // Simple health check - try to access cache
        let stats = self.stats.read();
        let healthy = stats.item_count <= self.config.max_items;
        
        let response_time = start_time.elapsed().as_millis() as u64;
        
        Ok(StorageHealth {
            healthy,
            response_time,
            error_message: if healthy { None } else { Some("Cache is overloaded".to_string()) },
            last_check: Utc::now(),
            storage_type: "Cache".to_string(),
        })
    }
    
    async fn get_stats(&self) -> Result<StorageStats, AIError> {
        let stats = self.stats.read();
        let mut metadata = HashMap::new();
        
        metadata.insert("hits".to_string(), stats.hits.to_string());
        metadata.insert("misses".to_string(), stats.misses.to_string());
        metadata.insert("evictions".to_string(), stats.evictions.to_string());
        metadata.insert("hit_ratio".to_string(), format!("{:.2}%", stats.hit_ratio * 100.0));
        
        Ok(StorageStats {
            total_records: stats.item_count as u64,
            storage_size: stats.total_size as u64,
            avg_query_time: 0.1, // Cache is very fast
            queries_last_hour: stats.hits + stats.misses,
            utilization: (stats.total_size as f32 / self.config.max_size as f32) * 100.0,
            metadata,
        })
    }
    
    async fn cleanup(&self, _retention_policy: RetentionPolicy) -> Result<u64, AIError> {
        let cleaned = self.cleanup_expired();
        Ok(cleaned)
    }
}

impl Clone for CacheStats {
    fn clone(&self) -> Self {
        Self {
            hits: self.hits,
            misses: self.misses,
            evictions: self.evictions,
            total_size: self.total_size,
            item_count: self.item_count,
            hit_ratio: self.hit_ratio,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_cache_basic_operations() {
        let config = CacheConfig::default();
        let cache = CacheManager::new(config).await.unwrap();
        
        // Test put and get
        cache.put("test_key", &"test_value", None).unwrap();
        let result: Option<String> = cache.get("test_key");
        assert_eq!(result, Some("test_value".to_string()));
        
        // Test remove
        assert!(cache.remove("test_key"));
        let result: Option<String> = cache.get("test_key");
        assert!(result.is_none());
    }
    
    #[tokio::test]
    async fn test_cache_expiration() {
        let config = CacheConfig::default();
        let cache = CacheManager::new(config).await.unwrap();
        
        // Put item with short TTL
        cache.put("test_key", &"test_value", Some(Duration::milliseconds(100))).unwrap();
        
        // Should exist immediately
        let result: Option<String> = cache.get("test_key");
        assert_eq!(result, Some("test_value".to_string()));
        
        // Wait for expiration
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        
        // Should be expired
        let result: Option<String> = cache.get("test_key");
        assert!(result.is_none());
    }
    
    #[test]
    fn test_semantic_hash() {
        let content1 = "hello world";
        let content2 = "hello world";
        let content3 = "different content";
        
        assert_eq!(CacheManager::semantic_hash(&content1), CacheManager::semantic_hash(&content2));
        assert_ne!(CacheManager::semantic_hash(&content1), CacheManager::semantic_hash(&content3));
    }
}