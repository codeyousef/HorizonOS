//! Workspace layout management

use serde::{Deserialize, Serialize};
use nalgebra::Point3;
use std::collections::HashMap;
use horizonos_graph_engine::scene::SceneId;

/// Workspace layout configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceLayout {
    /// Layout type
    pub layout_type: LayoutType,
    /// Node positions (if manually positioned)
    pub node_positions: HashMap<SceneId, Point3<f32>>,
    /// Layout parameters
    pub parameters: LayoutParameters,
    /// Viewport configuration
    pub viewport: ViewportConfig,
}

impl Default for WorkspaceLayout {
    fn default() -> Self {
        Self {
            layout_type: LayoutType::ForceDirected,
            node_positions: HashMap::new(),
            parameters: LayoutParameters::default(),
            viewport: ViewportConfig::default(),
        }
    }
}

/// Layout types
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum LayoutType {
    /// Manual positioning
    Manual,
    /// Force-directed graph layout
    ForceDirected,
    /// Hierarchical tree layout
    Hierarchical,
    /// Circular layout
    Circular,
    /// Grid layout
    Grid,
    /// Radial layout
    Radial,
    /// Timeline layout (for temporal data)
    Timeline,
}

/// Layout parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutParameters {
    /// Force strength for force-directed layout
    pub force_strength: f32,
    /// Ideal link distance
    pub link_distance: f32,
    /// Repulsion strength
    pub repulsion_strength: f32,
    /// Grid columns for grid layout
    pub grid_columns: u32,
    /// Circle radius for circular layout
    pub circle_radius: f32,
    /// Layer spacing for hierarchical layout
    pub layer_spacing: f32,
    /// Node spacing
    pub node_spacing: f32,
}

impl Default for LayoutParameters {
    fn default() -> Self {
        Self {
            force_strength: 0.1,
            link_distance: 150.0,
            repulsion_strength: 1000.0,
            grid_columns: 5,
            circle_radius: 300.0,
            layer_spacing: 150.0,
            node_spacing: 100.0,
        }
    }
}

/// Viewport configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewportConfig {
    /// Camera position
    pub camera_position: Point3<f32>,
    /// Camera target
    pub camera_target: Point3<f32>,
    /// Zoom level
    pub zoom: f32,
    /// Field of view
    pub fov: f32,
}

impl Default for ViewportConfig {
    fn default() -> Self {
        Self {
            camera_position: Point3::new(0.0, 0.0, 1000.0),
            camera_target: Point3::new(0.0, 0.0, 0.0),
            zoom: 1.0,
            fov: 60.0,
        }
    }
}

/// Layout calculator
pub struct LayoutCalculator;

impl LayoutCalculator {
    /// Calculate node positions based on layout type
    pub fn calculate_positions(
        layout: &WorkspaceLayout,
        node_count: usize,
    ) -> HashMap<usize, Point3<f32>> {
        let mut positions = HashMap::new();
        
        match layout.layout_type {
            LayoutType::Grid => {
                let cols = layout.parameters.grid_columns as usize;
                let spacing = layout.parameters.node_spacing;
                
                for i in 0..node_count {
                    let row = i / cols;
                    let col = i % cols;
                    positions.insert(i, Point3::new(
                        col as f32 * spacing,
                        row as f32 * spacing,
                        0.0,
                    ));
                }
            }
            LayoutType::Circular => {
                let radius = layout.parameters.circle_radius;
                let angle_step = 2.0 * std::f32::consts::PI / node_count as f32;
                
                for i in 0..node_count {
                    let angle = i as f32 * angle_step;
                    positions.insert(i, Point3::new(
                        radius * angle.cos(),
                        radius * angle.sin(),
                        0.0,
                    ));
                }
            }
            LayoutType::Hierarchical => {
                // Simple hierarchical layout
                let layer_spacing = layout.parameters.layer_spacing;
                let node_spacing = layout.parameters.node_spacing;
                let nodes_per_layer = (node_count as f32).sqrt().ceil() as usize;
                
                for i in 0..node_count {
                    let layer = i / nodes_per_layer;
                    let pos_in_layer = i % nodes_per_layer;
                    positions.insert(i, Point3::new(
                        pos_in_layer as f32 * node_spacing,
                        layer as f32 * layer_spacing,
                        0.0,
                    ));
                }
            }
            _ => {
                // Default to grid for other layouts
                let cols = 5;
                let spacing = layout.parameters.node_spacing;
                
                for i in 0..node_count {
                    let row = i / cols;
                    let col = i % cols;
                    positions.insert(i, Point3::new(
                        col as f32 * spacing,
                        row as f32 * spacing,
                        0.0,
                    ));
                }
            }
        }
        
        positions
    }
}