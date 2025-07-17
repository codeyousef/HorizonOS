//! Render cache system for reusing computed data

use horizonos_graph_engine::SceneId;
use crate::{PerformanceMetrics, lod::LodLevel};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::hash::{Hash, Hasher};

/// Render cache system
pub struct RenderCache {
    /// Geometry cache for nodes and edges
    geometry_cache: HashMap<GeometryCacheKey, GeometryCacheEntry>,
    /// Texture cache for icons and images
    texture_cache: HashMap<TextureCacheKey, TextureCacheEntry>,
    /// Shader uniform cache
    uniform_cache: HashMap<UniformCacheKey, UniformCacheEntry>,
    /// Layout cache for positioning
    layout_cache: HashMap<LayoutCacheKey, LayoutCacheEntry>,
    /// Cache statistics
    stats: CacheStats,
    /// Cache settings
    settings: CacheSettings,
    /// Last cleanup time
    last_cleanup: Instant,
}

/// Geometry cache key
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct GeometryCacheKey {
    /// Node or edge ID
    pub object_id: SceneId,
    /// LOD level
    pub lod_level: LodLevel,
    /// Object type hash
    pub type_hash: u64,
    /// Size/scale hash
    pub size_hash: u64,
}

/// Geometry cache entry
#[derive(Debug, Clone)]
pub struct GeometryCacheEntry {
    /// Vertex data
    pub vertices: Vec<u8>,
    /// Index data
    pub indices: Vec<u32>,
    /// Creation time
    pub created_at: Instant,
    /// Last access time
    pub last_accessed: Instant,
    /// Access count
    pub access_count: u32,
    /// Data size in bytes
    pub size_bytes: usize,
}

/// Texture cache key
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct TextureCacheKey {
    /// Texture path or identifier
    pub texture_id: String,
    /// Resolution level
    pub resolution: u32,
    /// Format hash
    pub format_hash: u64,
}

/// Texture cache entry
#[derive(Debug, Clone)]
pub struct TextureCacheEntry {
    /// Texture data
    pub data: Vec<u8>,
    /// Texture dimensions
    pub width: u32,
    pub height: u32,
    /// Creation time
    pub created_at: Instant,
    /// Last access time
    pub last_accessed: Instant,
    /// Access count
    pub access_count: u32,
    /// Data size in bytes
    pub size_bytes: usize,
}

/// Shader uniform cache key
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct UniformCacheKey {
    /// Shader program hash
    pub shader_hash: u64,
    /// Uniform data hash
    pub data_hash: u64,
    /// Frame number (for temporal caching)
    pub frame: u64,
}

/// Shader uniform cache entry
#[derive(Debug, Clone)]
pub struct UniformCacheEntry {
    /// Uniform buffer data
    pub data: Vec<u8>,
    /// Creation time
    pub created_at: Instant,
    /// Last access time
    pub last_accessed: Instant,
    /// Access count
    pub access_count: u32,
}

/// Layout cache key
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct LayoutCacheKey {
    /// Layout algorithm hash
    pub algorithm_hash: u64,
    /// Node set hash
    pub node_set_hash: u64,
    /// Parameter hash
    pub parameter_hash: u64,
}

/// Layout cache entry
#[derive(Debug, Clone)]
pub struct LayoutCacheEntry {
    /// Node positions
    pub positions: HashMap<SceneId, [f32; 3]>,
    /// Creation time
    pub created_at: Instant,
    /// Last access time
    pub last_accessed: Instant,
    /// Access count
    pub access_count: u32,
    /// Quality score (higher = better layout)
    pub quality_score: f32,
}

/// Cache statistics
#[derive(Debug, Default)]
pub struct CacheStats {
    /// Total cache hits
    pub total_hits: u64,
    /// Total cache misses
    pub total_misses: u64,
    /// Geometry cache stats
    pub geometry_hits: u64,
    pub geometry_misses: u64,
    /// Texture cache stats
    pub texture_hits: u64,
    pub texture_misses: u64,
    /// Uniform cache stats
    pub uniform_hits: u64,
    pub uniform_misses: u64,
    /// Layout cache stats
    pub layout_hits: u64,
    pub layout_misses: u64,
    /// Total memory used by cache
    pub memory_used: usize,
    /// Number of entries evicted
    pub entries_evicted: u64,
}

/// Cache settings
#[derive(Debug, Clone)]
pub struct CacheSettings {
    /// Maximum memory for geometry cache (bytes)
    pub max_geometry_memory: usize,
    /// Maximum memory for texture cache (bytes)
    pub max_texture_memory: usize,
    /// Maximum memory for uniform cache (bytes)
    pub max_uniform_memory: usize,
    /// Maximum memory for layout cache (bytes)
    pub max_layout_memory: usize,
    /// Entry lifetime before expiration
    pub entry_lifetime: Duration,
    /// Cleanup interval
    pub cleanup_interval: Duration,
    /// Enable cache compression
    pub enable_compression: bool,
}

impl RenderCache {
    /// Create a new render cache
    pub fn new() -> Self {
        Self {
            geometry_cache: HashMap::new(),
            texture_cache: HashMap::new(),
            uniform_cache: HashMap::new(),
            layout_cache: HashMap::new(),
            stats: CacheStats::default(),
            settings: CacheSettings::default(),
            last_cleanup: Instant::now(),
        }
    }
    
    /// Update cache system
    pub fn update(&mut self, metrics: &PerformanceMetrics) {
        // Periodic cleanup
        if Instant::now().duration_since(self.last_cleanup) >= self.settings.cleanup_interval {
            self.cleanup_expired_entries();
            self.last_cleanup = Instant::now();
        }
        
        // Check memory pressure and evict if needed
        if self.get_total_memory_usage() > self.get_max_memory() {
            self.evict_least_recently_used();
        }
    }
    
    /// Get geometry from cache
    pub fn get_geometry(&mut self, key: &GeometryCacheKey) -> Option<&GeometryCacheEntry> {
        if let Some(entry) = self.geometry_cache.get_mut(key) {
            entry.last_accessed = Instant::now();
            entry.access_count += 1;
            self.stats.geometry_hits += 1;
            self.stats.total_hits += 1;
            Some(entry)
        } else {
            self.stats.geometry_misses += 1;
            self.stats.total_misses += 1;
            None
        }
    }
    
    /// Cache geometry data
    pub fn cache_geometry(&mut self, key: GeometryCacheKey, vertices: Vec<u8>, indices: Vec<u32>) {
        let size_bytes = vertices.len() + (indices.len() * 4);
        let entry = GeometryCacheEntry {
            vertices,
            indices,
            created_at: Instant::now(),
            last_accessed: Instant::now(),
            access_count: 1,
            size_bytes,
        };
        
        self.geometry_cache.insert(key, entry);
        self.stats.memory_used += size_bytes;
    }
    
    /// Get texture from cache
    pub fn get_texture(&mut self, key: &TextureCacheKey) -> Option<&TextureCacheEntry> {
        if let Some(entry) = self.texture_cache.get_mut(key) {
            entry.last_accessed = Instant::now();
            entry.access_count += 1;
            self.stats.texture_hits += 1;
            self.stats.total_hits += 1;
            Some(entry)
        } else {
            self.stats.texture_misses += 1;
            self.stats.total_misses += 1;
            None
        }
    }
    
    /// Cache texture data
    pub fn cache_texture(&mut self, key: TextureCacheKey, data: Vec<u8>, width: u32, height: u32) {
        let size_bytes = data.len();
        let entry = TextureCacheEntry {
            data,
            width,
            height,
            created_at: Instant::now(),
            last_accessed: Instant::now(),
            access_count: 1,
            size_bytes,
        };
        
        self.texture_cache.insert(key, entry);
        self.stats.memory_used += size_bytes;
    }
    
    /// Get uniform data from cache
    pub fn get_uniform(&mut self, key: &UniformCacheKey) -> Option<&UniformCacheEntry> {
        if let Some(entry) = self.uniform_cache.get_mut(key) {
            entry.last_accessed = Instant::now();
            entry.access_count += 1;
            self.stats.uniform_hits += 1;
            self.stats.total_hits += 1;
            Some(entry)
        } else {
            self.stats.uniform_misses += 1;
            self.stats.total_misses += 1;
            None
        }
    }
    
    /// Cache uniform data
    pub fn cache_uniform(&mut self, key: UniformCacheKey, data: Vec<u8>) {
        let size_bytes = data.len();
        let entry = UniformCacheEntry {
            data,
            created_at: Instant::now(),
            last_accessed: Instant::now(),
            access_count: 1,
        };
        
        self.uniform_cache.insert(key, entry);
        self.stats.memory_used += size_bytes;
    }
    
    /// Get layout from cache
    pub fn get_layout(&mut self, key: &LayoutCacheKey) -> Option<&LayoutCacheEntry> {
        if let Some(entry) = self.layout_cache.get_mut(key) {
            entry.last_accessed = Instant::now();
            entry.access_count += 1;
            self.stats.layout_hits += 1;
            self.stats.total_hits += 1;
            Some(entry)
        } else {
            self.stats.layout_misses += 1;
            self.stats.total_misses += 1;
            None
        }
    }
    
    /// Cache layout data
    pub fn cache_layout(&mut self, key: LayoutCacheKey, positions: HashMap<SceneId, [f32; 3]>, quality_score: f32) {
        let size_bytes = positions.len() * (8 + 12); // ID + position
        let entry = LayoutCacheEntry {
            positions,
            created_at: Instant::now(),
            last_accessed: Instant::now(),
            access_count: 1,
            quality_score,
        };
        
        self.layout_cache.insert(key, entry);
        self.stats.memory_used += size_bytes;
    }
    
    /// Clean up expired entries
    fn cleanup_expired_entries(&mut self) {
        let now = Instant::now();
        let lifetime = self.settings.entry_lifetime;
        
        // Clean geometry cache
        let mut removed_size = 0;
        self.geometry_cache.retain(|_, entry| {
            let expired = now.duration_since(entry.created_at) > lifetime;
            if expired {
                removed_size += entry.size_bytes;
                self.stats.entries_evicted += 1;
            }
            !expired
        });
        
        // Clean texture cache
        self.texture_cache.retain(|_, entry| {
            let expired = now.duration_since(entry.created_at) > lifetime;
            if expired {
                removed_size += entry.size_bytes;
                self.stats.entries_evicted += 1;
            }
            !expired
        });
        
        // Clean uniform cache
        self.uniform_cache.retain(|_, entry| {
            let expired = now.duration_since(entry.created_at) > lifetime;
            if expired {
                removed_size += entry.data.len();
                self.stats.entries_evicted += 1;
            }
            !expired
        });
        
        // Clean layout cache
        self.layout_cache.retain(|_, entry| {
            let expired = now.duration_since(entry.created_at) > lifetime;
            if expired {
                removed_size += entry.positions.len() * 20; // Estimated size
                self.stats.entries_evicted += 1;
            }
            !expired
        });
        
        self.stats.memory_used = self.stats.memory_used.saturating_sub(removed_size);
    }
    
    /// Evict least recently used entries
    fn evict_least_recently_used(&mut self) {
        let target_reduction = self.get_total_memory_usage() / 4; // Remove 25%
        let mut removed_size = 0;
        
        // Collect geometry entries with access times for sorting
        let mut geometry_entries: Vec<_> = self.geometry_cache.iter()
            .map(|(key, entry)| (key.clone(), entry.last_accessed, entry.size_bytes))
            .collect();
        
        // Sort by last accessed time (oldest first)
        geometry_entries.sort_by_key(|(_, last_accessed, _)| *last_accessed);
        
        // Remove oldest entries until we meet the target
        for (key, _, size) in geometry_entries {
            self.geometry_cache.remove(&key);
            removed_size += size;
            self.stats.entries_evicted += 1;
            
            if removed_size >= target_reduction {
                break;
            }
        }
        
        self.stats.memory_used = self.stats.memory_used.saturating_sub(removed_size);
    }
    
    /// Get total memory usage
    fn get_total_memory_usage(&self) -> usize {
        self.stats.memory_used
    }
    
    /// Get maximum allowed memory
    fn get_max_memory(&self) -> usize {
        self.settings.max_geometry_memory +
        self.settings.max_texture_memory +
        self.settings.max_uniform_memory +
        self.settings.max_layout_memory
    }
    
    /// Get cache hit rate
    pub fn get_hit_rate(&self) -> f32 {
        let total_requests = self.stats.total_hits + self.stats.total_misses;
        if total_requests > 0 {
            self.stats.total_hits as f32 / total_requests as f32
        } else {
            0.0
        }
    }
    
    /// Get cache statistics
    pub fn get_stats(&self) -> &CacheStats {
        &self.stats
    }
    
    /// Force cleanup of all caches
    pub fn force_cleanup(&mut self) {
        self.geometry_cache.clear();
        self.texture_cache.clear();
        self.uniform_cache.clear();
        self.layout_cache.clear();
        self.stats.memory_used = 0;
        self.stats.entries_evicted += 
            self.geometry_cache.len() as u64 +
            self.texture_cache.len() as u64 +
            self.uniform_cache.len() as u64 +
            self.layout_cache.len() as u64;
    }
    
    /// Configure cache settings
    pub fn configure(&mut self, settings: CacheSettings) {
        self.settings = settings;
    }
    
    /// Create geometry cache key
    pub fn create_geometry_key(&self, object_id: SceneId, lod_level: LodLevel, type_data: &[u8], size_data: &[u8]) -> GeometryCacheKey {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        type_data.hash(&mut hasher);
        let type_hash = hasher.finish();
        
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        size_data.hash(&mut hasher);
        let size_hash = hasher.finish();
        
        GeometryCacheKey {
            object_id,
            lod_level,
            type_hash,
            size_hash,
        }
    }
    
    /// Create texture cache key
    pub fn create_texture_key(&self, texture_id: String, resolution: u32, format_data: &[u8]) -> TextureCacheKey {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        format_data.hash(&mut hasher);
        let format_hash = hasher.finish();
        
        TextureCacheKey {
            texture_id,
            resolution,
            format_hash,
        }
    }
}

impl Default for CacheSettings {
    fn default() -> Self {
        Self {
            max_geometry_memory: 50 * 1024 * 1024, // 50MB
            max_texture_memory: 100 * 1024 * 1024, // 100MB
            max_uniform_memory: 10 * 1024 * 1024, // 10MB
            max_layout_memory: 20 * 1024 * 1024, // 20MB
            entry_lifetime: Duration::from_secs(300), // 5 minutes
            cleanup_interval: Duration::from_secs(60), // 1 minute
            enable_compression: false,
        }
    }
}

impl Default for RenderCache {
    fn default() -> Self {
        Self::new()
    }
}