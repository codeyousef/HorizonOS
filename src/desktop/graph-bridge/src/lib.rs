//! Traditional Mode Bridge for HorizonOS graph desktop
//!
//! This module provides compatibility with traditional desktop paradigms
//! by mapping familiar metaphors onto the graph desktop architecture.
//! Users can interact with the system using conventional file managers,
//! application grids, and window management while the underlying graph
//! structure handles the actual organization and relationships.

pub mod file_manager;
pub mod app_grid;
pub mod window_bridge;
pub mod migration;
pub mod fallback;

pub use file_manager::*;
pub use app_grid::*;
pub use window_bridge::*;
pub use migration::*;
pub use fallback::*;

use horizonos_graph_engine::GraphEngine;
use horizonos_graph_nodes::NodeManager;
use std::collections::HashMap;
use std::path::PathBuf;

/// Traditional Mode Bridge Manager
/// 
/// Coordinates traditional desktop interfaces with the graph backend
pub struct TraditionalBridge {
    /// File manager interface
    file_manager: FileManagerBridge,
    /// Application grid interface
    app_grid: ApplicationGrid,
    /// Window management bridge
    window_bridge: WindowBridge,
    /// Migration tools for transitioning users
    migration: MigrationTools,
    /// Fallback interface for graph failures
    fallback: FallbackInterface,
    /// Bridge configuration
    config: BridgeConfig,
    /// Active mode
    mode: BridgeMode,
}

/// Bridge configuration
#[derive(Debug, Clone)]
pub struct BridgeConfig {
    /// Enable file manager mode
    pub enable_file_manager: bool,
    /// Enable application grid mode
    pub enable_app_grid: bool,
    /// Enable window management bridge
    pub enable_window_bridge: bool,
    /// Auto-migrate existing desktop configuration
    pub auto_migrate: bool,
    /// Show traditional UI hints
    pub show_hints: bool,
    /// Animation duration for mode transitions (ms)
    pub transition_duration: u32,
}

/// Bridge operation modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeMode {
    /// Pure graph mode (no traditional elements)
    GraphOnly,
    /// Hybrid mode (graph with traditional overlays)
    Hybrid,
    /// Traditional mode (graph hidden, traditional UI shown)
    Traditional,
    /// Fallback mode (emergency traditional interface)
    Fallback,
}

/// Bridge events for mode transitions
#[derive(Debug, Clone)]
pub enum BridgeEvent {
    /// Mode changed
    ModeChanged { old_mode: BridgeMode, new_mode: BridgeMode },
    /// File manager opened
    FileManagerOpened { path: PathBuf },
    /// Application launched from grid
    AppLaunched { app_id: String },
    /// Window management action
    WindowAction { action: WindowAction },
    /// Migration started
    MigrationStarted,
    /// Migration completed
    MigrationCompleted { success: bool },
    /// Fallback activated
    FallbackActivated { reason: String },
}

/// Window management actions
#[derive(Debug, Clone)]
pub enum WindowAction {
    /// Minimize window
    Minimize { window_id: u64 },
    /// Maximize window
    Maximize { window_id: u64 },
    /// Close window
    Close { window_id: u64 },
    /// Move window
    Move { window_id: u64, x: i32, y: i32 },
    /// Resize window
    Resize { window_id: u64, width: u32, height: u32 },
    /// Focus window
    Focus { window_id: u64 },
    /// Create workspace
    CreateWorkspace { name: String },
    /// Switch workspace
    SwitchWorkspace { workspace_id: String },
}

impl TraditionalBridge {
    /// Create a new traditional bridge
    pub fn new(config: BridgeConfig) -> Self {
        Self {
            file_manager: FileManagerBridge::new(),
            app_grid: ApplicationGrid::new(),
            window_bridge: WindowBridge::new(),
            migration: MigrationTools::new(),
            fallback: FallbackInterface::new(),
            config,
            mode: BridgeMode::GraphOnly,
        }
    }
    
    /// Initialize the bridge
    pub fn initialize(&mut self, engine: &mut GraphEngine, node_manager: &mut NodeManager) -> Result<(), BridgeError> {
        // Initialize all bridge components
        self.file_manager.initialize(engine, node_manager)?;
        self.app_grid.initialize(engine, node_manager)?;
        self.window_bridge.initialize(engine, node_manager)?;
        
        // Auto-migrate if enabled
        if self.config.auto_migrate {
            self.migration.start_migration(engine, node_manager)?;
        }
        
        Ok(())
    }
    
    /// Set bridge mode
    pub fn set_mode(&mut self, mode: BridgeMode) -> Result<(), BridgeError> {
        let old_mode = self.mode;
        self.mode = mode;
        
        // Handle mode transition
        match mode {
            BridgeMode::GraphOnly => {
                self.hide_traditional_ui();
            }
            BridgeMode::Hybrid => {
                self.show_traditional_overlay();
            }
            BridgeMode::Traditional => {
                self.show_traditional_ui();
                self.hide_graph_ui();
            }
            BridgeMode::Fallback => {
                self.activate_fallback();
            }
        }
        
        Ok(())
    }
    
    /// Get current bridge mode
    pub fn current_mode(&self) -> BridgeMode {
        self.mode
    }
    
    /// Open file manager at path
    pub fn open_file_manager(&mut self, path: Option<PathBuf>) -> Result<(), BridgeError> {
        if !self.config.enable_file_manager {
            return Err(BridgeError::FeatureDisabled("file_manager".to_string()));
        }
        
        let path = path.unwrap_or_else(|| std::env::home_dir().unwrap_or_else(|| PathBuf::from("/")));
        self.file_manager.open_location(&path)
    }
    
    /// Open application grid
    pub fn open_app_grid(&mut self) -> Result<(), BridgeError> {
        if !self.config.enable_app_grid {
            return Err(BridgeError::FeatureDisabled("app_grid".to_string()));
        }
        
        self.app_grid.show()
    }
    
    /// Launch application
    pub fn launch_application(&mut self, app_id: &str) -> Result<(), BridgeError> {
        self.app_grid.launch_application(app_id)
    }
    
    /// Handle window action
    pub fn handle_window_action(&mut self, action: WindowAction) -> Result<(), BridgeError> {
        if !self.config.enable_window_bridge {
            return Err(BridgeError::FeatureDisabled("window_bridge".to_string()));
        }
        
        self.window_bridge.handle_action(action)
    }
    
    /// Start migration from traditional desktop
    pub fn start_migration(&mut self, engine: &mut GraphEngine, node_manager: &mut NodeManager) -> Result<(), BridgeError> {
        self.migration.start_migration(engine, node_manager)
    }
    
    /// Check migration status
    pub fn migration_status(&self) -> MigrationStatus {
        self.migration.status()
    }
    
    /// Activate fallback mode
    pub fn activate_fallback(&mut self) {
        self.fallback.activate();
        self.mode = BridgeMode::Fallback;
    }
    
    /// Update bridge (called each frame)
    pub fn update(&mut self, engine: &mut GraphEngine, node_manager: &mut NodeManager) -> Result<Vec<BridgeEvent>, BridgeError> {
        let mut events = Vec::new();
        
        // Update all bridge components
        events.extend(self.file_manager.update(engine, node_manager)?);
        events.extend(self.app_grid.update(engine, node_manager)?);
        events.extend(self.window_bridge.update(engine, node_manager)?);
        events.extend(self.migration.update(engine, node_manager)?);
        events.extend(self.fallback.update()?);
        
        Ok(events)
    }
    
    /// Hide traditional UI elements
    fn hide_traditional_ui(&mut self) {
        self.file_manager.hide();
        self.app_grid.hide();
        self.window_bridge.hide_decorations();
    }
    
    /// Show traditional UI overlay
    fn show_traditional_overlay(&mut self) {
        self.file_manager.show_overlay();
        self.app_grid.show_overlay();
        self.window_bridge.show_decorations();
    }
    
    /// Show traditional UI
    fn show_traditional_ui(&mut self) {
        self.file_manager.show();
        self.app_grid.show();
        self.window_bridge.show_full_decorations();
    }
    
    /// Hide graph UI elements
    fn hide_graph_ui(&mut self) {
        // This would communicate with the graph engine to hide graph elements
        log::info!("Hiding graph UI for traditional mode");
    }
    
    /// Get bridge configuration
    pub fn config(&self) -> &BridgeConfig {
        &self.config
    }
    
    /// Update bridge configuration
    pub fn set_config(&mut self, config: BridgeConfig) {
        self.config = config;
    }
    
    /// Check if feature is enabled
    pub fn is_feature_enabled(&self, feature: &str) -> bool {
        match feature {
            "file_manager" => self.config.enable_file_manager,
            "app_grid" => self.config.enable_app_grid,
            "window_bridge" => self.config.enable_window_bridge,
            _ => false,
        }
    }
}

/// Bridge error types
#[derive(Debug, Clone)]
pub enum BridgeError {
    /// Feature is disabled
    FeatureDisabled(String),
    /// Initialization failed
    InitializationFailed(String),
    /// Migration failed
    MigrationFailed(String),
    /// IO error
    IoError(String),
    /// Invalid configuration
    InvalidConfig(String),
    /// Fallback activation failed
    FallbackFailed(String),
}

impl std::fmt::Display for BridgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BridgeError::FeatureDisabled(feature) => write!(f, "Feature disabled: {}", feature),
            BridgeError::InitializationFailed(msg) => write!(f, "Initialization failed: {}", msg),
            BridgeError::MigrationFailed(msg) => write!(f, "Migration failed: {}", msg),
            BridgeError::IoError(msg) => write!(f, "IO error: {}", msg),
            BridgeError::InvalidConfig(msg) => write!(f, "Invalid configuration: {}", msg),
            BridgeError::FallbackFailed(msg) => write!(f, "Fallback failed: {}", msg),
        }
    }
}

impl std::error::Error for BridgeError {}

impl Default for BridgeConfig {
    fn default() -> Self {
        Self {
            enable_file_manager: true,
            enable_app_grid: true,
            enable_window_bridge: true,
            auto_migrate: true,
            show_hints: true,
            transition_duration: 300,
        }
    }
}

impl Default for TraditionalBridge {
    fn default() -> Self {
        Self::new(BridgeConfig::default())
    }
}
