//! Node selection and highlighting system

use horizonos_graph_engine::{SceneId, Scene, Position, Ray};
use std::collections::HashSet;

/// Manages node selection state and operations
pub struct SelectionManager {
    /// Currently selected nodes
    selected_nodes: HashSet<SceneId>,
    /// Primary selection (for multi-selection operations)
    primary_selection: Option<SceneId>,
    /// Currently hovered node
    hovered_node: Option<SceneId>,
    /// Box selection state
    box_selection: Option<BoxSelection>,
}

/// Box selection state
struct BoxSelection {
    start_pos: (f32, f32),
    current_pos: (f32, f32),
    initial_selection: HashSet<SceneId>,
}

impl SelectionManager {
    /// Create a new selection manager
    pub fn new() -> Self {
        Self {
            selected_nodes: HashSet::new(),
            primary_selection: None,
            hovered_node: None,
            box_selection: None,
        }
    }
    
    /// Set the selection to a single node
    pub fn set_selection(&mut self, nodes: Vec<SceneId>) {
        self.selected_nodes.clear();
        for node in nodes {
            self.selected_nodes.insert(node);
        }
        self.primary_selection = self.selected_nodes.iter().next().copied();
    }
    
    /// Add a node to the selection
    pub fn add_to_selection(&mut self, node: SceneId) {
        self.selected_nodes.insert(node);
        if self.primary_selection.is_none() {
            self.primary_selection = Some(node);
        }
    }
    
    /// Remove a node from the selection
    pub fn remove_from_selection(&mut self, node: SceneId) {
        self.selected_nodes.remove(&node);
        if self.primary_selection == Some(node) {
            self.primary_selection = self.selected_nodes.iter().next().copied();
        }
    }
    
    /// Toggle node selection
    pub fn toggle_selection(&mut self, node: SceneId) {
        if self.selected_nodes.contains(&node) {
            self.remove_from_selection(node);
        } else {
            self.add_to_selection(node);
        }
    }
    
    /// Clear all selections
    pub fn clear_selection(&mut self) {
        self.selected_nodes.clear();
        self.primary_selection = None;
    }
    
    /// Get current selection
    pub fn get_selection(&self) -> Vec<SceneId> {
        self.selected_nodes.iter().copied().collect()
    }
    
    /// Get primary selection
    pub fn get_primary_selection(&self) -> Option<SceneId> {
        self.primary_selection
    }
    
    /// Check if a node is selected
    pub fn is_selected(&self, node: SceneId) -> bool {
        self.selected_nodes.contains(&node)
    }
    
    /// Set hovered node
    pub fn set_hover(&mut self, node: Option<SceneId>) {
        self.hovered_node = node;
    }
    
    /// Get hovered node
    pub fn get_hover(&self) -> Option<SceneId> {
        self.hovered_node
    }
    
    /// Start box selection
    pub fn start_box_selection(&mut self, start_pos: (f32, f32)) {
        self.box_selection = Some(BoxSelection {
            start_pos,
            current_pos: start_pos,
            initial_selection: self.selected_nodes.clone(),
        });
    }
    
    /// Update box selection
    pub fn update_box_selection(&mut self, current_pos: (f32, f32)) {
        if let Some(box_sel) = &mut self.box_selection {
            box_sel.current_pos = current_pos;
        }
    }
    
    /// Finish box selection and return selected nodes
    pub fn finish_box_selection(&mut self, engine: &horizonos_graph_engine::GraphEngine) -> Vec<SceneId> {
        if let Some(box_sel) = self.box_selection.take() {
            // Calculate box bounds
            let min_x = box_sel.start_pos.0.min(box_sel.current_pos.0);
            let max_x = box_sel.start_pos.0.max(box_sel.current_pos.0);
            let min_y = box_sel.start_pos.1.min(box_sel.current_pos.1);
            let max_y = box_sel.start_pos.1.max(box_sel.current_pos.1);
            
            // Select nodes within box
            self.selected_nodes.clear();
            let scene = engine.scene();
            
            // TODO: Implement proper screen-space box selection
            // For now, return empty selection
            
            self.get_selection()
        } else {
            self.get_selection()
        }
    }
    
    /// Get box selection bounds (for rendering)
    pub fn get_box_selection_bounds(&self) -> Option<((f32, f32), (f32, f32))> {
        self.box_selection.as_ref().map(|box_sel| {
            (box_sel.start_pos, box_sel.current_pos)
        })
    }
    
    /// Select all nodes
    pub fn select_all(&mut self, engine: &horizonos_graph_engine::GraphEngine) {
        self.selected_nodes.clear();
        let scene = engine.scene();
        // TODO: Add all node IDs from scene
        // For now, this is a placeholder
    }
    
    /// Perform ray-based node picking
    pub fn ray_pick_node(&self, ray: &Ray, scene: &Scene) -> Option<SceneId> {
        let mut closest_node = None;
        let mut closest_distance = f32::MAX;
        
        // Simple ray-sphere intersection for each node
        for (id, node) in scene.nodes() {
            let sphere_center = node.position;
            let sphere_radius = node.radius;
            
            // Ray-sphere intersection
            let oc = ray.origin - sphere_center;
            let a = ray.direction.dot(&ray.direction);
            let b = 2.0 * oc.dot(&ray.direction);
            let c = oc.dot(&oc) - sphere_radius * sphere_radius;
            let discriminant = b * b - 4.0 * a * c;
            
            if discriminant >= 0.0 {
                let sqrt_discriminant = discriminant.sqrt();
                let t1 = (-b - sqrt_discriminant) / (2.0 * a);
                let t2 = (-b + sqrt_discriminant) / (2.0 * a);
                
                let t = if t1 > 0.0 { t1 } else if t2 > 0.0 { t2 } else { continue };
                
                if t < closest_distance {
                    closest_distance = t;
                    closest_node = Some(*id);
                }
            }
        }
        
        closest_node
    }
    
    /// Get nodes within a radius of a position
    pub fn get_nodes_in_radius(&self, center: Position, radius: f32, scene: &Scene) -> Vec<SceneId> {
        let mut nodes = Vec::new();
        
        for (id, node) in scene.nodes() {
            let distance = (node.position - center).magnitude();
            if distance <= radius {
                nodes.push(*id);
            }
        }
        
        nodes
    }
}