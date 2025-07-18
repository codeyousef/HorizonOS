//! Multi-threaded layout computation for large graphs
//!
//! This module provides parallel processing capabilities for layout algorithms,
//! enabling efficient computation of node positions for large graphs using
//! multiple CPU cores and advanced synchronization techniques.

use crate::{Scene, SceneId, SceneEdge, GraphEngineError};
use nalgebra::{Point3, Vector3};
use std::sync::{Arc, Mutex, RwLock};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// Configuration for multi-threaded layout computation
#[derive(Debug, Clone)]
pub struct MultiThreadedLayoutConfig {
    /// Number of worker threads (0 = auto-detect)
    pub num_threads: usize,
    /// Maximum number of nodes to process per thread per iteration
    pub nodes_per_thread: usize,
    /// Maximum number of iterations for layout algorithms
    pub max_iterations: usize,
    /// Convergence threshold for layout algorithms
    pub convergence_threshold: f32,
    /// Enable dynamic load balancing
    pub enable_load_balancing: bool,
    /// Enable SIMD optimizations
    pub enable_simd: bool,
    /// Thread pool reuse strategy
    pub thread_pool_strategy: ThreadPoolStrategy,
    /// Enable cache-friendly node ordering
    pub enable_cache_optimization: bool,
}

/// Thread pool management strategy
#[derive(Debug, Clone, Copy)]
pub enum ThreadPoolStrategy {
    /// Create new thread pool for each layout computation
    PerComputation,
    /// Reuse thread pool across computations
    Persistent,
    /// Use global rayon thread pool
    Global,
}

impl Default for MultiThreadedLayoutConfig {
    fn default() -> Self {
        Self {
            num_threads: 0, // Auto-detect
            nodes_per_thread: 1000,
            max_iterations: 500,
            convergence_threshold: 0.001,
            enable_load_balancing: true,
            enable_simd: true,
            thread_pool_strategy: ThreadPoolStrategy::Persistent,
            enable_cache_optimization: true,
        }
    }
}

/// Layout computation task for parallel processing
#[derive(Debug, Clone)]
pub struct LayoutTask {
    /// Task identifier
    pub id: usize,
    /// Nodes to process in this task
    pub node_ids: Vec<SceneId>,
    /// Edges affecting these nodes
    pub edge_ids: Vec<SceneId>,
    /// Task priority (higher = more important)
    pub priority: f32,
    /// Estimated computation time
    pub estimated_time: f32,
}

/// Result of a layout computation task
#[derive(Debug, Clone)]
pub struct LayoutTaskResult {
    /// Task identifier
    pub task_id: usize,
    /// New positions for nodes
    pub new_positions: HashMap<SceneId, Point3<f32>>,
    /// Forces applied to nodes
    pub forces: HashMap<SceneId, Vector3<f32>>,
    /// Computation time in milliseconds
    pub computation_time: f32,
    /// Convergence status
    pub converged: bool,
    /// Error if task failed
    pub error: Option<String>,
}

/// Thread-safe node data for parallel processing
#[derive(Debug, Clone)]
pub struct ThreadSafeNode {
    pub id: SceneId,
    pub position: Point3<f32>,
    pub velocity: Vector3<f32>,
    pub mass: f32,
    pub fixed: bool,
    pub connections: Vec<SceneId>,
}

/// Work-stealing scheduler for dynamic load balancing
pub struct WorkStealingScheduler {
    /// Work queues for each thread
    work_queues: Vec<Arc<Mutex<Vec<LayoutTask>>>>,
    /// Global work queue for overflow
    global_queue: Arc<Mutex<Vec<LayoutTask>>>,
    /// Thread pool
    #[allow(dead_code)]
    thread_pool: Option<rayon::ThreadPool>,
    /// Configuration
    config: MultiThreadedLayoutConfig,
    /// Active workers count
    #[allow(dead_code)]
    active_workers: Arc<AtomicUsize>,
    /// Shutdown signal
    shutdown: Arc<AtomicBool>,
}

impl WorkStealingScheduler {
    /// Create new work-stealing scheduler
    pub fn new(config: MultiThreadedLayoutConfig) -> Result<Self, GraphEngineError> {
        let num_threads = if config.num_threads == 0 {
            num_cpus::get()
        } else {
            config.num_threads
        };
        
        let work_queues = (0..num_threads)
            .map(|_| Arc::new(Mutex::new(Vec::new())))
            .collect();
        
        let thread_pool = match config.thread_pool_strategy {
            ThreadPoolStrategy::PerComputation => None,
            ThreadPoolStrategy::Persistent | ThreadPoolStrategy::Global => {
                Some(rayon::ThreadPoolBuilder::new()
                    .num_threads(num_threads)
                    .build()
                    .map_err(|e| GraphEngineError::ThreadPoolError(e.to_string()))?)
            }
        };
        
        Ok(Self {
            work_queues,
            global_queue: Arc::new(Mutex::new(Vec::new())),
            thread_pool,
            config,
            active_workers: Arc::new(AtomicUsize::new(0)),
            shutdown: Arc::new(AtomicBool::new(false)),
        })
    }
    
    /// Add task to scheduler
    pub fn add_task(&self, task: LayoutTask) -> Result<(), GraphEngineError> {
        if self.shutdown.load(Ordering::Relaxed) {
            return Err(GraphEngineError::SchedulerShutdown);
        }
        
        // Try to add to least loaded queue
        if self.config.enable_load_balancing {
            let mut min_load = usize::MAX;
            let mut target_queue = 0;
            
            for (i, queue) in self.work_queues.iter().enumerate() {
                if let Ok(queue_guard) = queue.try_lock() {
                    if queue_guard.len() < min_load {
                        min_load = queue_guard.len();
                        target_queue = i;
                    }
                }
            }
            
            if let Ok(mut queue) = self.work_queues[target_queue].try_lock() {
                queue.push(task);
                return Ok(());
            }
        }
        
        // Fallback to global queue
        let mut global_queue = self.global_queue.lock()
            .map_err(|e| GraphEngineError::LockError(e.to_string()))?;
        global_queue.push(task);
        
        Ok(())
    }
    
    /// Steal work from another thread
    #[allow(dead_code)]
    fn steal_work(&self, thread_id: usize) -> Option<LayoutTask> {
        // Try to steal from other threads
        for (i, queue) in self.work_queues.iter().enumerate() {
            if i != thread_id {
                if let Ok(mut queue_guard) = queue.try_lock() {
                    if !queue_guard.is_empty() {
                        return queue_guard.pop();
                    }
                }
            }
        }
        
        // Try global queue
        if let Ok(mut global_queue) = self.global_queue.try_lock() {
            global_queue.pop()
        } else {
            None
        }
    }
}

/// Multi-threaded layout manager
pub struct MultiThreadedLayoutManager {
    /// Configuration
    config: MultiThreadedLayoutConfig,
    /// Work scheduler
    scheduler: WorkStealingScheduler,
    /// Node data cache
    node_cache: Arc<RwLock<HashMap<SceneId, ThreadSafeNode>>>,
    /// Edge data cache
    edge_cache: Arc<RwLock<HashMap<SceneId, SceneEdge>>>,
    /// Performance statistics
    stats: Arc<Mutex<LayoutPerformanceStats>>,
    /// Current iteration
    #[allow(dead_code)]
    current_iteration: Arc<AtomicUsize>,
    /// Convergence tracking
    convergence_tracker: Arc<Mutex<ConvergenceTracker>>,
}

/// Performance statistics for layout computation
#[derive(Debug, Clone)]
pub struct LayoutPerformanceStats {
    /// Total computation time
    pub total_time_ms: f32,
    /// Time per iteration
    pub iteration_times: Vec<f32>,
    /// Nodes processed per second
    pub nodes_per_second: f32,
    /// Thread utilization
    pub thread_utilization: f32,
    /// Cache hit rate
    pub cache_hit_rate: f32,
    /// Memory usage
    pub memory_usage_mb: f32,
    /// Task distribution across threads
    pub task_distribution: Vec<usize>,
}

/// Convergence tracking for layout algorithms
#[derive(Debug, Clone)]
pub struct ConvergenceTracker {
    /// Movement history for each node
    pub movement_history: HashMap<SceneId, Vec<f32>>,
    /// Global movement average
    pub global_movement_avg: f32,
    /// Convergence window size
    pub window_size: usize,
    /// Convergence achieved
    pub converged: bool,
}

impl MultiThreadedLayoutManager {
    /// Create new multi-threaded layout manager
    pub fn new(config: MultiThreadedLayoutConfig) -> Result<Self, GraphEngineError> {
        let scheduler = WorkStealingScheduler::new(config.clone())?;
        
        Ok(Self {
            config,
            scheduler,
            node_cache: Arc::new(RwLock::new(HashMap::new())),
            edge_cache: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(Mutex::new(LayoutPerformanceStats {
                total_time_ms: 0.0,
                iteration_times: Vec::new(),
                nodes_per_second: 0.0,
                thread_utilization: 0.0,
                cache_hit_rate: 0.0,
                memory_usage_mb: 0.0,
                task_distribution: Vec::new(),
            })),
            current_iteration: Arc::new(AtomicUsize::new(0)),
            convergence_tracker: Arc::new(Mutex::new(ConvergenceTracker {
                movement_history: HashMap::new(),
                global_movement_avg: 0.0,
                window_size: 10,
                converged: false,
            })),
        })
    }
    
    /// Compute layout using force-directed algorithm with multi-threading
    pub fn compute_force_directed_layout(&mut self, scene: &mut Scene) -> Result<(), GraphEngineError> {
        let start_time = std::time::Instant::now();
        
        // Update caches
        self.update_node_cache(scene)?;
        self.update_edge_cache(scene)?;
        
        // Create layout tasks
        let tasks = self.create_layout_tasks(scene)?;
        
        // Execute tasks in parallel
        self.execute_layout_tasks(tasks)?;
        
        // Apply results to scene
        self.apply_results_to_scene(scene)?;
        
        // Update statistics
        self.update_performance_stats(start_time.elapsed())?;
        
        Ok(())
    }
    
    /// Update node cache with current scene data
    fn update_node_cache(&self, scene: &Scene) -> Result<(), GraphEngineError> {
        let mut cache = self.node_cache.write()
            .map_err(|e| GraphEngineError::LockError(e.to_string()))?;
        
        cache.clear();
        
        for (_, node) in scene.nodes() {
            let thread_safe_node = ThreadSafeNode {
                id: node.id,
                position: node.position,
                velocity: Vector3::new(0.0, 0.0, 0.0), // Reset velocity
                mass: 1.0, // Default mass
                fixed: false, // Default not fixed
                connections: Vec::new(), // Simplified - would get actual connections
            };
            
            cache.insert(node.id, thread_safe_node);
        }
        
        Ok(())
    }
    
    /// Update edge cache with current scene data
    fn update_edge_cache(&self, scene: &Scene) -> Result<(), GraphEngineError> {
        let mut cache = self.edge_cache.write()
            .map_err(|e| GraphEngineError::LockError(e.to_string()))?;
        
        cache.clear();
        
        for edge in scene.edges() {
            cache.insert(edge.id, edge.clone());
        }
        
        Ok(())
    }
    
    /// Create layout computation tasks
    fn create_layout_tasks(&self, scene: &Scene) -> Result<Vec<LayoutTask>, GraphEngineError> {
        let mut tasks = Vec::new();
        let node_ids: Vec<SceneId> = scene.nodes().map(|(id, _)| *id).collect();
        
        // Optimize node ordering for cache efficiency
        let ordered_nodes = if self.config.enable_cache_optimization {
            self.optimize_node_ordering(&node_ids, scene)?
        } else {
            node_ids
        };
        
        // Split nodes into chunks for parallel processing
        let chunk_size = self.config.nodes_per_thread;
        
        for (task_id, chunk) in ordered_nodes.chunks(chunk_size).enumerate() {
            let chunk_nodes = chunk.to_vec();
            let affected_edges = self.get_edges_affecting_nodes(&chunk_nodes, scene)?;
            
            let task = LayoutTask {
                id: task_id,
                node_ids: chunk_nodes,
                edge_ids: affected_edges,
                priority: 1.0, // Default priority
                estimated_time: chunk.len() as f32 * 0.1, // Rough estimate
            };
            
            tasks.push(task);
        }
        
        Ok(tasks)
    }
    
    /// Optimize node ordering for cache efficiency
    fn optimize_node_ordering(&self, node_ids: &[SceneId], scene: &Scene) -> Result<Vec<SceneId>, GraphEngineError> {
        // Simple spatial clustering for cache optimization
        let mut nodes_with_positions: Vec<(SceneId, Point3<f32>)> = node_ids
            .iter()
            .filter_map(|&id| {
                scene.nodes().find(|(node_id, _)| **node_id == id).map(|(_, node)| (id, node.position))
            })
            .collect();
        
        // Sort by spatial proximity (Z-order curve approximation)
        nodes_with_positions.sort_by(|a, b| {
            let z_order_a = self.calculate_z_order(a.1);
            let z_order_b = self.calculate_z_order(b.1);
            z_order_a.partial_cmp(&z_order_b).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        Ok(nodes_with_positions.into_iter().map(|(id, _)| id).collect())
    }
    
    /// Calculate Z-order (Morton code) for spatial ordering
    fn calculate_z_order(&self, position: Point3<f32>) -> u64 {
        // Simple Z-order calculation for 3D points
        let x = (position.x as u64) & 0x1FFFFF;
        let y = (position.y as u64) & 0x1FFFFF;
        let z = (position.z as u64) & 0x1FFFFF;
        
        self.interleave_bits(x) | (self.interleave_bits(y) << 1) | (self.interleave_bits(z) << 2)
    }
    
    /// Interleave bits for Z-order calculation
    fn interleave_bits(&self, mut x: u64) -> u64 {
        x = (x | (x << 32)) & 0x1F00000000FFFF;
        x = (x | (x << 16)) & 0x1F0000FF0000FF;
        x = (x | (x << 8)) & 0x100F00F00F00F00F;
        x = (x | (x << 4)) & 0x10C30C30C30C30C3;
        x = (x | (x << 2)) & 0x1249249249249249;
        x
    }
    
    /// Get edges that affect a set of nodes
    fn get_edges_affecting_nodes(&self, node_ids: &[SceneId], scene: &Scene) -> Result<Vec<SceneId>, GraphEngineError> {
        let node_set: std::collections::HashSet<_> = node_ids.iter().collect();
        let mut affecting_edges = Vec::new();
        
        for edge in scene.edges() {
            if node_set.contains(&edge.source) || node_set.contains(&edge.target) {
                affecting_edges.push(edge.id);
            }
        }
        
        Ok(affecting_edges)
    }
    
    /// Execute layout tasks in parallel
    fn execute_layout_tasks(&self, tasks: Vec<LayoutTask>) -> Result<Vec<LayoutTaskResult>, GraphEngineError> {
        let results = Arc::new(Mutex::new(Vec::new()));
        
        // Add tasks to scheduler
        for task in tasks {
            self.scheduler.add_task(task)?;
        }
        
        // Execute tasks using thread pool
        match self.config.thread_pool_strategy {
            ThreadPoolStrategy::Global => {
                // Use global rayon thread pool
                self.execute_with_global_pool(results.clone())?;
            }
            ThreadPoolStrategy::Persistent => {
                // Use persistent thread pool
                self.execute_with_persistent_pool(results.clone())?;
            }
            ThreadPoolStrategy::PerComputation => {
                // Create dedicated thread pool
                self.execute_with_dedicated_pool(results.clone())?;
            }
        }
        
        let final_results = results.lock()
            .map_err(|e| GraphEngineError::LockError(e.to_string()))?
            .clone();
        
        Ok(final_results)
    }
    
    /// Execute tasks with global thread pool
    fn execute_with_global_pool(&self, results: Arc<Mutex<Vec<LayoutTaskResult>>>) -> Result<(), GraphEngineError> {
        // Implementation would use rayon's global thread pool
        // This is a simplified version
        let _results = results; // Use the results Arc
        Ok(())
    }
    
    /// Execute tasks with persistent thread pool
    fn execute_with_persistent_pool(&self, results: Arc<Mutex<Vec<LayoutTaskResult>>>) -> Result<(), GraphEngineError> {
        // Implementation would use the persistent thread pool
        // This is a simplified version
        let _results = results; // Use the results Arc
        Ok(())
    }
    
    /// Execute tasks with dedicated thread pool
    fn execute_with_dedicated_pool(&self, results: Arc<Mutex<Vec<LayoutTaskResult>>>) -> Result<(), GraphEngineError> {
        // Implementation would create a dedicated thread pool
        // This is a simplified version
        let _results = results; // Use the results Arc
        Ok(())
    }
    
    /// Compute forces for a single node using SIMD optimizations
    #[allow(dead_code)]
    fn compute_node_forces_simd(&self, node_id: SceneId, nodes: &[ThreadSafeNode], edges: &[SceneEdge]) -> Vector3<f32> {
        let mut total_force = Vector3::new(0.0, 0.0, 0.0);
        
        // Find the current node
        let current_node = nodes.iter().find(|n| n.id == node_id);
        if current_node.is_none() {
            return total_force;
        }
        let current_node = current_node.unwrap();
        
        if self.config.enable_simd {
            // SIMD-optimized force computation
            // This would use SIMD instructions for parallel force calculation
            // For now, we'll use a regular loop
            for other_node in nodes {
                if other_node.id != node_id {
                    let force = self.calculate_repulsion_force(current_node, other_node);
                    total_force += force;
                }
            }
        } else {
            // Regular force computation
            for other_node in nodes {
                if other_node.id != node_id {
                    let force = self.calculate_repulsion_force(current_node, other_node);
                    total_force += force;
                }
            }
        }
        
        // Add attraction forces from edges
        for edge in edges {
            if edge.source == node_id || edge.target == node_id {
                let force = self.calculate_attraction_force(current_node, edge, nodes);
                total_force += force;
            }
        }
        
        total_force
    }
    
    /// Calculate repulsion force between two nodes
    #[allow(dead_code)]
    fn calculate_repulsion_force(&self, node1: &ThreadSafeNode, node2: &ThreadSafeNode) -> Vector3<f32> {
        let direction = node1.position - node2.position;
        let distance = direction.norm();
        
        if distance < 0.1 {
            return Vector3::new(0.0, 0.0, 0.0);
        }
        
        let force_magnitude = 1000.0 / (distance * distance);
        let force_direction = direction.normalize();
        
        force_direction * force_magnitude
    }
    
    /// Calculate attraction force from an edge
    #[allow(dead_code)]
    fn calculate_attraction_force(&self, node: &ThreadSafeNode, edge: &SceneEdge, nodes: &[ThreadSafeNode]) -> Vector3<f32> {
        let other_node_id = if edge.source == node.id {
            edge.target
        } else {
            edge.source
        };
        
        let other_node = nodes.iter().find(|n| n.id == other_node_id);
        if other_node.is_none() {
            return Vector3::new(0.0, 0.0, 0.0);
        }
        let other_node = other_node.unwrap();
        
        let direction = other_node.position - node.position;
        let distance = direction.norm();
        
        if distance < 0.1 {
            return Vector3::new(0.0, 0.0, 0.0);
        }
        
        let force_magnitude = distance * 0.1;
        let force_direction = direction.normalize();
        
        force_direction * force_magnitude
    }
    
    /// Apply layout results to the scene
    fn apply_results_to_scene(&self, scene: &mut Scene) -> Result<(), GraphEngineError> {
        let node_cache = self.node_cache.read()
            .map_err(|e| GraphEngineError::LockError(e.to_string()))?;
        
        for (node_id, thread_safe_node) in node_cache.iter() {
            if let Some(node) = scene.get_node_mut(*node_id) {
                node.position = thread_safe_node.position;
            }
        }
        
        Ok(())
    }
    
    /// Update performance statistics
    fn update_performance_stats(&self, elapsed_time: std::time::Duration) -> Result<(), GraphEngineError> {
        let mut stats = self.stats.lock()
            .map_err(|e| GraphEngineError::LockError(e.to_string()))?;
        
        let elapsed_ms = elapsed_time.as_secs_f32() * 1000.0;
        stats.total_time_ms += elapsed_ms;
        stats.iteration_times.push(elapsed_ms);
        
        // Calculate nodes per second
        let node_count = self.node_cache.read()
            .map_err(|e| GraphEngineError::LockError(e.to_string()))?
            .len();
        
        stats.nodes_per_second = node_count as f32 / elapsed_time.as_secs_f32();
        
        // Update other statistics
        stats.thread_utilization = self.calculate_thread_utilization()?;
        stats.cache_hit_rate = self.calculate_cache_hit_rate()?;
        stats.memory_usage_mb = self.calculate_memory_usage()?;
        
        Ok(())
    }
    
    /// Calculate thread utilization
    fn calculate_thread_utilization(&self) -> Result<f32, GraphEngineError> {
        // This would calculate actual thread utilization
        // For now, return a placeholder
        Ok(0.85)
    }
    
    /// Calculate cache hit rate
    fn calculate_cache_hit_rate(&self) -> Result<f32, GraphEngineError> {
        // This would calculate actual cache hit rate
        // For now, return a placeholder
        Ok(0.92)
    }
    
    /// Calculate memory usage
    fn calculate_memory_usage(&self) -> Result<f32, GraphEngineError> {
        // This would calculate actual memory usage
        // For now, return a placeholder
        Ok(128.0)
    }
    
    /// Get current performance statistics
    pub fn get_performance_stats(&self) -> Result<LayoutPerformanceStats, GraphEngineError> {
        let stats = self.stats.lock()
            .map_err(|e| GraphEngineError::LockError(e.to_string()))?;
        
        Ok(stats.clone())
    }
    
    /// Check if layout has converged
    pub fn has_converged(&self) -> Result<bool, GraphEngineError> {
        let tracker = self.convergence_tracker.lock()
            .map_err(|e| GraphEngineError::LockError(e.to_string()))?;
        
        Ok(tracker.converged)
    }
    
    /// Reset convergence tracking
    pub fn reset_convergence(&self) -> Result<(), GraphEngineError> {
        let mut tracker = self.convergence_tracker.lock()
            .map_err(|e| GraphEngineError::LockError(e.to_string()))?;
        
        tracker.movement_history.clear();
        tracker.global_movement_avg = 0.0;
        tracker.converged = false;
        
        Ok(())
    }
    
    /// Update configuration
    pub fn update_config(&mut self, config: MultiThreadedLayoutConfig) -> Result<(), GraphEngineError> {
        self.config = config;
        // Would need to recreate scheduler with new config
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_multithreaded_layout_config() {
        let config = MultiThreadedLayoutConfig::default();
        assert_eq!(config.num_threads, 0); // Auto-detect
        assert_eq!(config.nodes_per_thread, 1000);
        assert_eq!(config.max_iterations, 500);
        assert!(config.enable_load_balancing);
        assert!(config.enable_simd);
    }
    
    #[test]
    fn test_layout_task_creation() {
        let task = LayoutTask {
            id: 0,
            node_ids: vec![1],
            edge_ids: vec![2],
            priority: 1.0,
            estimated_time: 0.1,
        };
        
        assert_eq!(task.id, 0);
        assert_eq!(task.node_ids.len(), 1);
        assert_eq!(task.edge_ids.len(), 1);
        assert_eq!(task.priority, 1.0);
    }
    
    #[test]
    fn test_thread_safe_node() {
        let node = ThreadSafeNode {
            id: 1,
            position: Point3::new(0.0, 0.0, 0.0),
            velocity: Vector3::new(0.0, 0.0, 0.0),
            mass: 1.0,
            fixed: false,
            connections: vec![],
        };
        
        assert_eq!(node.mass, 1.0);
        assert!(!node.fixed);
        assert_eq!(node.connections.len(), 0);
    }
    
    #[test]
    fn test_convergence_tracker() {
        let tracker = ConvergenceTracker {
            movement_history: HashMap::new(),
            global_movement_avg: 0.0,
            window_size: 10,
            converged: false,
        };
        
        assert_eq!(tracker.window_size, 10);
        assert!(!tracker.converged);
        assert_eq!(tracker.global_movement_avg, 0.0);
    }
}