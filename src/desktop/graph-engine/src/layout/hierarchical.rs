//! Hierarchical layout algorithm implementation
//!
//! This module provides hierarchical layout algorithms that arrange nodes in
//! tree-like structures, suitable for displaying hierarchical data like file
//! systems, organizational charts, or dependency graphs.

use super::{LayoutAlgorithm, LayoutConfig, SceneChange};
use crate::{Scene, SceneId, GraphEngineError};
use nalgebra::Point3;
use std::collections::{HashMap, HashSet, VecDeque};

/// Hierarchical layout implementation
pub struct HierarchicalLayout {
    /// Node levels in the hierarchy
    levels: HashMap<SceneId, usize>,
    /// Node positions within each level
    level_positions: HashMap<SceneId, usize>,
    /// Root nodes (nodes with no incoming edges)
    roots: Vec<SceneId>,
    /// Algorithm configuration
    config: HierarchicalConfig,
}

/// Configuration for hierarchical layout
#[derive(Debug, Clone)]
pub struct HierarchicalConfig {
    /// Vertical spacing between levels
    pub level_spacing: f32,
    /// Horizontal spacing between nodes in same level
    pub node_spacing: f32,
    /// Layout direction
    pub direction: HierarchicalDirection,
    /// Alignment of nodes within levels
    pub alignment: HierarchicalAlignment,
    /// Whether to minimize edge crossings
    pub minimize_crossings: bool,
    /// Maximum width before wrapping nodes
    pub max_width: Option<f32>,
    /// Whether to balance subtree sizes
    pub balance_subtrees: bool,
}

/// Direction of hierarchical layout
#[derive(Debug, Clone, Copy)]
pub enum HierarchicalDirection {
    /// Top to bottom
    TopToBottom,
    /// Bottom to top
    BottomToTop,
    /// Left to right
    LeftToRight,
    /// Right to left
    RightToLeft,
}

/// Alignment of nodes within levels
#[derive(Debug, Clone, Copy)]
pub enum HierarchicalAlignment {
    /// Center alignment
    Center,
    /// Left alignment
    Left,
    /// Right alignment
    Right,
    /// Justify (spread evenly)
    Justify,
}

impl Default for HierarchicalConfig {
    fn default() -> Self {
        Self {
            level_spacing: 100.0,
            node_spacing: 80.0,
            direction: HierarchicalDirection::TopToBottom,
            alignment: HierarchicalAlignment::Center,
            minimize_crossings: true,
            max_width: None,
            balance_subtrees: true,
        }
    }
}

impl HierarchicalLayout {
    /// Create a new hierarchical layout algorithm
    pub fn new() -> Result<Self, GraphEngineError> {
        Ok(Self {
            levels: HashMap::new(),
            level_positions: HashMap::new(),
            roots: Vec::new(),
            config: HierarchicalConfig::default(),
        })
    }
    
    /// Create with custom configuration
    pub fn with_config(config: HierarchicalConfig) -> Result<Self, GraphEngineError> {
        Ok(Self {
            levels: HashMap::new(),
            level_positions: HashMap::new(),
            roots: Vec::new(),
            config,
        })
    }
    
    /// Find root nodes (nodes with no incoming edges)
    fn find_roots(&self, scene: &Scene) -> Vec<SceneId> {
        let mut has_incoming = HashSet::new();
        
        // Mark all nodes that have incoming edges
        for edge in scene.edges() {
            has_incoming.insert(edge.target);
        }
        
        // Find nodes without incoming edges
        scene.nodes()
            .filter(|(_, node)| !has_incoming.contains(&node.id))
            .map(|(_, node)| node.id)
            .collect()
    }
    
    /// Assign levels to nodes using breadth-first traversal
    fn assign_levels(&mut self, scene: &Scene) -> Result<(), GraphEngineError> {
        self.levels.clear();
        self.roots = self.find_roots(scene);
        
        let mut queue = VecDeque::new();
        
        // Start with root nodes at level 0
        for &root_id in &self.roots {
            self.levels.insert(root_id, 0);
            queue.push_back(root_id);
        }
        
        // If no roots found, pick arbitrary starting nodes
        if self.roots.is_empty() {
            for (_, node) in scene.nodes().take(1) {
                self.levels.insert(node.id, 0);
                queue.push_back(node.id);
            }
        }
        
        // Breadth-first traversal to assign levels
        while let Some(node_id) = queue.pop_front() {
            let current_level = self.levels[&node_id];
            
            // Find outgoing edges
            for edge in scene.get_connected_edges(node_id) {
                let target_id = if edge.source == node_id {
                    edge.target
                } else {
                    continue; // Skip incoming edges
                };
                
                // Assign level to target node
                let target_level = current_level + 1;
                if let Some(&existing_level) = self.levels.get(&target_id) {
                    // If target already has a level, use the maximum
                    if target_level > existing_level {
                        self.levels.insert(target_id, target_level);
                        queue.push_back(target_id);
                    }
                } else {
                    self.levels.insert(target_id, target_level);
                    queue.push_back(target_id);
                }
            }
        }
        
        // Assign remaining nodes to level 0
        for (_, node) in scene.nodes() {
            if !self.levels.contains_key(&node.id) {
                self.levels.insert(node.id, 0);
            }
        }
        
        Ok(())
    }
    
    /// Group nodes by level
    fn group_by_level(&self) -> HashMap<usize, Vec<SceneId>> {
        let mut level_groups = HashMap::new();
        
        for (&node_id, &level) in &self.levels {
            level_groups.entry(level).or_insert_with(Vec::new).push(node_id);
        }
        
        level_groups
    }
    
    /// Minimize edge crossings using barycenter heuristic
    fn minimize_crossings(&mut self, scene: &Scene, level_groups: &mut HashMap<usize, Vec<SceneId>>) -> Result<(), GraphEngineError> {
        if !self.config.minimize_crossings {
            return Ok(());
        }
        
        let max_level = level_groups.keys().max().copied().unwrap_or(0);
        
        // Iterate through levels and minimize crossings
        for iterations in 0..10 {
            let forward = iterations % 2 == 0;
            
            if forward {
                // Forward pass: order based on predecessors
                for level in 1..=max_level {
                    let prev_nodes = level_groups.get(&(level - 1)).cloned().unwrap_or_default();
                    if let Some(nodes) = level_groups.get_mut(&level) {
                        *nodes = self.order_by_predecessors_internal(scene, nodes, &prev_nodes)?;
                    }
                }
            } else {
                // Backward pass: order based on successors
                for level in (0..max_level).rev() {
                    let next_nodes = level_groups.get(&(level + 1)).cloned().unwrap_or_default();
                    if let Some(nodes) = level_groups.get_mut(&level) {
                        *nodes = self.order_by_successors_internal(scene, nodes, &next_nodes)?;
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Order nodes based on their predecessors (internal helper)
    fn order_by_predecessors_internal(&self, scene: &Scene, nodes: &[SceneId], prev_nodes: &[SceneId]) -> Result<Vec<SceneId>, GraphEngineError> {
        let mut node_barycenters: Vec<(SceneId, f32)> = Vec::new();
        
        for &node_id in nodes {
            let mut barycenter = 0.0;
            let mut count = 0;
            
            // Find predecessors in previous level
            for edge in scene.get_connected_edges(node_id) {
                let source_id = if edge.target == node_id {
                    edge.source
                } else {
                    continue;
                };
                
                if let Some(pos) = prev_nodes.iter().position(|&id| id == source_id) {
                    barycenter += pos as f32;
                    count += 1;
                }
            }
            
            if count > 0 {
                barycenter /= count as f32;
            } else {
                barycenter = nodes.iter().position(|&id| id == node_id).unwrap_or(0) as f32;
            }
            
            node_barycenters.push((node_id, barycenter));
        }
        
        // Sort by barycenter
        node_barycenters.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        
        // Return reordered nodes
        Ok(node_barycenters.into_iter().map(|(node_id, _)| node_id).collect())
    }
    
    /// Order nodes based on their successors (internal helper)
    fn order_by_successors_internal(&self, scene: &Scene, nodes: &[SceneId], next_nodes: &[SceneId]) -> Result<Vec<SceneId>, GraphEngineError> {
        let mut node_barycenters: Vec<(SceneId, f32)> = Vec::new();
        
        for &node_id in nodes {
            let mut barycenter = 0.0;
            let mut count = 0;
            
            // Find successors in next level
            for edge in scene.get_connected_edges(node_id) {
                let target_id = if edge.source == node_id {
                    edge.target
                } else {
                    continue;
                };
                
                if let Some(pos) = next_nodes.iter().position(|&id| id == target_id) {
                    barycenter += pos as f32;
                    count += 1;
                }
            }
            
            if count > 0 {
                barycenter /= count as f32;
            } else {
                barycenter = nodes.iter().position(|&id| id == node_id).unwrap_or(0) as f32;
            }
            
            node_barycenters.push((node_id, barycenter));
        }
        
        // Sort by barycenter
        node_barycenters.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        
        // Return reordered nodes
        Ok(node_barycenters.into_iter().map(|(node_id, _)| node_id).collect())
    }
    
    /// Calculate positions for nodes in each level
    fn calculate_positions(&mut self, scene: &mut Scene, level_groups: &HashMap<usize, Vec<SceneId>>) -> Result<(), GraphEngineError> {
        for (&level, nodes) in level_groups {
            self.position_level(scene, level, nodes)?;
        }
        
        Ok(())
    }
    
    /// Position nodes within a single level
    fn position_level(&mut self, scene: &mut Scene, level: usize, nodes: &[SceneId]) -> Result<(), GraphEngineError> {
        if nodes.is_empty() {
            return Ok(());
        }
        
        let level_y = self.calculate_level_y(level);
        let total_width = (nodes.len() - 1) as f32 * self.config.node_spacing;
        
        // Calculate starting x position based on alignment
        let start_x = match self.config.alignment {
            HierarchicalAlignment::Center => -total_width / 2.0,
            HierarchicalAlignment::Left => 0.0,
            HierarchicalAlignment::Right => -total_width,
            HierarchicalAlignment::Justify => {
                if nodes.len() == 1 {
                    0.0
                } else {
                    -total_width / 2.0
                }
            }
        };
        
        // Position nodes
        for (i, &node_id) in nodes.iter().enumerate() {
            let x = if matches!(self.config.alignment, HierarchicalAlignment::Justify) && nodes.len() > 1 {
                start_x + (i as f32 * total_width / (nodes.len() - 1) as f32)
            } else {
                start_x + (i as f32 * self.config.node_spacing)
            };
            
            let position = match self.config.direction {
                HierarchicalDirection::TopToBottom => Point3::new(x, level_y, 0.0),
                HierarchicalDirection::BottomToTop => Point3::new(x, -level_y, 0.0),
                HierarchicalDirection::LeftToRight => Point3::new(level_y, x, 0.0),
                HierarchicalDirection::RightToLeft => Point3::new(-level_y, x, 0.0),
            };
            
            if let Some(node) = scene.get_node_mut(node_id) {
                node.position = position;
            }
            
            self.level_positions.insert(node_id, i);
        }
        
        Ok(())
    }
    
    /// Calculate Y coordinate for a level
    fn calculate_level_y(&self, level: usize) -> f32 {
        level as f32 * self.config.level_spacing
    }
    
    /// Balance subtree sizes to improve layout
    fn balance_subtrees(&mut self, scene: &Scene, level_groups: &mut HashMap<usize, Vec<SceneId>>) -> Result<(), GraphEngineError> {
        if !self.config.balance_subtrees {
            return Ok(());
        }
        
        // This is a simplified balancing approach
        // In a full implementation, we would consider subtree sizes and adjust positions
        for nodes in level_groups.values_mut() {
            // Sort by subtree size (approximated by number of descendants)
            nodes.sort_by_key(|&node_id| {
                self.count_descendants(scene, node_id)
            });
        }
        
        Ok(())
    }
    
    /// Count descendants of a node
    fn count_descendants(&self, scene: &Scene, node_id: SceneId) -> usize {
        let mut count = 0;
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        
        queue.push_back(node_id);
        visited.insert(node_id);
        
        while let Some(current_id) = queue.pop_front() {
            for edge in scene.get_connected_edges(current_id) {
                let target_id = if edge.source == current_id {
                    edge.target
                } else {
                    continue;
                };
                
                if !visited.contains(&target_id) {
                    visited.insert(target_id);
                    queue.push_back(target_id);
                    count += 1;
                }
            }
        }
        
        count
    }
    
    /// Update configuration
    pub fn update_config(&mut self, config: HierarchicalConfig) {
        self.config = config;
    }
    
    /// Get node level
    pub fn get_node_level(&self, node_id: SceneId) -> Option<usize> {
        self.levels.get(&node_id).copied()
    }
    
    /// Get node position within level
    pub fn get_level_position(&self, node_id: SceneId) -> Option<usize> {
        self.level_positions.get(&node_id).copied()
    }
    
    /// Get root nodes
    pub fn get_roots(&self) -> &[SceneId] {
        &self.roots
    }
}

impl LayoutAlgorithm for HierarchicalLayout {
    type Config = LayoutConfig;
    
    fn apply_layout(&mut self, scene: &mut Scene, _config: &Self::Config) -> Result<(), GraphEngineError> {
        // Clear previous state
        self.levels.clear();
        self.level_positions.clear();
        self.roots.clear();
        
        // Assign levels to nodes
        self.assign_levels(scene)?;
        
        // Group nodes by level
        let mut level_groups = self.group_by_level();
        
        // Balance subtrees
        self.balance_subtrees(scene, &mut level_groups)?;
        
        // Minimize edge crossings
        self.minimize_crossings(scene, &mut level_groups)?;
        
        // Calculate final positions
        self.calculate_positions(scene, &level_groups)?;
        
        Ok(())
    }
    
    fn name(&self) -> &'static str {
        "Hierarchical"
    }
    
    fn supports_incremental(&self) -> bool {
        false // Hierarchical layout typically requires full recalculation
    }
    
    fn apply_incremental(&mut self, scene: &mut Scene, _changes: &[SceneChange]) -> Result<(), GraphEngineError> {
        // For hierarchical layout, we typically need to recalculate everything
        let config = LayoutConfig::default();
        self.apply_layout(scene, &config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_hierarchical_config() {
        let config = HierarchicalConfig::default();
        assert_eq!(config.level_spacing, 100.0);
        assert_eq!(config.node_spacing, 80.0);
        assert!(matches!(config.direction, HierarchicalDirection::TopToBottom));
        assert!(matches!(config.alignment, HierarchicalAlignment::Center));
        assert!(config.minimize_crossings);
        assert!(config.balance_subtrees);
        assert!(config.max_width.is_none());
    }
    
    #[test]
    fn test_hierarchical_layout_creation() {
        let layout = HierarchicalLayout::new();
        assert!(layout.is_ok());
        
        let layout = layout.unwrap();
        assert_eq!(layout.name(), "Hierarchical");
        assert!(!layout.supports_incremental());
        assert_eq!(layout.get_roots().len(), 0);
    }
    
    #[test]
    fn test_hierarchical_direction() {
        let directions = [
            HierarchicalDirection::TopToBottom,
            HierarchicalDirection::BottomToTop,
            HierarchicalDirection::LeftToRight,
            HierarchicalDirection::RightToLeft,
        ];
        
        for direction in directions {
            let config = HierarchicalConfig {
                direction,
                ..Default::default()
            };
            let layout = HierarchicalLayout::with_config(config);
            assert!(layout.is_ok());
        }
    }
    
    #[test]
    fn test_hierarchical_alignment() {
        let alignments = [
            HierarchicalAlignment::Center,
            HierarchicalAlignment::Left,
            HierarchicalAlignment::Right,
            HierarchicalAlignment::Justify,
        ];
        
        for alignment in alignments {
            let config = HierarchicalConfig {
                alignment,
                ..Default::default()
            };
            let layout = HierarchicalLayout::with_config(config);
            assert!(layout.is_ok());
        }
    }
    
    #[test]
    fn test_level_calculation() {
        let layout = HierarchicalLayout::new().unwrap();
        
        // Test level Y calculation
        assert_eq!(layout.calculate_level_y(0), 0.0);
        assert_eq!(layout.calculate_level_y(1), 100.0);
        assert_eq!(layout.calculate_level_y(2), 200.0);
    }
}