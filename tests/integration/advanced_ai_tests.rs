//! Advanced AI Integration Tests
//! 
//! Comprehensive test scenarios for complex AI workflows and interactions.

use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use horizonos_graph_ai::{
    AIManager, AIConfig, AIError,
    monitoring::resource_monitor::{ResourceMonitor, ResourceConfig},
    automation::scheduler::{WorkflowScheduler, SchedulerConfig, Schedule, ScheduleType, ScheduleConfig},
    patterns::workflow::{WorkflowEngine, WorkflowConfig, WorkflowStep, WorkflowStepType},
    suggestions::agent::{SuggestionAgent, SuggestionConfig},
    ollama::{OllamaClient, OllamaConfig},
    agents::coordinator::{AgentCoordinator, CoordinatorConfig},
    storage::config::{ConfigManager, ConfigLayer, ConfigValue},
};

/// Test fixture for advanced AI integration tests
pub struct AdvancedAITestFixture {
    pub ai_manager: AIManager,
    pub resource_monitor: ResourceMonitor,
    pub scheduler: WorkflowScheduler,
    pub workflow_engine: WorkflowEngine,
    pub suggestion_agent: SuggestionAgent,
    pub ollama_client: OllamaClient,
    pub agent_coordinator: AgentCoordinator,
    pub config_manager: ConfigManager,
}

impl AdvancedAITestFixture {
    /// Create a new test fixture with comprehensive AI components
    pub async fn new() -> Result<Self, AIError> {
        // Initialize configuration manager
        let config_manager = ConfigManager::new()?;
        
        // Configure AI manager
        let ai_config = AIConfig {
            enabled: true,
            model_name: "qwen2.5:1.5b".to_string(),
            max_context_length: 8192,
            temperature: 0.7,
            max_tokens: 1024,
            hardware_optimization: true,
            concurrent_requests: 4,
            request_timeout: Duration::from_secs(30),
            cache_size: 100,
            background_processing: true,
            privacy_mode: true,
            local_only: true,
            data_retention_hours: 24,
            user_consent_required: true,
            metrics_enabled: true,
            rate_limit: 100,
        };
        
        let ai_manager = AIManager::new(ai_config).await?;
        
        // Initialize resource monitor
        let resource_config = ResourceConfig {
            enabled: true,
            monitoring_interval: 5, // Fast monitoring for tests
            ..Default::default()
        };
        let resource_monitor = ResourceMonitor::new(resource_config).await?;
        
        // Initialize workflow scheduler
        let scheduler_config = SchedulerConfig {
            enabled: true,
            max_concurrent_workflows: 5,
            check_interval: 10, // Check every 10 seconds in tests
            ..Default::default()
        };
        let scheduler = WorkflowScheduler::new(scheduler_config).await?;
        
        // Initialize workflow engine
        let workflow_config = WorkflowConfig {
            enabled: true,
            max_concurrent_workflows: 10,
            execution_timeout: Duration::from_secs(300),
            ..Default::default()
        };
        let workflow_engine = WorkflowEngine::new(workflow_config).await?;
        
        // Initialize suggestion agent
        let suggestion_config = SuggestionConfig {
            enabled: true,
            max_suggestions: 10,
            ..Default::default()
        };
        let suggestion_agent = SuggestionAgent::new(suggestion_config).await?;
        
        // Initialize Ollama client
        let ollama_config = OllamaConfig {
            base_url: "http://localhost:11434".to_string(),
            model_name: "qwen2.5:1.5b".to_string(),
            ..Default::default()
        };
        let ollama_client = OllamaClient::new(ollama_config).await?;
        
        // Initialize agent coordinator
        let coordinator_config = CoordinatorConfig {
            enabled: true,
            max_agents: 20,
            ..Default::default()
        };
        let agent_coordinator = AgentCoordinator::new(coordinator_config).await?;
        
        Ok(Self {
            ai_manager,
            resource_monitor,
            scheduler,
            workflow_engine,
            suggestion_agent,
            ollama_client,
            agent_coordinator,
            config_manager,
        })
    }
    
    /// Start all components
    pub async fn start(&self) -> Result<(), AIError> {
        self.ai_manager.start().await?;
        self.resource_monitor.start().await?;
        self.scheduler.start().await?;
        self.workflow_engine.start().await?;
        self.suggestion_agent.start().await?;
        self.agent_coordinator.start().await?;
        Ok(())
    }
    
    /// Stop all components
    pub async fn stop(&self) -> Result<(), AIError> {
        self.ai_manager.stop().await?;
        self.resource_monitor.stop().await?;
        self.scheduler.stop().await?;
        self.workflow_engine.stop().await?;
        self.suggestion_agent.stop().await?;
        self.agent_coordinator.stop().await?;
        Ok(())
    }
}

#[tokio::test]
async fn test_comprehensive_ai_workflow() {
    let fixture = AdvancedAITestFixture::new().await.unwrap();
    fixture.start().await.unwrap();
    
    // Create a complex workflow with multiple steps
    let workflow_id = Uuid::new_v4().to_string();
    let workflow_steps = vec![
        WorkflowStep {
            id: "analyze_data".to_string(),
            name: "Analyze Data".to_string(),
            description: Some("Analyze input data using AI".to_string()),
            step_type: WorkflowStepType::AIAnalysis,
            input: json!({
                "data": "Sample data for analysis",
                "model": "qwen2.5:1.5b"
            }),
            output: None,
            timeout: Some(Duration::from_secs(60)),
            retry_config: None,
            depends_on: vec![],
            condition: None,
            parallel: false,
            metadata: std::collections::HashMap::new(),
        },
        WorkflowStep {
            id: "generate_suggestions".to_string(),
            name: "Generate Suggestions".to_string(),
            description: Some("Generate suggestions based on analysis".to_string()),
            step_type: WorkflowStepType::SuggestionGeneration,
            input: json!({
                "context": "analysis_result",
                "max_suggestions": 5
            }),
            output: None,
            timeout: Some(Duration::from_secs(30)),
            retry_config: None,
            depends_on: vec!["analyze_data".to_string()],
            condition: None,
            parallel: false,
            metadata: std::collections::HashMap::new(),
        },
        WorkflowStep {
            id: "resource_check".to_string(),
            name: "Resource Check".to_string(),
            description: Some("Check system resources".to_string()),
            step_type: WorkflowStepType::ResourceCheck,
            input: json!({}),
            output: None,
            timeout: Some(Duration::from_secs(15)),
            retry_config: None,
            depends_on: vec![],
            condition: None,
            parallel: true,
            metadata: std::collections::HashMap::new(),
        },
    ];
    
    // Create and execute workflow
    let workflow = horizonos_graph_ai::patterns::workflow::Workflow {
        id: workflow_id.clone(),
        name: "Comprehensive AI Test Workflow".to_string(),
        description: Some("Test workflow for comprehensive AI integration".to_string()),
        steps: workflow_steps,
        enabled: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        version: 1,
        metadata: std::collections::HashMap::new(),
    };
    
    let execution_id = fixture.workflow_engine.execute_workflow(workflow, "test_user").await.unwrap();
    
    // Wait for workflow completion
    let mut attempts = 0;
    let max_attempts = 30; // 30 seconds timeout
    
    loop {
        if attempts >= max_attempts {
            panic!("Workflow execution timed out");
        }
        
        if let Ok(result) = fixture.workflow_engine.get_execution_result(&execution_id).await {
            if result.is_complete() {
                assert!(result.is_success());
                break;
            }
        }
        
        sleep(Duration::from_secs(1)).await;
        attempts += 1;
    }
    
    fixture.stop().await.unwrap();
}

#[tokio::test]
async fn test_multi_agent_coordination() {
    let fixture = AdvancedAITestFixture::new().await.unwrap();
    fixture.start().await.unwrap();
    
    // Create multiple agents for different tasks
    let analysis_agent = fixture.agent_coordinator.create_agent("analysis_agent", "data_analysis").await.unwrap();
    let suggestion_agent = fixture.agent_coordinator.create_agent("suggestion_agent", "suggestion_generation").await.unwrap();
    let monitoring_agent = fixture.agent_coordinator.create_agent("monitoring_agent", "system_monitoring").await.unwrap();
    
    // Create coordination tasks
    let coordination_tasks = vec![
        json!({
            "agent_id": analysis_agent,
            "task": "analyze_system_logs",
            "priority": "high",
            "data": {"logs": "sample log data"}
        }),
        json!({
            "agent_id": suggestion_agent,
            "task": "generate_optimization_suggestions",
            "priority": "medium",
            "data": {"context": "system_performance"}
        }),
        json!({
            "agent_id": monitoring_agent,
            "task": "monitor_resources",
            "priority": "low",
            "data": {"interval": 5}
        }),
    ];
    
    // Execute coordination tasks
    let mut task_ids = Vec::new();
    for task in coordination_tasks {
        let task_id = fixture.agent_coordinator.assign_task(task).await.unwrap();
        task_ids.push(task_id);
    }
    
    // Wait for task completion
    sleep(Duration::from_secs(10)).await;
    
    // Check task results
    for task_id in task_ids {
        let result = fixture.agent_coordinator.get_task_result(&task_id).await.unwrap();
        assert!(result.is_some());
    }
    
    fixture.stop().await.unwrap();
}

#[tokio::test]
async fn test_scheduled_ai_workflows() {
    let fixture = AdvancedAITestFixture::new().await.unwrap();
    fixture.start().await.unwrap();
    
    // Create a simple workflow for scheduling
    let workflow_id = "scheduled_analysis_workflow".to_string();
    let workflow_steps = vec![
        WorkflowStep {
            id: "periodic_analysis".to_string(),
            name: "Periodic Analysis".to_string(),
            description: Some("Periodic system analysis".to_string()),
            step_type: WorkflowStepType::AIAnalysis,
            input: json!({
                "task": "system_health_check",
                "model": "qwen2.5:1.5b"
            }),
            output: None,
            timeout: Some(Duration::from_secs(30)),
            retry_config: None,
            depends_on: vec![],
            condition: None,
            parallel: false,
            metadata: std::collections::HashMap::new(),
        },
    ];
    
    let workflow = horizonos_graph_ai::patterns::workflow::Workflow {
        id: workflow_id.clone(),
        name: "Scheduled Analysis".to_string(),
        description: Some("Scheduled periodic analysis".to_string()),
        steps: workflow_steps,
        enabled: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        version: 1,
        metadata: std::collections::HashMap::new(),
    };
    
    // Register workflow
    fixture.workflow_engine.register_workflow(workflow).await.unwrap();
    
    // Create schedule for the workflow
    let schedule = Schedule {
        id: "".to_string(), // Will be assigned
        name: "Periodic Analysis Schedule".to_string(),
        description: Some("Schedule for periodic analysis".to_string()),
        schedule_type: ScheduleType::Interval,
        workflow_id: workflow_id.clone(),
        user_id: "test_user".to_string(),
        enabled: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        next_execution: None,
        last_execution: None,
        execution_count: 0,
        max_executions: Some(3), // Limit to 3 executions for test
        config: ScheduleConfig {
            interval_seconds: Some(15), // Every 15 seconds
            ..Default::default()
        },
        metadata: std::collections::HashMap::new(),
    };
    
    // Schedule the workflow
    let schedule_id = fixture.scheduler.schedule_workflow(&workflow_id, schedule, "test_user").await.unwrap();
    
    // Wait for several executions
    sleep(Duration::from_secs(60)).await;
    
    // Check execution history
    let history = fixture.scheduler.get_execution_history(Some(&schedule_id));
    assert!(history.len() >= 2, "Expected at least 2 executions, got {}", history.len());
    
    fixture.stop().await.unwrap();
}

#[tokio::test]
async fn test_resource_adaptive_behavior() {
    let fixture = AdvancedAITestFixture::new().await.unwrap();
    fixture.start().await.unwrap();
    
    // Get initial resource stats
    let initial_stats = fixture.resource_monitor.get_stats().await.unwrap();
    assert!(initial_stats.timestamp > Utc::now() - chrono::Duration::minutes(1));
    
    // Simulate high resource usage scenario
    // This would typically trigger adaptive behavior in the AI system
    let high_load_workflow = horizonos_graph_ai::patterns::workflow::Workflow {
        id: "high_load_test".to_string(),
        name: "High Load Test".to_string(),
        description: Some("Test workflow for high resource usage".to_string()),
        steps: vec![
            WorkflowStep {
                id: "resource_intensive_task".to_string(),
                name: "Resource Intensive Task".to_string(),
                description: Some("Task that uses significant resources".to_string()),
                step_type: WorkflowStepType::AIAnalysis,
                input: json!({
                    "task": "complex_analysis",
                    "model": "qwen2.5:1.5b",
                    "complexity": "high"
                }),
                output: None,
                timeout: Some(Duration::from_secs(60)),
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
    
    // Execute multiple workflows simultaneously to increase load
    let mut execution_ids = Vec::new();
    for i in 0..3 {
        let mut workflow = high_load_workflow.clone();
        workflow.id = format!("high_load_test_{}", i);
        
        let execution_id = fixture.workflow_engine.execute_workflow(workflow, "test_user").await.unwrap();
        execution_ids.push(execution_id);
    }
    
    // Monitor resource usage during execution
    sleep(Duration::from_secs(10)).await;
    
    let current_stats = fixture.resource_monitor.get_stats().await.unwrap();
    let health_summary = fixture.resource_monitor.get_health_summary().await.unwrap();
    
    // Verify resource monitoring is working
    assert!(current_stats.timestamp > initial_stats.timestamp);
    assert!(health_summary.health_score >= 0.0 && health_summary.health_score <= 1.0);
    
    // Check for any resource alerts
    let alerts = fixture.resource_monitor.get_alerts();
    if !alerts.is_empty() {
        println!("Resource alerts generated: {}", alerts.len());
    }
    
    // Wait for workflow completion
    sleep(Duration::from_secs(30)).await;
    
    fixture.stop().await.unwrap();
}

#[tokio::test]
async fn test_configuration_management_integration() {
    let fixture = AdvancedAITestFixture::new().await.unwrap();
    
    // Test configuration layers
    let user_config = json!({
        "ai": {
            "model": "qwen2.5:1.5b",
            "temperature": 0.8,
            "max_tokens": 2048
        },
        "monitoring": {
            "enabled": true,
            "interval": 30
        }
    });
    
    fixture.config_manager.set_config(
        ConfigLayer::User,
        "test_config",
        ConfigValue::Object(user_config.as_object().unwrap().clone())
    ).await.unwrap();
    
    // Test configuration retrieval
    let retrieved_config = fixture.config_manager.get_config(ConfigLayer::User, "test_config").await.unwrap();
    assert!(retrieved_config.is_some());
    
    // Test configuration updates
    let updated_config = json!({
        "ai": {
            "model": "qwen2.5:1.5b",
            "temperature": 0.9,
            "max_tokens": 4096
        }
    });
    
    fixture.config_manager.set_config(
        ConfigLayer::User,
        "test_config",
        ConfigValue::Object(updated_config.as_object().unwrap().clone())
    ).await.unwrap();
    
    let final_config = fixture.config_manager.get_config(ConfigLayer::User, "test_config").await.unwrap();
    assert!(final_config.is_some());
    
    // Test configuration validation
    let invalid_config = json!({
        "ai": {
            "temperature": 2.0, // Invalid temperature > 1.0
            "max_tokens": -1    // Invalid negative value
        }
    });
    
    let validation_result = fixture.config_manager.validate_config(&invalid_config).await;
    assert!(validation_result.is_err());
}

#[tokio::test]
async fn test_suggestion_system_integration() {
    let fixture = AdvancedAITestFixture::new().await.unwrap();
    fixture.start().await.unwrap();
    
    // Create context for suggestions
    let context = json!({
        "user_id": "test_user",
        "current_activity": "code_analysis",
        "project_type": "rust",
        "files": ["src/main.rs", "src/lib.rs"],
        "recent_actions": ["compile", "test", "debug"]
    });
    
    // Request suggestions
    let suggestions = fixture.suggestion_agent.get_suggestions(context).await.unwrap();
    assert!(!suggestions.is_empty());
    
    // Verify suggestion quality
    for suggestion in &suggestions {
        assert!(!suggestion.title.is_empty());
        assert!(!suggestion.description.is_empty());
        assert!(suggestion.confidence >= 0.0 && suggestion.confidence <= 1.0);
    }
    
    // Test suggestion feedback
    if let Some(first_suggestion) = suggestions.first() {
        fixture.suggestion_agent.provide_feedback(
            &first_suggestion.id,
            horizonos_graph_ai::suggestions::agent::SuggestionFeedback::Helpful
        ).await.unwrap();
    }
    
    // Request suggestions again to test learning
    let new_suggestions = fixture.suggestion_agent.get_suggestions(context).await.unwrap();
    assert!(!new_suggestions.is_empty());
    
    fixture.stop().await.unwrap();
}

#[tokio::test]
async fn test_error_handling_and_recovery() {
    let fixture = AdvancedAITestFixture::new().await.unwrap();
    fixture.start().await.unwrap();
    
    // Test workflow with intentional failure
    let failing_workflow = horizonos_graph_ai::patterns::workflow::Workflow {
        id: "failing_test_workflow".to_string(),
        name: "Failing Test Workflow".to_string(),
        description: Some("Workflow designed to fail for testing".to_string()),
        steps: vec![
            WorkflowStep {
                id: "failing_step".to_string(),
                name: "Failing Step".to_string(),
                description: Some("Step that will fail".to_string()),
                step_type: WorkflowStepType::AIAnalysis,
                input: json!({
                    "invalid_model": "nonexistent_model",
                    "task": "impossible_task"
                }),
                output: None,
                timeout: Some(Duration::from_secs(10)),
                retry_config: Some(horizonos_graph_ai::patterns::workflow::RetryConfig {
                    max_attempts: 3,
                    delay_seconds: 1,
                    backoff_factor: 2.0,
                    max_delay_seconds: 10,
                }),
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
    
    // Execute failing workflow
    let execution_id = fixture.workflow_engine.execute_workflow(failing_workflow, "test_user").await.unwrap();
    
    // Wait for failure and retry attempts
    sleep(Duration::from_secs(15)).await;
    
    // Check execution result
    let result = fixture.workflow_engine.get_execution_result(&execution_id).await.unwrap();
    assert!(result.is_complete());
    assert!(result.is_failure());
    
    // Verify retry attempts were made
    assert!(result.retry_attempts > 0);
    
    fixture.stop().await.unwrap();
}

#[tokio::test]
async fn test_concurrent_operations() {
    let fixture = AdvancedAITestFixture::new().await.unwrap();
    fixture.start().await.unwrap();
    
    // Create multiple concurrent workflows
    let workflow_count = 5;
    let mut tasks = Vec::new();
    
    for i in 0..workflow_count {
        let workflow = horizonos_graph_ai::patterns::workflow::Workflow {
            id: format!("concurrent_workflow_{}", i),
            name: format!("Concurrent Workflow {}", i),
            description: Some(format!("Test workflow {} for concurrency", i)),
            steps: vec![
                WorkflowStep {
                    id: format!("step_{}", i),
                    name: format!("Step {}", i),
                    description: Some(format!("Test step {}", i)),
                    step_type: WorkflowStepType::AIAnalysis,
                    input: json!({
                        "task": format!("concurrent_task_{}", i),
                        "model": "qwen2.5:1.5b"
                    }),
                    output: None,
                    timeout: Some(Duration::from_secs(30)),
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
        
        let engine = fixture.workflow_engine.clone();
        let task = tokio::spawn(async move {
            engine.execute_workflow(workflow, "test_user").await
        });
        tasks.push(task);
    }
    
    // Wait for all tasks to complete
    let results = futures::future::join_all(tasks).await;
    
    // Verify all workflows were executed
    let mut execution_ids = Vec::new();
    for result in results {
        let execution_id = result.unwrap().unwrap();
        execution_ids.push(execution_id);
    }
    
    assert_eq!(execution_ids.len(), workflow_count);
    
    // Wait for workflow completion
    sleep(Duration::from_secs(30)).await;
    
    // Check all executions completed
    for execution_id in execution_ids {
        let result = fixture.workflow_engine.get_execution_result(&execution_id).await.unwrap();
        assert!(result.is_complete());
    }
    
    fixture.stop().await.unwrap();
}