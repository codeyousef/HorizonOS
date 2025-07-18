//! Security and Privacy Integration Tests
//! 
//! Tests for security features, privacy controls, and data protection.

use std::time::Duration;
use serde_json::json;
use chrono::Utc;
use uuid::Uuid;

use horizonos_graph_ai::{
    AIManager, AIConfig, AIError,
    storage::privacy::{PrivacyManager, PrivacyConfig, ConsentLevel, DataCategory},
    storage::encryption::{EncryptionManager, EncryptionConfig},
    storage::audit::{AuditManager, AuditConfig, AuditEvent, AuditLevel},
    storage::anonymization::{AnonymizationManager, AnonymizationConfig},
    storage::memory::{MemoryManager, MemoryConfig},
    storage::config::{ConfigManager, ConfigLayer, ConfigValue},
};

/// Security test fixture with privacy and security components
pub struct SecurityTestFixture {
    pub ai_manager: AIManager,
    pub privacy_manager: PrivacyManager,
    pub encryption_manager: EncryptionManager,
    pub audit_manager: AuditManager,
    pub anonymization_manager: AnonymizationManager,
    pub memory_manager: MemoryManager,
    pub config_manager: ConfigManager,
}

impl SecurityTestFixture {
    /// Create a new security test fixture
    pub async fn new() -> Result<Self, AIError> {
        // Initialize configuration manager
        let config_manager = ConfigManager::new()?;
        
        // Configure AI manager with privacy settings
        let ai_config = AIConfig {
            enabled: true,
            model_name: "qwen2.5:1.5b".to_string(),
            max_context_length: 4096,
            temperature: 0.7,
            max_tokens: 512,
            hardware_optimization: true,
            concurrent_requests: 4,
            request_timeout: Duration::from_secs(30),
            cache_size: 100,
            background_processing: true,
            privacy_mode: true,
            local_only: true,
            data_retention_hours: 1, // Short retention for security tests
            user_consent_required: true,
            metrics_enabled: true,
            rate_limit: 100,
        };
        
        let ai_manager = AIManager::new(ai_config).await?;
        
        // Initialize privacy manager
        let privacy_config = PrivacyConfig {
            enabled: true,
            default_consent_level: ConsentLevel::Explicit,
            data_retention_days: 1,
            anonymization_enabled: true,
            audit_enabled: true,
            encryption_enabled: true,
            user_data_export: true,
            right_to_be_forgotten: true,
            consent_expiry_days: 365,
            ..Default::default()
        };
        let privacy_manager = PrivacyManager::new(privacy_config).await?;
        
        // Initialize encryption manager
        let encryption_config = EncryptionConfig {
            enabled: true,
            encryption_key: "test_encryption_key_for_security_tests".to_string(),
            algorithm: "AES-256-GCM".to_string(),
            key_rotation_days: 30,
            ..Default::default()
        };
        let encryption_manager = EncryptionManager::new(encryption_config).await?;
        
        // Initialize audit manager
        let audit_config = AuditConfig {
            enabled: true,
            log_level: AuditLevel::Debug,
            retention_days: 7,
            audit_file: Some("/tmp/horizonos/audit_test.log".to_string()),
            ..Default::default()
        };
        let audit_manager = AuditManager::new(audit_config).await?;
        
        // Initialize anonymization manager
        let anonymization_config = AnonymizationConfig {
            enabled: true,
            anonymization_level: horizonos_graph_ai::storage::anonymization::AnonymizationLevel::High,
            hash_salt: "test_salt_for_security_tests".to_string(),
            ..Default::default()
        };
        let anonymization_manager = AnonymizationManager::new(anonymization_config).await?;
        
        // Initialize memory manager with encryption
        let memory_config = MemoryConfig {
            enabled: true,
            max_memory_mb: 256,
            cleanup_interval: Duration::from_secs(30),
            encryption_enabled: true,
            ..Default::default()
        };
        let memory_manager = MemoryManager::new(memory_config).await?;
        
        Ok(Self {
            ai_manager,
            privacy_manager,
            encryption_manager,
            audit_manager,
            anonymization_manager,
            memory_manager,
            config_manager,
        })
    }
    
    /// Start all components
    pub async fn start(&self) -> Result<(), AIError> {
        self.ai_manager.start().await?;
        self.privacy_manager.start().await?;
        self.encryption_manager.start().await?;
        self.audit_manager.start().await?;
        self.anonymization_manager.start().await?;
        self.memory_manager.start().await?;
        Ok(())
    }
    
    /// Stop all components
    pub async fn stop(&self) -> Result<(), AIError> {
        self.ai_manager.stop().await?;
        self.privacy_manager.stop().await?;
        self.encryption_manager.stop().await?;
        self.audit_manager.stop().await?;
        self.anonymization_manager.stop().await?;
        self.memory_manager.stop().await?;
        Ok(())
    }
}

#[tokio::test]
async fn test_privacy_consent_management() {
    let fixture = SecurityTestFixture::new().await.unwrap();
    fixture.start().await.unwrap();
    
    let user_id = "test_user_privacy";
    
    // Test consent granting
    let consent_granted = fixture.privacy_manager.grant_consent(
        user_id,
        DataCategory::PersonalData,
        ConsentLevel::Explicit,
        Some("Test consent for privacy management".to_string())
    ).await.unwrap();
    
    assert!(consent_granted);
    
    // Test consent checking
    let has_consent = fixture.privacy_manager.has_consent(user_id, DataCategory::PersonalData).await.unwrap();
    assert!(has_consent);
    
    // Test consent revocation
    let consent_revoked = fixture.privacy_manager.revoke_consent(user_id, DataCategory::PersonalData).await.unwrap();
    assert!(consent_revoked);
    
    // Verify consent is revoked
    let has_consent_after_revoke = fixture.privacy_manager.has_consent(user_id, DataCategory::PersonalData).await.unwrap();
    assert!(!has_consent_after_revoke);
    
    // Test consent history
    let consent_history = fixture.privacy_manager.get_consent_history(user_id).await.unwrap();
    assert!(!consent_history.is_empty());
    
    fixture.stop().await.unwrap();
}

#[tokio::test]
async fn test_data_encryption_and_decryption() {
    let fixture = SecurityTestFixture::new().await.unwrap();
    fixture.start().await.unwrap();
    
    let test_data = json!({
        "sensitive_info": "This is sensitive user data",
        "user_id": "test_user_encryption",
        "timestamp": Utc::now().to_rfc3339(),
        "personal_details": {
            "name": "John Doe",
            "email": "john@example.com"
        }
    });
    
    // Test encryption
    let encrypted_data = fixture.encryption_manager.encrypt_data(test_data.clone()).await.unwrap();
    assert_ne!(encrypted_data.as_str(), test_data.to_string());
    
    // Test decryption
    let decrypted_data = fixture.encryption_manager.decrypt_data(&encrypted_data).await.unwrap();
    assert_eq!(decrypted_data, test_data);
    
    // Test encryption with different key (should fail decryption)
    let mut wrong_encryption_manager = fixture.encryption_manager.clone();
    wrong_encryption_manager.update_encryption_key("wrong_key".to_string()).await.unwrap();
    
    let decrypt_with_wrong_key = wrong_encryption_manager.decrypt_data(&encrypted_data).await;
    assert!(decrypt_with_wrong_key.is_err());
    
    fixture.stop().await.unwrap();
}

#[tokio::test]
async fn test_audit_logging_and_tracking() {
    let fixture = SecurityTestFixture::new().await.unwrap();
    fixture.start().await.unwrap();
    
    let user_id = "test_user_audit";
    
    // Test audit event logging
    let events = vec![
        AuditEvent {
            id: Uuid::new_v4().to_string(),
            user_id: user_id.to_string(),
            event_type: "user_login".to_string(),
            description: "User logged in to the system".to_string(),
            level: AuditLevel::Info,
            timestamp: Utc::now(),
            metadata: json!({
                "ip_address": "127.0.0.1",
                "user_agent": "Test Agent"
            }),
        },
        AuditEvent {
            id: Uuid::new_v4().to_string(),
            user_id: user_id.to_string(),
            event_type: "data_access".to_string(),
            description: "User accessed sensitive data".to_string(),
            level: AuditLevel::Warning,
            timestamp: Utc::now(),
            metadata: json!({
                "resource": "user_profile",
                "action": "read"
            }),
        },
        AuditEvent {
            id: Uuid::new_v4().to_string(),
            user_id: user_id.to_string(),
            event_type: "security_violation".to_string(),
            description: "Attempted unauthorized access".to_string(),
            level: AuditLevel::Critical,
            timestamp: Utc::now(),
            metadata: json!({
                "attempted_resource": "admin_panel",
                "blocked": true
            }),
        },
    ];
    
    // Log audit events
    for event in &events {
        fixture.audit_manager.log_event(event.clone()).await.unwrap();
    }
    
    // Test audit query by user
    let user_events = fixture.audit_manager.get_events_by_user(user_id).await.unwrap();
    assert_eq!(user_events.len(), 3);
    
    // Test audit query by level
    let critical_events = fixture.audit_manager.get_events_by_level(AuditLevel::Critical).await.unwrap();
    assert!(critical_events.len() >= 1);
    
    // Test audit query by event type
    let login_events = fixture.audit_manager.get_events_by_type("user_login").await.unwrap();
    assert!(login_events.len() >= 1);
    
    fixture.stop().await.unwrap();
}

#[tokio::test]
async fn test_data_anonymization() {
    let fixture = SecurityTestFixture::new().await.unwrap();
    fixture.start().await.unwrap();
    
    let sensitive_data = json!({
        "name": "John Doe",
        "email": "john.doe@example.com",
        "phone": "+1-555-123-4567",
        "ssn": "123-45-6789",
        "address": {
            "street": "123 Main St",
            "city": "Anytown",
            "state": "CA",
            "zip": "12345"
        },
        "preferences": {
            "theme": "dark",
            "language": "en"
        }
    });
    
    // Test data anonymization
    let anonymized_data = fixture.anonymization_manager.anonymize_data(sensitive_data.clone()).await.unwrap();
    
    // Verify sensitive data is anonymized
    assert_ne!(anonymized_data["name"], sensitive_data["name"]);
    assert_ne!(anonymized_data["email"], sensitive_data["email"]);
    assert_ne!(anonymized_data["phone"], sensitive_data["phone"]);
    assert_ne!(anonymized_data["ssn"], sensitive_data["ssn"]);
    
    // Verify non-sensitive data is preserved
    assert_eq!(anonymized_data["preferences"]["theme"], sensitive_data["preferences"]["theme"]);
    assert_eq!(anonymized_data["preferences"]["language"], sensitive_data["preferences"]["language"]);
    
    // Test consistent anonymization (same input should produce same output)
    let anonymized_again = fixture.anonymization_manager.anonymize_data(sensitive_data.clone()).await.unwrap();
    assert_eq!(anonymized_data, anonymized_again);
    
    fixture.stop().await.unwrap();
}

#[tokio::test]
async fn test_secure_memory_management() {
    let fixture = SecurityTestFixture::new().await.unwrap();
    fixture.start().await.unwrap();
    
    let memory_id = "secure_memory_test";
    let sensitive_memory = json!({
        "type": "personal_data",
        "content": "Sensitive personal information",
        "user_id": "test_user_memory",
        "classification": "confidential",
        "metadata": {
            "created_at": Utc::now().to_rfc3339(),
            "source": "user_input"
        }
    });
    
    // Store encrypted memory
    fixture.memory_manager.store_memory(memory_id, sensitive_memory.clone()).await.unwrap();
    
    // Retrieve and verify memory
    let retrieved_memory = fixture.memory_manager.get_memory(memory_id).await.unwrap();
    assert!(retrieved_memory.is_some());
    assert_eq!(retrieved_memory.unwrap(), sensitive_memory);
    
    // Test memory search with encryption
    let search_results = fixture.memory_manager.search_memories("personal_data").await.unwrap();
    assert!(!search_results.is_empty());
    
    // Test secure deletion
    fixture.memory_manager.delete_memory(memory_id).await.unwrap();
    
    // Verify memory is completely deleted
    let deleted_memory = fixture.memory_manager.get_memory(memory_id).await.unwrap();
    assert!(deleted_memory.is_none());
    
    fixture.stop().await.unwrap();
}

#[tokio::test]
async fn test_ai_request_with_privacy_controls() {
    let fixture = SecurityTestFixture::new().await.unwrap();
    fixture.start().await.unwrap();
    
    let user_id = "test_user_ai_privacy";
    
    // Grant consent for AI processing
    fixture.privacy_manager.grant_consent(
        user_id,
        DataCategory::AIProcessing,
        ConsentLevel::Explicit,
        Some("Consent for AI processing test".to_string())
    ).await.unwrap();
    
    // Test AI request with privacy controls
    let request = json!({
        "user_id": user_id,
        "model": "qwen2.5:1.5b",
        "prompt": "Analyze this text: Hello world, this is a test",
        "max_tokens": 100,
        "privacy_mode": true
    });
    
    let response = fixture.ai_manager.process_request(request).await.unwrap();
    assert!(response.is_object());
    
    // Verify audit event was logged
    let audit_events = fixture.audit_manager.get_events_by_user(user_id).await.unwrap();
    assert!(!audit_events.is_empty());
    
    // Test AI request without consent (should fail)
    fixture.privacy_manager.revoke_consent(user_id, DataCategory::AIProcessing).await.unwrap();
    
    let request_without_consent = json!({
        "user_id": user_id,
        "model": "qwen2.5:1.5b",
        "prompt": "This should fail due to lack of consent",
        "max_tokens": 100,
        "privacy_mode": true
    });
    
    let response_without_consent = fixture.ai_manager.process_request(request_without_consent).await;
    assert!(response_without_consent.is_err());
    
    fixture.stop().await.unwrap();
}

#[tokio::test]
async fn test_data_retention_and_deletion() {
    let fixture = SecurityTestFixture::new().await.unwrap();
    fixture.start().await.unwrap();
    
    let user_id = "test_user_retention";
    
    // Store data with retention policy
    let data_id = "retention_test_data";
    let test_data = json!({
        "user_id": user_id,
        "data": "Test data for retention policy",
        "created_at": Utc::now().to_rfc3339(),
        "retention_days": 1
    });
    
    fixture.memory_manager.store_memory(data_id, test_data.clone()).await.unwrap();
    
    // Verify data exists
    let stored_data = fixture.memory_manager.get_memory(data_id).await.unwrap();
    assert!(stored_data.is_some());
    
    // Test right to be forgotten
    fixture.privacy_manager.exercise_right_to_be_forgotten(user_id).await.unwrap();
    
    // Verify data is deleted
    let deleted_data = fixture.memory_manager.get_memory(data_id).await.unwrap();
    assert!(deleted_data.is_none());
    
    // Verify audit event for deletion
    let audit_events = fixture.audit_manager.get_events_by_user(user_id).await.unwrap();
    let deletion_events: Vec<_> = audit_events.iter()
        .filter(|e| e.event_type == "data_deletion")
        .collect();
    assert!(!deletion_events.is_empty());
    
    fixture.stop().await.unwrap();
}

#[tokio::test]
async fn test_configuration_security() {
    let fixture = SecurityTestFixture::new().await.unwrap();
    fixture.start().await.unwrap();
    
    // Test secure configuration storage
    let sensitive_config = json!({
        "api_key": "secret_api_key_12345",
        "database_password": "super_secret_password",
        "encryption_key": "encryption_key_67890"
    });
    
    fixture.config_manager.set_config(
        ConfigLayer::System,
        "sensitive_config",
        ConfigValue::Object(sensitive_config.as_object().unwrap().clone())
    ).await.unwrap();
    
    // Test config retrieval
    let retrieved_config = fixture.config_manager.get_config(ConfigLayer::System, "sensitive_config").await.unwrap();
    assert!(retrieved_config.is_some());
    
    // Test config validation with security rules
    let invalid_config = json!({
        "api_key": "weak_key", // Too short
        "database_password": "123", // Too weak
        "encryption_key": "" // Empty
    });
    
    let validation_result = fixture.config_manager.validate_config(&invalid_config).await;
    assert!(validation_result.is_err());
    
    fixture.stop().await.unwrap();
}

#[tokio::test]
async fn test_security_incident_response() {
    let fixture = SecurityTestFixture::new().await.unwrap();
    fixture.start().await.unwrap();
    
    let user_id = "test_user_incident";
    
    // Simulate security incident
    let security_incident = AuditEvent {
        id: Uuid::new_v4().to_string(),
        user_id: user_id.to_string(),
        event_type: "security_incident".to_string(),
        description: "Multiple failed login attempts detected".to_string(),
        level: AuditLevel::Critical,
        timestamp: Utc::now(),
        metadata: json!({
            "incident_type": "brute_force_attack",
            "failed_attempts": 10,
            "ip_address": "192.168.1.100",
            "blocked": true
        }),
    };
    
    // Log security incident
    fixture.audit_manager.log_event(security_incident.clone()).await.unwrap();
    
    // Trigger incident response
    fixture.privacy_manager.handle_security_incident(&security_incident).await.unwrap();
    
    // Verify incident response actions
    let incident_response_events = fixture.audit_manager.get_events_by_type("incident_response").await.unwrap();
    assert!(!incident_response_events.is_empty());
    
    // Test automated blocking
    let user_blocked = fixture.privacy_manager.is_user_blocked(user_id).await.unwrap();
    assert!(user_blocked);
    
    fixture.stop().await.unwrap();
}

#[tokio::test]
async fn test_comprehensive_security_workflow() {
    let fixture = SecurityTestFixture::new().await.unwrap();
    fixture.start().await.unwrap();
    
    let user_id = "test_user_comprehensive";
    
    // Step 1: Grant consent
    fixture.privacy_manager.grant_consent(
        user_id,
        DataCategory::PersonalData,
        ConsentLevel::Explicit,
        Some("Comprehensive security test consent".to_string())
    ).await.unwrap();
    
    // Step 2: Process sensitive data with encryption
    let sensitive_data = json!({
        "user_id": user_id,
        "personal_info": "John Doe, john@example.com, 555-123-4567",
        "preferences": {"theme": "dark", "language": "en"}
    });
    
    let encrypted_data = fixture.encryption_manager.encrypt_data(sensitive_data.clone()).await.unwrap();
    let memory_id = "comprehensive_test_memory";
    fixture.memory_manager.store_memory(memory_id, serde_json::from_str(&encrypted_data).unwrap()).await.unwrap();
    
    // Step 3: Anonymize data for analytics
    let anonymized_data = fixture.anonymization_manager.anonymize_data(sensitive_data.clone()).await.unwrap();
    
    // Step 4: Process AI request with privacy controls
    let ai_request = json!({
        "user_id": user_id,
        "model": "qwen2.5:1.5b",
        "prompt": "Analyze user preferences",
        "max_tokens": 100,
        "privacy_mode": true
    });
    
    let ai_response = fixture.ai_manager.process_request(ai_request).await.unwrap();
    assert!(ai_response.is_object());
    
    // Step 5: Verify audit trail
    let audit_events = fixture.audit_manager.get_events_by_user(user_id).await.unwrap();
    assert!(audit_events.len() >= 3); // Consent, data access, AI processing
    
    // Step 6: Exercise right to be forgotten
    fixture.privacy_manager.exercise_right_to_be_forgotten(user_id).await.unwrap();
    
    // Step 7: Verify data deletion
    let deleted_memory = fixture.memory_manager.get_memory(memory_id).await.unwrap();
    assert!(deleted_memory.is_none());
    
    // Step 8: Verify audit trail for deletion
    let final_audit_events = fixture.audit_manager.get_events_by_user(user_id).await.unwrap();
    let deletion_events: Vec<_> = final_audit_events.iter()
        .filter(|e| e.event_type == "data_deletion")
        .collect();
    assert!(!deletion_events.is_empty());
    
    fixture.stop().await.unwrap();
}