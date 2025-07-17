//! Device node implementation

use crate::{GraphNode, BaseNode, NodeVisualData, NodeAction, NodeActionResult, NodeActionType, NodeError, NodeExportData};
use horizonos_graph_engine::{SceneNode, NodeType, NodeMetadata, DeviceType, SceneId, Position, Vec3};

#[derive(Debug, Clone)]
pub struct DeviceNode {
    base: BaseNode,
}

impl DeviceNode {
    pub fn new(id: SceneId) -> Self {
        DeviceNode {
            base: BaseNode::new(id),
        }
    }
}

impl GraphNode for DeviceNode {
    fn id(&self) -> SceneId { self.base.id }
    fn display_name(&self) -> String { "Device".to_string() }
    fn description(&self) -> Option<String> { Some("Device node".to_string()) }
    fn node_type(&self) -> NodeType {
        NodeType::Device {
            name: "Generic Device".to_string(),
            device_type: DeviceType::Computer,
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
            node_type: "device".to_string(),
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
            node_type: NodeType::Device { name: "Device".to_string(), device_type: DeviceType::Computer },
            metadata: self.base.metadata.clone(),
            visible: true,
            selected: false,
        }
    }
}