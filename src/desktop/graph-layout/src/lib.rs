//! Layout engine for HorizonOS graph desktop
//! 
//! This module provides various algorithms for positioning nodes in the graph desktop,
//! including force-directed layouts, hierarchical arrangements, and specialized layouts.

pub use horizonos_graph_engine::{SceneId, Position, Vec3};
pub use horizonos_graph_nodes::GraphNode;
pub use horizonos_graph_edges::{GraphEdge, EdgeManager};

pub mod force_directed;
pub mod hierarchical;
pub mod circular;
pub mod grid;
pub mod cluster;
pub mod temporal;
pub mod manager;

pub use force_directed::*;
pub use hierarchical::*;
pub use circular::*;
pub use grid::*;
pub use cluster::*;
pub use temporal::*;
pub use manager::*;

use nalgebra::{Vector3, Point3};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Different layout algorithms available
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LayoutType {
    ForceDirected {
        spring_strength: f32,
        repulsion_strength: f32,
        damping: f32,
    },
    Hierarchical {
        direction: HierarchicalDirection,
        layer_spacing: f32,
        node_spacing: f32,
    },
    Circular {
        radius: f32,
        center: Position,
    },
    Grid {
        cell_size: f32,
        columns: Option<usize>,
    },
    Cluster {
        cluster_separation: f32,
        cluster_compactness: f32,
    },
    Temporal {
        time_axis: TimeAxis,
        time_scale: f32,
    },
    Manual, // User-positioned nodes
}

/// Direction for hierarchical layouts
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HierarchicalDirection {
    TopToBottom,
    BottomToTop,
    LeftToRight,
    RightToLeft,
}

/// Time axis for temporal layouts
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TimeAxis {
    X, // Horizontal timeline
    Y, // Vertical timeline
    Z, // Depth-based timeline
}

/// Node positioning data for layout algorithms
#[derive(Debug, Clone)]
pub struct LayoutNode {
    pub id: SceneId,
    pub position: Position,
    pub velocity: Vector3<f32>,
    pub mass: f32,
    pub fixed: bool, // Whether the node position is locked
    pub cluster_id: Option<String>,
    pub timestamp: Option<chrono::DateTime<chrono::Utc>>,
}

/// Edge data for layout calculations
#[derive(Debug, Clone)]
pub struct LayoutEdge {
    pub source: SceneId,
    pub target: SceneId,
    pub weight: f32,
    pub length: f32, // Desired edge length
}

/// Layout calculation result
#[derive(Debug, Clone)]
pub struct LayoutResult {
    pub node_positions: HashMap<SceneId, Position>,
    pub iterations_performed: usize,
    pub energy: f32, // Final system energy
    pub converged: bool,
    pub processing_time: chrono::Duration,
}

/// Settings for layout animation and transitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutAnimationSettings {
    pub enabled: bool,
    pub duration: f32, // seconds
    pub easing: EasingFunction,
    pub max_distance: f32, // Maximum distance a node can move per frame
}

/// Easing functions for smooth animations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EasingFunction {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    Bounce,
}

/// Error types for layout operations
#[derive(Debug, thiserror::Error)]
pub enum LayoutError {
    #[error("Invalid layout configuration: {message}")]
    InvalidConfiguration { message: String },
    
    #[error("Layout calculation failed: {reason}")]
    CalculationFailed { reason: String },
    
    #[error("Node not found: {id}")]
    NodeNotFound { id: SceneId },
    
    #[error("Insufficient nodes for layout: {count}")]
    InsufficientNodes { count: usize },
    
    #[error("Layout timeout exceeded")]
    Timeout,
    
    #[error("System error: {message}")]
    SystemError { message: String },
}

/// Trait for layout algorithms
pub trait LayoutAlgorithm: Send + Sync {
    /// Calculate new positions for nodes
    fn calculate_layout(&self, nodes: &[LayoutNode], edges: &[LayoutEdge]) -> Result<LayoutResult, LayoutError>;
    
    /// Update layout incrementally (for real-time layouts)
    fn update_layout(&self, nodes: &mut [LayoutNode], edges: &[LayoutEdge], delta_time: f32) -> Result<f32, LayoutError>;
    
    /// Get the name of this layout algorithm
    fn name(&self) -> &str;
    
    /// Check if this algorithm supports incremental updates
    fn supports_incremental(&self) -> bool { false }
    
    /// Get recommended settings for this layout type
    fn recommended_settings(&self) -> LayoutType;
}

impl Default for LayoutType {
    fn default() -> Self {
        LayoutType::ForceDirected {
            spring_strength: 0.05,
            repulsion_strength: 100.0,
            damping: 0.9,
        }
    }
}

impl Default for LayoutAnimationSettings {
    fn default() -> Self {
        LayoutAnimationSettings {
            enabled: true,
            duration: 1.0,
            easing: EasingFunction::EaseOut,
            max_distance: 10.0,
        }
    }
}

impl LayoutNode {
    pub fn new(id: SceneId, position: Position) -> Self {
        LayoutNode {
            id,
            position,
            velocity: Vector3::zeros(),
            mass: 1.0,
            fixed: false,
            cluster_id: None,
            timestamp: None,
        }
    }
    
    pub fn with_mass(mut self, mass: f32) -> Self {
        self.mass = mass;
        self
    }
    
    pub fn with_cluster(mut self, cluster_id: String) -> Self {
        self.cluster_id = Some(cluster_id);
        self
    }
    
    pub fn with_timestamp(mut self, timestamp: chrono::DateTime<chrono::Utc>) -> Self {
        self.timestamp = Some(timestamp);
        self
    }
    
    pub fn fixed(mut self) -> Self {
        self.fixed = true;
        self
    }
}

impl LayoutEdge {
    pub fn new(source: SceneId, target: SceneId, weight: f32) -> Self {
        LayoutEdge {
            source,
            target,
            weight,
            length: weight * 50.0, // Default length based on weight
        }
    }
    
    pub fn with_length(mut self, length: f32) -> Self {
        self.length = length;
        self
    }
}

/// Utility functions for layout calculations
pub mod utils {
    use super::*;
    use nalgebra::{Point3, Vector3};
    
    /// Calculate distance between two positions
    pub fn distance(a: &Position, b: &Position) -> f32 {
        let diff = Vector3::new(a.x - b.x, a.y - b.y, a.z - b.z);
        diff.magnitude()
    }
    
    /// Calculate the center of mass for a set of nodes
    pub fn center_of_mass(nodes: &[LayoutNode]) -> Position {
        if nodes.is_empty() {
            return Position::new(0.0, 0.0, 0.0);
        }
        
        let mut total_mass = 0.0;
        let mut weighted_sum = Vector3::zeros();
        
        for node in nodes {
            let pos = Vector3::new(node.position.x, node.position.y, node.position.z);
            weighted_sum += pos * node.mass;
            total_mass += node.mass;
        }
        
        if total_mass > 0.0 {
            let center = weighted_sum / total_mass;
            Position::new(center.x, center.y, center.z)
        } else {
            Position::new(0.0, 0.0, 0.0)
        }
    }
    
    /// Apply easing function to a value between 0 and 1
    pub fn apply_easing(t: f32, easing: &EasingFunction) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match easing {
            EasingFunction::Linear => t,
            EasingFunction::EaseIn => t * t,
            EasingFunction::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
            EasingFunction::EaseInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - 2.0 * (1.0 - t) * (1.0 - t)
                }
            }
            EasingFunction::Bounce => {
                if t < 1.0 / 2.75 {
                    7.5625 * t * t
                } else if t < 2.0 / 2.75 {
                    let t = t - 1.5 / 2.75;
                    7.5625 * t * t + 0.75
                } else if t < 2.5 / 2.75 {
                    let t = t - 2.25 / 2.75;
                    7.5625 * t * t + 0.9375
                } else {
                    let t = t - 2.625 / 2.75;
                    7.5625 * t * t + 0.984375
                }
            }
        }
    }
    
    /// Generate a random position within bounds
    pub fn random_position(bounds: &LayoutBounds) -> Position {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        
        Position::new(
            rng.gen_range(bounds.min_x..=bounds.max_x),
            rng.gen_range(bounds.min_y..=bounds.max_y),
            rng.gen_range(bounds.min_z..=bounds.max_z),
        )
    }
    
    /// Keep nodes within specified bounds
    pub fn apply_bounds(position: &mut Position, bounds: &LayoutBounds) {
        position.x = position.x.clamp(bounds.min_x, bounds.max_x);
        position.y = position.y.clamp(bounds.min_y, bounds.max_y);
        position.z = position.z.clamp(bounds.min_z, bounds.max_z);
    }
}

/// Bounds for layout calculations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutBounds {
    pub min_x: f32,
    pub max_x: f32,
    pub min_y: f32,
    pub max_y: f32,
    pub min_z: f32,
    pub max_z: f32,
}

impl Default for LayoutBounds {
    fn default() -> Self {
        LayoutBounds {
            min_x: -100.0,
            max_x: 100.0,
            min_y: -100.0,
            max_y: 100.0,
            min_z: -50.0,
            max_z: 50.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_layout_node_creation() {
        let node = LayoutNode::new(1, Position::new(0.0, 0.0, 0.0))
            .with_mass(2.0)
            .with_cluster("test_cluster".to_string())
            .fixed();
        
        assert_eq!(node.id, 1);
        assert_eq!(node.mass, 2.0);
        assert_eq!(node.cluster_id, Some("test_cluster".to_string()));
        assert!(node.fixed);
    }
    
    #[test]
    fn test_layout_edge_creation() {
        let edge = LayoutEdge::new(1, 2, 0.5).with_length(25.0);
        
        assert_eq!(edge.source, 1);
        assert_eq!(edge.target, 2);
        assert_eq!(edge.weight, 0.5);
        assert_eq!(edge.length, 25.0);
    }
    
    #[test]
    fn test_distance_calculation() {
        let pos1 = Position::new(0.0, 0.0, 0.0);
        let pos2 = Position::new(3.0, 4.0, 0.0);
        
        assert_eq!(utils::distance(&pos1, &pos2), 5.0);
    }
    
    #[test]
    fn test_center_of_mass() {
        let nodes = vec![
            LayoutNode::new(1, Position::new(0.0, 0.0, 0.0)).with_mass(1.0),
            LayoutNode::new(2, Position::new(2.0, 0.0, 0.0)).with_mass(1.0),
        ];
        
        let center = utils::center_of_mass(&nodes);
        assert_eq!(center.x, 1.0);
        assert_eq!(center.y, 0.0);
        assert_eq!(center.z, 0.0);
    }
    
    #[test]
    fn test_easing_functions() {
        assert_eq!(utils::apply_easing(0.0, &EasingFunction::Linear), 0.0);
        assert_eq!(utils::apply_easing(1.0, &EasingFunction::Linear), 1.0);
        assert_eq!(utils::apply_easing(0.5, &EasingFunction::Linear), 0.5);
        
        // Test bounds
        assert_eq!(utils::apply_easing(-1.0, &EasingFunction::Linear), 0.0);
        assert_eq!(utils::apply_easing(2.0, &EasingFunction::Linear), 1.0);
    }
    
    #[test]
    fn test_bounds_application() {
        let bounds = LayoutBounds::default();
        let mut position = Position::new(200.0, -200.0, 100.0);
        
        utils::apply_bounds(&mut position, &bounds);
        
        assert_eq!(position.x, bounds.max_x);
        assert_eq!(position.y, bounds.min_y);
        assert_eq!(position.z, bounds.max_z);
    }
}