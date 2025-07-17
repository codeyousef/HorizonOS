//! Task node implementation

use crate::{GraphNode, BaseNode, NodeVisualData, NodeAction, NodeActionResult, NodeActionType, NodeError, NodeExportData};
use horizonos_graph_engine::{SceneNode, NodeType, NodeMetadata, TaskStatus, SceneId, Position, Vec3};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone)]
pub struct TaskNode {
    base: BaseNode,
    task_data: TaskData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskData {
    pub title: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub due_date: Option<chrono::DateTime<chrono::Utc>>,
    pub created_date: chrono::DateTime<chrono::Utc>,
    pub completed_date: Option<chrono::DateTime<chrono::Utc>>,
    pub estimated_duration: Option<chrono::Duration>,
    pub actual_duration: Option<chrono::Duration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskPriority {
    Low,
    Medium,
    High,
    Critical,
}

impl TaskNode {
    pub fn new(id: SceneId, title: String) -> Self {
        let mut base = BaseNode::new(id)
            .with_color([0.9, 0.6, 0.2, 1.0]) // Orange for tasks
            .with_radius(1.0);
        
        base.metadata.tags.push("task".to_string());
        base.metadata.description = Some(format!("Task: {}", title));
        base.visual_data.icon = Some("📋".to_string());
        
        TaskNode {
            base,
            task_data: TaskData {
                title,
                description: None,
                status: TaskStatus::Todo,
                priority: TaskPriority::Medium,
                due_date: None,
                created_date: chrono::Utc::now(),
                completed_date: None,
                estimated_duration: None,
                actual_duration: None,
            },
        }
    }
}

impl GraphNode for TaskNode {
    fn id(&self) -> SceneId { self.base.id }
    fn display_name(&self) -> String { self.task_data.title.clone() }
    fn description(&self) -> Option<String> { 
        Some(format!("Task: {} - {:?}", self.task_data.title, self.task_data.status))
    }
    fn node_type(&self) -> NodeType {
        NodeType::Task {
            title: self.task_data.title.clone(),
            status: self.task_data.status.clone(),
        }
    }
    fn metadata(&self) -> NodeMetadata { self.base.metadata.clone() }
    fn visual_data(&self) -> NodeVisualData { 
        let mut visual = self.base.visual_data.clone();
        visual.badge = match self.task_data.status {
            TaskStatus::Todo => Some("⏳".to_string()),
            TaskStatus::InProgress => Some("🔄".to_string()),
            TaskStatus::Completed => Some("✅".to_string()),
            TaskStatus::Cancelled => Some("❌".to_string()),
        };
        visual
    }
    fn update(&mut self, _delta_time: f32) -> Result<(), NodeError> { Ok(()) }
    fn handle_action(&mut self, action: NodeAction) -> Result<NodeActionResult, NodeError> {
        match action {
            NodeAction::Open => Ok(NodeActionResult::Success { message: Some("Task opened".to_string()) }),
            _ => Err(NodeError::InvalidAction { action }),
        }
    }
    fn available_actions(&self) -> Vec<NodeActionType> {
        vec![NodeActionType::Open, NodeActionType::Edit, NodeActionType::Delete]
    }
    fn export_data(&self) -> Result<NodeExportData, NodeError> {
        Ok(NodeExportData {
            node_type: "task".to_string(),
            display_name: self.display_name(),
            description: self.description(),
            visual_data: self.visual_data(),
            metadata: self.base.metadata.clone(),
            type_specific_data: serde_json::to_value(&self.task_data)?,
        })
    }
    fn to_scene_node(&self) -> SceneNode {
        SceneNode {
            id: self.base.id,
            position: Position::new(self.base.visual_data.position[0], self.base.visual_data.position[1], self.base.visual_data.position[2]),
            velocity: Vec3::zeros(),
            radius: self.base.visual_data.radius,
            color: self.base.visual_data.color,
            node_type: NodeType::Task {
                title: self.task_data.title.clone(),
                status: self.task_data.status.clone(),
            },
            metadata: self.base.metadata.clone(),
            visible: self.base.visual_data.visible,
            selected: self.base.visual_data.selected,
        }
    }
}