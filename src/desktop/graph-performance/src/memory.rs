//! Memory management and pooling system

use crate::PerformanceMetrics;
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

/// Memory management system
pub struct MemoryManager {
    /// Object pools for different types
    pools: HashMap<PoolType, ObjectPool>,
    /// Memory usage tracker
    usage_tracker: MemoryUsageTracker,
    /// Garbage collection settings
    gc_settings: GcSettings,
    /// Last garbage collection time
    last_gc: Instant,
    /// Memory statistics
    stats: MemoryStats,
}

/// Types of object pools
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PoolType {
    /// Node render data
    NodeRenderData,
    /// Edge render data
    EdgeRenderData,
    /// Vertex buffers
    VertexBuffers,
    /// Index buffers
    IndexBuffers,
    /// Texture objects
    Textures,
    /// Shader uniforms
    Uniforms,
    /// Temporary vectors
    TempVectors,
    /// Matrix objects
    Matrices,
}

/// Object pool for memory reuse
pub struct ObjectPool {
    /// Pool type
    pool_type: PoolType,
    /// Available objects
    available: VecDeque<PooledObject>,
    /// Objects currently in use
    in_use: Vec<PooledObject>,
    /// Pool configuration
    config: PoolConfig,
    /// Pool statistics
    stats: PoolStats,
}

/// Pooled object wrapper
pub struct PooledObject {
    /// Object ID
    pub id: u64,
    /// Object size in bytes
    pub size: usize,
    /// Creation time
    pub created_at: Instant,
    /// Last used time
    pub last_used: Instant,
    /// Use count
    pub use_count: u32,
    /// Object data (simplified as bytes)
    pub data: Vec<u8>,
}

/// Pool configuration
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Maximum objects in pool
    pub max_objects: usize,
    /// Maximum memory per pool (bytes)
    pub max_memory: usize,
    /// Objects to pre-allocate
    pub pre_allocate: usize,
    /// Object lifetime before cleanup
    pub object_lifetime: Duration,
    /// Enable automatic cleanup
    pub auto_cleanup: bool,
}

/// Pool statistics
#[derive(Debug, Default)]
pub struct PoolStats {
    /// Total allocations
    pub total_allocations: u64,
    /// Total deallocations
    pub total_deallocations: u64,
    /// Cache hits (reused objects)
    pub cache_hits: u64,
    /// Cache misses (new allocations)
    pub cache_misses: u64,
    /// Current memory usage
    pub current_memory: usize,
    /// Peak memory usage
    pub peak_memory: usize,
}

/// Memory usage tracker
pub struct MemoryUsageTracker {
    /// Memory samples over time
    samples: VecDeque<MemorySample>,
    /// Sample interval
    sample_interval: Duration,
    /// Last sample time
    last_sample: Instant,
    /// Memory thresholds
    thresholds: MemoryThresholds,
}

/// Memory sample point
#[derive(Debug, Clone)]
pub struct MemorySample {
    /// Sample timestamp
    pub timestamp: Instant,
    /// Total memory used (bytes)
    pub total_used: usize,
    /// System memory used (bytes)
    pub system_used: usize,
    /// GPU memory used (bytes)
    pub gpu_used: usize,
    /// Pool memory used (bytes)
    pub pool_used: usize,
}

/// Memory thresholds for warnings
#[derive(Debug, Clone)]
pub struct MemoryThresholds {
    /// Warning threshold (bytes)
    pub warning: usize,
    /// Critical threshold (bytes)
    pub critical: usize,
    /// Cleanup threshold (bytes)
    pub cleanup: usize,
}

/// Garbage collection settings
#[derive(Debug, Clone)]
pub struct GcSettings {
    /// Enable automatic garbage collection
    pub enabled: bool,
    /// GC interval
    pub interval: Duration,
    /// Memory pressure threshold for forced GC
    pub pressure_threshold: f32,
    /// Age threshold for object cleanup
    pub age_threshold: Duration,
}

/// Memory statistics
#[derive(Debug, Default)]
pub struct MemoryStats {
    /// Total memory allocated
    pub total_allocated: usize,
    /// Total memory freed
    pub total_freed: usize,
    /// Current memory usage
    pub current_usage: usize,
    /// Peak memory usage
    pub peak_usage: usize,
    /// Number of GC runs
    pub gc_runs: u32,
    /// Objects cleaned up
    pub objects_cleaned: u32,
}

/// Memory information
#[derive(Debug, Clone)]
pub struct MemoryInfo {
    /// Total memory used (MB)
    pub used_mb: f32,
    /// Total memory available (MB)
    pub available_mb: f32,
    /// Memory usage percentage
    pub usage_percent: f32,
    /// Pool memory breakdown
    pub pool_breakdown: HashMap<PoolType, f32>,
    /// Recent memory trend
    pub trend: MemoryTrend,
}

/// Memory usage trend
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MemoryTrend {
    /// Memory usage increasing
    Increasing,
    /// Memory usage stable
    Stable,
    /// Memory usage decreasing
    Decreasing,
    /// Not enough data
    Unknown,
}

impl MemoryManager {
    /// Create a new memory manager
    pub fn new() -> Self {
        Self {
            pools: Self::create_default_pools(),
            usage_tracker: MemoryUsageTracker::new(),
            gc_settings: GcSettings::default(),
            last_gc: Instant::now(),
            stats: MemoryStats::default(),
        }
    }
    
    /// Update memory manager
    pub fn update(&mut self, metrics: &PerformanceMetrics) {
        // Update usage tracking
        self.usage_tracker.update();
        
        // Check if garbage collection is needed
        if self.should_run_gc() {
            self.run_garbage_collection();
        }
        
        // Update statistics
        self.update_stats();
    }
    
    /// Allocate object from pool
    pub fn allocate<T>(&mut self, pool_type: PoolType, size: usize) -> Option<u64> {
        if let Some(pool) = self.pools.get_mut(&pool_type) {
            pool.allocate(size)
        } else {
            None
        }
    }
    
    /// Deallocate object back to pool
    pub fn deallocate(&mut self, pool_type: PoolType, object_id: u64) {
        if let Some(pool) = self.pools.get_mut(&pool_type) {
            pool.deallocate(object_id);
        }
    }
    
    /// Get object from pool
    pub fn get_object(&self, pool_type: PoolType, object_id: u64) -> Option<&PooledObject> {
        self.pools.get(&pool_type)?.get_object(object_id)
    }
    
    /// Get mutable object from pool
    pub fn get_object_mut(&mut self, pool_type: PoolType, object_id: u64) -> Option<&mut PooledObject> {
        self.pools.get_mut(&pool_type)?.get_object_mut(object_id)
    }
    
    /// Check if garbage collection should run
    fn should_run_gc(&self) -> bool {
        if !self.gc_settings.enabled {
            return false;
        }
        
        let time_since_last_gc = Instant::now().duration_since(self.last_gc);
        if time_since_last_gc >= self.gc_settings.interval {
            return true;
        }
        
        // Check memory pressure
        let memory_pressure = self.calculate_memory_pressure();
        memory_pressure >= self.gc_settings.pressure_threshold
    }
    
    /// Calculate memory pressure (0.0 to 1.0)
    fn calculate_memory_pressure(&self) -> f32 {
        let total_used = self.stats.current_usage;
        let threshold = self.usage_tracker.thresholds.warning;
        
        if threshold > 0 {
            (total_used as f32) / (threshold as f32)
        } else {
            0.0
        }
    }
    
    /// Run garbage collection
    fn run_garbage_collection(&mut self) {
        let start_time = Instant::now();
        let mut cleaned_objects = 0;
        
        // Clean up old objects in all pools
        for pool in self.pools.values_mut() {
            cleaned_objects += pool.cleanup_old_objects(&self.gc_settings);
        }
        
        // Update statistics
        self.stats.gc_runs += 1;
        self.stats.objects_cleaned += cleaned_objects;
        self.last_gc = Instant::now();
        
        let gc_duration = Instant::now().duration_since(start_time);
        log::debug!("GC completed in {:.2}ms, cleaned {} objects", 
                   gc_duration.as_secs_f64() * 1000.0, cleaned_objects);
    }
    
    /// Force garbage collection
    pub fn force_cleanup(&mut self) {
        self.run_garbage_collection();
    }
    
    /// Update memory statistics
    fn update_stats(&mut self) {
        self.stats.current_usage = self.pools.values()
            .map(|pool| pool.stats.current_memory)
            .sum();
        
        if self.stats.current_usage > self.stats.peak_usage {
            self.stats.peak_usage = self.stats.current_usage;
        }
    }
    
    /// Get memory information
    pub fn get_info(&self) -> MemoryInfo {
        let used_bytes = self.stats.current_usage;
        let used_mb = used_bytes as f32 / (1024.0 * 1024.0);
        
        // Calculate pool breakdown
        let mut pool_breakdown = HashMap::new();
        for (pool_type, pool) in &self.pools {
            let pool_mb = pool.stats.current_memory as f32 / (1024.0 * 1024.0);
            pool_breakdown.insert(*pool_type, pool_mb);
        }
        
        MemoryInfo {
            used_mb,
            available_mb: 1024.0, // Placeholder - would be system memory
            usage_percent: (used_mb / 1024.0) * 100.0,
            pool_breakdown,
            trend: self.usage_tracker.calculate_trend(),
        }
    }
    
    /// Configure garbage collection
    pub fn configure_gc(&mut self, settings: GcSettings) {
        self.gc_settings = settings;
    }
    
    /// Get memory statistics
    pub fn get_stats(&self) -> &MemoryStats {
        &self.stats
    }
    
    /// Create default object pools
    fn create_default_pools() -> HashMap<PoolType, ObjectPool> {
        let mut pools = HashMap::new();
        
        // Create pools for different object types
        pools.insert(PoolType::NodeRenderData, ObjectPool::new(PoolType::NodeRenderData, PoolConfig {
            max_objects: 1000,
            max_memory: 10 * 1024 * 1024, // 10MB
            pre_allocate: 100,
            object_lifetime: Duration::from_secs(300), // 5 minutes
            auto_cleanup: true,
        }));
        
        pools.insert(PoolType::EdgeRenderData, ObjectPool::new(PoolType::EdgeRenderData, PoolConfig {
            max_objects: 2000,
            max_memory: 20 * 1024 * 1024, // 20MB
            pre_allocate: 200,
            object_lifetime: Duration::from_secs(300),
            auto_cleanup: true,
        }));
        
        pools.insert(PoolType::VertexBuffers, ObjectPool::new(PoolType::VertexBuffers, PoolConfig {
            max_objects: 500,
            max_memory: 50 * 1024 * 1024, // 50MB
            pre_allocate: 50,
            object_lifetime: Duration::from_secs(600), // 10 minutes
            auto_cleanup: true,
        }));
        
        pools.insert(PoolType::Textures, ObjectPool::new(PoolType::Textures, PoolConfig {
            max_objects: 200,
            max_memory: 100 * 1024 * 1024, // 100MB
            pre_allocate: 20,
            object_lifetime: Duration::from_secs(1800), // 30 minutes
            auto_cleanup: true,
        }));
        
        // Add other pool types...
        
        pools
    }
}

impl ObjectPool {
    /// Create a new object pool
    pub fn new(pool_type: PoolType, config: PoolConfig) -> Self {
        let mut pool = Self {
            pool_type,
            available: VecDeque::new(),
            in_use: Vec::new(),
            config,
            stats: PoolStats::default(),
        };
        
        // Pre-allocate objects
        pool.pre_allocate_objects();
        
        pool
    }
    
    /// Pre-allocate objects
    fn pre_allocate_objects(&mut self) {
        for _ in 0..self.config.pre_allocate {
            let object = PooledObject::new(1024); // Default size
            self.available.push_back(object);
        }
    }
    
    /// Allocate object from pool
    pub fn allocate(&mut self, size: usize) -> Option<u64> {
        // Try to reuse existing object
        if let Some(mut object) = self.available.pop_front() {
            object.last_used = Instant::now();
            object.use_count += 1;
            
            // Resize if needed
            if object.data.len() < size {
                object.data.resize(size, 0);
                object.size = size;
            }
            
            let id = object.id;
            self.in_use.push(object);
            self.stats.cache_hits += 1;
            Some(id)
        } else if self.in_use.len() < self.config.max_objects {
            // Create new object
            let object = PooledObject::new(size);
            let id = object.id;
            self.in_use.push(object);
            self.stats.cache_misses += 1;
            self.stats.total_allocations += 1;
            Some(id)
        } else {
            // Pool is full
            None
        }
    }
    
    /// Deallocate object back to pool
    pub fn deallocate(&mut self, object_id: u64) {
        if let Some(pos) = self.in_use.iter().position(|obj| obj.id == object_id) {
            let object = self.in_use.remove(pos);
            self.available.push_back(object);
            self.stats.total_deallocations += 1;
        }
    }
    
    /// Get object by ID
    pub fn get_object(&self, object_id: u64) -> Option<&PooledObject> {
        self.in_use.iter().find(|obj| obj.id == object_id)
    }
    
    /// Get mutable object by ID
    pub fn get_object_mut(&mut self, object_id: u64) -> Option<&mut PooledObject> {
        self.in_use.iter_mut().find(|obj| obj.id == object_id)
    }
    
    /// Clean up old objects
    pub fn cleanup_old_objects(&mut self, gc_settings: &GcSettings) -> u32 {
        let now = Instant::now();
        let mut cleaned = 0;
        
        // Clean up available objects
        self.available.retain(|obj| {
            let age = now.duration_since(obj.created_at);
            if age <= gc_settings.age_threshold {
                true
            } else {
                cleaned += 1;
                self.stats.current_memory = self.stats.current_memory.saturating_sub(obj.size);
                false
            }
        });
        
        cleaned
    }
}

impl PooledObject {
    /// Create a new pooled object
    pub fn new(size: usize) -> Self {
        static mut NEXT_ID: u64 = 1;
        
        let id = unsafe {
            let id = NEXT_ID;
            NEXT_ID += 1;
            id
        };
        
        Self {
            id,
            size,
            created_at: Instant::now(),
            last_used: Instant::now(),
            use_count: 0,
            data: vec![0; size],
        }
    }
}

impl MemoryUsageTracker {
    /// Create a new memory usage tracker
    pub fn new() -> Self {
        Self {
            samples: VecDeque::new(),
            sample_interval: Duration::from_secs(1),
            last_sample: Instant::now(),
            thresholds: MemoryThresholds {
                warning: 1024 * 1024 * 1024, // 1GB
                critical: 2048 * 1024 * 1024, // 2GB
                cleanup: 1536 * 1024 * 1024, // 1.5GB
            },
        }
    }
    
    /// Update memory tracking
    pub fn update(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last_sample) >= self.sample_interval {
            let sample = MemorySample {
                timestamp: now,
                total_used: 0, // Would be filled from system
                system_used: 0,
                gpu_used: 0,
                pool_used: 0,
            };
            
            self.samples.push_back(sample);
            
            // Keep only recent samples (last hour)
            let cutoff = now - Duration::from_secs(3600);
            self.samples.retain(|sample| sample.timestamp >= cutoff);
            
            self.last_sample = now;
        }
    }
    
    /// Calculate memory trend
    pub fn calculate_trend(&self) -> MemoryTrend {
        if self.samples.len() < 10 {
            return MemoryTrend::Unknown;
        }
        
        let recent_avg = self.samples.iter().rev().take(5)
            .map(|s| s.total_used as f64)
            .sum::<f64>() / 5.0;
        
        let older_avg = self.samples.iter().rev().skip(5).take(5)
            .map(|s| s.total_used as f64)
            .sum::<f64>() / 5.0;
        
        let change_rate = (recent_avg - older_avg) / older_avg;
        
        if change_rate > 0.05 {
            MemoryTrend::Increasing
        } else if change_rate < -0.05 {
            MemoryTrend::Decreasing
        } else {
            MemoryTrend::Stable
        }
    }
}

impl Default for GcSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            interval: Duration::from_secs(30),
            pressure_threshold: 0.8,
            age_threshold: Duration::from_secs(300),
        }
    }
}

impl Default for MemoryManager {
    fn default() -> Self {
        Self::new()
    }
}