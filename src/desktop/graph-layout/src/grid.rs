//! Grid layout algorithm implementation

use crate::{LayoutAlgorithm, LayoutNode, LayoutEdge, LayoutResult, LayoutError, LayoutType, LayoutBounds, utils};
use horizonos_graph_engine::{SceneId, Position};
use std::collections::HashMap;

/// Grid layout algorithm for regular arrangements
pub struct GridLayout {
    pub cell_size: f32,
    pub columns: Option<usize>,
    pub center: Position,
    pub bounds: LayoutBounds,
    pub variant: GridVariant,
}

#[derive(Debug, Clone)]
pub enum GridVariant {
    Square,
    Hexagonal,
    Triangular,
}

impl GridLayout {
    pub fn new() -> Self {
        GridLayout {
            cell_size: 60.0,
            columns: None,
            center: Position::new(0.0, 0.0, 0.0),
            bounds: LayoutBounds::default(),
            variant: GridVariant::Square,
        }
    }
    
    pub fn with_cell_size(mut self, size: f32) -> Self {
        self.cell_size = size;
        self
    }
    
    pub fn with_columns(mut self, columns: usize) -> Self {
        self.columns = Some(columns);
        self
    }
    
    pub fn with_variant(mut self, variant: GridVariant) -> Self {
        self.variant = variant;
        self
    }
}

impl LayoutAlgorithm for GridLayout {
    fn calculate_layout(&self, nodes: &[LayoutNode], _edges: &[LayoutEdge]) -> Result<LayoutResult, LayoutError> {
        if nodes.is_empty() {
            return Err(LayoutError::InsufficientNodes { count: 0 });
        }
        
        let start_time = chrono::Utc::now();
        let mut node_positions = HashMap::new();
        
        let cols = self.columns.unwrap_or_else(|| (nodes.len() as f32).sqrt().ceil() as usize);
        
        match self.variant {
            GridVariant::Square => {
                for (i, node) in nodes.iter().enumerate() {
                    let row = i / cols;
                    let col = i % cols;
                    
                    let x = self.center.x + (col as f32 - cols as f32 / 2.0) * self.cell_size;
                    let y = self.center.y + (row as f32 - (nodes.len() / cols) as f32 / 2.0) * self.cell_size;
                    
                    let mut position = Position::new(x, y, self.center.z);
                    utils::apply_bounds(&mut position, &self.bounds);
                    node_positions.insert(node.id, position);
                }
            }
            GridVariant::Hexagonal => {
                for (i, node) in nodes.iter().enumerate() {
                    let row = i / cols;
                    let col = i % cols;
                    
                    let x_offset = if row % 2 == 1 { self.cell_size * 0.5 } else { 0.0 };
                    let x = self.center.x + (col as f32 - cols as f32 / 2.0) * self.cell_size + x_offset;
                    let y = self.center.y + (row as f32 - (nodes.len() / cols) as f32 / 2.0) * self.cell_size * 0.866;
                    
                    let mut position = Position::new(x, y, self.center.z);
                    utils::apply_bounds(&mut position, &self.bounds);
                    node_positions.insert(node.id, position);
                }
            }
            GridVariant::Triangular => {
                for (i, node) in nodes.iter().enumerate() {
                    let row = i / cols;
                    let col = i % cols;
                    
                    let x = self.center.x + (col as f32 - cols as f32 / 2.0) * self.cell_size;
                    let y = self.center.y + (row as f32 - (nodes.len() / cols) as f32 / 2.0) * self.cell_size * 0.75;
                    
                    let mut position = Position::new(x, y, self.center.z);
                    utils::apply_bounds(&mut position, &self.bounds);
                    node_positions.insert(node.id, position);
                }
            }
        }
        
        let processing_time = chrono::Utc::now() - start_time;
        
        Ok(LayoutResult {
            node_positions,
            iterations_performed: 1,
            energy: 0.0,
            converged: true,
            processing_time,
        })
    }
    
    fn update_layout(&self, nodes: &mut [LayoutNode], edges: &[LayoutEdge], _delta_time: f32) -> Result<f32, LayoutError> {
        let result = self.calculate_layout(nodes, edges)?;
        for node in nodes.iter_mut() {
            if let Some(position) = result.node_positions.get(&node.id) {
                node.position = *position;
            }
        }
        Ok(0.0)
    }
    
    fn name(&self) -> &str {
        "Grid"
    }
    
    fn supports_incremental(&self) -> bool {
        false
    }
    
    fn recommended_settings(&self) -> LayoutType {
        LayoutType::Grid {
            cell_size: self.cell_size,
            columns: self.columns,
        }
    }
}

impl Default for GridLayout {
    fn default() -> Self {
        Self::new()
    }
}