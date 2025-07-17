//! Graph rendering integration for the compositor

use smithay::{
    backend::{
        renderer::{
            element::{Element, RenderElement},
            gles::GlesRenderer,
            Frame, ImportAll, Renderer, Bind,
        },
    },
    desktop::Window,
    output::Output,
    utils::{Rectangle, Transform, Physical, Scale, Logical},
};
use crate::AppState;
use horizonos_graph_engine::{Scene, Camera};
use std::sync::{Arc, Mutex};
use anyhow::Result;

/// Graph rendering integration
pub struct GraphRenderIntegration {
    // For now, keep this simple
}

impl GraphRenderIntegration {
    pub fn new() -> Result<Self> {
        Ok(Self {})
    }
    
    /// Render a frame combining Wayland surfaces and graph visualization
    pub fn render_frame(
        &self,
        renderer: &mut GlesRenderer,
        state: &mut AppState,
        output: &Output,
        age: usize,
    ) -> Result<()> {
        let output_geometry = state.space.output_geometry(output)
            .unwrap_or(Rectangle::from_loc_and_size((0, 0), (1920, 1080)));
        
        // Convert to physical coordinates
        let scale = Scale::from(output.current_scale().fractional_scale());
        let output_rect = output_geometry.to_physical_precise_round(scale);
        
        // Update window positions based on graph layout
        self.update_window_positions(state, output_rect);
        
        // Actual rendering would be done by Smithay's render pipeline
        // This is just updating positions
        
        Ok(())
    }
    
    /// Update window positions based on graph layout
    fn update_window_positions(
        &self,
        state: &mut AppState,
        output_rect: Rectangle<i32, Physical>,
    ) {
        // Update window positions based on graph layout
        let windows_to_update: Vec<_> = state.space.elements()
            .filter_map(|window| {
                window.toplevel().map(|toplevel| {
                    let surface = toplevel.wl_surface();
                    (window.clone(), surface.clone())
                })
            })
            .collect();
            
        for (window, surface) in windows_to_update {
            // Get the window position from graph if available
            if let Some(node_id) = state.surface_to_node.get(&surface) {
                if let Some(node) = state.graph_scene.lock().unwrap().get_node(*node_id) {
                    // Convert 3D graph position to 2D screen position
                    let x = (node.position.x * 100.0 + output_rect.size.w as f32 / 2.0) as i32;
                    let y = (node.position.y * 100.0 + output_rect.size.h as f32 / 2.0) as i32;
                    
                    // Update window position in space
                    state.space.map_element(window, (x, y), false);
                }
            }
        }
    }
    
    /// Update camera based on interaction
    pub fn update_camera(&self, state: &AppState) {
        // Get camera updates from interaction manager
        // This is handled by the interaction system
    }
}