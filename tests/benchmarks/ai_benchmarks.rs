//! Performance benchmarks for HorizonOS AI system
//! 
//! Run with: cargo bench --features benchmarks

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use horizonos_ai::*;
use std::time::Duration;
use tokio::runtime::Runtime;

/// Benchmark hardware detection performance
fn bench_hardware_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("hardware_detection");
    
    group.bench_function("full_detection", |b| {
        b.iter(|| {
            hardware::detect_hardware_profile().unwrap()
        });
    });
    
    group.bench_function("cpu_only", |b| {
        b.iter(|| {
            hardware::detect_cpu_info()
        });
    });
    
    group.bench_function("gpu_only", |b| {
        b.iter(|| {
            hardware::detect_gpu_info()
        });
    });
    
    group.bench_function("memory_only", |b| {
        b.iter(|| {
            hardware::detect_memory_info()
        });
    });
    
    group.finish();
}

/// Benchmark Ollama client performance
fn bench_ollama_client(c: &mut Criterion) {
    let runtime = Runtime::new().unwrap();
    let ollama_client = runtime.block_on(async {
        ollama::OllamaClient::new("http://localhost:11434")
    });
    
    let mut group = c.benchmark_group("ollama_client");
    group.measurement_time(Duration::from_secs(10));
    
    // Benchmark connection pooling
    group.bench_function("connection_pool_acquire", |b| {
        b.to_async(&runtime).iter(|| async {
            ollama_client.acquire_connection().await.unwrap()
        });
    });
    
    // Benchmark model selection
    group.bench_function("model_selection", |b| {
        let hardware_profile = hardware::detect_hardware_profile().unwrap();
        b.iter(|| {
            hardware::select_optimal_model(&hardware_profile, HardwareOptimization::Auto)
        });
    });
    
    // Benchmark response streaming
    let prompts = vec![
        "Hello",
        "What is 2+2?",
        "Explain quantum computing in one sentence.",
    ];
    
    for prompt in prompts {
        group.bench_with_input(
            BenchmarkId::new("generate", prompt.len()),
            prompt,
            |b, prompt| {
                b.to_async(&runtime).iter(|| async {
                    ollama_client.generate(
                        "llama3.2:latest",
                        prompt,
                        Default::default()
                    ).await.unwrap()
                });
            }
        );
    }
    
    group.finish();
}

/// Benchmark storage operations
fn bench_storage(c: &mut Criterion) {
    let runtime = Runtime::new().unwrap();
    let storage = runtime.block_on(async {
        storage::StorageManager::new_default()
    });
    
    let mut group = c.benchmark_group("storage");
    
    // Benchmark single action storage
    let action = monitoring::UserAction {
        time: chrono::Utc::now(),
        user_id: "bench-user".to_string(),
        action_type: "app_launch".to_string(),
        target: "firefox".to_string(),
        context: serde_json::json!({"benchmark": true}),
        duration_ms: Some(100),
        success: true,
        error_message: None,
        metadata: Default::default(),
    };
    
    group.bench_function("store_single_action", |b| {
        b.to_async(&runtime).iter(|| async {
            storage.store_user_action(action.clone()).await.unwrap()
        });
    });
    
    // Benchmark batch storage
    let batch_sizes = vec![10, 100, 1000];
    
    for size in batch_sizes {
        let actions: Vec<_> = (0..size)
            .map(|i| monitoring::UserAction {
                time: chrono::Utc::now(),
                user_id: format!("user-{}", i),
                action_type: "test".to_string(),
                target: format!("target-{}", i),
                context: serde_json::json!({}),
                duration_ms: Some(50),
                success: true,
                error_message: None,
                metadata: Default::default(),
            })
            .collect();
        
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::new("store_batch", size),
            &actions,
            |b, actions| {
                b.to_async(&runtime).iter(|| async {
                    storage.store_user_actions_batch(actions.clone()).await.unwrap()
                });
            }
        );
    }
    
    // Benchmark query operations
    group.bench_function("query_recent_actions", |b| {
        b.to_async(&runtime).iter(|| async {
            storage.query_user_actions(
                Some("bench-user"),
                None,
                None,
                100
            ).await.unwrap()
        });
    });
    
    // Benchmark pattern detection
    group.bench_function("detect_patterns", |b| {
        b.to_async(&runtime).iter(|| async {
            storage.detect_patterns("bench-user", 24).await.unwrap()
        });
    });
    
    group.finish();
}

/// Benchmark privacy operations
fn bench_privacy(c: &mut Criterion) {
    let runtime = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("privacy");
    
    // Benchmark anonymization
    let anonymizer = runtime.block_on(async {
        privacy::AnonymizationEngine::new(Default::default()).await.unwrap()
    });
    
    let test_data = vec![
        "Simple text without PII",
        "Email: john.doe@example.com",
        "Phone: 555-123-4567, SSN: 123-45-6789",
        "Complex data with john@example.com, 555-9876, IP: 192.168.1.1",
    ];
    
    for data in test_data {
        group.bench_with_input(
            BenchmarkId::new("anonymize", data.len()),
            data,
            |b, data| {
                b.to_async(&runtime).iter(|| async {
                    anonymizer.anonymize(data).await.unwrap()
                });
            }
        );
    }
    
    // Benchmark encryption
    let encryption_mgr = runtime.block_on(async {
        privacy::EncryptionManager::new(Default::default()).await.unwrap()
    });
    
    let data_sizes = vec![1024, 10240, 102400]; // 1KB, 10KB, 100KB
    
    for size in data_sizes {
        let data = vec![0u8; size];
        group.throughput(Throughput::Bytes(size as u64));
        
        group.bench_with_input(
            BenchmarkId::new("encrypt", size),
            &data,
            |b, data| {
                b.to_async(&runtime).iter(|| async {
                    encryption_mgr.encrypt(data).await.unwrap()
                });
            }
        );
    }
    
    // Benchmark consent checking
    let consent_mgr = runtime.block_on(async {
        privacy::ConsentManager::new(Default::default()).await.unwrap()
    });
    
    group.bench_function("check_consent", |b| {
        b.to_async(&runtime).iter(|| async {
            consent_mgr.has_consent("behavioral-learning").await.unwrap()
        });
    });
    
    group.finish();
}

/// Benchmark monitoring system
fn bench_monitoring(c: &mut Criterion) {
    let runtime = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("monitoring");
    
    // Benchmark event filtering
    let privacy_filter = monitoring::PrivacyFilter::new(Default::default());
    
    let events = vec![
        monitoring::MonitoringEvent {
            event_type: monitoring::EventType::AppLaunch,
            timestamp: chrono::Utc::now(),
            user: Some("test".to_string()),
            application: Some("firefox".to_string()),
            window_title: Some("Google Search".to_string()),
            url: None,
            file_path: None,
            metadata: Default::default(),
        },
        monitoring::MonitoringEvent {
            event_type: monitoring::EventType::FileAccess,
            timestamp: chrono::Utc::now(),
            user: Some("test".to_string()),
            application: None,
            window_title: None,
            url: None,
            file_path: Some("/home/user/document.pdf".to_string()),
            metadata: Default::default(),
        },
    ];
    
    for event in events {
        group.bench_with_input(
            BenchmarkId::new("filter_event", format!("{:?}", event.event_type)),
            &event,
            |b, event| {
                b.iter(|| {
                    privacy_filter.filter_event(event.clone())
                });
            }
        );
    }
    
    // Benchmark idle detection
    let idle_detector = monitoring::IdleDetector::new(Default::default());
    
    group.bench_function("check_idle", |b| {
        b.iter(|| {
            idle_detector.is_idle()
        });
    });
    
    // Benchmark resource monitoring
    let resource_monitor = monitoring::ResourceMonitor::new();
    
    group.bench_function("get_system_resources", |b| {
        b.to_async(&runtime).iter(|| async {
            resource_monitor.get_current_usage().await
        });
    });
    
    group.finish();
}

/// Benchmark agent operations
fn bench_agents(c: &mut Criterion) {
    let runtime = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("agents");
    group.measurement_time(Duration::from_secs(20));
    
    // Benchmark task decomposition
    let decomposer = agents::TaskDecomposer::new();
    
    let tasks = vec![
        "Write a simple Python script",
        "Research and summarize quantum computing concepts",
        "Create a REST API with authentication and database",
    ];
    
    for task in tasks {
        group.bench_with_input(
            BenchmarkId::new("decompose_task", task.len()),
            task,
            |b, task| {
                b.to_async(&runtime).iter(|| async {
                    decomposer.decompose_task(task, None).await.unwrap()
                });
            }
        );
    }
    
    // Benchmark memory operations
    let memory = runtime.block_on(async {
        agents::AgentMemory::new(Default::default()).await.unwrap()
    });
    
    let memory_sizes = vec![10, 100, 1000];
    
    for size in memory_sizes {
        let memories: Vec<_> = (0..size)
            .map(|i| agents::Memory {
                id: format!("mem-{}", i),
                memory_type: agents::MemoryType::Episodic,
                content: format!("Memory content {}", i),
                timestamp: chrono::Utc::now(),
                importance: 0.5,
                metadata: Default::default(),
            })
            .collect();
        
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::new("store_memories", size),
            &memories,
            |b, memories| {
                b.to_async(&runtime).iter(|| async {
                    for memory in memories {
                        memory.store(memory.clone()).await.unwrap();
                    }
                });
            }
        );
    }
    
    // Benchmark agent communication
    let channel = agents::CommunicationChannel::new(1000);
    
    group.bench_function("send_message", |b| {
        let message = agents::AgentMessage {
            id: "test-msg".to_string(),
            from: "agent1".to_string(),
            to: "agent2".to_string(),
            message_type: agents::MessageType::TaskRequest,
            content: serde_json::json!({"task": "test"}),
            timestamp: chrono::Utc::now(),
            reply_to: None,
        };
        
        b.to_async(&runtime).iter(|| async {
            channel.send(message.clone()).await.unwrap()
        });
    });
    
    group.finish();
}

/// Benchmark automation operations
fn bench_automation(c: &mut Criterion) {
    let runtime = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("automation");
    
    // Benchmark workflow validation
    let scheduler = runtime.block_on(async {
        automation::WorkflowScheduler::new(Default::default()).await.unwrap()
    });
    
    let workflow = automation::WorkflowDefinition {
        id: "bench-workflow".to_string(),
        name: "Benchmark Workflow".to_string(),
        description: "Test workflow".to_string(),
        trigger: automation::WorkflowTrigger::Manual,
        steps: vec![
            automation::WorkflowStep {
                id: "step1".to_string(),
                name: "Step 1".to_string(),
                action: automation::ActionType::Custom("test".to_string()),
                parameters: serde_json::json!({}),
                retry_policy: None,
                timeout: None,
            },
        ],
        enabled: true,
    };
    
    group.bench_function("validate_workflow", |b| {
        b.iter(|| {
            scheduler.validate_workflow(&workflow)
        });
    });
    
    // Benchmark browser automation setup
    group.bench_function("create_browser_context", |b| {
        b.to_async(&runtime).iter(|| async {
            let browser = automation::BrowserAutomation::new(Default::default()).await.unwrap();
            browser.create_context(Default::default()).await.unwrap()
        });
    });
    
    group.finish();
}

/// Main benchmark groups
criterion_group!(
    benches,
    bench_hardware_detection,
    bench_ollama_client,
    bench_storage,
    bench_privacy,
    bench_monitoring,
    bench_agents,
    bench_automation
);

criterion_main!(benches);