//! Core compositor functionality - updated for Smithay 0.7

use smithay::{
    desktop::Window,
    utils::Rectangle,
    wayland::shell::xdg::ToplevelSurface,
};
use crate::AppState;

/// Initialize the compositor
pub fn init_compositor(_state: &mut AppState) {
    log::info!("Initializing HorizonOS Graph Compositor");
    
    // Set up initial output
    // TODO: Proper output management
    
    // Initialize graph scene
    // TODO: Set up initial graph layout
}

/// Handle window creation
pub fn handle_new_window(state: &mut AppState, window: Window) {
    use horizonos_graph_engine::{SceneNode, NodeType};
    
    // Create a graph node for the window
    let node = SceneNode {
        id: 0, // Will be set by Scene
        position: nalgebra::Point3::new(0.0, 0.0, 0.0),
        velocity: nalgebra::Vector3::zeros(),
        node_type: NodeType::Application { 
            pid: 0, 
            name: "Window".to_string() 
        },
        radius: 1.0,
        color: [0.5, 0.5, 0.5, 1.0],
        metadata: horizonos_graph_engine::NodeMetadata {
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            tags: vec![],
            description: None,
            properties: std::collections::HashMap::new(),
        },
        visible: true,
        selected: false,
    };
    
    let node_id = state.graph_scene.lock().unwrap().add_node(node);
    
    // Add to space
    state.space.map_element(window.clone(), (0, 0), true);
    
    // Store mapping
    if let Some(toplevel) = window.toplevel() {
        let surface = toplevel.wl_surface();
        state.surface_to_node.insert(surface.clone(), node_id);
    }
    
    log::info!("Created new window with graph node {:?}", node_id);
}

/// Update window positions based on graph layout
pub fn update_window_positions(state: &mut AppState) {
    let scene = state.graph_scene.lock().unwrap();
    
    // Collect windows and their new positions
    let mut updates = Vec::new();
    
    for (surface, node_id) in &state.surface_to_node {
        if let Some(node) = scene.get_node(*node_id) {
            // Find window in space
            for window in state.space.elements() {
                if let Some(toplevel) = window.toplevel() {
                    if toplevel.wl_surface() == surface {
                        // Convert 3D position to 2D screen coordinates
                        let screen_pos = (
                            (node.position.x * 100.0) as i32,
                            (node.position.y * 100.0) as i32,
                        );
                        
                        updates.push((window.clone(), screen_pos));
                        break;
                    }
                }
            }
        }
    }
    
    // Apply updates
    drop(scene);
    for (window, pos) in updates {
        state.space.map_element(window, pos, false);
    }
}