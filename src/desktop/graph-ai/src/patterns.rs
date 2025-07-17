//! Pattern detection and storage for behavioral learning

use crate::AIError;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Pattern storage for learned behaviors
pub struct PatternStorage {
    /// Stored patterns by category
    patterns: DashMap<String, Vec<Pattern>>,
}

impl PatternStorage {
    /// Create new pattern storage
    pub fn new() -> Self {
        Self {
            patterns: DashMap::new(),
        }
    }
    
    /// Initialize pattern storage
    pub async fn initialize(&self) -> Result<(), AIError> {
        // TODO: Load patterns from persistent storage
        log::info!("Pattern storage initialized");
        Ok(())
    }
    
    /// Add a new pattern
    pub fn add_pattern(&self, category: &str, pattern: Pattern) {
        self.patterns
            .entry(category.to_string())
            .or_insert_with(Vec::new)
            .push(pattern);
    }
    
    /// Get patterns for a category
    pub fn get_patterns(&self, category: &str) -> Option<Vec<Pattern>> {
        self.patterns.get(category).map(|p| p.clone())
    }
    
    /// Update pattern confidence
    pub fn update_confidence(&self, pattern_id: &str, new_confidence: f32) {
        for mut entry in self.patterns.iter_mut() {
            for pattern in entry.value_mut() {
                if pattern.id == pattern_id {
                    pattern.confidence = new_confidence;
                    pattern.last_seen = Utc::now();
                    break;
                }
            }
        }
    }
}

/// A learned pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    /// Unique pattern ID
    pub id: String,
    /// Pattern type
    pub pattern_type: PatternType,
    /// Pattern data
    pub data: PatternData,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// First time this pattern was seen
    pub first_seen: DateTime<Utc>,
    /// Last time this pattern was seen
    pub last_seen: DateTime<Utc>,
    /// Number of occurrences
    pub occurrence_count: u32,
    /// Whether the pattern is enabled
    pub enabled: bool,
}

/// Types of patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternType {
    /// Application launch pattern
    AppLaunch,
    /// Document access pattern
    DocumentAccess,
    /// Website visit pattern
    WebsiteVisit,
    /// Workflow pattern
    Workflow,
    /// Time-based pattern
    Temporal,
    /// Sequence pattern
    Sequence,
}

/// Pattern data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternData {
    /// App launch at specific time
    AppLaunch {
        app_name: String,
        time_of_day: String,
        day_of_week: Option<String>,
    },
    /// Document frequently accessed together
    DocumentGroup {
        documents: Vec<String>,
        context: Option<String>,
    },
    /// Website visit pattern
    WebsiteSequence {
        urls: Vec<String>,
        typical_duration: u64,
    },
    /// Workflow sequence
    WorkflowSequence {
        actions: Vec<String>,
        typical_interval: u64,
    },
}

/// Pattern detector
pub struct PatternDetector {
    /// Minimum confidence threshold
    min_confidence: f32,
    /// Minimum occurrences
    min_occurrences: u32,
}

impl PatternDetector {
    /// Create new pattern detector
    pub fn new(min_confidence: f32, min_occurrences: u32) -> Self {
        Self {
            min_confidence,
            min_occurrences,
        }
    }
    
    /// Process a user action to detect patterns
    pub async fn process_action(&self, _action: UserAction) -> Option<Pattern> {
        // TODO: Implement pattern detection logic
        // This would analyze action sequences and detect patterns
        // For now, return None
        None
    }
    
    /// Detect app launch patterns
    pub fn detect_app_patterns(&self, actions: &[UserAction]) -> Vec<Pattern> {
        let mut patterns = Vec::new();
        let mut app_sequences = HashMap::new();
        
        // Group actions by time windows (e.g., within 5 minutes)
        for window in actions.windows(3) {
            if let Some(first) = window.first() {
                if matches!(first.action_type, ActionType::AppLaunch) {
                    let key = format!("{}-{}", first.target, first.timestamp.format("%H"));
                    *app_sequences.entry(key).or_insert(0) += 1;
                }
            }
        }
        
        // Create patterns from frequent sequences
        for (key, count) in app_sequences {
            if count >= self.min_occurrences {
                let parts: Vec<&str> = key.split('-').collect();
                if parts.len() >= 2 {
                    patterns.push(Pattern {
                        id: uuid::Uuid::new_v4().to_string(),
                        pattern_type: PatternType::AppLaunch,
                        confidence: (count as f32 / actions.len() as f32).min(1.0),
                        occurrences: count,
                        data: PatternData::AppLaunch {
                            app_name: parts[0].to_string(),
                            time_of_day: parts[1].to_string(),
                        },
                        first_seen: Utc::now(),
                        last_seen: Utc::now(),
                    });
                }
            }
        }
        
        patterns
    }
}

/// User action for pattern detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAction {
    /// Action type
    pub action_type: ActionType,
    /// Target of the action
    pub target: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Additional context
    pub context: Option<serde_json::Value>,
}

/// Types of user actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionType {
    AppLaunch,
    AppClose,
    FileOpen,
    FileSave,
    WebNavigate,
    WebClose,
    CommandExecute,
    WindowFocus,
    WindowClose,
}