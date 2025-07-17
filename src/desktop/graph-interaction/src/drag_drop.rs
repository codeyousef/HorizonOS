//! Drag and drop handling for nodes

use horizonos_graph_engine::{GraphEngine, SceneId, Position};
use nalgebra::Vector3;
use std::collections::HashMap;

/// Handles drag and drop operations
pub struct DragDropHandler {
    /// Currently dragged node
    dragged_node: Option<SceneId>,
    /// Drag start position (screen space)
    drag_start_pos: Option<(f32, f32)>,
    /// Original node positions (for multi-selection drag)
    original_positions: HashMap<SceneId, Position>,
    /// Drag offset from node center
    drag_offset: (f32, f32),
    /// Whether drag is active
    is_dragging: bool,
}

impl DragDropHandler {
    /// Create a new drag drop handler
    pub fn new() -> Self {
        Self {
            dragged_node: None,
            drag_start_pos: None,
            original_positions: HashMap::new(),
            drag_offset: (0.0, 0.0),
            is_dragging: false,
        }
    }
    
    /// Start dragging a node
    pub fn start_drag(&mut self, node_id: SceneId, screen_pos: (f32, f32)) {
        self.dragged_node = Some(node_id);
        self.drag_start_pos = Some(screen_pos);
        self.is_dragging = true;
        
        // TODO: Calculate drag offset from node center
        self.drag_offset = (0.0, 0.0);
    }
    
    /// Update drag position
    pub fn update_drag(&mut self, node_id: SceneId, screen_pos: (f32, f32), engine: &mut GraphEngine) {
        if !self.is_dragging || self.dragged_node != Some(node_id) {
            return;
        }
        
        // Calculate delta from start position
        if let Some(start_pos) = self.drag_start_pos {
            let delta = (
                screen_pos.0 - start_pos.0,
                screen_pos.1 - start_pos.1,
            );
            
            // Convert screen delta to world space
            // This is simplified - proper implementation would use camera projection
            let world_delta = Vector3::new(
                delta.0 * 0.01, // Scale factor
                -delta.1 * 0.01, // Invert Y
                0.0,
            );
            
            // Update node position
            if let Some(node) = engine.scene_mut().get_node_mut(node_id) {
                if let Some(original_pos) = self.original_positions.get(&node_id) {
                    node.position = *original_pos + world_delta;
                } else {
                    // Store original position on first update
                    self.original_positions.insert(node_id, node.position);
                    node.position = node.position + world_delta;
                }
            }
        }
    }
    
    /// End drag operation
    pub fn end_drag(&mut self) {
        self.dragged_node = None;
        self.drag_start_pos = None;
        self.original_positions.clear();
        self.is_dragging = false;
    }
    
    /// Check if currently dragging
    pub fn is_dragging(&self) -> bool {
        self.is_dragging
    }
    
    /// Get the currently dragged node
    pub fn get_dragged_node(&self) -> Option<SceneId> {
        self.dragged_node
    }
    
    /// Handle drag over event (for drop targets)
    pub fn handle_drag_over(&self, target_node: SceneId, screen_pos: (f32, f32)) -> DragOverResult {
        if !self.is_dragging {
            return DragOverResult::None;
        }
        
        // Check if we can drop on this target
        // This is a simplified check - real implementation would check node types, etc.
        if self.dragged_node != Some(target_node) {
            DragOverResult::Accept
        } else {
            DragOverResult::Reject
        }
    }
    
    /// Handle drop event
    pub fn handle_drop(&mut self, target_node: SceneId, engine: &mut GraphEngine) -> Option<DropAction> {
        if !self.is_dragging {
            return None;
        }
        
        if let Some(source_node) = self.dragged_node {
            if source_node != target_node {
                // Perform drop action
                let action = DropAction::CreateEdge {
                    source: source_node,
                    target: target_node,
                };
                
                self.end_drag();
                return Some(action);
            }
        }
        
        self.end_drag();
        None
    }
    
    /// Cancel drag operation
    pub fn cancel_drag(&mut self, engine: &mut GraphEngine) {
        // Restore original positions
        for (node_id, original_pos) in &self.original_positions {
            if let Some(node) = engine.scene_mut().get_node_mut(*node_id) {
                node.position = *original_pos;
            }
        }
        
        self.end_drag();
    }
}

/// Result of drag over check
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DragOverResult {
    None,
    Accept,
    Reject,
}

/// Action to perform on drop
#[derive(Debug, Clone)]
pub enum DropAction {
    CreateEdge { source: SceneId, target: SceneId },
    MoveToFolder { node: SceneId, folder: SceneId },
    AttachToNode { child: SceneId, parent: SceneId },
}