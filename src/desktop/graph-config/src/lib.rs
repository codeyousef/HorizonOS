//! Configuration and theming system for the graph desktop
//! 
//! Provides runtime configuration management, theme loading,
//! and hot-reloading capabilities for the desktop environment.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};
use tokio::sync::watch;
use anyhow::Result;

pub mod theme;
pub mod loader;
pub mod watcher;
pub mod validation;

use theme::{Theme, ThemeManager};
use loader::ConfigLoader;
use watcher::ConfigWatcher;
use validation::ConfigValidator;

/// Main configuration manager
pub struct ConfigManager {
    /// Current configuration
    config: Arc<RwLock<GraphDesktopConfig>>,
    /// Theme manager
    theme_manager: ThemeManager,
    /// Configuration loader
    loader: ConfigLoader,
    /// Configuration watcher for hot-reload
    watcher: Option<ConfigWatcher>,
    /// Configuration change notifier
    change_tx: watch::Sender<ConfigChangeEvent>,
    /// Configuration validator
    validator: ConfigValidator,
}

impl ConfigManager {
    /// Create a new configuration manager
    pub fn new() -> (Self, watch::Receiver<ConfigChangeEvent>) {
        let default_config = GraphDesktopConfig::default();
        let (change_tx, change_rx) = watch::channel(ConfigChangeEvent::Initialized);
        
        let manager = Self {
            config: Arc::new(RwLock::new(default_config)),
            theme_manager: ThemeManager::new(),
            loader: ConfigLoader::new(),
            watcher: None,
            change_tx,
            validator: ConfigValidator::new(),
        };
        
        (manager, change_rx)
    }
    
    /// Initialize from configuration files
    pub async fn initialize(&mut self, config_dir: &Path) -> Result<()> {
        // Load main configuration
        let config_path = config_dir.join("graph-desktop.toml");
        if config_path.exists() {
            let config = self.loader.load_config(&config_path).await?;
            
            // Validate configuration
            self.validator.validate(&config)?;
            
            *self.config.write().unwrap() = config;
            self.change_tx.send(ConfigChangeEvent::ConfigReloaded)?;
        }
        
        // Load themes
        let themes_dir = config_dir.join("themes");
        if themes_dir.exists() {
            self.theme_manager.load_themes(&themes_dir).await?;
        }
        
        // Set up file watcher for hot-reload
        self.setup_watcher(config_dir)?;
        
        Ok(())
    }
    
    /// Set up configuration file watcher
    fn setup_watcher(&mut self, config_dir: &Path) -> Result<()> {
        let config = self.config.clone();
        let change_tx = self.change_tx.clone();
        let loader = self.loader.clone();
        let validator = self.validator.clone();
        
        let watcher = ConfigWatcher::new(config_dir, move |path| {
            // Handle configuration file changes
            let config = config.clone();
            let change_tx = change_tx.clone();
            let loader = loader.clone();
            let validator = validator.clone();
            
            tokio::spawn(async move {
                if let Ok(new_config) = loader.load_config(&path).await {
                    if validator.validate(&new_config).is_ok() {
                        *config.write().unwrap() = new_config;
                        let _ = change_tx.send(ConfigChangeEvent::ConfigReloaded);
                    }
                }
            });
        })?;
        
        self.watcher = Some(watcher);
        Ok(())
    }
    
    /// Get current configuration
    pub fn config(&self) -> GraphDesktopConfig {
        self.config.read().unwrap().clone()
    }
    
    /// Get specific configuration value
    pub fn get<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        let config = self.config.read().unwrap();
        config.custom.get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }
    
    /// Set configuration value
    pub fn set<T: Serialize>(&self, key: &str, value: T) -> Result<()> {
        let mut config = self.config.write().unwrap();
        config.custom.insert(key.to_string(), serde_json::to_value(value)?);
        self.change_tx.send(ConfigChangeEvent::ValueChanged(key.to_string()))?;
        Ok(())
    }
    
    /// Get current theme
    pub fn current_theme(&self) -> Theme {
        let config = self.config.read().unwrap();
        self.theme_manager.get_theme(&config.appearance.theme)
            .unwrap_or_else(|| self.theme_manager.default_theme())
    }
    
    /// Switch theme
    pub fn switch_theme(&mut self, theme_name: &str) -> Result<()> {
        if self.theme_manager.has_theme(theme_name) {
            self.config.write().unwrap().appearance.theme = theme_name.to_string();
            self.change_tx.send(ConfigChangeEvent::ThemeChanged(theme_name.to_string()))?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Theme '{}' not found", theme_name))
        }
    }
    
    /// Save current configuration
    pub async fn save(&self, path: &Path) -> Result<()> {
        let config = self.config.read().unwrap().clone();
        self.loader.save_config(&config, path).await
    }
}

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphDesktopConfig {
    /// General settings
    pub general: GeneralConfig,
    /// Appearance settings
    pub appearance: AppearanceConfig,
    /// Graph rendering settings
    pub graph: GraphConfig,
    /// Interaction settings
    pub interaction: InteractionConfig,
    /// Performance settings
    pub performance: PerformanceConfig,
    /// AI settings
    pub ai: AIConfig,
    /// Workspace settings
    pub workspace: WorkspaceConfig,
    /// Accessibility settings
    pub accessibility: AccessibilityConfig,
    /// Keyboard shortcuts
    pub shortcuts: HashMap<String, KeyboardShortcut>,
    /// Custom configuration values
    pub custom: HashMap<String, serde_json::Value>,
}

impl Default for GraphDesktopConfig {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            appearance: AppearanceConfig::default(),
            graph: GraphConfig::default(),
            interaction: InteractionConfig::default(),
            performance: PerformanceConfig::default(),
            ai: AIConfig::default(),
            workspace: WorkspaceConfig::default(),
            accessibility: AccessibilityConfig::default(),
            shortcuts: default_shortcuts(),
            custom: HashMap::new(),
        }
    }
}

/// General configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    /// Auto-start applications
    pub autostart: Vec<String>,
    /// Default terminal
    pub terminal: String,
    /// Default browser
    pub browser: String,
    /// Log level
    pub log_level: String,
    /// Enable debug mode
    pub debug: bool,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            autostart: vec![],
            terminal: "alacritty".to_string(),
            browser: "firefox".to_string(),
            log_level: "info".to_string(),
            debug: false,
        }
    }
}

/// Appearance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppearanceConfig {
    /// Current theme name
    pub theme: String,
    /// Icon theme
    pub icon_theme: String,
    /// Font settings
    pub fonts: FontConfig,
    /// Animation settings
    pub animations: AnimationConfig,
    /// Transparency settings
    pub transparency: TransparencyConfig,
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            theme: "horizon-dark".to_string(),
            icon_theme: "Papirus-Dark".to_string(),
            fonts: FontConfig::default(),
            animations: AnimationConfig::default(),
            transparency: TransparencyConfig::default(),
        }
    }
}

/// Font configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontConfig {
    /// UI font
    pub ui_font: String,
    /// UI font size
    pub ui_size: f32,
    /// Monospace font
    pub mono_font: String,
    /// Monospace font size
    pub mono_size: f32,
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            ui_font: "Inter".to_string(),
            ui_size: 11.0,
            mono_font: "JetBrains Mono".to_string(),
            mono_size: 10.0,
        }
    }
}

/// Animation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationConfig {
    /// Enable animations
    pub enabled: bool,
    /// Animation duration multiplier
    pub speed: f32,
    /// Enable spring animations
    pub spring_animations: bool,
    /// Spring stiffness
    pub spring_stiffness: f32,
    /// Spring damping
    pub spring_damping: f32,
}

impl Default for AnimationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            speed: 1.0,
            spring_animations: true,
            spring_stiffness: 300.0,
            spring_damping: 20.0,
        }
    }
}

/// Transparency configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransparencyConfig {
    /// Window transparency
    pub windows: f32,
    /// Panel transparency
    pub panels: f32,
    /// Blur radius
    pub blur_radius: f32,
}

impl Default for TransparencyConfig {
    fn default() -> Self {
        Self {
            windows: 0.95,
            panels: 0.85,
            blur_radius: 20.0,
        }
    }
}

/// Graph rendering configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphConfig {
    /// Default layout algorithm
    pub layout_algorithm: String,
    /// Node size
    pub node_size: f32,
    /// Edge thickness
    pub edge_thickness: f32,
    /// Show labels
    pub show_labels: bool,
    /// Label size
    pub label_size: f32,
    /// Physics enabled
    pub physics_enabled: bool,
    /// Physics settings
    pub physics: PhysicsConfig,
}

impl Default for GraphConfig {
    fn default() -> Self {
        Self {
            layout_algorithm: "force-directed".to_string(),
            node_size: 50.0,
            edge_thickness: 2.0,
            show_labels: true,
            label_size: 12.0,
            physics_enabled: true,
            physics: PhysicsConfig::default(),
        }
    }
}

/// Physics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicsConfig {
    /// Gravity strength
    pub gravity: f32,
    /// Repulsion strength
    pub repulsion: f32,
    /// Link strength
    pub link_strength: f32,
    /// Friction
    pub friction: f32,
}

impl Default for PhysicsConfig {
    fn default() -> Self {
        Self {
            gravity: 0.1,
            repulsion: 100.0,
            link_strength: 1.0,
            friction: 0.9,
        }
    }
}

/// Interaction configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionConfig {
    /// Mouse sensitivity
    pub mouse_sensitivity: f32,
    /// Scroll speed
    pub scroll_speed: f32,
    /// Double-click interval (ms)
    pub double_click_interval: u32,
    /// Drag threshold (pixels)
    pub drag_threshold: f32,
    /// Edge creation mode
    pub edge_creation_mode: EdgeCreationMode,
}

impl Default for InteractionConfig {
    fn default() -> Self {
        Self {
            mouse_sensitivity: 1.0,
            scroll_speed: 1.0,
            double_click_interval: 400,
            drag_threshold: 5.0,
            edge_creation_mode: EdgeCreationMode::DragFromNode,
        }
    }
}

/// Edge creation modes
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum EdgeCreationMode {
    DragFromNode,
    ClickTwoNodes,
    ContextMenu,
}

/// Performance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Enable GPU acceleration
    pub gpu_acceleration: bool,
    /// Maximum FPS
    pub max_fps: u32,
    /// Enable LOD
    pub level_of_detail: bool,
    /// LOD distance thresholds
    pub lod_distances: [f32; 3],
    /// Maximum nodes to render
    pub max_nodes: usize,
    /// Enable culling
    pub frustum_culling: bool,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            gpu_acceleration: true,
            max_fps: 60,
            level_of_detail: true,
            lod_distances: [100.0, 500.0, 1000.0],
            max_nodes: 10000,
            frustum_culling: true,
        }
    }
}

/// AI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIConfig {
    /// Enable AI features
    pub enabled: bool,
    /// Ollama endpoint
    pub ollama_endpoint: String,
    /// Default model
    pub default_model: String,
    /// Enable suggestions
    pub suggestions_enabled: bool,
    /// Suggestion frequency
    pub suggestion_frequency: u32,
}

impl Default for AIConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            ollama_endpoint: "http://localhost:11434".to_string(),
            default_model: "llama3.2:latest".to_string(),
            suggestions_enabled: true,
            suggestion_frequency: 30,
        }
    }
}

/// Workspace configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    /// Default workspace count
    pub default_count: u32,
    /// Auto-save interval (seconds)
    pub auto_save_interval: u32,
    /// Show workspace indicator
    pub show_indicator: bool,
    /// Workspace switch animation
    pub switch_animation: bool,
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            default_count: 4,
            auto_save_interval: 300,
            show_indicator: true,
            switch_animation: true,
        }
    }
}

/// Accessibility configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessibilityConfig {
    /// Enable screen reader
    pub screen_reader: bool,
    /// Enable keyboard navigation
    pub keyboard_navigation: bool,
    /// Enable magnification
    pub magnification: bool,
    /// Magnification level
    pub magnification_level: f32,
    /// High contrast mode
    pub high_contrast: bool,
    /// Reduce motion
    pub reduce_motion: bool,
}

impl Default for AccessibilityConfig {
    fn default() -> Self {
        Self {
            screen_reader: false,
            keyboard_navigation: true,
            magnification: false,
            magnification_level: 2.0,
            high_contrast: false,
            reduce_motion: false,
        }
    }
}

/// Keyboard shortcut
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyboardShortcut {
    /// Key combination
    pub keys: String,
    /// Action to perform
    pub action: String,
    /// Description
    pub description: String,
}

/// Configuration change events
#[derive(Debug, Clone)]
pub enum ConfigChangeEvent {
    Initialized,
    ConfigReloaded,
    ThemeChanged(String),
    ValueChanged(String),
}

/// Get default keyboard shortcuts
fn default_shortcuts() -> HashMap<String, KeyboardShortcut> {
    let mut shortcuts = HashMap::new();
    
    shortcuts.insert("quit".to_string(), KeyboardShortcut {
        keys: "Super+Q".to_string(),
        action: "quit".to_string(),
        description: "Quit the compositor".to_string(),
    });
    
    shortcuts.insert("launcher".to_string(), KeyboardShortcut {
        keys: "Super+Space".to_string(),
        action: "open_launcher".to_string(),
        description: "Open application launcher".to_string(),
    });
    
    shortcuts.insert("switch_workspace".to_string(), KeyboardShortcut {
        keys: "Super+Tab".to_string(),
        action: "switch_workspace".to_string(),
        description: "Switch workspaces".to_string(),
    });
    
    shortcuts.insert("close_window".to_string(), KeyboardShortcut {
        keys: "Super+W".to_string(),
        action: "close_window".to_string(),
        description: "Close focused window".to_string(),
    });
    
    shortcuts
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        let config = GraphDesktopConfig::default();
        assert_eq!(config.general.terminal, "alacritty");
        assert_eq!(config.appearance.theme, "horizon-dark");
        assert!(config.graph.physics_enabled);
    }
    
    #[tokio::test]
    async fn test_config_manager() {
        let (mut manager, _rx) = ConfigManager::new();
        
        // Test getting and setting values
        manager.set("test_key", "test_value").unwrap();
        let value: String = manager.get("test_key").unwrap();
        assert_eq!(value, "test_value");
    }
}