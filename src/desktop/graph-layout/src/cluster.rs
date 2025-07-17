//! Cluster layout algorithm implementation
//! 
//! This module implements clustering-based layouts that group related nodes together.

use crate::{
    LayoutAlgorithm, LayoutNode, LayoutEdge, LayoutResult, LayoutError, LayoutType, 
    LayoutBounds, utils, ForceDirectedLayout
};
use horizonos_graph_engine::{SceneId, Position};
use std::collections::{HashMap, HashSet};
use nalgebra::Vector3;
use rand::Rng;

/// Cluster layout algorithm
pub struct ClusterLayout {
    pub cluster_separation: f32,
    pub cluster_compactness: f32,
    pub bounds: LayoutBounds,
    pub clustering_method: ClusteringMethod,
    pub inner_layout: Box<dyn LayoutAlgorithm>,
}

/// Different clustering methods
#[derive(Debug, Clone)]
pub enum ClusteringMethod {
    ConnectedComponents,
    ModularityClustering,
    AttributeBased,
    KMeans { k: usize },
}

/// Cluster information
#[derive(Debug, Clone)]
struct Cluster {
    id: String,
    nodes: Vec<SceneId>,
    center: Position,
    radius: f32,
}

impl ClusterLayout {
    pub fn new() -> Self {
        ClusterLayout {
            cluster_separation: 100.0,
            cluster_compactness: 0.8,
            bounds: LayoutBounds::default(),
            clustering_method: ClusteringMethod::ConnectedComponents,
            inner_layout: Box::new(ForceDirectedLayout::new()),
        }
    }
    
    pub fn with_separation(mut self, separation: f32) -> Self {
        self.cluster_separation = separation;
        self
    }
    
    pub fn with_compactness(mut self, compactness: f32) -> Self {
        self.cluster_compactness = compactness;
        self
    }
    
    pub fn with_clustering_method(mut self, method: ClusteringMethod) -> Self {
        self.clustering_method = method;
        self
    }
    
    pub fn with_inner_layout(mut self, layout: Box<dyn LayoutAlgorithm>) -> Self {
        self.inner_layout = layout;
        self
    }
    
    /// Find clusters using the specified method
    fn find_clusters(&self, nodes: &[LayoutNode], edges: &[LayoutEdge]) -> Vec<Cluster> {
        match &self.clustering_method {
            ClusteringMethod::ConnectedComponents => {
                self.find_connected_components(nodes, edges)
            }
            ClusteringMethod::ModularityClustering => {
                self.find_modularity_clusters(nodes, edges)
            }
            ClusteringMethod::AttributeBased => {
                self.find_attribute_clusters(nodes)
            }
            ClusteringMethod::KMeans { k } => {
                self.find_kmeans_clusters(nodes, *k)
            }
        }
    }
    
    /// Find connected components as clusters
    fn find_connected_components(&self, nodes: &[LayoutNode], edges: &[LayoutEdge]) -> Vec<Cluster> {
        let mut adjacency: HashMap<SceneId, Vec<SceneId>> = HashMap::new();
        let mut visited = HashSet::new();
        let mut clusters = Vec::new();
        
        // Build adjacency list
        for node in nodes {
            adjacency.insert(node.id, Vec::new());
        }
        
        for edge in edges {
            adjacency.entry(edge.source).or_insert_with(Vec::new).push(edge.target);
            adjacency.entry(edge.target).or_insert_with(Vec::new).push(edge.source);
        }
        
        // Find connected components using DFS
        for node in nodes {
            if !visited.contains(&node.id) {
                let mut component = Vec::new();
                self.dfs_component(&adjacency, node.id, &mut visited, &mut component);
                
                if !component.is_empty() {
                    clusters.push(Cluster {
                        id: format!("component_{}", clusters.len()),
                        nodes: component,
                        center: Position::new(0.0, 0.0, 0.0),
                        radius: 50.0,
                    });
                }
            }
        }
        
        clusters
    }
    
    /// DFS helper for connected components
    fn dfs_component(&self, adjacency: &HashMap<SceneId, Vec<SceneId>>, node_id: SceneId, visited: &mut HashSet<SceneId>, component: &mut Vec<SceneId>) {
        if visited.contains(&node_id) {
            return;
        }
        
        visited.insert(node_id);
        component.push(node_id);
        
        if let Some(neighbors) = adjacency.get(&node_id) {
            for &neighbor in neighbors {
                self.dfs_component(adjacency, neighbor, visited, component);
            }
        }
    }
    
    /// Find clusters using modularity optimization (simplified Louvain)
    fn find_modularity_clusters(&self, nodes: &[LayoutNode], edges: &[LayoutEdge]) -> Vec<Cluster> {
        // Simplified modularity clustering - in practice would use Louvain algorithm
        let mut clusters = HashMap::new();
        let mut cluster_counter = 0;
        
        // Build adjacency list with weights
        let mut adjacency: HashMap<SceneId, HashMap<SceneId, f32>> = HashMap::new();
        let mut node_degrees: HashMap<SceneId, f32> = HashMap::new();
        
        for node in nodes {
            adjacency.insert(node.id, HashMap::new());
            node_degrees.insert(node.id, 0.0);
        }
        
        for edge in edges {
            let weight = edge.weight;
            adjacency.get_mut(&edge.source).unwrap().insert(edge.target, weight);
            adjacency.get_mut(&edge.target).unwrap().insert(edge.source, weight);
            *node_degrees.get_mut(&edge.source).unwrap() += weight;
            *node_degrees.get_mut(&edge.target).unwrap() += weight;
        }
        
        // Simple greedy clustering based on edge weights
        let mut assigned = HashSet::new();
        
        for node in nodes {
            if !assigned.contains(&node.id) {
                let mut cluster_nodes = vec![node.id];
                assigned.insert(node.id);
                
                // Add strongly connected neighbors
                if let Some(neighbors) = adjacency.get(&node.id) {
                    for (&neighbor_id, &weight) in neighbors {
                        if !assigned.contains(&neighbor_id) && weight > 0.5 {
                            cluster_nodes.push(neighbor_id);
                            assigned.insert(neighbor_id);
                        }
                    }
                }
                
                clusters.insert(cluster_counter, cluster_nodes);
                cluster_counter += 1;
            }
        }
        
        // Convert to Cluster structs
        clusters.into_iter().map(|(id, nodes)| Cluster {
            id: format!("modularity_{}", id),
            nodes,
            center: Position::new(0.0, 0.0, 0.0),
            radius: 50.0,
        }).collect()
    }
    
    /// Find clusters based on node attributes (cluster_id)
    fn find_attribute_clusters(&self, nodes: &[LayoutNode]) -> Vec<Cluster> {
        let mut attribute_clusters: HashMap<String, Vec<SceneId>> = HashMap::new();
        
        for node in nodes {
            let cluster_id = node.cluster_id.clone().unwrap_or_else(|| "default".to_string());
            attribute_clusters.entry(cluster_id).or_insert_with(Vec::new).push(node.id);
        }
        
        attribute_clusters.into_iter().map(|(id, nodes)| Cluster {
            id,
            nodes,
            center: Position::new(0.0, 0.0, 0.0),
            radius: 50.0,
        }).collect()
    }
    
    /// Find clusters using K-means algorithm
    fn find_kmeans_clusters(&self, nodes: &[LayoutNode], k: usize) -> Vec<Cluster> {
        if nodes.len() < k {
            // If fewer nodes than clusters, put each node in its own cluster
            return nodes.iter().enumerate().map(|(i, node)| Cluster {
                id: format!("kmeans_{}", i),
                nodes: vec![node.id],
                center: node.position,
                radius: 20.0,
            }).collect();
        }
        
        let mut rng = rand::thread_rng();
        
        // Initialize centroids randomly
        let mut centroids: Vec<Position> = Vec::new();
        for _ in 0..k {
            centroids.push(utils::random_position(&self.bounds));
        }
        
        // K-means iteration
        for _ in 0..20 { // Max iterations
            let mut clusters: Vec<Vec<SceneId>> = vec![Vec::new(); k];
            
            // Assign nodes to closest centroid
            for node in nodes {
                let mut min_distance = f32::INFINITY;
                let mut closest_cluster = 0;
                
                for (i, centroid) in centroids.iter().enumerate() {
                    let distance = utils::distance(&node.position, centroid);
                    if distance < min_distance {
                        min_distance = distance;
                        closest_cluster = i;
                    }
                }
                
                clusters[closest_cluster].push(node.id);
            }
            
            // Update centroids
            let mut new_centroids = Vec::new();
            for cluster_nodes in &clusters {
                if cluster_nodes.is_empty() {
                    new_centroids.push(utils::random_position(&self.bounds));
                } else {
                    let mut sum_x = 0.0;
                    let mut sum_y = 0.0;
                    let mut sum_z = 0.0;
                    
                    for &node_id in cluster_nodes {
                        if let Some(node) = nodes.iter().find(|n| n.id == node_id) {
                            sum_x += node.position.x;
                            sum_y += node.position.y;
                            sum_z += node.position.z;
                        }
                    }
                    
                    let count = cluster_nodes.len() as f32;
                    new_centroids.push(Position::new(sum_x / count, sum_y / count, sum_z / count));
                }
            }
            
            centroids = new_centroids;
        }
        
        // Create final clusters
        let mut final_clusters: Vec<Vec<SceneId>> = vec![Vec::new(); k];
        
        for node in nodes {
            let mut min_distance = f32::INFINITY;
            let mut closest_cluster = 0;
            
            for (i, centroid) in centroids.iter().enumerate() {
                let distance = utils::distance(&node.position, centroid);
                if distance < min_distance {
                    min_distance = distance;
                    closest_cluster = i;
                }
            }
            
            final_clusters[closest_cluster].push(node.id);
        }
        
        final_clusters.into_iter().enumerate()
            .filter(|(_, nodes)| !nodes.is_empty())
            .map(|(i, nodes)| Cluster {
                id: format!("kmeans_{}", i),
                nodes,
                center: centroids[i],
                radius: 50.0,
            }).collect()
    }
    
    /// Position clusters in space
    fn position_clusters(&self, clusters: &mut [Cluster]) {
        let cluster_count = clusters.len();
        
        if cluster_count == 0 {
            return;
        }
        
        if cluster_count == 1 {
            clusters[0].center = Position::new(0.0, 0.0, 0.0);
            return;
        }
        
        // Arrange clusters in a circle
        let radius = self.cluster_separation * cluster_count as f32 / (2.0 * std::f32::consts::PI);
        let angle_increment = 2.0 * std::f32::consts::PI / cluster_count as f32;
        
        for (i, cluster) in clusters.iter_mut().enumerate() {
            let angle = i as f32 * angle_increment;
            cluster.center = Position::new(
                radius * angle.cos(),
                radius * angle.sin(),
                0.0,
            );
            
            // Calculate cluster radius based on node count
            cluster.radius = (cluster.nodes.len() as f32).sqrt() * 20.0 * self.cluster_compactness;
        }
    }
    
    /// Layout nodes within each cluster
    fn layout_cluster_nodes(&self, cluster: &Cluster, all_nodes: &[LayoutNode], all_edges: &[LayoutEdge]) -> HashMap<SceneId, Position> {
        let cluster_nodes: Vec<_> = all_nodes.iter()
            .filter(|node| cluster.nodes.contains(&node.id))
            .cloned()
            .collect();
        
        let cluster_edges: Vec<_> = all_edges.iter()
            .filter(|edge| cluster.nodes.contains(&edge.source) && cluster.nodes.contains(&edge.target))
            .cloned()
            .collect();
        
        if cluster_nodes.is_empty() {
            return HashMap::new();
        }
        
        // Use inner layout algorithm for cluster
        let inner_result = self.inner_layout.calculate_layout(&cluster_nodes, &cluster_edges);
        
        match inner_result {
            Ok(result) => {
                // Scale and translate positions to fit within cluster bounds
                let mut positions = HashMap::new();
                
                for (node_id, position) in result.node_positions {
                    let scaled_position = Position::new(
                        cluster.center.x + position.x * self.cluster_compactness,
                        cluster.center.y + position.y * self.cluster_compactness,
                        cluster.center.z + position.z * self.cluster_compactness,
                    );
                    positions.insert(node_id, scaled_position);
                }
                
                positions
            }
            Err(_) => {
                // Fallback: arrange nodes in a small circle
                let mut positions = HashMap::new();
                let node_count = cluster_nodes.len();
                
                if node_count == 1 {
                    positions.insert(cluster_nodes[0].id, cluster.center);
                } else {
                    let angle_increment = 2.0 * std::f32::consts::PI / node_count as f32;
                    
                    for (i, node) in cluster_nodes.iter().enumerate() {
                        let angle = i as f32 * angle_increment;
                        let x = cluster.center.x + cluster.radius * 0.5 * angle.cos();
                        let y = cluster.center.y + cluster.radius * 0.5 * angle.sin();
                        
                        positions.insert(node.id, Position::new(x, y, cluster.center.z));
                    }
                }
                
                positions
            }
        }
    }
}

impl LayoutAlgorithm for ClusterLayout {
    fn calculate_layout(&self, nodes: &[LayoutNode], edges: &[LayoutEdge]) -> Result<LayoutResult, LayoutError> {
        if nodes.is_empty() {
            return Err(LayoutError::InsufficientNodes { count: 0 });
        }
        
        let start_time = chrono::Utc::now();
        
        // Find clusters
        let mut clusters = self.find_clusters(nodes, edges);
        
        if clusters.is_empty() {
            return Err(LayoutError::CalculationFailed { 
                reason: "No clusters found".to_string() 
            });
        }
        
        // Position clusters
        self.position_clusters(&mut clusters);
        
        // Layout nodes within each cluster
        let mut node_positions = HashMap::new();
        
        for cluster in &clusters {
            let cluster_positions = self.layout_cluster_nodes(cluster, nodes, edges);
            node_positions.extend(cluster_positions);
        }
        
        // Apply bounds to all positions
        for position in node_positions.values_mut() {
            utils::apply_bounds(position, &self.bounds);
        }
        
        let processing_time = chrono::Utc::now() - start_time;
        
        log::info!(
            "Cluster layout completed: {} nodes in {} clusters",
            nodes.len(),
            clusters.len()
        );
        
        Ok(LayoutResult {
            node_positions,
            iterations_performed: 1,
            energy: 0.0,
            converged: true,
            processing_time,
        })
    }
    
    fn update_layout(&self, nodes: &mut [LayoutNode], edges: &[LayoutEdge], _delta_time: f32) -> Result<f32, LayoutError> {
        let result = self.calculate_layout(nodes, edges)?;
        
        for node in nodes.iter_mut() {
            if let Some(position) = result.node_positions.get(&node.id) {
                node.position = *position;
            }
        }
        
        Ok(0.0)
    }
    
    fn name(&self) -> &str {
        "Cluster"
    }
    
    fn supports_incremental(&self) -> bool {
        false
    }
    
    fn recommended_settings(&self) -> LayoutType {
        LayoutType::Cluster {
            cluster_separation: self.cluster_separation,
            cluster_compactness: self.cluster_compactness,
        }
    }
}

impl Default for ClusterLayout {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LayoutNode, LayoutEdge, Position};
    
    #[test]
    fn test_cluster_layout_creation() {
        let layout = ClusterLayout::new()
            .with_separation(150.0)
            .with_compactness(0.6);
        
        assert_eq!(layout.cluster_separation, 150.0);
        assert_eq!(layout.cluster_compactness, 0.6);
    }
    
    #[test]
    fn test_connected_components_clustering() {
        let layout = ClusterLayout::new();
        
        let nodes = vec![
            LayoutNode::new(1, Position::new(0.0, 0.0, 0.0)),
            LayoutNode::new(2, Position::new(0.0, 0.0, 0.0)),
            LayoutNode::new(3, Position::new(0.0, 0.0, 0.0)),
            LayoutNode::new(4, Position::new(0.0, 0.0, 0.0)),
        ];
        
        // Two separate components
        let edges = vec![
            LayoutEdge::new(1, 2, 1.0),
            LayoutEdge::new(3, 4, 1.0),
        ];
        
        let result = layout.calculate_layout(&nodes, &edges).unwrap();
        assert_eq!(result.node_positions.len(), 4);
    }
    
    #[test]
    fn test_attribute_based_clustering() {
        let layout = ClusterLayout::new()
            .with_clustering_method(ClusteringMethod::AttributeBased);
        
        let nodes = vec![
            LayoutNode::new(1, Position::new(0.0, 0.0, 0.0)).with_cluster("A".to_string()),
            LayoutNode::new(2, Position::new(0.0, 0.0, 0.0)).with_cluster("A".to_string()),
            LayoutNode::new(3, Position::new(0.0, 0.0, 0.0)).with_cluster("B".to_string()),
        ];
        
        let clusters = layout.find_clusters(&nodes, &[]);
        assert_eq!(clusters.len(), 2);
    }
    
    #[test]
    fn test_kmeans_clustering() {
        let layout = ClusterLayout::new()
            .with_clustering_method(ClusteringMethod::KMeans { k: 2 });
        
        let nodes = vec![
            LayoutNode::new(1, Position::new(0.0, 0.0, 0.0)),
            LayoutNode::new(2, Position::new(1.0, 0.0, 0.0)),
            LayoutNode::new(3, Position::new(10.0, 0.0, 0.0)),
            LayoutNode::new(4, Position::new(11.0, 0.0, 0.0)),
        ];
        
        let clusters = layout.find_clusters(&nodes, &[]);
        assert_eq!(clusters.len(), 2);
    }
    
    #[test]
    fn test_algorithm_properties() {
        let layout = ClusterLayout::new();
        
        assert_eq!(layout.name(), "Cluster");
        assert!(!layout.supports_incremental());
        
        let settings = layout.recommended_settings();
        assert!(matches!(settings, LayoutType::Cluster { .. }));
    }
}