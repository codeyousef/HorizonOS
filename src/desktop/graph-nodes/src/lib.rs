//! Node system for HorizonOS graph desktop
//! 
//! This module provides concrete implementations and management for all node types
//! in the graph desktop environment.

pub use horizonos_graph_engine::{SceneNode, NodeType, NodeMetadata, SceneId, EdgeType};

pub mod application;
pub mod file;
pub mod person;
pub mod task;
pub mod device;
pub mod ai_agent;
pub mod concept;
pub mod system;
pub mod manager;
pub mod url;
pub mod automation;
pub mod setting;
pub mod config_group;

pub use application::*;
pub use file::*;
pub use person::*;
pub use task::*;
pub use device::*;
pub use ai_agent::*;
pub use concept::*;
pub use system::*;
pub use manager::*;
pub use url::*;
pub use automation::*;
pub use setting::*;
pub use config_group::*;

use std::collections::HashMap;
use serde::{Serialize, Deserialize};

/// Trait that all node implementations must implement
pub trait GraphNode: Send + Sync {
    /// Get the unique identifier for this node
    fn id(&self) -> SceneId;
    
    /// Get the display name for this node
    fn display_name(&self) -> String;
    
    /// Get a description of this node
    fn description(&self) -> Option<String>;
    
    /// Get the visual representation data
    fn visual_data(&self) -> NodeVisualData;
    
    /// Update the node's internal state
    fn update(&mut self, delta_time: f32) -> Result<(), NodeError>;
    
    /// Handle an action on this node
    fn handle_action(&mut self, action: NodeAction) -> Result<NodeActionResult, NodeError>;
    
    /// Get available actions for this node
    fn available_actions(&self) -> Vec<NodeActionType>;
    
    /// Export node data for serialization
    fn export_data(&self) -> Result<NodeExportData, NodeError>;
    
    /// Convert to scene node for rendering
    fn to_scene_node(&self) -> SceneNode;
}

/// Visual representation data for a node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeVisualData {
    pub position: [f32; 3],
    pub radius: f32,
    pub color: [f32; 4],
    pub icon: Option<String>,
    pub badge: Option<String>,
    pub glow: bool,
    pub selected: bool,
    pub visible: bool,
}

impl Default for NodeVisualData {
    fn default() -> Self {
        NodeVisualData {
            position: [0.0, 0.0, 0.0],
            radius: 1.0,
            color: [0.5, 0.5, 0.5, 1.0],
            icon: None,
            badge: None,
            glow: false,
            selected: false,
            visible: true,
        }
    }
}

/// Actions that can be performed on nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeAction {
    /// Open/activate the node
    Open,
    /// Edit the node's properties
    Edit,
    /// Delete the node
    Delete,
    /// Copy the node
    Copy,
    /// Move the node to a position
    MoveTo { position: [f32; 3] },
    /// Connect to another node
    ConnectTo { target_id: SceneId, edge_type: EdgeType },
    /// Disconnect from another node
    DisconnectFrom { target_id: SceneId },
    /// Custom action with parameters
    Custom { action_type: String, parameters: HashMap<String, String> },
}

/// Types of actions available for nodes
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeActionType {
    Open,
    Edit,
    Delete,
    Copy,
    Move,
    Connect,
    Disconnect,
    Custom(String),
}

/// Result of performing an action on a node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeActionResult {
    /// Action completed successfully
    Success { message: Option<String> },
    /// Action failed with error message
    Error { error: String },
    /// Action requires confirmation
    ConfirmationRequired { prompt: String },
    /// Action spawned another node
    NodeSpawned { node_id: SceneId },
    /// Action modified relationships
    RelationshipChanged { 
        target_id: SceneId, 
        edge_type: EdgeType, 
        added: bool 
    },
}

/// Data exported from a node for serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeExportData {
    pub node_type: String,
    pub display_name: String,
    pub description: Option<String>,
    pub visual_data: NodeVisualData,
    pub metadata: NodeMetadata,
    pub type_specific_data: serde_json::Value,
}

/// Errors that can occur during node operations
#[derive(Debug, thiserror::Error)]
pub enum NodeError {
    #[error("Invalid action for node type: {action:?}")]
    InvalidAction { action: NodeAction },
    
    #[error("Node not found: {id}")]
    NodeNotFound { id: SceneId },
    
    #[error("Permission denied: {operation}")]
    PermissionDenied { operation: String },
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    
    #[error("System error: {message}")]
    SystemError { message: String },
}

/// Base node implementation that provides common functionality
#[derive(Debug, Clone)]
pub struct BaseNode {
    pub id: SceneId,
    pub visual_data: NodeVisualData,
    pub metadata: NodeMetadata,
    pub last_update: std::time::Instant,
}

impl BaseNode {
    pub fn new(id: SceneId) -> Self {
        BaseNode {
            id,
            visual_data: NodeVisualData::default(),
            metadata: NodeMetadata::default(),
            last_update: std::time::Instant::now(),
        }
    }
    
    pub fn with_position(mut self, position: [f32; 3]) -> Self {
        self.visual_data.position = position;
        self
    }
    
    pub fn with_color(mut self, color: [f32; 4]) -> Self {
        self.visual_data.color = color;
        self
    }
    
    pub fn with_radius(mut self, radius: f32) -> Self {
        self.visual_data.radius = radius;
        self
    }
    
    pub fn update_timestamp(&mut self) {
        self.last_update = std::time::Instant::now();
        self.metadata.updated_at = chrono::Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_node_creation() {
        let node = BaseNode::new(123)
            .with_position([1.0, 2.0, 3.0])
            .with_color([0.8, 0.2, 0.2, 1.0])
            .with_radius(1.5);
            
        assert_eq!(node.id, 123);
        assert_eq!(node.visual_data.position, [1.0, 2.0, 3.0]);
        assert_eq!(node.visual_data.color, [0.8, 0.2, 0.2, 1.0]);
        assert_eq!(node.visual_data.radius, 1.5);
    }

    #[test]
    fn test_node_action_types() {
        let actions = vec![
            NodeActionType::Open,
            NodeActionType::Edit,
            NodeActionType::Delete,
            NodeActionType::Custom("test".to_string()),
        ];
        
        assert_eq!(actions.len(), 4);
        assert!(actions.contains(&NodeActionType::Open));
        assert!(actions.contains(&NodeActionType::Custom("test".to_string())));
    }
}