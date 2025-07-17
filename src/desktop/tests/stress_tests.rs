//! Stress tests for HorizonOS Graph Desktop

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::timeout;
use horizonos_graph_engine::{GraphEngine, Scene, SceneId, NodeType};
use super::{TestResult, TestResults};

/// Stress test suite for graph desktop
pub struct StressTestSuite {
    graph_engine: Arc<GraphEngine>,
    scene: Scene,
}

impl StressTestSuite {
    /// Create new stress test suite
    pub async fn new() -> anyhow::Result<Self> {
        let graph_engine = Arc::new(GraphEngine::new().await?);
        let scene = Scene::new();

        Ok(Self {
            graph_engine,
            scene,
        })
    }

    /// Run all stress tests
    pub async fn run_all_tests(&mut self) -> anyhow::Result<TestResults> {
        let mut results = TestResults::new();

        // Extreme load tests
        results.add_result("extreme_node_count", self.test_extreme_node_count().await);
        results.add_result("rapid_operations", self.test_rapid_operations().await);
        results.add_result("concurrent_access", self.test_concurrent_access().await);
        
        // Resource exhaustion tests
        results.add_result("memory_pressure", self.test_memory_pressure().await);
        results.add_result("cpu_saturation", self.test_cpu_saturation().await);
        
        // Stability tests
        results.add_result("long_running_stability", self.test_long_running_stability().await);
        results.add_result("error_recovery", self.test_error_recovery().await);

        Ok(results)
    }

    /// Test with extreme number of nodes
    async fn test_extreme_node_count(&mut self) -> TestResult {
        let test_name = "Extreme Node Count Test";
        let extreme_count = 100000; // 100k nodes
        let timeout_duration = Duration::from_secs(60); // 1 minute timeout

        let test_future = async {
            let start_time = Instant::now();
            
            for i in 0..extreme_count {
                if i % 10000 == 0 {
                    // Progress check every 10k nodes
                    println!("  Progress: {}/{} nodes added", i, extreme_count);
                    
                    // Check if taking too long
                    if start_time.elapsed() > Duration::from_secs(45) {
                        return TestResult::failed(test_name, "Extreme node creation taking too long");
                    }
                }
                
                let node_id = SceneId::new();
                let node = horizonos_graph_engine::SceneNode {
                    id: node_id,
                    position: [
                        (i % 300) as f32,
                        ((i / 300) % 300) as f32,
                        (i / 90000) as f32
                    ].into(),
                    velocity: nalgebra::Vector3::zeros(),
                    radius: 0.1,
                    color: [0.1, 0.1, 0.1, 1.0],
                    node_type: NodeType::System {
                        component: format!("extreme-{}", i),
                        status: horizonos_graph_engine::SystemStatus::Running,
                    },
                    metadata: horizonos_graph_engine::NodeMetadata::default(),
                    visible: i % 100 == 0, // Only 1% visible
                    selected: false,
                };
                self.scene.add_node(node);
            }

            // Test operations on extreme scene
            let query_start = Instant::now();
            let visible_count = self.scene.get_visible_nodes().len();
            let query_time = query_start.elapsed();

            if visible_count != extreme_count / 100 {
                return TestResult::failed(test_name, "Incorrect visible node count in extreme scene");
            }

            if query_time.as_millis() > 1000 {
                return TestResult::failed(
                    test_name,
                    &format!("Extreme scene query too slow: {}ms", query_time.as_millis())
                );
            }

            TestResult::passed(test_name)
        };

        match timeout(timeout_duration, test_future).await {
            Ok(result) => result,
            Err(_) => TestResult::failed(test_name, "Test timeout"),
        }
    }

    /// Test rapid operations
    async fn test_rapid_operations(&mut self) -> TestResult {
        let test_name = "Rapid Operations Test";
        let operation_count = 100000;
        let timeout_duration = Duration::from_secs(30);

        let test_future = async {
            let start_time = Instant::now();
            let mut node_ids = Vec::new();

            // Rapid create/update/query/delete cycle
            for i in 0..operation_count {
                match i % 4 {
                    0 => {
                        // Create node
                        let node_id = SceneId::new();
                        let node = horizonos_graph_engine::SceneNode {
                            id: node_id,
                            position: [fastrand::f32() * 100.0, fastrand::f32() * 100.0, 0.0].into(),
                            velocity: nalgebra::Vector3::zeros(),
                            radius: 1.0,
                            color: [1.0, 0.0, 0.0, 1.0],
                            node_type: NodeType::System {
                                component: format!("rapid-{}", i),
                                status: horizonos_graph_engine::SystemStatus::Running,
                            },
                            metadata: horizonos_graph_engine::NodeMetadata::default(),
                            visible: true,
                            selected: false,
                        };
                        self.scene.add_node(node);
                        node_ids.push(node_id);
                    }
                    1 => {
                        // Query node
                        if !node_ids.is_empty() {
                            let random_id = node_ids[fastrand::usize(0..node_ids.len())];
                            let _node = self.scene.get_node(random_id);
                        }
                    }
                    2 => {
                        // Update scene
                        self.scene.update(0.016);
                    }
                    3 => {
                        // Delete node
                        if !node_ids.is_empty() {
                            let index = fastrand::usize(0..node_ids.len());
                            let node_id = node_ids.remove(index);
                            self.scene.remove_node(node_id);
                        }
                    }
                    _ => unreachable!(),
                }

                // Progress check
                if i % 10000 == 0 && i > 0 {
                    let elapsed = start_time.elapsed();
                    let ops_per_sec = i as f64 / elapsed.as_secs_f64();
                    println!("  Rapid ops progress: {} ops, {:.1} ops/sec", i, ops_per_sec);
                }
            }

            let elapsed = start_time.elapsed();
            let ops_per_sec = operation_count as f64 / elapsed.as_secs_f64();

            // Should handle at least 1000 operations per second
            if ops_per_sec >= 1000.0 {
                TestResult::passed(test_name)
            } else {
                TestResult::failed(
                    test_name,
                    &format!("Too slow: {:.1} ops/sec (target: ≥1000 ops/sec)", ops_per_sec)
                )
            }
        };

        match timeout(timeout_duration, test_future).await {
            Ok(result) => result,
            Err(_) => TestResult::failed(test_name, "Test timeout"),
        }
    }

    /// Test concurrent access
    async fn test_concurrent_access(&mut self) -> TestResult {
        let test_name = "Concurrent Access Test";
        
        // This is a simplified test - in a real implementation you'd need
        // proper thread-safe scene access and multiple threads/tasks
        
        let start_time = Instant::now();
        let iterations = 1000;
        
        // Simulate concurrent-like operations by interleaving different operation types
        for i in 0..iterations {
            // Simulate multiple "threads" doing different operations
            for thread_id in 0..4 {
                let node_id = SceneId::new();
                let node = horizonos_graph_engine::SceneNode {
                    id: node_id,
                    position: [thread_id as f32 * 10.0, i as f32, 0.0].into(),
                    velocity: nalgebra::Vector3::zeros(),
                    radius: 1.0,
                    color: [thread_id as f32 * 0.25, 1.0, 0.0, 1.0],
                    node_type: NodeType::System {
                        component: format!("concurrent-{}-{}", thread_id, i),
                        status: horizonos_graph_engine::SystemStatus::Running,
                    },
                    metadata: horizonos_graph_engine::NodeMetadata::default(),
                    visible: true,
                    selected: false,
                };
                self.scene.add_node(node);
                
                // Query operation
                let _visible = self.scene.get_visible_nodes();
                
                // Update operation
                self.scene.update(0.001);
            }
        }

        let elapsed = start_time.elapsed();
        
        // Should complete concurrent-like operations reasonably quickly
        if elapsed.as_secs() < 10 {
            TestResult::passed(test_name)
        } else {
            TestResult::failed(
                test_name,
                &format!("Concurrent operations too slow: {}s", elapsed.as_secs())
            )
        }
    }

    /// Test memory pressure handling
    async fn test_memory_pressure(&mut self) -> TestResult {
        let test_name = "Memory Pressure Test";
        
        // Create a large number of nodes to pressure memory
        let pressure_count = 200000;
        let start_time = Instant::now();
        
        for i in 0..pressure_count {
            if i % 50000 == 0 {
                println!("  Memory pressure progress: {}/{}", i, pressure_count);
                
                // Check if taking too long
                if start_time.elapsed() > Duration::from_secs(120) {
                    return TestResult::failed(test_name, "Memory pressure test timeout");
                }
            }
            
            let node_id = SceneId::new();
            let node = horizonos_graph_engine::SceneNode {
                id: node_id,
                position: [
                    fastrand::f32() * 1000.0,
                    fastrand::f32() * 1000.0,
                    fastrand::f32() * 100.0
                ].into(),
                velocity: nalgebra::Vector3::zeros(),
                radius: fastrand::f32() * 2.0,
                color: [fastrand::f32(), fastrand::f32(), fastrand::f32(), 1.0],
                node_type: NodeType::System {
                    component: format!("pressure-{}", i),
                    status: horizonos_graph_engine::SystemStatus::Running,
                },
                metadata: horizonos_graph_engine::NodeMetadata::default(),
                visible: fastrand::bool(),
                selected: false,
            };
            self.scene.add_node(node);
        }

        // Test operations under memory pressure
        let query_start = Instant::now();
        let _visible = self.scene.get_visible_nodes();
        let query_time = query_start.elapsed();

        // Should still be responsive under memory pressure
        if query_time.as_millis() < 2000 {
            TestResult::passed(test_name)
        } else {
            TestResult::failed(
                test_name,
                &format!("Query too slow under memory pressure: {}ms", query_time.as_millis())
            )
        }
    }

    /// Test CPU saturation handling
    async fn test_cpu_saturation(&mut self) -> TestResult {
        let test_name = "CPU Saturation Test";
        
        // Add nodes with high update frequency to saturate CPU
        let cpu_stress_count = 10000;
        
        for i in 0..cpu_stress_count {
            let node_id = SceneId::new();
            let node = horizonos_graph_engine::SceneNode {
                id: node_id,
                position: [i as f32, 0.0, 0.0].into(),
                velocity: nalgebra::Vector3::new(
                    fastrand::f32() * 10.0 - 5.0,
                    fastrand::f32() * 10.0 - 5.0,
                    0.0
                ),
                radius: 1.0,
                color: [1.0, 1.0, 0.0, 1.0],
                node_type: NodeType::System {
                    component: format!("cpu-stress-{}", i),
                    status: horizonos_graph_engine::SystemStatus::Running,
                },
                metadata: horizonos_graph_engine::NodeMetadata::default(),
                visible: true,
                selected: false,
            };
            self.scene.add_node(node);
        }

        // Run many update cycles to stress CPU
        let start_time = Instant::now();
        let update_cycles = 1000;
        
        for _ in 0..update_cycles {
            self.scene.update(0.016);
        }

        let elapsed = start_time.elapsed();
        let avg_update_time = elapsed.as_micros() / update_cycles;

        // Should maintain reasonable performance even under CPU stress
        if avg_update_time < 50000 { // 50ms average
            TestResult::passed(test_name)
        } else {
            TestResult::failed(
                test_name,
                &format!("Updates too slow under CPU stress: {}μs average", avg_update_time)
            )
        }
    }

    /// Test long-running stability
    async fn test_long_running_stability(&mut self) -> TestResult {
        let test_name = "Long Running Stability Test";
        let timeout_duration = Duration::from_secs(30); // Shortened for testing
        
        let test_future = async {
            let start_time = Instant::now();
            let mut iteration = 0;
            
            while start_time.elapsed() < Duration::from_secs(25) {
                // Continuous operations
                let node_id = SceneId::new();
                let node = horizonos_graph_engine::SceneNode {
                    id: node_id,
                    position: [iteration as f32, 0.0, 0.0].into(),
                    velocity: nalgebra::Vector3::zeros(),
                    radius: 1.0,
                    color: [0.0, 1.0, 1.0, 1.0],
                    node_type: NodeType::System {
                        component: format!("stability-{}", iteration),
                        status: horizonos_graph_engine::SystemStatus::Running,
                    },
                    metadata: horizonos_graph_engine::NodeMetadata::default(),
                    visible: true,
                    selected: false,
                };
                self.scene.add_node(node);
                
                self.scene.update(0.016);
                
                // Occasional cleanup
                if iteration % 1000 == 0 {
                    // Remove some old nodes to prevent infinite growth
                    if let Some(old_node) = self.scene.get_visible_nodes().first() {
                        self.scene.remove_node(old_node.id);
                    }
                }
                
                iteration += 1;
            }

            TestResult::passed(test_name)
        };

        match timeout(timeout_duration, test_future).await {
            Ok(result) => result,
            Err(_) => TestResult::failed(test_name, "Long running stability test timeout"),
        }
    }

    /// Test error recovery
    async fn test_error_recovery(&mut self) -> TestResult {
        let test_name = "Error Recovery Test";
        
        // Test recovery from various error conditions
        
        // 1. Try to access non-existent nodes
        let fake_id = SceneId::new();
        let result = self.scene.get_node(fake_id);
        if result.is_some() {
            return TestResult::failed(test_name, "Should not find non-existent node");
        }

        // 2. Try to remove non-existent nodes
        let removed = self.scene.remove_node(fake_id);
        if removed.is_some() {
            return TestResult::failed(test_name, "Should not remove non-existent node");
        }

        // 3. Add node and then continue normal operations
        let node_id = SceneId::new();
        let node = horizonos_graph_engine::SceneNode {
            id: node_id,
            position: [0.0, 0.0, 0.0].into(),
            velocity: nalgebra::Vector3::zeros(),
            radius: 1.0,
            color: [1.0, 0.0, 1.0, 1.0],
            node_type: NodeType::System {
                component: "error-recovery-test".to_string(),
                status: horizonos_graph_engine::SystemStatus::Running,
            },
            metadata: horizonos_graph_engine::NodeMetadata::default(),
            visible: true,
            selected: false,
        };
        self.scene.add_node(node);

        // 4. Verify normal operations work after errors
        if !self.scene.has_node(node_id) {
            return TestResult::failed(test_name, "Normal operations failed after error conditions");
        }

        TestResult::passed(test_name)
    }
}

/// Simple random number generation for stress tests
mod fastrand {
    use std::sync::atomic::{AtomicU64, Ordering};
    
    static RNG: AtomicU64 = AtomicU64::new(12345);
    
    pub fn f32() -> f32 {
        let mut x = RNG.load(Ordering::Relaxed);
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        RNG.store(x, Ordering::Relaxed);
        
        (x as f32 / u64::MAX as f32).abs()
    }
    
    pub fn bool() -> bool {
        f32() > 0.5
    }
    
    pub fn usize(min: usize, max: usize) -> usize {
        min + ((f32() * (max - min) as f32) as usize)
    }
}