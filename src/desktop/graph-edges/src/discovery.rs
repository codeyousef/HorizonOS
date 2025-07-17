//! Automated relationship discovery system

use crate::{GraphEdge, EdgeError, RelationshipAnalyzer, RelationshipAnalysis, EdgeManager};
use horizonos_graph_engine::SceneId;
use horizonos_graph_nodes::GraphNode;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use tokio::sync::Mutex;
use serde::{Serialize, Deserialize};

/// Manages automated discovery of relationships between nodes
pub struct RelationshipDiscovery {
    analyzer: RelationshipAnalyzer,
    discovery_queue: Arc<Mutex<Vec<DiscoveryTask>>>,
    discovered_relationships: Arc<RwLock<HashMap<(SceneId, SceneId), RelationshipAnalysis>>>,
    discovery_settings: DiscoverySettings,
    processing_stats: Arc<RwLock<DiscoveryStatistics>>,
}

/// A task for discovering relationships
#[derive(Debug, Clone)]
pub struct DiscoveryTask {
    pub task_id: String,
    pub source_nodes: Vec<SceneId>,
    pub target_nodes: Option<Vec<SceneId>>, // None means analyze against all nodes
    pub priority: DiscoveryPriority,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub max_processing_time: chrono::Duration,
}

/// Priority levels for discovery tasks
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiscoveryPriority {
    Low,
    Medium,
    High,
    Urgent,
}

/// Settings for relationship discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoverySettings {
    pub auto_discovery_enabled: bool,
    pub max_concurrent_tasks: usize,
    pub discovery_interval: chrono::Duration,
    pub min_confidence_threshold: f32,
    pub max_relationships_per_node: usize,
    pub enable_temporal_discovery: bool,
    pub enable_content_analysis: bool,
    pub enable_usage_pattern_analysis: bool,
}

/// Statistics about discovery operations
#[derive(Debug, Clone)]
pub struct DiscoveryStatistics {
    pub total_tasks_processed: usize,
    pub relationships_discovered: usize,
    pub high_confidence_discoveries: usize,
    pub processing_time_total: chrono::Duration,
    pub last_discovery_run: Option<chrono::DateTime<chrono::Utc>>,
    pub discovery_by_type: HashMap<String, usize>,
    pub average_processing_time: chrono::Duration,
}

/// Result of a discovery operation
#[derive(Debug, Clone)]
pub struct DiscoveryResult {
    pub task_id: String,
    pub relationships_found: Vec<RelationshipAnalysis>,
    pub processing_time: chrono::Duration,
    pub nodes_analyzed: usize,
    pub success: bool,
    pub error_message: Option<String>,
}

impl RelationshipDiscovery {
    pub fn new() -> Self {
        RelationshipDiscovery {
            analyzer: RelationshipAnalyzer::new(),
            discovery_queue: Arc::new(Mutex::new(Vec::new())),
            discovered_relationships: Arc::new(RwLock::new(HashMap::new())),
            discovery_settings: DiscoverySettings::default(),
            processing_stats: Arc::new(RwLock::new(DiscoveryStatistics::default())),
        }
    }
    
    /// Add a discovery task to the queue
    pub async fn queue_discovery_task(&self, task: DiscoveryTask) -> Result<(), EdgeError> {
        let mut queue = self.discovery_queue.lock().await;
        
        // Insert task in priority order
        let insert_position = queue.iter()
            .position(|existing_task| existing_task.priority < task.priority)
            .unwrap_or(queue.len());
            
        queue.insert(insert_position, task);
        Ok(())
    }
    
    /// Process the next discovery task in the queue
    pub async fn process_next_task(&self, nodes: &HashMap<SceneId, &dyn GraphNode>) -> Option<DiscoveryResult> {
        let task = {
            let mut queue = self.discovery_queue.lock().await;
            queue.pop()
        };
        
        if let Some(task) = task {
            Some(self.process_discovery_task(task, nodes).await)
        } else {
            None
        }
    }
    
    /// Process a specific discovery task
    async fn process_discovery_task(&self, task: DiscoveryTask, nodes: &HashMap<SceneId, &dyn GraphNode>) -> DiscoveryResult {
        let start_time = chrono::Utc::now();
        let mut relationships_found = Vec::new();
        let mut nodes_analyzed = 0;
        
        // Get source nodes
        let source_nodes: Vec<&dyn GraphNode> = task.source_nodes.iter()
            .filter_map(|id| nodes.get(id).copied())
            .collect();
            
        // Get target nodes (or all nodes if none specified)
        let target_nodes: Vec<&dyn GraphNode> = if let Some(target_ids) = &task.target_nodes {
            target_ids.iter()
                .filter_map(|id| nodes.get(id).copied())
                .collect()
        } else {
            nodes.values().copied().collect()
        };
        
        // Analyze relationships
        for source in &source_nodes {
            for target in &target_nodes {
                if source.id() != target.id() {
                    nodes_analyzed += 1;
                    
                    if let Some(analysis) = self.analyzer.analyze_relationship(*source, *target) {
                        if analysis.confidence >= self.discovery_settings.min_confidence_threshold {
                            relationships_found.push(analysis);
                        }
                    }
                    
                    // Check if we've exceeded maximum processing time
                    if chrono::Utc::now() - start_time > task.max_processing_time {
                        break;
                    }
                }
            }
        }
        
        // Store discovered relationships
        {
            let mut discovered = self.discovered_relationships.write().unwrap();
            for relationship in &relationships_found {
                discovered.insert(
                    (relationship.source_id, relationship.target_id),
                    relationship.clone(),
                );
            }
        }
        
        // Update statistics
        self.update_statistics(&relationships_found, chrono::Utc::now() - start_time);
        
        DiscoveryResult {
            task_id: task.task_id,
            relationships_found,
            processing_time: chrono::Utc::now() - start_time,
            nodes_analyzed,
            success: true,
            error_message: None,
        }
    }
    
    /// Auto-discover relationships for new or updated nodes
    pub async fn auto_discover_for_nodes(&self, node_ids: Vec<SceneId>) -> Result<String, EdgeError> {
        let task_id = format!("auto_{}", uuid::Uuid::new_v4());
        
        let task = DiscoveryTask {
            task_id: task_id.clone(),
            source_nodes: node_ids,
            target_nodes: None, // Analyze against all nodes
            priority: DiscoveryPriority::Medium,
            created_at: chrono::Utc::now(),
            max_processing_time: chrono::Duration::minutes(5),
        };
        
        self.queue_discovery_task(task).await?;
        Ok(task_id)
    }
    
    /// Discover relationships between specific sets of nodes
    pub async fn discover_between_sets(&self, source_ids: Vec<SceneId>, target_ids: Vec<SceneId>) -> Result<String, EdgeError> {
        let task_id = format!("targeted_{}", uuid::Uuid::new_v4());
        
        let task = DiscoveryTask {
            task_id: task_id.clone(),
            source_nodes: source_ids,
            target_nodes: Some(target_ids),
            priority: DiscoveryPriority::High,
            created_at: chrono::Utc::now(),
            max_processing_time: chrono::Duration::minutes(10),
        };
        
        self.queue_discovery_task(task).await?;
        Ok(task_id)
    }
    
    /// Apply discovered relationships to an edge manager
    pub async fn apply_discoveries_to_manager(&self, edge_manager: &mut EdgeManager) -> Result<usize, EdgeError> {
        let discovered = self.discovered_relationships.read().unwrap();
        let mut edges_created = 0;
        
        for ((_source, _target), analysis) in discovered.iter() {
            // Check if edge already exists
            let existing_edges = edge_manager.get_outgoing_edges(analysis.source_id);
            let edge_exists = existing_edges.iter()
                .any(|edge| edge.target == analysis.target_id && 
                     std::mem::discriminant(&edge.edge_type) == std::mem::discriminant(&analysis.suggested_edge_type));
            
            if !edge_exists {
                match edge_manager.add_edge(analysis.source_id, analysis.target_id, analysis.suggested_edge_type.clone()) {
                    Ok(edge_id) => {
                        // Update the edge with discovery data
                        let _ = edge_manager.update_edge_strength(edge_id, analysis.strength);
                        edges_created += 1;
                        log::info!("Applied discovered relationship: {} -> {} (confidence: {:.2})", 
                                 analysis.source_id, analysis.target_id, analysis.confidence);
                    }
                    Err(e) => {
                        log::warn!("Failed to create edge from discovery: {:?}", e);
                    }
                }
            }
        }
        
        Ok(edges_created)
    }
    
    /// Get all discovered relationships
    pub fn get_discovered_relationships(&self) -> Vec<RelationshipAnalysis> {
        let discovered = self.discovered_relationships.read().unwrap();
        discovered.values().cloned().collect()
    }
    
    /// Get discovered relationships for a specific node
    pub fn get_relationships_for_node(&self, node_id: SceneId) -> Vec<RelationshipAnalysis> {
        let discovered = self.discovered_relationships.read().unwrap();
        discovered.values()
            .filter(|analysis| analysis.source_id == node_id || analysis.target_id == node_id)
            .cloned()
            .collect()
    }
    
    /// Clear old discovered relationships
    pub fn cleanup_old_discoveries(&self, max_age: chrono::Duration) -> usize {
        let mut discovered = self.discovered_relationships.write().unwrap();
        let cutoff_time = chrono::Utc::now() - max_age;
        let initial_count = discovered.len();
        
        discovered.retain(|_key, analysis| {
            // Keep relationships that are recent or have high confidence
            analysis.confidence > 0.8 || 
            chrono::Utc::now().signed_duration_since(cutoff_time) < max_age
        });
        
        let removed_count = initial_count - discovered.len();
        if removed_count > 0 {
            log::info!("Cleaned up {} old relationship discoveries", removed_count);
        }
        
        removed_count
    }
    
    /// Update discovery settings
    pub fn update_settings(&mut self, settings: DiscoverySettings) {
        self.discovery_settings = settings;
        log::info!("Updated relationship discovery settings");
    }
    
    /// Get current discovery settings
    pub fn get_settings(&self) -> &DiscoverySettings {
        &self.discovery_settings
    }
    
    /// Get discovery statistics
    pub fn get_statistics(&self) -> DiscoveryStatistics {
        let stats = self.processing_stats.read().unwrap();
        stats.clone()
    }
    
    /// Update processing statistics
    fn update_statistics(&self, relationships: &[RelationshipAnalysis], processing_time: chrono::Duration) {
        let mut stats = self.processing_stats.write().unwrap();
        
        stats.total_tasks_processed += 1;
        stats.relationships_discovered += relationships.len();
        stats.high_confidence_discoveries += relationships.iter()
            .filter(|r| r.confidence > 0.8)
            .count();
        stats.processing_time_total = stats.processing_time_total + processing_time;
        stats.last_discovery_run = Some(chrono::Utc::now());
        
        // Update discovery by type
        for relationship in relationships {
            let type_name = format!("{:?}", relationship.suggested_edge_type);
            *stats.discovery_by_type.entry(type_name).or_insert(0) += 1;
        }
        
        // Update average processing time
        if stats.total_tasks_processed > 0 {
            stats.average_processing_time = stats.processing_time_total / stats.total_tasks_processed as i32;
        }
    }
    
    /// Schedule periodic discovery
    pub async fn schedule_periodic_discovery(&self, node_ids: Vec<SceneId>) -> Result<(), EdgeError> {
        if !self.discovery_settings.auto_discovery_enabled {
            return Ok(());
        }
        
        let task_id = format!("periodic_{}", chrono::Utc::now().timestamp());
        
        let task = DiscoveryTask {
            task_id,
            source_nodes: node_ids,
            target_nodes: None,
            priority: DiscoveryPriority::Low,
            created_at: chrono::Utc::now(),
            max_processing_time: chrono::Duration::minutes(2),
        };
        
        self.queue_discovery_task(task).await
    }
}

impl Default for DiscoverySettings {
    fn default() -> Self {
        DiscoverySettings {
            auto_discovery_enabled: true,
            max_concurrent_tasks: 3,
            discovery_interval: chrono::Duration::minutes(15),
            min_confidence_threshold: 0.6,
            max_relationships_per_node: 50,
            enable_temporal_discovery: true,
            enable_content_analysis: false, // CPU intensive
            enable_usage_pattern_analysis: true,
        }
    }
}

impl Default for DiscoveryStatistics {
    fn default() -> Self {
        DiscoveryStatistics {
            total_tasks_processed: 0,
            relationships_discovered: 0,
            high_confidence_discoveries: 0,
            processing_time_total: chrono::Duration::zero(),
            last_discovery_run: None,
            discovery_by_type: HashMap::new(),
            average_processing_time: chrono::Duration::zero(),
        }
    }
}

impl Default for RelationshipDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use horizonos_graph_nodes::{BaseNode, NodeVisualData, NodeAction, NodeActionResult, NodeActionType, NodeError, NodeExportData};
    use horizonos_graph_engine::{SceneNode, NodeType, Position, Vec3};
    
    // Mock node for testing
    struct MockNode {
        id: SceneId,
        name: String,
    }
    
    impl GraphNode for MockNode {
        fn id(&self) -> SceneId { self.id }
        fn display_name(&self) -> String { self.name.clone() }
        fn description(&self) -> Option<String> { Some(format!("Mock node: {}", self.name)) }
        fn visual_data(&self) -> NodeVisualData { NodeVisualData::default() }
        fn update(&mut self, _delta_time: f32) -> Result<(), NodeError> { Ok(()) }
        fn handle_action(&mut self, _action: NodeAction) -> Result<NodeActionResult, NodeError> { 
            Ok(NodeActionResult::Success { message: None }) 
        }
        fn available_actions(&self) -> Vec<NodeActionType> { Vec::new() }
        fn export_data(&self) -> Result<NodeExportData, NodeError> { 
            Ok(NodeExportData {
                node_type: "mock".to_string(),
                display_name: self.name.clone(),
                description: Some(format!("Mock node: {}", self.name)),
                visual_data: NodeVisualData::default(),
                metadata: horizonos_graph_engine::NodeMetadata::default(),
                type_specific_data: serde_json::Value::Null,
            })
        }
        fn to_scene_node(&self) -> SceneNode {
            SceneNode {
                id: self.id,
                position: Position::new(0.0, 0.0, 0.0),
                velocity: Vec3::zeros(),
                radius: 1.0,
                color: [1.0, 1.0, 1.0, 1.0],
                node_type: NodeType::System { component: "mock".to_string(), status: horizonos_graph_engine::SystemStatus::Running },
                metadata: horizonos_graph_engine::NodeMetadata::default(),
                visible: true,
                selected: false,
            }
        }
    }
    
    #[tokio::test]
    async fn test_discovery_task_queue() {
        let discovery = RelationshipDiscovery::new();
        
        let task = DiscoveryTask {
            task_id: "test_task".to_string(),
            source_nodes: vec![1, 2],
            target_nodes: None,
            priority: DiscoveryPriority::High,
            created_at: chrono::Utc::now(),
            max_processing_time: chrono::Duration::minutes(5),
        };
        
        discovery.queue_discovery_task(task).await.unwrap();
        
        let queue = discovery.discovery_queue.lock().await;
        assert_eq!(queue.len(), 1);
        assert_eq!(queue[0].task_id, "test_task");
    }
    
    #[tokio::test]
    async fn test_auto_discover_for_nodes() {
        let discovery = RelationshipDiscovery::new();
        
        let task_id = discovery.auto_discover_for_nodes(vec![1, 2, 3]).await.unwrap();
        assert!(task_id.starts_with("auto_"));
        
        let queue = discovery.discovery_queue.lock().await;
        assert_eq!(queue.len(), 1);
    }
    
    #[test]
    fn test_discovery_settings() {
        let settings = DiscoverySettings::default();
        assert!(settings.auto_discovery_enabled);
        assert!(settings.min_confidence_threshold > 0.0);
        assert!(settings.max_relationships_per_node > 0);
    }
    
    #[test]
    fn test_statistics_initialization() {
        let stats = DiscoveryStatistics::default();
        assert_eq!(stats.total_tasks_processed, 0);
        assert_eq!(stats.relationships_discovered, 0);
        assert!(stats.last_discovery_run.is_none());
    }
    
    #[test]
    fn test_cleanup_old_discoveries() {
        let discovery = RelationshipDiscovery::new();
        
        // Add some mock discoveries
        {
            let mut discovered = discovery.discovered_relationships.write().unwrap();
            discovered.insert((1, 2), RelationshipAnalysis {
                source_id: 1,
                target_id: 2,
                suggested_edge_type: horizonos_graph_engine::EdgeType::Contains,
                confidence: 0.5, // Low confidence
                strength: 0.5,
                evidence: vec!["test".to_string()],
                bidirectional: false,
            });
        }
        
        let removed = discovery.cleanup_old_discoveries(chrono::Duration::hours(1));
        assert_eq!(removed, 1); // Should remove the low-confidence relationship
    }
}