//! Layout algorithms for graph positioning and organization
//!
//! This module provides various layout algorithms for positioning nodes and edges
//! in the graph visualization, including force-directed algorithms, hierarchical
//! layouts, and multi-threaded computation capabilities.

pub mod multi_threaded;
pub mod force_directed;
pub mod hierarchical;
pub mod circular;

pub use multi_threaded::*;
pub use force_directed::*;
pub use hierarchical::*;
pub use circular::*;

use crate::{Scene, SceneId, GraphEngineError};
use nalgebra::Point3;
use std::collections::HashMap;

/// Common interface for all layout algorithms
pub trait LayoutAlgorithm {
    /// Configuration type for this layout algorithm
    type Config;
    
    /// Apply the layout algorithm to a scene
    fn apply_layout(&mut self, scene: &mut Scene, config: &Self::Config) -> Result<(), GraphEngineError>;
    
    /// Get the name of this layout algorithm
    fn name(&self) -> &'static str;
    
    /// Check if the layout supports incremental updates
    fn supports_incremental(&self) -> bool;
    
    /// Apply incremental update to the layout
    fn apply_incremental(&mut self, scene: &mut Scene, changes: &[SceneChange]) -> Result<(), GraphEngineError> {
        // Default implementation - incremental updates not supported by default
        let _ = (scene, changes);
        Err(GraphEngineError::System("Incremental updates not supported".to_string()))
    }
}

/// Types of changes that can occur in a scene
#[derive(Debug, Clone)]
pub enum SceneChange {
    /// Node was added
    NodeAdded { id: SceneId },
    /// Node was removed
    NodeRemoved { id: SceneId },
    /// Node was moved
    NodeMoved { id: SceneId, old_position: Point3<f32>, new_position: Point3<f32> },
    /// Edge was added
    EdgeAdded { id: SceneId, source: SceneId, target: SceneId },
    /// Edge was removed
    EdgeRemoved { id: SceneId, source: SceneId, target: SceneId },
    /// Edge weight changed
    EdgeWeightChanged { id: SceneId, old_weight: f32, new_weight: f32 },
}

/// Layout quality metrics
#[derive(Debug, Clone)]
pub struct LayoutMetrics {
    /// Total edge crossing count
    pub edge_crossings: usize,
    /// Average edge length
    pub avg_edge_length: f32,
    /// Node distribution uniformity (0.0 = clustered, 1.0 = uniform)
    pub distribution_uniformity: f32,
    /// Stress (sum of squared differences between graph and Euclidean distances)
    pub stress: f32,
    /// Computation time in milliseconds
    pub computation_time_ms: f32,
    /// Number of iterations performed
    pub iterations: usize,
    /// Whether the algorithm converged
    pub converged: bool,
}

/// Layout manager that coordinates different layout algorithms
pub struct LayoutManager {
    /// Currently active layout algorithm
    current_algorithm: Box<dyn LayoutAlgorithm<Config = LayoutConfig>>,
    /// Layout history for undo/redo
    history: Vec<LayoutSnapshot>,
    /// Current layout metrics
    metrics: LayoutMetrics,
    /// Configuration
    config: LayoutConfig,
}

/// Generic layout configuration
#[derive(Debug, Clone)]
pub struct LayoutConfig {
    /// Maximum number of iterations
    pub max_iterations: usize,
    /// Convergence threshold
    pub convergence_threshold: f32,
    /// Enable multi-threading
    pub enable_multithreading: bool,
    /// Random seed for reproducible layouts
    pub random_seed: Option<u64>,
    /// Node spacing factor
    pub node_spacing: f32,
    /// Edge length factor
    pub edge_length: f32,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            max_iterations: 1000,
            convergence_threshold: 0.001,
            enable_multithreading: true,
            random_seed: None,
            node_spacing: 50.0,
            edge_length: 100.0,
        }
    }
}

/// Snapshot of node positions for undo/redo
#[derive(Debug, Clone)]
pub struct LayoutSnapshot {
    /// Node positions
    pub positions: HashMap<SceneId, Point3<f32>>,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Algorithm used
    pub algorithm: String,
    /// Metrics at this snapshot
    pub metrics: LayoutMetrics,
}

impl LayoutManager {
    /// Create a new layout manager with force-directed algorithm
    pub fn new() -> Result<Self, GraphEngineError> {
        let algorithm = Box::new(ForceDirectedLayout::new()?);
        
        Ok(Self {
            current_algorithm: algorithm,
            history: Vec::new(),
            metrics: LayoutMetrics::default(),
            config: LayoutConfig::default(),
        })
    }
    
    /// Apply current layout algorithm to scene
    pub fn apply_layout(&mut self, scene: &mut Scene) -> Result<(), GraphEngineError> {
        let start_time = std::time::Instant::now();
        
        // Create snapshot before applying layout
        let snapshot = self.create_snapshot(scene)?;
        
        // Apply layout
        self.current_algorithm.apply_layout(scene, &self.config)?;
        
        // Calculate metrics
        self.metrics = self.calculate_metrics(scene, start_time.elapsed())?;
        
        // Store snapshot
        self.history.push(snapshot);
        
        // Limit history size
        if self.history.len() > 100 {
            self.history.remove(0);
        }
        
        Ok(())
    }
    
    /// Switch to a different layout algorithm
    pub fn set_algorithm(&mut self, algorithm: Box<dyn LayoutAlgorithm<Config = LayoutConfig>>) {
        self.current_algorithm = algorithm;
    }
    
    /// Get current layout metrics
    pub fn get_metrics(&self) -> &LayoutMetrics {
        &self.metrics
    }
    
    /// Update layout configuration
    pub fn update_config(&mut self, config: LayoutConfig) {
        self.config = config;
    }
    
    /// Undo last layout operation
    pub fn undo(&mut self, scene: &mut Scene) -> Result<(), GraphEngineError> {
        if let Some(snapshot) = self.history.pop() {
            self.apply_snapshot(scene, &snapshot)?;
            self.metrics = snapshot.metrics;
        }
        Ok(())
    }
    
    /// Create a snapshot of current scene state
    fn create_snapshot(&self, scene: &Scene) -> Result<LayoutSnapshot, GraphEngineError> {
        let mut positions = HashMap::new();
        
        for (_, node) in scene.nodes() {
            positions.insert(node.id, node.position);
        }
        
        Ok(LayoutSnapshot {
            positions,
            timestamp: chrono::Utc::now(),
            algorithm: self.current_algorithm.name().to_string(),
            metrics: self.metrics.clone(),
        })
    }
    
    /// Apply a snapshot to the scene
    fn apply_snapshot(&self, scene: &mut Scene, snapshot: &LayoutSnapshot) -> Result<(), GraphEngineError> {
        for (node_id, position) in &snapshot.positions {
            if let Some(node) = scene.get_node_mut(*node_id) {
                node.position = *position;
            }
        }
        Ok(())
    }
    
    /// Calculate layout quality metrics
    fn calculate_metrics(&self, scene: &Scene, computation_time: std::time::Duration) -> Result<LayoutMetrics, GraphEngineError> {
        let edge_crossings = self.count_edge_crossings(scene)?;
        let avg_edge_length = self.calculate_avg_edge_length(scene)?;
        let distribution_uniformity = self.calculate_distribution_uniformity(scene)?;
        let stress = self.calculate_stress(scene)?;
        
        Ok(LayoutMetrics {
            edge_crossings,
            avg_edge_length,
            distribution_uniformity,
            stress,
            computation_time_ms: computation_time.as_secs_f32() * 1000.0,
            iterations: 0, // Would be set by the algorithm
            converged: false, // Would be set by the algorithm
        })
    }
    
    /// Count edge crossings in the layout
    fn count_edge_crossings(&self, scene: &Scene) -> Result<usize, GraphEngineError> {
        let edges: Vec<_> = scene.edges().collect();
        let mut crossings = 0;
        
        for (i, edge1) in edges.iter().enumerate() {
            for edge2 in edges.iter().skip(i + 1) {
                if self.edges_cross(edge1, edge2, scene)? {
                    crossings += 1;
                }
            }
        }
        
        Ok(crossings)
    }
    
    /// Check if two edges cross
    fn edges_cross(&self, edge1: &crate::SceneEdge, edge2: &crate::SceneEdge, scene: &Scene) -> Result<bool, GraphEngineError> {
        let node1_start = scene.get_node(edge1.source).ok_or(GraphEngineError::NodeNotFound(edge1.source))?;
        let node1_end = scene.get_node(edge1.target).ok_or(GraphEngineError::NodeNotFound(edge1.target))?;
        let node2_start = scene.get_node(edge2.source).ok_or(GraphEngineError::NodeNotFound(edge2.source))?;
        let node2_end = scene.get_node(edge2.target).ok_or(GraphEngineError::NodeNotFound(edge2.target))?;
        
        // Project to 2D for crossing calculation (use x,y coordinates)
        let p1 = (node1_start.position.x, node1_start.position.y);
        let p2 = (node1_end.position.x, node1_end.position.y);
        let p3 = (node2_start.position.x, node2_start.position.y);
        let p4 = (node2_end.position.x, node2_end.position.y);
        
        Ok(self.line_segments_intersect(p1, p2, p3, p4))
    }
    
    /// Check if two line segments intersect
    fn line_segments_intersect(&self, p1: (f32, f32), p2: (f32, f32), p3: (f32, f32), p4: (f32, f32)) -> bool {
        let (x1, y1) = p1;
        let (x2, y2) = p2;
        let (x3, y3) = p3;
        let (x4, y4) = p4;
        
        let denom = (x1 - x2) * (y3 - y4) - (y1 - y2) * (x3 - x4);
        if denom.abs() < 1e-10 {
            return false; // Lines are parallel
        }
        
        let t = ((x1 - x3) * (y3 - y4) - (y1 - y3) * (x3 - x4)) / denom;
        let u = -((x1 - x2) * (y1 - y3) - (y1 - y2) * (x1 - x3)) / denom;
        
        t >= 0.0 && t <= 1.0 && u >= 0.0 && u <= 1.0
    }
    
    /// Calculate average edge length
    fn calculate_avg_edge_length(&self, scene: &Scene) -> Result<f32, GraphEngineError> {
        let mut total_length = 0.0;
        let mut count = 0;
        
        for edge in scene.edges() {
            let source = scene.get_node(edge.source).ok_or(GraphEngineError::NodeNotFound(edge.source))?;
            let target = scene.get_node(edge.target).ok_or(GraphEngineError::NodeNotFound(edge.target))?;
            
            let length = (source.position - target.position).norm();
            total_length += length;
            count += 1;
        }
        
        Ok(if count > 0 { total_length / count as f32 } else { 0.0 })
    }
    
    /// Calculate distribution uniformity
    fn calculate_distribution_uniformity(&self, scene: &Scene) -> Result<f32, GraphEngineError> {
        let nodes: Vec<_> = scene.nodes().collect();
        if nodes.len() < 2 {
            return Ok(1.0);
        }
        
        // Calculate pairwise distances
        let mut distances = Vec::new();
        for (i, (_, node1)) in nodes.iter().enumerate() {
            for (_, node2) in nodes.iter().skip(i + 1) {
                let distance = (node1.position - node2.position).norm();
                distances.push(distance);
            }
        }
        
        // Calculate coefficient of variation
        let mean = distances.iter().sum::<f32>() / distances.len() as f32;
        let variance = distances.iter().map(|d| (d - mean).powi(2)).sum::<f32>() / distances.len() as f32;
        let std_dev = variance.sqrt();
        
        let cv = if mean > 0.0 { std_dev / mean } else { 0.0 };
        
        // Convert to uniformity (lower CV = higher uniformity)
        Ok(1.0 / (1.0 + cv))
    }
    
    /// Calculate layout stress
    fn calculate_stress(&self, scene: &Scene) -> Result<f32, GraphEngineError> {
        let nodes: Vec<_> = scene.nodes().collect();
        let mut stress = 0.0;
        
        for (i, (_, node1)) in nodes.iter().enumerate() {
            for (_, node2) in nodes.iter().skip(i + 1) {
                let euclidean_distance = (node1.position - node2.position).norm();
                let graph_distance = self.calculate_graph_distance(node1.id, node2.id, scene)?;
                
                if graph_distance > 0.0 {
                    let diff = euclidean_distance - graph_distance;
                    stress += diff * diff;
                }
            }
        }
        
        Ok(stress)
    }
    
    /// Calculate shortest path distance between two nodes
    fn calculate_graph_distance(&self, start: crate::SceneId, end: crate::SceneId, scene: &Scene) -> Result<f32, GraphEngineError> {
        // Simple BFS for unweighted shortest path
        use std::collections::{HashMap, VecDeque};
        
        let mut distances = HashMap::new();
        let mut queue = VecDeque::new();
        
        distances.insert(start, 0.0);
        queue.push_back(start);
        
        while let Some(current) = queue.pop_front() {
            let current_distance = distances[&current];
            
            if current == end {
                return Ok(current_distance);
            }
            
            for edge in scene.get_connected_edges(current) {
                let neighbor = if edge.source == current { edge.target } else { edge.source };
                
                if !distances.contains_key(&neighbor) {
                    distances.insert(neighbor, current_distance + 1.0);
                    queue.push_back(neighbor);
                }
            }
        }
        
        Ok(f32::INFINITY) // No path found
    }
}

impl Default for LayoutMetrics {
    fn default() -> Self {
        Self {
            edge_crossings: 0,
            avg_edge_length: 0.0,
            distribution_uniformity: 0.0,
            stress: 0.0,
            computation_time_ms: 0.0,
            iterations: 0,
            converged: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_layout_config() {
        let config = LayoutConfig::default();
        assert_eq!(config.max_iterations, 1000);
        assert_eq!(config.convergence_threshold, 0.001);
        assert!(config.enable_multithreading);
        assert_eq!(config.node_spacing, 50.0);
        assert_eq!(config.edge_length, 100.0);
    }
    
    #[test]
    fn test_layout_metrics() {
        let metrics = LayoutMetrics::default();
        assert_eq!(metrics.edge_crossings, 0);
        assert_eq!(metrics.avg_edge_length, 0.0);
        assert_eq!(metrics.distribution_uniformity, 0.0);
        assert_eq!(metrics.stress, 0.0);
        assert_eq!(metrics.computation_time_ms, 0.0);
        assert_eq!(metrics.iterations, 0);
        assert!(!metrics.converged);
    }
    
    #[test]
    fn test_scene_change() {
        let node_id = 1;
        let change = SceneChange::NodeAdded { id: node_id };
        
        match change {
            SceneChange::NodeAdded { id } => assert_eq!(id, node_id),
            _ => panic!("Wrong change type"),
        }
    }
    
    #[test]
    fn test_layout_snapshot() {
        let snapshot = LayoutSnapshot {
            positions: HashMap::new(),
            timestamp: chrono::Utc::now(),
            algorithm: "test".to_string(),
            metrics: LayoutMetrics::default(),
        };
        
        assert_eq!(snapshot.algorithm, "test");
        assert_eq!(snapshot.positions.len(), 0);
    }
}