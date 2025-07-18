//! Privacy controls and data protection for AI integration
//! 
//! This module provides comprehensive privacy controls, data protection,
//! and user consent management for all AI operations.

pub mod consent;
pub mod encryption;
pub mod audit;
pub mod anonymization;

use crate::AIError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use chrono::{DateTime, Utc};
use log::{info, warn, error, debug};

/// Privacy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyConfig {
    /// Master privacy switch
    pub enabled: bool,
    /// Process all data locally
    pub local_only: bool,
    /// Enable telemetry collection
    pub telemetry_enabled: bool,
    /// Data retention settings
    pub retention: DataRetentionConfig,
    /// Encryption settings
    pub encryption: EncryptionConfig,
    /// Consent management settings
    pub consent: ConsentConfig,
    /// Audit logging settings
    pub audit: AuditConfig,
    /// Anonymization settings
    pub anonymization: AnonymizationConfig,
    /// Sensitive data detection
    pub sensitive_data: SensitiveDataConfig,
}

impl Default for PrivacyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            local_only: true,
            telemetry_enabled: false,
            retention: DataRetentionConfig::default(),
            encryption: EncryptionConfig::default(),
            consent: ConsentConfig::default(),
            audit: AuditConfig::default(),
            anonymization: AnonymizationConfig::default(),
            sensitive_data: SensitiveDataConfig::default(),
        }
    }
}

/// Data retention configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataRetentionConfig {
    /// Retention policy
    pub policy: RetentionPolicy,
    /// Detailed retention period (days)
    pub detailed_days: u32,
    /// Aggregated retention period (days)
    pub aggregated_days: u32,
    /// Auto-delete on expiry
    pub auto_delete: bool,
    /// Data categories with custom retention
    pub custom_retention: HashMap<String, u32>,
}

impl Default for DataRetentionConfig {
    fn default() -> Self {
        Self {
            policy: RetentionPolicy::TimeBased,
            detailed_days: 30,
            aggregated_days: 365,
            auto_delete: true,
            custom_retention: HashMap::new(),
        }
    }
}

/// Retention policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RetentionPolicy {
    /// Keep data for session only
    SessionOnly,
    /// Time-based retention
    TimeBased,
    /// Event-based retention
    EventBased,
    /// Keep forever (not recommended)
    Forever,
}

/// Encryption configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    /// Enable encryption at rest
    pub at_rest: bool,
    /// Enable encryption in transit
    pub in_transit: bool,
    /// Encryption algorithm
    pub algorithm: EncryptionAlgorithm,
    /// Key management
    pub key_management: KeyManagement,
    /// Encrypt logs
    pub encrypt_logs: bool,
    /// Encrypt temporary files
    pub encrypt_temp: bool,
}

impl Default for EncryptionConfig {
    fn default() -> Self {
        Self {
            at_rest: true,
            in_transit: true,
            algorithm: EncryptionAlgorithm::AES256GCM,
            key_management: KeyManagement::SystemKeyring,
            encrypt_logs: false,
            encrypt_temp: true,
        }
    }
}

/// Encryption algorithm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EncryptionAlgorithm {
    /// AES-256 with GCM
    AES256GCM,
    /// ChaCha20-Poly1305
    ChaCha20Poly1305,
    /// XChaCha20-Poly1305
    XChaCha20Poly1305,
}

/// Key management strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeyManagement {
    /// System keyring (recommended)
    SystemKeyring,
    /// Hardware security module
    HSM,
    /// Encrypted file storage
    FileStorage,
    /// Memory only (session keys)
    MemoryOnly,
}

/// Consent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsentConfig {
    /// Require explicit consent
    pub require_explicit_consent: bool,
    /// Granular consent options
    pub granular_consent: bool,
    /// Consent renewal period (days)
    pub renewal_period: Option<u32>,
    /// Allow withdrawal anytime
    pub allow_withdrawal: bool,
    /// Default consent state
    pub default_state: ConsentState,
}

impl Default for ConsentConfig {
    fn default() -> Self {
        Self {
            require_explicit_consent: true,
            granular_consent: true,
            renewal_period: Some(365),
            allow_withdrawal: true,
            default_state: ConsentState::NotGiven,
        }
    }
}

/// Consent state
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ConsentState {
    /// Consent not given
    NotGiven,
    /// Consent given
    Given,
    /// Consent withdrawn
    Withdrawn,
    /// Consent expired
    Expired,
}

/// Audit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    /// Enable audit logging
    pub enabled: bool,
    /// Audit log level
    pub level: AuditLevel,
    /// Log retention (days)
    pub retention_days: u32,
    /// Include user actions
    pub log_user_actions: bool,
    /// Include system actions
    pub log_system_actions: bool,
    /// Include data access
    pub log_data_access: bool,
    /// Audit log encryption
    pub encrypt_logs: bool,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            level: AuditLevel::Standard,
            retention_days: 90,
            log_user_actions: true,
            log_system_actions: true,
            log_data_access: true,
            encrypt_logs: true,
        }
    }
}

/// Audit log level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditLevel {
    /// Minimal logging
    Minimal,
    /// Standard logging
    Standard,
    /// Detailed logging
    Detailed,
    /// Forensic level logging
    Forensic,
}

/// Anonymization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnonymizationConfig {
    /// Enable anonymization
    pub enabled: bool,
    /// Anonymization technique
    pub technique: AnonymizationTechnique,
    /// Preserve data utility
    pub preserve_utility: bool,
    /// Reversible anonymization
    pub reversible: bool,
    /// Custom anonymization rules
    pub custom_rules: HashMap<String, String>,
}

impl Default for AnonymizationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            technique: AnonymizationTechnique::Pseudonymization,
            preserve_utility: true,
            reversible: false,
            custom_rules: HashMap::new(),
        }
    }
}

/// Anonymization technique
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnonymizationTechnique {
    /// Replace with pseudonyms
    Pseudonymization,
    /// Generalization
    Generalization,
    /// Suppression
    Suppression,
    /// Noise addition
    NoiseAddition,
    /// K-anonymity
    KAnonymity(u32),
}

/// Sensitive data configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitiveDataConfig {
    /// Enable sensitive data detection
    pub detection_enabled: bool,
    /// PII detection patterns
    pub pii_patterns: Vec<PIIPattern>,
    /// Custom sensitive patterns
    pub custom_patterns: Vec<String>,
    /// Action on detection
    pub detection_action: DetectionAction,
    /// Exclusion zones
    pub exclusion_zones: Vec<String>,
}

impl Default for SensitiveDataConfig {
    fn default() -> Self {
        Self {
            detection_enabled: true,
            pii_patterns: vec![
                PIIPattern::Email,
                PIIPattern::Phone,
                PIIPattern::SSN,
                PIIPattern::CreditCard,
                PIIPattern::Password,
            ],
            custom_patterns: Vec::new(),
            detection_action: DetectionAction::Redact,
            exclusion_zones: vec![
                "~/private".to_string(),
                "~/.ssh".to_string(),
                "~/.gnupg".to_string(),
            ],
        }
    }
}

/// PII pattern types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PIIPattern {
    /// Email addresses
    Email,
    /// Phone numbers
    Phone,
    /// Social security numbers
    SSN,
    /// Credit card numbers
    CreditCard,
    /// Passwords
    Password,
    /// IP addresses
    IPAddress,
    /// Physical addresses
    Address,
    /// Names
    Name,
    /// Custom pattern
    Custom(String),
}

/// Action on sensitive data detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DetectionAction {
    /// Redact the data
    Redact,
    /// Skip processing
    Skip,
    /// Alert user
    Alert,
    /// Block operation
    Block,
    /// Log only
    LogOnly,
}

/// Privacy manager
pub struct PrivacyManager {
    /// Configuration
    config: Arc<RwLock<PrivacyConfig>>,
    /// Consent manager
    consent_manager: Arc<consent::ConsentManager>,
    /// Encryption manager
    encryption_manager: Arc<encryption::EncryptionManager>,
    /// Audit logger
    audit_logger: Arc<audit::AuditLogger>,
    /// Anonymization engine
    anonymization_engine: Arc<anonymization::AnonymizationEngine>,
    /// Privacy statistics
    stats: Arc<RwLock<PrivacyStats>>,
}

/// Privacy statistics
#[derive(Debug, Default)]
pub struct PrivacyStats {
    /// Total privacy checks
    total_checks: u64,
    /// Sensitive data detected
    sensitive_data_detected: u64,
    /// Data anonymized
    data_anonymized: u64,
    /// Consent requests
    consent_requests: u64,
    /// Consent denials
    consent_denials: u64,
    /// Audit events logged
    audit_events: u64,
    /// Last privacy check
    last_check: Option<DateTime<Utc>>,
}

impl PrivacyManager {
    /// Create a new privacy manager
    pub async fn new(config: PrivacyConfig) -> Result<Self, AIError> {
        let consent_manager = Arc::new(consent::ConsentManager::new(config.consent.clone()).await?);
        let encryption_manager = Arc::new(encryption::EncryptionManager::new(config.encryption.clone()).await?);
        let audit_logger = Arc::new(audit::AuditLogger::new(config.audit.clone()).await?);
        let anonymization_engine = Arc::new(anonymization::AnonymizationEngine::new(config.anonymization.clone()).await?);
        
        let manager = Self {
            config: Arc::new(RwLock::new(config)),
            consent_manager,
            encryption_manager,
            audit_logger,
            anonymization_engine,
            stats: Arc::new(RwLock::new(PrivacyStats::default())),
        };
        
        info!("Privacy manager initialized");
        Ok(manager)
    }
    
    /// Check if operation is allowed
    pub async fn check_operation(&self, operation: &PrivacyOperation) -> Result<bool, AIError> {
        if !self.config.read().enabled {
            return Ok(true); // Privacy checks disabled
        }
        
        let mut stats = self.stats.write();
        stats.total_checks += 1;
        stats.last_check = Some(Utc::now());
        drop(stats);
        
        // Check consent
        if !self.consent_manager.has_consent(&operation.category).await? {
            self.stats.write().consent_denials += 1;
            return Ok(false);
        }
        
        // Check data sensitivity
        if self.contains_sensitive_data(&operation.data).await? {
            self.stats.write().sensitive_data_detected += 1;
            
            match self.config.read().sensitive_data.detection_action {
                DetectionAction::Block => return Ok(false),
                DetectionAction::Alert => self.alert_user(operation).await?,
                _ => {}
            }
        }
        
        // Log audit event
        self.audit_logger.log_operation(operation).await?;
        self.stats.write().audit_events += 1;
        
        Ok(true)
    }
    
    /// Anonymize data
    pub async fn anonymize_data(&self, data: &str) -> Result<String, AIError> {
        if !self.config.read().anonymization.enabled {
            return Ok(data.to_string());
        }
        
        let result = self.anonymization_engine.anonymize(data).await?;
        self.stats.write().data_anonymized += 1;
        
        Ok(result)
    }
    
    /// Encrypt data
    pub async fn encrypt_data(&self, data: &[u8]) -> Result<Vec<u8>, AIError> {
        if !self.config.read().encryption.at_rest {
            return Ok(data.to_vec());
        }
        
        self.encryption_manager.encrypt(data).await
    }
    
    /// Decrypt data
    pub async fn decrypt_data(&self, data: &[u8]) -> Result<Vec<u8>, AIError> {
        self.encryption_manager.decrypt(data).await
    }
    
    /// Request user consent
    pub async fn request_consent(&self, category: &str, purpose: &str) -> Result<bool, AIError> {
        self.stats.write().consent_requests += 1;
        self.consent_manager.request_consent(category, purpose).await
    }
    
    /// Withdraw consent
    pub async fn withdraw_consent(&self, category: &str) -> Result<(), AIError> {
        self.consent_manager.withdraw_consent(category).await
    }
    
    /// Get privacy report
    pub async fn get_privacy_report(&self) -> Result<PrivacyReport, AIError> {
        let stats = self.stats.read().clone();
        let consent_status = self.consent_manager.get_consent_status().await?;
        let retention_status = self.get_retention_status().await?;
        
        Ok(PrivacyReport {
            stats: PrivacyReportStats {
                total_checks: stats.total_checks,
                sensitive_data_detected: stats.sensitive_data_detected,
                data_anonymized: stats.data_anonymized,
                consent_requests: stats.consent_requests,
                consent_denials: stats.consent_denials,
                audit_events: stats.audit_events,
            },
            consent_status,
            retention_status,
            encryption_enabled: self.config.read().encryption.at_rest,
            local_only: self.config.read().local_only,
            last_check: stats.last_check,
        })
    }
    
    /// Export user data (GDPR compliance)
    pub async fn export_user_data(&self) -> Result<UserDataExport, AIError> {
        // Collect all user data
        let patterns = self.get_user_patterns().await?;
        let suggestions = self.get_user_suggestions().await?;
        let automations = self.get_user_automations().await?;
        let settings = self.get_user_settings().await?;
        
        Ok(UserDataExport {
            export_date: Utc::now(),
            patterns,
            suggestions,
            automations,
            settings,
            metadata: HashMap::new(),
        })
    }
    
    /// Delete all user data (Right to erasure)
    pub async fn delete_all_user_data(&self) -> Result<(), AIError> {
        // Delete from all systems
        self.delete_patterns().await?;
        self.delete_suggestions().await?;
        self.delete_automations().await?;
        self.delete_settings().await?;
        
        // Clear caches
        self.clear_caches().await?;
        
        // Log the deletion
        self.audit_logger.log_data_deletion().await?;
        
        info!("All user data deleted");
        Ok(())
    }
    
    /// Update configuration
    pub async fn update_config(&self, new_config: PrivacyConfig) -> Result<(), AIError> {
        *self.config.write() = new_config.clone();
        
        // Update sub-components
        self.consent_manager.update_config(new_config.consent).await?;
        self.encryption_manager.update_config(new_config.encryption).await?;
        self.audit_logger.update_config(new_config.audit).await?;
        self.anonymization_engine.update_config(new_config.anonymization).await?;
        
        info!("Privacy configuration updated");
        Ok(())
    }
    
    /// Check if data contains sensitive information
    async fn contains_sensitive_data(&self, data: &str) -> Result<bool, AIError> {
        let config = self.config.read();
        
        if !config.sensitive_data.detection_enabled {
            return Ok(false);
        }
        
        // Check PII patterns
        for pattern in &config.sensitive_data.pii_patterns {
            if self.matches_pii_pattern(data, pattern).await? {
                return Ok(true);
            }
        }
        
        // Check custom patterns
        for pattern in &config.sensitive_data.custom_patterns {
            if data.contains(pattern) {
                return Ok(true);
            }
        }
        
        Ok(false)
    }
    
    /// Check if data matches PII pattern
    async fn matches_pii_pattern(&self, data: &str, pattern: &PIIPattern) -> Result<bool, AIError> {
        match pattern {
            PIIPattern::Email => Ok(data.contains('@') && data.contains('.')),
            PIIPattern::Phone => Ok(data.chars().filter(|c| c.is_ascii_digit()).count() >= 10),
            PIIPattern::SSN => Ok(data.chars().filter(|c| c.is_ascii_digit()).count() == 9),
            PIIPattern::CreditCard => Ok(data.chars().filter(|c| c.is_ascii_digit()).count() >= 13),
            PIIPattern::Password => Ok(data.contains("password") || data.contains("passwd")),
            PIIPattern::IPAddress => Ok(data.split('.').count() == 4),
            PIIPattern::Address => Ok(data.contains("street") || data.contains("avenue")),
            PIIPattern::Name => Ok(false), // Complex detection needed
            PIIPattern::Custom(regex) => Ok(data.contains(regex)),
        }
    }
    
    /// Alert user about sensitive data
    async fn alert_user(&self, operation: &PrivacyOperation) -> Result<(), AIError> {
        // TODO: Implement user notification
        log::warn!("Sensitive data detected in operation: {:?}", operation.category);
        Ok(())
    }
    
    /// Get retention status
    async fn get_retention_status(&self) -> Result<RetentionStatus, AIError> {
        // TODO: Implement retention status check
        Ok(RetentionStatus {
            policy: self.config.read().retention.policy.clone(),
            data_age_days: HashMap::new(),
            next_deletion: None,
        })
    }
    
    /// Get user patterns
    async fn get_user_patterns(&self) -> Result<Vec<serde_json::Value>, AIError> {
        // TODO: Implement pattern retrieval
        Ok(Vec::new())
    }
    
    /// Get user suggestions
    async fn get_user_suggestions(&self) -> Result<Vec<serde_json::Value>, AIError> {
        // TODO: Implement suggestion retrieval
        Ok(Vec::new())
    }
    
    /// Get user automations
    async fn get_user_automations(&self) -> Result<Vec<serde_json::Value>, AIError> {
        // TODO: Implement automation retrieval
        Ok(Vec::new())
    }
    
    /// Get user settings
    async fn get_user_settings(&self) -> Result<serde_json::Value, AIError> {
        // TODO: Implement settings retrieval
        Ok(serde_json::json!({}))
    }
    
    /// Delete patterns
    async fn delete_patterns(&self) -> Result<(), AIError> {
        // TODO: Implement pattern deletion
        Ok(())
    }
    
    /// Delete suggestions
    async fn delete_suggestions(&self) -> Result<(), AIError> {
        // TODO: Implement suggestion deletion
        Ok(())
    }
    
    /// Delete automations
    async fn delete_automations(&self) -> Result<(), AIError> {
        // TODO: Implement automation deletion
        Ok(())
    }
    
    /// Delete settings
    async fn delete_settings(&self) -> Result<(), AIError> {
        // TODO: Implement settings deletion
        Ok(())
    }
    
    /// Clear caches
    async fn clear_caches(&self) -> Result<(), AIError> {
        // TODO: Implement cache clearing
        Ok(())
    }
}

/// Privacy operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyOperation {
    /// Operation ID
    pub id: String,
    /// Operation category
    pub category: String,
    /// Operation type
    pub operation_type: OperationType,
    /// Data involved
    pub data: String,
    /// User ID
    pub user_id: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Operation type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationType {
    /// Data collection
    Collection,
    /// Data processing
    Processing,
    /// Data storage
    Storage,
    /// Data sharing
    Sharing,
    /// Data deletion
    Deletion,
    /// Data export
    Export,
}

/// Privacy report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyReport {
    /// Statistics
    pub stats: PrivacyReportStats,
    /// Consent status
    pub consent_status: HashMap<String, ConsentState>,
    /// Retention status
    pub retention_status: RetentionStatus,
    /// Encryption enabled
    pub encryption_enabled: bool,
    /// Local only mode
    pub local_only: bool,
    /// Last check time
    pub last_check: Option<DateTime<Utc>>,
}

/// Privacy report statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyReportStats {
    /// Total privacy checks
    pub total_checks: u64,
    /// Sensitive data detected
    pub sensitive_data_detected: u64,
    /// Data anonymized
    pub data_anonymized: u64,
    /// Consent requests
    pub consent_requests: u64,
    /// Consent denials
    pub consent_denials: u64,
    /// Audit events logged
    pub audit_events: u64,
}

/// Retention status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionStatus {
    /// Current policy
    pub policy: RetentionPolicy,
    /// Data age by category
    pub data_age_days: HashMap<String, u32>,
    /// Next scheduled deletion
    pub next_deletion: Option<DateTime<Utc>>,
}

/// User data export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserDataExport {
    /// Export date
    pub export_date: DateTime<Utc>,
    /// User patterns
    pub patterns: Vec<serde_json::Value>,
    /// User suggestions
    pub suggestions: Vec<serde_json::Value>,
    /// User automations
    pub automations: Vec<serde_json::Value>,
    /// User settings
    pub settings: serde_json::Value,
    /// Export metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Clone for PrivacyStats {
    fn clone(&self) -> Self {
        Self {
            total_checks: self.total_checks,
            sensitive_data_detected: self.sensitive_data_detected,
            data_anonymized: self.data_anonymized,
            consent_requests: self.consent_requests,
            consent_denials: self.consent_denials,
            audit_events: self.audit_events,
            last_check: self.last_check,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_privacy_config_default() {
        let config = PrivacyConfig::default();
        assert!(config.enabled);
        assert!(config.local_only);
        assert!(!config.telemetry_enabled);
        assert!(matches!(config.retention.policy, RetentionPolicy::TimeBased));
        assert_eq!(config.retention.detailed_days, 30);
        assert_eq!(config.retention.aggregated_days, 365);
    }
    
    #[test]
    fn test_encryption_config_default() {
        let config = EncryptionConfig::default();
        assert!(config.at_rest);
        assert!(config.in_transit);
        assert!(matches!(config.algorithm, EncryptionAlgorithm::AES256GCM));
        assert!(matches!(config.key_management, KeyManagement::SystemKeyring));
    }
    
    #[test]
    fn test_consent_config_default() {
        let config = ConsentConfig::default();
        assert!(config.require_explicit_consent);
        assert!(config.granular_consent);
        assert_eq!(config.renewal_period, Some(365));
        assert!(config.allow_withdrawal);
        assert!(matches!(config.default_state, ConsentState::NotGiven));
    }
    
    #[test]
    fn test_privacy_operation() {
        let operation = PrivacyOperation {
            id: "op-123".to_string(),
            category: "data-collection".to_string(),
            operation_type: OperationType::Collection,
            data: "user interaction data".to_string(),
            user_id: "user-456".to_string(),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };
        
        assert_eq!(operation.id, "op-123");
        assert_eq!(operation.category, "data-collection");
        assert!(matches!(operation.operation_type, OperationType::Collection));
    }
}