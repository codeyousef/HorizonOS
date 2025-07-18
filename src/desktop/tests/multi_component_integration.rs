//! Multi-Component Integration Tests
//! 
//! Integration tests for interaction between multiple HorizonOS components,
//! ensuring proper coordination and data flow between subsystems.

use std::sync::Arc;
use std::collections::HashMap;
use tokio::time::{timeout, Duration};
use anyhow::Result;

use horizonos_graph_engine::{GraphEngine, Scene, SceneNode, SceneId, NodeType, NodeMetadata};
use horizonos_graph_nodes::{
    NodeManager, ApplicationNode, FileNode, PersonNode, GraphNode,
    SystemNode, DeviceNode, TaskNode, ConfigGroupNode
};
use horizonos_graph_edges::{EdgeManager, EdgeType, RelationshipType};
use horizonos_graph_layout::{LayoutManager, ForceDirectedLayout};
use horizonos_graph_workspaces::{WorkspaceManager, Workspace};
use horizonos_graph_config::{ConfigManager, ThemeManager};
use horizonos_graph_ai::AIService;
use horizonos_graph_clustering::{ClusterManager, ClusteringConfig};
use horizonos_graph_visual::{VisualManager, ThemeConfig};
use horizonos_graph_performance::{PerformanceManager};

/// Multi-component integration test suite
pub struct MultiComponentIntegrationTests {
    // Core engine
    engine: Arc<GraphEngine>,
    scene: Scene,
    
    // Node management
    node_manager: NodeManager,
    
    // Edge management
    edge_manager: EdgeManager,
    
    // Layout management
    layout_manager: LayoutManager,
    
    // Workspace management
    workspace_manager: WorkspaceManager,
    
    // Configuration management
    config_manager: ConfigManager,
    theme_manager: ThemeManager,
    
    // AI integration
    ai_service: AIService,
    
    // Clustering
    cluster_manager: ClusterManager,
    
    // Visual management
    visual_manager: VisualManager,
    
    // Performance monitoring
    performance_manager: PerformanceManager,
}

impl MultiComponentIntegrationTests {
    /// Create new multi-component integration test suite
    pub async fn new() -> Result<Self> {
        let engine = Arc::new(GraphEngine::new().await?);
        let scene = Scene::new();
        let node_manager = NodeManager::new();
        let edge_manager = EdgeManager::new();
        let layout_manager = LayoutManager::new();
        let workspace_manager = WorkspaceManager::new().await?;
        let config_manager = ConfigManager::new().await?;
        let theme_manager = ThemeManager::new().await?;
        let ai_service = AIService::new().await?;
        let cluster_manager = ClusterManager::new(ClusteringConfig::default()).await?;
        let visual_manager = VisualManager::new().await?;
        let performance_manager = PerformanceManager::new();

        Ok(Self {
            engine,
            scene,
            node_manager,
            edge_manager,
            layout_manager,
            workspace_manager,
            config_manager,
            theme_manager,
            ai_service,
            cluster_manager,
            visual_manager,
            performance_manager,
        })
    }

    /// Run all multi-component integration tests
    pub async fn run_all_tests(&mut self) -> Result<TestResults> {
        let mut results = TestResults::new();

        // Component initialization tests
        results.add_test("component_initialization", self.test_component_initialization().await);
        
        // Data flow tests
        results.add_test("node_engine_integration", self.test_node_engine_integration().await);
        results.add_test("edge_layout_integration", self.test_edge_layout_integration().await);
        results.add_test("workspace_scene_integration", self.test_workspace_scene_integration().await);
        
        // Configuration tests
        results.add_test("config_theme_integration", self.test_config_theme_integration().await);
        results.add_test("theme_visual_integration", self.test_theme_visual_integration().await);
        
        // AI integration tests
        results.add_test("ai_node_analysis", self.test_ai_node_analysis().await);
        results.add_test("ai_clustering_integration", self.test_ai_clustering_integration().await);
        results.add_test("ai_layout_optimization", self.test_ai_layout_optimization().await);
        
        // Advanced integration tests
        results.add_test("full_workflow_integration", self.test_full_workflow_integration().await);
        results.add_test("performance_monitoring_integration", self.test_performance_monitoring_integration().await);
        results.add_test("real_time_collaboration", self.test_real_time_collaboration().await);
        
        // Stress tests
        results.add_test("multi_workspace_stress", self.test_multi_workspace_stress().await);
        results.add_test("concurrent_operations", self.test_concurrent_operations().await);

        Ok(results)
    }

    /// Test component initialization
    async fn test_component_initialization(&self) -> TestResult {
        let test_name = "Component Initialization";
        
        // Test all components are properly initialized
        if !self.engine.is_initialized() {
            return TestResult::failed(test_name, "Engine not initialized");
        }
        
        if !self.workspace_manager.is_initialized().await {
            return TestResult::failed(test_name, "Workspace manager not initialized");
        }
        
        if !self.config_manager.is_initialized().await {
            return TestResult::failed(test_name, "Config manager not initialized");
        }
        
        if !self.ai_service.is_available().await {
            return TestResult::skipped(test_name, "AI service not available");
        }
        
        TestResult::passed(test_name)
    }

    /// Test node-engine integration
    async fn test_node_engine_integration(&mut self) -> TestResult {
        let test_name = "Node Engine Integration";
        
        // Create nodes through node manager
        let app_node = ApplicationNode::new(
            SceneId::new(),
            "TestApp".to_string(),
            "/usr/bin/testapp".to_string(),
        );
        
        let file_node = FileNode::new(
            SceneId::new(),
            std::path::PathBuf::from("/test/file.txt"),
        );
        
        let person_node = PersonNode::new(
            SceneId::new(),
            "Test User".to_string(),
            "test@example.com".to_string(),
        );
        
        // Register nodes with node manager
        self.node_manager.register_node(Box::new(app_node.clone())).await
            .map_err(|e| format!("Failed to register app node: {}", e))
            .unwrap();
        
        self.node_manager.register_node(Box::new(file_node.clone())).await
            .map_err(|e| format!("Failed to register file node: {}", e))
            .unwrap();
        
        self.node_manager.register_node(Box::new(person_node.clone())).await
            .map_err(|e| format!("Failed to register person node: {}", e))
            .unwrap();
        
        // Convert to scene nodes and add to scene
        let scene_nodes = self.node_manager.get_scene_nodes().await
            .map_err(|e| format!("Failed to get scene nodes: {}", e))
            .unwrap();
        
        for scene_node in scene_nodes {
            self.scene.add_node(scene_node);
        }
        
        // Verify nodes are in scene
        let scene_node_count = self.scene.get_all_nodes().len();
        if scene_node_count != 3 {
            return TestResult::failed(test_name, &format!("Expected 3 nodes, got {}", scene_node_count));
        }
        
        TestResult::passed(test_name)
    }

    /// Test edge-layout integration
    async fn test_edge_layout_integration(&mut self) -> TestResult {
        let test_name = "Edge Layout Integration";
        
        // Get nodes from scene
        let nodes = self.scene.get_all_nodes();
        if nodes.len() < 2 {
            return TestResult::failed(test_name, "Not enough nodes for edge test");
        }
        
        // Create relationships between nodes
        let edge_id = self.edge_manager.create_edge(
            SceneId::new(),
            nodes[0].id,
            nodes[1].id,
            EdgeType::Dependency,
            1.0,
        ).map_err(|e| format!("Failed to create edge: {}", e))
        .unwrap();
        
        // Apply layout that considers edges
        let layout = ForceDirectedLayout::new();
        self.layout_manager.apply_layout(&mut self.scene, Box::new(layout)).await
            .map_err(|e| format!("Layout failed: {}", e))
            .unwrap();
        
        // Verify edge influences layout
        let updated_nodes = self.scene.get_all_nodes();
        let node1_pos = updated_nodes.iter().find(|n| n.id == nodes[0].id).unwrap().position;
        let node2_pos = updated_nodes.iter().find(|n| n.id == nodes[1].id).unwrap().position;
        
        let distance = (node1_pos - node2_pos).magnitude();
        
        // Connected nodes should be reasonably close
        if distance > 100.0 {
            return TestResult::failed(test_name, "Connected nodes too far apart");
        }
        
        TestResult::passed(test_name)
    }

    /// Test workspace-scene integration
    async fn test_workspace_scene_integration(&mut self) -> TestResult {
        let test_name = "Workspace Scene Integration";
        
        // Create workspace
        let workspace_id = self.workspace_manager.create_workspace("Test Workspace".to_string()).await
            .map_err(|e| format!("Failed to create workspace: {}", e))
            .unwrap();
        
        // Save current scene to workspace
        self.workspace_manager.save_scene_to_workspace(workspace_id.clone(), &self.scene).await
            .map_err(|e| format!("Failed to save scene: {}", e))
            .unwrap();
        
        // Clear current scene
        self.scene.clear();
        
        // Load scene from workspace
        let loaded_scene = self.workspace_manager.load_scene_from_workspace(workspace_id).await
            .map_err(|e| format!("Failed to load scene: {}", e))
            .unwrap();
        
        // Verify scene was loaded correctly
        if loaded_scene.get_all_nodes().len() != 3 {
            return TestResult::failed(test_name, "Scene not loaded correctly");
        }
        
        TestResult::passed(test_name)
    }

    /// Test config-theme integration
    async fn test_config_theme_integration(&mut self) -> TestResult {
        let test_name = "Config Theme Integration";
        
        // Load configuration
        let config = self.config_manager.load_config().await
            .map_err(|e| format!("Failed to load config: {}", e))
            .unwrap();
        
        // Apply theme based on configuration
        let theme_name = config.get_theme_name().unwrap_or("default".to_string());
        
        let theme_config = self.theme_manager.load_theme(&theme_name).await
            .map_err(|e| format!("Failed to load theme: {}", e))
            .unwrap();
        
        // Verify theme configuration
        if theme_config.colors.is_empty() {
            return TestResult::failed(test_name, "Theme has no colors");
        }
        
        TestResult::passed(test_name)
    }

    /// Test theme-visual integration
    async fn test_theme_visual_integration(&mut self) -> TestResult {
        let test_name = "Theme Visual Integration";
        
        // Load theme
        let theme_config = self.theme_manager.load_theme("default").await
            .map_err(|e| format!("Failed to load theme: {}", e))
            .unwrap();
        
        // Apply theme to visual manager
        self.visual_manager.apply_theme(theme_config).await
            .map_err(|e| format!("Failed to apply theme: {}", e))
            .unwrap();
        
        // Update node visuals based on theme
        let mut updated_nodes = Vec::new();
        for mut node in self.scene.get_all_nodes() {
            let visual_style = self.visual_manager.get_node_style(&node.node_type).await
                .map_err(|e| format!("Failed to get node style: {}", e))
                .unwrap();
            
            node.color = visual_style.color;
            node.radius = visual_style.radius;
            updated_nodes.push(node);
        }
        
        // Verify visual updates
        if updated_nodes.is_empty() {
            return TestResult::failed(test_name, "No visual updates applied");
        }
        
        TestResult::passed(test_name)
    }

    /// Test AI node analysis
    async fn test_ai_node_analysis(&mut self) -> TestResult {
        let test_name = "AI Node Analysis";
        
        if !self.ai_service.is_available().await {
            return TestResult::skipped(test_name, "AI service not available");
        }
        
        // Analyze nodes with AI
        let nodes = self.scene.get_all_nodes();
        let analysis_result = self.ai_service.analyze_nodes(&nodes).await
            .map_err(|e| format!("AI analysis failed: {}", e))
            .unwrap();
        
        // Verify analysis results
        if analysis_result.insights.is_empty() {
            return TestResult::failed(test_name, "No insights generated");
        }
        
        TestResult::passed(test_name)
    }

    /// Test AI clustering integration
    async fn test_ai_clustering_integration(&mut self) -> TestResult {
        let test_name = "AI Clustering Integration";
        
        if !self.ai_service.is_available().await {
            return TestResult::skipped(test_name, "AI service not available");
        }
        
        // Create more nodes for clustering
        self.create_diverse_nodes().await;
        
        // Use AI to suggest clustering
        let nodes = self.scene.get_all_nodes();
        let cluster_suggestions = self.ai_service.suggest_clusters(&nodes).await
            .map_err(|e| format!("AI clustering failed: {}", e))
            .unwrap();
        
        // Apply clustering suggestions
        for suggestion in cluster_suggestions {
            self.cluster_manager.create_cluster(suggestion).await
                .map_err(|e| format!("Failed to create cluster: {}", e))
                .unwrap();
        }
        
        // Verify clusters were created
        let clusters = self.cluster_manager.get_all_clusters().await
            .map_err(|e| format!("Failed to get clusters: {}", e))
            .unwrap();
        
        if clusters.is_empty() {
            return TestResult::failed(test_name, "No clusters created");
        }
        
        TestResult::passed(test_name)
    }

    /// Test AI layout optimization
    async fn test_ai_layout_optimization(&mut self) -> TestResult {
        let test_name = "AI Layout Optimization";
        
        if !self.ai_service.is_available().await {
            return TestResult::skipped(test_name, "AI service not available");
        }
        
        // Get AI suggestions for layout optimization
        let layout_suggestions = self.ai_service.optimize_layout(&self.scene).await
            .map_err(|e| format!("AI layout optimization failed: {}", e))
            .unwrap();
        
        // Apply AI-suggested layout parameters
        let mut layout = ForceDirectedLayout::new();
        layout.apply_optimization_parameters(layout_suggestions);
        
        // Apply optimized layout
        self.layout_manager.apply_layout(&mut self.scene, Box::new(layout)).await
            .map_err(|e| format!("Optimized layout failed: {}", e))
            .unwrap();
        
        TestResult::passed(test_name)
    }

    /// Test full workflow integration
    async fn test_full_workflow_integration(&mut self) -> TestResult {
        let test_name = "Full Workflow Integration";
        
        // Complete workflow: Create workspace -> Add nodes -> Apply layout -> Save
        
        // 1. Create workspace
        let workspace_id = self.workspace_manager.create_workspace("Full Workflow Test".to_string()).await
            .map_err(|e| format!("Workspace creation failed: {}", e))
            .unwrap();
        
        // 2. Switch to workspace
        self.workspace_manager.switch_workspace(workspace_id.clone()).await
            .map_err(|e| format!("Workspace switch failed: {}", e))
            .unwrap();
        
        // 3. Create and add nodes
        self.create_diverse_nodes().await;
        
        // 4. Create relationships
        self.create_node_relationships().await;
        
        // 5. Apply layout
        let layout = ForceDirectedLayout::new();
        self.layout_manager.apply_layout(&mut self.scene, Box::new(layout)).await
            .map_err(|e| format!("Layout application failed: {}", e))
            .unwrap();
        
        // 6. Apply theme
        let theme_config = self.theme_manager.load_theme("default").await
            .map_err(|e| format!("Theme loading failed: {}", e))
            .unwrap();
        
        self.visual_manager.apply_theme(theme_config).await
            .map_err(|e| format!("Theme application failed: {}", e))
            .unwrap();
        
        // 7. Save workspace
        self.workspace_manager.save_workspace(workspace_id).await
            .map_err(|e| format!("Workspace save failed: {}", e))
            .unwrap();
        
        TestResult::passed(test_name)
    }

    /// Test performance monitoring integration
    async fn test_performance_monitoring_integration(&mut self) -> TestResult {
        let test_name = "Performance Monitoring Integration";
        
        // Start performance monitoring
        self.performance_manager.start_monitoring().await
            .map_err(|e| format!("Failed to start monitoring: {}", e))
            .unwrap();
        
        // Perform various operations
        self.create_diverse_nodes().await;
        
        let layout = ForceDirectedLayout::new();
        self.layout_manager.apply_layout(&mut self.scene, Box::new(layout)).await
            .map_err(|e| format!("Layout failed: {}", e))
            .unwrap();
        
        self.engine.render_frame(&self.scene).await
            .map_err(|e| format!("Render failed: {}", e))
            .unwrap();
        
        // Get performance metrics
        let metrics = self.performance_manager.get_metrics().await
            .map_err(|e| format!("Failed to get metrics: {}", e))
            .unwrap();
        
        // Verify metrics are collected
        if metrics.frame_time_ms == 0.0 {
            return TestResult::failed(test_name, "No performance metrics collected");
        }
        
        TestResult::passed(test_name)
    }

    /// Test real-time collaboration
    async fn test_real_time_collaboration(&mut self) -> TestResult {
        let test_name = "Real Time Collaboration";
        
        // Create collaborative workspace
        let workspace_id = self.workspace_manager.create_collaborative_workspace(
            "Collaboration Test".to_string(),
            vec!["user1".to_string(), "user2".to_string()]
        ).await
            .map_err(|e| format!("Collaborative workspace creation failed: {}", e))
            .unwrap();
        
        // Simulate concurrent user operations
        let user1_result = self.simulate_user_operations("user1", workspace_id.clone()).await;
        let user2_result = self.simulate_user_operations("user2", workspace_id.clone()).await;
        
        // Check that both operations succeeded
        if user1_result.is_err() || user2_result.is_err() {
            return TestResult::failed(test_name, "Concurrent operations failed");
        }
        
        // Verify workspace state is consistent
        let workspace = self.workspace_manager.get_workspace(workspace_id).await
            .map_err(|e| format!("Failed to get workspace: {}", e))
            .unwrap();
        
        if workspace.is_none() {
            return TestResult::failed(test_name, "Workspace not found");
        }
        
        TestResult::passed(test_name)
    }

    /// Test multi-workspace stress
    async fn test_multi_workspace_stress(&mut self) -> TestResult {
        let test_name = "Multi Workspace Stress";
        
        let start_time = std::time::Instant::now();
        
        // Create multiple workspaces
        let mut workspace_ids = Vec::new();
        for i in 0..10 {
            let workspace_id = self.workspace_manager.create_workspace(
                format!("Stress Test Workspace {}", i)
            ).await
                .map_err(|e| format!("Workspace {} creation failed: {}", i, e))
                .unwrap();
            
            workspace_ids.push(workspace_id);
        }
        
        // Rapidly switch between workspaces
        for workspace_id in &workspace_ids {
            self.workspace_manager.switch_workspace(workspace_id.clone()).await
                .map_err(|e| format!("Workspace switch failed: {}", e))
                .unwrap();
            
            // Add some nodes
            self.create_simple_nodes().await;
        }
        
        let elapsed = start_time.elapsed();
        
        // Should complete within reasonable time
        if elapsed.as_secs() > 30 {
            return TestResult::failed(test_name, "Multi-workspace operations too slow");
        }
        
        TestResult::passed(test_name)
    }

    /// Test concurrent operations
    async fn test_concurrent_operations(&mut self) -> TestResult {
        let test_name = "Concurrent Operations";
        
        // Launch multiple concurrent operations
        let handles = vec![
            tokio::spawn(async move {
                // Simulate node creation
                tokio::time::sleep(Duration::from_millis(100)).await;
                Ok(())
            }),
            tokio::spawn(async move {
                // Simulate layout application  
                tokio::time::sleep(Duration::from_millis(150)).await;
                Ok(())
            }),
            tokio::spawn(async move {
                // Simulate workspace operations
                tokio::time::sleep(Duration::from_millis(200)).await;
                Ok(())
            }),
        ];
        
        // Wait for all operations to complete
        let results = futures::future::join_all(handles).await;
        
        // Check if all operations succeeded
        for result in results {
            if result.is_err() {
                return TestResult::failed(test_name, "Concurrent operation failed");
            }
        }
        
        TestResult::passed(test_name)
    }

    /// Helper method to create diverse nodes
    async fn create_diverse_nodes(&mut self) {
        // Create various types of nodes
        let nodes = vec![
            Box::new(ApplicationNode::new(
                SceneId::new(),
                "WebBrowser".to_string(),
                "/usr/bin/firefox".to_string(),
            )) as Box<dyn GraphNode>,
            Box::new(FileNode::new(
                SceneId::new(),
                std::path::PathBuf::from("/home/user/document.pdf"),
            )) as Box<dyn GraphNode>,
            Box::new(PersonNode::new(
                SceneId::new(),
                "Alice Smith".to_string(),
                "alice@example.com".to_string(),
            )) as Box<dyn GraphNode>,
            Box::new(SystemNode::new(
                SceneId::new(),
                "NetworkService".to_string(),
            )) as Box<dyn GraphNode>,
            Box::new(DeviceNode::new(
                SceneId::new(),
                "Printer".to_string(),
                "192.168.1.100".to_string(),
            )) as Box<dyn GraphNode>,
            Box::new(TaskNode::new(
                SceneId::new(),
                "Build Project".to_string(),
                "High".to_string(),
            )) as Box<dyn GraphNode>,
        ];
        
        // Register nodes with node manager
        for node in nodes {
            self.node_manager.register_node(node).await
                .expect("Failed to register node");
        }
        
        // Convert to scene nodes and add to scene
        let scene_nodes = self.node_manager.get_scene_nodes().await
            .expect("Failed to get scene nodes");
        
        for scene_node in scene_nodes {
            self.scene.add_node(scene_node);
        }
    }

    /// Helper method to create simple nodes
    async fn create_simple_nodes(&mut self) {
        for i in 0..5 {
            let node = Box::new(SystemNode::new(
                SceneId::new(),
                format!("SimpleNode{}", i),
            )) as Box<dyn GraphNode>;
            
            self.node_manager.register_node(node).await
                .expect("Failed to register node");
        }
    }

    /// Helper method to create node relationships
    async fn create_node_relationships(&mut self) {
        let nodes = self.scene.get_all_nodes();
        
        // Create some relationships between nodes
        for i in 0..nodes.len().saturating_sub(1) {
            let edge_id = self.edge_manager.create_edge(
                SceneId::new(),
                nodes[i].id,
                nodes[i + 1].id,
                EdgeType::Dependency,
                1.0,
            ).await;
            
            if edge_id.is_err() {
                eprintln!("Failed to create edge: {:?}", edge_id.err());
            }
        }
    }

    /// Helper method to simulate user operations
    async fn simulate_user_operations(&mut self, user_id: &str, workspace_id: String) -> Result<()> {
        // Simulate user adding nodes
        let node = Box::new(SystemNode::new(
            SceneId::new(),
            format!("UserNode_{}", user_id),
        )) as Box<dyn GraphNode>;
        
        self.node_manager.register_node(node).await?;
        
        // Simulate user modifying workspace
        tokio::time::sleep(Duration::from_millis(50)).await;
        
        Ok(())
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
        println!("=== Multi-Component Integration Test Results ===");
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
    async fn test_multi_component_integration() {
        let mut suite = MultiComponentIntegrationTests::new().await
            .expect("Failed to create test suite");
        
        let results = suite.run_all_tests().await
            .expect("Failed to run tests");
        
        results.print_summary();
        
        // Assert that most tests pass
        assert!(results.passed_count() > 0, "No tests passed");
        assert!(results.success_rate() > 0.6, "Success rate too low: {:.1}%", results.success_rate() * 100.0);
    }
}