//! Temporal layout algorithm implementation
//! 
//! This module implements time-based layouts that arrange nodes along temporal axes.

use crate::{
    LayoutAlgorithm, LayoutNode, LayoutEdge, LayoutResult, LayoutError, LayoutType, 
    TimeAxis, LayoutBounds, utils
};
use horizonos_graph_engine::{SceneId, Position};
use std::collections::HashMap;
use chrono::{DateTime, Utc, Duration};

/// Temporal layout algorithm for time-based visualization
pub struct TemporalLayout {
    pub time_axis: TimeAxis,
    pub time_scale: f32,
    pub bounds: LayoutBounds,
    pub variant: TemporalVariant,
    pub time_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
}

/// Different temporal layout variants
#[derive(Debug, Clone)]
pub enum TemporalVariant {
    Timeline,
    Spiral { turns: f32 },
    Layers { layer_height: f32 },
    Flow { flow_direction: FlowDirection },
}

/// Direction for flow-based temporal layouts
#[derive(Debug, Clone)]
pub enum FlowDirection {
    Horizontal,
    Vertical,
    Diagonal,
}

impl TemporalLayout {
    pub fn new() -> Self {
        TemporalLayout {
            time_axis: TimeAxis::X,
            time_scale: 100.0,
            bounds: LayoutBounds::default(),
            variant: TemporalVariant::Timeline,
            time_range: None,
        }
    }
    
    pub fn with_time_axis(mut self, axis: TimeAxis) -> Self {
        self.time_axis = axis;
        self
    }
    
    pub fn with_time_scale(mut self, scale: f32) -> Self {
        self.time_scale = scale;
        self
    }
    
    pub fn with_variant(mut self, variant: TemporalVariant) -> Self {
        self.variant = variant;
        self
    }
    
    pub fn with_time_range(mut self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        self.time_range = Some((start, end));
        self
    }
    
    pub fn with_bounds(mut self, bounds: LayoutBounds) -> Self {
        self.bounds = bounds;
        self
    }
    
    /// Extract or estimate timestamps from nodes
    fn extract_timestamps(&self, nodes: &[LayoutNode]) -> HashMap<SceneId, DateTime<Utc>> {
        let mut timestamps = HashMap::new();
        let now = Utc::now();
        
        for (i, node) in nodes.iter().enumerate() {
            let timestamp = node.timestamp.unwrap_or_else(|| {
                // Fallback: distribute nodes evenly over the last 24 hours
                now - Duration::hours(24) + Duration::minutes(i as i64 * 10)
            });
            timestamps.insert(node.id, timestamp);
        }
        
        timestamps
    }
    
    /// Determine time range from nodes or use provided range
    fn determine_time_range(&self, timestamps: &HashMap<SceneId, DateTime<Utc>>) -> (DateTime<Utc>, DateTime<Utc>) {
        if let Some(range) = self.time_range {
            return range;
        }
        
        if timestamps.is_empty() {
            let now = Utc::now();
            return (now - Duration::hours(24), now);
        }
        
        let min_time = timestamps.values().min().copied().unwrap();
        let max_time = timestamps.values().max().copied().unwrap();
        
        // Add some padding
        let padding = (max_time - min_time) / 10;
        (min_time - padding, max_time + padding)
    }
    
    /// Convert timestamp to position along time axis
    fn timestamp_to_position(&self, timestamp: DateTime<Utc>, time_range: (DateTime<Utc>, DateTime<Utc>)) -> f32 {
        let (start_time, end_time) = time_range;
        let total_duration = end_time - start_time;
        let elapsed_duration = timestamp - start_time;
        
        if total_duration.num_milliseconds() == 0 {
            return 0.0;
        }
        
        let progress = elapsed_duration.num_milliseconds() as f32 / total_duration.num_milliseconds() as f32;
        progress.clamp(0.0, 1.0) * self.time_scale
    }
    
    /// Calculate positions for timeline layout
    fn calculate_timeline_layout(&self, nodes: &[LayoutNode], timestamps: &HashMap<SceneId, DateTime<Utc>>) -> HashMap<SceneId, Position> {
        let mut positions = HashMap::new();
        let time_range = self.determine_time_range(timestamps);
        
        // Group nodes by timestamp for better distribution
        let mut time_groups: HashMap<i64, Vec<SceneId>> = HashMap::new();
        
        for (&node_id, &timestamp) in timestamps {
            let time_key = timestamp.timestamp() / 3600; // Group by hour
            time_groups.entry(time_key).or_insert_with(Vec::new).push(node_id);
        }
        
        for (&node_id, &timestamp) in timestamps {
            let time_position = self.timestamp_to_position(timestamp, time_range);
            
            // Calculate offset within time group to prevent overlap
            let time_key = timestamp.timestamp() / 3600;
            let group = time_groups.get(&time_key).unwrap();
            let node_index = group.iter().position(|&id| id == node_id).unwrap_or(0);
            let group_offset = (node_index as f32 - group.len() as f32 / 2.0) * 20.0;
            
            let position = match self.time_axis {
                TimeAxis::X => Position::new(
                    time_position - self.time_scale / 2.0,
                    group_offset,
                    0.0,
                ),
                TimeAxis::Y => Position::new(
                    group_offset,
                    time_position - self.time_scale / 2.0,
                    0.0,
                ),
                TimeAxis::Z => Position::new(
                    group_offset,
                    0.0,
                    time_position - self.time_scale / 2.0,
                ),
            };
            
            let mut final_position = position;
            utils::apply_bounds(&mut final_position, &self.bounds);
            positions.insert(node_id, final_position);
        }
        
        positions
    }
    
    /// Calculate positions for spiral layout
    fn calculate_spiral_layout(&self, nodes: &[LayoutNode], timestamps: &HashMap<SceneId, DateTime<Utc>>, turns: f32) -> HashMap<SceneId, Position> {
        let mut positions = HashMap::new();
        let time_range = self.determine_time_range(timestamps);
        
        for (&node_id, &timestamp) in timestamps {
            let time_progress = {
                let (start_time, end_time) = time_range;
                let total_duration = end_time - start_time;
                let elapsed_duration = timestamp - start_time;
                
                if total_duration.num_milliseconds() == 0 {
                    0.0
                } else {
                    elapsed_duration.num_milliseconds() as f32 / total_duration.num_milliseconds() as f32
                }
            };
            
            let angle = time_progress * turns * 2.0 * std::f32::consts::PI;
            let radius = time_progress * self.time_scale * 0.5;
            
            let position = match self.time_axis {
                TimeAxis::X => Position::new(
                    time_progress * self.time_scale - self.time_scale / 2.0,
                    radius * angle.sin(),
                    radius * angle.cos(),
                ),
                TimeAxis::Y => Position::new(
                    radius * angle.cos(),
                    time_progress * self.time_scale - self.time_scale / 2.0,
                    radius * angle.sin(),
                ),
                TimeAxis::Z => Position::new(
                    radius * angle.cos(),
                    radius * angle.sin(),
                    time_progress * self.time_scale - self.time_scale / 2.0,
                ),
            };
            
            let mut final_position = position;
            utils::apply_bounds(&mut final_position, &self.bounds);
            positions.insert(node_id, final_position);
        }
        
        positions
    }
    
    /// Calculate positions for layered layout
    fn calculate_layers_layout(&self, nodes: &[LayoutNode], timestamps: &HashMap<SceneId, DateTime<Utc>>, layer_height: f32) -> HashMap<SceneId, Position> {
        let mut positions = HashMap::new();
        let time_range = self.determine_time_range(timestamps);
        
        // Group nodes into time layers
        let (start_time, end_time) = time_range;
        let total_duration = end_time - start_time;
        let layer_duration = total_duration / 10; // 10 layers
        
        let mut layers: HashMap<usize, Vec<SceneId>> = HashMap::new();
        
        for (&node_id, &timestamp) in timestamps {
            let elapsed = timestamp - start_time;
            let layer_index = if layer_duration.num_milliseconds() > 0 {
                (elapsed.num_milliseconds() / layer_duration.num_milliseconds()).min(9) as usize
            } else {
                0
            };
            
            layers.entry(layer_index).or_insert_with(Vec::new).push(node_id);
        }
        
        // Position nodes within each layer
        for (layer_index, layer_nodes) in layers {
            let layer_y = (layer_index as f32) * layer_height;
            let nodes_per_row = (layer_nodes.len() as f32).sqrt().ceil() as usize;
            
            for (i, &node_id) in layer_nodes.iter().enumerate() {
                let row = i / nodes_per_row;
                let col = i % nodes_per_row;
                
                let x = (col as f32 - nodes_per_row as f32 / 2.0) * 30.0;
                let z = (row as f32) * 30.0;
                
                let position = match self.time_axis {
                    TimeAxis::X => Position::new(layer_y, x, z),
                    TimeAxis::Y => Position::new(x, layer_y, z),
                    TimeAxis::Z => Position::new(x, z, layer_y),
                };
                
                let mut final_position = position;
                utils::apply_bounds(&mut final_position, &self.bounds);
                positions.insert(node_id, final_position);
            }
        }
        
        positions
    }
    
    /// Calculate positions for flow layout
    fn calculate_flow_layout(&self, nodes: &[LayoutNode], timestamps: &HashMap<SceneId, DateTime<Utc>>, edges: &[LayoutEdge], flow_direction: &FlowDirection) -> HashMap<SceneId, Position> {
        let mut positions = HashMap::new();
        let time_range = self.determine_time_range(timestamps);
        
        // Build temporal sequence from edges
        let temporal_order = self.build_temporal_sequence(nodes, edges, timestamps);
        
        for (sequence_index, &node_id) in temporal_order.iter().enumerate() {
            let timestamp = timestamps.get(&node_id).copied().unwrap_or_else(Utc::now);
            let time_position = self.timestamp_to_position(timestamp, time_range);
            
            let flow_position = sequence_index as f32 * 40.0;
            
            let position = match flow_direction {
                FlowDirection::Horizontal => Position::new(
                    time_position - self.time_scale / 2.0,
                    flow_position - (temporal_order.len() as f32 * 20.0),
                    0.0,
                ),
                FlowDirection::Vertical => Position::new(
                    flow_position - (temporal_order.len() as f32 * 20.0),
                    time_position - self.time_scale / 2.0,
                    0.0,
                ),
                FlowDirection::Diagonal => Position::new(
                    time_position - self.time_scale / 2.0,
                    flow_position - (temporal_order.len() as f32 * 20.0),
                    (time_position + flow_position) * 0.1,
                ),
            };
            
            let mut final_position = position;
            utils::apply_bounds(&mut final_position, &self.bounds);
            positions.insert(node_id, final_position);
        }
        
        positions
    }
    
    /// Build temporal sequence from graph structure
    fn build_temporal_sequence(&self, nodes: &[LayoutNode], edges: &[LayoutEdge], timestamps: &HashMap<SceneId, DateTime<Utc>>) -> Vec<SceneId> {
        // Simple topological sort with temporal ordering
        let mut in_degree: HashMap<SceneId, usize> = HashMap::new();
        let mut adjacency: HashMap<SceneId, Vec<SceneId>> = HashMap::new();
        
        // Initialize
        for node in nodes {
            in_degree.insert(node.id, 0);
            adjacency.insert(node.id, Vec::new());
        }
        
        // Build graph
        for edge in edges {
            adjacency.get_mut(&edge.source).unwrap().push(edge.target);
            *in_degree.get_mut(&edge.target).unwrap() += 1;
        }
        
        // Topological sort with temporal tie-breaking
        let mut queue = std::collections::BinaryHeap::new();
        let mut result = Vec::new();
        
        for (&node_id, &degree) in &in_degree {
            if degree == 0 {
                let timestamp = timestamps.get(&node_id).copied().unwrap_or_else(Utc::now);
                queue.push(std::cmp::Reverse((timestamp, node_id)));
            }
        }
        
        while let Some(std::cmp::Reverse((_, node_id))) = queue.pop() {
            result.push(node_id);
            
            if let Some(neighbors) = adjacency.get(&node_id) {
                for &neighbor in neighbors {
                    if let Some(degree) = in_degree.get_mut(&neighbor) {
                        *degree -= 1;
                        if *degree == 0 {
                            let timestamp = timestamps.get(&neighbor).copied().unwrap_or_else(Utc::now);
                            queue.push(std::cmp::Reverse((timestamp, neighbor)));
                        }
                    }
                }
            }
        }
        
        result
    }
}

impl LayoutAlgorithm for TemporalLayout {
    fn calculate_layout(&self, nodes: &[LayoutNode], edges: &[LayoutEdge]) -> Result<LayoutResult, LayoutError> {
        if nodes.is_empty() {
            return Err(LayoutError::InsufficientNodes { count: 0 });
        }
        
        let start_time = chrono::Utc::now();
        
        // Extract timestamps from nodes
        let timestamps = self.extract_timestamps(nodes);
        
        // Calculate positions based on variant
        let node_positions = match &self.variant {
            TemporalVariant::Timeline => {
                self.calculate_timeline_layout(nodes, &timestamps)
            }
            TemporalVariant::Spiral { turns } => {
                self.calculate_spiral_layout(nodes, &timestamps, *turns)
            }
            TemporalVariant::Layers { layer_height } => {
                self.calculate_layers_layout(nodes, &timestamps, *layer_height)
            }
            TemporalVariant::Flow { flow_direction } => {
                self.calculate_flow_layout(nodes, &timestamps, edges, flow_direction)
            }
        };
        
        let processing_time = chrono::Utc::now() - start_time;
        
        log::info!(
            "Temporal layout completed: {} nodes using {:?} variant along {:?} axis",
            nodes.len(),
            self.variant,
            self.time_axis
        );
        
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
        "Temporal"
    }
    
    fn supports_incremental(&self) -> bool {
        false
    }
    
    fn recommended_settings(&self) -> LayoutType {
        LayoutType::Temporal {
            time_axis: self.time_axis.clone(),
            time_scale: self.time_scale,
        }
    }
}

impl Default for TemporalLayout {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LayoutNode, LayoutEdge, Position};
    use chrono::{Duration, Utc};
    
    #[test]
    fn test_temporal_layout_creation() {
        let layout = TemporalLayout::new()
            .with_time_axis(TimeAxis::Y)
            .with_time_scale(200.0)
            .with_variant(TemporalVariant::Spiral { turns: 2.0 });
        
        assert_eq!(layout.time_axis, TimeAxis::Y);
        assert_eq!(layout.time_scale, 200.0);
    }
    
    #[test]
    fn test_timeline_layout() {
        let layout = TemporalLayout::new()
            .with_variant(TemporalVariant::Timeline);
        
        let now = Utc::now();
        let nodes = vec![
            LayoutNode::new(1, Position::new(0.0, 0.0, 0.0))
                .with_timestamp(now - Duration::hours(2)),
            LayoutNode::new(2, Position::new(0.0, 0.0, 0.0))
                .with_timestamp(now - Duration::hours(1)),
            LayoutNode::new(3, Position::new(0.0, 0.0, 0.0))
                .with_timestamp(now),
        ];
        
        let result = layout.calculate_layout(&nodes, &[]).unwrap();
        
        assert_eq!(result.node_positions.len(), 3);
        assert!(result.converged);
        
        // Nodes should be ordered by time along X axis
        let pos1 = result.node_positions.get(&1).unwrap();
        let pos2 = result.node_positions.get(&2).unwrap();
        let pos3 = result.node_positions.get(&3).unwrap();
        
        assert!(pos1.x < pos2.x);
        assert!(pos2.x < pos3.x);
    }
    
    #[test]
    fn test_spiral_layout() {
        let layout = TemporalLayout::new()
            .with_variant(TemporalVariant::Spiral { turns: 1.0 });
        
        let nodes = vec![
            LayoutNode::new(1, Position::new(0.0, 0.0, 0.0)),
            LayoutNode::new(2, Position::new(0.0, 0.0, 0.0)),
        ];
        
        let result = layout.calculate_layout(&nodes, &[]).unwrap();
        assert_eq!(result.node_positions.len(), 2);
    }
    
    #[test]
    fn test_layers_layout() {
        let layout = TemporalLayout::new()
            .with_variant(TemporalVariant::Layers { layer_height: 50.0 });
        
        let nodes = vec![
            LayoutNode::new(1, Position::new(0.0, 0.0, 0.0)),
            LayoutNode::new(2, Position::new(0.0, 0.0, 0.0)),
            LayoutNode::new(3, Position::new(0.0, 0.0, 0.0)),
        ];
        
        let result = layout.calculate_layout(&nodes, &[]).unwrap();
        assert_eq!(result.node_positions.len(), 3);
    }
    
    #[test]
    fn test_flow_layout() {
        let layout = TemporalLayout::new()
            .with_variant(TemporalVariant::Flow { 
                flow_direction: FlowDirection::Horizontal 
            });
        
        let nodes = vec![
            LayoutNode::new(1, Position::new(0.0, 0.0, 0.0)),
            LayoutNode::new(2, Position::new(0.0, 0.0, 0.0)),
        ];
        
        let edges = vec![LayoutEdge::new(1, 2, 1.0)];
        
        let result = layout.calculate_layout(&nodes, &edges).unwrap();
        assert_eq!(result.node_positions.len(), 2);
    }
    
    #[test]
    fn test_timestamp_extraction() {
        let layout = TemporalLayout::new();
        let now = Utc::now();
        
        let nodes = vec![
            LayoutNode::new(1, Position::new(0.0, 0.0, 0.0))
                .with_timestamp(now),
            LayoutNode::new(2, Position::new(0.0, 0.0, 0.0)), // No timestamp
        ];
        
        let timestamps = layout.extract_timestamps(&nodes);
        
        assert_eq!(timestamps.len(), 2);
        assert_eq!(timestamps.get(&1), Some(&now));
        assert!(timestamps.contains_key(&2)); // Should have fallback timestamp
    }
    
    #[test]
    fn test_algorithm_properties() {
        let layout = TemporalLayout::new();
        
        assert_eq!(layout.name(), "Temporal");
        assert!(!layout.supports_incremental());
        
        let settings = layout.recommended_settings();
        assert!(matches!(settings, LayoutType::Temporal { .. }));
    }
}