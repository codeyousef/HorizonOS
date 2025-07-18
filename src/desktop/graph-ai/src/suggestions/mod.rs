//! Suggestion engine for AI integration
//! 
//! This module provides intelligent suggestions based on user patterns,
//! context, and learned behaviors.

use crate::AIError;
use crate::SuggestionConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use chrono::{DateTime, Utc};

/// Suggestion engine for generating intelligent recommendations
pub struct SuggestionEngine {
    /// Current suggestions
    suggestions: Arc<RwLock<Vec<Suggestion>>>,
    /// Configuration
    config: Arc<RwLock<SuggestionConfig>>,
    /// Statistics
    stats: Arc<RwLock<SuggestionStats>>,
}

/// Generated suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    /// Suggestion ID
    pub id: String,
    /// Suggestion type
    pub suggestion_type: SuggestionType,
    /// Suggestion title
    pub title: String,
    /// Suggestion description
    pub description: String,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// Priority level
    pub priority: SuggestionPriority,
    /// Generated timestamp
    pub generated_at: DateTime<Utc>,
    /// Expires at
    pub expires_at: Option<DateTime<Utc>>,
    /// Action to take
    pub action: SuggestionAction,
    /// Context information
    pub context: serde_json::Value,
}

/// Types of suggestions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SuggestionType {
    /// Application launch suggestion
    AppLaunch,
    /// Document open suggestion
    DocumentOpen,
    /// Website visit suggestion
    WebsiteVisit,
    /// Workflow automation suggestion
    WorkflowAutomation,
    /// Configuration change suggestion
    ConfigChange,
    /// Optimization suggestion
    Optimization,
    /// Custom suggestion
    Custom(String),
}

/// Suggestion priority levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SuggestionPriority {
    /// Low priority
    Low,
    /// Medium priority
    Medium,
    /// High priority
    High,
    /// Critical priority
    Critical,
}

/// Action to take when suggestion is accepted
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionAction {
    /// Action type
    pub action_type: String,
    /// Action parameters
    pub parameters: HashMap<String, serde_json::Value>,
}

/// Suggestion engine statistics
#[derive(Debug, Default)]
pub struct SuggestionStats {
    /// Total suggestions generated
    total_generated: u64,
    /// Suggestions accepted
    accepted: u64,
    /// Suggestions rejected
    rejected: u64,
    /// Suggestions ignored
    ignored: u64,
    /// Acceptance rate
    acceptance_rate: f32,
    /// Last suggestion time
    last_suggestion: Option<DateTime<Utc>>,
}

impl SuggestionEngine {
    /// Create a new suggestion engine
    pub fn new() -> Self {
        Self {
            suggestions: Arc::new(RwLock::new(Vec::new())),
            config: Arc::new(RwLock::new(SuggestionConfig::default())),
            stats: Arc::new(RwLock::new(SuggestionStats::default())),
        }
    }
    
    /// Initialize the suggestion engine
    pub async fn initialize(&self, config: SuggestionConfig) -> Result<(), AIError> {
        *self.config.write() = config;
        Ok(())
    }
    
    /// Generate a suggestion
    pub fn generate_suggestion(&self, suggestion: Suggestion) {
        let mut suggestions = self.suggestions.write();
        let mut stats = self.stats.write();
        
        suggestions.push(suggestion);
        stats.total_generated += 1;
        stats.last_suggestion = Some(Utc::now());
    }
    
    /// Get current suggestions
    pub fn get_suggestions(&self) -> Vec<Suggestion> {
        let suggestions = self.suggestions.read();
        suggestions.clone()
    }
    
    /// Clear suggestions
    pub fn clear_suggestions(&self) {
        let mut suggestions = self.suggestions.write();
        suggestions.clear();
    }
}