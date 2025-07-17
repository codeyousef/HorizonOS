//! Edge manager for the graph desktop

use crate::{GraphEdge, EdgeError, RelationshipData};
use horizonos_graph_engine::{SceneId, EdgeType, Scene};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

/// Manages all edges and relationships in the graph desktop
pub struct EdgeManager {
    edges: Arc<RwLock<HashMap<SceneId, GraphEdge>>>,
    adjacency_list: Arc<RwLock<HashMap<SceneId, HashSet<SceneId>>>>, // node_id -> connected_node_ids
    reverse_adjacency: Arc<RwLock<HashMap<SceneId, HashSet<SceneId>>>>, // incoming edges
    next_id: SceneId,
    max_edges_per_node: usize,
}

impl EdgeManager {
    pub fn new() -> Self {
        EdgeManager {
            edges: Arc::new(RwLock::new(HashMap::new())),
            adjacency_list: Arc::new(RwLock::new(HashMap::new())),
            reverse_adjacency: Arc::new(RwLock::new(HashMap::new())),
            next_id: 1,
            max_edges_per_node: 100, // Prevent excessive connections
        }
    }
    
    /// Generate next unique edge ID
    pub fn next_id(&mut self) -> SceneId {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
    
    /// Add a new edge
    pub fn add_edge(&mut self, source: SceneId, target: SceneId, edge_type: EdgeType) -> Result<SceneId, EdgeError> {
        // Check for circular dependencies in dependency chains
        if matches!(edge_type, EdgeType::DependsOn) && self.would_create_cycle(source, target)? {
            return Err(EdgeError::CircularDependency);
        }
        
        // Check edge limits
        if self.get_node_edge_count(source) >= self.max_edges_per_node {
            return Err(EdgeError::MaxEdgesExceeded { node_id: source });
        }
        
        let edge_id = self.next_id();
        let edge = GraphEdge::new(edge_id, source, target, edge_type);
        
        // Add to data structures
        {
            let mut edges = self.edges.write().unwrap();
            edges.insert(edge_id, edge);
        }
        
        {
            let mut adj_list = self.adjacency_list.write().unwrap();
            adj_list.entry(source).or_insert_with(HashSet::new).insert(target);
        }
        
        {
            let mut rev_adj = self.reverse_adjacency.write().unwrap();
            rev_adj.entry(target).or_insert_with(HashSet::new).insert(source);
        }
        
        log::info!("Added edge {} -> {} (type: {:?})", source, target, edge.edge_type);
        Ok(edge_id)
    }
    
    /// Remove an edge
    pub fn remove_edge(&mut self, edge_id: SceneId) -> Result<GraphEdge, EdgeError> {
        let edge = {
            let mut edges = self.edges.write().unwrap();
            edges.remove(&edge_id).ok_or(EdgeError::EdgeNotFound { id: edge_id })?
        };
        
        // Remove from adjacency lists
        {
            let mut adj_list = self.adjacency_list.write().unwrap();
            if let Some(targets) = adj_list.get_mut(&edge.source) {
                targets.remove(&edge.target);
                if targets.is_empty() {
                    adj_list.remove(&edge.source);
                }
            }
        }
        
        {
            let mut rev_adj = self.reverse_adjacency.write().unwrap();
            if let Some(sources) = rev_adj.get_mut(&edge.target) {
                sources.remove(&edge.source);
                if sources.is_empty() {
                    rev_adj.remove(&edge.target);
                }
            }
        }
        
        log::info!("Removed edge {} -> {}", edge.source, edge.target);
        Ok(edge)
    }
    
    /// Get an edge by ID
    pub fn get_edge(&self, edge_id: SceneId) -> Option<GraphEdge> {
        let edges = self.edges.read().unwrap();
        edges.get(&edge_id).cloned()
    }
    
    /// Get all edges for a node (outgoing)
    pub fn get_outgoing_edges(&self, node_id: SceneId) -> Vec<GraphEdge> {
        let adj_list = self.adjacency_list.read().unwrap();
        let edges = self.edges.read().unwrap();
        
        if let Some(targets) = adj_list.get(&node_id) {
            edges.values()
                .filter(|edge| edge.source == node_id && targets.contains(&edge.target))
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }
    
    /// Get all edges for a node (incoming)
    pub fn get_incoming_edges(&self, node_id: SceneId) -> Vec<GraphEdge> {
        let rev_adj = self.reverse_adjacency.read().unwrap();
        let edges = self.edges.read().unwrap();
        
        if let Some(sources) = rev_adj.get(&node_id) {
            edges.values()
                .filter(|edge| edge.target == node_id && sources.contains(&edge.source))
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }
    
    /// Get all edges connected to a node (both directions)
    pub fn get_all_edges(&self, node_id: SceneId) -> Vec<GraphEdge> {
        let mut result = self.get_outgoing_edges(node_id);
        result.extend(self.get_incoming_edges(node_id));
        result
    }
    
    /// Update edge strength
    pub fn update_edge_strength(&mut self, edge_id: SceneId, strength: f32) -> Result<(), EdgeError> {
        let mut edges = self.edges.write().unwrap();
        if let Some(edge) = edges.get_mut(&edge_id) {
            edge.update_strength(strength);
            Ok(())
        } else {
            Err(EdgeError::EdgeNotFound { id: edge_id })
        }
    }
    
    /// Record edge access
    pub fn record_edge_access(&mut self, edge_id: SceneId) -> Result<(), EdgeError> {
        let mut edges = self.edges.write().unwrap();
        if let Some(edge) = edges.get_mut(&edge_id) {
            edge.record_access();
            Ok(())
        } else {
            Err(EdgeError::EdgeNotFound { id: edge_id })
        }
    }
    
    /// Find edges by type
    pub fn find_edges_by_type(&self, edge_type: &EdgeType) -> Vec<GraphEdge> {
        let edges = self.edges.read().unwrap();
        edges.values()
            .filter(|edge| std::mem::discriminant(&edge.edge_type) == std::mem::discriminant(edge_type))
            .cloned()
            .collect()
    }
    
    /// Get edge count for a node
    pub fn get_node_edge_count(&self, node_id: SceneId) -> usize {
        let adj_list = self.adjacency_list.read().unwrap();
        let rev_adj = self.reverse_adjacency.read().unwrap();
        
        let outgoing = adj_list.get(&node_id).map(|s| s.len()).unwrap_or(0);
        let incoming = rev_adj.get(&node_id).map(|s| s.len()).unwrap_or(0);
        
        outgoing + incoming
    }
    
    /// Check if adding an edge would create a cycle (for dependency edges)
    fn would_create_cycle(&self, source: SceneId, target: SceneId) -> Result<bool, EdgeError> {
        // Use DFS to check if target can reach source
        let mut visited = HashSet::new();
        let mut stack = vec![target];
        
        let adj_list = self.adjacency_list.read().unwrap();
        
        while let Some(current) = stack.pop() {
            if current == source {
                return Ok(true); // Cycle detected
            }
            
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current);
            
            if let Some(neighbors) = adj_list.get(&current) {
                for &neighbor in neighbors {
                    if !visited.contains(&neighbor) {
                        stack.push(neighbor);
                    }
                }
            }
        }
        
        Ok(false)
    }
    
    /// Clean up expired edges
    pub fn cleanup_expired_edges(&mut self) -> usize {
        let mut expired_ids = Vec::new();
        
        {
            let edges = self.edges.read().unwrap();
            for (id, edge) in edges.iter() {
                if edge.should_expire() {
                    expired_ids.push(*id);
                }
            }
        }
        
        let count = expired_ids.len();
        for id in expired_ids {
            let _ = self.remove_edge(id);
        }
        
        if count > 0 {
            log::info!("Cleaned up {} expired edges", count);
        }
        
        count
    }
    
    /// Get statistics about the edge graph
    pub fn get_statistics(&self) -> EdgeStatistics {
        let edges = self.edges.read().unwrap();
        let adj_list = self.adjacency_list.read().unwrap();
        
        let total_edges = edges.len();
        let total_nodes = adj_list.len();
        
        let mut edge_type_counts = HashMap::new();
        let mut total_strength = 0.0;
        let mut strong_edges = 0;
        
        for edge in edges.values() {
            let type_name = format!("{:?}", edge.edge_type);
            *edge_type_counts.entry(type_name).or_insert(0) += 1;
            
            total_strength += edge.relationship_data.strength;
            if edge.relationship_data.strength > 0.8 {
                strong_edges += 1;
            }
        }
        
        let average_strength = if total_edges > 0 {
            total_strength / total_edges as f32
        } else {
            0.0
        };
        
        EdgeStatistics {
            total_edges,
            total_nodes,
            edge_type_counts,
            average_strength,
            strong_edges,
        }
    }
    
    /// Sync edges to scene for rendering
    pub fn sync_to_scene(&self, scene: &mut Scene) {
        let edges = self.edges.read().unwrap();
        for edge in edges.values() {
            if edge.visual_style.visible {
                let scene_edge = edge.to_scene_edge();
                scene.add_edge(scene_edge);
            }
        }
    }
}

/// Statistics about the edge graph
#[derive(Debug, Clone)]
pub struct EdgeStatistics {
    pub total_edges: usize,
    pub total_nodes: usize,
    pub edge_type_counts: HashMap<String, usize>,
    pub average_strength: f32,
    pub strong_edges: usize,
}

impl Default for EdgeManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edge_manager_creation() {
        let mut manager = EdgeManager::new();
        assert_eq!(manager.next_id(), 1);
        assert_eq!(manager.next_id(), 2);
    }

    #[test]
    fn test_add_edge() {
        let mut manager = EdgeManager::new();
        let edge_id = manager.add_edge(100, 200, EdgeType::Contains).unwrap();
        assert_eq!(edge_id, 1);
        
        let edge = manager.get_edge(edge_id).unwrap();
        assert_eq!(edge.source, 100);
        assert_eq!(edge.target, 200);
    }

    #[test]
    fn test_circular_dependency_detection() {
        let mut manager = EdgeManager::new();
        
        // Add A -> B
        manager.add_edge(1, 2, EdgeType::DependsOn).unwrap();
        // Add B -> C
        manager.add_edge(2, 3, EdgeType::DependsOn).unwrap();
        
        // Try to add C -> A (should create a cycle)
        let result = manager.add_edge(3, 1, EdgeType::DependsOn);
        assert!(matches!(result, Err(EdgeError::CircularDependency)));
    }

    #[test]
    fn test_edge_statistics() {
        let mut manager = EdgeManager::new();
        manager.add_edge(1, 2, EdgeType::Contains).unwrap();
        manager.add_edge(2, 3, EdgeType::DependsOn).unwrap();
        
        let stats = manager.get_statistics();
        assert_eq!(stats.total_edges, 2);
        assert!(stats.edge_type_counts.contains_key("Contains"));
        assert!(stats.edge_type_counts.contains_key("DependsOn"));
    }
}