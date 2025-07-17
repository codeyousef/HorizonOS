//! Cluster management and operations

use crate::{Cluster, ClusterId, ClusterType};
use anyhow::{Result, anyhow};
use dashmap::DashMap;
use horizonos_graph_engine::SceneId;
use indexmap::IndexMap;
use std::collections::{HashMap, HashSet};

/// Manages all clusters in the system
pub struct ClusterManager {
    /// All clusters indexed by ID
    clusters: DashMap<ClusterId, Cluster>,
    /// Node to clusters mapping (nodes can be in multiple clusters)
    node_clusters: DashMap<SceneId, HashSet<ClusterId>>,
    /// Cluster hierarchy (parent -> children)
    cluster_hierarchy: DashMap<ClusterId, HashSet<ClusterId>>,
    /// Reverse hierarchy (child -> parent)
    cluster_parents: DashMap<ClusterId, ClusterId>,
}

impl ClusterManager {
    /// Create a new cluster manager
    pub fn new() -> Self {
        Self {
            clusters: DashMap::new(),
            node_clusters: DashMap::new(),
            cluster_hierarchy: DashMap::new(),
            cluster_parents: DashMap::new(),
        }
    }
    
    /// Add a new cluster
    pub fn add_cluster(&self, cluster: Cluster) -> ClusterId {
        let cluster_id = cluster.id;
        
        // Update node-to-clusters mapping
        for &node_id in &cluster.nodes {
            self.node_clusters
                .entry(node_id)
                .or_insert_with(HashSet::new)
                .insert(cluster_id);
        }
        
        self.clusters.insert(cluster_id, cluster);
        cluster_id
    }
    
    /// Remove a cluster
    pub fn remove_cluster(&self, cluster_id: ClusterId) -> Result<Cluster> {
        let cluster = self.clusters.remove(&cluster_id)
            .ok_or_else(|| anyhow!("Cluster not found: {}", cluster_id))?
            .1;
        
        // Update node-to-clusters mapping
        for &node_id in &cluster.nodes {
            if let Some(mut node_clusters) = self.node_clusters.get_mut(&node_id) {
                node_clusters.remove(&cluster_id);
                if node_clusters.is_empty() {
                    drop(node_clusters);
                    self.node_clusters.remove(&node_id);
                }
            }
        }
        
        // Remove from hierarchy
        self.remove_from_hierarchy(cluster_id);
        
        Ok(cluster)
    }
    
    /// Get a cluster by ID
    pub fn get_cluster(&self, cluster_id: ClusterId) -> Option<Cluster> {
        self.clusters.get(&cluster_id).map(|entry| entry.clone())
    }
    
    /// Get all clusters
    pub fn clusters(&self) -> Vec<Cluster> {
        self.clusters.iter().map(|entry| entry.value().clone()).collect()
    }
    
    /// Get clusters containing a specific node
    pub fn get_node_clusters(&self, node_id: SceneId) -> Vec<ClusterId> {
        self.node_clusters
            .get(&node_id)
            .map(|clusters| clusters.iter().copied().collect())
            .unwrap_or_default()
    }
    
    /// Add a node to an existing cluster
    pub fn add_node_to_cluster(&self, cluster_id: ClusterId, node_id: SceneId) -> Result<()> {
        let mut cluster = self.clusters.get_mut(&cluster_id)
            .ok_or_else(|| anyhow!("Cluster not found: {}", cluster_id))?;
        
        cluster.add_node(node_id);
        
        // Update node-to-clusters mapping
        self.node_clusters
            .entry(node_id)
            .or_insert_with(HashSet::new)
            .insert(cluster_id);
        
        Ok(())
    }
    
    /// Remove a node from a cluster
    pub fn remove_node_from_cluster(&self, cluster_id: ClusterId, node_id: SceneId) -> Result<()> {
        let mut cluster = self.clusters.get_mut(&cluster_id)
            .ok_or_else(|| anyhow!("Cluster not found: {}", cluster_id))?;
        
        if cluster.remove_node(node_id) {
            // Update node-to-clusters mapping
            if let Some(mut node_clusters) = self.node_clusters.get_mut(&node_id) {
                node_clusters.remove(&cluster_id);
                if node_clusters.is_empty() {
                    drop(node_clusters);
                    self.node_clusters.remove(&node_id);
                }
            }
            
            // If cluster is now empty, consider removing it
            if cluster.is_empty() {
                log::info!("Cluster {} is now empty after removing node {}", cluster_id, node_id);
            }
        }
        
        Ok(())
    }
    
    /// Merge two clusters
    pub fn merge_clusters(&self, cluster1_id: ClusterId, cluster2_id: ClusterId) -> Result<ClusterId> {
        if cluster1_id == cluster2_id {
            return Err(anyhow!("Cannot merge cluster with itself"));
        }
        
        // Get both clusters
        let cluster2 = self.remove_cluster(cluster2_id)?;
        
        {
            let mut cluster1 = self.clusters.get_mut(&cluster1_id)
                .ok_or_else(|| anyhow!("Cluster not found: {}", cluster1_id))?;
            
            cluster1.merge_with(&cluster2);
        }
        
        // Update node-to-clusters mapping for nodes from cluster2
        for &node_id in &cluster2.nodes {
            self.node_clusters
                .entry(node_id)
                .or_insert_with(HashSet::new)
                .insert(cluster1_id);
        }
        
        Ok(cluster1_id)
    }
    
    /// Split a cluster into multiple clusters based on a function
    pub fn split_cluster<F>(&self, cluster_id: ClusterId, split_fn: F) -> Result<Vec<ClusterId>>
    where
        F: Fn(SceneId) -> usize,
    {
        let cluster = self.remove_cluster(cluster_id)?;
        
        // Group nodes by split function result
        let mut groups: HashMap<usize, Vec<SceneId>> = HashMap::new();
        for &node_id in &cluster.nodes {
            let group = split_fn(node_id);
            groups.entry(group).or_default().push(node_id);
        }
        
        // Create new clusters for each group
        let mut new_cluster_ids = Vec::new();
        for (group_index, nodes) in groups {
            if nodes.len() > 0 {
                let new_cluster = Cluster::new_auto(
                    format!("{} - Part {}", cluster.name, group_index + 1),
                    nodes,
                    cluster.cluster_type,
                );
                let new_id = self.add_cluster(new_cluster);
                new_cluster_ids.push(new_id);
            }
        }
        
        Ok(new_cluster_ids)
    }
    
    /// Create a hierarchical relationship between clusters
    pub fn set_cluster_parent(&self, child_id: ClusterId, parent_id: ClusterId) -> Result<()> {
        if child_id == parent_id {
            return Err(anyhow!("Cluster cannot be its own parent"));
        }
        
        // Check if both clusters exist
        if !self.clusters.contains_key(&child_id) {
            return Err(anyhow!("Child cluster not found: {}", child_id));
        }
        if !self.clusters.contains_key(&parent_id) {
            return Err(anyhow!("Parent cluster not found: {}", parent_id));
        }
        
        // Check for circular dependency
        if self.would_create_cycle(child_id, parent_id) {
            return Err(anyhow!("Setting parent would create a circular dependency"));
        }
        
        // Remove from previous parent if any
        if let Some(old_parent) = self.cluster_parents.get(&child_id) {
            let old_parent_id = *old_parent;
            drop(old_parent);
            if let Some(mut children) = self.cluster_hierarchy.get_mut(&old_parent_id) {
                children.remove(&child_id);
            }
        }
        
        // Set new parent
        self.cluster_parents.insert(child_id, parent_id);
        self.cluster_hierarchy
            .entry(parent_id)
            .or_insert_with(HashSet::new)
            .insert(child_id);
        
        Ok(())
    }
    
    /// Remove cluster from hierarchy
    fn remove_from_hierarchy(&self, cluster_id: ClusterId) {
        // Remove as child
        if let Some(parent_id) = self.cluster_parents.remove(&cluster_id).map(|(_, v)| v) {
            if let Some(mut children) = self.cluster_hierarchy.get_mut(&parent_id) {
                children.remove(&cluster_id);
            }
        }
        
        // Remove as parent (move children to no parent)
        if let Some(children) = self.cluster_hierarchy.remove(&cluster_id).map(|(_, v)| v) {
            for child_id in children {
                self.cluster_parents.remove(&child_id);
            }
        }
    }
    
    /// Check if setting a parent would create a cycle
    fn would_create_cycle(&self, child_id: ClusterId, parent_id: ClusterId) -> bool {
        let mut current = parent_id;
        let mut visited = HashSet::new();
        
        loop {
            if current == child_id {
                return true; // Cycle detected
            }
            
            if !visited.insert(current) {
                return false; // Already visited, no cycle through child_id
            }
            
            if let Some(next_parent) = self.cluster_parents.get(&current) {
                current = *next_parent;
            } else {
                return false; // Reached root, no cycle
            }
        }
    }
    
    /// Get child clusters
    pub fn get_child_clusters(&self, parent_id: ClusterId) -> Vec<ClusterId> {
        self.cluster_hierarchy
            .get(&parent_id)
            .map(|children| children.iter().copied().collect())
            .unwrap_or_default()
    }
    
    /// Get parent cluster
    pub fn get_parent_cluster(&self, child_id: ClusterId) -> Option<ClusterId> {
        self.cluster_parents.get(&child_id).map(|entry| *entry)
    }
    
    /// Get all root clusters (clusters with no parent)
    pub fn get_root_clusters(&self) -> Vec<ClusterId> {
        self.clusters
            .iter()
            .filter_map(|entry| {
                let cluster_id = *entry.key();
                if self.cluster_parents.contains_key(&cluster_id) {
                    None
                } else {
                    Some(cluster_id)
                }
            })
            .collect()
    }
    
    /// Get cluster statistics
    pub fn get_statistics(&self) -> ClusterStatistics {
        let total_clusters = self.clusters.len();
        let mut clusters_by_type = HashMap::new();
        let mut total_nodes = 0;
        let mut nodes_in_multiple_clusters = 0;
        
        // Count clusters by type and total nodes
        for cluster in self.clusters.iter() {
            *clusters_by_type.entry(cluster.cluster_type).or_insert(0) += 1;
            total_nodes += cluster.node_count();
        }
        
        // Count nodes in multiple clusters
        for node_clusters in self.node_clusters.iter() {
            if node_clusters.len() > 1 {
                nodes_in_multiple_clusters += 1;
            }
        }
        
        ClusterStatistics {
            total_clusters,
            clusters_by_type,
            total_nodes_in_clusters: total_nodes,
            unique_nodes_in_clusters: self.node_clusters.len(),
            nodes_in_multiple_clusters,
        }
    }
    
    /// Clear all clusters
    pub fn clear(&self) {
        self.clusters.clear();
        self.node_clusters.clear();
        self.cluster_hierarchy.clear();
        self.cluster_parents.clear();
    }
}

/// Clustering statistics
#[derive(Debug, Clone)]
pub struct ClusterStatistics {
    /// Total number of clusters
    pub total_clusters: usize,
    /// Number of clusters by type
    pub clusters_by_type: HashMap<ClusterType, usize>,
    /// Total number of node memberships (can be > unique nodes due to multi-membership)
    pub total_nodes_in_clusters: usize,
    /// Number of unique nodes that are in at least one cluster
    pub unique_nodes_in_clusters: usize,
    /// Number of nodes that are in multiple clusters
    pub nodes_in_multiple_clusters: usize,
}

impl Default for ClusterManager {
    fn default() -> Self {
        Self::new()
    }
}