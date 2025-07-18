//! Audit logging for privacy and compliance
//! 
//! This module provides comprehensive audit logging for all AI operations,
//! data access, and privacy-related events with tamper protection.

use crate::AIError;
use crate::privacy::{AuditConfig, AuditLevel, PrivacyOperation, OperationType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use chrono::{DateTime, Utc, Duration};
use tokio::fs::{self, OpenOptions};
use tokio::io::AsyncWriteExt;
use log::{info, warn, error, debug};
use sha2::{Sha256, Digest};

/// Audit event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Event ID
    pub id: String,
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
    /// Event type
    pub event_type: AuditEventType,
    /// Event category
    pub category: String,
    /// Actor (user or system)
    pub actor: Actor,
    /// Resource accessed
    pub resource: Option<String>,
    /// Operation performed
    pub operation: Option<String>,
    /// Result of operation
    pub result: AuditResult,
    /// Event severity
    pub severity: AuditSeverity,
    /// Event details
    pub details: HashMap<String, serde_json::Value>,
    /// Event hash (for integrity)
    pub hash: Option<String>,
    /// Previous event hash (for chain integrity)
    pub previous_hash: Option<String>,
}

/// Audit event type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditEventType {
    /// Data access event
    DataAccess,
    /// Data modification event
    DataModification,
    /// Data deletion event
    DataDeletion,
    /// Consent change event
    ConsentChange,
    /// Privacy operation event
    PrivacyOperation,
    /// System configuration change
    ConfigurationChange,
    /// Authentication event
    Authentication,
    /// Authorization event
    Authorization,
    /// Error event
    Error,
    /// Security event
    Security,
}

/// Actor information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Actor {
    /// Actor ID
    pub id: String,
    /// Actor type
    pub actor_type: ActorType,
    /// Actor name
    pub name: String,
    /// Actor IP address
    pub ip_address: Option<String>,
    /// Actor session ID
    pub session_id: Option<String>,
}

/// Actor type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActorType {
    /// Human user
    User,
    /// System component
    System,
    /// AI agent
    Agent,
    /// External service
    Service,
}

/// Audit result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditResult {
    /// Operation succeeded
    Success,
    /// Operation failed
    Failure,
    /// Operation denied
    Denied,
    /// Operation pending
    Pending,
}

/// Audit severity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum AuditSeverity {
    /// Informational
    Info,
    /// Warning
    Warning,
    /// Error
    Error,
    /// Critical
    Critical,
}

/// Audit query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditQuery {
    /// Time range
    pub time_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    /// Event types to include
    pub event_types: Option<Vec<AuditEventType>>,
    /// Actors to include
    pub actors: Option<Vec<String>>,
    /// Resources to include
    pub resources: Option<Vec<String>>,
    /// Minimum severity
    pub min_severity: Option<AuditSeverity>,
    /// Maximum results
    pub limit: Option<usize>,
    /// Query metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Audit report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditReport {
    /// Report ID
    pub id: String,
    /// Report generation time
    pub generated_at: DateTime<Utc>,
    /// Report period
    pub period: (DateTime<Utc>, DateTime<Utc>),
    /// Total events
    pub total_events: usize,
    /// Events by type
    pub events_by_type: HashMap<String, usize>,
    /// Events by severity
    pub events_by_severity: HashMap<String, usize>,
    /// Top actors
    pub top_actors: Vec<(String, usize)>,
    /// Top resources
    pub top_resources: Vec<(String, usize)>,
    /// Security incidents
    pub security_incidents: usize,
    /// Compliance violations
    pub compliance_violations: usize,
    /// Report metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Audit logger
pub struct AuditLogger {
    /// Configuration
    config: Arc<RwLock<AuditConfig>>,
    /// Event storage
    event_storage: Arc<RwLock<EventStorage>>,
    /// Current chain hash
    current_hash: Arc<RwLock<Option<String>>>,
    /// Logger statistics
    stats: Arc<RwLock<AuditStats>>,
}

/// Event storage backend
struct EventStorage {
    /// Storage path
    storage_path: String,
    /// In-memory buffer
    buffer: Vec<AuditEvent>,
    /// Buffer size limit
    buffer_limit: usize,
}

/// Audit statistics
#[derive(Debug, Default)]
pub struct AuditStats {
    /// Total events logged
    total_events: u64,
    /// Events by type
    events_by_type: HashMap<String, u64>,
    /// Events by severity
    events_by_severity: HashMap<String, u64>,
    /// Storage size (bytes)
    storage_size: u64,
    /// Last event time
    last_event: Option<DateTime<Utc>>,
    /// Chain verification status
    chain_verified: bool,
    /// Last verification time
    last_verification: Option<DateTime<Utc>>,
}

impl AuditLogger {
    /// Create a new audit logger
    pub async fn new(config: AuditConfig) -> Result<Self, AIError> {
        let storage_path = "/tmp/horizonos/audit/logs".to_string();
        
        // Create storage directory
        fs::create_dir_all(&storage_path).await
            .map_err(|e| AIError::Io(e))?;
        
        let event_storage = EventStorage {
            storage_path,
            buffer: Vec::new(),
            buffer_limit: 1000,
        };
        
        let logger = Self {
            config: Arc::new(RwLock::new(config)),
            event_storage: Arc::new(RwLock::new(event_storage)),
            current_hash: Arc::new(RwLock::new(None)),
            stats: Arc::new(RwLock::new(AuditStats::default())),
        };
        
        // Load existing hash chain
        logger.load_hash_chain().await?;
        
        info!("Audit logger initialized");
        Ok(logger)
    }
    
    /// Log an audit event
    pub async fn log_event(&self, event: AuditEvent) -> Result<(), AIError> {
        if !self.config.read().enabled {
            return Ok(());
        }
        
        // Check if event meets logging level
        if !self.should_log_event(&event) {
            return Ok(());
        }
        
        // Add integrity hash
        let mut event = event;
        let previous_hash = self.current_hash.read().clone();
        event.previous_hash = previous_hash;
        event.hash = Some(self.calculate_event_hash(&event));
        
        // Update current hash
        *self.current_hash.write() = event.hash.clone();
        
        // Store event
        self.store_event(event.clone()).await?;
        
        // Update statistics
        self.update_stats(&event);
        
        debug!("Audit event logged: {}", event.id);
        Ok(())
    }
    
    /// Log a privacy operation
    pub async fn log_operation(&self, operation: &PrivacyOperation) -> Result<(), AIError> {
        let event = AuditEvent {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: operation.timestamp,
            event_type: AuditEventType::PrivacyOperation,
            category: operation.category.clone(),
            actor: Actor {
                id: operation.user_id.clone(),
                actor_type: ActorType::User,
                name: operation.user_id.clone(),
                ip_address: None,
                session_id: None,
            },
            resource: Some(format!("data:{:?}", operation.operation_type)),
            operation: Some(format!("{:?}", operation.operation_type)),
            result: AuditResult::Success,
            severity: AuditSeverity::Info,
            details: operation.metadata.clone(),
            hash: None,
            previous_hash: None,
        };
        
        self.log_event(event).await
    }
    
    /// Log data access
    pub async fn log_data_access(&self, user_id: &str, resource: &str, operation: &str) -> Result<(), AIError> {
        if !self.config.read().log_data_access {
            return Ok(());
        }
        
        let event = AuditEvent {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            event_type: AuditEventType::DataAccess,
            category: "data-access".to_string(),
            actor: Actor {
                id: user_id.to_string(),
                actor_type: ActorType::User,
                name: user_id.to_string(),
                ip_address: None,
                session_id: None,
            },
            resource: Some(resource.to_string()),
            operation: Some(operation.to_string()),
            result: AuditResult::Success,
            severity: AuditSeverity::Info,
            details: HashMap::new(),
            hash: None,
            previous_hash: None,
        };
        
        self.log_event(event).await
    }
    
    /// Log data deletion
    pub async fn log_data_deletion(&self) -> Result<(), AIError> {
        let event = AuditEvent {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            event_type: AuditEventType::DataDeletion,
            category: "data-deletion".to_string(),
            actor: Actor {
                id: "system".to_string(),
                actor_type: ActorType::System,
                name: "Privacy Manager".to_string(),
                ip_address: None,
                session_id: None,
            },
            resource: Some("all-user-data".to_string()),
            operation: Some("delete-all".to_string()),
            result: AuditResult::Success,
            severity: AuditSeverity::Warning,
            details: HashMap::new(),
            hash: None,
            previous_hash: None,
        };
        
        self.log_event(event).await
    }
    
    /// Log security event
    pub async fn log_security_event(&self, event_type: &str, severity: AuditSeverity, details: HashMap<String, serde_json::Value>) -> Result<(), AIError> {
        let event = AuditEvent {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            event_type: AuditEventType::Security,
            category: "security".to_string(),
            actor: Actor {
                id: "system".to_string(),
                actor_type: ActorType::System,
                name: "Security Monitor".to_string(),
                ip_address: None,
                session_id: None,
            },
            resource: None,
            operation: Some(event_type.to_string()),
            result: AuditResult::Success,
            severity,
            details,
            hash: None,
            previous_hash: None,
        };
        
        self.log_event(event).await
    }
    
    /// Query audit events
    pub async fn query_events(&self, query: &AuditQuery) -> Result<Vec<AuditEvent>, AIError> {
        let events = self.load_all_events().await?;
        let mut results = Vec::new();
        
        for event in events {
            // Apply filters
            if let Some((start, end)) = &query.time_range {
                if event.timestamp < *start || event.timestamp > *end {
                    continue;
                }
            }
            
            if let Some(event_types) = &query.event_types {
                if !event_types.iter().any(|t| std::mem::discriminant(t) == std::mem::discriminant(&event.event_type)) {
                    continue;
                }
            }
            
            if let Some(actors) = &query.actors {
                if !actors.contains(&event.actor.id) {
                    continue;
                }
            }
            
            if let Some(resources) = &query.resources {
                if let Some(resource) = &event.resource {
                    if !resources.contains(resource) {
                        continue;
                    }
                } else {
                    continue;
                }
            }
            
            if let Some(min_severity) = &query.min_severity {
                if event.severity < *min_severity {
                    continue;
                }
            }
            
            results.push(event);
            
            if let Some(limit) = query.limit {
                if results.len() >= limit {
                    break;
                }
            }
        }
        
        Ok(results)
    }
    
    /// Generate audit report
    pub async fn generate_report(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<AuditReport, AIError> {
        let query = AuditQuery {
            time_range: Some((start, end)),
            event_types: None,
            actors: None,
            resources: None,
            min_severity: None,
            limit: None,
            metadata: HashMap::new(),
        };
        
        let events = self.query_events(&query).await?;
        
        // Analyze events
        let mut events_by_type = HashMap::new();
        let mut events_by_severity = HashMap::new();
        let mut actor_counts = HashMap::new();
        let mut resource_counts = HashMap::new();
        let mut security_incidents = 0;
        let mut compliance_violations = 0;
        
        for event in &events {
            // Count by type
            let type_key = format!("{:?}", event.event_type);
            events_by_type.entry(type_key).and_modify(|c| *c += 1).or_insert(1);
            
            // Count by severity
            let severity_key = format!("{:?}", event.severity);
            events_by_severity.entry(severity_key).and_modify(|c| *c += 1).or_insert(1);
            
            // Count actors
            actor_counts.entry(event.actor.name.clone()).and_modify(|c| *c += 1).or_insert(1);
            
            // Count resources
            if let Some(resource) = &event.resource {
                resource_counts.entry(resource.clone()).and_modify(|c| *c += 1).or_insert(1);
            }
            
            // Count incidents
            if matches!(event.event_type, AuditEventType::Security) && event.severity >= AuditSeverity::Warning {
                security_incidents += 1;
            }
            
            if matches!(event.result, AuditResult::Denied) {
                compliance_violations += 1;
            }
        }
        
        // Get top actors and resources
        let mut top_actors: Vec<_> = actor_counts.into_iter().collect();
        top_actors.sort_by(|a, b| b.1.cmp(&a.1));
        top_actors.truncate(10);
        
        let mut top_resources: Vec<_> = resource_counts.into_iter().collect();
        top_resources.sort_by(|a, b| b.1.cmp(&a.1));
        top_resources.truncate(10);
        
        Ok(AuditReport {
            id: uuid::Uuid::new_v4().to_string(),
            generated_at: Utc::now(),
            period: (start, end),
            total_events: events.len(),
            events_by_type,
            events_by_severity,
            top_actors,
            top_resources,
            security_incidents,
            compliance_violations,
            metadata: HashMap::new(),
        })
    }
    
    /// Verify audit log integrity
    pub async fn verify_integrity(&self) -> Result<bool, AIError> {
        let events = self.load_all_events().await?;
        
        if events.is_empty() {
            return Ok(true);
        }
        
        let mut previous_hash: Option<String> = None;
        
        for event in events {
            // Check hash chain
            if event.previous_hash != previous_hash {
                error!("Hash chain broken at event: {}", event.id);
                return Ok(false);
            }
            
            // Verify event hash
            let calculated_hash = self.calculate_event_hash(&event);
            if event.hash.as_ref() != Some(&calculated_hash) {
                error!("Invalid hash for event: {}", event.id);
                return Ok(false);
            }
            
            previous_hash = event.hash;
        }
        
        // Update verification status
        let mut stats = self.stats.write();
        stats.chain_verified = true;
        stats.last_verification = Some(Utc::now());
        
        info!("Audit log integrity verified");
        Ok(true)
    }
    
    /// Should log event based on configuration
    fn should_log_event(&self, event: &AuditEvent) -> bool {
        let config = self.config.read();
        
        match config.level {
            AuditLevel::Minimal => event.severity >= AuditSeverity::Warning,
            AuditLevel::Standard => event.severity >= AuditSeverity::Info,
            AuditLevel::Detailed => true,
            AuditLevel::Forensic => true,
        }
    }
    
    /// Calculate event hash
    fn calculate_event_hash(&self, event: &AuditEvent) -> String {
        let mut hasher = Sha256::new();
        
        // Hash event fields
        hasher.update(event.id.as_bytes());
        hasher.update(event.timestamp.to_rfc3339().as_bytes());
        hasher.update(format!("{:?}", event.event_type).as_bytes());
        hasher.update(event.category.as_bytes());
        hasher.update(event.actor.id.as_bytes());
        
        if let Some(resource) = &event.resource {
            hasher.update(resource.as_bytes());
        }
        
        if let Some(operation) = &event.operation {
            hasher.update(operation.as_bytes());
        }
        
        hasher.update(format!("{:?}", event.result).as_bytes());
        hasher.update(format!("{:?}", event.severity).as_bytes());
        
        if let Some(previous_hash) = &event.previous_hash {
            hasher.update(previous_hash.as_bytes());
        }
        
        format!("{:x}", hasher.finalize())
    }
    
    /// Store event
    async fn store_event(&self, event: AuditEvent) -> Result<(), AIError> {
        let mut storage = self.event_storage.write();
        storage.buffer.push(event.clone());
        
        // Flush buffer if needed
        if storage.buffer.len() >= storage.buffer_limit {
            let events = storage.buffer.drain(..).collect::<Vec<_>>();
            drop(storage);
            self.flush_events(events).await?;
        }
        
        Ok(())
    }
    
    /// Flush events to disk
    async fn flush_events(&self, events: Vec<AuditEvent>) -> Result<(), AIError> {
        let storage = self.event_storage.read();
        let date = Utc::now().format("%Y-%m-%d");
        let file_path = format!("{}/audit-{}.jsonl", storage.storage_path, date);
        drop(storage);
        
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)
            .await
            .map_err(|e| AIError::Io(e))?;
        
        for event in events {
            let json = serde_json::to_string(&event)
                .map_err(|e| AIError::Serialization(e))?;
            file.write_all(json.as_bytes()).await
                .map_err(|e| AIError::Io(e))?;
            file.write_all(b"\n").await
                .map_err(|e| AIError::Io(e))?;
        }
        
        file.flush().await
            .map_err(|e| AIError::Io(e))?;
        
        Ok(())
    }
    
    /// Load all events
    async fn load_all_events(&self) -> Result<Vec<AuditEvent>, AIError> {
        let storage = self.event_storage.read();
        let storage_path = storage.storage_path.clone();
        drop(storage);
        
        let mut all_events = Vec::new();
        
        // Read all log files
        let mut entries = fs::read_dir(&storage_path).await
            .map_err(|e| AIError::Io(e))?;
        
        while let Some(entry) = entries.next_entry().await
            .map_err(|e| AIError::Io(e))? {
            if entry.path().extension().and_then(|s| s.to_str()) == Some("jsonl") {
                let content = fs::read_to_string(entry.path()).await
                    .map_err(|e| AIError::Io(e))?;
                
                for line in content.lines() {
                    if !line.is_empty() {
                        let event: AuditEvent = serde_json::from_str(line)
                            .map_err(|e| AIError::Serialization(e))?;
                        all_events.push(event);
                    }
                }
            }
        }
        
        // Add buffered events
        let storage = self.event_storage.read();
        all_events.extend(storage.buffer.clone());
        
        Ok(all_events)
    }
    
    /// Load hash chain
    async fn load_hash_chain(&self) -> Result<(), AIError> {
        let events = self.load_all_events().await?;
        
        if let Some(last_event) = events.last() {
            *self.current_hash.write() = last_event.hash.clone();
        }
        
        Ok(())
    }
    
    /// Update statistics
    fn update_stats(&self, event: &AuditEvent) {
        let mut stats = self.stats.write();
        stats.total_events += 1;
        stats.last_event = Some(event.timestamp);
        
        let type_key = format!("{:?}", event.event_type);
        stats.events_by_type.entry(type_key).and_modify(|c| *c += 1).or_insert(1);
        
        let severity_key = format!("{:?}", event.severity);
        stats.events_by_severity.entry(severity_key).and_modify(|c| *c += 1).or_insert(1);
    }
    
    /// Clean up old logs
    pub async fn cleanup_old_logs(&self) -> Result<(), AIError> {
        let retention_days = self.config.read().retention_days;
        let cutoff = Utc::now() - Duration::days(retention_days as i64);
        
        let storage = self.event_storage.read();
        let storage_path = storage.storage_path.clone();
        drop(storage);
        
        let mut entries = fs::read_dir(&storage_path).await
            .map_err(|e| AIError::Io(e))?;
        
        while let Some(entry) = entries.next_entry().await
            .map_err(|e| AIError::Io(e))? {
            if let Some(file_name) = entry.file_name().to_str() {
                if file_name.starts_with("audit-") && file_name.ends_with(".jsonl") {
                    // Extract date from filename
                    if let Some(date_str) = file_name.strip_prefix("audit-").and_then(|s| s.strip_suffix(".jsonl")) {
                        if let Ok(file_date) = DateTime::parse_from_str(&format!("{} 00:00:00 +0000", date_str), "%Y-%m-%d %H:%M:%S %z") {
                            if file_date.with_timezone(&Utc) < cutoff {
                                fs::remove_file(entry.path()).await
                                    .map_err(|e| AIError::Io(e))?;
                                info!("Deleted old audit log: {}", file_name);
                            }
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Update configuration
    pub async fn update_config(&self, new_config: AuditConfig) -> Result<(), AIError> {
        *self.config.write() = new_config;
        info!("Audit configuration updated");
        Ok(())
    }
    
    /// Get statistics
    pub fn get_stats(&self) -> AuditStats {
        self.stats.read().clone()
    }
}

impl OperationType {
    fn to_string(&self) -> &'static str {
        match self {
            OperationType::Collection => "collection",
            OperationType::Processing => "processing",
            OperationType::Storage => "storage",
            OperationType::Sharing => "sharing",
            OperationType::Deletion => "deletion",
            OperationType::Export => "export",
        }
    }
}

impl Clone for AuditStats {
    fn clone(&self) -> Self {
        Self {
            total_events: self.total_events,
            events_by_type: self.events_by_type.clone(),
            events_by_severity: self.events_by_severity.clone(),
            storage_size: self.storage_size,
            last_event: self.last_event,
            chain_verified: self.chain_verified,
            last_verification: self.last_verification,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_audit_event() {
        let event = AuditEvent {
            id: "event-123".to_string(),
            timestamp: Utc::now(),
            event_type: AuditEventType::DataAccess,
            category: "data-access".to_string(),
            actor: Actor {
                id: "user-456".to_string(),
                actor_type: ActorType::User,
                name: "Test User".to_string(),
                ip_address: None,
                session_id: None,
            },
            resource: Some("file.txt".to_string()),
            operation: Some("read".to_string()),
            result: AuditResult::Success,
            severity: AuditSeverity::Info,
            details: HashMap::new(),
            hash: None,
            previous_hash: None,
        };
        
        assert_eq!(event.id, "event-123");
        assert!(matches!(event.event_type, AuditEventType::DataAccess));
        assert!(matches!(event.result, AuditResult::Success));
        assert!(matches!(event.severity, AuditSeverity::Info));
    }
    
    #[test]
    fn test_audit_query() {
        let query = AuditQuery {
            time_range: Some((Utc::now() - Duration::days(7), Utc::now())),
            event_types: Some(vec![AuditEventType::DataAccess, AuditEventType::Security]),
            actors: Some(vec!["user-123".to_string()]),
            resources: None,
            min_severity: Some(AuditSeverity::Warning),
            limit: Some(100),
            metadata: HashMap::new(),
        };
        
        assert!(query.time_range.is_some());
        assert_eq!(query.event_types.as_ref().unwrap().len(), 2);
        assert_eq!(query.actors.as_ref().unwrap().len(), 1);
        assert_eq!(query.limit, Some(100));
    }
    
    #[test]
    fn test_severity_ordering() {
        let mut severities = vec![
            AuditSeverity::Error,
            AuditSeverity::Info,
            AuditSeverity::Critical,
            AuditSeverity::Warning,
        ];
        
        severities.sort();
        
        assert_eq!(severities, vec![
            AuditSeverity::Info,
            AuditSeverity::Warning,
            AuditSeverity::Error,
            AuditSeverity::Critical,
        ]);
    }
}