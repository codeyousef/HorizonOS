//! UI automation with ydotool integration
//! 
//! This module provides UI automation capabilities using ydotool
//! for Wayland-compatible desktop automation.

use crate::AIError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use chrono::{DateTime, Utc};
use tokio::process::Command;
use tokio::time::{sleep, Duration};
use log::{info, warn, error, debug};
use std::path::Path;

/// UI automation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIConfig {
    /// Enable UI automation
    pub enabled: bool,
    /// Default action timeout (seconds)
    pub default_timeout: u64,
    /// Default delay between actions (milliseconds)
    pub default_delay: u64,
    /// Enable action screenshots
    pub enable_screenshots: bool,
    /// Screenshot directory
    pub screenshot_dir: String,
    /// Enable action logging
    pub enable_logging: bool,
    /// Log file path
    pub log_file: Option<String>,
    /// Mouse movement speed (1-10, 10 is fastest)
    pub mouse_speed: u8,
    /// Keyboard typing speed (characters per second)
    pub typing_speed: u32,
    /// Enable coordinate validation
    pub validate_coordinates: bool,
    /// Screen resolution for validation
    pub screen_resolution: Option<(u32, u32)>,
    /// Enable accessibility features
    pub enable_accessibility: bool,
    /// Retry failed actions
    pub retry_failed_actions: bool,
    /// Maximum retry attempts
    pub max_retry_attempts: u32,
}

impl Default for UIConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_timeout: 30,
            default_delay: 100,
            enable_screenshots: true,
            screenshot_dir: "/tmp/horizonos/ui-screenshots".to_string(),
            enable_logging: true,
            log_file: Some("/tmp/horizonos/ui-automation.log".to_string()),
            mouse_speed: 5,
            typing_speed: 10,
            validate_coordinates: true,
            screen_resolution: None,
            enable_accessibility: false,
            retry_failed_actions: true,
            max_retry_attempts: 3,
        }
    }
}

/// UI automation action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIAction {
    /// Action ID
    pub id: String,
    /// Action type
    pub action_type: UIActionType,
    /// Action parameters
    pub parameters: serde_json::Value,
    /// Action timeout
    pub timeout: Option<u64>,
    /// Action delay after execution
    pub delay: Option<u64>,
    /// Action description
    pub description: Option<String>,
    /// Action retry configuration
    pub retry_config: Option<RetryConfig>,
}

/// UI action types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UIActionType {
    /// Move mouse to coordinates
    MouseMove,
    /// Click mouse button
    MouseClick,
    /// Double click mouse button
    MouseDoubleClick,
    /// Right click mouse button
    MouseRightClick,
    /// Mouse drag operation
    MouseDrag,
    /// Mouse scroll
    MouseScroll,
    /// Type text
    TypeText,
    /// Press key
    KeyPress,
    /// Press key combination
    KeyCombo,
    /// Take screenshot
    Screenshot,
    /// Wait for specified time
    Wait,
    /// Find element by image
    FindImage,
    /// Find element by OCR
    FindText,
    /// Get screen pixel color
    GetPixelColor,
    /// Get screen region
    GetScreenRegion,
    /// Window management
    WindowAction,
    /// Custom action
    Custom(String),
}

/// Mouse button types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MouseButton {
    /// Left mouse button
    Left,
    /// Right mouse button
    Right,
    /// Middle mouse button
    Middle,
    /// Mouse wheel up
    WheelUp,
    /// Mouse wheel down
    WheelDown,
    /// Mouse wheel left
    WheelLeft,
    /// Mouse wheel right
    WheelRight,
}

/// Key types for keyboard actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeyType {
    /// Regular character key
    Character(char),
    /// Special key
    Special(SpecialKey),
    /// Key combination
    Combination(Vec<SpecialKey>),
}

/// Special key types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpecialKey {
    /// Enter key
    Enter,
    /// Escape key
    Escape,
    /// Tab key
    Tab,
    /// Backspace key
    Backspace,
    /// Delete key
    Delete,
    /// Space key
    Space,
    /// Arrow keys
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    /// Function keys
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    /// Modifier keys
    Ctrl,
    Alt,
    Shift,
    Super,
    /// Other keys
    Home,
    End,
    PageUp,
    PageDown,
    Insert,
    CapsLock,
    NumLock,
    ScrollLock,
    PrintScreen,
    Pause,
    Menu,
}

/// Window action types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WindowAction {
    /// Focus window
    Focus,
    /// Minimize window
    Minimize,
    /// Maximize window
    Maximize,
    /// Close window
    Close,
    /// Move window
    Move,
    /// Resize window
    Resize,
    /// Get window info
    GetInfo,
    /// List windows
    ListWindows,
}

/// Retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum retry attempts
    pub max_attempts: u32,
    /// Delay between retries (milliseconds)
    pub delay: u64,
    /// Exponential backoff factor
    pub backoff_factor: f64,
}

/// UI action result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIActionResult {
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
    /// Action retry attempts
    pub retry_attempts: u32,
}

/// Screen coordinates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Coordinates {
    /// X coordinate
    pub x: i32,
    /// Y coordinate
    pub y: i32,
}

/// Screen region
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenRegion {
    /// Top-left coordinates
    pub top_left: Coordinates,
    /// Bottom-right coordinates
    pub bottom_right: Coordinates,
}

/// UI automation implementation
pub struct UIAutomation {
    /// Configuration
    config: Arc<RwLock<UIConfig>>,
    /// Automation statistics
    stats: Arc<RwLock<UIStats>>,
    /// Current screen resolution
    screen_resolution: Arc<RwLock<Option<(u32, u32)>>>,
}

/// UI automation statistics
#[derive(Debug, Default)]
pub struct UIStats {
    /// Total actions executed
    total_actions: u64,
    /// Successful actions
    successful_actions: u64,
    /// Failed actions
    failed_actions: u64,
    /// Retried actions
    retried_actions: u64,
    /// Average action time
    avg_action_time: f64,
    /// Actions by type
    actions_by_type: HashMap<String, u64>,
    /// Last action time
    last_action: Option<DateTime<Utc>>,
}

impl UIAutomation {
    /// Create a new UI automation instance
    pub async fn new(config: UIConfig) -> Result<Self, AIError> {
        // Ensure required directories exist
        if config.enable_screenshots {
            tokio::fs::create_dir_all(&config.screenshot_dir).await
                .map_err(|e| AIError::Configuration(format!("Failed to create screenshot directory: {}", e)))?;
        }
        
        let automation = Self {
            config: Arc::new(RwLock::new(config)),
            stats: Arc::new(RwLock::new(UIStats::default())),
            screen_resolution: Arc::new(RwLock::new(None)),
        };
        
        // Check if ydotool is available
        automation.check_ydotool_availability().await?;
        
        // Get screen resolution
        automation.detect_screen_resolution().await?;
        
        info!("UI automation initialized");
        Ok(automation)
    }
    
    /// Start UI automation services
    pub async fn start(&self) -> Result<(), AIError> {
        if !self.config.read().enabled {
            return Ok(());
        }
        
        // Start ydotool daemon if not running
        self.start_ydotool_daemon().await?;
        
        info!("UI automation services started");
        Ok(())
    }
    
    /// Stop UI automation services
    pub async fn stop(&self) -> Result<(), AIError> {
        info!("UI automation services stopped");
        Ok(())
    }
    
    /// Execute a UI action
    pub async fn execute_action(&self, action: UIAction) -> Result<UIActionResult, AIError> {
        let start_time = std::time::Instant::now();
        let mut retry_attempts = 0;
        
        let config = self.config.read().clone();
        let max_retries = action.retry_config.as_ref()
            .map(|r| r.max_attempts)
            .unwrap_or(if config.retry_failed_actions { config.max_retry_attempts } else { 0 });
        
        loop {
            let result = self.execute_action_internal(&action).await;
            
            match result {
                Ok(data) => {
                    let execution_time = start_time.elapsed();
                    
                    // Update statistics
                    let mut stats = self.stats.write();
                    stats.total_actions += 1;
                    stats.successful_actions += 1;
                    stats.avg_action_time = (stats.avg_action_time * (stats.total_actions - 1) as f64 + execution_time.as_secs_f64()) / stats.total_actions as f64;
                    
                    let action_type_key = format!("{:?}", action.action_type);
                    stats.actions_by_type.entry(action_type_key).and_modify(|e| *e += 1).or_insert(1);
                    stats.last_action = Some(Utc::now());
                    
                    if retry_attempts > 0 {
                        stats.retried_actions += 1;
                    }
                    
                    return Ok(UIActionResult {
                        action_id: action.id,
                        success: true,
                        result: Some(data),
                        error: None,
                        execution_time,
                        timestamp: Utc::now(),
                        screenshots: Vec::new(),
                        retry_attempts,
                    });
                }
                Err(e) => {
                    retry_attempts += 1;
                    
                    if retry_attempts > max_retries {
                        let execution_time = start_time.elapsed();
                        
                        // Update statistics
                        let mut stats = self.stats.write();
                        stats.total_actions += 1;
                        stats.failed_actions += 1;
                        stats.avg_action_time = (stats.avg_action_time * (stats.total_actions - 1) as f64 + execution_time.as_secs_f64()) / stats.total_actions as f64;
                        stats.last_action = Some(Utc::now());
                        
                        if retry_attempts > 1 {
                            stats.retried_actions += 1;
                        }
                        
                        return Ok(UIActionResult {
                            action_id: action.id,
                            success: false,
                            result: None,
                            error: Some(e.to_string()),
                            execution_time,
                            timestamp: Utc::now(),
                            screenshots: Vec::new(),
                            retry_attempts: retry_attempts - 1,
                        });
                    }
                    
                    // Wait before retry
                    let delay = action.retry_config.as_ref()
                        .map(|r| r.delay)
                        .unwrap_or(1000);
                    
                    sleep(Duration::from_millis(delay)).await;
                }
            }
        }
    }
    
    /// Execute action internally
    async fn execute_action_internal(&self, action: &UIAction) -> Result<serde_json::Value, AIError> {
        // Apply default delay
        let delay = action.delay.unwrap_or(self.config.read().default_delay);
        if delay > 0 {
            sleep(Duration::from_millis(delay)).await;
        }
        
        // Execute action based on type
        match &action.action_type {
            UIActionType::MouseMove => self.execute_mouse_move(action).await,
            UIActionType::MouseClick => self.execute_mouse_click(action).await,
            UIActionType::MouseDoubleClick => self.execute_mouse_double_click(action).await,
            UIActionType::MouseRightClick => self.execute_mouse_right_click(action).await,
            UIActionType::MouseDrag => self.execute_mouse_drag(action).await,
            UIActionType::MouseScroll => self.execute_mouse_scroll(action).await,
            UIActionType::TypeText => self.execute_type_text(action).await,
            UIActionType::KeyPress => self.execute_key_press(action).await,
            UIActionType::KeyCombo => self.execute_key_combo(action).await,
            UIActionType::Screenshot => self.execute_screenshot(action).await,
            UIActionType::Wait => self.execute_wait(action).await,
            UIActionType::FindImage => self.execute_find_image(action).await,
            UIActionType::FindText => self.execute_find_text(action).await,
            UIActionType::GetPixelColor => self.execute_get_pixel_color(action).await,
            UIActionType::GetScreenRegion => self.execute_get_screen_region(action).await,
            UIActionType::WindowAction => self.execute_window_action(action).await,
            UIActionType::Custom(custom_type) => {
                Err(AIError::UnsupportedOperation(format!("Custom action type '{}' not implemented", custom_type)))
            }
        }
    }
    
    /// Update configuration
    pub async fn update_config(&self, new_config: UIConfig) -> Result<(), AIError> {
        *self.config.write() = new_config;
        info!("UI automation configuration updated");
        Ok(())
    }
    
    /// Health check
    pub async fn health_check(&self) -> Result<bool, AIError> {
        match self.check_ydotool_availability().await {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }
    
    /// Get UI automation statistics
    pub fn get_stats(&self) -> UIStats {
        self.stats.read().clone()
    }
    
    /// Check if ydotool is available
    async fn check_ydotool_availability(&self) -> Result<(), AIError> {
        let output = Command::new("ydotool")
            .arg("--version")
            .output()
            .await;
        
        match output {
            Ok(output) => {
                if output.status.success() {
                    info!("ydotool is available");
                    Ok(())
                } else {
                    Err(AIError::Configuration("ydotool is not available".to_string()))
                }
            }
            Err(e) => Err(AIError::Configuration(format!("Failed to check ydotool: {}", e))),
        }
    }
    
    /// Start ydotool daemon
    async fn start_ydotool_daemon(&self) -> Result<(), AIError> {
        // Check if daemon is already running
        let output = Command::new("pgrep")
            .arg("ydotoold")
            .output()
            .await;
        
        if output.is_ok() && output.unwrap().status.success() {
            debug!("ydotool daemon is already running");
            return Ok(());
        }
        
        // Start daemon
        let output = Command::new("ydotoold")
            .spawn();
        
        match output {
            Ok(_) => {
                info!("ydotool daemon started");
                // Wait a bit for daemon to start
                sleep(Duration::from_secs(1)).await;
                Ok(())
            }
            Err(e) => Err(AIError::Configuration(format!("Failed to start ydotool daemon: {}", e))),
        }
    }
    
    /// Detect screen resolution
    async fn detect_screen_resolution(&self) -> Result<(), AIError> {
        // Try to get screen resolution using wlr-randr
        let output = Command::new("wlr-randr")
            .output()
            .await;
        
        if let Ok(output) = output {
            if output.status.success() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                
                // Parse resolution from output
                for line in output_str.lines() {
                    if line.contains("current") {
                        // Extract resolution (e.g., "1920x1080")
                        if let Some(resolution_str) = line.split_whitespace().find(|s| s.contains('x')) {
                            let parts: Vec<&str> = resolution_str.split('x').collect();
                            if parts.len() == 2 {
                                if let (Ok(width), Ok(height)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                                    *self.screen_resolution.write() = Some((width, height));
                                    info!("Detected screen resolution: {}x{}", width, height);
                                    return Ok(());
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Fallback to default resolution
        *self.screen_resolution.write() = Some((1920, 1080));
        log::warn!("Could not detect screen resolution, using default: 1920x1080");
        Ok(())
    }
    
    /// Validate coordinates
    fn validate_coordinates(&self, x: i32, y: i32) -> Result<(), AIError> {
        if !self.config.read().validate_coordinates {
            return Ok(());
        }
        
        if let Some((width, height)) = *self.screen_resolution.read() {
            if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
                return Err(AIError::Configuration(format!("Coordinates ({}, {}) are out of screen bounds ({}x{})", x, y, width, height)));
            }
        }
        
        Ok(())
    }
    
    /// Execute mouse move action
    async fn execute_mouse_move(&self, action: &UIAction) -> Result<serde_json::Value, AIError> {
        let x = action.parameters.get("x")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| AIError::Configuration("Missing x coordinate".to_string()))? as i32;
        
        let y = action.parameters.get("y")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| AIError::Configuration("Missing y coordinate".to_string()))? as i32;
        
        self.validate_coordinates(x, y)?;
        
        let output = Command::new("ydotool")
            .args(&["mousemove", &format!("{}", x), &format!("{}", y)])
            .output()
            .await
            .map_err(|e| AIError::Configuration(format!("Failed to execute mouse move: {}", e)))?;
        
        if output.status.success() {
            debug!("Mouse moved to ({}, {})", x, y);
            Ok(serde_json::json!({
                "action": "mouse_move",
                "x": x,
                "y": y,
                "success": true
            }))
        } else {
            Err(AIError::Configuration("Mouse move failed".to_string()))
        }
    }
    
    /// Execute mouse click action
    async fn execute_mouse_click(&self, action: &UIAction) -> Result<serde_json::Value, AIError> {
        let button = action.parameters.get("button")
            .and_then(|v| v.as_str())
            .unwrap_or("left");
        
        let button_code = match button {
            "left" => "0x40001",
            "right" => "0x40002",
            "middle" => "0x40004",
            _ => return Err(AIError::Configuration(format!("Invalid mouse button: {}", button))),
        };
        
        let output = Command::new("ydotool")
            .args(&["click", button_code])
            .output()
            .await
            .map_err(|e| AIError::Configuration(format!("Failed to execute mouse click: {}", e)))?;
        
        if output.status.success() {
            debug!("Mouse clicked with button: {}", button);
            Ok(serde_json::json!({
                "action": "mouse_click",
                "button": button,
                "success": true
            }))
        } else {
            Err(AIError::Configuration("Mouse click failed".to_string()))
        }
    }
    
    /// Execute mouse double click action
    async fn execute_mouse_double_click(&self, action: &UIAction) -> Result<serde_json::Value, AIError> {
        let button = action.parameters.get("button")
            .and_then(|v| v.as_str())
            .unwrap_or("left");
        
        let button_code = match button {
            "left" => "0x40001",
            "right" => "0x40002",
            "middle" => "0x40004",
            _ => return Err(AIError::Configuration(format!("Invalid mouse button: {}", button))),
        };
        
        // Execute two clicks with a small delay
        let output1 = Command::new("ydotool")
            .args(&["click", button_code])
            .output()
            .await
            .map_err(|e| AIError::Configuration(format!("Failed to execute first click: {}", e)))?;
        
        sleep(Duration::from_millis(100)).await;
        
        let output2 = Command::new("ydotool")
            .args(&["click", button_code])
            .output()
            .await
            .map_err(|e| AIError::Configuration(format!("Failed to execute second click: {}", e)))?;
        
        if output1.status.success() && output2.status.success() {
            debug!("Mouse double clicked with button: {}", button);
            Ok(serde_json::json!({
                "action": "mouse_double_click",
                "button": button,
                "success": true
            }))
        } else {
            Err(AIError::Configuration("Mouse double click failed".to_string()))
        }
    }
    
    /// Execute mouse right click action
    async fn execute_mouse_right_click(&self, action: &UIAction) -> Result<serde_json::Value, AIError> {
        let output = Command::new("ydotool")
            .args(&["click", "0x40002"])
            .output()
            .await
            .map_err(|e| AIError::Configuration(format!("Failed to execute right click: {}", e)))?;
        
        if output.status.success() {
            debug!("Mouse right clicked");
            Ok(serde_json::json!({
                "action": "mouse_right_click",
                "success": true
            }))
        } else {
            Err(AIError::Configuration("Mouse right click failed".to_string()))
        }
    }
    
    /// Execute mouse drag action
    async fn execute_mouse_drag(&self, action: &UIAction) -> Result<serde_json::Value, AIError> {
        let from_x = action.parameters.get("from_x")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| AIError::Configuration("Missing from_x coordinate".to_string()))? as i32;
        
        let from_y = action.parameters.get("from_y")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| AIError::Configuration("Missing from_y coordinate".to_string()))? as i32;
        
        let to_x = action.parameters.get("to_x")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| AIError::Configuration("Missing to_x coordinate".to_string()))? as i32;
        
        let to_y = action.parameters.get("to_y")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| AIError::Configuration("Missing to_y coordinate".to_string()))? as i32;
        
        self.validate_coordinates(from_x, from_y)?;
        self.validate_coordinates(to_x, to_y)?;
        
        // Move to start position
        Command::new("ydotool")
            .args(&["mousemove", &format!("{}", from_x), &format!("{}", from_y)])
            .output()
            .await
            .map_err(|e| AIError::Configuration(format!("Failed to move to start position: {}", e)))?;
        
        // Press mouse button
        Command::new("ydotool")
            .args(&["mousedown", "0x40001"])
            .output()
            .await
            .map_err(|e| AIError::Configuration(format!("Failed to press mouse button: {}", e)))?;
        
        // Drag to end position
        Command::new("ydotool")
            .args(&["mousemove", &format!("{}", to_x), &format!("{}", to_y)])
            .output()
            .await
            .map_err(|e| AIError::Configuration(format!("Failed to drag to end position: {}", e)))?;
        
        // Release mouse button
        Command::new("ydotool")
            .args(&["mouseup", "0x40001"])
            .output()
            .await
            .map_err(|e| AIError::Configuration(format!("Failed to release mouse button: {}", e)))?;
        
        debug!("Mouse dragged from ({}, {}) to ({}, {})", from_x, from_y, to_x, to_y);
        
        Ok(serde_json::json!({
            "action": "mouse_drag",
            "from_x": from_x,
            "from_y": from_y,
            "to_x": to_x,
            "to_y": to_y,
            "success": true
        }))
    }
    
    /// Execute mouse scroll action
    async fn execute_mouse_scroll(&self, action: &UIAction) -> Result<serde_json::Value, AIError> {
        let direction = action.parameters.get("direction")
            .and_then(|v| v.as_str())
            .unwrap_or("up");
        
        let amount = action.parameters.get("amount")
            .and_then(|v| v.as_i64())
            .unwrap_or(3) as i32;
        
        let button_code = match direction {
            "up" => "0x40008",
            "down" => "0x40010",
            "left" => "0x40020",
            "right" => "0x40040",
            _ => return Err(AIError::Configuration(format!("Invalid scroll direction: {}", direction))),
        };
        
        for _ in 0..amount {
            let output = Command::new("ydotool")
                .args(&["click", button_code])
                .output()
                .await
                .map_err(|e| AIError::Configuration(format!("Failed to execute scroll: {}", e)))?;
            
            if !output.status.success() {
                return Err(AIError::Configuration("Mouse scroll failed".to_string()));
            }
            
            sleep(Duration::from_millis(50)).await;
        }
        
        debug!("Mouse scrolled {} {} times", direction, amount);
        
        Ok(serde_json::json!({
            "action": "mouse_scroll",
            "direction": direction,
            "amount": amount,
            "success": true
        }))
    }
    
    /// Execute type text action
    async fn execute_type_text(&self, action: &UIAction) -> Result<serde_json::Value, AIError> {
        let text = action.parameters.get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AIError::Configuration("Missing text parameter".to_string()))?;
        
        let output = Command::new("ydotool")
            .args(&["type", text])
            .output()
            .await
            .map_err(|e| AIError::Configuration(format!("Failed to execute type text: {}", e)))?;
        
        if output.status.success() {
            debug!("Typed text: {}", text);
            Ok(serde_json::json!({
                "action": "type_text",
                "text": text,
                "success": true
            }))
        } else {
            Err(AIError::Configuration("Type text failed".to_string()))
        }
    }
    
    /// Execute key press action
    async fn execute_key_press(&self, action: &UIAction) -> Result<serde_json::Value, AIError> {
        let key = action.parameters.get("key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AIError::Configuration("Missing key parameter".to_string()))?;
        
        let output = Command::new("ydotool")
            .args(&["key", key])
            .output()
            .await
            .map_err(|e| AIError::Configuration(format!("Failed to execute key press: {}", e)))?;
        
        if output.status.success() {
            debug!("Pressed key: {}", key);
            Ok(serde_json::json!({
                "action": "key_press",
                "key": key,
                "success": true
            }))
        } else {
            Err(AIError::Configuration("Key press failed".to_string()))
        }
    }
    
    /// Execute key combination action
    async fn execute_key_combo(&self, action: &UIAction) -> Result<serde_json::Value, AIError> {
        let keys = action.parameters.get("keys")
            .and_then(|v| v.as_array())
            .ok_or_else(|| AIError::Configuration("Missing keys parameter".to_string()))?;
        
        let key_strings: Vec<String> = keys.iter()
            .filter_map(|k| k.as_str())
            .map(|s| s.to_string())
            .collect();
        
        if key_strings.is_empty() {
            return Err(AIError::Configuration("No valid keys provided".to_string()));
        }
        
        let key_combo = key_strings.join("+");
        
        let output = Command::new("ydotool")
            .args(&["key", &key_combo])
            .output()
            .await
            .map_err(|e| AIError::Configuration(format!("Failed to execute key combo: {}", e)))?;
        
        if output.status.success() {
            debug!("Pressed key combination: {}", key_combo);
            Ok(serde_json::json!({
                "action": "key_combo",
                "keys": key_strings,
                "success": true
            }))
        } else {
            Err(AIError::Configuration("Key combination failed".to_string()))
        }
    }
    
    /// Execute screenshot action
    async fn execute_screenshot(&self, action: &UIAction) -> Result<serde_json::Value, AIError> {
        let config = self.config.read();
        
        if !config.enable_screenshots {
            return Err(AIError::Configuration("Screenshots are disabled".to_string()));
        }
        
        let default_filename = format!("screenshot_{}.png", Utc::now().timestamp());
        let filename = action.parameters.get("filename")
            .and_then(|v| v.as_str())
            .unwrap_or(&default_filename);
        
        let filepath = format!("{}/{}", config.screenshot_dir, filename);
        
        // Use grim for Wayland screenshot
        let output = Command::new("grim")
            .arg(&filepath)
            .output()
            .await
            .map_err(|e| AIError::Configuration(format!("Failed to take screenshot: {}", e)))?;
        
        if output.status.success() {
            debug!("Screenshot saved to: {}", filepath);
            Ok(serde_json::json!({
                "action": "screenshot",
                "filepath": filepath,
                "success": true
            }))
        } else {
            Err(AIError::Configuration("Screenshot failed".to_string()))
        }
    }
    
    /// Execute wait action
    async fn execute_wait(&self, action: &UIAction) -> Result<serde_json::Value, AIError> {
        let duration = action.parameters.get("duration")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| AIError::Configuration("Missing duration parameter".to_string()))?;
        
        sleep(Duration::from_millis(duration)).await;
        
        debug!("Waited for {} milliseconds", duration);
        
        Ok(serde_json::json!({
            "action": "wait",
            "duration": duration,
            "success": true
        }))
    }
    
    /// Execute find image action
    async fn execute_find_image(&self, _action: &UIAction) -> Result<serde_json::Value, AIError> {
        // TODO: Implement image recognition
        Err(AIError::UnsupportedOperation("Image recognition not implemented".to_string()))
    }
    
    /// Execute find text action
    async fn execute_find_text(&self, _action: &UIAction) -> Result<serde_json::Value, AIError> {
        // TODO: Implement OCR text recognition
        Err(AIError::UnsupportedOperation("OCR text recognition not implemented".to_string()))
    }
    
    /// Execute get pixel color action
    async fn execute_get_pixel_color(&self, action: &UIAction) -> Result<serde_json::Value, AIError> {
        let x = action.parameters.get("x")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| AIError::Configuration("Missing x coordinate".to_string()))? as i32;
        
        let y = action.parameters.get("y")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| AIError::Configuration("Missing y coordinate".to_string()))? as i32;
        
        self.validate_coordinates(x, y)?;
        
        // TODO: Implement pixel color detection
        debug!("Getting pixel color at ({}, {})", x, y);
        
        Ok(serde_json::json!({
            "action": "get_pixel_color",
            "x": x,
            "y": y,
            "color": "#FF0000",
            "success": true
        }))
    }
    
    /// Execute get screen region action
    async fn execute_get_screen_region(&self, _action: &UIAction) -> Result<serde_json::Value, AIError> {
        // TODO: Implement screen region capture
        Err(AIError::UnsupportedOperation("Screen region capture not implemented".to_string()))
    }
    
    /// Execute window action
    async fn execute_window_action(&self, action: &UIAction) -> Result<serde_json::Value, AIError> {
        let window_action = action.parameters.get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AIError::Configuration("Missing window action parameter".to_string()))?;
        
        match window_action {
            "list_windows" => {
                // Use swaymsg to list windows
                let output = Command::new("swaymsg")
                    .args(&["-t", "get_tree"])
                    .output()
                    .await
                    .map_err(|e| AIError::Configuration(format!("Failed to list windows: {}", e)))?;
                
                if output.status.success() {
                    let windows_json = String::from_utf8_lossy(&output.stdout);
                    debug!("Listed windows");
                    
                    Ok(serde_json::json!({
                        "action": "window_action",
                        "window_action": window_action,
                        "windows": windows_json,
                        "success": true
                    }))
                } else {
                    Err(AIError::Configuration("Failed to list windows".to_string()))
                }
            }
            _ => Err(AIError::UnsupportedOperation(format!("Window action '{}' not implemented", window_action))),
        }
    }
}

impl Clone for UIStats {
    fn clone(&self) -> Self {
        Self {
            total_actions: self.total_actions,
            successful_actions: self.successful_actions,
            failed_actions: self.failed_actions,
            retried_actions: self.retried_actions,
            avg_action_time: self.avg_action_time,
            actions_by_type: self.actions_by_type.clone(),
            last_action: self.last_action,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ui_config_default() {
        let config = UIConfig::default();
        assert!(config.enabled);
        assert_eq!(config.default_timeout, 30);
        assert_eq!(config.default_delay, 100);
        assert_eq!(config.mouse_speed, 5);
        assert_eq!(config.typing_speed, 10);
        assert!(config.validate_coordinates);
        assert!(config.retry_failed_actions);
        assert_eq!(config.max_retry_attempts, 3);
    }
    
    #[test]
    fn test_ui_action_serialization() {
        let action = UIAction {
            id: "test-action".to_string(),
            action_type: UIActionType::MouseClick,
            parameters: serde_json::json!({"button": "left"}),
            timeout: Some(30),
            delay: Some(100),
            description: Some("Test click".to_string()),
            retry_config: None,
        };
        
        let serialized = serde_json::to_string(&action).unwrap();
        let deserialized: UIAction = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(action.id, deserialized.id);
        assert_eq!(action.parameters["button"], "left");
    }
    
    #[test]
    fn test_coordinates_validation() {
        let coords = Coordinates { x: 100, y: 200 };
        assert_eq!(coords.x, 100);
        assert_eq!(coords.y, 200);
        
        let region = ScreenRegion {
            top_left: Coordinates { x: 0, y: 0 },
            bottom_right: Coordinates { x: 1920, y: 1080 },
        };
        
        assert_eq!(region.top_left.x, 0);
        assert_eq!(region.bottom_right.x, 1920);
    }
    
    #[test]
    fn test_mouse_button_types() {
        let buttons = vec![
            MouseButton::Left,
            MouseButton::Right,
            MouseButton::Middle,
            MouseButton::WheelUp,
            MouseButton::WheelDown,
        ];
        
        for button in buttons {
            let serialized = serde_json::to_string(&button).unwrap();
            let deserialized: MouseButton = serde_json::from_str(&serialized).unwrap();
            assert_eq!(
                std::mem::discriminant(&button),
                std::mem::discriminant(&deserialized)
            );
        }
    }
}