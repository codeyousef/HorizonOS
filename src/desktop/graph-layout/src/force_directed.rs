//! Force-directed layout algorithm implementation
//! 
//! This module implements spring-force algorithms for natural graph layouts,
//! including Fruchterman-Reingold and spring-electrical models.

use crate::{LayoutAlgorithm, LayoutNode, LayoutEdge, LayoutResult, LayoutError, LayoutType, LayoutBounds, utils};
use nalgebra::Vector3;
use std::collections::HashMap;

/// Force-directed layout algorithm using spring-electrical model
pub struct ForceDirectedLayout {
    pub spring_strength: f32,
    pub repulsion_strength: f32,
    pub damping: f32,
    pub max_iterations: usize,
    pub convergence_threshold: f32,
    pub bounds: LayoutBounds,
    pub cool_down_factor: f32,
}

/// Different force-directed algorithm variants
#[derive(Debug, Clone)]
pub enum ForceDirectedVariant {
    SpringElectrical,
    FruchtermanReingold,
    ForceAtlas2,
    LinLog,
}

impl ForceDirectedLayout {
    pub fn new() -> Self {
        ForceDirectedLayout {
            spring_strength: 0.05,
            repulsion_strength: 100.0,
            damping: 0.9,
            max_iterations: 500,
            convergence_threshold: 0.1,
            bounds: LayoutBounds::default(),
            cool_down_factor: 0.95,
        }
    }
    
    pub fn with_spring_strength(mut self, strength: f32) -> Self {
        self.spring_strength = strength;
        self
    }
    
    pub fn with_repulsion_strength(mut self, strength: f32) -> Self {
        self.repulsion_strength = strength;
        self
    }
    
    pub fn with_damping(mut self, damping: f32) -> Self {
        self.damping = damping;
        self
    }
    
    pub fn with_max_iterations(mut self, iterations: usize) -> Self {
        self.max_iterations = iterations;
        self
    }
    
    pub fn with_bounds(mut self, bounds: LayoutBounds) -> Self {
        self.bounds = bounds;
        self
    }
    
    /// Calculate forces between all nodes and update positions
    fn calculate_forces(&self, nodes: &mut [LayoutNode], edges: &[LayoutEdge]) -> f32 {
        let node_count = nodes.len();
        let mut forces = vec![Vector3::zeros(); node_count];
        
        // Calculate repulsive forces between all pairs of nodes
        for i in 0..node_count {
            for j in (i + 1)..node_count {
                let pos_i = Vector3::new(nodes[i].position.x, nodes[i].position.y, nodes[i].position.z);
                let pos_j = Vector3::new(nodes[j].position.x, nodes[j].position.y, nodes[j].position.z);
                
                let diff = pos_i - pos_j;
                let distance = diff.magnitude();
                
                if distance > 0.001 {
                    let force_magnitude = self.repulsion_strength / (distance * distance);
                    let force_direction = diff.normalize();
                    let force = force_direction * force_magnitude;
                    
                    forces[i] += force / nodes[i].mass;
                    forces[j] -= force / nodes[j].mass;
                }
            }
        }
        
        // Calculate attractive forces along edges
        for edge in edges {
            if let (Some(i), Some(j)) = (
                nodes.iter().position(|n| n.id == edge.source),
                nodes.iter().position(|n| n.id == edge.target)
            ) {
                let pos_i = Vector3::new(nodes[i].position.x, nodes[i].position.y, nodes[i].position.z);
                let pos_j = Vector3::new(nodes[j].position.x, nodes[j].position.y, nodes[j].position.z);
                
                let diff = pos_j - pos_i;
                let distance = diff.magnitude();
                
                if distance > 0.001 {
                    let spring_force = self.spring_strength * (distance - edge.length) * edge.weight;
                    let force_direction = diff.normalize();
                    let force = force_direction * spring_force;
                    
                    forces[i] += force / nodes[i].mass;
                    forces[j] -= force / nodes[j].mass;
                }
            }
        }
        
        // Apply forces and update positions
        let mut total_energy = 0.0;
        
        for (i, node) in nodes.iter_mut().enumerate() {
            if !node.fixed {
                // Update velocity with damping
                node.velocity = node.velocity * self.damping + forces[i];
                
                // Update position
                node.position.x += node.velocity.x;
                node.position.y += node.velocity.y;
                node.position.z += node.velocity.z;
                
                // Apply bounds
                utils::apply_bounds(&mut node.position, &self.bounds);
                
                // Calculate energy for convergence detection
                total_energy += node.velocity.magnitude_squared();
            }
        }
        
        total_energy
    }
    
    /// Calculate attractive force using Hooke's law
    fn calculate_attractive_force(&self, distance: f32, ideal_length: f32, weight: f32) -> f32 {
        self.spring_strength * (distance - ideal_length) * weight
    }
    
    /// Calculate repulsive force using inverse square law
    fn calculate_repulsive_force(&self, distance: f32) -> f32 {
        if distance < 0.001 {
            return self.repulsion_strength * 1000.0;
        }
        self.repulsion_strength / (distance * distance)
    }
    
    /// Initialize node positions randomly if not set
    fn initialize_positions(&self, nodes: &mut [LayoutNode]) {
        for node in nodes.iter_mut() {
            if node.position.x == 0.0 && node.position.y == 0.0 && node.position.z == 0.0 {
                node.position = utils::random_position(&self.bounds);
            }
        }
    }
    
    /// Fruchterman-Reingold specific implementation
    fn fruchterman_reingold_step(&self, nodes: &mut [LayoutNode], edges: &[LayoutEdge], temperature: f32) -> f32 {
        let k = (self.bounds.max_x - self.bounds.min_x) / (nodes.len() as f32).sqrt();
        let node_count = nodes.len();
        let mut displacements = vec![Vector3::zeros(); node_count];
        
        // Calculate repulsive forces
        for i in 0..node_count {
            for j in 0..node_count {
                if i != j {
                    let pos_i = Vector3::new(nodes[i].position.x, nodes[i].position.y, nodes[i].position.z);
                    let pos_j = Vector3::new(nodes[j].position.x, nodes[j].position.y, nodes[j].position.z);
                    
                    let diff = pos_i - pos_j;
                    let distance = diff.magnitude().max(0.001);
                    
                    let force_magnitude = k * k / distance;
                    let force = diff.normalize() * force_magnitude;
                    
                    displacements[i] += force;
                }
            }
        }
        
        // Calculate attractive forces
        for edge in edges {
            if let (Some(i), Some(j)) = (
                nodes.iter().position(|n| n.id == edge.source),
                nodes.iter().position(|n| n.id == edge.target)
            ) {
                let pos_i = Vector3::new(nodes[i].position.x, nodes[i].position.y, nodes[i].position.z);
                let pos_j = Vector3::new(nodes[j].position.x, nodes[j].position.y, nodes[j].position.z);
                
                let diff = pos_j - pos_i;
                let distance = diff.magnitude().max(0.001);
                
                let force_magnitude = distance * distance / k;
                let force = diff.normalize() * force_magnitude;
                
                displacements[i] += force * edge.weight;
                displacements[j] -= force * edge.weight;
            }
        }
        
        // Apply displacements with temperature cooling
        let mut total_energy = 0.0;
        
        for (i, node) in nodes.iter_mut().enumerate() {
            if !node.fixed {
                let displacement_magnitude = displacements[i].magnitude();
                if displacement_magnitude > 0.001 {
                    let limited_displacement = displacements[i].normalize() * displacement_magnitude.min(temperature);
                    
                    node.position.x += limited_displacement.x;
                    node.position.y += limited_displacement.y;
                    node.position.z += limited_displacement.z;
                    
                    utils::apply_bounds(&mut node.position, &self.bounds);
                    total_energy += displacement_magnitude;
                }
            }
        }
        
        total_energy
    }
}

impl LayoutAlgorithm for ForceDirectedLayout {
    fn calculate_layout(&self, nodes: &[LayoutNode], edges: &[LayoutEdge]) -> Result<LayoutResult, LayoutError> {
        if nodes.len() < 2 {
            return Err(LayoutError::InsufficientNodes { count: nodes.len() });
        }
        
        let start_time = chrono::Utc::now();
        let mut working_nodes = nodes.to_vec();
        
        // Initialize random positions for unpositioned nodes
        self.initialize_positions(&mut working_nodes);
        
        let mut iteration = 0;
        let mut energy = f32::INFINITY;
        let mut converged = false;
        
        // Main simulation loop
        while iteration < self.max_iterations && !converged {
            energy = self.calculate_forces(&mut working_nodes, edges);
            
            // Check for convergence
            if energy < self.convergence_threshold {
                converged = true;
            }
            
            iteration += 1;
            
            // Optional: Cool down the system over time
            if iteration % 50 == 0 {
                log::debug!("Force-directed layout iteration {}, energy: {:.4}", iteration, energy);
            }
        }
        
        // Collect final positions
        let mut node_positions = HashMap::new();
        for node in &working_nodes {
            node_positions.insert(node.id, node.position);
        }
        
        let processing_time = chrono::Utc::now() - start_time;
        
        log::info!(
            "Force-directed layout completed: {} iterations, energy: {:.4}, converged: {}",
            iteration, energy, converged
        );
        
        Ok(LayoutResult {
            node_positions,
            iterations_performed: iteration,
            energy,
            converged,
            processing_time,
        })
    }
    
    fn update_layout(&self, nodes: &mut [LayoutNode], edges: &[LayoutEdge], delta_time: f32) -> Result<f32, LayoutError> {
        if nodes.is_empty() {
            return Ok(0.0);
        }
        
        // Perform one step of force calculation with time scaling
        let energy = self.calculate_forces(nodes, edges);
        
        // Scale movement by delta_time for smooth real-time updates
        for node in nodes.iter_mut() {
            if !node.fixed {
                node.velocity *= delta_time;
            }
        }
        
        Ok(energy)
    }
    
    fn name(&self) -> &str {
        "Force-Directed"
    }
    
    fn supports_incremental(&self) -> bool {
        true
    }
    
    fn recommended_settings(&self) -> LayoutType {
        LayoutType::ForceDirected {
            spring_strength: self.spring_strength,
            repulsion_strength: self.repulsion_strength,
            damping: self.damping,
        }
    }
}

impl Default for ForceDirectedLayout {
    fn default() -> Self {
        Self::new()
    }
}

/// Specialized force-directed algorithm for large graphs
pub struct ScalableForceDirectedLayout {
    base: ForceDirectedLayout,
    use_grid_approximation: bool,
    grid_size: usize,
    barnes_hut_theta: f32, // Theta parameter for Barnes-Hut approximation
}

impl ScalableForceDirectedLayout {
    pub fn new() -> Self {
        ScalableForceDirectedLayout {
            base: ForceDirectedLayout::new(),
            use_grid_approximation: true,
            grid_size: 32,
            barnes_hut_theta: 0.5,
        }
    }
    
    pub fn with_base(mut self, base: ForceDirectedLayout) -> Self {
        self.base = base;
        self
    }
    
    pub fn with_grid_approximation(mut self, enabled: bool, grid_size: usize) -> Self {
        self.use_grid_approximation = enabled;
        self.grid_size = grid_size;
        self
    }
}

impl LayoutAlgorithm for ScalableForceDirectedLayout {
    fn calculate_layout(&self, nodes: &[LayoutNode], edges: &[LayoutEdge]) -> Result<LayoutResult, LayoutError> {
        // For large graphs (>1000 nodes), use approximation techniques
        if nodes.len() > 1000 && self.use_grid_approximation {
            log::info!("Using grid approximation for {} nodes", nodes.len());
            // Implementation would use spatial subdivision for O(n log n) complexity
            // For now, fall back to basic algorithm
        }
        
        self.base.calculate_layout(nodes, edges)
    }
    
    fn update_layout(&self, nodes: &mut [LayoutNode], edges: &[LayoutEdge], delta_time: f32) -> Result<f32, LayoutError> {
        self.base.update_layout(nodes, edges, delta_time)
    }
    
    fn name(&self) -> &str {
        "Scalable Force-Directed"
    }
    
    fn supports_incremental(&self) -> bool {
        true
    }
    
    fn recommended_settings(&self) -> LayoutType {
        self.base.recommended_settings()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LayoutNode, LayoutEdge, Position};
    
    #[test]
    fn test_force_directed_creation() {
        let layout = ForceDirectedLayout::new()
            .with_spring_strength(0.1)
            .with_repulsion_strength(200.0)
            .with_damping(0.8);
        
        assert_eq!(layout.spring_strength, 0.1);
        assert_eq!(layout.repulsion_strength, 200.0);
        assert_eq!(layout.damping, 0.8);
    }
    
    #[test]
    fn test_simple_layout_calculation() {
        let layout = ForceDirectedLayout::new().with_max_iterations(10);
        
        let nodes = vec![
            LayoutNode::new(1, Position::new(0.0, 0.0, 0.0)),
            LayoutNode::new(2, Position::new(1.0, 0.0, 0.0)),
            LayoutNode::new(3, Position::new(0.5, 1.0, 0.0)),
        ];
        
        let edges = vec![
            LayoutEdge::new(1, 2, 1.0),
            LayoutEdge::new(2, 3, 1.0),
        ];
        
        let result = layout.calculate_layout(&nodes, &edges).unwrap();
        
        assert_eq!(result.node_positions.len(), 3);
        assert!(result.iterations_performed <= 10);
        assert!(!result.processing_time.is_zero());
    }
    
    #[test]
    fn test_insufficient_nodes() {
        let layout = ForceDirectedLayout::new();
        let nodes = vec![LayoutNode::new(1, Position::new(0.0, 0.0, 0.0))];
        let edges = vec![];
        
        let result = layout.calculate_layout(&nodes, &edges);
        assert!(matches!(result, Err(LayoutError::InsufficientNodes { count: 1 })));
    }
    
    #[test]
    fn test_incremental_update() {
        let layout = ForceDirectedLayout::new();
        let mut nodes = vec![
            LayoutNode::new(1, Position::new(0.0, 0.0, 0.0)),
            LayoutNode::new(2, Position::new(10.0, 0.0, 0.0)),
        ];
        let edges = vec![LayoutEdge::new(1, 2, 1.0)];
        
        let energy = layout.update_layout(&mut nodes, &edges, 0.016).unwrap();
        assert!(energy >= 0.0);
        assert!(layout.supports_incremental());
    }
    
    #[test]
    fn test_fixed_node_positions() {
        let layout = ForceDirectedLayout::new().with_max_iterations(5);
        
        let nodes = vec![
            LayoutNode::new(1, Position::new(0.0, 0.0, 0.0)).fixed(),
            LayoutNode::new(2, Position::new(1.0, 0.0, 0.0)),
        ];
        
        let edges = vec![LayoutEdge::new(1, 2, 1.0)];
        
        let result = layout.calculate_layout(&nodes, &edges).unwrap();
        
        // Fixed node should not move
        let fixed_pos = result.node_positions.get(&1).unwrap();
        assert_eq!(fixed_pos.x, 0.0);
        assert_eq!(fixed_pos.y, 0.0);
        assert_eq!(fixed_pos.z, 0.0);
    }
    
    #[test]
    fn test_scalable_layout() {
        let layout = ScalableForceDirectedLayout::new()
            .with_grid_approximation(true, 16);
        
        assert_eq!(layout.name(), "Scalable Force-Directed");
        assert!(layout.supports_incremental());
    }
}