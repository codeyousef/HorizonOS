//! Agent memory and context management system
//! 
//! This module provides comprehensive memory management for AI agents, including
//! short-term, long-term, episodic, and semantic memory systems with context-aware retrieval.

use crate::AIError;
use crate::agents::langchain::{MemoryEntry, MemoryType};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, BinaryHeap};
use std::sync::Arc;
use parking_lot::RwLock;
use chrono::{DateTime, Utc, Duration};
use log::{info, debug};

/// Memory management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryManagerConfig {
    /// Enable memory management
    pub enabled: bool,
    /// Maximum memory entries per type
    pub max_entries_per_type: usize,
    /// Memory retention period (hours)
    pub retention_hours: u32,
    /// Enable semantic similarity search
    pub enable_semantic_search: bool,
    /// Similarity threshold for retrieval
    pub similarity_threshold: f32,
    /// Enable memory consolidation
    pub enable_consolidation: bool,
    /// Consolidation interval (hours)
    pub consolidation_interval: u32,
    /// Enable memory compression
    pub enable_compression: bool,
    /// Compression threshold (memory usage percentage)
    pub compression_threshold: f32,
    /// Enable persistent storage
    pub enable_persistent_storage: bool,
    /// Storage directory
    pub storage_dir: String,
    /// Memory indexing strategy
    pub indexing_strategy: IndexingStrategy,
    /// Context window size
    pub context_window_size: usize,
    /// Enable memory importance scoring
    pub enable_importance_scoring: bool,
    /// Importance decay rate
    pub importance_decay_rate: f32,
}

impl Default for MemoryManagerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_entries_per_type: 10000,
            retention_hours: 24 * 30, // 30 days
            enable_semantic_search: true,
            similarity_threshold: 0.8,
            enable_consolidation: true,
            consolidation_interval: 24,
            enable_compression: true,
            compression_threshold: 0.8,
            enable_persistent_storage: true,
            storage_dir: "/tmp/horizonos/agent_memory".to_string(),
            indexing_strategy: IndexingStrategy::Hybrid,
            context_window_size: 50,
            enable_importance_scoring: true,
            importance_decay_rate: 0.1,
        }
    }
}

/// Memory indexing strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IndexingStrategy {
    /// Text-based indexing
    TextBased,
    /// Semantic embedding indexing
    Semantic,
    /// Time-based indexing
    Temporal,
    /// Hybrid indexing (combines multiple strategies)
    Hybrid,
}

/// Memory search query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryQuery {
    /// Query text
    pub query: String,
    /// Memory types to search
    pub memory_types: Vec<MemoryType>,
    /// Maximum results
    pub max_results: usize,
    /// Minimum similarity threshold
    pub min_similarity: f32,
    /// Time range filter
    pub time_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    /// Importance threshold
    pub min_importance: Option<f32>,
    /// Search metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Memory search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySearchResult {
    /// Search results
    pub results: Vec<MemoryMatch>,
    /// Total matches found
    pub total_matches: usize,
    /// Search time
    pub search_time: std::time::Duration,
    /// Search metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Memory match
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMatch {
    /// Memory entry
    pub entry: MemoryEntry,
    /// Similarity score
    pub similarity: f32,
    /// Relevance score
    pub relevance: f32,
    /// Context information
    pub context: MemoryContext,
}

/// Memory context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryContext {
    /// Related memories
    pub related_memories: Vec<String>,
    /// Contextual tags
    pub tags: Vec<String>,
    /// Context metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Memory consolidation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationResult {
    /// Consolidated entries
    pub consolidated_entries: Vec<MemoryEntry>,
    /// Removed entries
    pub removed_entries: Vec<String>,
    /// Consolidation statistics
    pub stats: ConsolidationStats,
}

/// Consolidation statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationStats {
    /// Entries processed
    pub entries_processed: usize,
    /// Entries consolidated
    pub entries_consolidated: usize,
    /// Entries removed
    pub entries_removed: usize,
    /// Space saved (bytes)
    pub space_saved: usize,
    /// Processing time
    pub processing_time: std::time::Duration,
}

/// Memory importance scorer
#[derive(Debug, Clone)]
pub struct ImportanceScorer {
    /// Scoring weights
    pub weights: ImportanceWeights,
    /// Decay configuration
    pub decay_config: DecayConfig,
}

/// Importance scoring weights
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportanceWeights {
    /// Recency weight
    pub recency: f32,
    /// Frequency weight
    pub frequency: f32,
    /// Relevance weight
    pub relevance: f32,
    /// Emotional weight
    pub emotional: f32,
    /// Context weight
    pub context: f32,
}

impl Default for ImportanceWeights {
    fn default() -> Self {
        Self {
            recency: 0.3,
            frequency: 0.2,
            relevance: 0.3,
            emotional: 0.1,
            context: 0.1,
        }
    }
}

/// Decay configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecayConfig {
    /// Decay rate
    pub rate: f32,
    /// Decay function
    pub function: DecayFunction,
    /// Minimum importance threshold
    pub min_importance: f32,
}

/// Decay function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DecayFunction {
    /// Linear decay
    Linear,
    /// Exponential decay
    Exponential,
    /// Logarithmic decay
    Logarithmic,
    /// Power decay
    Power(f32),
}

impl Default for DecayConfig {
    fn default() -> Self {
        Self {
            rate: 0.1,
            function: DecayFunction::Exponential,
            min_importance: 0.1,
        }
    }
}

/// Memory manager
pub struct MemoryManager {
    /// Configuration
    config: Arc<RwLock<MemoryManagerConfig>>,
    /// Memory storage by type
    memories: Arc<RwLock<HashMap<MemoryType, Vec<MemoryEntry>>>>,
    /// Memory index
    index: Arc<RwLock<MemoryIndex>>,
    /// Importance scorer
    importance_scorer: Arc<ImportanceScorer>,
    /// Consolidation task handle
    consolidation_task: Option<tokio::task::JoinHandle<()>>,
    /// Memory maintenance task handle
    maintenance_task: Option<tokio::task::JoinHandle<()>>,
    /// Memory statistics
    stats: Arc<RwLock<MemoryStats>>,
}

/// Memory index
#[derive(Debug, Default)]
struct MemoryIndex {
    /// Text-based index
    text_index: HashMap<String, Vec<String>>,
    /// Semantic index
    semantic_index: HashMap<String, Vec<(String, f32)>>,
    /// Temporal index
    temporal_index: BinaryHeap<TemporalEntry>,
    /// Importance index
    importance_index: BinaryHeap<ImportanceEntry>,
}

/// Temporal index entry
#[derive(Debug, Clone, PartialEq, Eq)]
struct TemporalEntry {
    /// Memory entry ID
    entry_id: String,
    /// Timestamp
    timestamp: DateTime<Utc>,
}

impl Ord for TemporalEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.timestamp.cmp(&other.timestamp)
    }
}

impl PartialOrd for TemporalEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Importance index entry
#[derive(Debug, Clone, PartialEq)]
struct ImportanceEntry {
    /// Memory entry ID
    entry_id: String,
    /// Importance score
    importance: f32,
}

impl Ord for ImportanceEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.importance.partial_cmp(&other.importance).unwrap_or(std::cmp::Ordering::Equal)
    }
}

impl PartialOrd for ImportanceEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for ImportanceEntry {}

/// Memory statistics
#[derive(Debug, Default)]
pub struct MemoryStats {
    /// Total memory entries
    total_entries: usize,
    /// Entries by type
    entries_by_type: HashMap<MemoryType, usize>,
    /// Memory usage (bytes)
    memory_usage: usize,
    /// Total searches performed
    total_searches: u64,
    /// Average search time
    avg_search_time: f64,
    /// Cache hit rate
    cache_hit_rate: f64,
    /// Last consolidation time
    last_consolidation: Option<DateTime<Utc>>,
    /// Total consolidations
    total_consolidations: u64,
}

impl MemoryManager {
    /// Create a new memory manager
    pub async fn new(config: MemoryManagerConfig) -> Result<Self, AIError> {
        let manager = Self {
            config: Arc::new(RwLock::new(config.clone())),
            memories: Arc::new(RwLock::new(HashMap::new())),
            index: Arc::new(RwLock::new(MemoryIndex::default())),
            importance_scorer: Arc::new(ImportanceScorer {
                weights: ImportanceWeights::default(),
                decay_config: DecayConfig::default(),
            }),
            consolidation_task: None,
            maintenance_task: None,
            stats: Arc::new(RwLock::new(MemoryStats::default())),
        };
        
        // Initialize storage directory
        if config.enable_persistent_storage {
            tokio::fs::create_dir_all(&config.storage_dir).await
                .map_err(|e| AIError::Configuration(format!("Failed to create storage directory: {}", e)))?;
        }
        
        // Initialize memory types
        let mut memories = manager.memories.write();
        for memory_type in &[MemoryType::Conversation, MemoryType::Episodic, MemoryType::Semantic, MemoryType::Working] {
            memories.insert(memory_type.clone(), Vec::new());
        }
        drop(memories);
        
        info!("Memory manager initialized");
        Ok(manager)
    }
    
    /// Start memory management tasks
    pub async fn start(&mut self) -> Result<(), AIError> {
        if !self.config.read().enabled {
            return Ok(());
        }
        
        // Start consolidation task
        if self.config.read().enable_consolidation {
            let config = self.config.clone();
            let memories = self.memories.clone();
            let index = self.index.clone();
            let stats = self.stats.clone();
            
            self.consolidation_task = Some(tokio::spawn(async move {
                Self::consolidation_loop(config, memories, index, stats).await;
            }));
        }
        
        // Start maintenance task
        let config = self.config.clone();
        let memories = self.memories.clone();
        let index = self.index.clone();
        let importance_scorer = self.importance_scorer.clone();
        let stats = self.stats.clone();
        
        self.maintenance_task = Some(tokio::spawn(async move {
            Self::maintenance_loop(config, memories, index, importance_scorer, stats).await;
        }));
        
        info!("Memory management tasks started");
        Ok(())
    }
    
    /// Stop memory management tasks
    pub async fn stop(&mut self) -> Result<(), AIError> {
        if let Some(handle) = self.consolidation_task.take() {
            handle.abort();
        }
        
        if let Some(handle) = self.maintenance_task.take() {
            handle.abort();
        }
        
        info!("Memory management tasks stopped");
        Ok(())
    }
    
    /// Store a memory entry
    pub async fn store_memory(&self, entry: MemoryEntry) -> Result<(), AIError> {
        let mut memories = self.memories.write();
        let memory_type = entry.entry_type.clone();
        
        // Add to memory storage
        if let Some(type_memories) = memories.get_mut(&memory_type) {
            // Check capacity
            let max_entries = self.config.read().max_entries_per_type;
            if type_memories.len() >= max_entries {
                // Remove oldest entry
                type_memories.remove(0);
            }
            
            type_memories.push(entry.clone());
        }
        
        drop(memories);
        
        // Update index
        self.update_index(&entry).await?;
        
        // Update statistics
        let mut stats = self.stats.write();
        stats.total_entries += 1;
        stats.entries_by_type.entry(memory_type).and_modify(|c| *c += 1).or_insert(1);
        
        // Persist if enabled
        if self.config.read().enable_persistent_storage {
            self.persist_memory(&entry).await?;
        }
        
        debug!("Memory stored: {}", entry.id);
        Ok(())
    }
    
    /// Search memories
    pub async fn search_memories(&self, query: &MemoryQuery) -> Result<MemorySearchResult, AIError> {
        let start_time = std::time::Instant::now();
        
        let mut results = Vec::new();
        let memories = self.memories.read();
        
        // Search in specified memory types
        for memory_type in &query.memory_types {
            if let Some(type_memories) = memories.get(memory_type) {
                for entry in type_memories {
                    // Apply filters
                    if let Some((start, end)) = query.time_range {
                        if entry.timestamp < start || entry.timestamp > end {
                            continue;
                        }
                    }
                    
                    if let Some(min_importance) = query.min_importance {
                        if entry.importance < min_importance {
                            continue;
                        }
                    }
                    
                    // Calculate similarity
                    let similarity = self.calculate_similarity(&query.query, &entry.content).await?;
                    
                    if similarity >= query.min_similarity {
                        results.push(MemoryMatch {
                            entry: entry.clone(),
                            similarity,
                            relevance: similarity * entry.importance,
                            context: self.build_context(entry).await?,
                        });
                    }
                }
            }
        }
        
        // Sort by relevance
        results.sort_by(|a, b| b.relevance.partial_cmp(&a.relevance).unwrap_or(std::cmp::Ordering::Equal));
        
        // Limit results
        results.truncate(query.max_results);
        
        let search_time = start_time.elapsed();
        
        // Update statistics
        let mut stats = self.stats.write();
        stats.total_searches += 1;
        stats.avg_search_time = (stats.avg_search_time * (stats.total_searches - 1) as f64 + search_time.as_secs_f64()) / stats.total_searches as f64;
        
        let total_matches = results.len();
        Ok(MemorySearchResult {
            results,
            total_matches,
            search_time,
            metadata: HashMap::new(),
        })
    }
    
    /// Get memory by ID
    pub async fn get_memory(&self, entry_id: &str) -> Option<MemoryEntry> {
        let memories = self.memories.read();
        
        for type_memories in memories.values() {
            for entry in type_memories {
                if entry.id == entry_id {
                    return Some(entry.clone());
                }
            }
        }
        
        None
    }
    
    /// Update memory entry
    pub async fn update_memory(&self, entry: MemoryEntry) -> Result<(), AIError> {
        let mut memories = self.memories.write();
        let memory_type = entry.entry_type.clone();
        
        if let Some(type_memories) = memories.get_mut(&memory_type) {
            for existing_entry in type_memories.iter_mut() {
                if existing_entry.id == entry.id {
                    *existing_entry = entry.clone();
                    break;
                }
            }
        }
        
        drop(memories);
        
        // Update index
        self.update_index(&entry).await?;
        
        // Persist if enabled
        if self.config.read().enable_persistent_storage {
            self.persist_memory(&entry).await?;
        }
        
        debug!("Memory updated: {}", entry.id);
        Ok(())
    }
    
    /// Delete memory entry
    pub async fn delete_memory(&self, entry_id: &str) -> Result<(), AIError> {
        let mut memories = self.memories.write();
        let mut found = false;
        
        for type_memories in memories.values_mut() {
            if let Some(index) = type_memories.iter().position(|e| e.id == entry_id) {
                type_memories.remove(index);
                found = true;
                break;
            }
        }
        
        if !found {
            return Err(AIError::Configuration(format!("Memory not found: {}", entry_id)));
        }
        
        drop(memories);
        
        // Remove from index
        self.remove_from_index(entry_id).await?;
        
        // Update statistics
        let mut stats = self.stats.write();
        stats.total_entries = stats.total_entries.saturating_sub(1);
        
        debug!("Memory deleted: {}", entry_id);
        Ok(())
    }
    
    /// Get memory statistics
    pub fn get_stats(&self) -> MemoryStats {
        self.stats.read().clone()
    }
    
    /// Update configuration
    pub async fn update_config(&self, new_config: MemoryManagerConfig) -> Result<(), AIError> {
        *self.config.write() = new_config;
        info!("Memory manager configuration updated");
        Ok(())
    }
    
    /// Clear all memories
    pub async fn clear_memories(&self) -> Result<(), AIError> {
        let mut memories = self.memories.write();
        memories.clear();
        
        // Reinitialize memory types
        for memory_type in &[MemoryType::Conversation, MemoryType::Episodic, MemoryType::Semantic, MemoryType::Working] {
            memories.insert(memory_type.clone(), Vec::new());
        }
        
        drop(memories);
        
        // Clear index
        *self.index.write() = MemoryIndex::default();
        
        // Reset statistics
        *self.stats.write() = MemoryStats::default();
        
        info!("All memories cleared");
        Ok(())
    }
    
    /// Consolidation loop
    async fn consolidation_loop(
        config: Arc<RwLock<MemoryManagerConfig>>,
        memories: Arc<RwLock<HashMap<MemoryType, Vec<MemoryEntry>>>>,
        index: Arc<RwLock<MemoryIndex>>,
        stats: Arc<RwLock<MemoryStats>>,
    ) {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3600)); // 1 hour
        
        info!("Memory consolidation loop started");
        
        loop {
            interval.tick().await;
            
            let consolidation_interval = config.read().consolidation_interval;
            let last_consolidation = stats.read().last_consolidation;
            
            // Check if consolidation is needed
            if let Some(last) = last_consolidation {
                if Utc::now() - last < Duration::hours(consolidation_interval as i64) {
                    continue;
                }
            }
            
            // Perform consolidation
            debug!("Starting memory consolidation");
            
            // TODO: Implement actual consolidation logic
            // This would involve:
            // 1. Identifying similar memories
            // 2. Merging related entries
            // 3. Removing redundant information
            // 4. Updating indices
            
            // Update statistics
            let mut stats = stats.write();
            stats.last_consolidation = Some(Utc::now());
            stats.total_consolidations += 1;
            
            info!("Memory consolidation completed");
        }
    }
    
    /// Maintenance loop
    async fn maintenance_loop(
        config: Arc<RwLock<MemoryManagerConfig>>,
        memories: Arc<RwLock<HashMap<MemoryType, Vec<MemoryEntry>>>>,
        index: Arc<RwLock<MemoryIndex>>,
        importance_scorer: Arc<ImportanceScorer>,
        stats: Arc<RwLock<MemoryStats>>,
    ) {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(900)); // 15 minutes
        
        info!("Memory maintenance loop started");
        
        loop {
            interval.tick().await;
            
            // Update importance scores
            if config.read().enable_importance_scoring {
                Self::update_importance_scores(&memories, &importance_scorer).await;
            }
            
            // Remove expired memories
            let retention_hours = config.read().retention_hours;
            Self::remove_expired_memories(&memories, retention_hours).await;
            
            // Update statistics
            Self::update_memory_stats(&memories, &stats).await;
        }
    }
    
    /// Update importance scores
    async fn update_importance_scores(
        memories: &Arc<RwLock<HashMap<MemoryType, Vec<MemoryEntry>>>>,
        importance_scorer: &Arc<ImportanceScorer>,
    ) {
        let mut memories = memories.write();
        
        for type_memories in memories.values_mut() {
            for entry in type_memories.iter_mut() {
                // Calculate new importance score
                let age = Utc::now() - entry.timestamp;
                let decay_factor = Self::calculate_decay_factor(age, &importance_scorer.decay_config);
                entry.importance = (entry.importance * decay_factor).max(importance_scorer.decay_config.min_importance);
            }
        }
    }
    
    /// Remove expired memories
    async fn remove_expired_memories(
        memories: &Arc<RwLock<HashMap<MemoryType, Vec<MemoryEntry>>>>,
        retention_hours: u32,
    ) {
        let mut memories = memories.write();
        let cutoff = Utc::now() - Duration::hours(retention_hours as i64);
        
        for type_memories in memories.values_mut() {
            type_memories.retain(|entry| entry.timestamp > cutoff);
        }
    }
    
    /// Update memory statistics
    async fn update_memory_stats(
        memories: &Arc<RwLock<HashMap<MemoryType, Vec<MemoryEntry>>>>,
        stats: &Arc<RwLock<MemoryStats>>,
    ) {
        let memories = memories.read();
        let mut stats = stats.write();
        
        stats.total_entries = 0;
        stats.entries_by_type.clear();
        
        for (memory_type, type_memories) in memories.iter() {
            let count = type_memories.len();
            stats.total_entries += count;
            stats.entries_by_type.insert(memory_type.clone(), count);
        }
    }
    
    /// Calculate decay factor
    fn calculate_decay_factor(age: Duration, decay_config: &DecayConfig) -> f32 {
        let hours = age.num_hours() as f32;
        
        match decay_config.function {
            DecayFunction::Linear => (1.0 - decay_config.rate * hours).max(0.0),
            DecayFunction::Exponential => (-decay_config.rate * hours).exp(),
            DecayFunction::Logarithmic => (1.0 / (1.0 + decay_config.rate * hours)),
            DecayFunction::Power(power) => (1.0 + decay_config.rate * hours).powf(-power),
        }
    }
    
    /// Update memory index
    async fn update_index(&self, entry: &MemoryEntry) -> Result<(), AIError> {
        let mut index = self.index.write();
        
        // Update text index
        let words: Vec<String> = entry.content.split_whitespace().map(|w| w.to_lowercase()).collect();
        for word in words {
            index.text_index.entry(word).or_insert_with(Vec::new).push(entry.id.clone());
        }
        
        // Update temporal index
        index.temporal_index.push(TemporalEntry {
            entry_id: entry.id.clone(),
            timestamp: entry.timestamp,
        });
        
        // Update importance index
        index.importance_index.push(ImportanceEntry {
            entry_id: entry.id.clone(),
            importance: entry.importance,
        });
        
        Ok(())
    }
    
    /// Remove from index
    async fn remove_from_index(&self, entry_id: &str) -> Result<(), AIError> {
        let mut index = self.index.write();
        
        // Remove from text index
        for word_entries in index.text_index.values_mut() {
            word_entries.retain(|id| id != entry_id);
        }
        
        // Note: For heap-based indices, we would need to rebuild or use tombstones
        // For now, we'll let the maintenance loop handle cleanup
        
        Ok(())
    }
    
    /// Calculate similarity between query and content
    async fn calculate_similarity(&self, query: &str, content: &str) -> Result<f32, AIError> {
        // Simple word-based similarity for now
        let query_words: std::collections::HashSet<String> = query.split_whitespace()
            .map(|w| w.to_lowercase())
            .collect();
        
        let content_words: std::collections::HashSet<String> = content.split_whitespace()
            .map(|w| w.to_lowercase())
            .collect();
        
        let intersection = query_words.intersection(&content_words).count();
        let union = query_words.union(&content_words).count();
        
        if union == 0 {
            Ok(0.0)
        } else {
            Ok(intersection as f32 / union as f32)
        }
    }
    
    /// Build context for memory entry
    async fn build_context(&self, entry: &MemoryEntry) -> Result<MemoryContext, AIError> {
        // TODO: Implement context building
        Ok(MemoryContext {
            related_memories: Vec::new(),
            tags: Vec::new(),
            metadata: HashMap::new(),
        })
    }
    
    /// Persist memory to storage
    async fn persist_memory(&self, entry: &MemoryEntry) -> Result<(), AIError> {
        // TODO: Implement persistence
        Ok(())
    }
}

impl Clone for MemoryStats {
    fn clone(&self) -> Self {
        Self {
            total_entries: self.total_entries,
            entries_by_type: self.entries_by_type.clone(),
            memory_usage: self.memory_usage,
            total_searches: self.total_searches,
            avg_search_time: self.avg_search_time,
            cache_hit_rate: self.cache_hit_rate,
            last_consolidation: self.last_consolidation,
            total_consolidations: self.total_consolidations,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    
    #[test]
    fn test_memory_manager_config_default() {
        let config = MemoryManagerConfig::default();
        assert!(config.enabled);
        assert_eq!(config.max_entries_per_type, 10000);
        assert_eq!(config.retention_hours, 24 * 30);
        assert!(config.enable_semantic_search);
        assert_eq!(config.similarity_threshold, 0.8);
        assert!(config.enable_consolidation);
    }
    
    #[test]
    fn test_memory_query_creation() {
        let query = MemoryQuery {
            query: "test query".to_string(),
            memory_types: vec![MemoryType::Conversation, MemoryType::Episodic],
            max_results: 10,
            min_similarity: 0.5,
            time_range: None,
            min_importance: Some(0.3),
            metadata: HashMap::new(),
        };
        
        assert_eq!(query.query, "test query");
        assert_eq!(query.memory_types.len(), 2);
        assert_eq!(query.max_results, 10);
        assert_eq!(query.min_similarity, 0.5);
        assert_eq!(query.min_importance, Some(0.3));
    }
    
    #[test]
    fn test_importance_scorer() {
        let scorer = ImportanceScorer {
            weights: ImportanceWeights::default(),
            decay_config: DecayConfig::default(),
        };
        
        assert_eq!(scorer.weights.recency, 0.3);
        assert_eq!(scorer.weights.frequency, 0.2);
        assert_eq!(scorer.weights.relevance, 0.3);
        assert_eq!(scorer.decay_config.rate, 0.1);
        assert!(matches!(scorer.decay_config.function, DecayFunction::Exponential));
    }
    
    #[test]
    fn test_decay_factor_calculation() {
        let decay_config = DecayConfig {
            rate: 0.1,
            function: DecayFunction::Exponential,
            min_importance: 0.1,
        };
        
        let age = Duration::hours(24);
        let factor = MemoryManager::calculate_decay_factor(age, &decay_config);
        
        assert!(factor > 0.0);
        assert!(factor < 1.0);
    }
}