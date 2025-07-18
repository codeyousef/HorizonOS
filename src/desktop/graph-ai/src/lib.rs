//! AI integration layer for HorizonOS graph desktop
//! 
//! This module provides comprehensive AI capabilities including:
//! - Local LLM processing with Ollama
//! - Hardware detection and model selection
//! - AI agents for various tasks
//! - Integration with the graph desktop

pub mod hardware;
pub mod ollama;
pub mod agents;
pub mod services;
pub mod patterns;
pub mod suggestions;
pub mod storage;
pub mod monitoring;
pub mod automation;
pub mod privacy;

use std::sync::Arc;
use dashmap::DashMap;
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use chrono::{DateTime, Utc};

/// Global AI service instance
pub static AI_SERVICE: Lazy<Arc<AIService>> = Lazy::new(|| {
    Arc::new(AIService::new())
});

/// AI service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIConfig {
    /// Whether AI features are enabled
    pub enabled: bool,
    /// Ollama server endpoint
    pub ollama_endpoint: String,
    /// Default model to use
    pub default_model: String,
    /// Hardware optimization settings
    pub hardware_optimization: HardwareOptimization,
    /// Privacy settings
    pub privacy: PrivacyConfig,
    /// Learning settings
    pub learning: LearningConfig,
    /// Suggestion settings
    pub suggestions: SuggestionConfig,
}

impl Default for AIConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            ollama_endpoint: "http://localhost:11434".to_string(),
            default_model: "llama3.2:latest".to_string(),
            hardware_optimization: HardwareOptimization::Auto,
            privacy: PrivacyConfig::default(),
            learning: LearningConfig::default(),
            suggestions: SuggestionConfig::default(),
        }
    }
}

/// Hardware optimization mode
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum HardwareOptimization {
    /// Automatically detect and optimize
    Auto,
    /// Prefer GPU acceleration
    PreferGPU,
    /// CPU only
    CPUOnly,
    /// Low power mode
    PowerSaving,
}

/// Privacy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyConfig {
    /// Process all data locally
    pub local_only: bool,
    /// Enable telemetry
    pub telemetry_enabled: bool,
    /// Data retention policy
    pub data_retention: DataRetention,
    /// Encrypt stored data
    pub encrypt_storage: bool,
    /// Filter sensitive data
    pub sensitive_data_filter: bool,
}

impl Default for PrivacyConfig {
    fn default() -> Self {
        Self {
            local_only: true,
            telemetry_enabled: false,
            data_retention: DataRetention::Days(30),
            encrypt_storage: true,
            sensitive_data_filter: true,
        }
    }
}

/// Data retention policy
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DataRetention {
    /// Keep data for session only
    SessionOnly,
    /// Keep data for specified days
    Days(u32),
    /// Keep data forever
    Forever,
}

/// Learning configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningConfig {
    /// Enable behavioral learning
    pub enabled: bool,
    /// Learn from applications
    pub applications: bool,
    /// Learn from documents
    pub documents: bool,
    /// Learn from websites
    pub websites: bool,
    /// Learn from workflows
    pub workflows: bool,
    /// Minimum confidence threshold
    pub min_confidence: f32,
    /// Minimum occurrences before learning
    pub min_occurrences: u32,
    /// Excluded applications
    pub excluded_apps: Vec<String>,
    /// Excluded paths
    pub excluded_paths: Vec<String>,
    /// Excluded domains
    pub excluded_domains: Vec<String>,
}

impl Default for LearningConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            applications: true,
            documents: true,
            websites: true,
            workflows: true,
            min_confidence: 0.7,
            min_occurrences: 5,
            excluded_apps: vec!["1password".to_string(), "keepassxc".to_string()],
            excluded_paths: vec!["~/private".to_string(), "~/secure".to_string()],
            excluded_domains: vec!["*.bank.com".to_string(), "*.health.gov".to_string()],
        }
    }
}

/// Suggestion configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionConfig {
    /// Enable suggestions
    pub enabled: bool,
    /// Display mode for suggestions
    pub display_mode: DisplayMode,
    /// Maximum suggestions per hour
    pub max_per_hour: u32,
    /// Quiet hours (no suggestions)
    pub quiet_hours: Option<(String, String)>,
    /// Suggest app launches
    pub app_launch: bool,
    /// Suggest document opens
    pub document_open: bool,
    /// Suggest website visits
    pub website_visit: bool,
    /// Suggest workflow automations
    pub workflow_automation: bool,
}

impl Default for SuggestionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            display_mode: DisplayMode::Toast,
            max_per_hour: 3,
            quiet_hours: Some(("22:00".to_string(), "08:00".to_string())),
            app_launch: true,
            document_open: true,
            website_visit: true,
            workflow_automation: true,
        }
    }
}

/// Display mode for suggestions
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DisplayMode {
    /// Small toast notification
    Toast,
    /// Floating bubble
    Bubble,
    /// Sidebar widget
    Sidebar,
    /// System tray icon
    SysTray,
    /// No display (log only)
    None,
}

/// Main AI service
pub struct AIService {
    /// Configuration
    config: RwLock<AIConfig>,
    /// Active AI sessions
    sessions: DashMap<String, AISession>,
    /// Pattern storage
    patterns: Arc<patterns::PatternStorage>,
    /// Suggestion engine
    suggestions: Arc<suggestions::SuggestionEngine>,
    /// Storage manager
    storage: Arc<storage::StorageManager>,
    /// Hardware monitor
    hardware_monitor: Arc<hardware::HardwareMonitor>,
}

impl AIService {
    /// Create a new AI service
    pub fn new() -> Self {
        Self {
            config: RwLock::new(AIConfig::default()),
            sessions: DashMap::new(),
            patterns: Arc::new(patterns::PatternStorage::new()),
            suggestions: Arc::new(suggestions::SuggestionEngine::new()),
            storage: Arc::new(storage::StorageManager::new_default()),
            hardware_monitor: Arc::new(hardware::HardwareMonitor::new()),
        }
    }

    /// Initialize the AI service
    pub async fn initialize(&self) -> Result<(), AIError> {
        let config = self.config.read().clone();
        
        if !config.enabled {
            log::info!("AI service is disabled");
            return Ok(());
        }

        // Initialize Ollama connection
        ollama::OllamaClient::new(&config.ollama_endpoint)
            .test_connection()
            .await?;

        // Detect hardware capabilities
        let hardware_profile = hardware::detect_hardware_profile()?;
        log::info!("Detected hardware profile: {:?}", hardware_profile);

        // Select optimal model based on hardware
        let model = hardware::select_optimal_model(
            &hardware_profile,
            config.hardware_optimization,
        );
        log::info!("Selected model: {}", model);

        // Initialize pattern detection
        self.patterns.initialize().await?;

        // Initialize suggestion engine
        self.suggestions.initialize(config.suggestions).await?;

        log::info!("AI service initialized successfully");
        Ok(())
    }

    /// Create a new AI session
    pub async fn create_session(&self, purpose: &str) -> Result<String, AIError> {
        let session_id = uuid::Uuid::new_v4().to_string();
        let session = AISession::new(session_id.clone(), purpose.to_string());
        
        self.sessions.insert(session_id.clone(), session);
        
        Ok(session_id)
    }

    /// Get current configuration
    pub fn config(&self) -> AIConfig {
        self.config.read().clone()
    }

    /// Update configuration
    pub fn update_config<F>(&self, f: F) 
    where
        F: FnOnce(&mut AIConfig),
    {
        let mut config = self.config.write();
        f(&mut config);
    }
}

/// AI session for a specific task
#[derive(Debug, Clone)]
pub struct AISession {
    /// Session ID
    pub id: String,
    /// Purpose of the session
    pub purpose: String,
    /// Creation time
    pub created_at: DateTime<Utc>,
    /// Last activity time
    pub last_activity: DateTime<Utc>,
    /// Session context
    pub context: SessionContext,
}

impl AISession {
    /// Create a new session
    pub fn new(id: String, purpose: String) -> Self {
        let now = Utc::now();
        Self {
            id,
            purpose,
            created_at: now,
            last_activity: now,
            context: SessionContext::default(),
        }
    }

    /// Update last activity time
    pub fn touch(&mut self) {
        self.last_activity = Utc::now();
    }
}

/// Session context for maintaining conversation state
#[derive(Debug, Clone, Default)]
pub struct SessionContext {
    /// Conversation history
    pub messages: Vec<Message>,
    /// Session metadata
    pub metadata: serde_json::Value,
}

/// Message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Role (user, assistant, system)
    pub role: MessageRole,
    /// Message content
    pub content: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Message role
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

/// AI service errors
#[derive(Debug, Error)]
pub enum AIError {
    #[error("Ollama connection failed: {0}")]
    OllamaConnection(String),
    
    #[error("Hardware detection failed: {0}")]
    HardwareDetection(String),
    
    #[error("Model not available: {0}")]
    ModelNotAvailable(String),
    
    #[error("Session not found: {0}")]
    SessionNotFound(String),
    
    #[error("Pattern detection error: {0}")]
    PatternDetection(String),
    
    #[error("Suggestion error: {0}")]
    SuggestionError(String),
    
    #[error("Unsupported operation: {0}")]
    UnsupportedOperation(String),
    
    #[error("Configuration error: {0}")]
    Configuration(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

// Re-export uuid
pub use uuid;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AIConfig::default();
        assert!(config.enabled);
        assert_eq!(config.ollama_endpoint, "http://localhost:11434");
        assert!(config.privacy.local_only);
        assert!(!config.privacy.telemetry_enabled);
    }

    #[test]
    fn test_session_creation() {
        let session = AISession::new(
            "test-id".to_string(),
            "test-purpose".to_string(),
        );
        assert_eq!(session.id, "test-id");
        assert_eq!(session.purpose, "test-purpose");
        assert!(session.context.messages.is_empty());
    }
}