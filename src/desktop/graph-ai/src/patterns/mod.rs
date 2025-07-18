//! Pattern detection and behavioral learning for AI integration
//! 
//! This module provides pattern detection, behavioral learning, and
//! intelligent suggestions based on user actions and preferences.

use crate::AIError;
use crate::storage::{Pattern, PatternType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use chrono::{DateTime, Utc};

/// Pattern storage for behavioral learning
pub struct PatternStorage {
    /// Detected patterns
    patterns: Arc<RwLock<HashMap<String, DetectedPattern>>>,
    /// Pattern statistics
    stats: Arc<RwLock<PatternStats>>,
}

// UserAction and ActionType are now imported from storage module

/// Detected behavioral pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedPattern {
    /// Pattern ID
    pub id: String,
    /// Pattern name
    pub name: String,
    /// Pattern confidence (0.0 to 1.0)
    pub confidence: f32,
    /// Number of occurrences
    pub occurrences: u32,
    /// First detected time
    pub first_detected: DateTime<Utc>,
    /// Last detected time
    pub last_detected: DateTime<Utc>,
    /// Pattern type
    pub pattern_type: PatternType,
    /// Pattern details
    pub details: serde_json::Value,
}

// PatternType is now imported from storage module

/// Pattern detection statistics
#[derive(Debug, Default)]
pub struct PatternStats {
    /// Total patterns detected
    total_patterns: u64,
    /// Patterns by type
    patterns_by_type: HashMap<String, u64>,
    /// Pattern detection accuracy
    accuracy: f32,
    /// Last detection time
    last_detection: Option<DateTime<Utc>>,
}

impl PatternStorage {
    /// Create a new pattern storage
    pub fn new() -> Self {
        Self {
            patterns: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(PatternStats::default())),
        }
    }
    
    /// Initialize pattern storage
    pub async fn initialize(&self) -> Result<(), AIError> {
        // TODO: Load existing patterns from storage
        Ok(())
    }
    
    /// Add a detected pattern
    pub fn add_pattern(&self, pattern: DetectedPattern) {
        let mut patterns = self.patterns.write();
        let mut stats = self.stats.write();
        
        patterns.insert(pattern.id.clone(), pattern.clone());
        stats.total_patterns += 1;
        
        let pattern_type_key = format!("{:?}", pattern.pattern_type);
        stats.patterns_by_type.entry(pattern_type_key).and_modify(|e| *e += 1).or_insert(1);
        stats.last_detection = Some(Utc::now());
    }
    
    /// Get patterns by type
    pub fn get_patterns_by_type(&self, pattern_type: PatternType) -> Vec<DetectedPattern> {
        let patterns = self.patterns.read();
        patterns.values()
            .filter(|p| std::mem::discriminant(&p.pattern_type) == std::mem::discriminant(&pattern_type))
            .cloned()
            .collect()
    }
    
    /// Get all patterns
    pub fn get_all_patterns(&self) -> Vec<DetectedPattern> {
        let patterns = self.patterns.read();
        patterns.values().cloned().collect()
    }
}