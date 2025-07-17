//! Basic example of the graph engine in action

use horizonos_graph_engine::{GraphEngine, Scene, SceneNode, SceneEdge, NodeType, EdgeType, NodeMetadata};
use nalgebra::Point3;
use std::sync::Arc;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    
    // Create window
    let event_loop = EventLoop::new()?;
    let window = Arc::new(
        WindowBuilder::new()
            .with_title("HorizonOS Graph Desktop - Basic Example")
            .with_inner_size(winit::dpi::LogicalSize::new(1200, 800))
            .build(&event_loop)?,
    );
    
    // Initialize graph engine
    let mut engine = GraphEngine::new(window.clone()).await?;
    
    // Create some sample nodes
    create_sample_graph(engine.scene_mut());
    
    // Position camera to view the graph
    engine.camera_mut().position = Point3::new(0.0, 0.0, 15.0);
    engine.camera_mut().look_at(Point3::new(0.0, 0.0, 0.0));
    
    log::info!("Starting graph desktop example");
    
    event_loop.run(move |event, target| {
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => match event {
                WindowEvent::CloseRequested => target.exit(),
                WindowEvent::Resized(physical_size) => {
                    if let Err(e) = engine.resize(*physical_size) {
                        log::error!("Resize error: {:?}", e);
                    }
                }
                WindowEvent::RedrawRequested => {
                    // Update engine
                    if let Err(e) = engine.update(1.0 / 60.0) {
                        log::error!("Update error: {:?}", e);
                    }
                    
                    // Render frame
                    match engine.render() {
                        Ok(_) => {},
                        Err(e) => {
                            log::error!("Render error: {:?}", e);
                        }
                    }
                }
                _ => {}
            },
            Event::AboutToWait => {
                window.request_redraw();
            }
            _ => {}
        }
    })?;
    
    Ok(())
}

fn create_sample_graph(scene: &mut Scene) {
    // Create some sample nodes
    let file_node = SceneNode {
        id: 0,
        position: Point3::new(-5.0, 0.0, 0.0),
        velocity: nalgebra::Vector3::zeros(),
        radius: 1.0,
        color: [0.3, 0.7, 0.9, 1.0], // Blue
        node_type: NodeType::File {
            path: "/home/user/documents/readme.txt".to_string(),
            file_type: horizonos_graph_engine::FileType::Document,
        },
        metadata: NodeMetadata::default(),
        visible: true,
        selected: false,
    };
    
    let app_node = SceneNode {
        id: 1,
        position: Point3::new(0.0, 0.0, 0.0),
        velocity: nalgebra::Vector3::zeros(),
        radius: 1.5,
        color: [0.9, 0.3, 0.3, 1.0], // Red
        node_type: NodeType::Application {
            pid: 1234,
            name: "Text Editor".to_string(),
        },
        metadata: NodeMetadata::default(),
        visible: true,
        selected: false,
    };
    
    let person_node = SceneNode {
        id: 2,
        position: Point3::new(5.0, 0.0, 0.0),
        velocity: nalgebra::Vector3::zeros(),
        radius: 1.2,
        color: [0.3, 0.9, 0.3, 1.0], // Green
        node_type: NodeType::Person {
            name: "Alice".to_string(),
            contact_info: horizonos_graph_engine::ContactInfo {
                email: Some("alice@example.com".to_string()),
                phone: None,
                social: std::collections::HashMap::new(),
            },
        },
        metadata: NodeMetadata::default(),
        visible: true,
        selected: false,
    };
    
    // Add nodes to scene
    let file_id = scene.add_node(file_node);
    let app_id = scene.add_node(app_node);
    let person_id = scene.add_node(person_node);
    
    // Create relationships
    let file_to_app_edge = SceneEdge {
        id: 0,
        source: file_id,
        target: app_id,
        edge_type: EdgeType::DependsOn,
        weight: 1.0,
        color: [0.8, 0.8, 0.8, 0.8],
        visible: true,
        animated: false,
    };
    
    let app_to_person_edge = SceneEdge {
        id: 1,
        source: app_id,
        target: person_id,
        edge_type: EdgeType::CreatedBy,
        weight: 1.0,
        color: [0.9, 0.9, 0.3, 0.8], // Yellow
        visible: true,
        animated: false,
    };
    
    scene.add_edge(file_to_app_edge);
    scene.add_edge(app_to_person_edge);
    
    log::info!("Created sample graph with {} nodes and {} edges", 
              scene.nodes().count(), 
              scene.edges().count());
}