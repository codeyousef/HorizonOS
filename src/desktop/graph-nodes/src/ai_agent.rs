//! AI Agent node implementation

use crate::{GraphNode, BaseNode, NodeVisualData, NodeAction, NodeActionResult, NodeActionType, NodeError, NodeExportData};
use horizonos_graph_engine::{SceneNode, NodeType, NodeMetadata, SceneId, Position, Vec3};

#[derive(Debug, Clone)]
pub struct AIAgentNode {
    base: BaseNode,
}

impl AIAgentNode {
    pub fn new(id: SceneId) -> Self {
        AIAgentNode {
            base: BaseNode::new(id),
        }
    }
}

impl GraphNode for AIAgentNode {
    fn id(&self) -> SceneId { self.base.id }
    fn display_name(&self) -> String { "AI Agent".to_string() }
    fn description(&self) -> Option<String> { Some("AI Agent node".to_string()) }
    fn node_type(&self) -> NodeType {
        NodeType::AIAgent {
            name: "Generic AI Agent".to_string(),
            model: "unknown".to_string(),
        }
    }
    fn metadata(&self) -> NodeMetadata { self.base.metadata.clone() }
    fn visual_data(&self) -> NodeVisualData { self.base.visual_data.clone() }
    fn update(&mut self, _delta_time: f32) -> Result<(), NodeError> { Ok(()) }
    fn handle_action(&mut self, action: NodeAction) -> Result<NodeActionResult, NodeError> {
        Err(NodeError::InvalidAction { action })
    }
    fn available_actions(&self) -> Vec<NodeActionType> { vec![] }
    fn export_data(&self) -> Result<NodeExportData, NodeError> {
        Ok(NodeExportData {
            node_type: "ai_agent".to_string(),
            display_name: self.display_name(),
            description: self.description(),
            visual_data: self.visual_data(),
            metadata: self.base.metadata.clone(),
            type_specific_data: serde_json::Value::Null,
        })
    }
    fn to_scene_node(&self) -> SceneNode {
        SceneNode {
            id: self.base.id,
            position: Position::new(0.0, 0.0, 0.0),
            velocity: Vec3::zeros(),
            radius: 1.0,
            color: [0.5, 0.5, 0.5, 1.0],
            node_type: NodeType::AIAgent { name: "AI Agent".to_string(), model: "local".to_string() },
            metadata: self.base.metadata.clone(),
            visible: true,
            selected: false,
        }
    }
}