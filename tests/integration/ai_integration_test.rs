//! Integration tests for HorizonOS AI system
//! 
//! These tests verify the complete AI integration stack works correctly
//! when all components are running together.

#[cfg(test)]
mod tests {
    use super::*;
    use horizonos_ai::*;
    use tokio::test;
    use std::time::Duration;
    
    /// Test complete AI system initialization
    #[tokio::test]
    async fn test_ai_system_initialization() {
        // Initialize AI system
        let ai_config = AIConfig {
            enabled: true,
            ollama_endpoint: "http://localhost:11434".to_string(),
            default_model: "llama3.2:latest".to_string(),
            hardware_optimization: HardwareOptimization::Auto,
            privacy: PrivacyConfig::default(),
            learning: LearningConfig::default(),
            suggestions: SuggestionConfig::default(),
        };
        
        let ai_service = AIService::new(ai_config);
        let result = ai_service.initialize().await;
        
        assert!(result.is_ok(), "AI system should initialize successfully");
        assert!(ai_service.is_ready(), "AI system should be ready after initialization");
    }
    
    /// Test hardware detection and model selection
    #[tokio::test]
    async fn test_hardware_detection() {
        let hardware_profile = hardware::detect_hardware_profile().unwrap();
        
        assert!(hardware_profile.cpu.cores > 0, "Should detect CPU cores");
        assert!(hardware_profile.memory.total > 0, "Should detect total memory");
        
        // Test model selection based on hardware
        let selected_model = hardware::select_optimal_model(
            &hardware_profile,
            HardwareOptimization::Auto
        );
        
        assert!(!selected_model.is_empty(), "Should select a model");
        
        // Verify model is appropriate for hardware
        if hardware_profile.memory.total < 8_000_000_000 {
            assert!(selected_model.contains("small") || selected_model.contains("tiny"),
                   "Should select small model for low memory");
        }
    }
    
    /// Test Ollama integration
    #[tokio::test]
    async fn test_ollama_integration() {
        let ollama_client = ollama::OllamaClient::new("http://localhost:11434");
        
        // Test connection
        let connection_result = ollama_client.test_connection().await;
        assert!(connection_result.is_ok(), "Should connect to Ollama");
        
        // Test model listing
        let models = ollama_client.list_models().await.unwrap();
        assert!(!models.is_empty(), "Should have at least one model available");
        
        // Test simple generation
        let response = ollama_client.generate(
            "llama3.2:latest",
            "Hello, how are you?",
            Default::default()
        ).await;
        
        assert!(response.is_ok(), "Should generate response");
        assert!(!response.unwrap().is_empty(), "Response should not be empty");
    }
    
    /// Test TimescaleDB storage integration
    #[tokio::test]
    async fn test_storage_integration() {
        let storage = storage::StorageManager::new_with_config(
            storage::StorageConfig {
                timescale_url: "postgresql://postgres:password@localhost:5432/horizonos".to_string(),
                redis_url: "redis://localhost:6379".to_string(),
                cache_ttl: Duration::from_secs(300),
                batch_size: 100,
            }
        ).await.unwrap();
        
        // Test storing user action
        let action = monitoring::UserAction {
            time: chrono::Utc::now(),
            user_id: "test-user".to_string(),
            action_type: "app_launch".to_string(),
            target: "firefox".to_string(),
            context: serde_json::json!({"source": "test"}),
            duration_ms: Some(100),
            success: true,
            error_message: None,
            metadata: Default::default(),
        };
        
        let result = storage.store_user_action(action).await;
        assert!(result.is_ok(), "Should store user action");
        
        // Test querying actions
        let actions = storage.query_user_actions(
            Some("test-user"),
            None,
            None,
            10
        ).await.unwrap();
        
        assert!(!actions.is_empty(), "Should retrieve stored actions");
    }
    
    /// Test privacy controls
    #[tokio::test]
    async fn test_privacy_controls() {
        let privacy_manager = privacy::PrivacyManager::new(
            privacy::PrivacyConfig::default()
        ).await.unwrap();
        
        // Test consent management
        let consent_result = privacy_manager.request_consent(
            "behavioral-learning",
            "To provide personalized suggestions"
        ).await;
        
        assert!(consent_result.is_ok(), "Should handle consent request");
        
        // Test data anonymization
        let sensitive_data = "My email is john.doe@example.com and phone is 555-123-4567";
        let anonymized = privacy_manager.anonymize_data(sensitive_data).await.unwrap();
        
        assert!(!anonymized.contains("john.doe@example.com"), "Should anonymize email");
        assert!(!anonymized.contains("555-123-4567"), "Should anonymize phone");
        
        // Test audit logging
        let audit_result = privacy_manager.log_data_access(
            "test-user",
            "user_profile",
            "read"
        ).await;
        
        assert!(audit_result.is_ok(), "Should log data access");
    }
    
    /// Test monitoring system
    #[tokio::test]
    async fn test_monitoring_system() {
        let monitor = monitoring::EventMonitor::new(
            monitoring::MonitorConfig::default()
        );
        
        // Start monitoring
        monitor.start().await.unwrap();
        
        // Wait a bit for events
        tokio::time::sleep(Duration::from_secs(2)).await;
        
        // Check if monitoring is active
        assert!(monitor.is_monitoring(), "Monitor should be active");
        
        // Stop monitoring
        monitor.stop().await.unwrap();
        assert!(!monitor.is_monitoring(), "Monitor should be stopped");
    }
    
    /// Test automation framework
    #[tokio::test]
    async fn test_automation_framework() {
        let automation = automation::AutomationOrchestrator::new(
            automation::AutomationConfig {
                n8n_endpoint: "http://localhost:5678".to_string(),
                temporal_endpoint: "localhost:7233".to_string(),
                enable_browser: true,
                enable_ui: true,
            }
        ).await.unwrap();
        
        // Test workflow creation
        let workflow = automation::WorkflowDefinition {
            id: "test-workflow".to_string(),
            name: "Test Workflow".to_string(),
            description: "Integration test workflow".to_string(),
            trigger: automation::WorkflowTrigger::Manual,
            steps: vec![
                automation::WorkflowStep {
                    id: "step1".to_string(),
                    name: "Log Message".to_string(),
                    action: automation::ActionType::Custom("log".to_string()),
                    parameters: serde_json::json!({"message": "Test"}),
                    retry_policy: None,
                    timeout: None,
                },
            ],
            enabled: true,
        };
        
        let result = automation.create_workflow(workflow).await;
        assert!(result.is_ok(), "Should create workflow");
        
        // Test workflow execution
        let execution_result = automation.execute_workflow("test-workflow").await;
        assert!(execution_result.is_ok(), "Should execute workflow");
    }
    
    /// Test AI agent framework
    #[tokio::test]
    async fn test_agent_framework() {
        let coordinator = agents::AgentCoordinator::new(
            agents::CoordinatorConfig::default()
        );
        
        // Create a test agent
        let agent_config = agents::AgentConfig {
            name: "test-agent".to_string(),
            agent_type: agents::AgentType::TaskAgent,
            capabilities: vec!["testing".to_string()],
            model: "llama3.2:latest".to_string(),
            temperature: 0.7,
            max_tokens: 1024,
            timeout: Duration::from_secs(30),
        };
        
        let agent = coordinator.create_agent(agent_config).await.unwrap();
        assert_eq!(agent.status(), agents::AgentStatus::Idle);
        
        // Test task execution
        let task = agents::Task {
            id: "test-task".to_string(),
            description: "Test task".to_string(),
            priority: agents::TaskPriority::Normal,
            required_capabilities: vec!["testing".to_string()],
            timeout: Some(Duration::from_secs(30)),
            dependencies: vec![],
            metadata: Default::default(),
        };
        
        let result = coordinator.execute_task(task).await;
        assert!(result.is_ok(), "Should execute task");
    }
    
    /// Test end-to-end workflow with all components
    #[tokio::test]
    async fn test_end_to_end_workflow() {
        // Initialize complete AI system
        let ai_service = AIService::new(AIConfig::default());
        ai_service.initialize().await.unwrap();
        
        // Start monitoring
        let monitor = ai_service.get_monitor();
        monitor.start().await.unwrap();
        
        // Simulate user action
        let action = monitoring::UserAction {
            time: chrono::Utc::now(),
            user_id: "test-user".to_string(),
            action_type: "app_launch".to_string(),
            target: "firefox".to_string(),
            context: serde_json::json!({"test": true}),
            duration_ms: Some(150),
            success: true,
            error_message: None,
            metadata: Default::default(),
        };
        
        // Process action through the system
        ai_service.process_user_action(action).await.unwrap();
        
        // Wait for processing
        tokio::time::sleep(Duration::from_secs(1)).await;
        
        // Check if pattern was detected
        let patterns = ai_service.get_detected_patterns("test-user").await.unwrap();
        
        // For integration test, we just verify the system processes without errors
        assert!(patterns.is_ok(), "Should retrieve patterns without error");
        
        // Stop monitoring
        monitor.stop().await.unwrap();
    }
    
    /// Test system resilience and error handling
    #[tokio::test]
    async fn test_system_resilience() {
        let ai_service = AIService::new(AIConfig::default());
        
        // Test with invalid Ollama endpoint
        let mut config = AIConfig::default();
        config.ollama_endpoint = "http://invalid:99999".to_string();
        
        let service_with_bad_config = AIService::new(config);
        let result = service_with_bad_config.initialize().await;
        
        assert!(result.is_err(), "Should fail with invalid endpoint");
        
        // Test recovery
        let recovery_result = service_with_bad_config.recover().await;
        assert!(recovery_result.is_ok(), "Should attempt recovery");
    }
}

/// Performance benchmarks module
#[cfg(all(test, not(debug_assertions)))]
mod benchmarks {
    use super::*;
    use criterion::{criterion_group, criterion_main, Criterion};
    
    fn benchmark_hardware_detection(c: &mut Criterion) {
        c.bench_function("hardware_detection", |b| {
            b.iter(|| {
                hardware::detect_hardware_profile().unwrap()
            });
        });
    }
    
    fn benchmark_anonymization(c: &mut Criterion) {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let anonymizer = runtime.block_on(async {
            privacy::AnonymizationEngine::new(Default::default()).await.unwrap()
        });
        
        let test_data = "John Doe, email: john@example.com, phone: 555-123-4567";
        
        c.bench_function("data_anonymization", |b| {
            b.iter(|| {
                runtime.block_on(async {
                    anonymizer.anonymize(test_data).await.unwrap()
                })
            });
        });
    }
    
    criterion_group!(benches, benchmark_hardware_detection, benchmark_anonymization);
    criterion_main!(benches);
}