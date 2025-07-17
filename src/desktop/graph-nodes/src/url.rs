//! URL node implementation for web links and resources

use crate::{GraphNode, NodeVisualData, NodeAction, NodeActionResult, NodeActionType, NodeError, NodeExportData};
use horizonos_graph_engine::{SceneId, SceneNode, NodeMetadata};
use horizonos_graph_engine::scene::{NodeType, UrlType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// URL node for web links and resources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrlNode {
    /// Node ID
    pub id: SceneId,
    /// URL
    pub url: String,
    /// Display title
    pub title: Option<String>,
    /// Favicon URL
    pub favicon: Option<String>,
    /// Last visited timestamp
    pub last_visited: Option<chrono::DateTime<chrono::Utc>>,
    /// Visit count
    pub visit_count: u64,
    /// Bookmarked status
    pub bookmarked: bool,
    /// Node metadata
    pub metadata: NodeMetadata,
    /// Custom properties
    pub properties: HashMap<String, String>,
}

impl UrlNode {
    /// Create a new URL node
    pub fn new(id: SceneId, url: String) -> Self {
        Self {
            id,
            url,
            title: None,
            favicon: None,
            last_visited: None,
            visit_count: 0,
            bookmarked: false,
            metadata: NodeMetadata::default(),
            properties: HashMap::new(),
        }
    }

    /// Create a new URL node with title
    pub fn with_title(id: SceneId, url: String, title: String) -> Self {
        Self {
            id,
            url,
            title: Some(title),
            favicon: None,
            last_visited: None,
            visit_count: 0,
            bookmarked: false,
            metadata: NodeMetadata::default(),
            properties: HashMap::new(),
        }
    }

    /// Set favicon URL
    pub fn set_favicon(&mut self, favicon: String) {
        self.favicon = Some(favicon);
    }

    /// Mark as visited
    pub fn mark_visited(&mut self) {
        self.last_visited = Some(chrono::Utc::now());
        self.visit_count += 1;
    }

    /// Set bookmark status
    pub fn set_bookmarked(&mut self, bookmarked: bool) {
        self.bookmarked = bookmarked;
    }

    /// Get domain from URL
    pub fn get_domain(&self) -> Option<String> {
        if let Ok(parsed) = url::Url::parse(&self.url) {
            parsed.domain().map(|d| d.to_string())
        } else {
            None
        }
    }

    /// Get protocol from URL
    pub fn get_protocol(&self) -> Option<String> {
        if let Ok(parsed) = url::Url::parse(&self.url) {
            Some(parsed.scheme().to_string())
        } else {
            None
        }
    }

    /// Check if URL is secure (HTTPS)
    pub fn is_secure(&self) -> bool {
        self.url.starts_with("https://")
    }

    /// Get URL path
    pub fn get_path(&self) -> Option<String> {
        if let Ok(parsed) = url::Url::parse(&self.url) {
            Some(parsed.path().to_string())
        } else {
            None
        }
    }

    /// Get query parameters
    pub fn get_query_params(&self) -> HashMap<String, String> {
        let mut params = HashMap::new();
        
        if let Ok(parsed) = url::Url::parse(&self.url) {
            for (key, value) in parsed.query_pairs() {
                params.insert(key.to_string(), value.to_string());
            }
        }
        
        params
    }
}

impl GraphNode for UrlNode {
    fn id(&self) -> SceneId {
        self.id
    }

    fn display_name(&self) -> String {
        self.title.clone().unwrap_or_else(|| self.url.clone())
    }
    
    fn description(&self) -> Option<String> {
        Some(format!("URL: {}", self.url))
    }
    
    fn visual_data(&self) -> NodeVisualData {
        let mut visual_data = NodeVisualData::default();
        visual_data.color = if self.bookmarked {
            [1.0, 0.8, 0.0, 1.0] // Gold for bookmarked
        } else if self.is_secure() {
            [0.0, 0.8, 0.0, 1.0] // Green for secure
        } else {
            [0.6, 0.6, 0.6, 1.0] // Gray for regular
        };
        visual_data.radius = 1.0;
        visual_data.icon = Some("link".to_string());
        visual_data
    }
    
    fn update(&mut self, _delta_time: f32) -> Result<(), NodeError> {
        Ok(())
    }
    
    fn handle_action(&mut self, action: NodeAction) -> Result<NodeActionResult, NodeError> {
        match action {
            NodeAction::Open => {
                Ok(NodeActionResult::Success { 
                    message: Some(format!("Opening URL: {}", self.url)) 
                })
            }
            NodeAction::Edit => {
                Ok(NodeActionResult::Success { 
                    message: Some("URL properties opened for editing".to_string()) 
                })
            }
            _ => Err(NodeError::InvalidAction { action })
        }
    }
    
    fn available_actions(&self) -> Vec<NodeActionType> {
        vec![
            NodeActionType::Open,
            NodeActionType::Edit,
            NodeActionType::Copy,
        ]
    }
    
    fn export_data(&self) -> Result<NodeExportData, NodeError> {
        Ok(NodeExportData {
            node_type: "URL".to_string(),
            display_name: self.display_name(),
            description: self.description(),
            visual_data: self.visual_data(),
            metadata: self.metadata.clone(),
            type_specific_data: serde_json::to_value(self)?,
        })
    }

    fn to_scene_node(&self) -> SceneNode {
        SceneNode {
            id: self.id,
            position: [0.0, 0.0, 0.0].into(),
            velocity: nalgebra::Vector3::zeros(),
            radius: 1.0,
            color: if self.bookmarked {
                [1.0, 0.8, 0.0, 1.0] // Gold for bookmarked
            } else if self.is_secure() {
                [0.0, 0.8, 0.0, 1.0] // Green for secure
            } else {
                [0.6, 0.6, 0.6, 1.0] // Gray for regular
            },
            node_type: NodeType::URL {
                url: self.url.clone(),
                title: self.title.clone(),
                url_type: UrlType::Website,
            },
            metadata: self.metadata.clone(),
            visible: true,
            selected: false,
        }
    }
}