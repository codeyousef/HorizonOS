//! Suggestion engine for intelligent recommendations

use crate::{AIError, DisplayMode, SuggestionConfig};
use crate::patterns::{Pattern, PatternType};
use chrono::{DateTime, Local, Utc};
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

/// Suggestion engine
pub struct SuggestionEngine {
    /// Current configuration
    config: RwLock<SuggestionConfig>,
    /// Active suggestions
    active_suggestions: DashMap<String, Suggestion>,
    /// Suggestion history (for rate limiting)
    suggestion_history: RwLock<Vec<SuggestionRecord>>,
    /// Channel for sending suggestions to UI
    suggestion_sender: Option<mpsc::Sender<Suggestion>>,
}

impl SuggestionEngine {
    /// Create new suggestion engine
    pub fn new() -> Self {
        Self {
            config: RwLock::new(SuggestionConfig::default()),
            active_suggestions: DashMap::new(),
            suggestion_history: RwLock::new(Vec::new()),
            suggestion_sender: None,
        }
    }
    
    /// Initialize the suggestion engine
    pub async fn initialize(&self, config: SuggestionConfig) -> Result<(), AIError> {
        *self.config.write() = config;
        log::info!("Suggestion engine initialized");
        Ok(())
    }
    
    /// Set the suggestion sender channel
    pub fn set_sender(&mut self, sender: mpsc::Sender<Suggestion>) {
        self.suggestion_sender = Some(sender);
    }
    
    /// Generate suggestion from pattern
    pub async fn suggest_from_pattern(&self, pattern: &Pattern) -> Option<Suggestion> {
        let config = self.config.read();
        
        if !config.enabled {
            return None;
        }
        
        // Check if we're in quiet hours
        if let Some((start, end)) = &config.quiet_hours {
            if is_in_quiet_hours(start, end) {
                return None;
            }
        }
        
        // Check rate limit
        if !self.check_rate_limit() {
            return None;
        }
        
        // Generate suggestion based on pattern type
        let suggestion = match &pattern.pattern_type {
            PatternType::AppLaunch => {
                if config.app_launch {
                    self.create_app_suggestion(pattern)
                } else {
                    None
                }
            }
            PatternType::DocumentAccess => {
                if config.document_open {
                    self.create_document_suggestion(pattern)
                } else {
                    None
                }
            }
            PatternType::WebsiteVisit => {
                if config.website_visit {
                    self.create_website_suggestion(pattern)
                } else {
                    None
                }
            }
            PatternType::Workflow => {
                if config.workflow_automation {
                    self.create_workflow_suggestion(pattern)
                } else {
                    None
                }
            }
            _ => None,
        };
        
        if let Some(suggestion) = suggestion {
            self.record_suggestion(&suggestion);
            self.send_suggestion(suggestion.clone()).await;
            Some(suggestion)
        } else {
            None
        }
    }
    
    /// Check if we're within rate limits
    fn check_rate_limit(&self) -> bool {
        let config = self.config.read();
        let history = self.suggestion_history.read();
        
        let one_hour_ago = Utc::now() - chrono::Duration::hours(1);
        let recent_count = history
            .iter()
            .filter(|record| record.timestamp > one_hour_ago)
            .count();
        
        recent_count < config.max_per_hour as usize
    }
    
    /// Record a suggestion in history
    fn record_suggestion(&self, suggestion: &Suggestion) {
        let mut history = self.suggestion_history.write();
        history.push(SuggestionRecord {
            suggestion_id: suggestion.id.clone(),
            timestamp: Utc::now(),
        });
        
        // Keep only last 100 records
        if history.len() > 100 {
            let drain_count = history.len() - 100;
            history.drain(0..drain_count);
        }
    }
    
    /// Send suggestion to UI
    async fn send_suggestion(&self, suggestion: Suggestion) {
        if let Some(sender) = &self.suggestion_sender {
            let _ = sender.send(suggestion).await;
        }
    }
    
    /// Create app launch suggestion
    fn create_app_suggestion(&self, pattern: &Pattern) -> Option<Suggestion> {
        if let crate::patterns::PatternData::AppLaunch { app_name, .. } = &pattern.data {
            Some(Suggestion {
                id: uuid::Uuid::new_v4().to_string(),
                pattern_id: pattern.id.clone(),
                action: SuggestedAction::LaunchApp {
                    app_name: app_name.clone(),
                },
                title: format!("Launch {}", app_name),
                description: Some("Based on your usage pattern".to_string()),
                confidence: pattern.confidence,
                display_mode: self.config.read().display_mode,
                created_at: Utc::now(),
            })
        } else {
            None
        }
    }
    
    /// Create document suggestion
    fn create_document_suggestion(&self, pattern: &Pattern) -> Option<Suggestion> {
        if let crate::patterns::PatternData::DocumentGroup { documents, .. } = &pattern.data {
            if !documents.is_empty() {
                Some(Suggestion {
                    id: uuid::Uuid::new_v4().to_string(),
                    pattern_id: pattern.id.clone(),
                    action: SuggestedAction::OpenDocuments {
                        paths: documents.clone(),
                    },
                    title: "Open recent documents".to_string(),
                    description: Some(format!("Open {} frequently used documents", documents.len())),
                    confidence: pattern.confidence,
                    display_mode: self.config.read().display_mode,
                    created_at: Utc::now(),
                })
            } else {
                None
            }
        } else {
            None
        }
    }
    
    /// Create website suggestion
    fn create_website_suggestion(&self, pattern: &Pattern) -> Option<Suggestion> {
        if let crate::patterns::PatternData::WebsiteSequence { urls, .. } = &pattern.data {
            if !urls.is_empty() {
                Some(Suggestion {
                    id: uuid::Uuid::new_v4().to_string(),
                    pattern_id: pattern.id.clone(),
                    action: SuggestedAction::OpenWebsites {
                        urls: urls.clone(),
                    },
                    title: "Open frequently visited sites".to_string(),
                    description: Some("Based on your browsing pattern".to_string()),
                    confidence: pattern.confidence,
                    display_mode: self.config.read().display_mode,
                    created_at: Utc::now(),
                })
            } else {
                None
            }
        } else {
            None
        }
    }
    
    /// Create workflow suggestion
    fn create_workflow_suggestion(&self, pattern: &Pattern) -> Option<Suggestion> {
        if let crate::patterns::PatternData::WorkflowSequence { actions, .. } = &pattern.data {
            Some(Suggestion {
                id: uuid::Uuid::new_v4().to_string(),
                pattern_id: pattern.id.clone(),
                action: SuggestedAction::RunWorkflow {
                    workflow_id: pattern.id.clone(),
                    steps: actions.clone(),
                },
                title: "Run automated workflow".to_string(),
                description: Some(format!("Automate {} common actions", actions.len())),
                confidence: pattern.confidence,
                display_mode: self.config.read().display_mode,
                created_at: Utc::now(),
            })
        } else {
            None
        }
    }
}

/// A suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    /// Unique suggestion ID
    pub id: String,
    /// Pattern that triggered this suggestion
    pub pattern_id: String,
    /// Suggested action
    pub action: SuggestedAction,
    /// Display title
    pub title: String,
    /// Display description
    pub description: Option<String>,
    /// Confidence score
    pub confidence: f32,
    /// Display mode
    pub display_mode: DisplayMode,
    /// Creation time
    pub created_at: DateTime<Utc>,
}

/// Suggested actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SuggestedAction {
    /// Launch an application
    LaunchApp {
        app_name: String,
    },
    /// Open documents
    OpenDocuments {
        paths: Vec<String>,
    },
    /// Open websites
    OpenWebsites {
        urls: Vec<String>,
    },
    /// Run a workflow
    RunWorkflow {
        workflow_id: String,
        steps: Vec<String>,
    },
}

/// Suggestion record for history
#[derive(Debug, Clone)]
struct SuggestionRecord {
    suggestion_id: String,
    timestamp: DateTime<Utc>,
}

/// Check if current time is in quiet hours
fn is_in_quiet_hours(start: &str, end: &str) -> bool {
    let now = Local::now().format("%H:%M").to_string();
    
    // Simple comparison - assumes end time is after start time
    // TODO: Handle overnight quiet hours (e.g., 22:00 to 08:00)
    now.as_str() >= start && now.as_str() <= end
}

// Re-export uuid
pub use uuid;