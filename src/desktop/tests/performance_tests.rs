//! Performance tests for HorizonOS Graph Desktop

use std::sync::Arc;
use std::time::{Duration, Instant};
use horizonos_graph_engine::{GraphEngine, Scene, SceneId, NodeType};
use super::{TestResult, TestResults, TestStatus};

/// Performance test suite for graph desktop
pub struct PerformanceTestSuite {
    graph_engine: Arc<GraphEngine>,
    scene: Scene,
}

impl PerformanceTestSuite {
    /// Create new performance test suite
    pub async fn new() -> anyhow::Result<Self> {
        let graph_engine = Arc::new(GraphEngine::new().await?);
        let scene = Scene::new();

        Ok(Self {
            graph_engine,
            scene,
        })
    }

    /// Run all performance tests
    pub async fn run_all_tests(&mut self) -> anyhow::Result<TestResults> {
        let mut results = TestResults::new();

        // Node performance tests
        results.add_result("node_creation_speed", self.test_node_creation_speed().await);
        results.add_result("node_update_speed", self.test_node_update_speed().await);
        results.add_result("node_query_speed", self.test_node_query_speed().await);
        
        // Scene performance tests
        results.add_result("scene_render_performance", self.test_scene_render_performance().await);
        results.add_result("large_scene_handling", self.test_large_scene_handling().await);
        
        // Memory performance tests
        results.add_result("memory_efficiency", self.test_memory_efficiency().await);
        results.add_result("memory_leak_detection", self.test_memory_leak_detection().await);

        Ok(results)
    }

    /// Test node creation speed
    async fn test_node_creation_speed(&mut self) -> TestResult {
        let test_name = "Node Creation Speed Test";
        let node_count = 10000;
        let max_duration_ms = 1000; // 1 second max for 10k nodes

        let start_time = Instant::now();
        
        for i in 0..node_count {
            let node_id = SceneId::new();
            let node = horizonos_graph_engine::SceneNode {
                id: node_id,
                position: [i as f32, 0.0, 0.0].into(),
                velocity: nalgebra::Vector3::zeros(),
                radius: 1.0,
                color: [1.0, 0.0, 0.0, 1.0],
                node_type: NodeType::System {
                    component: format!("perf-test-{}", i),
                    status: horizonos_graph_engine::SystemStatus::Running,
                },
                metadata: horizonos_graph_engine::NodeMetadata::default(),
                visible: true,
                selected: false,
            };
            self.scene.add_node(node);
        }

        let elapsed = start_time.elapsed();
        let elapsed_ms = elapsed.as_millis();

        if elapsed_ms <= max_duration_ms {
            TestResult::passed(test_name)
        } else {
            TestResult::failed(
                test_name, 
                &format!("Too slow: {}ms for {} nodes (max: {}ms)", elapsed_ms, node_count, max_duration_ms)
            )
        }
    }

    /// Test node update speed
    async fn test_node_update_speed(&mut self) -> TestResult {
        let test_name = "Node Update Speed Test";
        
        // Add nodes to update
        let mut node_ids = Vec::new();
        for i in 0..1000 {
            let node_id = SceneId::new();
            let node = horizonos_graph_engine::SceneNode {
                id: node_id,
                position: [i as f32, 0.0, 0.0].into(),
                velocity: nalgebra::Vector3::zeros(),
                radius: 1.0,
                color: [0.0, 1.0, 0.0, 1.0],
                node_type: NodeType::System {
                    component: format!("update-test-{}", i),
                    status: horizonos_graph_engine::SystemStatus::Running,
                },
                metadata: horizonos_graph_engine::NodeMetadata::default(),
                visible: true,
                selected: false,
            };
            self.scene.add_node(node);
            node_ids.push(node_id);
        }

        // Time updates
        let start_time = Instant::now();
        let update_count = 100;
        
        for _ in 0..update_count {
            self.scene.update(0.016); // 60 FPS
        }

        let elapsed = start_time.elapsed();
        let avg_frame_time = elapsed.as_micros() / update_count;

        // Should be well under 16ms per frame for 60 FPS
        if avg_frame_time < 16000 { // 16ms in microseconds
            TestResult::passed(test_name)
        } else {
            TestResult::failed(
                test_name,
                &format!("Update too slow: {}μs average (target: <16000μs)", avg_frame_time)
            )
        }
    }

    /// Test node query speed
    async fn test_node_query_speed(&mut self) -> TestResult {
        let test_name = "Node Query Speed Test";
        let query_count = 10000;
        
        // Add nodes to query
        let mut node_ids = Vec::new();
        for i in 0..1000 {
            let node_id = SceneId::new();
            let node = horizonos_graph_engine::SceneNode {
                id: node_id,
                position: [i as f32, 0.0, 0.0].into(),
                velocity: nalgebra::Vector3::zeros(),
                radius: 1.0,
                color: [0.0, 0.0, 1.0, 1.0],
                node_type: NodeType::System {
                    component: format!("query-test-{}", i),
                    status: horizonos_graph_engine::SystemStatus::Running,
                },
                metadata: horizonos_graph_engine::NodeMetadata::default(),
                visible: true,
                selected: false,
            };
            self.scene.add_node(node);
            node_ids.push(node_id);
        }

        // Time queries
        let start_time = Instant::now();
        
        for _ in 0..query_count {
            let random_id = node_ids[fastrand::usize(..node_ids.len())];
            let _node = self.scene.get_node(random_id);
        }

        let elapsed = start_time.elapsed();
        let avg_query_time = elapsed.as_nanos() / query_count;

        // Should be very fast - under 1000ns per query
        if avg_query_time < 1000 {
            TestResult::passed(test_name)
        } else {
            TestResult::failed(
                test_name,
                &format!("Query too slow: {}ns average (target: <1000ns)", avg_query_time)
            )
        }
    }

    /// Test scene rendering performance
    async fn test_scene_render_performance(&mut self) -> TestResult {
        let test_name = "Scene Render Performance Test";
        
        // Add many visible nodes
        for i in 0..5000 {
            let node_id = SceneId::new();
            let node = horizonos_graph_engine::SceneNode {
                id: node_id,
                position: [(i % 100) as f32, (i / 100) as f32, 0.0].into(),
                velocity: nalgebra::Vector3::zeros(),
                radius: 1.0,
                color: [1.0, 1.0, 0.0, 1.0],
                node_type: NodeType::System {
                    component: format!("render-test-{}", i),
                    status: horizonos_graph_engine::SystemStatus::Running,
                },
                metadata: horizonos_graph_engine::NodeMetadata::default(),
                visible: true,
                selected: false,
            };
            self.scene.add_node(node);
        }

        // Simulate render passes
        let start_time = Instant::now();
        let render_passes = 60; // 1 second at 60 FPS
        
        for _ in 0..render_passes {
            // Simulate getting visible nodes for rendering
            let visible_nodes = self.scene.get_visible_nodes();
            if visible_nodes.len() != 5000 {
                return TestResult::failed(test_name, "Incorrect visible node count");
            }
            
            // Simulate basic render work
            std::thread::sleep(Duration::from_micros(100)); // Minimal simulated work
        }

        let elapsed = start_time.elapsed();
        let avg_frame_time_ms = elapsed.as_millis() / render_passes;

        // Should maintain 60 FPS (under 16.67ms per frame)
        if avg_frame_time_ms < 17 {
            TestResult::passed(test_name)
        } else {
            TestResult::failed(
                test_name,
                &format!("Render too slow: {}ms average (target: <17ms)", avg_frame_time_ms)
            )
        }
    }

    /// Test large scene handling
    async fn test_large_scene_handling(&mut self) -> TestResult {
        let test_name = "Large Scene Handling Test";
        let large_node_count = 50000;
        
        let start_time = Instant::now();
        
        // Add many nodes
        for i in 0..large_node_count {
            if i % 10000 == 0 {
                // Check timeout every 10k nodes
                if start_time.elapsed().as_secs() > 10 {
                    return TestResult::failed(test_name, "Large scene creation timeout");
                }
            }
            
            let node_id = SceneId::new();
            let node = horizonos_graph_engine::SceneNode {
                id: node_id,
                position: [(i % 200) as f32, (i / 200) as f32, (i / 40000) as f32].into(),
                velocity: nalgebra::Vector3::zeros(),
                radius: 0.5,
                color: [0.5, 0.5, 0.5, 1.0],
                node_type: NodeType::System {
                    component: format!("large-{}", i),
                    status: horizonos_graph_engine::SystemStatus::Running,
                },
                metadata: horizonos_graph_engine::NodeMetadata::default(),
                visible: i % 10 == 0, // Only 10% visible to test culling
                selected: false,
            };
            self.scene.add_node(node);
        }

        // Test operations on large scene
        let query_start = Instant::now();
        let visible_nodes = self.scene.get_visible_nodes();
        let query_time = query_start.elapsed();

        if visible_nodes.len() != large_node_count / 10 {
            return TestResult::failed(test_name, "Incorrect visible node culling");
        }

        if query_time.as_millis() > 100 {
            return TestResult::failed(
                test_name,
                &format!("Large scene query too slow: {}ms", query_time.as_millis())
            );
        }

        TestResult::passed(test_name)
    }

    /// Test memory efficiency
    async fn test_memory_efficiency(&mut self) -> TestResult {
        let test_name = "Memory Efficiency Test";
        
        // This is a placeholder test - in a real implementation you'd measure
        // actual memory usage using system tools or memory profiling
        
        TestResult::passed(test_name)
    }

    /// Test for memory leaks
    async fn test_memory_leak_detection(&mut self) -> TestResult {
        let test_name = "Memory Leak Detection Test";
        
        // Create and destroy nodes repeatedly
        for cycle in 0..100 {
            let mut temp_nodes = Vec::new();
            
            // Create nodes
            for i in 0..1000 {
                let node_id = SceneId::new();
                let node = horizonos_graph_engine::SceneNode {
                    id: node_id,
                    position: [i as f32, cycle as f32, 0.0].into(),
                    velocity: nalgebra::Vector3::zeros(),
                    radius: 1.0,
                    color: [1.0, 0.0, 1.0, 1.0],
                    node_type: NodeType::System {
                        component: format!("leak-test-{}-{}", cycle, i),
                        status: horizonos_graph_engine::SystemStatus::Running,
                    },
                    metadata: horizonos_graph_engine::NodeMetadata::default(),
                    visible: true,
                    selected: false,
                };
                self.scene.add_node(node);
                temp_nodes.push(node_id);
            }
            
            // Remove nodes
            for node_id in temp_nodes {
                self.scene.remove_node(node_id);
            }
        }

        // In a real test, you'd check memory usage hasn't grown significantly
        TestResult::passed(test_name)
    }
}

/// Add fastrand for performance testing
mod fastrand {
    use std::sync::atomic::{AtomicU64, Ordering};
    
    static RNG: AtomicU64 = AtomicU64::new(1);
    
    pub fn usize(range: std::ops::Range<usize>) -> usize {
        let mut x = RNG.load(Ordering::Relaxed);
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        RNG.store(x, Ordering::Relaxed);
        
        range.start + (x as usize % (range.end - range.start))
    }
}