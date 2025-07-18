//! Privacy filter for user action monitoring
//! 
//! Implements comprehensive privacy controls to ensure user data
//! is handled according to their preferences and privacy regulations.

use crate::AIError;
use crate::monitoring::RawEvent;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use parking_lot::RwLock;
use regex::Regex;
use chrono::{DateTime, Utc};
use log::{info, warn, error, debug};

/// Privacy filter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyFilterConfig {
    /// Enable privacy filtering
    pub enabled: bool,
    /// Exclude private browsing/incognito mode
    pub exclude_private_browsing: bool,
    /// Exclude password managers
    pub exclude_password_managers: bool,
    /// Exclude banking and financial apps
    pub exclude_financial_apps: bool,
    /// Exclude healthcare apps
    pub exclude_healthcare_apps: bool,
    /// Exclude private directories
    pub exclude_private_directories: bool,
    /// Custom excluded applications
    pub excluded_applications: Vec<String>,
    /// Custom excluded file paths (regex patterns)
    pub excluded_paths: Vec<String>,
    /// Custom excluded domains (regex patterns)
    pub excluded_domains: Vec<String>,
    /// Custom excluded window titles (regex patterns)
    pub excluded_window_titles: Vec<String>,
    /// Enable PII detection
    pub enable_pii_detection: bool,
    /// Enable credential detection
    pub enable_credential_detection: bool,
    /// Minimum confidence for PII detection
    pub pii_confidence_threshold: f32,
    /// Maximum text length to analyze for PII
    pub max_text_analysis_length: usize,
}

impl Default for PrivacyFilterConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            exclude_private_browsing: true,
            exclude_password_managers: true,
            exclude_financial_apps: true,
            exclude_healthcare_apps: true,
            exclude_private_directories: true,
            excluded_applications: vec![
                "1password".to_string(),
                "keepassxc".to_string(),
                "bitwarden".to_string(),
                "lastpass".to_string(),
                "gnome-keyring".to_string(),
                "kwallet".to_string(),
            ],
            excluded_paths: vec![
                r".*\.ssh/.*".to_string(),
                r".*\.gnupg/.*".to_string(),
                r".*/Private/.*".to_string(),
                r".*/private/.*".to_string(),
                r".*/\.password-store/.*".to_string(),
                r".*/Passwords/.*".to_string(),
            ],
            excluded_domains: vec![
                r".*\.bank\.com".to_string(),
                r".*banking\..*".to_string(),
                r".*paypal\.com".to_string(),
                r".*stripe\.com".to_string(),
                r".*health\.gov".to_string(),
                r".*medical\..*".to_string(),
                r"localhost".to_string(),
                r"127\.0\.0\.1".to_string(),
            ],
            excluded_window_titles: vec![
                r".*password.*".to_string(),
                r".*private.*".to_string(),
                r".*incognito.*".to_string(),
                r".*sign.?in.*".to_string(),
                r".*login.*".to_string(),
            ],
            enable_pii_detection: true,
            enable_credential_detection: true,
            pii_confidence_threshold: 0.8,
            max_text_analysis_length: 1000,
        }
    }
}

/// Privacy filter statistics
#[derive(Debug, Default)]
pub struct PrivacyFilterStats {
    /// Total events processed
    pub events_processed: u64,
    /// Events filtered out
    pub events_filtered: u64,
    /// Events with PII detected
    pub pii_detections: u64,
    /// Events with credentials detected
    pub credential_detections: u64,
    /// Filter by reason
    pub filter_reasons: HashMap<String, u64>,
}

/// Privacy filter implementation
pub struct PrivacyFilter {
    /// Configuration
    config: Arc<RwLock<PrivacyFilterConfig>>,
    /// Compiled regex patterns for paths
    path_patterns: Arc<RwLock<Vec<Regex>>>,
    /// Compiled regex patterns for domains
    domain_patterns: Arc<RwLock<Vec<Regex>>>,
    /// Compiled regex patterns for window titles
    window_title_patterns: Arc<RwLock<Vec<Regex>>>,
    /// Known sensitive applications
    sensitive_apps: Arc<RwLock<HashSet<String>>>,
    /// PII detection patterns
    pii_patterns: Arc<RwLock<Vec<PIIPattern>>>,
    /// Filter statistics
    stats: Arc<RwLock<PrivacyFilterStats>>,
}

/// PII detection pattern
#[derive(Debug, Clone)]
struct PIIPattern {
    /// Pattern name
    name: String,
    /// Compiled regex
    regex: Regex,
    /// Confidence score
    confidence: f32,
    /// Pattern type
    pattern_type: PIIType,
}

/// Types of PII
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PIIType {
    /// Email address
    Email,
    /// Phone number
    Phone,
    /// Social Security Number
    SSN,
    /// Credit card number
    CreditCard,
    /// IP address
    IPAddress,
    /// URL with sensitive path
    SensitiveURL,
    /// Password-like string
    Password,
    /// API key or token
    APIKey,
}

impl PrivacyFilter {
    /// Create a new privacy filter
    pub async fn new(config: PrivacyFilterConfig) -> Result<Self, AIError> {
        let config = Arc::new(RwLock::new(config));
        
        let filter = Self {
            config: config.clone(),
            path_patterns: Arc::new(RwLock::new(Vec::new())),
            domain_patterns: Arc::new(RwLock::new(Vec::new())),
            window_title_patterns: Arc::new(RwLock::new(Vec::new())),
            sensitive_apps: Arc::new(RwLock::new(HashSet::new())),
            pii_patterns: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(PrivacyFilterStats::default())),
        };
        
        // Compile patterns
        filter.compile_patterns().await?;
        
        // Initialize PII patterns
        filter.initialize_pii_patterns().await?;
        
        // Initialize sensitive apps
        filter.initialize_sensitive_apps().await?;
        
        info!("Privacy filter initialized with {} patterns", filter.get_pattern_count());
        Ok(filter)
    }
    
    /// Filter an event based on privacy configuration
    pub async fn filter_event(&self, event: &RawEvent) -> Option<RawEvent> {
        let mut stats = self.stats.write();
        stats.events_processed += 1;
        
        let config = self.config.read();
        
        if !config.enabled {
            return Some(event.clone());
        }
        
        // Check various privacy filters
        if self.should_filter_application(event, &config) {
            stats.events_filtered += 1;
            stats.filter_reasons.entry("application".to_string()).and_modify(|e| *e += 1).or_insert(1);
            return None;
        }
        
        if self.should_filter_path(event, &config) {
            stats.events_filtered += 1;
            stats.filter_reasons.entry("path".to_string()).and_modify(|e| *e += 1).or_insert(1);
            return None;
        }
        
        if self.should_filter_domain(event, &config) {
            stats.events_filtered += 1;
            stats.filter_reasons.entry("domain".to_string()).and_modify(|e| *e += 1).or_insert(1);
            return None;
        }
        
        if self.should_filter_window_title(event, &config) {
            stats.events_filtered += 1;
            stats.filter_reasons.entry("window_title".to_string()).and_modify(|e| *e += 1).or_insert(1);
            return None;
        }
        
        if config.enable_pii_detection && self.contains_pii(event, &config) {
            stats.events_filtered += 1;
            stats.pii_detections += 1;
            stats.filter_reasons.entry("pii".to_string()).and_modify(|e| *e += 1).or_insert(1);
            return None;
        }
        
        if config.enable_credential_detection && self.contains_credentials(event, &config) {
            stats.events_filtered += 1;
            stats.credential_detections += 1;
            stats.filter_reasons.entry("credentials".to_string()).and_modify(|e| *e += 1).or_insert(1);
            return None;
        }
        
        // Event passed all filters
        Some(self.sanitize_event(event, &config))
    }
    
    /// Check if application should be filtered
    fn should_filter_application(&self, event: &RawEvent, config: &PrivacyFilterConfig) -> bool {
        let app_name = event.data.get("application")
            .or_else(|| event.data.get("app_name"))
            .or_else(|| event.data.get("process_name"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        
        if app_name.is_empty() {
            return false;
        }
        
        let app_name_lower = app_name.to_lowercase();
        
        // Check against excluded applications
        if config.excluded_applications.iter().any(|excluded| app_name_lower.contains(&excluded.to_lowercase())) {
            return true;
        }
        
        // Check against sensitive apps
        if self.sensitive_apps.read().contains(&app_name_lower) {
            return true;
        }
        
        // Check for private browsing indicators
        if config.exclude_private_browsing {
            if app_name_lower.contains("incognito") || app_name_lower.contains("private") {
                return true;
            }
        }
        
        false
    }
    
    /// Check if file path should be filtered
    fn should_filter_path(&self, event: &RawEvent, _config: &PrivacyFilterConfig) -> bool {
        let file_path = event.data.get("file_path")
            .or_else(|| event.data.get("path"))
            .or_else(|| event.data.get("target"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        
        if file_path.is_empty() {
            return false;
        }
        
        // Check against path patterns
        let path_patterns = self.path_patterns.read();
        path_patterns.iter().any(|pattern| pattern.is_match(file_path))
    }
    
    /// Check if domain should be filtered
    fn should_filter_domain(&self, event: &RawEvent, _config: &PrivacyFilterConfig) -> bool {
        let url = event.data.get("url")
            .or_else(|| event.data.get("domain"))
            .or_else(|| event.data.get("host"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        
        if url.is_empty() {
            return false;
        }
        
        // Extract domain from URL
        let domain = if let Ok(parsed_url) = url::Url::parse(url) {
            parsed_url.host_str().unwrap_or("").to_string()
        } else {
            url.to_string()
        };
        
        // Check against domain patterns
        let domain_patterns = self.domain_patterns.read();
        domain_patterns.iter().any(|pattern| pattern.is_match(&domain))
    }
    
    /// Check if window title should be filtered
    fn should_filter_window_title(&self, event: &RawEvent, _config: &PrivacyFilterConfig) -> bool {
        let window_title = event.data.get("window_title")
            .or_else(|| event.data.get("title"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        
        if window_title.is_empty() {
            return false;
        }
        
        let title_lower = window_title.to_lowercase();
        
        // Check against window title patterns
        let window_title_patterns = self.window_title_patterns.read();
        window_title_patterns.iter().any(|pattern| pattern.is_match(&title_lower))
    }
    
    /// Check if event contains PII
    fn contains_pii(&self, event: &RawEvent, config: &PrivacyFilterConfig) -> bool {
        let text_fields = [
            event.data.get("text").and_then(|v| v.as_str()),
            event.data.get("content").and_then(|v| v.as_str()),
            event.data.get("value").and_then(|v| v.as_str()),
            event.data.get("clipboard").and_then(|v| v.as_str()),
        ];
        
        for text_field in text_fields.iter().flatten() {
            if text_field.len() > config.max_text_analysis_length {
                continue;
            }
            
            let pii_patterns = self.pii_patterns.read();
            for pattern in pii_patterns.iter() {
                if pattern.regex.is_match(text_field) && pattern.confidence >= config.pii_confidence_threshold {
                    debug!("PII detected: {} in text field", pattern.name);
                    return true;
                }
            }
        }
        
        false
    }
    
    /// Check if event contains credentials
    fn contains_credentials(&self, event: &RawEvent, config: &PrivacyFilterConfig) -> bool {
        let text_fields = [
            event.data.get("text").and_then(|v| v.as_str()),
            event.data.get("content").and_then(|v| v.as_str()),
            event.data.get("value").and_then(|v| v.as_str()),
        ];
        
        for text_field in text_fields.iter().flatten() {
            if text_field.len() > config.max_text_analysis_length {
                continue;
            }
            
            // Check for common credential patterns
            let credential_patterns = [
                r"password[\s=:]+[\w!@#$%^&*]+",
                r"token[\s=:]+[\w-]+",
                r"key[\s=:]+[\w-]+",
                r"secret[\s=:]+[\w-]+",
                r"api[_-]?key[\s=:]+[\w-]+",
                r"bearer[\s]+[\w.-]+",
            ];
            
            for pattern in credential_patterns.iter() {
                if let Ok(regex) = Regex::new(pattern) {
                    if regex.is_match(&text_field.to_lowercase()) {
                        debug!("Credential pattern detected: {}", pattern);
                        return true;
                    }
                }
            }
        }
        
        false
    }
    
    /// Sanitize event by removing sensitive data
    fn sanitize_event(&self, event: &RawEvent, config: &PrivacyFilterConfig) -> RawEvent {
        let mut sanitized_event = event.clone();
        
        // Remove or hash sensitive fields
        let sensitive_fields = ["password", "token", "key", "secret", "auth", "credential"];
        
        for field in sensitive_fields.iter() {
            if sanitized_event.data.get(field).is_some() {
                sanitized_event.data.as_object_mut().unwrap().insert(
                    field.to_string(),
                    serde_json::Value::String("[REDACTED]".to_string()),
                );
            }
        }
        
        // Truncate long text fields
        if let Some(text) = sanitized_event.data.get("text").and_then(|v| v.as_str()) {
            if text.len() > config.max_text_analysis_length {
                let truncated_text = text[..config.max_text_analysis_length].to_string();
                sanitized_event.data.as_object_mut().unwrap().insert(
                    "text".to_string(),
                    serde_json::Value::String(format!("{}...[TRUNCATED]", truncated_text)),
                );
            }
        }
        
        sanitized_event
    }
    
    /// Update configuration
    pub async fn update_config(&self, new_config: PrivacyFilterConfig) -> Result<(), AIError> {
        info!("Updating privacy filter configuration");
        
        *self.config.write() = new_config;
        
        // Recompile patterns
        self.compile_patterns().await?;
        
        info!("Privacy filter configuration updated");
        Ok(())
    }
    
    /// Compile regex patterns from configuration
    async fn compile_patterns(&self) -> Result<(), AIError> {
        let config = self.config.read();
        
        // Compile path patterns
        let mut path_patterns = Vec::new();
        for pattern in &config.excluded_paths {
            match Regex::new(pattern) {
                Ok(regex) => path_patterns.push(regex),
                Err(e) => log::warn!("Invalid path pattern '{}': {}", pattern, e),
            }
        }
        *self.path_patterns.write() = path_patterns;
        
        // Compile domain patterns
        let mut domain_patterns = Vec::new();
        for pattern in &config.excluded_domains {
            match Regex::new(pattern) {
                Ok(regex) => domain_patterns.push(regex),
                Err(e) => log::warn!("Invalid domain pattern '{}': {}", pattern, e),
            }
        }
        *self.domain_patterns.write() = domain_patterns;
        
        // Compile window title patterns
        let mut window_title_patterns = Vec::new();
        for pattern in &config.excluded_window_titles {
            match Regex::new(pattern) {
                Ok(regex) => window_title_patterns.push(regex),
                Err(e) => log::warn!("Invalid window title pattern '{}': {}", pattern, e),
            }
        }
        *self.window_title_patterns.write() = window_title_patterns;
        
        Ok(())
    }
    
    /// Initialize PII detection patterns
    async fn initialize_pii_patterns(&self) -> Result<(), AIError> {
        let mut patterns = Vec::new();
        
        // Email pattern
        if let Ok(regex) = Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}") {
            patterns.push(PIIPattern {
                name: "Email".to_string(),
                regex,
                confidence: 0.9,
                pattern_type: PIIType::Email,
            });
        }
        
        // Phone number pattern
        if let Ok(regex) = Regex::new(r"\b\d{3}[-.]?\d{3}[-.]?\d{4}\b") {
            patterns.push(PIIPattern {
                name: "Phone".to_string(),
                regex,
                confidence: 0.8,
                pattern_type: PIIType::Phone,
            });
        }
        
        // SSN pattern
        if let Ok(regex) = Regex::new(r"\b\d{3}-\d{2}-\d{4}\b") {
            patterns.push(PIIPattern {
                name: "SSN".to_string(),
                regex,
                confidence: 0.95,
                pattern_type: PIIType::SSN,
            });
        }
        
        // Credit card pattern
        if let Ok(regex) = Regex::new(r"\b\d{4}[\s-]?\d{4}[\s-]?\d{4}[\s-]?\d{4}\b") {
            patterns.push(PIIPattern {
                name: "Credit Card".to_string(),
                regex,
                confidence: 0.85,
                pattern_type: PIIType::CreditCard,
            });
        }
        
        // IP address pattern
        if let Ok(regex) = Regex::new(r"\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b") {
            patterns.push(PIIPattern {
                name: "IP Address".to_string(),
                regex,
                confidence: 0.7,
                pattern_type: PIIType::IPAddress,
            });
        }
        
        // API key pattern
        if let Ok(regex) = Regex::new(r"[a-zA-Z0-9]{32,}") {
            patterns.push(PIIPattern {
                name: "API Key".to_string(),
                regex,
                confidence: 0.6,
                pattern_type: PIIType::APIKey,
            });
        }
        
        *self.pii_patterns.write() = patterns;
        Ok(())
    }
    
    /// Initialize sensitive applications list
    async fn initialize_sensitive_apps(&self) -> Result<(), AIError> {
        let mut sensitive_apps = HashSet::new();
        
        // Password managers
        sensitive_apps.insert("1password".to_string());
        sensitive_apps.insert("keepassxc".to_string());
        sensitive_apps.insert("bitwarden".to_string());
        sensitive_apps.insert("lastpass".to_string());
        sensitive_apps.insert("dashlane".to_string());
        
        // Banking apps
        sensitive_apps.insert("mint".to_string());
        sensitive_apps.insert("quicken".to_string());
        sensitive_apps.insert("banking".to_string());
        
        // Privacy tools
        sensitive_apps.insert("tor".to_string());
        sensitive_apps.insert("vpn".to_string());
        sensitive_apps.insert("private".to_string());
        
        // System security
        sensitive_apps.insert("gnome-keyring".to_string());
        sensitive_apps.insert("kwallet".to_string());
        sensitive_apps.insert("seahorse".to_string());
        
        *self.sensitive_apps.write() = sensitive_apps;
        Ok(())
    }
    
    /// Get privacy filter statistics
    pub fn get_stats(&self) -> PrivacyFilterStats {
        self.stats.read().clone()
    }
    
    /// Get total number of compiled patterns
    fn get_pattern_count(&self) -> usize {
        self.path_patterns.read().len() +
        self.domain_patterns.read().len() +
        self.window_title_patterns.read().len() +
        self.pii_patterns.read().len()
    }
    
    /// Reset statistics
    pub fn reset_stats(&self) {
        *self.stats.write() = PrivacyFilterStats::default();
    }
    
    /// Add custom exclusion pattern
    pub async fn add_exclusion_pattern(&self, pattern_type: &str, pattern: &str) -> Result<(), AIError> {
        let mut config = self.config.write();
        
        match pattern_type {
            "path" => {
                config.excluded_paths.push(pattern.to_string());
            }
            "domain" => {
                config.excluded_domains.push(pattern.to_string());
            }
            "window_title" => {
                config.excluded_window_titles.push(pattern.to_string());
            }
            "application" => {
                config.excluded_applications.push(pattern.to_string());
            }
            _ => {
                return Err(AIError::Configuration(format!("Unknown pattern type: {}", pattern_type)));
            }
        }
        
        drop(config);
        
        // Recompile patterns
        self.compile_patterns().await?;
        
        info!("Added exclusion pattern: {} -> {}", pattern_type, pattern);
        Ok(())
    }
    
    /// Remove custom exclusion pattern
    pub async fn remove_exclusion_pattern(&self, pattern_type: &str, pattern: &str) -> Result<bool, AIError> {
        let mut config = self.config.write();
        let removed = match pattern_type {
            "path" => {
                let len_before = config.excluded_paths.len();
                config.excluded_paths.retain(|p| p != pattern);
                len_before != config.excluded_paths.len()
            }
            "domain" => {
                let len_before = config.excluded_domains.len();
                config.excluded_domains.retain(|p| p != pattern);
                len_before != config.excluded_domains.len()
            }
            "window_title" => {
                let len_before = config.excluded_window_titles.len();
                config.excluded_window_titles.retain(|p| p != pattern);
                len_before != config.excluded_window_titles.len()
            }
            "application" => {
                let len_before = config.excluded_applications.len();
                config.excluded_applications.retain(|p| p != pattern);
                len_before != config.excluded_applications.len()
            }
            _ => false,
        };
        
        drop(config);
        
        if removed {
            // Recompile patterns
            self.compile_patterns().await?;
            info!("Removed exclusion pattern: {} -> {}", pattern_type, pattern);
        }
        
        Ok(removed)
    }
}

impl Clone for PrivacyFilterStats {
    fn clone(&self) -> Self {
        Self {
            events_processed: self.events_processed,
            events_filtered: self.events_filtered,
            pii_detections: self.pii_detections,
            credential_detections: self.credential_detections,
            filter_reasons: self.filter_reasons.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monitoring::{RawEvent, EventSource};
    
    #[tokio::test]
    async fn test_privacy_filter_creation() {
        let config = PrivacyFilterConfig::default();
        let filter = PrivacyFilter::new(config).await.unwrap();
        
        assert!(filter.get_pattern_count() > 0);
    }
    
    #[tokio::test]
    async fn test_application_filtering() {
        let config = PrivacyFilterConfig::default();
        let filter = PrivacyFilter::new(config).await.unwrap();
        
        let event = RawEvent {
            source: EventSource::Application,
            timestamp: Utc::now(),
            data: serde_json::json!({
                "application": "1password",
                "action": "open"
            }),
            metadata: serde_json::json!({}),
        };
        
        let filtered = filter.filter_event(&event).await;
        assert!(filtered.is_none()); // Should be filtered out
    }
    
    #[tokio::test]
    async fn test_pii_detection() {
        let config = PrivacyFilterConfig::default();
        let filter = PrivacyFilter::new(config).await.unwrap();
        
        let event = RawEvent {
            source: EventSource::System,
            timestamp: Utc::now(),
            data: serde_json::json!({
                "text": "My email is john.doe@example.com",
                "action": "type"
            }),
            metadata: serde_json::json!({}),
        };
        
        let filtered = filter.filter_event(&event).await;
        assert!(filtered.is_none()); // Should be filtered out due to PII
    }
    
    #[tokio::test]
    async fn test_allowed_event() {
        let config = PrivacyFilterConfig::default();
        let filter = PrivacyFilter::new(config).await.unwrap();
        
        let event = RawEvent {
            source: EventSource::Application,
            timestamp: Utc::now(),
            data: serde_json::json!({
                "application": "firefox",
                "action": "open",
                "url": "https://example.com"
            }),
            metadata: serde_json::json!({}),
        };
        
        let filtered = filter.filter_event(&event).await;
        assert!(filtered.is_some()); // Should pass through
    }
}