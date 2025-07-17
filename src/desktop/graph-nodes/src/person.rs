//! Person/Contact node implementation

use crate::{GraphNode, BaseNode, NodeVisualData, NodeAction, NodeActionResult, NodeActionType, NodeError, NodeExportData};
use horizonos_graph_engine::{SceneNode, NodeType, NodeMetadata, ContactInfo, SceneId, Position, Vec3};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct PersonNode {
    base: BaseNode,
    person_data: PersonData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonData {
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub social_media: HashMap<String, String>,
    pub avatar_url: Option<String>,
    pub organization: Option<String>,
    pub role: Option<String>,
    pub last_contacted: Option<chrono::DateTime<chrono::Utc>>,
    pub contact_frequency: u32,
}

impl PersonNode {
    pub fn new(id: SceneId, name: String) -> Self {
        let mut base = BaseNode::new(id)
            .with_color([0.2, 0.8, 0.6, 1.0]) // Teal for people
            .with_radius(1.1);
        
        base.metadata.tags.push("person".to_string());
        base.metadata.description = Some(format!("Contact: {}", name));
        base.visual_data.icon = Some("👤".to_string());
        
        PersonNode {
            base,
            person_data: PersonData {
                name,
                email: None,
                phone: None,
                social_media: HashMap::new(),
                avatar_url: None,
                organization: None,
                role: None,
                last_contacted: None,
                contact_frequency: 0,
            },
        }
    }
}

impl GraphNode for PersonNode {
    fn id(&self) -> SceneId { self.base.id }
    fn display_name(&self) -> String { self.person_data.name.clone() }
    fn description(&self) -> Option<String> { 
        Some(format!("Contact: {}", self.person_data.name))
    }
    fn node_type(&self) -> NodeType {
        NodeType::Person {
            name: self.person_data.name.clone(),
            contact_info: horizonos_graph_engine::ContactInfo {
                email: self.person_data.email.clone(),
                phone: self.person_data.phone.clone(),
                social: std::collections::HashMap::new(),
            },
        }
    }
    fn metadata(&self) -> NodeMetadata { self.base.metadata.clone() }
    fn visual_data(&self) -> NodeVisualData { self.base.visual_data.clone() }
    fn update(&mut self, _delta_time: f32) -> Result<(), NodeError> { Ok(()) }
    fn handle_action(&mut self, action: NodeAction) -> Result<NodeActionResult, NodeError> {
        match action {
            NodeAction::Open => Ok(NodeActionResult::Success { message: Some("Contact opened".to_string()) }),
            _ => Err(NodeError::InvalidAction { action }),
        }
    }
    fn available_actions(&self) -> Vec<NodeActionType> {
        vec![NodeActionType::Open, NodeActionType::Edit, NodeActionType::Delete]
    }
    fn export_data(&self) -> Result<NodeExportData, NodeError> {
        Ok(NodeExportData {
            node_type: "person".to_string(),
            display_name: self.display_name(),
            description: self.description(),
            visual_data: self.visual_data(),
            metadata: self.base.metadata.clone(),
            type_specific_data: serde_json::to_value(&self.person_data)?,
        })
    }
    fn to_scene_node(&self) -> SceneNode {
        SceneNode {
            id: self.base.id,
            position: Position::new(self.base.visual_data.position[0], self.base.visual_data.position[1], self.base.visual_data.position[2]),
            velocity: Vec3::zeros(),
            radius: self.base.visual_data.radius,
            color: self.base.visual_data.color,
            node_type: NodeType::Person {
                name: self.person_data.name.clone(),
                contact_info: ContactInfo {
                    email: self.person_data.email.clone(),
                    phone: self.person_data.phone.clone(),
                    social: self.person_data.social_media.clone(),
                },
            },
            metadata: self.base.metadata.clone(),
            visible: self.base.visual_data.visible,
            selected: self.base.visual_data.selected,
        }
    }
}