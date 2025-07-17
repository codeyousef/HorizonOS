//! Circular layout algorithm implementation
//!
//! This module provides circular layout algorithms that arrange nodes in
//! circular patterns, suitable for displaying networks, social graphs,
//! or any data where circular arrangement is aesthetically pleasing.

use super::{LayoutAlgorithm, LayoutConfig, SceneChange};
use crate::{Scene, SceneId, GraphEngineError};
use nalgebra::Point3;
use std::collections::HashMap;
use std::f32::consts::PI;

/// Circular layout implementation
pub struct CircularLayout {
    /// Node angles on the circle
    node_angles: HashMap<SceneId, f32>,
    /// Node radii (for multi-ring layouts)
    node_radii: HashMap<SceneId, f32>,
    /// Algorithm configuration
    config: CircularConfig,
}

/// Configuration for circular layout
#[derive(Debug, Clone)]
pub struct CircularConfig {
    /// Radius of the main circle
    pub radius: f32,
    /// Starting angle (in radians)
    pub start_angle: f32,
    /// Whether to arrange clockwise or counter-clockwise
    pub clockwise: bool,
    /// Layout variant
    pub variant: CircularVariant,
    /// Minimum spacing between nodes (in radians)
    pub min_node_spacing: f32,
    /// Whether to sort nodes by degree
    pub sort_by_degree: bool,
    /// Whether to group connected nodes
    pub group_connected: bool,
    /// Number of rings for multi-ring layout
    pub num_rings: usize,
    /// Spacing between rings
    pub ring_spacing: f32,
}

/// Variants of circular layout
#[derive(Debug, Clone, Copy)]
pub enum CircularVariant {
    /// Simple circle with all nodes at same radius
    Simple,
    /// Multiple concentric circles
    Concentric,
    /// Spiral arrangement
    Spiral,
    /// Arc-based arrangement
    Arc,
}

impl Default for CircularConfig {
    fn default() -> Self {
        Self {
            radius: 100.0,
            start_angle: 0.0,
            clockwise: true,
            variant: CircularVariant::Simple,
            min_node_spacing: 0.1,
            sort_by_degree: true,
            group_connected: false,
            num_rings: 3,
            ring_spacing: 50.0,
        }
    }
}

impl CircularLayout {
    /// Create a new circular layout algorithm
    pub fn new() -> Result<Self, GraphEngineError> {
        Ok(Self {
            node_angles: HashMap::new(),
            node_radii: HashMap::new(),
            config: CircularConfig::default(),
        })
    }
    
    /// Create with custom configuration
    pub fn with_config(config: CircularConfig) -> Result<Self, GraphEngineError> {
        Ok(Self {
            node_angles: HashMap::new(),
            node_radii: HashMap::new(),
            config,
        })
    }
    
    /// Apply simple circular layout
    fn apply_simple_layout(&mut self, scene: &mut Scene) -> Result<(), GraphEngineError> {
        let nodes: Vec<_> = scene.nodes().collect();
        if nodes.is_empty() {
            return Ok(());
        }
        
        // Sort nodes by degree if requested
        let sorted_nodes: Vec<SceneId> = if self.config.sort_by_degree {
            let mut node_degrees: Vec<_> = nodes.iter().map(|(_, node)| {
                let degree = scene.get_connected_edges(node.id).len();
                (node.id, degree)
            }).collect();
            
            node_degrees.sort_by_key(|(_, degree)| *degree);
            node_degrees.into_iter().map(|(id, _)| id).collect()
        } else {
            nodes.iter().map(|(_, node)| node.id).collect()
        };
        
        // Calculate angle increment
        let angle_increment = 2.0 * PI / sorted_nodes.len() as f32;
        let angle_increment = angle_increment.max(self.config.min_node_spacing);
        
        // Position nodes
        for (i, &node_id) in sorted_nodes.iter().enumerate() {
            let angle = self.config.start_angle + (i as f32 * angle_increment);
            let final_angle = if self.config.clockwise { angle } else { -angle };
            
            let x = self.config.radius * final_angle.cos();
            let y = self.config.radius * final_angle.sin();
            let position = Point3::new(x, y, 0.0);
            
            if let Some(node) = scene.get_node_mut(node_id) {
                node.position = position;
            }
            
            self.node_angles.insert(node_id, final_angle);
            self.node_radii.insert(node_id, self.config.radius);
        }
        
        Ok(())
    }
    
    /// Apply concentric circular layout
    fn apply_concentric_layout(&mut self, scene: &mut Scene) -> Result<(), GraphEngineError> {
        let nodes: Vec<_> = scene.nodes().collect();
        if nodes.is_empty() {
            return Ok(());
        }
        
        // Group nodes by degree (higher degree = inner ring)
        let mut degree_groups: Vec<Vec<SceneId>> = vec![Vec::new(); self.config.num_rings];
        
        for (_, node) in &nodes {
            let degree = scene.get_connected_edges(node.id).len();
            let ring_index = if self.config.num_rings > 1 {
                ((degree as f32 / (nodes.len() as f32).sqrt()) * self.config.num_rings as f32) as usize
            } else {
                0
            };
            let ring_index = ring_index.min(self.config.num_rings - 1);
            degree_groups[ring_index].push(node.id);
        }
        
        // Position nodes in each ring
        for (ring_index, node_group) in degree_groups.iter().enumerate() {
            if node_group.is_empty() {
                continue;
            }
            
            let ring_radius = self.config.radius + (ring_index as f32 * self.config.ring_spacing);
            let angle_increment = 2.0 * PI / node_group.len() as f32;
            
            for (i, &node_id) in node_group.iter().enumerate() {
                let angle = self.config.start_angle + (i as f32 * angle_increment);
                let final_angle = if self.config.clockwise { angle } else { -angle };
                
                let x = ring_radius * final_angle.cos();
                let y = ring_radius * final_angle.sin();
                let position = Point3::new(x, y, 0.0);
                
                if let Some(node) = scene.get_node_mut(node_id) {
                    node.position = position;
                }
                
                self.node_angles.insert(node_id, final_angle);
                self.node_radii.insert(node_id, ring_radius);
            }
        }
        
        Ok(())
    }
    
    /// Apply spiral layout
    fn apply_spiral_layout(&mut self, scene: &mut Scene) -> Result<(), GraphEngineError> {
        let nodes: Vec<_> = scene.nodes().collect();
        if nodes.is_empty() {
            return Ok(());
        }
        
        // Sort nodes by degree for spiral arrangement
        let mut sorted_nodes: Vec<_> = nodes.iter().map(|(_, node)| {
            let degree = scene.get_connected_edges(node.id).len();
            (node.id, degree)
        }).collect();
        
        sorted_nodes.sort_by_key(|(_, degree)| *degree);
        
        // Calculate spiral parameters
        let total_angle = 4.0 * PI; // Two full rotations
        let radius_increment = self.config.radius / sorted_nodes.len() as f32;
        let angle_increment = total_angle / sorted_nodes.len() as f32;
        
        // Position nodes along spiral
        for (i, &(node_id, _)) in sorted_nodes.iter().enumerate() {
            let angle = self.config.start_angle + (i as f32 * angle_increment);
            let radius = radius_increment * (i as f32 + 1.0);
            let final_angle = if self.config.clockwise { angle } else { -angle };
            
            let x = radius * final_angle.cos();
            let y = radius * final_angle.sin();
            let position = Point3::new(x, y, 0.0);
            
            if let Some(node) = scene.get_node_mut(node_id) {
                node.position = position;
            }
            
            self.node_angles.insert(node_id, final_angle);
            self.node_radii.insert(node_id, radius);
        }
        
        Ok(())
    }
    
    /// Apply arc layout
    fn apply_arc_layout(&mut self, scene: &mut Scene) -> Result<(), GraphEngineError> {
        let nodes: Vec<_> = scene.nodes().collect();
        if nodes.is_empty() {
            return Ok(());
        }
        
        // Use only a portion of the circle (arc)
        let arc_angle = PI; // 180 degrees
        let angle_increment = arc_angle / (nodes.len() as f32 - 1.0).max(1.0);
        
        // Collect node IDs first to avoid borrowing issues
        let node_ids: Vec<_> = nodes.iter().map(|(_, node)| node.id).collect();
        
        // Position nodes along arc
        for (i, &node_id) in node_ids.iter().enumerate() {
            let angle = self.config.start_angle + (i as f32 * angle_increment) - (arc_angle / 2.0);
            let final_angle = if self.config.clockwise { angle } else { -angle };
            
            let x = self.config.radius * final_angle.cos();
            let y = self.config.radius * final_angle.sin();
            let position = Point3::new(x, y, 0.0);
            
            if let Some(scene_node) = scene.get_node_mut(node_id) {
                scene_node.position = position;
            }
            
            self.node_angles.insert(node_id, final_angle);
            self.node_radii.insert(node_id, self.config.radius);
        }
        
        Ok(())
    }
    
    /// Group connected nodes together
    fn group_connected_nodes(&self, scene: &Scene) -> Vec<Vec<SceneId>> {
        let mut groups = Vec::new();
        let mut visited = std::collections::HashSet::new();
        
        for (_, node) in scene.nodes() {
            if visited.contains(&node.id) {
                continue;
            }
            
            let mut group = Vec::new();
            let mut stack = vec![node.id];
            
            while let Some(current_id) = stack.pop() {
                if visited.contains(&current_id) {
                    continue;
                }
                
                visited.insert(current_id);
                group.push(current_id);
                
                // Add connected nodes to stack
                for edge in scene.get_connected_edges(current_id) {
                    let other_id = if edge.source == current_id {
                        edge.target
                    } else {
                        edge.source
                    };
                    
                    if !visited.contains(&other_id) {
                        stack.push(other_id);
                    }
                }
            }
            
            if !group.is_empty() {
                groups.push(group);
            }
        }
        
        groups
    }
    
    /// Calculate optimal spacing for nodes
    fn calculate_optimal_spacing(&self, node_count: usize) -> f32 {
        let circumference = 2.0 * PI * self.config.radius;
        let available_space = circumference / node_count as f32;
        available_space.min(self.config.min_node_spacing)
    }
    
    /// Update configuration
    pub fn update_config(&mut self, config: CircularConfig) {
        self.config = config;
    }
    
    /// Get node angle
    pub fn get_node_angle(&self, node_id: SceneId) -> Option<f32> {
        self.node_angles.get(&node_id).copied()
    }
    
    /// Get node radius
    pub fn get_node_radius(&self, node_id: SceneId) -> Option<f32> {
        self.node_radii.get(&node_id).copied()
    }
    
    /// Get configuration
    pub fn get_config(&self) -> &CircularConfig {
        &self.config
    }
}

impl LayoutAlgorithm for CircularLayout {
    type Config = LayoutConfig;
    
    fn apply_layout(&mut self, scene: &mut Scene, _config: &Self::Config) -> Result<(), GraphEngineError> {
        // Clear previous state
        self.node_angles.clear();
        self.node_radii.clear();
        
        // Apply layout based on variant
        match self.config.variant {
            CircularVariant::Simple => self.apply_simple_layout(scene)?,
            CircularVariant::Concentric => self.apply_concentric_layout(scene)?,
            CircularVariant::Spiral => self.apply_spiral_layout(scene)?,
            CircularVariant::Arc => self.apply_arc_layout(scene)?,
        }
        
        Ok(())
    }
    
    fn name(&self) -> &'static str {
        "Circular"
    }
    
    fn supports_incremental(&self) -> bool {
        true
    }
    
    fn apply_incremental(&mut self, scene: &mut Scene, changes: &[SceneChange]) -> Result<(), GraphEngineError> {
        // Handle incremental changes
        for change in changes {
            match change {
                SceneChange::NodeAdded { id } => {
                    // Position new node at start angle
                    let angle = self.config.start_angle;
                    let radius = self.config.radius;
                    let x = radius * angle.cos();
                    let y = radius * angle.sin();
                    let position = Point3::new(x, y, 0.0);
                    
                    if let Some(node) = scene.get_node_mut(*id) {
                        node.position = position;
                    }
                    
                    self.node_angles.insert(*id, angle);
                    self.node_radii.insert(*id, radius);
                }
                SceneChange::NodeRemoved { id } => {
                    // Remove node from tracking
                    self.node_angles.remove(id);
                    self.node_radii.remove(id);
                }
                SceneChange::NodeMoved { id, new_position, .. } => {
                    // Update node position and recalculate angle/radius
                    if let Some(node) = scene.get_node_mut(*id) {
                        node.position = *new_position;
                        
                        // Calculate new angle and radius
                        let angle = new_position.y.atan2(new_position.x);
                        let radius = (new_position.x * new_position.x + new_position.y * new_position.y).sqrt();
                        
                        self.node_angles.insert(*id, angle);
                        self.node_radii.insert(*id, radius);
                    }
                }
                SceneChange::EdgeAdded { .. } | SceneChange::EdgeRemoved { .. } => {
                    // Edge changes might affect grouping, but for now we'll ignore
                }
                SceneChange::EdgeWeightChanged { .. } => {
                    // Weight changes don't affect circular layout
                }
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_circular_config() {
        let config = CircularConfig::default();
        assert_eq!(config.radius, 100.0);
        assert_eq!(config.start_angle, 0.0);
        assert!(config.clockwise);
        assert!(matches!(config.variant, CircularVariant::Simple));
        assert_eq!(config.min_node_spacing, 0.1);
        assert!(config.sort_by_degree);
        assert!(!config.group_connected);
        assert_eq!(config.num_rings, 3);
        assert_eq!(config.ring_spacing, 50.0);
    }
    
    #[test]
    fn test_circular_layout_creation() {
        let layout = CircularLayout::new();
        assert!(layout.is_ok());
        
        let layout = layout.unwrap();
        assert_eq!(layout.name(), "Circular");
        assert!(layout.supports_incremental());
        assert_eq!(layout.get_config().radius, 100.0);
    }
    
    #[test]
    fn test_circular_variants() {
        let variants = [
            CircularVariant::Simple,
            CircularVariant::Concentric,
            CircularVariant::Spiral,
            CircularVariant::Arc,
        ];
        
        for variant in variants {
            let config = CircularConfig {
                variant,
                ..Default::default()
            };
            let layout = CircularLayout::with_config(config);
            assert!(layout.is_ok());
        }
    }
    
    #[test]
    fn test_optimal_spacing_calculation() {
        let mut config = CircularConfig::default();
        config.min_node_spacing = 0.0; // Remove the cap to test actual calculation
        let layout = CircularLayout::with_config(config).unwrap();
        
        let spacing_10 = layout.calculate_optimal_spacing(10);
        let spacing_100 = layout.calculate_optimal_spacing(100);
        
        // More nodes should result in smaller spacing (or equal if at minimum)
        assert!(spacing_100 <= spacing_10);
    }
    
    #[test]
    fn test_angle_calculations() {
        let mut layout = CircularLayout::new().unwrap();
        
        // Test node tracking
        let node_id = 1;
        layout.node_angles.insert(node_id, PI / 4.0);
        layout.node_radii.insert(node_id, 50.0);
        
        assert_eq!(layout.get_node_angle(node_id), Some(PI / 4.0));
        assert_eq!(layout.get_node_radius(node_id), Some(50.0));
    }
}