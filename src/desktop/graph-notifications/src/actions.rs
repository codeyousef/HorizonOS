//! Notification actions

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Notification action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationAction {
    /// Action ID
    pub id: String,
    /// Display label
    pub label: String,
    /// Icon
    pub icon: Option<String>,
    /// Action type
    pub action_type: ActionType,
    /// Additional parameters
    pub parameters: HashMap<String, String>,
    /// Destructive action (shows in red)
    pub destructive: bool,
    /// Primary action (default)
    pub primary: bool,
}

/// Action types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionType {
    /// Open URL
    OpenUrl { url: String },
    /// Run command
    RunCommand { command: String, args: Vec<String> },
    /// Open file
    OpenFile { path: String },
    /// Navigate to node
    NavigateToNode { node_id: u64 },
    /// Dismiss notification
    Dismiss,
    /// Snooze notification
    Snooze { duration: std::time::Duration },
    /// Custom action
    Custom { handler: String },
}

impl NotificationAction {
    /// Create open URL action
    pub fn open_url(label: String, url: String) -> Self {
        Self {
            id: format!("open_url_{}", uuid::Uuid::new_v4()),
            label,
            icon: Some("link".to_string()),
            action_type: ActionType::OpenUrl { url },
            parameters: HashMap::new(),
            destructive: false,
            primary: false,
        }
    }
    
    /// Create dismiss action
    pub fn dismiss() -> Self {
        Self {
            id: "dismiss".to_string(),
            label: "Dismiss".to_string(),
            icon: Some("x".to_string()),
            action_type: ActionType::Dismiss,
            parameters: HashMap::new(),
            destructive: false,
            primary: false,
        }
    }
    
    /// Create navigate to node action
    pub fn navigate_to_node(label: String, node_id: u64) -> Self {
        Self {
            id: format!("navigate_{}", node_id),
            label,
            icon: Some("arrow-right".to_string()),
            action_type: ActionType::NavigateToNode { node_id },
            parameters: HashMap::new(),
            destructive: false,
            primary: true,
        }
    }
}