//! Clustering system for HorizonOS graph desktop
//! 
//! This module provides:
//! - Automatic clustering algorithms
//! - Manual cluster management
//! - Cluster boundary visualization
//! - Multi-membership support
//! - Smart clustering suggestions

pub mod algorithms;
pub mod cluster;
pub mod manager;
pub mod boundaries;
pub mod suggestions;

pub use algorithms::*;
pub use cluster::*;
pub use manager::*;
pub use boundaries::*;
pub use suggestions::*;

use anyhow::Result;
use horizonos_graph_engine::{SceneId, Scene};
use nalgebra::Point3;
use std::collections::{HashMap, HashSet};

/// Main clustering system
pub struct ClusteringSystem {
    /// Cluster manager
    manager: ClusterManager,
    /// Clustering algorithms
    algorithms: ClusteringAlgorithms,
    /// Boundary renderer
    boundaries: BoundaryRenderer,
    /// Suggestion engine
    suggestions: SuggestionEngine,
}

impl ClusteringSystem {
    /// Create a new clustering system
    pub fn new() -> Result<Self> {
        Ok(Self {
            manager: ClusterManager::new(),
            algorithms: ClusteringAlgorithms::new(),
            boundaries: BoundaryRenderer::new(),
            suggestions: SuggestionEngine::new(),
        })
    }
    
    /// Get cluster manager
    pub fn manager(&self) -> &ClusterManager {
        &self.manager
    }
    
    /// Get mutable cluster manager
    pub fn manager_mut(&mut self) -> &mut ClusterManager {
        &mut self.manager
    }
    
    /// Get clustering algorithms
    pub fn algorithms(&self) -> &ClusteringAlgorithms {
        &self.algorithms
    }
    
    /// Get boundary renderer
    pub fn boundaries(&self) -> &BoundaryRenderer {
        &self.boundaries
    }
    
    /// Get suggestion engine
    pub fn suggestions(&self) -> &SuggestionEngine {
        &self.suggestions
    }
    
    /// Automatically cluster nodes based on various criteria
    pub fn auto_cluster(&mut self, scene: &Scene) -> Result<Vec<ClusterId>> {
        let mut new_clusters = Vec::new();
        
        // Run connected components clustering
        if let Ok(clusters) = self.algorithms.connected_components(scene) {
            for cluster_nodes in clusters {
                if cluster_nodes.len() > 1 {
                    let cluster = Cluster::new_auto(
                        "Connected Component".to_string(),
                        cluster_nodes,
                        ClusterType::Connected,
                    );
                    let id = self.manager.add_cluster(cluster);
                    new_clusters.push(id);
                }
            }
        }
        
        // Run proximity clustering
        if let Ok(clusters) = self.algorithms.proximity_clustering(scene, 50.0) {
            for cluster_nodes in clusters {
                if cluster_nodes.len() > 1 {
                    let cluster = Cluster::new_auto(
                        "Proximity Group".to_string(),
                        cluster_nodes,
                        ClusterType::Proximity,
                    );
                    let id = self.manager.add_cluster(cluster);
                    new_clusters.push(id);
                }
            }
        }
        
        // Run semantic clustering based on node types
        if let Ok(clusters) = self.algorithms.semantic_clustering(scene) {
            for (cluster_type, cluster_nodes) in clusters {
                if cluster_nodes.len() > 1 {
                    let cluster = Cluster::new_auto(
                        format!("{} Group", cluster_type),
                        cluster_nodes,
                        ClusterType::Semantic,
                    );
                    let id = self.manager.add_cluster(cluster);
                    new_clusters.push(id);
                }
            }
        }
        
        Ok(new_clusters)
    }
    
    /// Update cluster boundaries
    pub fn update_boundaries(&mut self, scene: &Scene) {
        for cluster in self.manager.clusters() {
            if let Ok(boundary) = self.boundaries.compute_boundary(&cluster, scene) {
                self.boundaries.update_cluster_boundary(cluster.id, boundary);
            }
        }
    }
    
    /// Get clustering suggestions for a node
    pub fn get_suggestions(&self, node_id: SceneId, scene: &Scene) -> Vec<ClusterSuggestion> {
        self.suggestions.suggest_for_node(node_id, scene, &self.manager)
    }
    
    /// Apply a clustering suggestion
    pub fn apply_suggestion(&mut self, suggestion: &ClusterSuggestion) -> Result<()> {
        match suggestion.action {
            SuggestionAction::CreateCluster { ref nodes } => {
                let cluster = Cluster::new_manual(
                    suggestion.description.clone(),
                    nodes.clone(),
                );
                self.manager.add_cluster(cluster);
            }
            SuggestionAction::AddToCluster { cluster_id, node_id } => {
                self.manager.add_node_to_cluster(cluster_id, node_id)?;
            }
            SuggestionAction::RemoveFromCluster { cluster_id, node_id } => {
                self.manager.remove_node_from_cluster(cluster_id, node_id)?;
            }
            SuggestionAction::MergeClusters { cluster1, cluster2 } => {
                self.manager.merge_clusters(cluster1, cluster2)?;
            }
        }
        Ok(())
    }
}

/// Cluster suggestion
#[derive(Debug, Clone)]
pub struct ClusterSuggestion {
    /// Suggestion description
    pub description: String,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// Suggested action
    pub action: SuggestionAction,
    /// Reasoning for the suggestion
    pub reasoning: String,
}

/// Suggested clustering actions
#[derive(Debug, Clone)]
pub enum SuggestionAction {
    /// Create a new cluster
    CreateCluster { nodes: Vec<SceneId> },
    /// Add node to existing cluster
    AddToCluster { cluster_id: ClusterId, node_id: SceneId },
    /// Remove node from cluster
    RemoveFromCluster { cluster_id: ClusterId, node_id: SceneId },
    /// Merge two clusters
    MergeClusters { cluster1: ClusterId, cluster2: ClusterId },
}
