//! Physics simulation for graph layout and interactions

use nalgebra::{Point3, Vector3};
use std::collections::HashMap;
use crate::{SceneId, Scene, SceneNode};

/// Physics engine for graph layout and node interactions
#[derive(Debug)]
pub struct PhysicsEngine {
    /// Physics bodies for nodes
    bodies: HashMap<SceneId, PhysicsBody>,
    /// Forces applied to nodes
    forces: HashMap<SceneId, Vector3<f32>>,
    /// Global physics settings
    settings: PhysicsSettings,
    /// Layout algorithm configuration
    layout_config: LayoutConfig,
}

/// Physics body representing a node
#[derive(Debug, Clone)]
pub struct PhysicsBody {
    pub id: SceneId,
    pub position: Point3<f32>,
    pub velocity: Vector3<f32>,
    pub mass: f32,
    pub radius: f32,
    pub fixed: bool, // If true, node position is locked
}

/// Global physics settings
#[derive(Debug, Clone)]
pub struct PhysicsSettings {
    /// Global damping factor (0.0 = no damping, 1.0 = maximum damping)
    pub damping: f32,
    /// Time step for simulation
    pub time_step: f32,
    /// Maximum velocity for any node
    pub max_velocity: f32,
    /// Minimum distance between nodes
    pub min_distance: f32,
    /// Enable collision detection
    pub collision_detection: bool,
}

/// Configuration for layout algorithms
#[derive(Debug, Clone)]
pub struct LayoutConfig {
    /// Force-directed layout settings
    pub force_directed: ForceDirectedConfig,
    /// Spring-damper system settings
    pub spring_damper: SpringDamperConfig,
    /// Repulsion force settings
    pub repulsion: RepulsionConfig,
}

#[derive(Debug, Clone)]
pub struct ForceDirectedConfig {
    /// Attraction force strength
    pub attraction_strength: f32,
    /// Repulsion force strength
    pub repulsion_strength: f32,
    /// Optimal edge length
    pub optimal_edge_length: f32,
    /// Maximum force magnitude
    pub max_force: f32,
}

#[derive(Debug, Clone)]
pub struct SpringDamperConfig {
    /// Spring constant
    pub spring_constant: f32,
    /// Damper constant
    pub damper_constant: f32,
    /// Rest length of springs
    pub rest_length: f32,
}

#[derive(Debug, Clone)]
pub struct RepulsionConfig {
    /// Base repulsion force
    pub base_force: f32,
    /// Repulsion distance threshold
    pub distance_threshold: f32,
    /// Repulsion falloff exponent
    pub falloff_exponent: f32,
}

impl PhysicsEngine {
    /// Create a new physics engine
    pub fn new() -> Self {
        PhysicsEngine {
            bodies: HashMap::new(),
            forces: HashMap::new(),
            settings: PhysicsSettings::default(),
            layout_config: LayoutConfig::default(),
        }
    }
    
    /// Add a physics body for a node
    pub fn add_body(&mut self, node: &SceneNode) {
        let body = PhysicsBody {
            id: node.id,
            position: node.position,
            velocity: node.velocity,
            mass: 1.0, // Can be derived from node size/type
            radius: node.radius,
            fixed: false,
        };
        
        self.bodies.insert(node.id, body);
        self.forces.insert(node.id, Vector3::zeros());
    }
    
    /// Remove a physics body
    pub fn remove_body(&mut self, id: SceneId) {
        self.bodies.remove(&id);
        self.forces.remove(&id);
    }
    
    /// Update physics body from scene node
    pub fn update_body(&mut self, node: &SceneNode) {
        if let Some(body) = self.bodies.get_mut(&node.id) {
            if !body.fixed {
                body.position = node.position;
                body.velocity = node.velocity;
                body.radius = node.radius;
            }
        }
    }
    
    /// Step the physics simulation
    pub fn step(&mut self, delta_time: f32) {
        // Clear previous forces
        for force in self.forces.values_mut() {
            *force = Vector3::zeros();
        }
        
        // Apply layout forces
        self.apply_force_directed_layout();
        self.apply_repulsion_forces();
        
        // Apply collision detection if enabled
        if self.settings.collision_detection {
            self.apply_collision_forces();
        }
        
        // Integrate forces and update positions
        self.integrate_forces(delta_time);
    }
    
    /// Synchronize physics bodies back to scene nodes
    pub fn sync_to_scene(&self, scene: &mut Scene) {
        for (id, body) in &self.bodies {
            if let Some(node) = scene.get_node_mut(*id) {
                node.position = body.position;
                node.velocity = body.velocity;
            }
        }
    }
    
    /// Set a node as fixed (won't move during simulation)
    pub fn set_node_fixed(&mut self, id: SceneId, fixed: bool) {
        if let Some(body) = self.bodies.get_mut(&id) {
            body.fixed = fixed;
        }
    }
    
    /// Apply a force to a specific node
    pub fn apply_force(&mut self, id: SceneId, force: Vector3<f32>) {
        if let Some(existing_force) = self.forces.get_mut(&id) {
            *existing_force += force;
        }
    }
    
    /// Apply force-directed layout algorithm
    fn apply_force_directed_layout(&mut self) {
        let config = &self.layout_config.force_directed;
        let bodies: Vec<_> = self.bodies.values().cloned().collect();
        
        for i in 0..bodies.len() {
            for j in (i + 1)..bodies.len() {
                let body1 = &bodies[i];
                let body2 = &bodies[j];
                
                let diff = body2.position - body1.position;
                let distance = diff.magnitude();
                
                if distance > 0.0 {
                    let direction = diff / distance;
                    
                    // Calculate force magnitude based on distance
                    let force_magnitude = if distance < config.optimal_edge_length {
                        // Repulsion when too close
                        -config.repulsion_strength * (config.optimal_edge_length - distance) / config.optimal_edge_length
                    } else {
                        // Attraction when too far
                        config.attraction_strength * (distance - config.optimal_edge_length) / config.optimal_edge_length
                    };
                    
                    let force = direction * force_magnitude.min(config.max_force);
                    
                    // Apply forces (Newton's third law)
                    if let Some(force1) = self.forces.get_mut(&body1.id) {
                        *force1 += force;
                    }
                    if let Some(force2) = self.forces.get_mut(&body2.id) {
                        *force2 -= force;
                    }
                }
            }
        }
    }
    
    /// Apply repulsion forces between all nodes
    fn apply_repulsion_forces(&mut self) {
        let config = &self.layout_config.repulsion;
        let bodies: Vec<_> = self.bodies.values().cloned().collect();
        
        for i in 0..bodies.len() {
            for j in (i + 1)..bodies.len() {
                let body1 = &bodies[i];
                let body2 = &bodies[j];
                
                let diff = body1.position - body2.position;
                let distance = diff.magnitude();
                
                if distance > 0.0 && distance < config.distance_threshold {
                    let direction = diff / distance;
                    let force_magnitude = config.base_force / distance.powf(config.falloff_exponent);
                    let force = direction * force_magnitude;
                    
                    // Apply repulsion forces
                    if let Some(force1) = self.forces.get_mut(&body1.id) {
                        *force1 += force;
                    }
                    if let Some(force2) = self.forces.get_mut(&body2.id) {
                        *force2 -= force;
                    }
                }
            }
        }
    }
    
    /// Apply collision detection and response
    fn apply_collision_forces(&mut self) {
        let bodies: Vec<_> = self.bodies.values().cloned().collect();
        
        for i in 0..bodies.len() {
            for j in (i + 1)..bodies.len() {
                let body1 = &bodies[i];
                let body2 = &bodies[j];
                
                let diff = body1.position - body2.position;
                let distance = diff.magnitude();
                let min_distance = body1.radius + body2.radius;
                
                if distance < min_distance && distance > 0.0 {
                    let direction = diff / distance;
                    let overlap = min_distance - distance;
                    let force_magnitude = overlap * 100.0; // Stiff collision response
                    let force = direction * force_magnitude;
                    
                    // Apply collision forces
                    if let Some(force1) = self.forces.get_mut(&body1.id) {
                        *force1 += force;
                    }
                    if let Some(force2) = self.forces.get_mut(&body2.id) {
                        *force2 -= force;
                    }
                }
            }
        }
    }
    
    /// Integrate forces and update positions
    fn integrate_forces(&mut self, delta_time: f32) {
        for (id, body) in self.bodies.iter_mut() {
            if body.fixed {
                continue;
            }
            
            if let Some(force) = self.forces.get(id) {
                // F = ma, so a = F/m
                let acceleration = *force / body.mass;
                
                // Update velocity with damping
                body.velocity += acceleration * delta_time;
                body.velocity *= 1.0 - self.settings.damping * delta_time;
                
                // Clamp velocity
                let speed = body.velocity.magnitude();
                if speed > self.settings.max_velocity {
                    body.velocity = body.velocity / speed * self.settings.max_velocity;
                }
                
                // Update position
                body.position += body.velocity * delta_time;
            }
        }
    }
    
    /// Get current physics settings
    pub fn settings(&self) -> &PhysicsSettings {
        &self.settings
    }
    
    /// Get mutable physics settings
    pub fn settings_mut(&mut self) -> &mut PhysicsSettings {
        &mut self.settings
    }
    
    /// Get layout configuration
    pub fn layout_config(&self) -> &LayoutConfig {
        &self.layout_config
    }
    
    /// Get mutable layout configuration
    pub fn layout_config_mut(&mut self) -> &mut LayoutConfig {
        &mut self.layout_config
    }
    
    /// Check if a physics body exists for a given node ID
    pub fn has_body(&self, id: SceneId) -> bool {
        self.bodies.contains_key(&id)
    }
}

impl Default for PhysicsSettings {
    fn default() -> Self {
        PhysicsSettings {
            damping: 0.9,
            time_step: 1.0 / 60.0, // 60 FPS
            max_velocity: 50.0,
            min_distance: 0.1,
            collision_detection: true,
        }
    }
}

impl Default for LayoutConfig {
    fn default() -> Self {
        LayoutConfig {
            force_directed: ForceDirectedConfig::default(),
            spring_damper: SpringDamperConfig::default(),
            repulsion: RepulsionConfig::default(),
        }
    }
}

impl Default for ForceDirectedConfig {
    fn default() -> Self {
        ForceDirectedConfig {
            attraction_strength: 0.1,
            repulsion_strength: 10.0,
            optimal_edge_length: 3.0,
            max_force: 5.0,
        }
    }
}

impl Default for SpringDamperConfig {
    fn default() -> Self {
        SpringDamperConfig {
            spring_constant: 0.5,
            damper_constant: 0.1,
            rest_length: 2.0,
        }
    }
}

impl Default for RepulsionConfig {
    fn default() -> Self {
        RepulsionConfig {
            base_force: 5.0,
            distance_threshold: 10.0,
            falloff_exponent: 2.0,
        }
    }
}