//! HorizonOS Graph Desktop Rendering Engine
//! 
//! Core rendering and physics engine for the graph-based desktop environment.
//! Provides WebGPU-accelerated rendering, physics simulation, and camera controls.

pub mod renderer;
pub mod physics;
pub mod camera;
pub mod scene;
pub mod error;

pub use renderer::*;
pub use physics::*;
pub use camera::*;
pub use scene::*;
pub use error::*;

use std::sync::Arc;
use wgpu::{Device, Queue, Surface};
use winit::window::Window;

/// Main graph engine that coordinates rendering, physics, and scene management
pub struct GraphEngine {
    /// WebGPU device for GPU operations
    device: Arc<Device>,
    /// Command queue for GPU commands
    queue: Arc<Queue>,
    /// Window surface for rendering
    surface: Surface<'static>,
    /// Scene graph containing all nodes and edges
    scene: Scene,
    /// Physics simulation engine
    physics: PhysicsEngine,
    /// Camera controller for navigation
    camera: Camera,
    /// Main renderer
    renderer: Renderer,
}

impl GraphEngine {
    /// Initialize the graph engine with a window
    pub async fn new(window: Arc<Window>) -> Result<Self, GraphEngineError> {
        log::info!("Initializing HorizonOS Graph Engine");
        
        // Initialize WebGPU
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        
        let surface = instance.create_surface(window.clone())?;
        
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or(GraphEngineError::AdapterNotFound)?;
            
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("HorizonOS Graph Engine Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                },
                None,
            )
            .await?;
            
        let device = Arc::new(device);
        let queue = Arc::new(queue);
        
        // Initialize components
        let scene = Scene::new();
        let physics = PhysicsEngine::new();
        let camera = Camera::new();
        let renderer = Renderer::new(device.clone(), queue.clone(), &surface, &window, &adapter).await?;
        
        log::info!("Graph engine initialized successfully");
        
        Ok(GraphEngine {
            device,
            queue,
            surface,
            scene,
            physics,
            camera,
            renderer,
        })
    }
    
    /// Update the engine state (physics, animations, etc.)
    pub fn update(&mut self, delta_time: f32) -> Result<(), GraphEngineError> {
        // Update physics simulation
        self.physics.step(delta_time);
        
        // Update scene animations
        self.scene.update(delta_time);
        
        // Update camera
        self.camera.update(delta_time);
        
        Ok(())
    }
    
    /// Render the current frame
    pub fn render(&mut self) -> Result<(), GraphEngineError> {
        self.renderer.render(&self.surface, &self.scene, &self.camera)?;
        Ok(())
    }
    
    /// Resize the rendering surface
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) -> Result<(), GraphEngineError> {
        self.renderer.resize(&self.surface, new_size)?;
        self.camera.set_aspect_ratio(new_size.width as f32 / new_size.height as f32);
        Ok(())
    }
    
    /// Get mutable reference to the scene
    pub fn scene_mut(&mut self) -> &mut Scene {
        &mut self.scene
    }
    
    /// Get reference to the scene
    pub fn scene(&self) -> &Scene {
        &self.scene
    }
    
    /// Get mutable reference to the camera
    pub fn camera_mut(&mut self) -> &mut Camera {
        &mut self.camera
    }
    
    /// Get reference to the camera
    pub fn camera(&self) -> &Camera {
        &self.camera
    }
    
    /// Get mutable reference to the physics engine
    pub fn physics_mut(&mut self) -> &mut PhysicsEngine {
        &mut self.physics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scene_creation() {
        let scene = Scene::new();
        assert_eq!(scene.nodes().count(), 0);
        assert_eq!(scene.edges().count(), 0);
    }

    #[test]
    fn test_node_creation() {
        let mut scene = Scene::new();
        
        let node = SceneNode {
            id: 0,
            position: nalgebra::Point3::new(0.0, 0.0, 0.0),
            velocity: nalgebra::Vector3::zeros(),
            radius: 1.0,
            color: [1.0, 0.0, 0.0, 1.0],
            node_type: NodeType::Application { 
                pid: 123, 
                name: "Test App".to_string() 
            },
            metadata: NodeMetadata::default(),
            visible: true,
            selected: false,
        };
        
        let node_id = scene.add_node(node);
        assert_eq!(scene.nodes().count(), 1);
        
        let retrieved_node = scene.get_node(node_id).unwrap();
        assert_eq!(retrieved_node.radius, 1.0);
        
        if let NodeType::Application { name, .. } = &retrieved_node.node_type {
            assert_eq!(name, "Test App");
        } else {
            panic!("Wrong node type");
        }
    }

    #[test]
    fn test_edge_creation() {
        let mut scene = Scene::new();
        
        // Create two nodes
        let node1 = SceneNode {
            id: 0,
            position: nalgebra::Point3::new(-1.0, 0.0, 0.0),
            velocity: nalgebra::Vector3::zeros(),
            radius: 1.0,
            color: [1.0, 0.0, 0.0, 1.0],
            node_type: NodeType::File { 
                path: "/test.txt".to_string(),
                file_type: FileType::Document,
            },
            metadata: NodeMetadata::default(),
            visible: true,
            selected: false,
        };
        
        let node2 = SceneNode {
            id: 1,
            position: nalgebra::Point3::new(1.0, 0.0, 0.0),
            velocity: nalgebra::Vector3::zeros(),
            radius: 1.0,
            color: [0.0, 1.0, 0.0, 1.0],
            node_type: NodeType::Person { 
                name: "Alice".to_string(),
                contact_info: ContactInfo {
                    email: Some("alice@test.com".to_string()),
                    phone: None,
                    social: std::collections::HashMap::new(),
                },
            },
            metadata: NodeMetadata::default(),
            visible: true,
            selected: false,
        };
        
        let node1_id = scene.add_node(node1);
        let node2_id = scene.add_node(node2);
        
        // Create edge
        let edge = SceneEdge {
            id: 0,
            source: node1_id,
            target: node2_id,
            edge_type: EdgeType::CreatedBy,
            weight: 1.0,
            color: [0.5, 0.5, 0.5, 1.0],
            visible: true,
            animated: false,
        };
        
        let edge_id = scene.add_edge(edge);
        assert_eq!(scene.edges().count(), 1);
        
        let retrieved_edge = scene.get_edge(edge_id).unwrap();
        assert_eq!(retrieved_edge.source, node1_id);
        assert_eq!(retrieved_edge.target, node2_id);
        
        // Test edge connectivity
        let connected_edges = scene.get_connected_edges(node1_id);
        assert_eq!(connected_edges.len(), 1);
        assert_eq!(connected_edges[0].id, edge_id);
    }

    #[test]
    fn test_physics_engine() {
        let mut physics = PhysicsEngine::new();
        
        // Create a mock node for testing
        let node = SceneNode {
            id: 0,
            position: nalgebra::Point3::new(0.0, 0.0, 0.0),
            velocity: nalgebra::Vector3::zeros(),
            radius: 1.0,
            color: [1.0, 0.0, 0.0, 1.0],
            node_type: NodeType::System { 
                component: "test".to_string(),
                status: SystemStatus::Running,
            },
            metadata: NodeMetadata::default(),
            visible: true,
            selected: false,
        };
        
        physics.add_body(&node);
        
        // Apply a force and step simulation
        physics.apply_force(node.id, nalgebra::Vector3::new(1.0, 0.0, 0.0));
        physics.step(1.0 / 60.0); // 60 FPS timestep
        
        // The physics body should exist
        assert!(physics.has_body(node.id));
    }

    #[test]
    fn test_camera_controls() {
        let mut camera = Camera::new();
        
        let initial_position = camera.position;
        camera.move_forward(1.0);
        
        // Camera should have moved
        assert_ne!(camera.position, initial_position);
        
        // Test view matrix generation
        let view_matrix = camera.view_matrix();
        assert!(!view_matrix.determinant().is_nan());
        
        // Test projection matrix
        let proj_matrix = camera.projection_matrix();
        assert!(!proj_matrix.determinant().is_nan());
    }

    #[test]
    fn test_spatial_queries() {
        let mut scene = Scene::new();
        
        // Create nodes at different positions
        let positions = [
            nalgebra::Point3::new(0.0, 0.0, 0.0),
            nalgebra::Point3::new(2.0, 0.0, 0.0),
            nalgebra::Point3::new(5.0, 0.0, 0.0),
        ];
        
        for (i, pos) in positions.iter().enumerate() {
            let node = SceneNode {
                id: i as u64,
                position: *pos,
                velocity: nalgebra::Vector3::zeros(),
                radius: 0.5,
                color: [1.0, 1.0, 1.0, 1.0],
                node_type: NodeType::Concept { 
                    title: format!("Node {}", i),
                    content: "Test content".to_string(),
                },
                metadata: NodeMetadata::default(),
                visible: true,
                selected: false,
            };
            scene.add_node(node);
        }
        
        // Find nodes within radius of origin
        let nearby = scene.find_nodes_in_radius(nalgebra::Point3::new(0.0, 0.0, 0.0), 3.0);
        assert_eq!(nearby.len(), 2); // Should find first two nodes
        
        let far_away = scene.find_nodes_in_radius(nalgebra::Point3::new(0.0, 0.0, 0.0), 1.0);
        assert_eq!(far_away.len(), 1); // Should find only first node
    }
}