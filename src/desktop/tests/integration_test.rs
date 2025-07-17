//! Integration tests for the graph desktop compositor

use horizonos_graph_engine::scene::{Scene, SceneId};
use horizonos_graph_nodes::{NodeManager, application::ApplicationNode};
use horizonos_graph_workspaces::WorkspaceManager;
use horizonos_graph_clustering::ClusteringSystem;
use horizonos_graph_interaction::InteractionSystem;

#[test]
fn test_scene_creation_and_node_management() {
    // Create a scene
    let mut scene = Scene::new();
    
    // Create node manager
    let node_manager = NodeManager::new();
    
    // Create and add an application node
    let app_node = ApplicationNode::new(
        12345,
        "Firefox".to_string(),
        "firefox".to_string(),
    );
    
    let node_id = node_manager.create_application_node(
        12345,
        "Firefox".to_string(), 
        "firefox".to_string(),
    );
    
    // Verify node was created
    assert!(node_manager.get_node(node_id).is_some());
    
    // Sync to scene
    node_manager.sync_to_scene(&mut scene);
}

#[tokio::test]
async fn test_workspace_management() {
    let mut workspace_manager = WorkspaceManager::new();
    
    // Initialize workspace manager
    workspace_manager.initialize().await.unwrap();
    
    // Create a new workspace
    let workspace_id = workspace_manager.create_workspace(
        "Development",
        "Development workspace for coding"
    ).unwrap();
    
    // Switch to the new workspace
    workspace_manager.switch_workspace(&workspace_id).unwrap();
    
    // Verify active workspace
    let active = workspace_manager.get_active_workspace();
    assert!(active.is_some());
    assert_eq!(active.unwrap().id, workspace_id);
    
    // List workspaces
    let workspaces = workspace_manager.list_workspaces();
    assert!(workspaces.len() >= 2); // Default + Development
}

#[test]
fn test_clustering_system() {
    let mut scene = Scene::new();
    let clustering = ClusteringSystem::new();
    
    // Add some test nodes to the scene
    // This would normally be done through the node manager
    
    // Detect clusters
    let detected = clustering.detect_clusters(&scene);
    assert!(detected.is_ok());
}

#[test]
fn test_interaction_system() {
    let mut scene = Scene::new();
    let interaction = InteractionSystem::new();
    
    // Test gesture recognition
    let gesture = interaction.gesture_recognizer.start_gesture(
        [100.0, 100.0].into(),
        std::time::Instant::now(),
    );
    
    // Add points to form a gesture
    interaction.gesture_recognizer.add_point(
        &gesture,
        [150.0, 150.0].into(),
        std::time::Instant::now(),
    );
    
    interaction.gesture_recognizer.add_point(
        &gesture,
        [200.0, 200.0].into(),
        std::time::Instant::now(),
    );
    
    // End gesture and recognize
    let result = interaction.gesture_recognizer.end_gesture(
        &gesture,
        std::time::Instant::now(),
    );
    
    assert!(result.is_some());
}

#[test]
fn test_graph_operations() {
    use horizonos_graph_edges::{EdgeManager, EdgeType};
    use nalgebra::Point3;
    
    let mut scene = Scene::new();
    let mut edge_manager = EdgeManager::new();
    
    // Create two nodes
    let node1 = SceneId::new();
    let node2 = SceneId::new();
    
    // Create an edge between them
    let edge_id = edge_manager.create_edge(
        node1,
        node2,
        EdgeType::Relationship,
    );
    
    // Verify edge exists
    assert!(edge_manager.get_edge(edge_id).is_some());
    
    // Test edge rendering
    let edges = edge_manager.get_edges_for_rendering();
    assert_eq!(edges.len(), 1);
}

#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;
    
    #[test]
    fn test_large_graph_performance() {
        let mut scene = Scene::new();
        let node_manager = NodeManager::new();
        
        let start = Instant::now();
        
        // Create 1000 nodes
        for i in 0..1000 {
            node_manager.create_application_node(
                i,
                format!("App {}", i),
                format!("app{}", i),
            );
        }
        
        let creation_time = start.elapsed();
        println!("Created 1000 nodes in {:?}", creation_time);
        
        // Sync to scene
        let sync_start = Instant::now();
        node_manager.sync_to_scene(&mut scene);
        let sync_time = sync_start.elapsed();
        println!("Synced to scene in {:?}", sync_time);
        
        // Assert reasonable performance
        assert!(creation_time.as_secs() < 1);
        assert!(sync_time.as_millis() < 100);
    }
}