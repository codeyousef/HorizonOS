//! Clustering algorithms for graph desktop

use crate::{ClusterType};
use anyhow::Result;
use horizonos_graph_engine::{SceneId, Scene};
use horizonos_graph_nodes::NodeType;
use nalgebra::Point3;
use petgraph::graph::{UnGraph, NodeIndex};
use petgraph::algo::connected_components;
use std::collections::{HashMap, HashSet};

/// Clustering algorithms implementation
pub struct ClusteringAlgorithms {
    /// Minimum cluster size
    min_cluster_size: usize,
}

impl ClusteringAlgorithms {
    /// Create new clustering algorithms instance
    pub fn new() -> Self {
        Self {
            min_cluster_size: 2,
        }
    }
    
    /// Find connected components in the graph
    pub fn connected_components(&self, scene: &Scene) -> Result<Vec<Vec<SceneId>>> {
        let mut graph = UnGraph::new_undirected();
        let mut node_map = HashMap::new();
        
        // Add nodes to petgraph
        for node_id in scene.get_all_nodes() {
            let index = graph.add_node(node_id);
            node_map.insert(node_id, index);
        }
        
        // Add edges to petgraph
        for edge in scene.get_all_edges() {
            if let (Some(&from_idx), Some(&to_idx)) = (
                node_map.get(&edge.source),
                node_map.get(&edge.target)
            ) {
                graph.add_edge(from_idx, to_idx, ());
            }
        }
        
        // Find connected components
        let num_components = connected_components(&graph);
        let mut clusters = vec![Vec::new(); num_components];
        
        // Use a visitor to collect components
        let mut component_map = HashMap::new();
        let mut current_component = 0;
        
        for node_idx in graph.node_indices() {
            if !component_map.contains_key(&node_idx) {
                // Start a new DFS from this node
                let mut stack = vec![node_idx];
                let mut visited = HashSet::new();
                
                while let Some(current_idx) = stack.pop() {
                    if visited.contains(&current_idx) {
                        continue;
                    }
                    visited.insert(current_idx);
                    component_map.insert(current_idx, current_component);
                    
                    // Add neighbors to stack
                    for neighbor_idx in graph.neighbors(current_idx) {
                        if !visited.contains(&neighbor_idx) {
                            stack.push(neighbor_idx);
                        }
                    }
                }
                current_component += 1;
            }
        }
        
        // Group nodes by component
        for node_idx in graph.node_indices() {
            if let (Some(&node_id), Some(&component)) = (graph.node_weight(node_idx), component_map.get(&node_idx)) {
                if component < clusters.len() {
                    clusters[component].push(node_id);
                }
            }
        }
        
        // Filter out small clusters
        Ok(clusters.into_iter()
            .filter(|cluster| cluster.len() >= self.min_cluster_size)
            .collect())
    }
    
    /// Proximity-based clustering using spatial distance
    pub fn proximity_clustering(&self, scene: &Scene, max_distance: f32) -> Result<Vec<Vec<SceneId>>> {
        let mut clusters = Vec::new();
        let mut visited = HashSet::new();
        
        let nodes = scene.get_all_nodes();
        
        for &node_id in &nodes {
            if visited.contains(&node_id) {
                continue;
            }
            
            let mut cluster = Vec::new();
            let mut stack = vec![node_id];
            
            while let Some(current_id) = stack.pop() {
                if visited.contains(&current_id) {
                    continue;
                }
                
                visited.insert(current_id);
                cluster.push(current_id);
                
                // Find nearby nodes
                if let Some(current_pos) = scene.get_node_position(current_id) {
                    for &other_id in &nodes {
                        if visited.contains(&other_id) {
                            continue;
                        }
                        
                        if let Some(other_pos) = scene.get_node_position(other_id) {
                            let distance = (current_pos - other_pos).magnitude();
                            if distance <= max_distance {
                                stack.push(other_id);
                            }
                        }
                    }
                }
            }
            
            if cluster.len() >= self.min_cluster_size {
                clusters.push(cluster);
            }
        }
        
        Ok(clusters)
    }
    
    /// Semantic clustering based on node types and properties
    pub fn semantic_clustering(&self, scene: &Scene) -> Result<HashMap<String, Vec<SceneId>>> {
        let mut clusters = HashMap::new();
        
        for node_id in scene.get_all_nodes() {
            if let Some(node) = scene.get_node(node_id) {
                let cluster_key = match &node.node_type {
                    horizonos_graph_engine::NodeType::Application { name, .. } => {
                        format!("Application: {}", name)
                    },
                    horizonos_graph_engine::NodeType::File { path, .. } => {
                        if let Some(ext) = std::path::Path::new(path)
                            .extension()
                            .and_then(|e| e.to_str()) {
                            format!("{} Files", ext.to_uppercase())
                        } else {
                            "Files".to_string()
                        }
                    },
                    horizonos_graph_engine::NodeType::Person { .. } => "People".to_string(),
                    horizonos_graph_engine::NodeType::Task { .. } => "Tasks".to_string(),
                    horizonos_graph_engine::NodeType::Device { .. } => "Devices".to_string(),
                    horizonos_graph_engine::NodeType::AIAgent { .. } => "AI Agents".to_string(),
                    horizonos_graph_engine::NodeType::Concept { .. } => "Concepts".to_string(),
                    horizonos_graph_engine::NodeType::System { .. } => "System Components".to_string(),
                    horizonos_graph_engine::NodeType::URL { .. } => "URLs".to_string(),
                    horizonos_graph_engine::NodeType::Automation { .. } => "Automations".to_string(),
                    horizonos_graph_engine::NodeType::Setting { .. } => "Settings".to_string(),
                    horizonos_graph_engine::NodeType::ConfigGroup { .. } => "Configuration".to_string(),
                };
                
                clusters.entry(cluster_key).or_insert_with(Vec::new).push(node_id);
            }
        }
        
        // Filter out small clusters
        clusters.retain(|_, nodes| nodes.len() >= self.min_cluster_size);
        
        Ok(clusters)
    }
    
    /// Temporal clustering based on creation/modification times
    pub fn temporal_clustering(&self, scene: &Scene, time_window_hours: i64) -> Result<Vec<Vec<SceneId>>> {
        let mut time_groups = HashMap::new();
        
        for node_id in scene.get_all_nodes() {
            if let Some(node) = scene.get_node(node_id) {
                // Use creation time from metadata
                let timestamp = node.metadata.created_at;
                
                // Group by time windows
                let window_start = timestamp.timestamp() / (time_window_hours * 3600);
                time_groups.entry(window_start).or_insert_with(Vec::new).push(node_id);
            }
        }
        
        // Convert to clusters
        let clusters: Vec<Vec<SceneId>> = time_groups
            .into_values()
            .filter(|cluster| cluster.len() >= self.min_cluster_size)
            .collect();
        
        Ok(clusters)
    }
    
    /// DBSCAN clustering algorithm
    pub fn dbscan_clustering(&self, scene: &Scene, eps: f32, min_points: usize) -> Result<Vec<Vec<SceneId>>> {
        let nodes: Vec<SceneId> = scene.get_all_nodes();
        let mut clusters = Vec::new();
        let mut visited = HashSet::new();
        let mut noise = HashSet::new();
        
        for &node_id in &nodes {
            if visited.contains(&node_id) {
                continue;
            }
            
            visited.insert(node_id);
            let neighbors = self.get_neighbors(scene, node_id, eps)?;
            
            if neighbors.len() < min_points {
                noise.insert(node_id);
            } else {
                let mut cluster = Vec::new();
                self.expand_cluster(scene, node_id, &neighbors, &mut cluster, &mut visited, eps, min_points)?;
                if cluster.len() >= self.min_cluster_size {
                    clusters.push(cluster);
                }
            }
        }
        
        Ok(clusters)
    }
    
    /// Get neighbors within eps distance
    fn get_neighbors(&self, scene: &Scene, node_id: SceneId, eps: f32) -> Result<Vec<SceneId>> {
        let mut neighbors = Vec::new();
        
        if let Some(node_pos) = scene.get_node_position(node_id) {
            for other_id in scene.get_all_nodes() {
                if other_id == node_id {
                    continue;
                }
                
                if let Some(other_pos) = scene.get_node_position(other_id) {
                    let distance = (node_pos - other_pos).magnitude();
                    if distance <= eps {
                        neighbors.push(other_id);
                    }
                }
            }
        }
        
        Ok(neighbors)
    }
    
    /// Expand cluster for DBSCAN
    fn expand_cluster(
        &self,
        scene: &Scene,
        node_id: SceneId,
        neighbors: &[SceneId],
        cluster: &mut Vec<SceneId>,
        visited: &mut HashSet<SceneId>,
        eps: f32,
        min_points: usize,
    ) -> Result<()> {
        cluster.push(node_id);
        
        let mut neighbor_queue = neighbors.to_vec();
        let mut i = 0;
        
        while i < neighbor_queue.len() {
            let neighbor_id = neighbor_queue[i];
            
            if !visited.contains(&neighbor_id) {
                visited.insert(neighbor_id);
                let neighbor_neighbors = self.get_neighbors(scene, neighbor_id, eps)?;
                
                if neighbor_neighbors.len() >= min_points {
                    for &nn in &neighbor_neighbors {
                        if !neighbor_queue.contains(&nn) {
                            neighbor_queue.push(nn);
                        }
                    }
                }
            }
            
            if !cluster.contains(&neighbor_id) {
                cluster.push(neighbor_id);
            }
            
            i += 1;
        }
        
        Ok(())
    }
    
    /// Set minimum cluster size
    pub fn set_min_cluster_size(&mut self, size: usize) {
        self.min_cluster_size = size;
    }
    
    /// Get minimum cluster size
    pub fn min_cluster_size(&self) -> usize {
        self.min_cluster_size
    }
}

impl Default for ClusteringAlgorithms {
    fn default() -> Self {
        Self::new()
    }
}