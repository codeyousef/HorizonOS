//! Graph Engine Integration Tests
//! 
//! Comprehensive integration tests for the HorizonOS graph engine system,
//! including GPU initialization, scene management, and rendering pipeline.

use std::sync::Arc;
use tokio::time::{timeout, Duration};
use horizonos_graph_engine::{
    GraphEngine, Scene, SceneNode, SceneId, NodeType, SystemStatus, NodeMetadata
};
use horizonos_graph_edges::{EdgeManager, EdgeType, Edge};
use horizonos_graph_layout::{LayoutManager, ForceDirectedLayout, CircularLayout};
use horizonos_graph_performance::{PerformanceManager, PerformanceMetrics};
use anyhow::Result;
use nalgebra::Vector3;

/// Graph Engine Integration Test Suite
pub struct GraphEngineIntegrationTests {
    engine: Arc<GraphEngine>,
    scene: Scene,
    edge_manager: EdgeManager,
    layout_manager: LayoutManager,
    performance_manager: PerformanceManager,
}

impl GraphEngineIntegrationTests {
    /// Create new integration test suite
    pub async fn new() -> Result<Self> {
        let engine = Arc::new(GraphEngine::new().await?);
        let scene = Scene::new();
        let edge_manager = EdgeManager::new();
        let layout_manager = LayoutManager::new();
        let performance_manager = PerformanceManager::new();

        Ok(Self {
            engine,
            scene,
            edge_manager,
            layout_manager,
            performance_manager,
        })
    }

    /// Run all graph engine integration tests
    pub async fn run_all_tests(&mut self) -> Result<TestResults> {
        let mut results = TestResults::new();

        // Core engine tests
        results.add_test("engine_initialization", self.test_engine_initialization().await);
        results.add_test("scene_management", self.test_scene_management().await);
        results.add_test("node_operations", self.test_node_operations().await);
        results.add_test("edge_operations", self.test_edge_operations().await);
        
        // Layout tests
        results.add_test("force_directed_layout", self.test_force_directed_layout().await);
        results.add_test("circular_layout", self.test_circular_layout().await);
        results.add_test("layout_transitions", self.test_layout_transitions().await);
        
        // Performance tests
        results.add_test("performance_metrics", self.test_performance_metrics().await);
        results.add_test("gpu_utilization", self.test_gpu_utilization().await);
        results.add_test("memory_management", self.test_memory_management().await);
        
        // Integration tests
        results.add_test("engine_layout_integration", self.test_engine_layout_integration().await);
        results.add_test("real_time_updates", self.test_real_time_updates().await);
        results.add_test("large_graph_performance", self.test_large_graph_performance().await);

        Ok(results)
    }

    /// Test engine initialization
    async fn test_engine_initialization(&self) -> TestResult {
        let test_name = "Engine Initialization";
        
        // Test engine is properly initialized
        if !self.engine.is_initialized() {
            return TestResult::failed(test_name, "Engine not initialized");
        }
        
        // Test GPU context is available
        if !self.engine.has_gpu_context() {
            return TestResult::skipped(test_name, "No GPU context available");
        }
        
        // Test engine capabilities
        let capabilities = self.engine.get_capabilities();
        if capabilities.max_nodes == 0 {
            return TestResult::failed(test_name, "Engine capabilities not detected");
        }
        
        TestResult::passed(test_name)
    }

    /// Test scene management operations
    async fn test_scene_management(&mut self) -> TestResult {
        let test_name = "Scene Management";
        
        // Test scene creation
        let scene_id = self.scene.create_subscene("test_scene").await
            .map_err(|e| format!("Failed to create scene: {}", e))
            .unwrap();
        
        // Test scene exists
        if !self.scene.has_subscene(&scene_id) {
            return TestResult::failed(test_name, "Scene not created");
        }
        
        // Test scene deletion
        self.scene.delete_subscene(&scene_id).await
            .map_err(|e| format!("Failed to delete scene: {}", e))
            .unwrap();
        
        if self.scene.has_subscene(&scene_id) {
            return TestResult::failed(test_name, "Scene not deleted");
        }
        
        TestResult::passed(test_name)
    }

    /// Test node operations
    async fn test_node_operations(&mut self) -> TestResult {
        let test_name = "Node Operations";
        
        // Create test nodes
        let node1_id = SceneId::new();
        let node2_id = SceneId::new();
        
        let node1 = SceneNode {
            id: node1_id,
            position: Vector3::new(0.0, 0.0, 0.0),
            velocity: Vector3::zeros(),
            radius: 1.0,
            color: [1.0, 0.0, 0.0, 1.0],
            node_type: NodeType::Application {
                name: "TestApp".to_string(),
                executable: "/usr/bin/testapp".to_string(),
                icon: None,
            },
            metadata: NodeMetadata::default(),
            visible: true,
            selected: false,
        };
        
        let node2 = SceneNode {
            id: node2_id,
            position: Vector3::new(5.0, 0.0, 0.0),
            velocity: Vector3::zeros(),
            radius: 1.0,
            color: [0.0, 1.0, 0.0, 1.0],
            node_type: NodeType::File {
                path: "/home/user/test.txt".to_string(),
                file_type: "text/plain".to_string(),
                size: 1024,
                modified: chrono::Utc::now(),
            },
            metadata: NodeMetadata::default(),
            visible: true,
            selected: false,
        };
        
        // Add nodes to scene
        self.scene.add_node(node1);
        self.scene.add_node(node2);
        
        // Test node existence
        if !self.scene.has_node(node1_id) {
            return TestResult::failed(test_name, "Node 1 not added");
        }
        
        if !self.scene.has_node(node2_id) {
            return TestResult::failed(test_name, "Node 2 not added");
        }
        
        // Test node retrieval
        let retrieved_node = self.scene.get_node(node1_id);
        if retrieved_node.is_none() {
            return TestResult::failed(test_name, "Node not retrievable");
        }
        
        // Test node updates
        let mut updated_node = retrieved_node.unwrap().clone();
        updated_node.position = Vector3::new(10.0, 10.0, 0.0);
        
        self.scene.update_node(updated_node);
        
        let final_node = self.scene.get_node(node1_id).unwrap();
        if final_node.position != Vector3::new(10.0, 10.0, 0.0) {
            return TestResult::failed(test_name, "Node not updated");
        }
        
        TestResult::passed(test_name)
    }

    /// Test edge operations
    async fn test_edge_operations(&mut self) -> TestResult {
        let test_name = "Edge Operations";
        
        // Get nodes from scene
        let nodes: Vec<_> = self.scene.get_all_nodes().into_iter().take(2).collect();
        if nodes.len() < 2 {
            return TestResult::failed(test_name, "Not enough nodes for edge test");
        }
        
        let node1_id = nodes[0].id;
        let node2_id = nodes[1].id;
        
        // Create edge
        let edge = Edge {
            id: SceneId::new(),
            source: node1_id,
            target: node2_id,
            edge_type: EdgeType::Dependency,
            weight: 1.0,
            color: [0.5, 0.5, 0.5, 1.0],
            visible: true,
            metadata: std::collections::HashMap::new(),
        };
        
        let edge_id = edge.id;
        
        // Add edge to manager
        self.edge_manager.add_edge(edge).await
            .map_err(|e| format!("Failed to add edge: {}", e))
            .unwrap();
        
        // Test edge existence
        if !self.edge_manager.has_edge(edge_id) {
            return TestResult::failed(test_name, "Edge not added");
        }
        
        // Test edge retrieval
        let retrieved_edge = self.edge_manager.get_edge(edge_id);
        if retrieved_edge.is_none() {
            return TestResult::failed(test_name, "Edge not retrievable");
        }
        
        // Test edge connections
        let connected_edges = self.edge_manager.get_edges_for_node(node1_id);
        if connected_edges.is_empty() {
            return TestResult::failed(test_name, "Edge connections not working");
        }
        
        TestResult::passed(test_name)
    }

    /// Test force-directed layout
    async fn test_force_directed_layout(&mut self) -> TestResult {
        let test_name = "Force Directed Layout";
        
        // Apply force-directed layout
        let layout = ForceDirectedLayout::new();
        let result = self.layout_manager.apply_layout(&mut self.scene, Box::new(layout)).await;
        
        match result {
            Ok(_) => {
                // Verify nodes have moved
                let nodes = self.scene.get_all_nodes();
                if nodes.len() < 2 {
                    return TestResult::failed(test_name, "Not enough nodes for layout test");
                }
                
                // Check if nodes have reasonable positions
                let mut valid_positions = 0;
                for node in &nodes {
                    if node.position.magnitude() > 0.1 {
                        valid_positions += 1;
                    }
                }
                
                if valid_positions == 0 {
                    return TestResult::failed(test_name, "Layout did not move nodes");
                }
                
                TestResult::passed(test_name)
            }
            Err(e) => TestResult::failed(test_name, &format!("Layout failed: {}", e)),
        }
    }

    /// Test circular layout
    async fn test_circular_layout(&mut self) -> TestResult {
        let test_name = "Circular Layout";
        
        // Apply circular layout
        let layout = CircularLayout::new();
        let result = self.layout_manager.apply_layout(&mut self.scene, Box::new(layout)).await;
        
        match result {
            Ok(_) => {
                // Verify nodes are arranged in a circle
                let nodes = self.scene.get_all_nodes();
                if nodes.len() < 2 {
                    return TestResult::failed(test_name, "Not enough nodes for layout test");
                }
                
                // Check if nodes are roughly equidistant from center
                let center = Vector3::zeros();
                let mut distances = Vec::new();
                
                for node in &nodes {
                    let distance = (node.position - center).magnitude();
                    distances.push(distance);
                }
                
                // Check if distances are roughly equal (within 20% tolerance)
                if distances.len() > 1 {
                    let avg_distance = distances.iter().sum::<f32>() / distances.len() as f32;
                    let max_deviation = distances.iter()
                        .map(|d| (d - avg_distance).abs() / avg_distance)
                        .fold(0.0, f32::max);
                    
                    if max_deviation > 0.2 {
                        return TestResult::failed(test_name, "Nodes not arranged in circle");
                    }
                }
                
                TestResult::passed(test_name)
            }
            Err(e) => TestResult::failed(test_name, &format!("Layout failed: {}", e)),
        }
    }

    /// Test layout transitions
    async fn test_layout_transitions(&mut self) -> TestResult {
        let test_name = "Layout Transitions";
        
        // Apply first layout
        let layout1 = ForceDirectedLayout::new();
        self.layout_manager.apply_layout(&mut self.scene, Box::new(layout1)).await
            .map_err(|e| format!("First layout failed: {}", e))
            .unwrap();
        
        // Store positions
        let positions_before: Vec<_> = self.scene.get_all_nodes()
            .into_iter()
            .map(|n| (n.id, n.position))
            .collect();
        
        // Apply second layout with transition
        let layout2 = CircularLayout::new();
        self.layout_manager.apply_layout_with_transition(&mut self.scene, Box::new(layout2), Duration::from_millis(500)).await
            .map_err(|e| format!("Layout transition failed: {}", e))
            .unwrap();
        
        // Check if positions changed
        let positions_after: Vec<_> = self.scene.get_all_nodes()
            .into_iter()
            .map(|n| (n.id, n.position))
            .collect();
        
        let mut positions_changed = false;
        for (before, after) in positions_before.iter().zip(positions_after.iter()) {
            if before.0 == after.0 && (before.1 - after.1).magnitude() > 0.1 {
                positions_changed = true;
                break;
            }
        }
        
        if !positions_changed {
            return TestResult::failed(test_name, "Layout transition did not change positions");
        }
        
        TestResult::passed(test_name)
    }

    /// Test performance metrics
    async fn test_performance_metrics(&mut self) -> TestResult {
        let test_name = "Performance Metrics";
        
        // Start performance monitoring
        self.performance_manager.start_monitoring().await
            .map_err(|e| format!("Failed to start monitoring: {}", e))
            .unwrap();
        
        // Perform some operations
        for i in 0..100 {
            let node = SceneNode {
                id: SceneId::new(),
                position: Vector3::new(i as f32, 0.0, 0.0),
                velocity: Vector3::zeros(),
                radius: 0.5,
                color: [0.0, 0.0, 1.0, 1.0],
                node_type: NodeType::System {
                    component: format!("component_{}", i),
                    status: SystemStatus::Running,
                },
                metadata: NodeMetadata::default(),
                visible: true,
                selected: false,
            };
            
            self.scene.add_node(node);
        }
        
        // Get metrics
        let metrics = self.performance_manager.get_metrics().await
            .map_err(|e| format!("Failed to get metrics: {}", e))
            .unwrap();
        
        // Verify metrics are reasonable
        if metrics.frame_time_ms == 0.0 {
            return TestResult::failed(test_name, "No frame time recorded");
        }
        
        if metrics.node_count == 0 {
            return TestResult::failed(test_name, "No nodes counted");
        }
        
        TestResult::passed(test_name)
    }

    /// Test GPU utilization
    async fn test_gpu_utilization(&mut self) -> TestResult {
        let test_name = "GPU Utilization";
        
        if !self.engine.has_gpu_context() {
            return TestResult::skipped(test_name, "No GPU context available");
        }
        
        // Perform GPU-intensive operations
        for _ in 0..10 {
            let result = self.engine.render_frame(&self.scene).await;
            if result.is_err() {
                return TestResult::failed(test_name, "GPU rendering failed");
            }
        }
        
        // Check GPU utilization
        let gpu_metrics = self.engine.get_gpu_metrics().await
            .map_err(|e| format!("Failed to get GPU metrics: {}", e))
            .unwrap();
        
        if gpu_metrics.utilization_percent == 0.0 {
            return TestResult::failed(test_name, "No GPU utilization recorded");
        }
        
        TestResult::passed(test_name)
    }

    /// Test memory management
    async fn test_memory_management(&mut self) -> TestResult {
        let test_name = "Memory Management";
        
        // Get initial memory usage
        let initial_memory = self.engine.get_memory_usage().await
            .map_err(|e| format!("Failed to get initial memory: {}", e))
            .unwrap();
        
        // Create many nodes
        let mut node_ids = Vec::new();
        for i in 0..1000 {
            let node_id = SceneId::new();
            let node = SceneNode {
                id: node_id,
                position: Vector3::new(i as f32, 0.0, 0.0),
                velocity: Vector3::zeros(),
                radius: 0.5,
                color: [1.0, 1.0, 0.0, 1.0],
                node_type: NodeType::System {
                    component: format!("memory_test_{}", i),
                    status: SystemStatus::Running,
                },
                metadata: NodeMetadata::default(),
                visible: true,
                selected: false,
            };
            
            self.scene.add_node(node);
            node_ids.push(node_id);
        }
        
        // Get memory after adding nodes
        let peak_memory = self.engine.get_memory_usage().await
            .map_err(|e| format!("Failed to get peak memory: {}", e))
            .unwrap();
        
        // Remove nodes
        for node_id in node_ids {
            self.scene.remove_node(node_id);
        }
        
        // Force garbage collection
        self.engine.collect_garbage().await
            .map_err(|e| format!("Failed to collect garbage: {}", e))
            .unwrap();
        
        // Get final memory usage
        let final_memory = self.engine.get_memory_usage().await
            .map_err(|e| format!("Failed to get final memory: {}", e))
            .unwrap();
        
        // Verify memory was reclaimed
        if final_memory.used_mb > peak_memory.used_mb {
            return TestResult::failed(test_name, "Memory not reclaimed");
        }
        
        TestResult::passed(test_name)
    }

    /// Test engine-layout integration
    async fn test_engine_layout_integration(&mut self) -> TestResult {
        let test_name = "Engine Layout Integration";
        
        // Create a complex scene
        self.create_complex_scene().await;
        
        // Apply layout and render
        let layout = ForceDirectedLayout::new();
        self.layout_manager.apply_layout(&mut self.scene, Box::new(layout)).await
            .map_err(|e| format!("Layout failed: {}", e))
            .unwrap();
        
        // Render the scene
        let render_result = self.engine.render_frame(&self.scene).await;
        
        match render_result {
            Ok(_) => TestResult::passed(test_name),
            Err(e) => TestResult::failed(test_name, &format!("Render failed: {}", e)),
        }
    }

    /// Test real-time updates
    async fn test_real_time_updates(&mut self) -> TestResult {
        let test_name = "Real Time Updates";
        
        // Start real-time update loop
        let update_handle = self.engine.start_real_time_updates(&mut self.scene).await
            .map_err(|e| format!("Failed to start updates: {}", e))
            .unwrap();
        
        // Let it run for a short time
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Stop updates
        self.engine.stop_real_time_updates(update_handle).await
            .map_err(|e| format!("Failed to stop updates: {}", e))
            .unwrap();
        
        TestResult::passed(test_name)
    }

    /// Test large graph performance
    async fn test_large_graph_performance(&mut self) -> TestResult {
        let test_name = "Large Graph Performance";
        
        let start_time = std::time::Instant::now();
        
        // Create large graph
        for i in 0..10000 {
            let node = SceneNode {
                id: SceneId::new(),
                position: Vector3::new(
                    (i % 100) as f32, 
                    (i / 100) as f32, 
                    0.0
                ),
                velocity: Vector3::zeros(),
                radius: 0.1,
                color: [0.5, 0.5, 0.5, 1.0],
                node_type: NodeType::System {
                    component: format!("large_test_{}", i),
                    status: SystemStatus::Running,
                },
                metadata: NodeMetadata::default(),
                visible: true,
                selected: false,
            };
            
            self.scene.add_node(node);
        }
        
        let creation_time = start_time.elapsed();
        
        // Apply layout
        let layout_start = std::time::Instant::now();
        let layout = ForceDirectedLayout::new();
        self.layout_manager.apply_layout(&mut self.scene, Box::new(layout)).await
            .map_err(|e| format!("Layout failed: {}", e))
            .unwrap();
        
        let layout_time = layout_start.elapsed();
        
        // Render frame
        let render_start = std::time::Instant::now();
        let render_result = self.engine.render_frame(&self.scene).await;
        let render_time = render_start.elapsed();
        
        // Check performance thresholds
        if creation_time.as_millis() > 5000 {
            return TestResult::failed(test_name, "Node creation too slow");
        }
        
        if layout_time.as_millis() > 10000 {
            return TestResult::failed(test_name, "Layout computation too slow");
        }
        
        if render_time.as_millis() > 1000 {
            return TestResult::failed(test_name, "Rendering too slow");
        }
        
        match render_result {
            Ok(_) => TestResult::passed(test_name),
            Err(e) => TestResult::failed(test_name, &format!("Render failed: {}", e)),
        }
    }

    /// Helper method to create a complex scene
    async fn create_complex_scene(&mut self) {
        // Create various types of nodes
        let app_node = SceneNode {
            id: SceneId::new(),
            position: Vector3::new(0.0, 0.0, 0.0),
            velocity: Vector3::zeros(),
            radius: 1.0,
            color: [1.0, 0.0, 0.0, 1.0],
            node_type: NodeType::Application {
                name: "Complex App".to_string(),
                executable: "/usr/bin/complex".to_string(),
                icon: None,
            },
            metadata: NodeMetadata::default(),
            visible: true,
            selected: false,
        };
        
        let file_node = SceneNode {
            id: SceneId::new(),
            position: Vector3::new(5.0, 0.0, 0.0),
            velocity: Vector3::zeros(),
            radius: 0.8,
            color: [0.0, 1.0, 0.0, 1.0],
            node_type: NodeType::File {
                path: "/home/user/important.doc".to_string(),
                file_type: "application/msword".to_string(),
                size: 1024000,
                modified: chrono::Utc::now(),
            },
            metadata: NodeMetadata::default(),
            visible: true,
            selected: false,
        };
        
        let person_node = SceneNode {
            id: SceneId::new(),
            position: Vector3::new(-5.0, 0.0, 0.0),
            velocity: Vector3::zeros(),
            radius: 1.2,
            color: [0.0, 0.0, 1.0, 1.0],
            node_type: NodeType::Person {
                name: "John Doe".to_string(),
                email: "john@example.com".to_string(),
                avatar: None,
            },
            metadata: NodeMetadata::default(),
            visible: true,
            selected: false,
        };
        
        self.scene.add_node(app_node);
        self.scene.add_node(file_node);
        self.scene.add_node(person_node);
    }
}

/// Test result for individual tests
#[derive(Debug, Clone)]
pub struct TestResult {
    pub name: String,
    pub status: TestStatus,
    pub message: Option<String>,
    pub duration: Duration,
}

impl TestResult {
    pub fn passed(name: &str) -> Self {
        Self {
            name: name.to_string(),
            status: TestStatus::Passed,
            message: None,
            duration: Duration::from_millis(0),
        }
    }

    pub fn failed(name: &str, message: &str) -> Self {
        Self {
            name: name.to_string(),
            status: TestStatus::Failed,
            message: Some(message.to_string()),
            duration: Duration::from_millis(0),
        }
    }

    pub fn skipped(name: &str, reason: &str) -> Self {
        Self {
            name: name.to_string(),
            status: TestStatus::Skipped,
            message: Some(reason.to_string()),
            duration: Duration::from_millis(0),
        }
    }
}

/// Test status enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum TestStatus {
    Passed,
    Failed,
    Skipped,
}

/// Collection of test results
#[derive(Debug)]
pub struct TestResults {
    pub tests: Vec<TestResult>,
}

impl TestResults {
    pub fn new() -> Self {
        Self { tests: Vec::new() }
    }

    pub fn add_test(&mut self, name: &str, result: TestResult) {
        self.tests.push(result);
    }

    pub fn passed_count(&self) -> usize {
        self.tests.iter().filter(|r| r.status == TestStatus::Passed).count()
    }

    pub fn failed_count(&self) -> usize {
        self.tests.iter().filter(|r| r.status == TestStatus::Failed).count()
    }

    pub fn skipped_count(&self) -> usize {
        self.tests.iter().filter(|r| r.status == TestStatus::Skipped).count()
    }

    pub fn success_rate(&self) -> f64 {
        if self.tests.is_empty() {
            0.0
        } else {
            self.passed_count() as f64 / self.tests.len() as f64
        }
    }

    pub fn print_summary(&self) {
        println!("=== Graph Engine Integration Test Results ===");
        println!("Total tests: {}", self.tests.len());
        println!("Passed: {}", self.passed_count());
        println!("Failed: {}", self.failed_count());
        println!("Skipped: {}", self.skipped_count());
        println!("Success rate: {:.1}%", self.success_rate() * 100.0);

        if self.failed_count() > 0 {
            println!("\n=== Failed Tests ===");
            for test in &self.tests {
                if test.status == TestStatus::Failed {
                    println!("❌ {}: {}", test.name, test.message.as_deref().unwrap_or("No message"));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_graph_engine_integration() {
        let mut suite = GraphEngineIntegrationTests::new().await
            .expect("Failed to create test suite");
        
        let results = suite.run_all_tests().await
            .expect("Failed to run tests");
        
        results.print_summary();
        
        // Assert that most tests pass
        assert!(results.passed_count() > 0, "No tests passed");
        assert!(results.success_rate() > 0.7, "Success rate too low: {:.1}%", results.success_rate() * 100.0);
    }
}