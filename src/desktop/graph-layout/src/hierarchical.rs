//! Hierarchical layout algorithm implementation
//! 
//! This module implements tree-like layouts for directed graphs with clear hierarchies.

use crate::{
    LayoutAlgorithm, LayoutNode, LayoutEdge, LayoutResult, LayoutError, LayoutType, 
    HierarchicalDirection, LayoutBounds, utils
};
use horizonos_graph_engine::{SceneId, Position};
use std::collections::{HashMap, HashSet, VecDeque};
use nalgebra::Vector3;

/// Hierarchical layout algorithm for tree-like structures
pub struct HierarchicalLayout {
    pub direction: HierarchicalDirection,
    pub layer_spacing: f32,
    pub node_spacing: f32,
    pub bounds: LayoutBounds,
    pub center_subtrees: bool,
    pub minimize_crossings: bool,
}

/// Node information for hierarchical layout
#[derive(Debug, Clone)]
struct HierarchicalNode {
    id: SceneId,
    layer: usize,
    position_in_layer: usize,
    children: Vec<SceneId>,
    parents: Vec<SceneId>,
    subtree_width: f32,
}

impl HierarchicalLayout {
    pub fn new() -> Self {
        HierarchicalLayout {
            direction: HierarchicalDirection::TopToBottom,
            layer_spacing: 80.0,
            node_spacing: 60.0,
            bounds: LayoutBounds::default(),
            center_subtrees: true,
            minimize_crossings: true,
        }
    }
    
    pub fn with_direction(mut self, direction: HierarchicalDirection) -> Self {
        self.direction = direction;
        self
    }
    
    pub fn with_spacing(mut self, layer_spacing: f32, node_spacing: f32) -> Self {
        self.layer_spacing = layer_spacing;
        self.node_spacing = node_spacing;
        self
    }
    
    pub fn with_bounds(mut self, bounds: LayoutBounds) -> Self {
        self.bounds = bounds;
        self
    }
    
    /// Assign nodes to layers using longest path from roots
    fn assign_layers(&self, nodes: &[LayoutNode], edges: &[LayoutEdge]) -> Result<HashMap<SceneId, HierarchicalNode>, LayoutError> {
        let mut node_map: HashMap<SceneId, HierarchicalNode> = HashMap::new();
        let mut children_map: HashMap<SceneId, Vec<SceneId>> = HashMap::new();
        let mut parents_map: HashMap<SceneId, Vec<SceneId>> = HashMap::new();
        let mut in_degree: HashMap<SceneId, usize> = HashMap::new();
        
        // Initialize node map
        for node in nodes {
            node_map.insert(node.id, HierarchicalNode {
                id: node.id,
                layer: 0,
                position_in_layer: 0,
                children: Vec::new(),
                parents: Vec::new(),
                subtree_width: 1.0,
            });
            in_degree.insert(node.id, 0);
        }
        
        // Build parent-child relationships
        for edge in edges {
            children_map.entry(edge.source).or_insert_with(Vec::new).push(edge.target);
            parents_map.entry(edge.target).or_insert_with(Vec::new).push(edge.source);
            *in_degree.get_mut(&edge.target).unwrap() += 1;
        }
        
        // Update node relationships
        for (node_id, node) in node_map.iter_mut() {
            if let Some(children) = children_map.get(node_id) {
                node.children = children.clone();
            }
            if let Some(parents) = parents_map.get(node_id) {
                node.parents = parents.clone();
            }
        }
        
        // Topological sort to assign layers
        let mut queue: VecDeque<SceneId> = VecDeque::new();
        let mut current_in_degree = in_degree.clone();
        
        // Start with nodes that have no incoming edges (roots)
        for (node_id, &degree) in &current_in_degree {
            if degree == 0 {
                queue.push_back(*node_id);
            }
        }
        
        let mut layer = 0;
        let mut nodes_in_current_layer = queue.len();
        
        while !queue.is_empty() {
            if nodes_in_current_layer == 0 {
                layer += 1;
                nodes_in_current_layer = queue.len();
            }
            
            let node_id = queue.pop_front().unwrap();
            nodes_in_current_layer -= 1;
            
            if let Some(node) = node_map.get_mut(&node_id) {
                node.layer = layer;
            }
            
            // Process children
            if let Some(children) = children_map.get(&node_id) {
                for &child_id in children {
                    if let Some(degree) = current_in_degree.get_mut(&child_id) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(child_id);
                        }
                    }
                }
            }
        }
        
        Ok(node_map)
    }
    
    /// Calculate positions within each layer
    fn calculate_layer_positions(&self, hierarchical_nodes: &mut HashMap<SceneId, HierarchicalNode>) {
        // Group nodes by layer
        let mut layers: HashMap<usize, Vec<SceneId>> = HashMap::new();
        for node in hierarchical_nodes.values() {
            layers.entry(node.layer).or_insert_with(Vec::new).push(node.id);
        }
        
        // Sort layers by layer number
        let mut sorted_layers: Vec<_> = layers.into_iter().collect();
        sorted_layers.sort_by_key(|(layer_num, _)| *layer_num);
        
        // Calculate positions for each layer
        for (layer_num, node_ids) in sorted_layers {
            let layer_width = node_ids.len() as f32 * self.node_spacing;
            let start_x = -layer_width / 2.0;
            
            for (i, node_id) in node_ids.iter().enumerate() {
                if let Some(node) = hierarchical_nodes.get_mut(node_id) {
                    node.position_in_layer = i;
                }
            }
        }
        
        // Minimize edge crossings if enabled
        if self.minimize_crossings {
            self.minimize_edge_crossings(hierarchical_nodes);
        }
    }
    
    /// Minimize edge crossings using barycenter method
    fn minimize_edge_crossings(&self, hierarchical_nodes: &mut HashMap<SceneId, HierarchicalNode>) {
        // Group nodes by layer
        let mut layers: HashMap<usize, Vec<SceneId>> = HashMap::new();
        for node in hierarchical_nodes.values() {
            layers.entry(node.layer).or_insert_with(Vec::new).push(node.id);
        }
        
        let max_layer = layers.keys().max().copied().unwrap_or(0);
        
        // Iterate through layers and adjust positions
        for iteration in 0..5 { // Limited iterations to prevent infinite loops
            for layer_num in 0..=max_layer {
                if let Some(layer_nodes) = layers.get(&layer_num) {
                    let mut node_positions: Vec<(SceneId, f32)> = Vec::new();
                    
                    for &node_id in layer_nodes {
                        let barycenter = self.calculate_barycenter(node_id, hierarchical_nodes);
                        node_positions.push((node_id, barycenter));
                    }
                    
                    // Sort by barycenter
                    node_positions.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
                    
                    // Update positions
                    for (i, (node_id, _)) in node_positions.iter().enumerate() {
                        if let Some(node) = hierarchical_nodes.get_mut(node_id) {
                            node.position_in_layer = i;
                        }
                    }
                }
            }
        }
    }
    
    /// Calculate barycenter for a node based on connected nodes in adjacent layers
    fn calculate_barycenter(&self, node_id: SceneId, hierarchical_nodes: &HashMap<SceneId, HierarchicalNode>) -> f32 {
        if let Some(node) = hierarchical_nodes.get(&node_id) {
            let mut sum = 0.0;
            let mut count = 0.0;
            
            // Consider parent positions
            for parent_id in &node.parents {
                if let Some(parent) = hierarchical_nodes.get(parent_id) {
                    sum += parent.position_in_layer as f32;
                    count += 1.0;
                }
            }
            
            // Consider child positions
            for child_id in &node.children {
                if let Some(child) = hierarchical_nodes.get(child_id) {
                    sum += child.position_in_layer as f32;
                    count += 1.0;
                }
            }
            
            if count > 0.0 {
                sum / count
            } else {
                node.position_in_layer as f32
            }
        } else {
            0.0
        }
    }
    
    /// Convert hierarchical positions to world coordinates
    fn convert_to_world_positions(&self, hierarchical_nodes: &HashMap<SceneId, HierarchicalNode>) -> HashMap<SceneId, Position> {
        let mut positions = HashMap::new();
        
        for node in hierarchical_nodes.values() {
            let (x, y, z) = match self.direction {
                HierarchicalDirection::TopToBottom => {
                    let x = (node.position_in_layer as f32 - 0.5) * self.node_spacing;
                    let y = -(node.layer as f32) * self.layer_spacing;
                    (x, y, 0.0)
                }
                HierarchicalDirection::BottomToTop => {
                    let x = (node.position_in_layer as f32 - 0.5) * self.node_spacing;
                    let y = (node.layer as f32) * self.layer_spacing;
                    (x, y, 0.0)
                }
                HierarchicalDirection::LeftToRight => {
                    let x = (node.layer as f32) * self.layer_spacing;
                    let y = (node.position_in_layer as f32 - 0.5) * self.node_spacing;
                    (x, y, 0.0)
                }
                HierarchicalDirection::RightToLeft => {
                    let x = -(node.layer as f32) * self.layer_spacing;
                    let y = (node.position_in_layer as f32 - 0.5) * self.node_spacing;
                    (x, y, 0.0)
                }
            };
            
            let mut position = Position::new(x, y, z);
            utils::apply_bounds(&mut position, &self.bounds);
            positions.insert(node.id, position);
        }
        
        positions
    }
}

impl LayoutAlgorithm for HierarchicalLayout {
    fn calculate_layout(&self, nodes: &[LayoutNode], edges: &[LayoutEdge]) -> Result<LayoutResult, LayoutError> {
        if nodes.is_empty() {
            return Err(LayoutError::InsufficientNodes { count: 0 });
        }
        
        let start_time = chrono::Utc::now();
        
        // Assign nodes to layers
        let mut hierarchical_nodes = self.assign_layers(nodes, edges)?;
        
        // Calculate positions within layers
        self.calculate_layer_positions(&mut hierarchical_nodes);
        
        // Convert to world positions
        let node_positions = self.convert_to_world_positions(&hierarchical_nodes);
        
        let processing_time = chrono::Utc::now() - start_time;
        
        log::info!(
            "Hierarchical layout completed: {} nodes in {} layers",
            nodes.len(),
            hierarchical_nodes.values().map(|n| n.layer).max().unwrap_or(0) + 1
        );
        
        Ok(LayoutResult {
            node_positions,
            iterations_performed: 1, // Hierarchical layout is deterministic
            energy: 0.0, // Not applicable for hierarchical layout
            converged: true,
            processing_time,
        })
    }
    
    fn update_layout(&self, nodes: &mut [LayoutNode], edges: &[LayoutEdge], _delta_time: f32) -> Result<f32, LayoutError> {
        // Hierarchical layout doesn't support incremental updates
        // Would need to recalculate entire layout
        let result = self.calculate_layout(nodes, edges)?;
        
        // Update node positions
        for node in nodes.iter_mut() {
            if let Some(position) = result.node_positions.get(&node.id) {
                node.position = *position;
            }
        }
        
        Ok(0.0)
    }
    
    fn name(&self) -> &str {
        "Hierarchical"
    }
    
    fn supports_incremental(&self) -> bool {
        false
    }
    
    fn recommended_settings(&self) -> LayoutType {
        LayoutType::Hierarchical {
            direction: self.direction.clone(),
            layer_spacing: self.layer_spacing,
            node_spacing: self.node_spacing,
        }
    }
}

impl Default for HierarchicalLayout {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LayoutNode, LayoutEdge, Position};
    
    #[test]
    fn test_hierarchical_layout_creation() {
        let layout = HierarchicalLayout::new()
            .with_direction(HierarchicalDirection::LeftToRight)
            .with_spacing(100.0, 80.0);
        
        assert_eq!(layout.direction, HierarchicalDirection::LeftToRight);
        assert_eq!(layout.layer_spacing, 100.0);
        assert_eq!(layout.node_spacing, 80.0);
    }
    
    #[test]
    fn test_simple_tree_layout() {
        let layout = HierarchicalLayout::new();
        
        let nodes = vec![
            LayoutNode::new(1, Position::new(0.0, 0.0, 0.0)), // root
            LayoutNode::new(2, Position::new(0.0, 0.0, 0.0)), // child 1
            LayoutNode::new(3, Position::new(0.0, 0.0, 0.0)), // child 2
            LayoutNode::new(4, Position::new(0.0, 0.0, 0.0)), // grandchild
        ];
        
        let edges = vec![
            LayoutEdge::new(1, 2, 1.0), // root -> child1
            LayoutEdge::new(1, 3, 1.0), // root -> child2
            LayoutEdge::new(2, 4, 1.0), // child1 -> grandchild
        ];
        
        let result = layout.calculate_layout(&nodes, &edges).unwrap();
        
        assert_eq!(result.node_positions.len(), 4);
        assert!(result.converged);
        assert_eq!(result.iterations_performed, 1);
        
        // Root should be at the top (layer 0)
        let root_pos = result.node_positions.get(&1).unwrap();
        let child1_pos = result.node_positions.get(&2).unwrap();
        
        // In TopToBottom layout, children should be below parents
        assert!(child1_pos.y < root_pos.y);
    }
    
    #[test]
    fn test_layer_assignment() {
        let layout = HierarchicalLayout::new();
        
        let nodes = vec![
            LayoutNode::new(1, Position::new(0.0, 0.0, 0.0)),
            LayoutNode::new(2, Position::new(0.0, 0.0, 0.0)),
            LayoutNode::new(3, Position::new(0.0, 0.0, 0.0)),
        ];
        
        let edges = vec![
            LayoutEdge::new(1, 2, 1.0),
            LayoutEdge::new(2, 3, 1.0),
        ];
        
        let hierarchical_nodes = layout.assign_layers(&nodes, &edges).unwrap();
        
        assert_eq!(hierarchical_nodes.get(&1).unwrap().layer, 0);
        assert_eq!(hierarchical_nodes.get(&2).unwrap().layer, 1);
        assert_eq!(hierarchical_nodes.get(&3).unwrap().layer, 2);
    }
    
    #[test]
    fn test_different_directions() {
        let layout = HierarchicalLayout::new().with_direction(HierarchicalDirection::LeftToRight);
        
        let nodes = vec![
            LayoutNode::new(1, Position::new(0.0, 0.0, 0.0)),
            LayoutNode::new(2, Position::new(0.0, 0.0, 0.0)),
        ];
        
        let edges = vec![LayoutEdge::new(1, 2, 1.0)];
        
        let result = layout.calculate_layout(&nodes, &edges).unwrap();
        
        let root_pos = result.node_positions.get(&1).unwrap();
        let child_pos = result.node_positions.get(&2).unwrap();
        
        // In LeftToRight layout, children should be to the right of parents
        assert!(child_pos.x > root_pos.x);
    }
    
    #[test]
    fn test_empty_graph() {
        let layout = HierarchicalLayout::new();
        let result = layout.calculate_layout(&[], &[]);
        
        assert!(matches!(result, Err(LayoutError::InsufficientNodes { count: 0 })));
    }
    
    #[test]
    fn test_algorithm_properties() {
        let layout = HierarchicalLayout::new();
        
        assert_eq!(layout.name(), "Hierarchical");
        assert!(!layout.supports_incremental());
        
        let settings = layout.recommended_settings();
        assert!(matches!(settings, LayoutType::Hierarchical { .. }));
    }
}