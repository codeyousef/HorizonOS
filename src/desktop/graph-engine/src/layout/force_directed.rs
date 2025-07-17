//! Force-directed layout algorithm implementation
//!
//! This module provides a classic force-directed layout algorithm that simulates
//! physical forces between nodes to create an aesthetically pleasing layout.

use super::{LayoutAlgorithm, LayoutConfig, SceneChange};
use crate::{Scene, SceneId, GraphEngineError};
use nalgebra::{Point3, Vector3};
use std::collections::HashMap;

/// Force-directed layout implementation
pub struct ForceDirectedLayout {
    /// Node velocities
    velocities: HashMap<SceneId, Vector3<f32>>,
    /// Algorithm-specific configuration
    config: ForceDirectedConfig,
    /// Current iteration count
    current_iteration: usize,
    /// Convergence tracking
    converged: bool,
}

/// Configuration for force-directed layout
#[derive(Debug, Clone)]
pub struct ForceDirectedConfig {
    /// Repulsion force strength
    pub repulsion_strength: f32,
    /// Attraction force strength
    pub attraction_strength: f32,
    /// Damping factor for velocity
    pub damping: f32,
    /// Time step for simulation
    pub time_step: f32,
    /// Minimum movement threshold for convergence
    pub min_movement: f32,
    /// Maximum force magnitude
    pub max_force: f32,
    /// Optimal edge length
    pub optimal_edge_length: f32,
}

impl Default for ForceDirectedConfig {
    fn default() -> Self {
        Self {
            repulsion_strength: 1000.0,
            attraction_strength: 0.1,
            damping: 0.9,
            time_step: 0.1,
            min_movement: 0.1,
            max_force: 100.0,
            optimal_edge_length: 100.0,
        }
    }
}

impl ForceDirectedLayout {
    /// Create a new force-directed layout algorithm
    pub fn new() -> Result<Self, GraphEngineError> {
        Ok(Self {
            velocities: HashMap::new(),
            config: ForceDirectedConfig::default(),
            current_iteration: 0,
            converged: false,
        })
    }
    
    /// Create with custom configuration
    pub fn with_config(config: ForceDirectedConfig) -> Result<Self, GraphEngineError> {
        Ok(Self {
            velocities: HashMap::new(),
            config,
            current_iteration: 0,
            converged: false,
        })
    }
    
    /// Calculate repulsion force between two nodes
    fn calculate_repulsion(&self, pos1: Point3<f32>, pos2: Point3<f32>) -> Vector3<f32> {
        let diff = pos1 - pos2;
        let distance = diff.norm();
        
        if distance < 0.1 {
            // Avoid division by zero and add small random displacement
            return Vector3::new(
                (rand::random::<f32>() - 0.5) * 0.1,
                (rand::random::<f32>() - 0.5) * 0.1,
                (rand::random::<f32>() - 0.5) * 0.1,
            );
        }
        
        let force_magnitude = self.config.repulsion_strength / (distance * distance);
        let force_direction = diff.normalize();
        
        force_direction * force_magnitude
    }
    
    /// Calculate attraction force for an edge
    fn calculate_attraction(&self, pos1: Point3<f32>, pos2: Point3<f32>) -> Vector3<f32> {
        let diff = pos2 - pos1;
        let distance = diff.norm();
        
        if distance < 0.1 {
            return Vector3::new(0.0, 0.0, 0.0);
        }
        
        let force_magnitude = self.config.attraction_strength * 
                              (distance - self.config.optimal_edge_length).max(0.0);
        let force_direction = diff.normalize();
        
        force_direction * force_magnitude
    }
    
    /// Apply forces to a single node
    fn apply_forces_to_node(&self, node_id: SceneId, scene: &Scene) -> Vector3<f32> {
        let node = scene.get_node(node_id).unwrap();
        let mut total_force = Vector3::new(0.0, 0.0, 0.0);
        
        // Repulsion from all other nodes
        for (_, other_node) in scene.nodes() {
            if other_node.id != node_id {
                let repulsion = self.calculate_repulsion(node.position, other_node.position);
                total_force += repulsion;
            }
        }
        
        // Attraction from connected nodes
        for edge in scene.get_connected_edges(node_id) {
            let other_node_id = if edge.source == node_id {
                edge.target
            } else {
                edge.source
            };
            
            if let Some(other_node) = scene.get_node(other_node_id) {
                let attraction = self.calculate_attraction(node.position, other_node.position);
                total_force += attraction;
            }
        }
        
        // Clamp force magnitude
        let force_magnitude = total_force.norm();
        if force_magnitude > self.config.max_force {
            total_force = total_force.normalize() * self.config.max_force;
        }
        
        total_force
    }
    
    /// Update node positions based on forces
    fn update_positions(&mut self, scene: &mut Scene) -> Result<f32, GraphEngineError> {
        let mut max_movement = 0.0;
        let mut new_positions = HashMap::new();
        
        // Calculate forces and update velocities
        for (_, node) in scene.nodes() {
            let force = self.apply_forces_to_node(node.id, scene);
            
            // Update velocity
            let velocity = self.velocities.entry(node.id).or_insert(Vector3::new(0.0, 0.0, 0.0));
            *velocity = (*velocity + force * self.config.time_step) * self.config.damping;
            
            // Calculate new position
            let new_position = node.position + *velocity * self.config.time_step;
            let movement = (new_position - node.position).norm();
            max_movement = f32::max(max_movement, movement);
            
            new_positions.insert(node.id, new_position);
        }
        
        // Apply new positions
        for (node_id, new_position) in new_positions {
            if let Some(node) = scene.get_node_mut(node_id) {
                node.position = new_position;
            }
        }
        
        Ok(max_movement)
    }
    
    /// Update algorithm configuration
    pub fn update_config(&mut self, config: ForceDirectedConfig) {
        self.config = config;
    }
    
    /// Get current iteration count
    pub fn get_iteration(&self) -> usize {
        self.current_iteration
    }
    
    /// Check if algorithm has converged
    pub fn has_converged(&self) -> bool {
        self.converged
    }
    
    /// Reset algorithm state
    pub fn reset(&mut self) {
        self.velocities.clear();
        self.current_iteration = 0;
        self.converged = false;
    }
}

impl LayoutAlgorithm for ForceDirectedLayout {
    type Config = LayoutConfig;
    
    fn apply_layout(&mut self, scene: &mut Scene, config: &Self::Config) -> Result<(), GraphEngineError> {
        self.reset();
        
        // Initialize velocities
        for (_, node) in scene.nodes() {
            self.velocities.insert(node.id, Vector3::new(0.0, 0.0, 0.0));
        }
        
        // Run simulation
        for iteration in 0..config.max_iterations {
            self.current_iteration = iteration;
            
            let max_movement = self.update_positions(scene)?;
            
            // Check convergence
            if max_movement < config.convergence_threshold {
                self.converged = true;
                break;
            }
            
            // Optional: Add cooling schedule
            if iteration % 100 == 0 {
                self.config.repulsion_strength *= 0.99;
                self.config.time_step *= 0.99;
            }
        }
        
        Ok(())
    }
    
    fn name(&self) -> &'static str {
        "Force-Directed"
    }
    
    fn supports_incremental(&self) -> bool {
        true
    }
    
    fn apply_incremental(&mut self, scene: &mut Scene, changes: &[SceneChange]) -> Result<(), GraphEngineError> {
        // Handle incremental updates
        for change in changes {
            match change {
                SceneChange::NodeAdded { id } => {
                    // Initialize velocity for new node
                    self.velocities.insert(*id, Vector3::new(0.0, 0.0, 0.0));
                    
                    // Position new node randomly
                    if let Some(node) = scene.get_node_mut(*id) {
                        node.position = Point3::new(
                            (rand::random::<f32>() - 0.5) * 200.0,
                            (rand::random::<f32>() - 0.5) * 200.0,
                            (rand::random::<f32>() - 0.5) * 200.0,
                        );
                    }
                }
                SceneChange::NodeRemoved { id } => {
                    // Remove velocity tracking
                    self.velocities.remove(id);
                }
                SceneChange::NodeMoved { id, new_position, .. } => {
                    // Update node position and reset velocity
                    if let Some(node) = scene.get_node_mut(*id) {
                        node.position = *new_position;
                    }
                    self.velocities.insert(*id, Vector3::new(0.0, 0.0, 0.0));
                }
                SceneChange::EdgeAdded { .. } | SceneChange::EdgeRemoved { .. } => {
                    // Edge changes affect forces but don't require special handling
                    // The force calculation will automatically account for new/removed edges
                }
                SceneChange::EdgeWeightChanged { .. } => {
                    // Weight changes could affect attraction strength
                    // For now, we'll let the normal force calculation handle it
                }
            }
        }
        
        // Run a few iterations to stabilize
        let temp_config = LayoutConfig {
            max_iterations: 50,
            convergence_threshold: 0.01,
            ..LayoutConfig::default()
        };
        
        for _ in 0..temp_config.max_iterations {
            let max_movement = self.update_positions(scene)?;
            if max_movement < temp_config.convergence_threshold {
                break;
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_force_directed_config() {
        let config = ForceDirectedConfig::default();
        assert_eq!(config.repulsion_strength, 1000.0);
        assert_eq!(config.attraction_strength, 0.1);
        assert_eq!(config.damping, 0.9);
        assert_eq!(config.time_step, 0.1);
        assert_eq!(config.min_movement, 0.1);
        assert_eq!(config.max_force, 100.0);
        assert_eq!(config.optimal_edge_length, 100.0);
    }
    
    #[test]
    fn test_force_directed_layout_creation() {
        let layout = ForceDirectedLayout::new();
        assert!(layout.is_ok());
        
        let layout = layout.unwrap();
        assert_eq!(layout.name(), "Force-Directed");
        assert!(layout.supports_incremental());
        assert!(!layout.has_converged());
        assert_eq!(layout.get_iteration(), 0);
    }
    
    #[test]
    fn test_force_calculation() {
        let layout = ForceDirectedLayout::new().unwrap();
        
        let pos1 = Point3::new(0.0, 0.0, 0.0);
        let pos2 = Point3::new(200.0, 0.0, 0.0); // Far apart so attraction works
        
        let repulsion = layout.calculate_repulsion(pos1, pos2);
        assert!(repulsion.x < 0.0); // Should repel in negative x direction
        
        let attraction = layout.calculate_attraction(pos1, pos2);
        assert!(attraction.x > 0.0); // Should attract in positive x direction
    }
    
    #[test]
    fn test_force_directed_reset() {
        let mut layout = ForceDirectedLayout::new().unwrap();
        
        // Add some state
        layout.velocities.insert(1, Vector3::new(1.0, 1.0, 1.0));
        layout.current_iteration = 10;
        layout.converged = true;
        
        // Reset
        layout.reset();
        
        assert_eq!(layout.velocities.len(), 0);
        assert_eq!(layout.current_iteration, 0);
        assert!(!layout.converged);
    }
}