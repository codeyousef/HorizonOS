//! Consent management for privacy controls
//! 
//! This module handles user consent for data collection, processing,
//! and AI operations with granular control and GDPR compliance.

use crate::AIError;
use crate::privacy::{ConsentConfig, ConsentState};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use chrono::{DateTime, Utc, Duration};
use tokio::sync::mpsc;
use log::{info, warn, debug};

/// Consent category
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ConsentCategory {
    /// Category ID
    pub id: String,
    /// Category name
    pub name: String,
    /// Category description
    pub description: String,
    /// Parent category (for hierarchical consent)
    pub parent: Option<String>,
    /// Required for core functionality
    pub required: bool,
    /// Data types involved
    pub data_types: Vec<String>,
    /// Purpose of data use
    pub purposes: Vec<String>,
}

/// Consent record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsentRecord {
    /// Record ID
    pub id: String,
    /// User ID
    pub user_id: String,
    /// Category ID
    pub category_id: String,
    /// Consent state
    pub state: ConsentState,
    /// Granted timestamp
    pub granted_at: Option<DateTime<Utc>>,
    /// Withdrawn timestamp
    pub withdrawn_at: Option<DateTime<Utc>>,
    /// Expiry timestamp
    pub expires_at: Option<DateTime<Utc>>,
    /// Consent version
    pub version: String,
    /// Consent method
    pub method: ConsentMethod,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Consent method
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsentMethod {
    /// Explicit checkbox
    Checkbox,
    /// Settings panel
    Settings,
    /// Initial setup
    Setup,
    /// Imported from backup
    Imported,
    /// System default
    Default,
}

/// Consent request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsentRequest {
    /// Request ID
    pub id: String,
    /// Category
    pub category: ConsentCategory,
    /// Purpose description
    pub purpose: String,
    /// Data usage description
    pub data_usage: String,
    /// Request timestamp
    pub requested_at: DateTime<Utc>,
    /// Response deadline
    pub response_deadline: Option<DateTime<Utc>>,
    /// Request priority
    pub priority: ConsentPriority,
}

/// Consent priority
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConsentPriority {
    /// Low priority
    Low,
    /// Normal priority
    Normal,
    /// High priority
    High,
    /// Critical (blocks functionality)
    Critical,
}

/// Consent change event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsentChangeEvent {
    /// Event ID
    pub id: String,
    /// Category ID
    pub category_id: String,
    /// Previous state
    pub previous_state: ConsentState,
    /// New state
    pub new_state: ConsentState,
    /// Change timestamp
    pub timestamp: DateTime<Utc>,
    /// Change reason
    pub reason: ChangeReason,
    /// Event metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Change reason
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeReason {
    /// User action
    UserAction,
    /// Expiry
    Expired,
    /// Policy update
    PolicyUpdate,
    /// System requirement
    SystemRequirement,
    /// Data deletion
    DataDeletion,
}

/// Consent manager
pub struct ConsentManager {
    /// Configuration
    config: Arc<RwLock<ConsentConfig>>,
    /// Consent records by category
    records: Arc<RwLock<HashMap<String, ConsentRecord>>>,
    /// Consent categories
    categories: Arc<RwLock<HashMap<String, ConsentCategory>>>,
    /// Pending requests
    pending_requests: Arc<RwLock<Vec<ConsentRequest>>>,
    /// Change history
    change_history: Arc<RwLock<Vec<ConsentChangeEvent>>>,
    /// Event sender
    event_sender: mpsc::UnboundedSender<ConsentChangeEvent>,
    /// Manager statistics
    stats: Arc<RwLock<ConsentStats>>,
}

/// Consent statistics
#[derive(Debug, Default)]
pub struct ConsentStats {
    /// Total consent requests
    total_requests: u64,
    /// Granted consents
    granted: u64,
    /// Denied consents
    denied: u64,
    /// Withdrawn consents
    withdrawn: u64,
    /// Expired consents
    expired: u64,
    /// Last consent check
    last_check: Option<DateTime<Utc>>,
}

impl ConsentManager {
    /// Create a new consent manager
    pub async fn new(config: ConsentConfig) -> Result<Self, AIError> {
        let (event_sender, mut event_receiver) = mpsc::unbounded_channel();
        
        let manager = Self {
            config: Arc::new(RwLock::new(config)),
            records: Arc::new(RwLock::new(HashMap::new())),
            categories: Arc::new(RwLock::new(HashMap::new())),
            pending_requests: Arc::new(RwLock::new(Vec::new())),
            change_history: Arc::new(RwLock::new(Vec::new())),
            event_sender,
            stats: Arc::new(RwLock::new(ConsentStats::default())),
        };
        
        // Initialize default categories
        manager.initialize_categories().await?;
        
        // Start event processor
        let change_history = manager.change_history.clone();
        tokio::spawn(async move {
            while let Some(event) = event_receiver.recv().await {
                change_history.write().push(event);
            }
        });
        
        info!("Consent manager initialized");
        Ok(manager)
    }
    
    /// Initialize default consent categories
    async fn initialize_categories(&self) -> Result<(), AIError> {
        let default_categories = vec![
            ConsentCategory {
                id: "behavioral-learning".to_string(),
                name: "Behavioral Learning".to_string(),
                description: "Learn from your usage patterns to provide suggestions".to_string(),
                parent: None,
                required: false,
                data_types: vec!["app-usage".to_string(), "file-access".to_string()],
                purposes: vec!["suggestions".to_string(), "automation".to_string()],
            },
            ConsentCategory {
                id: "automation".to_string(),
                name: "Process Automation".to_string(),
                description: "Automate repetitive tasks based on your behavior".to_string(),
                parent: None,
                required: false,
                data_types: vec!["ui-actions".to_string(), "workflows".to_string()],
                purposes: vec!["automation".to_string(), "efficiency".to_string()],
            },
            ConsentCategory {
                id: "ai-processing".to_string(),
                name: "AI Processing".to_string(),
                description: "Use AI to analyze and process your data locally".to_string(),
                parent: None,
                required: false,
                data_types: vec!["documents".to_string(), "conversations".to_string()],
                purposes: vec!["analysis".to_string(), "assistance".to_string()],
            },
            ConsentCategory {
                id: "telemetry".to_string(),
                name: "Anonymous Telemetry".to_string(),
                description: "Share anonymous usage statistics to improve the system".to_string(),
                parent: None,
                required: false,
                data_types: vec!["usage-stats".to_string(), "performance-metrics".to_string()],
                purposes: vec!["improvement".to_string(), "debugging".to_string()],
            },
        ];
        
        let mut categories = self.categories.write();
        for category in default_categories {
            categories.insert(category.id.clone(), category);
        }
        
        Ok(())
    }
    
    /// Check if user has consent for a category
    pub async fn has_consent(&self, category_id: &str) -> Result<bool, AIError> {
        // Check if consent is required
        if !self.config.read().require_explicit_consent {
            return Ok(true);
        }
        
        // Check consent record
        let records = self.records.read();
        if let Some(record) = records.get(category_id) {
            match record.state {
                ConsentState::Given => {
                    // Check expiry
                    if let Some(expires_at) = record.expires_at {
                        if expires_at < Utc::now() {
                            // Consent expired
                            drop(records);
                            self.update_consent_state(category_id, ConsentState::Expired).await?;
                            return Ok(false);
                        }
                    }
                    Ok(true)
                }
                _ => Ok(false),
            }
        } else {
            // No record, check default state
            Ok(matches!(self.config.read().default_state, ConsentState::Given))
        }
    }
    
    /// Request user consent
    pub async fn request_consent(&self, category_id: &str, purpose: &str) -> Result<bool, AIError> {
        let categories = self.categories.read();
        let category = categories.get(category_id)
            .ok_or_else(|| AIError::Configuration(format!("Unknown consent category: {}", category_id)))?
            .clone();
        drop(categories);
        
        // Create consent request
        let request = ConsentRequest {
            id: uuid::Uuid::new_v4().to_string(),
            category: category.clone(),
            purpose: purpose.to_string(),
            data_usage: self.generate_data_usage_description(&category),
            requested_at: Utc::now(),
            response_deadline: None,
            priority: if category.required { ConsentPriority::Critical } else { ConsentPriority::Normal },
        };
        
        // Add to pending requests
        self.pending_requests.write().push(request.clone());
        
        // Update statistics
        self.stats.write().total_requests += 1;
        
        // TODO: Show consent dialog to user
        // For now, simulate user response based on config
        let granted = !category.required || matches!(self.config.read().default_state, ConsentState::Given);
        
        if granted {
            self.grant_consent(category_id, ConsentMethod::Settings).await?;
            Ok(true)
        } else {
            self.deny_consent(category_id).await?;
            Ok(false)
        }
    }
    
    /// Grant consent for a category
    pub async fn grant_consent(&self, category_id: &str, method: ConsentMethod) -> Result<(), AIError> {
        let now = Utc::now();
        let expires_at = self.config.read().renewal_period
            .map(|days| now + Duration::days(days as i64));
        
        let record = ConsentRecord {
            id: uuid::Uuid::new_v4().to_string(),
            user_id: "default".to_string(), // TODO: Get actual user ID
            category_id: category_id.to_string(),
            state: ConsentState::Given,
            granted_at: Some(now),
            withdrawn_at: None,
            expires_at,
            version: "1.0".to_string(),
            method,
            metadata: HashMap::new(),
        };
        
        let previous_state = self.records.read()
            .get(category_id)
            .map(|r| r.state.clone())
            .unwrap_or(ConsentState::NotGiven);
        
        self.records.write().insert(category_id.to_string(), record);
        
        // Send change event
        let event = ConsentChangeEvent {
            id: uuid::Uuid::new_v4().to_string(),
            category_id: category_id.to_string(),
            previous_state,
            new_state: ConsentState::Given,
            timestamp: now,
            reason: ChangeReason::UserAction,
            metadata: HashMap::new(),
        };
        
        let _ = self.event_sender.send(event);
        
        // Update statistics
        self.stats.write().granted += 1;
        
        info!("Consent granted for category: {}", category_id);
        Ok(())
    }
    
    /// Deny consent for a category
    pub async fn deny_consent(&self, category_id: &str) -> Result<(), AIError> {
        let record = ConsentRecord {
            id: uuid::Uuid::new_v4().to_string(),
            user_id: "default".to_string(),
            category_id: category_id.to_string(),
            state: ConsentState::NotGiven,
            granted_at: None,
            withdrawn_at: None,
            expires_at: None,
            version: "1.0".to_string(),
            method: ConsentMethod::Settings,
            metadata: HashMap::new(),
        };
        
        self.records.write().insert(category_id.to_string(), record);
        self.stats.write().denied += 1;
        
        info!("Consent denied for category: {}", category_id);
        Ok(())
    }
    
    /// Withdraw consent for a category
    pub async fn withdraw_consent(&self, category_id: &str) -> Result<(), AIError> {
        if !self.config.read().allow_withdrawal {
            return Err(AIError::Configuration("Consent withdrawal is not allowed".to_string()));
        }
        
        let now = Utc::now();
        
        if let Some(record) = self.records.write().get_mut(category_id) {
            let previous_state = record.state.clone();
            record.state = ConsentState::Withdrawn;
            record.withdrawn_at = Some(now);
            
            // Send change event
            let event = ConsentChangeEvent {
                id: uuid::Uuid::new_v4().to_string(),
                category_id: category_id.to_string(),
                previous_state,
                new_state: ConsentState::Withdrawn,
                timestamp: now,
                reason: ChangeReason::UserAction,
                metadata: HashMap::new(),
            };
            
            let _ = self.event_sender.send(event);
            
            // Update statistics
            self.stats.write().withdrawn += 1;
            
            info!("Consent withdrawn for category: {}", category_id);
        }
        
        Ok(())
    }
    
    /// Get consent status for all categories
    pub async fn get_consent_status(&self) -> Result<HashMap<String, ConsentState>, AIError> {
        let mut status = HashMap::new();
        let records = self.records.read();
        
        for (category_id, record) in records.iter() {
            status.insert(category_id.clone(), record.state.clone());
        }
        
        Ok(status)
    }
    
    /// Get consent history
    pub async fn get_consent_history(&self) -> Vec<ConsentChangeEvent> {
        self.change_history.read().clone()
    }
    
    /// Update consent state
    async fn update_consent_state(&self, category_id: &str, new_state: ConsentState) -> Result<(), AIError> {
        if let Some(record) = self.records.write().get_mut(category_id) {
            let previous_state = record.state;
            record.state = new_state;
            
            // Send change event
            let event = ConsentChangeEvent {
                id: uuid::Uuid::new_v4().to_string(),
                category_id: category_id.to_string(),
                previous_state,
                new_state,
                timestamp: Utc::now(),
                reason: ChangeReason::Expired,
                metadata: HashMap::new(),
            };
            
            let _ = self.event_sender.send(event);
            
            // Update statistics
            if matches!(new_state, ConsentState::Expired) {
                self.stats.write().expired += 1;
            }
        }
        
        Ok(())
    }
    
    /// Generate data usage description
    fn generate_data_usage_description(&self, category: &ConsentCategory) -> String {
        format!(
            "We will collect {} to {}. This data will be used for: {}.",
            category.data_types.join(", "),
            category.description.to_lowercase(),
            category.purposes.join(", ")
        )
    }
    
    /// Check and update expired consents
    pub async fn check_expired_consents(&self) -> Result<(), AIError> {
        let now = Utc::now();
        let records = self.records.read().clone();
        
        for (category_id, record) in &records {
            if let Some(expires_at) = record.expires_at {
                if expires_at < now && matches!(record.state, ConsentState::Given) {
                    self.update_consent_state(category_id, ConsentState::Expired).await?;
                }
            }
        }
        
        self.stats.write().last_check = Some(now);
        Ok(())
    }
    
    /// Export consent records
    pub async fn export_consent_records(&self) -> Result<Vec<ConsentRecord>, AIError> {
        Ok(self.records.read().values().cloned().collect())
    }
    
    /// Import consent records
    pub async fn import_consent_records(&self, records: Vec<ConsentRecord>) -> Result<(), AIError> {
        let mut current_records = self.records.write();
        
        for record in records {
            current_records.insert(record.category_id.clone(), record);
        }
        
        info!("Imported {} consent records", current_records.len());
        Ok(())
    }
    
    /// Update configuration
    pub async fn update_config(&self, new_config: ConsentConfig) -> Result<(), AIError> {
        *self.config.write() = new_config;
        info!("Consent configuration updated");
        Ok(())
    }
    
    /// Get statistics
    pub fn get_stats(&self) -> ConsentStats {
        self.stats.read().clone()
    }
}

impl Clone for ConsentStats {
    fn clone(&self) -> Self {
        Self {
            total_requests: self.total_requests,
            granted: self.granted,
            denied: self.denied,
            withdrawn: self.withdrawn,
            expired: self.expired,
            last_check: self.last_check,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_consent_category() {
        let category = ConsentCategory {
            id: "test-category".to_string(),
            name: "Test Category".to_string(),
            description: "Test description".to_string(),
            parent: None,
            required: false,
            data_types: vec!["test-data".to_string()],
            purposes: vec!["testing".to_string()],
        };
        
        assert_eq!(category.id, "test-category");
        assert_eq!(category.name, "Test Category");
        assert!(!category.required);
        assert_eq!(category.data_types.len(), 1);
        assert_eq!(category.purposes.len(), 1);
    }
    
    #[test]
    fn test_consent_record() {
        let record = ConsentRecord {
            id: "record-123".to_string(),
            user_id: "user-456".to_string(),
            category_id: "test-category".to_string(),
            state: ConsentState::Given,
            granted_at: Some(Utc::now()),
            withdrawn_at: None,
            expires_at: None,
            version: "1.0".to_string(),
            method: ConsentMethod::Checkbox,
            metadata: HashMap::new(),
        };
        
        assert_eq!(record.id, "record-123");
        assert_eq!(record.user_id, "user-456");
        assert!(matches!(record.state, ConsentState::Given));
        assert!(record.granted_at.is_some());
        assert!(record.withdrawn_at.is_none());
    }
    
    #[test]
    fn test_consent_priority_ordering() {
        let mut priorities = vec![
            ConsentPriority::Normal,
            ConsentPriority::Critical,
            ConsentPriority::Low,
            ConsentPriority::High,
        ];
        
        priorities.sort();
        
        assert_eq!(priorities, vec![
            ConsentPriority::Low,
            ConsentPriority::Normal,
            ConsentPriority::High,
            ConsentPriority::Critical,
        ]);
    }
}