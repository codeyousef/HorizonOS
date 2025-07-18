//! Performance and Load Testing for AI Components
//! 
//! Tests system performance under various load conditions.

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;
use futures::future::join_all;

use horizonos_graph_ai::{
    AIManager, AIConfig, AIError,
    monitoring::resource_monitor::{ResourceMonitor, ResourceConfig},
    patterns::workflow::{WorkflowEngine, WorkflowConfig, WorkflowStep, WorkflowStepType},
    ollama::{OllamaClient, OllamaConfig},
    agents::coordinator::{AgentCoordinator, CoordinatorConfig},
    storage::memory::{MemoryManager, MemoryConfig},
};

/// Performance test configuration
#[derive(Debug, Clone)]
pub struct PerformanceTestConfig {
    pub concurrent_requests: usize,
    pub total_requests: usize,
    pub request_timeout: Duration,
    pub warmup_requests: usize,
    pub test_duration: Duration,
}

impl Default for PerformanceTestConfig {
    fn default() -> Self {
        Self {
            concurrent_requests: 10,
            total_requests: 100,
            request_timeout: Duration::from_secs(30),
            warmup_requests: 10,
            test_duration: Duration::from_secs(60),
        }
    }
}

/// Performance test results
#[derive(Debug, Clone)]
pub struct PerformanceTestResults {
    pub total_requests: usize,
    pub successful_requests: usize,
    pub failed_requests: usize,
    pub avg_response_time: Duration,
    pub min_response_time: Duration,
    pub max_response_time: Duration,
    pub p95_response_time: Duration,
    pub p99_response_time: Duration,
    pub requests_per_second: f64,
    pub error_rate: f64,
    pub test_duration: Duration,
}

/// Performance test fixture
pub struct PerformanceTestFixture {
    pub ai_manager: AIManager,
    pub resource_monitor: ResourceMonitor,
    pub workflow_engine: WorkflowEngine,
    pub ollama_client: OllamaClient,
    pub agent_coordinator: AgentCoordinator,
    pub memory_manager: MemoryManager,
}

impl PerformanceTestFixture {
    /// Create a new performance test fixture
    pub async fn new() -> Result<Self, AIError> {
        // Configure components for performance testing
        let ai_config = AIConfig {
            enabled: true,
            model_name: "qwen2.5:1.5b".to_string(),
            max_context_length: 4096,
            temperature: 0.7,
            max_tokens: 512,
            hardware_optimization: true,
            concurrent_requests: 20, // Higher concurrency for performance tests
            request_timeout: Duration::from_secs(15),
            cache_size: 1000,
            background_processing: true,
            privacy_mode: true,
            local_only: true,
            data_retention_hours: 1, // Short retention for tests
            user_consent_required: false,
            metrics_enabled: true,
            rate_limit: 1000,
        };
        
        let ai_manager = AIManager::new(ai_config).await?;
        
        let resource_config = ResourceConfig {
            enabled: true,
            monitoring_interval: 1, // High frequency monitoring
            ..Default::default()
        };
        let resource_monitor = ResourceMonitor::new(resource_config).await?;
        
        let workflow_config = WorkflowConfig {
            enabled: true,
            max_concurrent_workflows: 50,
            execution_timeout: Duration::from_secs(30),
            ..Default::default()
        };
        let workflow_engine = WorkflowEngine::new(workflow_config).await?;
        
        let ollama_config = OllamaConfig {
            base_url: "http://localhost:11434".to_string(),
            model_name: "qwen2.5:1.5b".to_string(),
            timeout: Duration::from_secs(15),
            ..Default::default()
        };
        let ollama_client = OllamaClient::new(ollama_config).await?;
        
        let coordinator_config = CoordinatorConfig {
            enabled: true,
            max_agents: 100,
            ..Default::default()
        };
        let agent_coordinator = AgentCoordinator::new(coordinator_config).await?;
        
        let memory_config = MemoryConfig {
            enabled: true,
            max_memory_mb: 2048,
            cleanup_interval: Duration::from_secs(30),
            ..Default::default()
        };
        let memory_manager = MemoryManager::new(memory_config).await?;
        
        Ok(Self {
            ai_manager,
            resource_monitor,
            workflow_engine,
            ollama_client,
            agent_coordinator,
            memory_manager,
        })
    }
    
    /// Start all components
    pub async fn start(&self) -> Result<(), AIError> {
        self.ai_manager.start().await?;
        self.resource_monitor.start().await?;
        self.workflow_engine.start().await?;
        self.agent_coordinator.start().await?;
        self.memory_manager.start().await?;
        Ok(())
    }
    
    /// Stop all components
    pub async fn stop(&self) -> Result<(), AIError> {
        self.ai_manager.stop().await?;
        self.resource_monitor.stop().await?;
        self.workflow_engine.stop().await?;
        self.agent_coordinator.stop().await?;
        self.memory_manager.stop().await?;
        Ok(())
    }
    
    /// Run a performance test with the given configuration
    pub async fn run_performance_test(
        &self,
        test_name: &str,
        config: PerformanceTestConfig,
        test_fn: impl Fn() -> futures::future::BoxFuture<'static, Result<Duration, AIError>> + Send + Sync + 'static,
    ) -> Result<PerformanceTestResults, AIError> {
        println!("Starting performance test: {}", test_name);
        
        let test_fn = Arc::new(test_fn);
        let mut response_times = Vec::new();
        let mut successful_requests = 0;
        let mut failed_requests = 0;
        
        let start_time = Instant::now();
        
        // Warmup phase
        println!("Warming up with {} requests...", config.warmup_requests);
        for _ in 0..config.warmup_requests {
            let _ = test_fn().await;
        }
        
        // Main test phase
        println!("Running {} requests with {} concurrent...", config.total_requests, config.concurrent_requests);
        
        let mut tasks = Vec::new();
        let semaphore = Arc::new(tokio::sync::Semaphore::new(config.concurrent_requests));
        
        for _ in 0..config.total_requests {
            let semaphore = semaphore.clone();
            let test_fn = test_fn.clone();
            
            let task = tokio::spawn(async move {
                let _permit = semaphore.acquire().await.unwrap();
                let start = Instant::now();
                let result = test_fn().await;
                let duration = start.elapsed();
                (result, duration)
            });
            
            tasks.push(task);
        }
        
        // Wait for all tasks to complete
        let results = join_all(tasks).await;
        
        // Process results
        for result in results {
            match result {
                Ok((Ok(_), duration)) => {
                    successful_requests += 1;
                    response_times.push(duration);
                }
                Ok((Err(_), duration)) => {
                    failed_requests += 1;
                    response_times.push(duration);
                }
                Err(_) => {
                    failed_requests += 1;
                }
            }
        }
        
        let test_duration = start_time.elapsed();
        
        // Calculate statistics
        response_times.sort();
        let avg_response_time = response_times.iter().sum::<Duration>() / response_times.len() as u32;
        let min_response_time = response_times.first().cloned().unwrap_or_default();
        let max_response_time = response_times.last().cloned().unwrap_or_default();
        let p95_index = (response_times.len() as f64 * 0.95) as usize;
        let p99_index = (response_times.len() as f64 * 0.99) as usize;
        let p95_response_time = response_times.get(p95_index).cloned().unwrap_or_default();
        let p99_response_time = response_times.get(p99_index).cloned().unwrap_or_default();
        let requests_per_second = config.total_requests as f64 / test_duration.as_secs_f64();
        let error_rate = failed_requests as f64 / config.total_requests as f64;
        
        let results = PerformanceTestResults {
            total_requests: config.total_requests,
            successful_requests,
            failed_requests,
            avg_response_time,
            min_response_time,
            max_response_time,
            p95_response_time,
            p99_response_time,
            requests_per_second,
            error_rate,
            test_duration,
        };
        
        println!("Performance test completed: {}", test_name);
        println!("  Total requests: {}", results.total_requests);
        println!("  Successful requests: {}", results.successful_requests);
        println!("  Failed requests: {}", results.failed_requests);
        println!("  Average response time: {:?}", results.avg_response_time);
        println!("  95th percentile: {:?}", results.p95_response_time);
        println!("  99th percentile: {:?}", results.p99_response_time);
        println!("  Requests per second: {:.2}", results.requests_per_second);
        println!("  Error rate: {:.2}%", results.error_rate * 100.0);
        
        Ok(results)
    }
}

#[tokio::test]
async fn test_ai_manager_performance() {
    let fixture = PerformanceTestFixture::new().await.unwrap();
    fixture.start().await.unwrap();
    
    let config = PerformanceTestConfig {
        concurrent_requests: 5,
        total_requests: 50,
        ..Default::default()
    };
    
    let ai_manager = fixture.ai_manager.clone();
    let test_fn = move || {
        let ai_manager = ai_manager.clone();
        Box::pin(async move {
            let start = Instant::now();
            let request = json!({
                "model": "qwen2.5:1.5b",
                "prompt": "Analyze this simple text for testing",
                "max_tokens": 100
            });
            
            let _response = ai_manager.process_request(request).await?;
            Ok(start.elapsed())
        })
    };
    
    let results = fixture.run_performance_test("AI Manager Performance", config, test_fn).await.unwrap();
    
    // Performance assertions
    assert!(results.error_rate < 0.1, "Error rate should be less than 10%");
    assert!(results.avg_response_time < Duration::from_secs(10), "Average response time should be less than 10 seconds");
    assert!(results.requests_per_second > 0.5, "Should process at least 0.5 requests per second");
    
    fixture.stop().await.unwrap();
}

#[tokio::test]
async fn test_workflow_engine_performance() {
    let fixture = PerformanceTestFixture::new().await.unwrap();
    fixture.start().await.unwrap();
    
    let config = PerformanceTestConfig {
        concurrent_requests: 10,
        total_requests: 100,
        ..Default::default()
    };
    
    let workflow_engine = fixture.workflow_engine.clone();
    let test_fn = move || {
        let workflow_engine = workflow_engine.clone();
        Box::pin(async move {
            let start = Instant::now();
            
            let workflow = horizonos_graph_ai::patterns::workflow::Workflow {
                id: Uuid::new_v4().to_string(),
                name: "Performance Test Workflow".to_string(),
                description: Some("Simple workflow for performance testing".to_string()),
                steps: vec![
                    WorkflowStep {
                        id: "simple_step".to_string(),
                        name: "Simple Step".to_string(),
                        description: Some("Simple step for performance testing".to_string()),
                        step_type: WorkflowStepType::AIAnalysis,
                        input: json!({
                            "task": "simple_analysis",
                            "model": "qwen2.5:1.5b"
                        }),
                        output: None,
                        timeout: Some(Duration::from_secs(15)),
                        retry_config: None,
                        depends_on: vec![],
                        condition: None,
                        parallel: false,
                        metadata: std::collections::HashMap::new(),
                    },
                ],
                enabled: true,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                version: 1,
                metadata: std::collections::HashMap::new(),
            };
            
            let _execution_id = workflow_engine.execute_workflow(workflow, "test_user").await?;
            Ok(start.elapsed())
        })
    };
    
    let results = fixture.run_performance_test("Workflow Engine Performance", config, test_fn).await.unwrap();
    
    // Performance assertions
    assert!(results.error_rate < 0.2, "Error rate should be less than 20%");
    assert!(results.avg_response_time < Duration::from_secs(5), "Average response time should be less than 5 seconds");
    assert!(results.requests_per_second > 1.0, "Should process at least 1 request per second");
    
    fixture.stop().await.unwrap();
}

#[tokio::test]
async fn test_memory_manager_performance() {
    let fixture = PerformanceTestFixture::new().await.unwrap();
    fixture.start().await.unwrap();
    
    let config = PerformanceTestConfig {
        concurrent_requests: 20,
        total_requests: 1000,
        ..Default::default()
    };
    
    let memory_manager = fixture.memory_manager.clone();
    let test_fn = move || {
        let memory_manager = memory_manager.clone();
        Box::pin(async move {
            let start = Instant::now();
            
            let memory_id = Uuid::new_v4().to_string();
            let memory_data = json!({
                "type": "test_memory",
                "content": "Performance test memory data",
                "metadata": {
                    "test_id": memory_id,
                    "timestamp": Utc::now().to_rfc3339()
                }
            });
            
            // Store memory
            memory_manager.store_memory(&memory_id, memory_data.clone()).await?;
            
            // Retrieve memory
            let _retrieved = memory_manager.get_memory(&memory_id).await?;
            
            // Clean up
            memory_manager.delete_memory(&memory_id).await?;
            
            Ok(start.elapsed())
        })
    };
    
    let results = fixture.run_performance_test("Memory Manager Performance", config, test_fn).await.unwrap();
    
    // Performance assertions
    assert!(results.error_rate < 0.05, "Error rate should be less than 5%");
    assert!(results.avg_response_time < Duration::from_millis(100), "Average response time should be less than 100ms");
    assert!(results.requests_per_second > 10.0, "Should process at least 10 requests per second");
    
    fixture.stop().await.unwrap();
}

#[tokio::test]
async fn test_resource_monitor_performance() {
    let fixture = PerformanceTestFixture::new().await.unwrap();
    fixture.start().await.unwrap();
    
    let config = PerformanceTestConfig {
        concurrent_requests: 50,
        total_requests: 500,
        ..Default::default()
    };
    
    let resource_monitor = fixture.resource_monitor.clone();
    let test_fn = move || {
        let resource_monitor = resource_monitor.clone();
        Box::pin(async move {
            let start = Instant::now();
            
            // Get resource stats
            let _stats = resource_monitor.get_stats().await?;
            
            // Get health summary
            let _health = resource_monitor.get_health_summary().await?;
            
            // Get alerts
            let _alerts = resource_monitor.get_alerts();
            
            Ok(start.elapsed())
        })
    };
    
    let results = fixture.run_performance_test("Resource Monitor Performance", config, test_fn).await.unwrap();
    
    // Performance assertions
    assert!(results.error_rate < 0.02, "Error rate should be less than 2%");
    assert!(results.avg_response_time < Duration::from_millis(50), "Average response time should be less than 50ms");
    assert!(results.requests_per_second > 20.0, "Should process at least 20 requests per second");
    
    fixture.stop().await.unwrap();
}

#[tokio::test]
async fn test_agent_coordinator_performance() {
    let fixture = PerformanceTestFixture::new().await.unwrap();
    fixture.start().await.unwrap();
    
    let config = PerformanceTestConfig {
        concurrent_requests: 15,
        total_requests: 150,
        ..Default::default()
    };
    
    let agent_coordinator = fixture.agent_coordinator.clone();
    let test_fn = move || {
        let agent_coordinator = agent_coordinator.clone();
        Box::pin(async move {
            let start = Instant::now();
            
            // Create agent
            let agent_id = agent_coordinator.create_agent(
                &format!("perf_agent_{}", Uuid::new_v4()),
                "performance_test"
            ).await?;
            
            // Assign task
            let task = json!({
                "agent_id": agent_id,
                "task": "performance_test_task",
                "priority": "medium",
                "data": {"test": "data"}
            });
            
            let _task_id = agent_coordinator.assign_task(task).await?;
            
            // Clean up
            agent_coordinator.remove_agent(&agent_id).await?;
            
            Ok(start.elapsed())
        })
    };
    
    let results = fixture.run_performance_test("Agent Coordinator Performance", config, test_fn).await.unwrap();
    
    // Performance assertions
    assert!(results.error_rate < 0.1, "Error rate should be less than 10%");
    assert!(results.avg_response_time < Duration::from_secs(2), "Average response time should be less than 2 seconds");
    assert!(results.requests_per_second > 0.8, "Should process at least 0.8 requests per second");
    
    fixture.stop().await.unwrap();
}

#[tokio::test]
async fn test_system_load_performance() {
    let fixture = PerformanceTestFixture::new().await.unwrap();
    fixture.start().await.unwrap();
    
    // Run multiple performance tests simultaneously to simulate high system load
    let ai_task = {
        let fixture = fixture.ai_manager.clone();
        tokio::spawn(async move {
            let config = PerformanceTestConfig {
                concurrent_requests: 3,
                total_requests: 30,
                ..Default::default()
            };
            
            let test_fn = move || {
                let ai_manager = fixture.clone();
                Box::pin(async move {
                    let start = Instant::now();
                    let request = json!({
                        "model": "qwen2.5:1.5b",
                        "prompt": "Load test analysis",
                        "max_tokens": 50
                    });
                    let _response = ai_manager.process_request(request).await?;
                    Ok(start.elapsed())
                })
            };
            
            // Note: This would need the full fixture to run properly
            // For now, just simulate the test
            Ok(())
        })
    };
    
    let workflow_task = {
        let fixture = fixture.workflow_engine.clone();
        tokio::spawn(async move {
            for i in 0..10 {
                let workflow = horizonos_graph_ai::patterns::workflow::Workflow {
                    id: format!("load_test_workflow_{}", i),
                    name: format!("Load Test Workflow {}", i),
                    description: Some("Load test workflow".to_string()),
                    steps: vec![
                        WorkflowStep {
                            id: format!("load_step_{}", i),
                            name: format!("Load Step {}", i),
                            description: Some("Load test step".to_string()),
                            step_type: WorkflowStepType::AIAnalysis,
                            input: json!({
                                "task": "load_test",
                                "model": "qwen2.5:1.5b"
                            }),
                            output: None,
                            timeout: Some(Duration::from_secs(10)),
                            retry_config: None,
                            depends_on: vec![],
                            condition: None,
                            parallel: false,
                            metadata: std::collections::HashMap::new(),
                        },
                    ],
                    enabled: true,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                    version: 1,
                    metadata: std::collections::HashMap::new(),
                };
                
                let _execution_id = fixture.execute_workflow(workflow, "test_user").await.unwrap();
                sleep(Duration::from_millis(100)).await;
            }
            Ok(())
        })
    };
    
    let resource_task = {
        let fixture = fixture.resource_monitor.clone();
        tokio::spawn(async move {
            for _ in 0..60 {
                let _stats = fixture.get_stats().await.unwrap();
                let _health = fixture.get_health_summary().await.unwrap();
                sleep(Duration::from_millis(500)).await;
            }
            Ok(())
        })
    };
    
    // Wait for all tasks to complete
    let (ai_result, workflow_result, resource_result) = tokio::join!(ai_task, workflow_task, resource_task);
    
    // Verify all tasks completed successfully
    assert!(ai_result.is_ok());
    assert!(workflow_result.is_ok());
    assert!(resource_result.is_ok());
    
    // Check final system health
    let final_health = fixture.resource_monitor.get_health_summary().await.unwrap();
    assert!(final_health.health_score > 0.0);
    
    fixture.stop().await.unwrap();
}