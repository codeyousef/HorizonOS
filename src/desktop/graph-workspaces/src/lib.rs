//! Workspace management system for the graph desktop
//! 
//! Provides workspace organization, switching, and persistence

use horizonos_graph_engine::scene::SceneId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;
use chrono::{DateTime, Utc};

pub mod layout;
pub mod persistence;
pub mod rules;
pub mod templates;

use layout::WorkspaceLayout;
use persistence::WorkspacePersistence;
use rules::WorkspaceRules;
use templates::WorkspaceTemplate;

/// Workspace manager for organizing graph desktop sessions
pub struct WorkspaceManager {
    /// All workspaces
    workspaces: Arc<RwLock<HashMap<String, Workspace>>>,
    /// Currently active workspace
    active_workspace: Arc<RwLock<Option<String>>>,
    /// Workspace event broadcaster
    event_sender: broadcast::Sender<WorkspaceEvent>,
    /// Persistence handler
    persistence: WorkspacePersistence,
    /// Workspace rules engine
    rules: WorkspaceRules,
}

impl WorkspaceManager {
    /// Create a new workspace manager
    pub fn new() -> Self {
        let (event_sender, _) = broadcast::channel(100);
        
        Self {
            workspaces: Arc::new(RwLock::new(HashMap::new())),
            active_workspace: Arc::new(RwLock::new(None)),
            event_sender,
            persistence: WorkspacePersistence::new(),
            rules: WorkspaceRules::new(),
        }
    }
    
    /// Initialize the workspace manager
    pub async fn initialize(&mut self) -> Result<(), WorkspaceError> {
        // Load saved workspaces
        let saved_workspaces = self.persistence.load_workspaces().await?;
        
        let mut workspaces = self.workspaces.write().unwrap();
        for workspace in saved_workspaces {
            workspaces.insert(workspace.id.clone(), workspace);
        }
        
        // Create default workspace if none exist
        if workspaces.is_empty() {
            let default_workspace = Workspace::new("default", "Default Workspace");
            workspaces.insert(default_workspace.id.clone(), default_workspace);
        }
        
        // Set the first workspace as active
        if let Some(first_id) = workspaces.keys().next().cloned() {
            *self.active_workspace.write().unwrap() = Some(first_id);
        }
        
        Ok(())
    }
    
    /// Create a new workspace
    pub fn create_workspace(&self, name: &str, description: &str) -> Result<String, WorkspaceError> {
        let mut workspace = Workspace::new(name, description);
        workspace.layout = WorkspaceLayout::default();
        
        let workspace_id = workspace.id.clone();
        
        self.workspaces.write().unwrap()
            .insert(workspace_id.clone(), workspace);
        
        self.event_sender.send(WorkspaceEvent::Created {
            workspace_id: workspace_id.clone(),
        }).ok();
        
        Ok(workspace_id)
    }
    
    /// Create workspace from template
    pub fn create_from_template(&self, template: WorkspaceTemplate) -> Result<String, WorkspaceError> {
        let workspace = template.instantiate();
        let workspace_id = workspace.id.clone();
        
        self.workspaces.write().unwrap()
            .insert(workspace_id.clone(), workspace);
        
        self.event_sender.send(WorkspaceEvent::Created {
            workspace_id: workspace_id.clone(),
        }).ok();
        
        Ok(workspace_id)
    }
    
    /// Switch to a different workspace
    pub fn switch_workspace(&self, workspace_id: &str) -> Result<(), WorkspaceError> {
        let workspaces = self.workspaces.read().unwrap();
        
        if !workspaces.contains_key(workspace_id) {
            return Err(WorkspaceError::NotFound(workspace_id.to_string()));
        }
        
        let previous = self.active_workspace.read().unwrap().clone();
        *self.active_workspace.write().unwrap() = Some(workspace_id.to_string());
        
        self.event_sender.send(WorkspaceEvent::Switched {
            from: previous,
            to: workspace_id.to_string(),
        }).ok();
        
        Ok(())
    }
    
    /// Get the active workspace
    pub fn get_active_workspace(&self) -> Option<Workspace> {
        let active_id = self.active_workspace.read().unwrap();
        
        if let Some(id) = active_id.as_ref() {
            self.workspaces.read().unwrap().get(id).cloned()
        } else {
            None
        }
    }
    
    /// Get a specific workspace
    pub fn get_workspace(&self, workspace_id: &str) -> Option<Workspace> {
        self.workspaces.read().unwrap().get(workspace_id).cloned()
    }
    
    /// List all workspaces
    pub fn list_workspaces(&self) -> Vec<WorkspaceInfo> {
        self.workspaces.read().unwrap()
            .values()
            .map(|w| WorkspaceInfo {
                id: w.id.clone(),
                name: w.name.clone(),
                description: w.description.clone(),
                created_at: w.created_at,
                last_accessed: w.last_accessed,
                node_count: w.nodes.len(),
                is_active: self.active_workspace.read().unwrap()
                    .as_ref()
                    .map(|id| id == &w.id)
                    .unwrap_or(false),
            })
            .collect()
    }
    
    /// Delete a workspace
    pub fn delete_workspace(&self, workspace_id: &str) -> Result<(), WorkspaceError> {
        let mut workspaces = self.workspaces.write().unwrap();
        
        if !workspaces.contains_key(workspace_id) {
            return Err(WorkspaceError::NotFound(workspace_id.to_string()));
        }
        
        // Don't delete the last workspace
        if workspaces.len() == 1 {
            return Err(WorkspaceError::CannotDeleteLast);
        }
        
        // Switch to another workspace if deleting the active one
        let active = self.active_workspace.read().unwrap();
        if active.as_ref() == Some(&workspace_id.to_string()) {
            drop(active);
            
            // Find another workspace to switch to
            if let Some(other_id) = workspaces.keys()
                .find(|id| *id != workspace_id)
                .cloned()
            {
                *self.active_workspace.write().unwrap() = Some(other_id);
            }
        }
        
        workspaces.remove(workspace_id);
        
        self.event_sender.send(WorkspaceEvent::Deleted {
            workspace_id: workspace_id.to_string(),
        }).ok();
        
        Ok(())
    }
    
    /// Save all workspaces
    pub async fn save_workspaces(&self) -> Result<(), WorkspaceError> {
        let workspaces: Vec<Workspace> = self.workspaces.read().unwrap()
            .values()
            .cloned()
            .collect();
        
        self.persistence.save_workspaces(&workspaces).await?;
        Ok(())
    }
    
    /// Subscribe to workspace events
    pub fn subscribe(&self) -> broadcast::Receiver<WorkspaceEvent> {
        self.event_sender.subscribe()
    }
    
    /// Apply rules to organize workspace
    pub fn apply_rules(&self, workspace_id: &str) -> Result<(), WorkspaceError> {
        let mut workspaces = self.workspaces.write().unwrap();
        
        if let Some(workspace) = workspaces.get_mut(workspace_id) {
            self.rules.apply_to_workspace(workspace);
            Ok(())
        } else {
            Err(WorkspaceError::NotFound(workspace_id.to_string()))
        }
    }
}

/// Individual workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    /// Unique workspace ID
    pub id: String,
    /// Workspace name
    pub name: String,
    /// Workspace description
    pub description: String,
    /// Nodes in this workspace
    pub nodes: Vec<SceneId>,
    /// Workspace layout configuration
    pub layout: WorkspaceLayout,
    /// Workspace-specific settings
    pub settings: WorkspaceSettings,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last accessed timestamp
    pub last_accessed: DateTime<Utc>,
    /// Custom metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Workspace {
    /// Create a new workspace
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            description: description.to_string(),
            nodes: Vec::new(),
            layout: WorkspaceLayout::default(),
            settings: WorkspaceSettings::default(),
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            metadata: HashMap::new(),
        }
    }
    
    /// Add a node to the workspace
    pub fn add_node(&mut self, node_id: SceneId) {
        if !self.nodes.contains(&node_id) {
            self.nodes.push(node_id);
            self.last_accessed = Utc::now();
        }
    }
    
    /// Remove a node from the workspace
    pub fn remove_node(&mut self, node_id: SceneId) {
        self.nodes.retain(|id| id != &node_id);
        self.last_accessed = Utc::now();
    }
    
    /// Update last accessed time
    pub fn touch(&mut self) {
        self.last_accessed = Utc::now();
    }
}

/// Workspace settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSettings {
    /// Background color
    pub background_color: [f32; 4],
    /// Grid visibility
    pub show_grid: bool,
    /// Grid size
    pub grid_size: f32,
    /// Auto-save enabled
    pub auto_save: bool,
    /// Auto-arrange enabled
    pub auto_arrange: bool,
    /// Default node spacing
    pub node_spacing: f32,
}

impl Default for WorkspaceSettings {
    fn default() -> Self {
        Self {
            background_color: [0.1, 0.1, 0.1, 1.0],
            show_grid: true,
            grid_size: 20.0,
            auto_save: true,
            auto_arrange: false,
            node_spacing: 100.0,
        }
    }
}

/// Workspace information summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
    pub node_count: usize,
    pub is_active: bool,
}

/// Workspace events
#[derive(Debug, Clone)]
pub enum WorkspaceEvent {
    Created { workspace_id: String },
    Deleted { workspace_id: String },
    Switched { from: Option<String>, to: String },
    Modified { workspace_id: String },
    NodeAdded { workspace_id: String, node_id: SceneId },
    NodeRemoved { workspace_id: String, node_id: SceneId },
}

/// Workspace errors
#[derive(Debug, thiserror::Error)]
pub enum WorkspaceError {
    #[error("Workspace not found: {0}")]
    NotFound(String),
    
    #[error("Cannot delete the last workspace")]
    CannotDeleteLast,
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

// Re-export uuid for workspace IDs
pub use uuid;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_workspace_creation() {
        let manager = WorkspaceManager::new();
        
        let workspace_id = manager.create_workspace("Test", "Test workspace").unwrap();
        
        let workspace = manager.get_workspace(&workspace_id).unwrap();
        assert_eq!(workspace.name, "Test");
        assert_eq!(workspace.description, "Test workspace");
    }
    
    #[tokio::test]
    async fn test_workspace_switching() {
        let manager = WorkspaceManager::new();
        
        let workspace1 = manager.create_workspace("Work", "Work workspace").unwrap();
        let workspace2 = manager.create_workspace("Personal", "Personal workspace").unwrap();
        
        manager.switch_workspace(&workspace1).unwrap();
        assert_eq!(
            manager.get_active_workspace().unwrap().id,
            workspace1
        );
        
        manager.switch_workspace(&workspace2).unwrap();
        assert_eq!(
            manager.get_active_workspace().unwrap().id,
            workspace2
        );
    }
}