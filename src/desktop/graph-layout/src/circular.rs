//! Circular layout algorithm implementation
//! 
//! This module implements circular and radial layouts for graph visualization.

use crate::{
    LayoutAlgorithm, LayoutNode, LayoutEdge, LayoutResult, LayoutError, LayoutType, 
    LayoutBounds, utils
};
use horizonos_graph_engine::{SceneId, Position};
use std::collections::{HashMap, HashSet};
use std::f32::consts::PI;

/// Circular layout algorithm
pub struct CircularLayout {
    pub radius: f32,
    pub center: Position,
    pub start_angle: f32,
    pub clockwise: bool,
    pub bounds: LayoutBounds,
    pub variant: CircularVariant,
}

/// Different circular layout variants
#[derive(Debug, Clone)]
pub enum CircularVariant {
    SimpleCircle,
    ConcentricCircles { rings: usize },
    Spiral { spacing: f32 },
    RadialTree { root_id: Option<SceneId> },
}

impl CircularLayout {
    pub fn new() -> Self {
        CircularLayout {
            radius: 50.0,
            center: Position::new(0.0, 0.0, 0.0),
            start_angle: 0.0,
            clockwise: true,
            bounds: LayoutBounds::default(),
            variant: CircularVariant::SimpleCircle,
        }
    }
    
    pub fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }
    
    pub fn with_center(mut self, center: Position) -> Self {
        self.center = center;
        self
    }
    
    pub fn with_start_angle(mut self, angle: f32) -> Self {
        self.start_angle = angle;
        self
    }
    
    pub fn with_variant(mut self, variant: CircularVariant) -> Self {
        self.variant = variant;
        self
    }
    
    pub fn with_bounds(mut self, bounds: LayoutBounds) -> Self {
        self.bounds = bounds;
        self
    }
    
    /// Calculate positions for simple circle layout
    fn calculate_simple_circle(&self, nodes: &[LayoutNode]) -> HashMap<SceneId, Position> {
        let mut positions = HashMap::new();
        let node_count = nodes.len();
        
        if node_count == 0 {
            return positions;
        }
        
        let angle_increment = 2.0 * PI / node_count as f32;
        let direction_multiplier = if self.clockwise { 1.0 } else { -1.0 };
        
        for (i, node) in nodes.iter().enumerate() {
            let angle = self.start_angle + (i as f32 * angle_increment * direction_multiplier);
            
            let x = self.center.x + self.radius * angle.cos();
            let y = self.center.y + self.radius * angle.sin();
            let z = self.center.z;
            
            let mut position = Position::new(x, y, z);
            utils::apply_bounds(&mut position, &self.bounds);
            positions.insert(node.id, position);
        }
        
        positions
    }
    
    /// Calculate positions for concentric circles layout
    fn calculate_concentric_circles(&self, nodes: &[LayoutNode], edges: &[LayoutEdge], rings: usize) -> HashMap<SceneId, Position> {
        let mut positions = HashMap::new();
        
        if nodes.is_empty() || rings == 0 {
            return positions;
        }
        
        // Find connected components and assign to rings based on distance from center
        let center_nodes = self.find_center_nodes(nodes, edges);
        let node_rings = self.assign_nodes_to_rings(nodes, edges, &center_nodes, rings);
        
        // Calculate positions for each ring
        for ring in 0..rings {
            let ring_nodes: Vec<_> = node_rings.iter()
                .filter(|(_, &node_ring)| node_ring == ring)
                .map(|(&node_id, _)| node_id)
                .collect();
            
            if ring_nodes.is_empty() {
                continue;
            }
            
            let ring_radius = self.radius * (ring + 1) as f32 / rings as f32;
            let angle_increment = 2.0 * PI / ring_nodes.len() as f32;
            
            for (i, &node_id) in ring_nodes.iter().enumerate() {
                let angle = self.start_angle + (i as f32 * angle_increment);
                
                let x = self.center.x + ring_radius * angle.cos();
                let y = self.center.y + ring_radius * angle.sin();
                let z = self.center.z;
                
                let mut position = Position::new(x, y, z);
                utils::apply_bounds(&mut position, &self.bounds);
                positions.insert(node_id, position);
            }
        }
        
        positions
    }
    
    /// Calculate positions for spiral layout
    fn calculate_spiral(&self, nodes: &[LayoutNode], spacing: f32) -> HashMap<SceneId, Position> {
        let mut positions = HashMap::new();
        
        if nodes.is_empty() {
            return positions;
        }
        
        let mut current_radius = spacing;
        let mut current_angle = self.start_angle;
        let angle_increment = spacing / current_radius; // Adaptive angle increment
        
        for node in nodes {
            let x = self.center.x + current_radius * current_angle.cos();
            let y = self.center.y + current_radius * current_angle.sin();
            let z = self.center.z;
            
            let mut position = Position::new(x, y, z);
            utils::apply_bounds(&mut position, &self.bounds);
            positions.insert(node.id, position);
            
            // Update for next node
            current_angle += angle_increment;
            current_radius += spacing * 0.1; // Gradual radius increase
            
            // Recalculate angle increment for new radius
            if current_radius > 0.0 {
                angle_increment = spacing / current_radius;
            }
        }
        
        positions
    }
    
    /// Calculate positions for radial tree layout
    fn calculate_radial_tree(&self, nodes: &[LayoutNode], edges: &[LayoutEdge], root_id: Option<SceneId>) -> HashMap<SceneId, Position> {
        let mut positions = HashMap::new();
        
        if nodes.is_empty() {
            return positions;
        }
        
        // Find or select root node
        let root = root_id.or_else(|| self.find_best_root(nodes, edges))
            .unwrap_or_else(|| nodes[0].id);
        
        // Build tree structure
        let tree = self.build_tree_from_root(nodes, edges, root);
        
        // Place root at center
        positions.insert(root, self.center);
        
        // Calculate positions for each level
        self.place_tree_nodes_radially(&tree, root, &mut positions, 0, 2.0 * PI, 1);
        
        positions
    }
    
    /// Find nodes that could serve as good centers (high connectivity)
    fn find_center_nodes(&self, nodes: &[LayoutNode], edges: &[LayoutEdge]) -> Vec<SceneId> {
        let mut connectivity: HashMap<SceneId, usize> = HashMap::new();
        
        // Count connections for each node
        for node in nodes {
            connectivity.insert(node.id, 0);
        }
        
        for edge in edges {
            *connectivity.entry(edge.source).or_insert(0) += 1;
            *connectivity.entry(edge.target).or_insert(0) += 1;
        }
        
        // Sort by connectivity
        let mut sorted_nodes: Vec<_> = connectivity.into_iter().collect();
        sorted_nodes.sort_by(|a, b| b.1.cmp(&a.1));
        
        // Return top 10% or at least 1
        let count = (sorted_nodes.len() / 10).max(1);
        sorted_nodes.into_iter().take(count).map(|(id, _)| id).collect()
    }
    
    /// Assign nodes to rings based on distance from center nodes
    fn assign_nodes_to_rings(&self, nodes: &[LayoutNode], edges: &[LayoutEdge], center_nodes: &[SceneId], rings: usize) -> HashMap<SceneId, usize> {
        let mut node_rings = HashMap::new();
        let mut distances = HashMap::new();
        
        // Initialize distances
        for node in nodes {
            distances.insert(node.id, if center_nodes.contains(&node.id) { 0 } else { usize::MAX });
        }
        
        // BFS to find distances from center nodes
        let mut queue = std::collections::VecDeque::new();
        for &center_id in center_nodes {
            queue.push_back((center_id, 0));
            distances.insert(center_id, 0);
        }
        
        // Build adjacency list
        let mut adjacency: HashMap<SceneId, Vec<SceneId>> = HashMap::new();
        for edge in edges {
            adjacency.entry(edge.source).or_insert_with(Vec::new).push(edge.target);
            adjacency.entry(edge.target).or_insert_with(Vec::new).push(edge.source);
        }
        
        while let Some((node_id, dist)) = queue.pop_front() {
            if let Some(neighbors) = adjacency.get(&node_id) {
                for &neighbor in neighbors {
                    if distances.get(&neighbor).copied().unwrap_or(usize::MAX) > dist + 1 {
                        distances.insert(neighbor, dist + 1);
                        queue.push_back((neighbor, dist + 1));
                    }
                }
            }
        }
        
        // Assign to rings based on distance
        let max_distance = distances.values().filter(|&&d| d != usize::MAX).max().copied().unwrap_or(0);
        
        for (&node_id, &distance) in &distances {
            if distance == usize::MAX {
                node_rings.insert(node_id, rings - 1); // Outliers go to outer ring
            } else {
                let ring = if max_distance > 0 {
                    (distance * (rings - 1) / max_distance).min(rings - 1)
                } else {
                    0
                };
                node_rings.insert(node_id, ring);
            }
        }
        
        node_rings
    }
    
    /// Find the best root node for radial tree layout
    fn find_best_root(&self, nodes: &[LayoutNode], edges: &[LayoutEdge]) -> Option<SceneId> {
        let center_nodes = self.find_center_nodes(nodes, edges);
        center_nodes.first().copied()
    }
    
    /// Build tree structure from root
    fn build_tree_from_root(&self, nodes: &[LayoutNode], edges: &[LayoutEdge], root: SceneId) -> HashMap<SceneId, Vec<SceneId>> {
        let mut tree: HashMap<SceneId, Vec<SceneId>> = HashMap::new();
        let mut visited = HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        
        // Build adjacency list
        let mut adjacency: HashMap<SceneId, Vec<SceneId>> = HashMap::new();
        for edge in edges {
            adjacency.entry(edge.source).or_insert_with(Vec::new).push(edge.target);
            adjacency.entry(edge.target).or_insert_with(Vec::new).push(edge.source);
        }
        
        // BFS from root
        queue.push_back(root);
        visited.insert(root);
        
        while let Some(node_id) = queue.pop_front() {
            if let Some(neighbors) = adjacency.get(&node_id) {
                for &neighbor in neighbors {
                    if !visited.contains(&neighbor) {
                        visited.insert(neighbor);
                        tree.entry(node_id).or_insert_with(Vec::new).push(neighbor);
                        queue.push_back(neighbor);
                    }
                }
            }
        }
        
        tree
    }
    
    /// Place tree nodes in radial pattern
    fn place_tree_nodes_radially(
        &self,
        tree: &HashMap<SceneId, Vec<SceneId>>,
        node_id: SceneId,
        positions: &mut HashMap<SceneId, Position>,
        level: usize,
        angle_range: f32,
        max_level: usize,
    ) {
        if let Some(children) = tree.get(&node_id) {
            if children.is_empty() {
                return;
            }
            
            let child_count = children.len();
            let angle_per_child = angle_range / child_count as f32;
            let radius = self.radius * (level as f32 + 1.0) / (max_level as f32 + 1.0);
            
            for (i, &child_id) in children.iter().enumerate() {
                let angle = self.start_angle + (i as f32 + 0.5) * angle_per_child;
                
                let x = self.center.x + radius * angle.cos();
                let y = self.center.y + radius * angle.sin();
                let z = self.center.z;
                
                let mut position = Position::new(x, y, z);
                utils::apply_bounds(&mut position, &self.bounds);
                positions.insert(child_id, position);
                
                // Recursively place grandchildren
                self.place_tree_nodes_radially(
                    tree,
                    child_id,
                    positions,
                    level + 1,
                    angle_per_child,
                    max_level,
                );
            }
        }
    }
}

impl LayoutAlgorithm for CircularLayout {
    fn calculate_layout(&self, nodes: &[LayoutNode], edges: &[LayoutEdge]) -> Result<LayoutResult, LayoutError> {
        if nodes.is_empty() {
            return Err(LayoutError::InsufficientNodes { count: 0 });
        }
        
        let start_time = chrono::Utc::now();
        
        let node_positions = match &self.variant {
            CircularVariant::SimpleCircle => {
                self.calculate_simple_circle(nodes)
            }
            CircularVariant::ConcentricCircles { rings } => {
                self.calculate_concentric_circles(nodes, edges, *rings)
            }
            CircularVariant::Spiral { spacing } => {
                self.calculate_spiral(nodes, *spacing)
            }
            CircularVariant::RadialTree { root_id } => {
                self.calculate_radial_tree(nodes, edges, *root_id)
            }
        };
        
        let processing_time = chrono::Utc::now() - start_time;
        
        log::info!(
            "Circular layout completed: {} nodes using {:?} variant",
            nodes.len(),
            self.variant
        );
        
        Ok(LayoutResult {
            node_positions,
            iterations_performed: 1, // Circular layout is deterministic
            energy: 0.0, // Not applicable for circular layout
            converged: true,
            processing_time,
        })
    }
    
    fn update_layout(&self, nodes: &mut [LayoutNode], edges: &[LayoutEdge], _delta_time: f32) -> Result<f32, LayoutError> {
        // Circular layout doesn't support incremental updates
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
        "Circular"
    }
    
    fn supports_incremental(&self) -> bool {
        false
    }
    
    fn recommended_settings(&self) -> LayoutType {
        LayoutType::Circular {
            radius: self.radius,
            center: self.center,
        }
    }
}

impl Default for CircularLayout {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LayoutNode, LayoutEdge, Position};
    
    #[test]
    fn test_circular_layout_creation() {
        let layout = CircularLayout::new()
            .with_radius(100.0)
            .with_center(Position::new(10.0, 20.0, 0.0))
            .with_start_angle(PI / 4.0);
        
        assert_eq!(layout.radius, 100.0);
        assert_eq!(layout.center.x, 10.0);
        assert_eq!(layout.center.y, 20.0);
        assert_eq!(layout.start_angle, PI / 4.0);
    }
    
    #[test]
    fn test_simple_circle_layout() {
        let layout = CircularLayout::new().with_radius(50.0);
        
        let nodes = vec![
            LayoutNode::new(1, Position::new(0.0, 0.0, 0.0)),
            LayoutNode::new(2, Position::new(0.0, 0.0, 0.0)),
            LayoutNode::new(3, Position::new(0.0, 0.0, 0.0)),
            LayoutNode::new(4, Position::new(0.0, 0.0, 0.0)),
        ];
        
        let result = layout.calculate_layout(&nodes, &[]).unwrap();
        
        assert_eq!(result.node_positions.len(), 4);
        assert!(result.converged);
        
        // Check that nodes are roughly on the circle
        for position in result.node_positions.values() {
            let distance = ((position.x * position.x) + (position.y * position.y)).sqrt();
            assert!((distance - 50.0).abs() < 0.1);
        }
    }
    
    #[test]
    fn test_concentric_circles_variant() {
        let layout = CircularLayout::new()
            .with_variant(CircularVariant::ConcentricCircles { rings: 2 });
        
        let nodes = vec![
            LayoutNode::new(1, Position::new(0.0, 0.0, 0.0)),
            LayoutNode::new(2, Position::new(0.0, 0.0, 0.0)),
            LayoutNode::new(3, Position::new(0.0, 0.0, 0.0)),
        ];
        
        let edges = vec![
            LayoutEdge::new(1, 2, 1.0),
            LayoutEdge::new(2, 3, 1.0),
        ];
        
        let result = layout.calculate_layout(&nodes, &edges).unwrap();
        assert_eq!(result.node_positions.len(), 3);
    }
    
    #[test]
    fn test_spiral_variant() {
        let layout = CircularLayout::new()
            .with_variant(CircularVariant::Spiral { spacing: 10.0 });
        
        let nodes = vec![
            LayoutNode::new(1, Position::new(0.0, 0.0, 0.0)),
            LayoutNode::new(2, Position::new(0.0, 0.0, 0.0)),
            LayoutNode::new(3, Position::new(0.0, 0.0, 0.0)),
        ];
        
        let result = layout.calculate_layout(&nodes, &[]).unwrap();
        
        assert_eq!(result.node_positions.len(), 3);
        
        // Nodes should be at increasing distances from center
        let distances: Vec<_> = result.node_positions.values()
            .map(|pos| ((pos.x * pos.x) + (pos.y * pos.y)).sqrt())
            .collect();
        
        assert!(distances[0] < distances[1]);
        assert!(distances[1] < distances[2]);
    }
    
    #[test]
    fn test_radial_tree_variant() {
        let layout = CircularLayout::new()
            .with_variant(CircularVariant::RadialTree { root_id: Some(1) });
        
        let nodes = vec![
            LayoutNode::new(1, Position::new(0.0, 0.0, 0.0)), // root
            LayoutNode::new(2, Position::new(0.0, 0.0, 0.0)),
            LayoutNode::new(3, Position::new(0.0, 0.0, 0.0)),
        ];
        
        let edges = vec![
            LayoutEdge::new(1, 2, 1.0),
            LayoutEdge::new(1, 3, 1.0),
        ];
        
        let result = layout.calculate_layout(&nodes, &edges).unwrap();
        
        assert_eq!(result.node_positions.len(), 3);
        
        // Root should be at center
        let root_pos = result.node_positions.get(&1).unwrap();
        assert!((root_pos.x).abs() < 0.1);
        assert!((root_pos.y).abs() < 0.1);
    }
    
    #[test]
    fn test_algorithm_properties() {
        let layout = CircularLayout::new();
        
        assert_eq!(layout.name(), "Circular");
        assert!(!layout.supports_incremental());
        
        let settings = layout.recommended_settings();
        assert!(matches!(settings, LayoutType::Circular { .. }));
    }
    
    #[test]
    fn test_empty_graph() {
        let layout = CircularLayout::new();
        let result = layout.calculate_layout(&[], &[]);
        
        assert!(matches!(result, Err(LayoutError::InsufficientNodes { count: 0 })));
    }
}