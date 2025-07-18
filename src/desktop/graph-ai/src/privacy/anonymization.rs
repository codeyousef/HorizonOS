//! Data anonymization for privacy protection
//! 
//! This module provides data anonymization techniques to protect user privacy
//! while maintaining data utility for AI processing.

use crate::AIError;
use crate::privacy::{AnonymizationConfig, AnonymizationTechnique};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use chrono::{DateTime, Utc};
use log::{info, warn, debug};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use sha2::{Sha256, Digest};

/// Anonymization rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnonymizationRule {
    /// Rule ID
    pub id: String,
    /// Rule name
    pub name: String,
    /// Rule pattern (regex)
    pub pattern: String,
    /// Replacement strategy
    pub strategy: ReplacementStrategy,
    /// Rule priority
    pub priority: u32,
    /// Rule enabled
    pub enabled: bool,
    /// Rule metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Replacement strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReplacementStrategy {
    /// Replace with fixed value
    Fixed(String),
    /// Replace with random value
    Random,
    /// Replace with hash
    Hash,
    /// Replace with generalization
    Generalize(GeneralizationLevel),
    /// Replace with format-preserving value
    FormatPreserving,
    /// Custom replacement function
    Custom(String),
}

/// Generalization level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GeneralizationLevel {
    /// Low generalization
    Low,
    /// Medium generalization
    Medium,
    /// High generalization
    High,
}

/// Anonymization context
#[derive(Debug, Clone)]
pub struct AnonymizationContext {
    /// Original value mappings
    pub mappings: HashMap<String, String>,
    /// Anonymization seed
    pub seed: Option<u64>,
    /// Context metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Anonymization result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnonymizationResult {
    /// Anonymized data
    pub data: String,
    /// Applied rules
    pub applied_rules: Vec<String>,
    /// Anonymization metrics
    pub metrics: AnonymizationMetrics,
    /// Result metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Anonymization metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnonymizationMetrics {
    /// Original data size
    pub original_size: usize,
    /// Anonymized data size
    pub anonymized_size: usize,
    /// Number of replacements
    pub replacements: usize,
    /// Data utility score (0.0 to 1.0)
    pub utility_score: f32,
    /// Privacy score (0.0 to 1.0)
    pub privacy_score: f32,
    /// Processing time
    pub processing_time: std::time::Duration,
}

/// K-anonymity group
#[derive(Debug, Clone)]
struct KAnonymityGroup {
    /// Group ID
    id: String,
    /// Group members
    members: Vec<String>,
    /// Quasi-identifiers
    quasi_identifiers: HashMap<String, String>,
    /// Group size
    size: usize,
}

/// Anonymization engine
pub struct AnonymizationEngine {
    /// Configuration
    config: Arc<RwLock<AnonymizationConfig>>,
    /// Anonymization rules
    rules: Arc<RwLock<Vec<AnonymizationRule>>>,
    /// Pseudonym mappings
    pseudonyms: Arc<RwLock<HashMap<String, String>>>,
    /// K-anonymity groups
    k_groups: Arc<RwLock<HashMap<String, KAnonymityGroup>>>,
    /// Engine statistics
    stats: Arc<RwLock<AnonymizationStats>>,
}

/// Anonymization statistics
#[derive(Debug, Default)]
pub struct AnonymizationStats {
    /// Total anonymizations
    total_anonymizations: u64,
    /// Total replacements
    total_replacements: u64,
    /// Average utility score
    avg_utility_score: f32,
    /// Average privacy score
    avg_privacy_score: f32,
    /// Cache hits
    cache_hits: u64,
    /// Cache misses
    cache_misses: u64,
    /// Last anonymization
    last_anonymization: Option<DateTime<Utc>>,
}

impl AnonymizationEngine {
    /// Create a new anonymization engine
    pub async fn new(config: AnonymizationConfig) -> Result<Self, AIError> {
        let engine = Self {
            config: Arc::new(RwLock::new(config)),
            rules: Arc::new(RwLock::new(Vec::new())),
            pseudonyms: Arc::new(RwLock::new(HashMap::new())),
            k_groups: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(AnonymizationStats::default())),
        };
        
        // Load default rules
        engine.load_default_rules().await?;
        
        info!("Anonymization engine initialized");
        Ok(engine)
    }
    
    /// Load default anonymization rules
    async fn load_default_rules(&self) -> Result<(), AIError> {
        let default_rules = vec![
            AnonymizationRule {
                id: "email".to_string(),
                name: "Email Anonymization".to_string(),
                pattern: r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}".to_string(),
                strategy: ReplacementStrategy::Hash,
                priority: 100,
                enabled: true,
                metadata: HashMap::new(),
            },
            AnonymizationRule {
                id: "phone".to_string(),
                name: "Phone Number Anonymization".to_string(),
                pattern: r"\b\d{3}[-.]?\d{3}[-.]?\d{4}\b".to_string(),
                strategy: ReplacementStrategy::Fixed("XXX-XXX-XXXX".to_string()),
                priority: 99,
                enabled: true,
                metadata: HashMap::new(),
            },
            AnonymizationRule {
                id: "ssn".to_string(),
                name: "SSN Anonymization".to_string(),
                pattern: r"\b\d{3}-\d{2}-\d{4}\b".to_string(),
                strategy: ReplacementStrategy::Fixed("XXX-XX-XXXX".to_string()),
                priority: 98,
                enabled: true,
                metadata: HashMap::new(),
            },
            AnonymizationRule {
                id: "credit_card".to_string(),
                name: "Credit Card Anonymization".to_string(),
                pattern: r"\b\d{4}[\s-]?\d{4}[\s-]?\d{4}[\s-]?\d{4}\b".to_string(),
                strategy: ReplacementStrategy::Fixed("XXXX-XXXX-XXXX-XXXX".to_string()),
                priority: 97,
                enabled: true,
                metadata: HashMap::new(),
            },
            AnonymizationRule {
                id: "ip_address".to_string(),
                name: "IP Address Anonymization".to_string(),
                pattern: r"\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b".to_string(),
                strategy: ReplacementStrategy::Generalize(GeneralizationLevel::Medium),
                priority: 96,
                enabled: true,
                metadata: HashMap::new(),
            },
        ];
        
        *self.rules.write() = default_rules;
        Ok(())
    }
    
    /// Anonymize data
    pub async fn anonymize(&self, data: &str) -> Result<String, AIError> {
        if !self.config.read().enabled {
            return Ok(data.to_string());
        }
        
        let start_time = std::time::Instant::now();
        let mut anonymized = data.to_string();
        let mut replacements = 0;
        let mut applied_rules = Vec::new();
        
        // Apply rules in priority order
        let mut rules = self.rules.read().clone();
        rules.sort_by_key(|r| std::cmp::Reverse(r.priority));
        
        for rule in rules {
            if !rule.enabled {
                continue;
            }
            
            let regex = regex::Regex::new(&rule.pattern)
                .map_err(|e| AIError::Configuration(format!("Invalid regex pattern: {}", e)))?;
            
            let mut rule_applied = false;
            
            anonymized = regex.replace_all(&anonymized, |caps: &regex::Captures| {
                rule_applied = true;
                replacements += 1;
                self.apply_replacement(&caps[0], &rule.strategy)
            }).to_string();
            
            if rule_applied {
                applied_rules.push(rule.id);
            }
        }
        
        // Apply custom rules
        let config = self.config.read();
        for (pattern, replacement) in &config.custom_rules {
            anonymized = anonymized.replace(pattern, replacement);
            replacements += 1;
        }
        
        // Calculate metrics
        let utility_score = self.calculate_utility_score(&data, &anonymized);
        let privacy_score = self.calculate_privacy_score(replacements, data.len());
        
        // Update statistics
        let mut stats = self.stats.write();
        stats.total_anonymizations += 1;
        stats.total_replacements += replacements as u64;
        stats.avg_utility_score = (stats.avg_utility_score * (stats.total_anonymizations - 1) as f32 + utility_score) / stats.total_anonymizations as f32;
        stats.avg_privacy_score = (stats.avg_privacy_score * (stats.total_anonymizations - 1) as f32 + privacy_score) / stats.total_anonymizations as f32;
        stats.last_anonymization = Some(Utc::now());
        
        debug!("Data anonymized: {} replacements in {:?}", replacements, start_time.elapsed());
        Ok(anonymized)
    }
    
    /// Apply replacement strategy
    fn apply_replacement(&self, value: &str, strategy: &ReplacementStrategy) -> String {
        match strategy {
            ReplacementStrategy::Fixed(replacement) => replacement.clone(),
            ReplacementStrategy::Random => self.generate_random_replacement(value),
            ReplacementStrategy::Hash => self.hash_value(value),
            ReplacementStrategy::Generalize(level) => self.generalize_value(value, level),
            ReplacementStrategy::FormatPreserving => self.format_preserving_replacement(value),
            ReplacementStrategy::Custom(func_name) => {
                // TODO: Implement custom function lookup
                format!("[{}]", func_name)
            }
        }
    }
    
    /// Generate random replacement
    fn generate_random_replacement(&self, value: &str) -> String {
        let mut rng = rand::thread_rng();
        let length = value.len();
        
        if value.chars().all(|c| c.is_ascii_digit()) {
            // Random digits
            (0..length).map(|_| rng.gen_range(0..10).to_string()).collect()
        } else if value.chars().all(|c| c.is_alphabetic()) {
            // Random letters
            (0..length).map(|_| {
                let c = rng.gen_range(b'a'..=b'z') as char;
                if value.chars().any(|vc| vc.is_uppercase()) {
                    c.to_uppercase().to_string()
                } else {
                    c.to_string()
                }
            }).collect()
        } else {
            // Mixed content
            format!("ANON{:06}", rng.gen::<u32>() % 1000000)
        }
    }
    
    /// Hash value for pseudonymization
    fn hash_value(&self, value: &str) -> String {
        let config = self.config.read();
        
        if config.reversible {
            // Store mapping for reversibility
            let mut pseudonyms = self.pseudonyms.write();
            
            if let Some(pseudonym) = pseudonyms.get(value) {
                self.stats.write().cache_hits += 1;
                return pseudonym.clone();
            }
            
            self.stats.write().cache_misses += 1;
            
            let pseudonym = format!("PSEUDO_{:08X}", pseudonyms.len());
            pseudonyms.insert(value.to_string(), pseudonym.clone());
            pseudonym
        } else {
            // One-way hash
            let mut hasher = Sha256::new();
            hasher.update(value.as_bytes());
            format!("HASH_{:016X}", u64::from_be_bytes(hasher.finalize()[0..8].try_into().unwrap()))
        }
    }
    
    /// Generalize value
    fn generalize_value(&self, value: &str, level: &GeneralizationLevel) -> String {
        // IP address generalization
        if let Ok(parts) = value.split('.').map(|p| p.parse::<u8>()).collect::<Result<Vec<_>, _>>() {
            if parts.len() == 4 {
                return match level {
                    GeneralizationLevel::Low => format!("{}.{}.{}.0", parts[0], parts[1], parts[2]),
                    GeneralizationLevel::Medium => format!("{}.{}.0.0", parts[0], parts[1]),
                    GeneralizationLevel::High => format!("{}.0.0.0", parts[0]),
                };
            }
        }
        
        // Date generalization
        if value.contains('-') && value.len() == 10 {
            let parts: Vec<&str> = value.split('-').collect();
            if parts.len() == 3 {
                return match level {
                    GeneralizationLevel::Low => format!("{}-{}-01", parts[0], parts[1]),
                    GeneralizationLevel::Medium => format!("{}-01-01", parts[0]),
                    GeneralizationLevel::High => format!("{}-XX-XX", parts[0]),
                };
            }
        }
        
        // Default generalization
        match level {
            GeneralizationLevel::Low => {
                if value.len() > 3 {
                    format!("{}***", &value[..3])
                } else {
                    "***".to_string()
                }
            }
            GeneralizationLevel::Medium => {
                if value.len() > 1 {
                    format!("{}***", &value[..1])
                } else {
                    "***".to_string()
                }
            }
            GeneralizationLevel::High => "***".to_string(),
        }
    }
    
    /// Format-preserving replacement
    fn format_preserving_replacement(&self, value: &str) -> String {
        value.chars().map(|c| {
            if c.is_ascii_digit() {
                'X'
            } else if c.is_alphabetic() {
                if c.is_uppercase() {
                    'X'
                } else {
                    'x'
                }
            } else {
                c
            }
        }).collect()
    }
    
    /// Apply k-anonymity
    pub async fn apply_k_anonymity(&self, data: Vec<HashMap<String, String>>, k: u32, quasi_identifiers: Vec<String>) -> Result<Vec<HashMap<String, String>>, AIError> {
        if data.len() < k as usize {
            return Err(AIError::Configuration(format!("Dataset too small for {}-anonymity", k)));
        }
        
        let technique = self.config.read().technique.clone();
        
        match technique {
            AnonymizationTechnique::KAnonymity(configured_k) => {
                let k_value = if configured_k > 0 { configured_k } else { k };
                self.create_k_groups(data, k_value, quasi_identifiers).await
            }
            _ => Err(AIError::Configuration("K-anonymity not configured".to_string())),
        }
    }
    
    /// Create k-anonymity groups
    async fn create_k_groups(&self, data: Vec<HashMap<String, String>>, k: u32, quasi_identifiers: Vec<String>) -> Result<Vec<HashMap<String, String>>, AIError> {
        // Group records by quasi-identifiers
        let mut groups: HashMap<String, Vec<HashMap<String, String>>> = HashMap::new();
        
        for record in data {
            let key = quasi_identifiers.iter()
                .filter_map(|qi| record.get(qi))
                .cloned()
                .collect::<Vec<_>>()
                .join("|");
            
            groups.entry(key).or_insert_with(Vec::new).push(record);
        }
        
        // Generalize groups with less than k members
        let mut result = Vec::new();
        
        for (_, group) in groups {
            if group.len() >= k as usize {
                // Group satisfies k-anonymity
                result.extend(group);
            } else {
                // Generalize quasi-identifiers
                for mut record in group {
                    for qi in &quasi_identifiers {
                        if let Some(value) = record.get_mut(qi) {
                            *value = self.generalize_value(value, &GeneralizationLevel::Medium);
                        }
                    }
                    result.push(record);
                }
            }
        }
        
        Ok(result)
    }
    
    /// Add differential privacy noise
    pub async fn add_noise(&self, value: f64, epsilon: f64) -> Result<f64, AIError> {
        let technique = self.config.read().technique.clone();
        
        match technique {
            AnonymizationTechnique::NoiseAddition => {
                // Laplace noise for differential privacy
                let mut rng = rand::thread_rng();
                let scale = 1.0 / epsilon;
                let u: f64 = rng.gen_range(-0.5..0.5);
                let noise = -scale * u.signum() * (1.0 - 2.0 * u.abs()).ln();
                Ok(value + noise)
            }
            _ => Ok(value),
        }
    }
    
    /// Calculate utility score
    fn calculate_utility_score(&self, original: &str, anonymized: &str) -> f32 {
        if !self.config.read().preserve_utility {
            return 0.5; // Default score when utility preservation is disabled
        }
        
        // Simple utility metric based on preserved structure
        let original_len = original.len() as f32;
        let anonymized_len = anonymized.len() as f32;
        let len_ratio = (original_len - (original_len - anonymized_len).abs()) / original_len;
        
        // Check preserved format
        let format_score = original.chars().zip(anonymized.chars())
            .filter(|(o, a)| {
                (o.is_alphabetic() && a.is_alphabetic()) ||
                (o.is_numeric() && (a.is_numeric() || *a == 'X')) ||
                (o.is_whitespace() && a.is_whitespace()) ||
                (o == a)
            })
            .count() as f32 / original_len;
        
        (len_ratio + format_score) / 2.0
    }
    
    /// Calculate privacy score
    fn calculate_privacy_score(&self, replacements: usize, data_length: usize) -> f32 {
        if data_length == 0 {
            return 1.0;
        }
        
        // Simple privacy metric based on replacement ratio
        let replacement_ratio = replacements as f32 / data_length as f32;
        (replacement_ratio * 10.0).min(1.0) // Scale to 0.0-1.0
    }
    
    /// Export pseudonym mappings
    pub async fn export_mappings(&self) -> HashMap<String, String> {
        self.pseudonyms.read().clone()
    }
    
    /// Import pseudonym mappings
    pub async fn import_mappings(&self, mappings: HashMap<String, String>) -> Result<(), AIError> {
        *self.pseudonyms.write() = mappings;
        info!("Imported {} pseudonym mappings", self.pseudonyms.read().len());
        Ok(())
    }
    
    /// Add custom rule
    pub async fn add_rule(&self, rule: AnonymizationRule) -> Result<(), AIError> {
        self.rules.write().push(rule);
        info!("Added anonymization rule");
        Ok(())
    }
    
    /// Remove rule
    pub async fn remove_rule(&self, rule_id: &str) -> Result<(), AIError> {
        self.rules.write().retain(|r| r.id != rule_id);
        info!("Removed anonymization rule: {}", rule_id);
        Ok(())
    }
    
    /// Update configuration
    pub async fn update_config(&self, new_config: AnonymizationConfig) -> Result<(), AIError> {
        *self.config.write() = new_config;
        info!("Anonymization configuration updated");
        Ok(())
    }
    
    /// Get statistics
    pub fn get_stats(&self) -> AnonymizationStats {
        self.stats.read().clone()
    }
}

impl Clone for AnonymizationStats {
    fn clone(&self) -> Self {
        Self {
            total_anonymizations: self.total_anonymizations,
            total_replacements: self.total_replacements,
            avg_utility_score: self.avg_utility_score,
            avg_privacy_score: self.avg_privacy_score,
            cache_hits: self.cache_hits,
            cache_misses: self.cache_misses,
            last_anonymization: self.last_anonymization,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_anonymization_rule() {
        let rule = AnonymizationRule {
            id: "test-rule".to_string(),
            name: "Test Rule".to_string(),
            pattern: r"\d+".to_string(),
            strategy: ReplacementStrategy::Fixed("XXX".to_string()),
            priority: 100,
            enabled: true,
            metadata: HashMap::new(),
        };
        
        assert_eq!(rule.id, "test-rule");
        assert_eq!(rule.name, "Test Rule");
        assert_eq!(rule.priority, 100);
        assert!(rule.enabled);
    }
    
    #[test]
    fn test_replacement_strategies() {
        let fixed = ReplacementStrategy::Fixed("REDACTED".to_string());
        let random = ReplacementStrategy::Random;
        let hash = ReplacementStrategy::Hash;
        let generalize = ReplacementStrategy::Generalize(GeneralizationLevel::Medium);
        let format_preserving = ReplacementStrategy::FormatPreserving;
        
        // Test that all variants can be created
        assert!(matches!(fixed, ReplacementStrategy::Fixed(_)));
        assert!(matches!(random, ReplacementStrategy::Random));
        assert!(matches!(hash, ReplacementStrategy::Hash));
        assert!(matches!(generalize, ReplacementStrategy::Generalize(_)));
        assert!(matches!(format_preserving, ReplacementStrategy::FormatPreserving));
    }
    
    #[test]
    fn test_anonymization_metrics() {
        let metrics = AnonymizationMetrics {
            original_size: 100,
            anonymized_size: 95,
            replacements: 5,
            utility_score: 0.8,
            privacy_score: 0.9,
            processing_time: std::time::Duration::from_millis(50),
        };
        
        assert_eq!(metrics.original_size, 100);
        assert_eq!(metrics.anonymized_size, 95);
        assert_eq!(metrics.replacements, 5);
        assert_eq!(metrics.utility_score, 0.8);
        assert_eq!(metrics.privacy_score, 0.9);
    }
}