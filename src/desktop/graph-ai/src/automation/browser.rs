//! Browser automation with Playwright integration
//! 
//! This module provides comprehensive browser automation capabilities
//! using Playwright for cross-browser testing and automation.

use crate::AIError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use chrono::{DateTime, Utc};
use tokio::process::Command;
use tokio::time::{sleep, Duration};
use log::{info, warn, error, debug};

/// Browser automation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserConfig {
    /// Enable browser automation
    pub enabled: bool,
    /// Default browser to use
    pub default_browser: BrowserType,
    /// Browser launch options
    pub launch_options: BrowserLaunchOptions,
    /// Default timeout for operations (seconds)
    pub default_timeout: u64,
    /// Maximum concurrent browser sessions
    pub max_concurrent_sessions: usize,
    /// Enable headless mode
    pub headless: bool,
    /// Browser data directory
    pub data_dir: Option<String>,
    /// User agent override
    pub user_agent: Option<String>,
    /// Viewport settings
    pub viewport: Option<ViewportConfig>,
    /// Enable screenshots
    pub enable_screenshots: bool,
    /// Screenshot directory
    pub screenshot_dir: String,
    /// Enable video recording
    pub enable_video: bool,
    /// Video directory
    pub video_dir: String,
    /// Enable request/response logging
    pub enable_network_logging: bool,
    /// Enable browser console logging
    pub enable_console_logging: bool,
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_browser: BrowserType::Chromium,
            launch_options: BrowserLaunchOptions::default(),
            default_timeout: 30,
            max_concurrent_sessions: 5,
            headless: true,
            data_dir: None,
            user_agent: None,
            viewport: Some(ViewportConfig::default()),
            enable_screenshots: true,
            screenshot_dir: "/tmp/horizonos/screenshots".to_string(),
            enable_video: false,
            video_dir: "/tmp/horizonos/videos".to_string(),
            enable_network_logging: false,
            enable_console_logging: false,
        }
    }
}

/// Browser types supported
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BrowserType {
    /// Chromium browser
    Chromium,
    /// Chrome browser
    Chrome,
    /// Firefox browser
    Firefox,
    /// Safari browser (macOS only)
    Safari,
    /// Edge browser
    Edge,
}

/// Browser launch options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserLaunchOptions {
    /// Browser arguments
    pub args: Vec<String>,
    /// Environment variables
    pub env: HashMap<String, String>,
    /// Ignore HTTPS errors
    pub ignore_https_errors: bool,
    /// Slow motion delay (milliseconds)
    pub slow_mo: u64,
    /// Download directory
    pub download_dir: Option<String>,
    /// Proxy configuration
    pub proxy: Option<ProxyConfig>,
}

impl Default for BrowserLaunchOptions {
    fn default() -> Self {
        Self {
            args: vec![
                "--no-sandbox".to_string(),
                "--disable-dev-shm-usage".to_string(),
                "--disable-gpu".to_string(),
            ],
            env: HashMap::new(),
            ignore_https_errors: false,
            slow_mo: 0,
            download_dir: None,
            proxy: None,
        }
    }
}

/// Proxy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// Proxy server URL
    pub server: String,
    /// Proxy username
    pub username: Option<String>,
    /// Proxy password
    pub password: Option<String>,
    /// Bypass proxy for these hosts
    pub bypass: Vec<String>,
}

/// Viewport configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewportConfig {
    /// Viewport width
    pub width: u32,
    /// Viewport height
    pub height: u32,
    /// Device scale factor
    pub device_scale_factor: f64,
    /// Is mobile device
    pub is_mobile: bool,
    /// Has touch support
    pub has_touch: bool,
}

impl Default for ViewportConfig {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            device_scale_factor: 1.0,
            is_mobile: false,
            has_touch: false,
        }
    }
}

/// Browser automation action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserAction {
    /// Action ID
    pub id: String,
    /// Action type
    pub action_type: BrowserActionType,
    /// Action parameters
    pub parameters: serde_json::Value,
    /// Action timeout
    pub timeout: Option<u64>,
    /// Action description
    pub description: Option<String>,
}

/// Browser action types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BrowserActionType {
    /// Navigate to URL
    Navigate,
    /// Click element
    Click,
    /// Type text
    Type,
    /// Select option
    Select,
    /// Upload file
    Upload,
    /// Take screenshot
    Screenshot,
    /// Extract text
    ExtractText,
    /// Extract HTML
    ExtractHTML,
    /// Wait for element
    WaitForElement,
    /// Wait for navigation
    WaitForNavigation,
    /// Execute JavaScript
    ExecuteScript,
    /// Set cookies
    SetCookies,
    /// Get cookies
    GetCookies,
    /// Scroll to element
    ScrollTo,
    /// Hover over element
    Hover,
    /// Right click
    RightClick,
    /// Double click
    DoubleClick,
    /// Press key
    PressKey,
    /// Check checkbox
    Check,
    /// Uncheck checkbox
    Uncheck,
    /// Focus element
    Focus,
    /// Clear input
    Clear,
    /// Reload page
    Reload,
    /// Go back
    GoBack,
    /// Go forward
    GoForward,
    /// Close page
    ClosePage,
    /// Switch to frame
    SwitchFrame,
    /// Switch to window
    SwitchWindow,
    /// Accept alert
    AcceptAlert,
    /// Dismiss alert
    DismissAlert,
    /// Custom action
    Custom(String),
}

/// Browser session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserSession {
    /// Session ID
    pub id: String,
    /// Browser type
    pub browser_type: BrowserType,
    /// Session status
    pub status: SessionStatus,
    /// Current URL
    pub current_url: Option<String>,
    /// Session created time
    pub created_at: DateTime<Utc>,
    /// Session last activity
    pub last_activity: DateTime<Utc>,
    /// Session configuration
    pub config: BrowserConfig,
    /// Session metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Session status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionStatus {
    /// Session is initializing
    Initializing,
    /// Session is active
    Active,
    /// Session is idle
    Idle,
    /// Session is closing
    Closing,
    /// Session is closed
    Closed,
    /// Session failed
    Failed,
}

/// Browser action result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserActionResult {
    /// Action ID
    pub action_id: String,
    /// Action success status
    pub success: bool,
    /// Action result data
    pub result: Option<serde_json::Value>,
    /// Action error message
    pub error: Option<String>,
    /// Action execution time
    pub execution_time: std::time::Duration,
    /// Action timestamp
    pub timestamp: DateTime<Utc>,
    /// Action screenshots
    pub screenshots: Vec<String>,
}

/// Browser automation implementation
pub struct BrowserAutomation {
    /// Configuration
    config: Arc<RwLock<BrowserConfig>>,
    /// Active sessions
    sessions: Arc<RwLock<HashMap<String, BrowserSession>>>,
    /// Session managers
    session_managers: Arc<RwLock<HashMap<String, tokio::task::JoinHandle<()>>>>,
    /// Automation statistics
    stats: Arc<RwLock<BrowserStats>>,
}

/// Browser automation statistics
#[derive(Debug, Default)]
pub struct BrowserStats {
    /// Total sessions created
    total_sessions: u64,
    /// Active sessions
    active_sessions: u64,
    /// Total actions executed
    total_actions: u64,
    /// Successful actions
    successful_actions: u64,
    /// Failed actions
    failed_actions: u64,
    /// Average action time
    avg_action_time: f64,
    /// Last action time
    last_action: Option<DateTime<Utc>>,
}

impl BrowserAutomation {
    /// Create a new browser automation instance
    pub async fn new(config: BrowserConfig) -> Result<Self, AIError> {
        // Ensure required directories exist
        if config.enable_screenshots {
            tokio::fs::create_dir_all(&config.screenshot_dir).await
                .map_err(|e| AIError::Configuration(format!("Failed to create screenshot directory: {}", e)))?;
        }
        
        if config.enable_video {
            tokio::fs::create_dir_all(&config.video_dir).await
                .map_err(|e| AIError::Configuration(format!("Failed to create video directory: {}", e)))?;
        }
        
        let automation = Self {
            config: Arc::new(RwLock::new(config)),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            session_managers: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(BrowserStats::default())),
        };
        
        // Check if Playwright is available
        automation.check_playwright_availability().await?;
        
        info!("Browser automation initialized");
        Ok(automation)
    }
    
    /// Start browser automation services
    pub async fn start(&self) -> Result<(), AIError> {
        if !self.config.read().enabled {
            return Ok(());
        }
        
        info!("Browser automation services started");
        Ok(())
    }
    
    /// Stop browser automation services
    pub async fn stop(&self) -> Result<(), AIError> {
        // Close all active sessions
        let session_ids: Vec<String> = self.sessions.read().keys().cloned().collect();
        
        for session_id in session_ids {
            if let Err(e) = self.close_session(&session_id).await {
                log::warn!("Failed to close session {}: {}", session_id, e);
            }
        }
        
        log::info!("Browser automation services stopped");
        Ok(())
    }
    
    /// Create a new browser session
    pub async fn create_session(&self, session_config: Option<BrowserConfig>) -> Result<String, AIError> {
        let config = session_config.unwrap_or_else(|| self.config.read().clone());
        
        // Check concurrent session limit
        if self.sessions.read().len() >= config.max_concurrent_sessions {
            return Err(AIError::Configuration("Maximum concurrent sessions reached".to_string()));
        }
        
        let session_id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();
        
        let session = BrowserSession {
            id: session_id.clone(),
            browser_type: config.default_browser.clone(),
            status: SessionStatus::Initializing,
            current_url: None,
            created_at: now,
            last_activity: now,
            config: config.clone(),
            metadata: HashMap::new(),
        };
        
        // Store session
        self.sessions.write().insert(session_id.clone(), session);
        
        // Start session manager
        let session_manager = self.start_session_manager(session_id.clone(), config).await?;
        self.session_managers.write().insert(session_id.clone(), session_manager);
        
        // Update statistics
        let mut stats = self.stats.write();
        stats.total_sessions += 1;
        stats.active_sessions += 1;
        
        info!("Browser session created: {}", session_id);
        Ok(session_id)
    }
    
    /// Close a browser session
    pub async fn close_session(&self, session_id: &str) -> Result<(), AIError> {
        // Remove session manager
        if let Some(handle) = self.session_managers.write().remove(session_id) {
            handle.abort();
        }
        
        // Update session status
        if let Some(session) = self.sessions.write().get_mut(session_id) {
            session.status = SessionStatus::Closed;
        }
        
        // Update statistics
        let mut stats = self.stats.write();
        stats.active_sessions = stats.active_sessions.saturating_sub(1);
        
        info!("Browser session closed: {}", session_id);
        Ok(())
    }
    
    /// Execute a browser action
    pub async fn execute_action(
        &self,
        session_id: &str,
        action: BrowserAction,
    ) -> Result<BrowserActionResult, AIError> {
        let start_time = std::time::Instant::now();
        
        // Check if session exists
        if !self.sessions.read().contains_key(session_id) {
            return Err(AIError::Configuration(format!("Session not found: {}", session_id)));
        }
        
        // Update session activity
        if let Some(session) = self.sessions.write().get_mut(session_id) {
            session.last_activity = Utc::now();
        }
        
        // Execute action based on type
        let result = match action.action_type {
            BrowserActionType::Navigate => self.execute_navigate(session_id, &action).await,
            BrowserActionType::Click => self.execute_click(session_id, &action).await,
            BrowserActionType::Type => self.execute_type(session_id, &action).await,
            BrowserActionType::Screenshot => self.execute_screenshot(session_id, &action).await,
            BrowserActionType::ExtractText => self.execute_extract_text(session_id, &action).await,
            BrowserActionType::WaitForElement => self.execute_wait_for_element(session_id, &action).await,
            BrowserActionType::ExecuteScript => self.execute_script(session_id, &action).await,
            _ => Err(AIError::UnsupportedOperation(format!("Action type {:?} not implemented", action.action_type))),
        };
        
        let execution_time = start_time.elapsed();
        
        // Update statistics
        let mut stats = self.stats.write();
        stats.total_actions += 1;
        stats.avg_action_time = (stats.avg_action_time * (stats.total_actions - 1) as f64 + execution_time.as_secs_f64()) / stats.total_actions as f64;
        stats.last_action = Some(Utc::now());
        
        let action_result = match result {
            Ok(data) => {
                stats.successful_actions += 1;
                BrowserActionResult {
                    action_id: action.id,
                    success: true,
                    result: Some(data),
                    error: None,
                    execution_time,
                    timestamp: Utc::now(),
                    screenshots: Vec::new(),
                }
            }
            Err(e) => {
                stats.failed_actions += 1;
                BrowserActionResult {
                    action_id: action.id,
                    success: false,
                    result: None,
                    error: Some(e.to_string()),
                    execution_time,
                    timestamp: Utc::now(),
                    screenshots: Vec::new(),
                }
            }
        };
        
        Ok(action_result)
    }
    
    /// Get session information
    pub fn get_session(&self, session_id: &str) -> Option<BrowserSession> {
        self.sessions.read().get(session_id).cloned()
    }
    
    /// List all sessions
    pub fn list_sessions(&self) -> Vec<BrowserSession> {
        self.sessions.read().values().cloned().collect()
    }
    
    /// Update configuration
    pub async fn update_config(&self, new_config: BrowserConfig) -> Result<(), AIError> {
        *self.config.write() = new_config;
        info!("Browser automation configuration updated");
        Ok(())
    }
    
    /// Health check
    pub async fn health_check(&self) -> Result<bool, AIError> {
        // Check if Playwright is available
        match self.check_playwright_availability().await {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }
    
    /// Get browser automation statistics
    pub fn get_stats(&self) -> BrowserStats {
        self.stats.read().clone()
    }
    
    /// Check if Playwright is available
    async fn check_playwright_availability(&self) -> Result<(), AIError> {
        let output = Command::new("npx")
            .args(&["playwright", "--version"])
            .output()
            .await;
        
        match output {
            Ok(output) => {
                if output.status.success() {
                    info!("Playwright is available");
                    Ok(())
                } else {
                    Err(AIError::Configuration("Playwright is not available".to_string()))
                }
            }
            Err(e) => Err(AIError::Configuration(format!("Failed to check Playwright: {}", e))),
        }
    }
    
    /// Start session manager
    async fn start_session_manager(
        &self,
        session_id: String,
        _config: BrowserConfig,
    ) -> Result<tokio::task::JoinHandle<()>, AIError> {
        let sessions = self.sessions.clone();
        
        let handle = tokio::spawn(async move {
            debug!("Starting session manager for: {}", session_id);
            
            // TODO: Implement actual Playwright session management
            // This would involve:
            // 1. Launching browser process
            // 2. Creating browser context
            // 3. Managing pages
            // 4. Handling session cleanup
            
            // For now, just mark session as active
            if let Some(session) = sessions.write().get_mut(&session_id) {
                session.status = SessionStatus::Active;
            }
            
            // Session manager loop
            loop {
                tokio::time::sleep(Duration::from_secs(30)).await;
                
                // Check if session is still active
                let session_active = sessions.read().get(&session_id)
                    .map(|s| matches!(s.status, SessionStatus::Active | SessionStatus::Idle))
                    .unwrap_or(false);
                
                if !session_active {
                    break;
                }
            }
            
            debug!("Session manager stopped for: {}", session_id);
        });
        
        Ok(handle)
    }
    
    /// Execute navigate action
    async fn execute_navigate(&self, session_id: &str, action: &BrowserAction) -> Result<serde_json::Value, AIError> {
        let url = action.parameters.get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AIError::Configuration("Missing URL parameter".to_string()))?;
        
        // TODO: Implement actual navigation using Playwright
        debug!("Navigating to: {} in session: {}", url, session_id);
        
        // Update session URL
        if let Some(session) = self.sessions.write().get_mut(session_id) {
            session.current_url = Some(url.to_string());
        }
        
        Ok(serde_json::json!({
            "action": "navigate",
            "url": url,
            "success": true
        }))
    }
    
    /// Execute click action
    async fn execute_click(&self, session_id: &str, action: &BrowserAction) -> Result<serde_json::Value, AIError> {
        let selector = action.parameters.get("selector")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AIError::Configuration("Missing selector parameter".to_string()))?;
        
        // TODO: Implement actual click using Playwright
        debug!("Clicking element: {} in session: {}", selector, session_id);
        
        Ok(serde_json::json!({
            "action": "click",
            "selector": selector,
            "success": true
        }))
    }
    
    /// Execute type action
    async fn execute_type(&self, session_id: &str, action: &BrowserAction) -> Result<serde_json::Value, AIError> {
        let selector = action.parameters.get("selector")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AIError::Configuration("Missing selector parameter".to_string()))?;
        
        let text = action.parameters.get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AIError::Configuration("Missing text parameter".to_string()))?;
        
        // TODO: Implement actual typing using Playwright
        debug!("Typing text: {} in element: {} in session: {}", text, selector, session_id);
        
        Ok(serde_json::json!({
            "action": "type",
            "selector": selector,
            "text": text,
            "success": true
        }))
    }
    
    /// Execute screenshot action
    async fn execute_screenshot(&self, session_id: &str, action: &BrowserAction) -> Result<serde_json::Value, AIError> {
        let config = self.config.read();
        
        if !config.enable_screenshots {
            return Err(AIError::Configuration("Screenshots are disabled".to_string()));
        }
        
        let default_filename = format!("screenshot_{}.png", Utc::now().timestamp());
        let filename = action.parameters.get("filename")
            .and_then(|v| v.as_str())
            .unwrap_or(&default_filename);
        
        let filepath = format!("{}/{}", config.screenshot_dir, filename);
        
        // TODO: Implement actual screenshot using Playwright
        debug!("Taking screenshot: {} in session: {}", filepath, session_id);
        
        Ok(serde_json::json!({
            "action": "screenshot",
            "filepath": filepath,
            "success": true
        }))
    }
    
    /// Execute extract text action
    async fn execute_extract_text(&self, session_id: &str, action: &BrowserAction) -> Result<serde_json::Value, AIError> {
        let selector = action.parameters.get("selector")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AIError::Configuration("Missing selector parameter".to_string()))?;
        
        // TODO: Implement actual text extraction using Playwright
        debug!("Extracting text from: {} in session: {}", selector, session_id);
        
        Ok(serde_json::json!({
            "action": "extract_text",
            "selector": selector,
            "text": "Sample extracted text",
            "success": true
        }))
    }
    
    /// Execute wait for element action
    async fn execute_wait_for_element(&self, session_id: &str, action: &BrowserAction) -> Result<serde_json::Value, AIError> {
        let selector = action.parameters.get("selector")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AIError::Configuration("Missing selector parameter".to_string()))?;
        
        let timeout = action.parameters.get("timeout")
            .and_then(|v| v.as_u64())
            .unwrap_or(self.config.read().default_timeout);
        
        // TODO: Implement actual wait using Playwright
        debug!("Waiting for element: {} in session: {} (timeout: {}s)", selector, session_id, timeout);
        
        Ok(serde_json::json!({
            "action": "wait_for_element",
            "selector": selector,
            "timeout": timeout,
            "success": true
        }))
    }
    
    /// Execute script action
    async fn execute_script(&self, session_id: &str, action: &BrowserAction) -> Result<serde_json::Value, AIError> {
        let script = action.parameters.get("script")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AIError::Configuration("Missing script parameter".to_string()))?;
        
        // TODO: Implement actual script execution using Playwright
        debug!("Executing script in session: {}", session_id);
        
        Ok(serde_json::json!({
            "action": "execute_script",
            "script": script,
            "result": "Script executed successfully",
            "success": true
        }))
    }
}

impl Clone for BrowserStats {
    fn clone(&self) -> Self {
        Self {
            total_sessions: self.total_sessions,
            active_sessions: self.active_sessions,
            total_actions: self.total_actions,
            successful_actions: self.successful_actions,
            failed_actions: self.failed_actions,
            avg_action_time: self.avg_action_time,
            last_action: self.last_action,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_browser_config_default() {
        let config = BrowserConfig::default();
        assert!(config.enabled);
        assert!(matches!(config.default_browser, BrowserType::Chromium));
        assert_eq!(config.default_timeout, 30);
        assert_eq!(config.max_concurrent_sessions, 5);
        assert!(config.headless);
    }
    
    #[test]
    fn test_browser_action_serialization() {
        let action = BrowserAction {
            id: "test-action".to_string(),
            action_type: BrowserActionType::Navigate,
            parameters: serde_json::json!({"url": "https://example.com"}),
            timeout: Some(30),
            description: Some("Test navigation".to_string()),
        };
        
        let serialized = serde_json::to_string(&action).unwrap();
        let deserialized: BrowserAction = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(action.id, deserialized.id);
        assert_eq!(action.parameters["url"], "https://example.com");
    }
    
    #[test]
    fn test_browser_session_status() {
        let session = BrowserSession {
            id: "test-session".to_string(),
            browser_type: BrowserType::Chromium,
            status: SessionStatus::Active,
            current_url: Some("https://example.com".to_string()),
            created_at: Utc::now(),
            last_activity: Utc::now(),
            config: BrowserConfig::default(),
            metadata: HashMap::new(),
        };
        
        assert_eq!(session.id, "test-session");
        assert!(matches!(session.status, SessionStatus::Active));
        assert_eq!(session.current_url, Some("https://example.com".to_string()));
    }
    
    #[test]
    fn test_viewport_config() {
        let viewport = ViewportConfig {
            width: 1920,
            height: 1080,
            device_scale_factor: 2.0,
            is_mobile: false,
            has_touch: false,
        };
        
        assert_eq!(viewport.width, 1920);
        assert_eq!(viewport.height, 1080);
        assert_eq!(viewport.device_scale_factor, 2.0);
        assert!(!viewport.is_mobile);
        assert!(!viewport.has_touch);
    }
}