//! Node manager for the graph desktop

use crate::{GraphNode, ApplicationNode, FileNode, PersonNode, TaskNode, NodeError, NodeAction, NodeActionResult};
use horizonos_graph_engine::{SceneId, Scene};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Manages all nodes in the graph desktop
pub struct NodeManager {
    nodes: Arc<RwLock<HashMap<SceneId, Box<dyn GraphNode + Send + Sync>>>>,
    next_id: SceneId,
}

impl NodeManager {
    pub fn new() -> Self {
        NodeManager {
            nodes: Arc::new(RwLock::new(HashMap::new())),
            next_id: 1,
        }
    }
    
    /// Generate next unique ID
    pub fn next_id(&mut self) -> SceneId {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
    
    /// Add a node to the manager
    pub fn add_node(&mut self, node: Box<dyn GraphNode + Send + Sync>) -> Result<SceneId, NodeError> {
        let id = node.id();
        let mut nodes = self.nodes.write().unwrap();
        nodes.insert(id, node);
        Ok(id)
    }
    
    /// Get a node by ID
    pub fn get_node(&self, id: SceneId) -> Option<Box<dyn GraphNode + Send + Sync>> {
        let nodes = self.nodes.read().unwrap();
        // This is a simplified version - in reality we'd need more complex cloning
        None // Placeholder
    }
    
    /// Remove a node
    pub fn remove_node(&mut self, id: SceneId) -> Result<(), NodeError> {
        let mut nodes = self.nodes.write().unwrap();
        nodes.remove(&id);
        Ok(())
    }
    
    /// Update all nodes
    pub fn update_all(&mut self, delta_time: f32) -> Result<(), NodeError> {
        let mut nodes = self.nodes.write().unwrap();
        for node in nodes.values_mut() {
            node.update(delta_time)?;
        }
        Ok(())
    }
    
    /// Handle action on a specific node
    pub fn handle_node_action(&mut self, id: SceneId, action: NodeAction) -> Result<NodeActionResult, NodeError> {
        let mut nodes = self.nodes.write().unwrap();
        if let Some(node) = nodes.get_mut(&id) {
            node.handle_action(action)
        } else {
            Err(NodeError::NodeNotFound { id })
        }
    }
    
    /// Sync nodes to scene for rendering
    pub fn sync_to_scene(&self, scene: &mut Scene) {
        let nodes = self.nodes.read().unwrap();
        for node in nodes.values() {
            let scene_node = node.to_scene_node();
            // This would need proper scene update logic
        }
    }
    
    /// Create application node
    pub fn create_application(&mut self, name: String, executable: String) -> Result<SceneId, NodeError> {
        let id = self.next_id();
        let app_node = ApplicationNode::new(id, name, executable);
        self.add_node(Box::new(app_node))
    }
    
    /// Create file node
    pub fn create_file(&mut self, path: std::path::PathBuf) -> Result<SceneId, NodeError> {
        let id = self.next_id();
        let file_node = FileNode::new(id, path)?;
        self.add_node(Box::new(file_node))
    }
    
    /// Create person node
    pub fn create_person(&mut self, name: String) -> Result<SceneId, NodeError> {
        let id = self.next_id();
        let person_node = PersonNode::new(id, name);
        self.add_node(Box::new(person_node))
    }
    
    /// Create task node
    pub fn create_task(&mut self, title: String) -> Result<SceneId, NodeError> {
        let id = self.next_id();
        let task_node = TaskNode::new(id, title);
        self.add_node(Box::new(task_node))
    }
}

impl Default for NodeManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_manager_creation() {
        let mut manager = NodeManager::new();
        assert_eq!(manager.next_id(), 1);
        assert_eq!(manager.next_id(), 2);
    }

    #[test]
    fn test_create_application_node() {
        let mut manager = NodeManager::new();
        let id = manager.create_application("Test App".to_string(), "/bin/test".to_string()).unwrap();
        assert_eq!(id, 1);
    }
    
    #[test]
    fn test_create_person_node() {
        let mut manager = NodeManager::new();
        let id = manager.create_person("Alice".to_string()).unwrap();
        assert_eq!(id, 1);
    }
}