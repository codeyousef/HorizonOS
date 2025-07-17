//! Smart clustering suggestions and recommendations

use crate::{Cluster, ClusterId, ClusterManager, ClusterSuggestion, SuggestionAction, ClusterType};
use anyhow::Result;
use horizonos_graph_engine::{SceneId, Scene};
use horizonos_graph_nodes::NodeType;
use nalgebra::Point3;
use std::collections::{HashMap, HashSet};

/// Suggestion engine for smart clustering
pub struct SuggestionEngine {
    /// Confidence thresholds for different suggestion types
    confidence_thresholds: SuggestionThresholds,
    /// History of accepted/rejected suggestions for learning
    suggestion_history: Vec<SuggestionHistoryEntry>,
}

/// Confidence thresholds for suggestions
#[derive(Debug, Clone)]
pub struct SuggestionThresholds {
    /// Minimum confidence for proximity suggestions
    pub proximity: f32,
    /// Minimum confidence for semantic suggestions
    pub semantic: f32,
    /// Minimum confidence for temporal suggestions
    pub temporal: f32,
    /// Minimum confidence for merge suggestions
    pub merge: f32,
    /// Minimum confidence for split suggestions
    pub split: f32,
}

/// History entry for suggestion learning
#[derive(Debug, Clone)]
pub struct SuggestionHistoryEntry {
    /// Suggestion that was made
    pub suggestion: ClusterSuggestion,
    /// Whether it was accepted
    pub accepted: bool,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl SuggestionEngine {
    /// Create new suggestion engine
    pub fn new() -> Self {
        Self {
            confidence_thresholds: SuggestionThresholds::default(),
            suggestion_history: Vec::new(),
        }
    }
    
    /// Generate suggestions for a specific node
    pub fn suggest_for_node(&self, node_id: SceneId, scene: &Scene, manager: &ClusterManager) -> Vec<ClusterSuggestion> {
        let mut suggestions = Vec::new();
        
        // Get current clusters for this node
        let current_clusters = manager.get_node_clusters(node_id);
        
        // Proximity-based suggestions
        suggestions.extend(self.suggest_proximity_clusters(node_id, scene, manager));
        
        // Semantic suggestions
        suggestions.extend(self.suggest_semantic_clusters(node_id, scene, manager));
        
        // Temporal suggestions
        suggestions.extend(self.suggest_temporal_clusters(node_id, scene, manager));
        
        // Remove from inappropriate clusters
        suggestions.extend(self.suggest_removals(node_id, scene, manager, &current_clusters));
        
        // Sort by confidence and filter by thresholds
        suggestions.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        suggestions.into_iter()
            .filter(|s| self.meets_confidence_threshold(s))
            .take(5) // Limit to top 5 suggestions
            .collect()
    }
    
    /// Generate cluster merge suggestions
    pub fn suggest_merges(&self, scene: &Scene, manager: &ClusterManager) -> Vec<ClusterSuggestion> {
        let mut suggestions = Vec::new();
        let clusters = manager.clusters();
        
        for i in 0..clusters.len() {
            for j in (i + 1)..clusters.len() {
                let cluster1 = &clusters[i];
                let cluster2 = &clusters[j];
                
                let confidence = self.calculate_merge_confidence(cluster1, cluster2, scene);
                if confidence >= self.confidence_thresholds.merge {
                    suggestions.push(ClusterSuggestion {
                        description: format!("Merge '{}' and '{}'", cluster1.name, cluster2.name),
                        confidence,
                        action: SuggestionAction::MergeClusters {
                            cluster1: cluster1.id,
                            cluster2: cluster2.id,
                        },
                        reasoning: self.explain_merge_reasoning(cluster1, cluster2, scene),
                    });
                }
            }
        }
        
        suggestions.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        suggestions
    }
    
    /// Generate cluster split suggestions
    pub fn suggest_splits(&self, scene: &Scene, manager: &ClusterManager) -> Vec<ClusterSuggestion> {
        let mut suggestions = Vec::new();
        
        for cluster in manager.clusters() {
            let confidence = self.calculate_split_confidence(&cluster, scene);
            if confidence >= self.confidence_thresholds.split {
                suggestions.push(ClusterSuggestion {
                    description: format!("Split cluster '{}'", cluster.name),
                    confidence,
                    action: SuggestionAction::CreateCluster {
                        nodes: cluster.nodes.iter().copied().collect(),
                    },
                    reasoning: self.explain_split_reasoning(&cluster, scene),
                });
            }
        }
        
        suggestions
    }
    
    /// Proximity-based clustering suggestions
    fn suggest_proximity_clusters(&self, node_id: SceneId, scene: &Scene, manager: &ClusterManager) -> Vec<ClusterSuggestion> {
        let mut suggestions = Vec::new();
        
        if let Some(node_pos) = scene.get_node_position(node_id) {
            let mut nearby_nodes = Vec::new();
            
            // Find nearby nodes
            for other_id in scene.get_all_nodes() {
                if other_id == node_id {
                    continue;
                }
                
                if let Some(other_pos) = scene.get_node_position(other_id) {
                    let distance = (node_pos - other_pos).magnitude();
                    if distance <= 50.0 { // Proximity threshold
                        nearby_nodes.push((other_id, distance));
                    }
                }
            }
            
            // Sort by distance
            nearby_nodes.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
            
            // Suggest adding to existing nearby clusters
            for (other_id, distance) in nearby_nodes.iter().take(3) {
                let other_clusters = manager.get_node_clusters(*other_id);
                for cluster_id in other_clusters {
                    if !manager.get_node_clusters(node_id).contains(&cluster_id) {
                        let confidence = 1.0 - (distance / 50.0) * 0.5; // Distance-based confidence
                        
                        if let Some(cluster) = manager.get_cluster(cluster_id) {
                            suggestions.push(ClusterSuggestion {
                                description: format!("Add to nearby cluster '{}'", cluster.name),
                                confidence,
                                action: SuggestionAction::AddToCluster { cluster_id, node_id },
                                reasoning: format!("Node is {:.1} units away from cluster", distance),
                            });
                        }
                    }
                }
            }
            
            // Suggest creating new cluster with nearby unclustered nodes
            let unclustered_nearby: Vec<SceneId> = nearby_nodes.iter()
                .filter(|(other_id, _)| manager.get_node_clusters(*other_id).is_empty())
                .map(|(other_id, _)| *other_id)
                .take(5)
                .collect();
            
            if unclustered_nearby.len() >= 2 {
                let mut nodes = unclustered_nearby;
                nodes.push(node_id);
                
                suggestions.push(ClusterSuggestion {
                    description: "Create proximity cluster".to_string(),
                    confidence: 0.7,
                    action: SuggestionAction::CreateCluster { nodes },
                    reasoning: "Multiple nearby unclustered nodes found".to_string(),
                });
            }
        }
        
        suggestions
    }
    
    /// Semantic clustering suggestions
    fn suggest_semantic_clusters(&self, node_id: SceneId, scene: &Scene, manager: &ClusterManager) -> Vec<ClusterSuggestion> {
        let mut suggestions = Vec::new();
        
        if let Some(node) = scene.get_node(node_id) {
            let node_type = &node.node_type;
            
            // Find existing semantic clusters of the same type
            for cluster in manager.clusters() {
                if cluster.cluster_type == ClusterType::Semantic {
                    // Check if cluster contains nodes of the same type
                    let same_type_count = cluster.nodes.iter()
                        .filter_map(|&id| scene.get_node(id))
                        .filter(|n| std::mem::discriminant(&n.node_type) == std::mem::discriminant(node_type))
                        .count();
                    
                    if same_type_count > 0 && !cluster.contains_node(node_id) {
                        let confidence = (same_type_count as f32 / cluster.node_count() as f32) * 0.8;
                        
                        suggestions.push(ClusterSuggestion {
                            description: format!("Add to {} cluster", cluster.name),
                            confidence,
                            action: SuggestionAction::AddToCluster {
                                cluster_id: cluster.id,
                                node_id,
                            },
                            reasoning: format!("Node type matches {}/{} nodes in cluster", 
                                same_type_count, cluster.node_count()),
                        });
                    }
                }
            }
            
            // Suggest creating new semantic cluster
            let same_type_nodes: Vec<SceneId> = scene.get_all_nodes()
                .into_iter()
                .filter(|&id| id != node_id)
                .filter(|&id| {
                    scene.get_node(id)
                        .map(|n| std::mem::discriminant(&n.node_type) == std::mem::discriminant(node_type))
                        .unwrap_or(false)
                })
                .filter(|&id| manager.get_node_clusters(id).is_empty())
                .take(10)
                .collect();
            
            if same_type_nodes.len() >= 2 {
                let nodes_count = same_type_nodes.len();
                let mut nodes = same_type_nodes;
                nodes.push(node_id);
                
                let type_name = match node_type {
                    horizonos_graph_engine::NodeType::Application { .. } => "Applications",
                    horizonos_graph_engine::NodeType::File { .. } => "Files",
                    horizonos_graph_engine::NodeType::Person { .. } => "People",
                    horizonos_graph_engine::NodeType::Task { .. } => "Tasks",
                    horizonos_graph_engine::NodeType::Device { .. } => "Devices",
                    horizonos_graph_engine::NodeType::AIAgent { .. } => "AI Agents",
                    horizonos_graph_engine::NodeType::Concept { .. } => "Concepts",
                    horizonos_graph_engine::NodeType::System { .. } => "System Components",
                    horizonos_graph_engine::NodeType::URL { .. } => "URLs",
                    horizonos_graph_engine::NodeType::Automation { .. } => "Automations",
                    horizonos_graph_engine::NodeType::Setting { .. } => "Settings",
                    horizonos_graph_engine::NodeType::ConfigGroup { .. } => "Configuration Groups",
                };
                
                suggestions.push(ClusterSuggestion {
                    description: format!("Create {} cluster", type_name),
                    confidence: 0.8,
                    action: SuggestionAction::CreateCluster { nodes },
                    reasoning: format!("Found {} similar unclustered nodes", nodes_count),
                });
            }
        }
        
        suggestions
    }
    
    /// Temporal clustering suggestions
    fn suggest_temporal_clusters(&self, node_id: SceneId, scene: &Scene, manager: &ClusterManager) -> Vec<ClusterSuggestion> {
        let mut suggestions = Vec::new();
        
        if let Some(node) = scene.get_node(node_id) {
            let node_time = node.metadata.created_at;
            
            // Find nodes created around the same time
            let time_window = chrono::Duration::hours(1);
            let mut contemporary_nodes = Vec::new();
            
            for other_id in scene.get_all_nodes() {
                if other_id == node_id {
                    continue;
                }
                
                if let Some(other_node) = scene.get_node(other_id) {
                    let other_time = other_node.metadata.created_at;
                    let time_diff = (node_time - other_time).abs();
                    
                    if time_diff <= time_window {
                        contemporary_nodes.push(other_id);
                    }
                }
            }
            
            if contemporary_nodes.len() >= 2 {
                // Filter out already clustered nodes
                let unclustered: Vec<SceneId> = contemporary_nodes.into_iter()
                    .filter(|&id| manager.get_node_clusters(id).is_empty())
                    .collect();
                
                if unclustered.len() >= 2 {
                    let mut nodes = unclustered;
                    nodes.push(node_id);
                    
                    suggestions.push(ClusterSuggestion {
                        description: "Create temporal cluster".to_string(),
                        confidence: 0.6,
                        action: SuggestionAction::CreateCluster { nodes },
                        reasoning: "Multiple nodes created around the same time".to_string(),
                    });
                }
            }
        }
        
        suggestions
    }
    
    /// Suggest removals from inappropriate clusters
    fn suggest_removals(&self, node_id: SceneId, scene: &Scene, manager: &ClusterManager, current_clusters: &[ClusterId]) -> Vec<ClusterSuggestion> {
        let mut suggestions = Vec::new();
        
        for &cluster_id in current_clusters {
            if let Some(cluster) = manager.get_cluster(cluster_id) {
                let confidence = self.calculate_removal_confidence(node_id, &cluster, scene);
                if confidence >= 0.6 {
                    suggestions.push(ClusterSuggestion {
                        description: format!("Remove from cluster '{}'", cluster.name),
                        confidence,
                        action: SuggestionAction::RemoveFromCluster { cluster_id, node_id },
                        reasoning: "Node doesn't fit well with other cluster members".to_string(),
                    });
                }
            }
        }
        
        suggestions
    }
    
    /// Calculate confidence for merging two clusters
    fn calculate_merge_confidence(&self, cluster1: &Cluster, cluster2: &Cluster, scene: &Scene) -> f32 {
        let mut confidence = 0.0;
        
        // Type similarity
        if cluster1.cluster_type == cluster2.cluster_type {
            confidence += 0.3;
        }
        
        // Spatial proximity
        if let (Some(center1), Some(center2)) = (
            self.calculate_cluster_center(cluster1, scene),
            self.calculate_cluster_center(cluster2, scene)
        ) {
            let distance = (center1 - center2).magnitude();
            if distance <= 100.0 {
                confidence += 0.4 * (1.0 - distance / 100.0);
            }
        }
        
        // Size similarity
        let size_ratio = (cluster1.node_count() as f32 / cluster2.node_count() as f32).min(
            cluster2.node_count() as f32 / cluster1.node_count() as f32
        );
        confidence += 0.2 * size_ratio;
        
        // Semantic similarity (if both contain same node types)
        let common_types = self.count_common_node_types(cluster1, cluster2, scene);
        if common_types > 0 {
            confidence += 0.1;
        }
        
        confidence.min(1.0_f32)
    }
    
    /// Calculate confidence for splitting a cluster
    fn calculate_split_confidence(&self, cluster: &Cluster, scene: &Scene) -> f32 {
        if cluster.node_count() < 4 {
            return 0.0; // Too small to split
        }
        
        let mut confidence: f32 = 0.0;
        
        // Large clusters are more likely to benefit from splitting
        if cluster.node_count() > 10 {
            confidence += 0.3;
        }
        
        // Check spatial distribution
        if let Some(spatial_variance) = self.calculate_spatial_variance(cluster, scene) {
            if spatial_variance > 50.0 {
                confidence += 0.4;
            }
        }
        
        // Check type diversity
        let type_diversity = self.calculate_type_diversity(cluster, scene);
        if type_diversity > 0.5 {
            confidence += 0.3;
        }
        
        confidence.min(1.0_f32)
    }
    
    /// Calculate confidence for removing a node from a cluster
    fn calculate_removal_confidence(&self, node_id: SceneId, cluster: &Cluster, scene: &Scene) -> f32 {
        let mut confidence: f32 = 0.0;
        
        if let Some(node) = scene.get_node(node_id) {
            let node_type = &node.node_type;
            
            // Check type similarity with other cluster members
            let same_type_count = cluster.nodes.iter()
                .filter(|&&id| id != node_id)
                .filter_map(|&id| scene.get_node(id))
                .filter(|n| std::mem::discriminant(&n.node_type) == std::mem::discriminant(node_type))
                .count();
            
            let type_ratio = same_type_count as f32 / (cluster.node_count() - 1) as f32;
            if type_ratio < 0.3 {
                confidence += 0.4; // Node type is uncommon in cluster
            }
            
            // Check spatial outlier
            if let Some(node_pos) = scene.get_node_position(node_id) {
                if let Some(cluster_center) = self.calculate_cluster_center(cluster, scene) {
                    let distance = (node_pos - cluster_center).magnitude();
                    let avg_distance = self.calculate_average_distance_to_center(cluster, scene, cluster_center);
                    
                    if distance > avg_distance * 2.0 {
                        confidence += 0.4; // Node is spatial outlier
                    }
                }
            }
        }
        
        confidence.min(1.0_f32)
    }
    
    /// Calculate cluster center
    fn calculate_cluster_center(&self, cluster: &Cluster, scene: &Scene) -> Option<Point3<f32>> {
        let positions: Vec<Point3<f32>> = cluster.nodes.iter()
            .filter_map(|&id| scene.get_node_position(id))
            .collect();
        
        if positions.is_empty() {
            None
        } else {
            let sum = positions.iter().fold(nalgebra::Vector3::zeros(), |acc, p| acc + p.coords);
            Some(Point3::from(sum / positions.len() as f32))
        }
    }
    
    /// Calculate spatial variance within cluster
    fn calculate_spatial_variance(&self, cluster: &Cluster, scene: &Scene) -> Option<f32> {
        if let Some(center) = self.calculate_cluster_center(cluster, scene) {
            let distances: Vec<f32> = cluster.nodes.iter()
                .filter_map(|&id| scene.get_node_position(id))
                .map(|pos| (pos - center).magnitude())
                .collect();
            
            if distances.is_empty() {
                None
            } else {
                let mean = distances.iter().sum::<f32>() / distances.len() as f32;
                let variance = distances.iter()
                    .map(|d| (d - mean).powi(2))
                    .sum::<f32>() / distances.len() as f32;
                Some(variance)
            }
        } else {
            None
        }
    }
    
    /// Calculate type diversity within cluster
    fn calculate_type_diversity(&self, cluster: &Cluster, scene: &Scene) -> f32 {
        let mut type_counts = HashMap::new();
        
        for &node_id in &cluster.nodes {
            if let Some(node) = scene.get_node(node_id) {
                let type_discriminant = std::mem::discriminant(&node.node_type);
                *type_counts.entry(type_discriminant).or_insert(0) += 1;
            }
        }
        
        if type_counts.is_empty() {
            0.0
        } else {
            let total = cluster.node_count() as f32;
            let entropy = type_counts.values()
                .map(|&count| {
                    let p = count as f32 / total;
                    -p * p.log2()
                })
                .sum::<f32>();
            
            entropy / (type_counts.len() as f32).log2() // Normalized entropy
        }
    }
    
    /// Count common node types between clusters
    fn count_common_node_types(&self, cluster1: &Cluster, cluster2: &Cluster, scene: &Scene) -> usize {
        use std::mem::Discriminant;
        
        let types1: HashSet<Discriminant<horizonos_graph_engine::NodeType>> = cluster1.nodes.iter()
            .filter_map(|&id| scene.get_node(id))
            .map(|n| std::mem::discriminant(&n.node_type))
            .collect();
        
        let types2: HashSet<Discriminant<horizonos_graph_engine::NodeType>> = cluster2.nodes.iter()
            .filter_map(|&id| scene.get_node(id))
            .map(|n| std::mem::discriminant(&n.node_type))
            .collect();
        
        types1.intersection(&types2).count()
    }
    
    /// Calculate average distance to center
    fn calculate_average_distance_to_center(&self, cluster: &Cluster, scene: &Scene, center: Point3<f32>) -> f32 {
        let distances: Vec<f32> = cluster.nodes.iter()
            .filter_map(|&id| scene.get_node_position(id))
            .map(|pos| (pos - center).magnitude())
            .collect();
        
        if distances.is_empty() {
            0.0
        } else {
            distances.iter().sum::<f32>() / distances.len() as f32
        }
    }
    
    /// Check if suggestion meets confidence threshold
    fn meets_confidence_threshold(&self, suggestion: &ClusterSuggestion) -> bool {
        match suggestion.action {
            SuggestionAction::CreateCluster { .. } => suggestion.confidence >= self.confidence_thresholds.proximity,
            SuggestionAction::AddToCluster { .. } => suggestion.confidence >= self.confidence_thresholds.semantic,
            SuggestionAction::RemoveFromCluster { .. } => suggestion.confidence >= self.confidence_thresholds.semantic,
            SuggestionAction::MergeClusters { .. } => suggestion.confidence >= self.confidence_thresholds.merge,
        }
    }
    
    /// Explain merge reasoning
    fn explain_merge_reasoning(&self, cluster1: &Cluster, cluster2: &Cluster, scene: &Scene) -> String {
        let mut reasons = Vec::new();
        
        if cluster1.cluster_type == cluster2.cluster_type {
            reasons.push("Same cluster type".to_string());
        }
        
        if let (Some(center1), Some(center2)) = (
            self.calculate_cluster_center(cluster1, scene),
            self.calculate_cluster_center(cluster2, scene)
        ) {
            let distance = (center1 - center2).magnitude();
            if distance <= 100.0 {
                reasons.push(format!("Clusters are {:.1} units apart", distance));
            }
        }
        
        let common_types = self.count_common_node_types(cluster1, cluster2, scene);
        if common_types > 0 {
            reasons.push(format!("{} common node types", common_types));
        }
        
        if reasons.is_empty() {
            "Clusters appear related".to_string()
        } else {
            reasons.join(", ")
        }
    }
    
    /// Explain split reasoning
    fn explain_split_reasoning(&self, cluster: &Cluster, scene: &Scene) -> String {
        let mut reasons = Vec::new();
        
        if cluster.node_count() > 10 {
            reasons.push(format!("Large cluster ({} nodes)", cluster.node_count()));
        }
        
        if let Some(variance) = self.calculate_spatial_variance(cluster, scene) {
            if variance > 50.0 {
                reasons.push("High spatial variance".to_string());
            }
        }
        
        let diversity = self.calculate_type_diversity(cluster, scene);
        if diversity > 0.5 {
            reasons.push("High type diversity".to_string());
        }
        
        if reasons.is_empty() {
            "Cluster may benefit from splitting".to_string()
        } else {
            reasons.join(", ")
        }
    }
    
    /// Record suggestion feedback for learning
    pub fn record_feedback(&mut self, suggestion: ClusterSuggestion, accepted: bool) {
        self.suggestion_history.push(SuggestionHistoryEntry {
            suggestion,
            accepted,
            timestamp: chrono::Utc::now(),
        });
        
        // Adjust thresholds based on feedback
        self.adjust_thresholds_from_feedback();
    }
    
    /// Adjust confidence thresholds based on historical feedback
    fn adjust_thresholds_from_feedback(&mut self) {
        if self.suggestion_history.len() < 10 {
            return;
        }
        
        let recent_history: Vec<_> = self.suggestion_history.iter()
            .rev()
            .take(50)
            .collect();
        
        // Calculate acceptance rates for different suggestion types
        let proximity_acceptance = self.calculate_acceptance_rate(&recent_history, |s| matches!(s.action, SuggestionAction::CreateCluster { .. }));
        let semantic_acceptance = self.calculate_acceptance_rate(&recent_history, |s| matches!(s.action, SuggestionAction::AddToCluster { .. }));
        let merge_acceptance = self.calculate_acceptance_rate(&recent_history, |s| matches!(s.action, SuggestionAction::MergeClusters { .. }));
        
        // Adjust thresholds based on acceptance rates
        if proximity_acceptance < 0.3 {
            self.confidence_thresholds.proximity = (self.confidence_thresholds.proximity + 0.1).min(0.9);
        } else if proximity_acceptance > 0.7 {
            self.confidence_thresholds.proximity = (self.confidence_thresholds.proximity - 0.05).max(0.1);
        }
        
        if semantic_acceptance < 0.3 {
            self.confidence_thresholds.semantic = (self.confidence_thresholds.semantic + 0.1).min(0.9);
        } else if semantic_acceptance > 0.7 {
            self.confidence_thresholds.semantic = (self.confidence_thresholds.semantic - 0.05).max(0.1);
        }
        
        if merge_acceptance < 0.3 {
            self.confidence_thresholds.merge = (self.confidence_thresholds.merge + 0.1).min(0.9);
        } else if merge_acceptance > 0.7 {
            self.confidence_thresholds.merge = (self.confidence_thresholds.merge - 0.05).max(0.1);
        }
    }
    
    /// Calculate acceptance rate for specific suggestion types
    fn calculate_acceptance_rate<F>(&self, history: &[&SuggestionHistoryEntry], filter: F) -> f32 
    where
        F: Fn(&ClusterSuggestion) -> bool,
    {
        let relevant: Vec<_> = history.iter()
            .filter(|entry| filter(&entry.suggestion))
            .collect();
        
        if relevant.is_empty() {
            0.5 // Default rate
        } else {
            let accepted = relevant.iter().filter(|entry| entry.accepted).count();
            accepted as f32 / relevant.len() as f32
        }
    }
}

impl Default for SuggestionEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for SuggestionThresholds {
    fn default() -> Self {
        Self {
            proximity: 0.6,
            semantic: 0.7,
            temporal: 0.5,
            merge: 0.8,
            split: 0.7,
        }
    }
}