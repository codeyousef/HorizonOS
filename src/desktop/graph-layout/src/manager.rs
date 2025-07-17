//! Layout manager for coordinating different layout algorithms

use crate::{
    LayoutAlgorithm, LayoutNode, LayoutEdge, LayoutResult, LayoutError, LayoutType, 
    LayoutAnimationSettings, LayoutBounds, ForceDirectedLayout, utils
};
use horizonos_graph_engine::{SceneId, Position};
use horizonos_graph_nodes::GraphNode;
use horizonos_graph_edges::{GraphEdge, EdgeManager};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::Mutex;
use serde::{Serialize, Deserialize};

/// Main layout manager that coordinates different layout algorithms
pub struct LayoutManager {
    current_layout: LayoutType,
    layout_algorithms: HashMap<String, Box<dyn LayoutAlgorithm>>,
    animation_settings: LayoutAnimationSettings,
    bounds: LayoutBounds,
    node_positions: Arc<RwLock<HashMap<SceneId, Position>>>,
    target_positions: Arc<RwLock<HashMap<SceneId, Position>>>,
    layout_cache: Arc<Mutex<HashMap<String, LayoutResult>>>,
    auto_layout_enabled: bool,
    layout_stats: LayoutStatistics,
}

/// Statistics about layout operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutStatistics {
    pub total_layouts_calculated: usize,
    pub total_processing_time: chrono::Duration,
    pub average_processing_time: chrono::Duration,
    pub last_layout_time: Option<chrono::DateTime<chrono::Utc>>,
    pub layout_type_usage: HashMap<String, usize>,
    pub convergence_rate: f32,
}

/// Configuration for layout behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutConfiguration {
    pub layout_type: LayoutType,
    pub animation_settings: LayoutAnimationSettings,
    pub bounds: LayoutBounds,
    pub auto_layout_enabled: bool,
    pub layout_update_interval: chrono::Duration,
    pub energy_threshold: f32,
}

/// Layout update event
#[derive(Debug, Clone)]
pub struct LayoutUpdateEvent {
    pub event_type: LayoutEventType,
    pub affected_nodes: Vec<SceneId>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Types of layout events
#[derive(Debug, Clone)]
pub enum LayoutEventType {
    NodesAdded,
    NodesRemoved,
    NodesModified,
    EdgesAdded,
    EdgesRemoved,
    LayoutChanged,
    ManualPositionUpdate,
}

impl LayoutManager {
    pub fn new() -> Self {
        let mut manager = LayoutManager {
            current_layout: LayoutType::default(),
            layout_algorithms: HashMap::new(),
            animation_settings: LayoutAnimationSettings::default(),
            bounds: LayoutBounds::default(),
            node_positions: Arc::new(RwLock::new(HashMap::new())),
            target_positions: Arc::new(RwLock::new(HashMap::new())),
            layout_cache: Arc::new(Mutex::new(HashMap::new())),
            auto_layout_enabled: true,
            layout_stats: LayoutStatistics::default(),
        };
        
        // Register default algorithms
        manager.register_algorithm("force_directed", Box::new(ForceDirectedLayout::new()));
        
        manager
    }
    
    /// Register a layout algorithm
    pub fn register_algorithm(&mut self, name: &str, algorithm: Box<dyn LayoutAlgorithm>) {
        self.layout_algorithms.insert(name.to_string(), algorithm);
        log::info!("Registered layout algorithm: {}", name);
    }
    
    /// Set the current layout type
    pub fn set_layout_type(&mut self, layout_type: LayoutType) -> Result<(), LayoutError> {
        self.current_layout = layout_type;
        
        // Clear cache when layout type changes
        let cache = self.layout_cache.clone();
        tokio::spawn(async move {
            let mut cache = cache.lock().await;
            cache.clear();
        });
        
        log::info!("Changed layout type to: {:?}", self.current_layout);
        Ok(())
    }
    
    /// Calculate layout for a set of nodes and edges
    pub async fn calculate_layout(
        &mut self,
        nodes: &[&dyn GraphNode],
        edge_manager: &EdgeManager,
    ) -> Result<LayoutResult, LayoutError> {
        let start_time = chrono::Utc::now();
        
        // Convert nodes to layout nodes
        let layout_nodes = self.convert_to_layout_nodes(nodes);
        
        // Get edges from edge manager
        let layout_edges = self.convert_to_layout_edges(edge_manager, &layout_nodes);
        
        // Get appropriate algorithm
        let algorithm = self.get_algorithm_for_layout_type(&self.current_layout)?;
        
        // Calculate layout
        let result = algorithm.calculate_layout(&layout_nodes, &layout_edges)?;
        
        // Update positions
        {
            let mut positions = self.node_positions.write().unwrap();
            for (node_id, position) in &result.node_positions {
                positions.insert(*node_id, *position);
            }
        }
        
        // Start animation to target positions if enabled
        if self.animation_settings.enabled {
            self.start_position_animation(&result.node_positions).await;
        }
        
        // Update statistics
        self.update_statistics(&result, start_time);
        
        log::info!("Layout calculation completed: {} nodes, {} iterations, energy: {:.4}", 
                  layout_nodes.len(), result.iterations_performed, result.energy);
        
        Ok(result)
    }
    
    /// Update layout incrementally for real-time updates
    pub async fn update_layout_incremental(
        &mut self,
        nodes: &[&dyn GraphNode],
        edge_manager: &EdgeManager,
        delta_time: f32,
    ) -> Result<f32, LayoutError> {
        let mut layout_nodes = self.convert_to_layout_nodes(nodes);
        let layout_edges = self.convert_to_layout_edges(edge_manager, &layout_nodes);
        
        // Get current positions
        {
            let positions = self.node_positions.read().unwrap();
            for node in &mut layout_nodes {
                if let Some(pos) = positions.get(&node.id) {
                    node.position = *pos;
                }
            }
        }
        
        let algorithm = self.get_algorithm_for_layout_type(&self.current_layout)?;
        
        if algorithm.supports_incremental() {
            let energy = algorithm.update_layout(&mut layout_nodes, &layout_edges, delta_time)?;
            
            // Update stored positions
            {
                let mut positions = self.node_positions.write().unwrap();
                for node in &layout_nodes {
                    positions.insert(node.id, node.position);
                }
            }
            
            Ok(energy)
        } else {
            // Fall back to full calculation for non-incremental algorithms
            let result = self.calculate_layout(nodes, edge_manager).await?;
            Ok(result.energy)
        }
    }
    
    /// Get current position of a node
    pub fn get_node_position(&self, node_id: SceneId) -> Option<Position> {
        let positions = self.node_positions.read().unwrap();
        positions.get(&node_id).copied()
    }
    
    /// Set position of a node manually
    pub fn set_node_position(&mut self, node_id: SceneId, position: Position) {
        let mut positions = self.node_positions.write().unwrap();
        positions.insert(node_id, position);
        
        log::debug!("Manually set position for node {}: {:?}", node_id, position);
    }
    
    /// Get all current node positions
    pub fn get_all_positions(&self) -> HashMap<SceneId, Position> {
        let positions = self.node_positions.read().unwrap();
        positions.clone()
    }
    
    /// Convert graph nodes to layout nodes
    fn convert_to_layout_nodes(&self, nodes: &[&dyn GraphNode]) -> Vec<LayoutNode> {
        let positions = self.node_positions.read().unwrap();
        
        nodes.iter().map(|node| {
            let position = positions.get(&node.id())
                .copied()
                .unwrap_or_else(|| utils::random_position(&self.bounds));
            
            LayoutNode::new(node.id(), position)
        }).collect()
    }
    
    /// Convert graph edges to layout edges
    fn convert_to_layout_edges(&self, edge_manager: &EdgeManager, nodes: &[LayoutNode]) -> Vec<LayoutEdge> {
        let mut layout_edges = Vec::new();
        
        for node in nodes {
            let edges = edge_manager.get_outgoing_edges(node.id);
            for edge in edges {
                let weight = edge.relationship_data.strength;
                let length = match &edge.edge_type {
                    horizonos_graph_engine::EdgeType::Contains => 30.0,
                    horizonos_graph_engine::EdgeType::DependsOn => 50.0,
                    horizonos_graph_engine::EdgeType::CommunicatesWith => 40.0,
                    horizonos_graph_engine::EdgeType::CreatedBy => 35.0,
                    horizonos_graph_engine::EdgeType::RelatedTo { similarity } => 20.0 + (1.0 - similarity) * 30.0,
                    horizonos_graph_engine::EdgeType::Temporal { .. } => 25.0,
                    horizonos_graph_engine::EdgeType::TaggedAs { .. } => 20.0,
                    horizonos_graph_engine::EdgeType::WorksOn => 45.0,
                };
                
                layout_edges.push(LayoutEdge::new(edge.source, edge.target, weight).with_length(length));
            }
        }
        
        layout_edges
    }
    
    /// Get algorithm for current layout type
    fn get_algorithm_for_layout_type(&self, layout_type: &LayoutType) -> Result<&dyn LayoutAlgorithm, LayoutError> {
        match layout_type {
            LayoutType::ForceDirected { .. } => {
                self.layout_algorithms.get("force_directed")
                    .map(|alg| alg.as_ref())
                    .ok_or_else(|| LayoutError::InvalidConfiguration { 
                        message: "Force-directed algorithm not registered".to_string() 
                    })
            }
            LayoutType::Hierarchical { .. } => {
                self.layout_algorithms.get("hierarchical")
                    .map(|alg| alg.as_ref())
                    .ok_or_else(|| LayoutError::InvalidConfiguration { 
                        message: "Hierarchical algorithm not registered".to_string() 
                    })
            }
            LayoutType::Circular { .. } => {
                self.layout_algorithms.get("circular")
                    .map(|alg| alg.as_ref())
                    .ok_or_else(|| LayoutError::InvalidConfiguration { 
                        message: "Circular algorithm not registered".to_string() 
                    })
            }
            LayoutType::Grid { .. } => {
                self.layout_algorithms.get("grid")
                    .map(|alg| alg.as_ref())
                    .ok_or_else(|| LayoutError::InvalidConfiguration { 
                        message: "Grid algorithm not registered".to_string() 
                    })
            }
            LayoutType::Cluster { .. } => {
                self.layout_algorithms.get("cluster")
                    .map(|alg| alg.as_ref())
                    .ok_or_else(|| LayoutError::InvalidConfiguration { 
                        message: "Cluster algorithm not registered".to_string() 
                    })
            }
            LayoutType::Temporal { .. } => {
                self.layout_algorithms.get("temporal")
                    .map(|alg| alg.as_ref())
                    .ok_or_else(|| LayoutError::InvalidConfiguration { 
                        message: "Temporal algorithm not registered".to_string() 
                    })
            }
            LayoutType::Manual => {
                Err(LayoutError::InvalidConfiguration { 
                    message: "Manual layout does not use algorithms".to_string() 
                })
            }
        }
    }
    
    /// Start smooth animation between current and target positions
    async fn start_position_animation(&self, target_positions: &HashMap<SceneId, Position>) {
        let mut targets = self.target_positions.write().unwrap();
        for (node_id, position) in target_positions {
            targets.insert(*node_id, *position);
        }
        
        // Animation would be handled by the rendering system
        log::debug!("Started position animation for {} nodes", target_positions.len());
    }
    
    /// Update layout statistics
    fn update_statistics(&mut self, result: &LayoutResult, start_time: chrono::DateTime<chrono::Utc>) {
        self.layout_stats.total_layouts_calculated += 1;
        self.layout_stats.total_processing_time = self.layout_stats.total_processing_time + result.processing_time;
        self.layout_stats.last_layout_time = Some(start_time);
        
        // Update average processing time
        if self.layout_stats.total_layouts_calculated > 0 {
            self.layout_stats.average_processing_time = 
                self.layout_stats.total_processing_time / self.layout_stats.total_layouts_calculated as i32;
        }
        
        // Update layout type usage
        let layout_name = format!("{:?}", self.current_layout);
        *self.layout_stats.layout_type_usage.entry(layout_name).or_insert(0) += 1;
        
        // Update convergence rate
        if result.converged {
            self.layout_stats.convergence_rate = 
                (self.layout_stats.convergence_rate * (self.layout_stats.total_layouts_calculated - 1) as f32 + 1.0) 
                / self.layout_stats.total_layouts_calculated as f32;
        }
    }
    
    /// Configure layout settings
    pub fn configure(&mut self, config: LayoutConfiguration) -> Result<(), LayoutError> {
        self.current_layout = config.layout_type;
        self.animation_settings = config.animation_settings;
        self.bounds = config.bounds;
        self.auto_layout_enabled = config.auto_layout_enabled;
        
        log::info!("Layout manager configuration updated");
        Ok(())
    }
    
    /// Get current layout statistics
    pub fn get_statistics(&self) -> &LayoutStatistics {
        &self.layout_stats
    }
    
    /// Clear all cached layout data
    pub async fn clear_cache(&self) {
        let mut cache = self.layout_cache.lock().await;
        cache.clear();
        
        let mut positions = self.node_positions.write().unwrap();
        positions.clear();
        
        let mut targets = self.target_positions.write().unwrap();
        targets.clear();
        
        log::info!("Layout cache cleared");
    }
    
    /// Handle layout update events
    pub async fn handle_layout_event(&mut self, event: LayoutUpdateEvent) -> Result<(), LayoutError> {
        if !self.auto_layout_enabled {
            return Ok(());
        }
        
        match event.event_type {
            LayoutEventType::NodesAdded | LayoutEventType::EdgesAdded => {
                // Trigger incremental layout update
                log::debug!("Handling layout event: {:?}", event.event_type);
            }
            LayoutEventType::NodesRemoved => {
                // Remove positions for deleted nodes
                let mut positions = self.node_positions.write().unwrap();
                for node_id in &event.affected_nodes {
                    positions.remove(node_id);
                }
            }
            LayoutEventType::LayoutChanged => {
                // Clear cache and recalculate
                self.clear_cache().await;
            }
            _ => {}
        }
        
        Ok(())
    }
}

impl Default for LayoutManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for LayoutStatistics {
    fn default() -> Self {
        LayoutStatistics {
            total_layouts_calculated: 0,
            total_processing_time: chrono::Duration::zero(),
            average_processing_time: chrono::Duration::zero(),
            last_layout_time: None,
            layout_type_usage: HashMap::new(),
            convergence_rate: 0.0,
        }
    }
}

impl Default for LayoutConfiguration {
    fn default() -> Self {
        LayoutConfiguration {
            layout_type: LayoutType::default(),
            animation_settings: LayoutAnimationSettings::default(),
            bounds: LayoutBounds::default(),
            auto_layout_enabled: true,
            layout_update_interval: chrono::Duration::milliseconds(100),
            energy_threshold: 0.1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use horizonos_graph_nodes::{BaseNode, NodeVisualData, NodeAction, NodeActionResult, NodeActionType, NodeError, NodeExportData};
    use horizonos_graph_engine::{SceneNode, NodeType, Vec3};
    
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
    
    #[test]
    fn test_layout_manager_creation() {
        let manager = LayoutManager::new();
        assert!(manager.layout_algorithms.contains_key("force_directed"));
        assert!(manager.auto_layout_enabled);
    }
    
    #[test]
    fn test_algorithm_registration() {
        let mut manager = LayoutManager::new();
        let algorithm = Box::new(ForceDirectedLayout::new());
        
        manager.register_algorithm("test_algorithm", algorithm);
        assert!(manager.layout_algorithms.contains_key("test_algorithm"));
    }
    
    #[test]
    fn test_layout_type_setting() {
        let mut manager = LayoutManager::new();
        let layout_type = LayoutType::ForceDirected {
            spring_strength: 0.1,
            repulsion_strength: 150.0,
            damping: 0.8,
        };
        
        manager.set_layout_type(layout_type.clone()).unwrap();
        assert_eq!(manager.current_layout, layout_type);
    }
    
    #[test]
    fn test_node_position_management() {
        let mut manager = LayoutManager::new();
        let position = Position::new(10.0, 20.0, 30.0);
        
        manager.set_node_position(1, position);
        assert_eq!(manager.get_node_position(1), Some(position));
        
        let all_positions = manager.get_all_positions();
        assert_eq!(all_positions.get(&1), Some(&position));
    }
    
    #[test]
    fn test_layout_statistics() {
        let manager = LayoutManager::new();
        let stats = manager.get_statistics();
        
        assert_eq!(stats.total_layouts_calculated, 0);
        assert_eq!(stats.convergence_rate, 0.0);
        assert!(stats.layout_type_usage.is_empty());
    }
    
    #[tokio::test]
    async fn test_cache_clearing() {
        let manager = LayoutManager::new();
        
        // Set some positions
        {
            let mut positions = manager.node_positions.write().unwrap();
            positions.insert(1, Position::new(1.0, 2.0, 3.0));
        }
        
        manager.clear_cache().await;
        
        let positions = manager.node_positions.read().unwrap();
        assert!(positions.is_empty());
    }
    
    #[tokio::test]
    async fn test_layout_event_handling() {
        let mut manager = LayoutManager::new();
        
        let event = LayoutUpdateEvent {
            event_type: LayoutEventType::NodesAdded,
            affected_nodes: vec![1, 2, 3],
            timestamp: chrono::Utc::now(),
        };
        
        let result = manager.handle_layout_event(event).await;
        assert!(result.is_ok());
    }
}