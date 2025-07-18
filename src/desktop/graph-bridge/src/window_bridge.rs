//! Window Management Bridge
//!
//! Provides traditional window management on top of the graph desktop

use horizonos_graph_engine::GraphEngine;
use horizonos_graph_nodes::NodeManager;
use crate::{BridgeError, BridgeEvent, WindowAction};
use std::collections::HashMap;

/// Window Bridge
/// 
/// Maps traditional window management operations to graph nodes
/// and provides familiar window decorations and controls
pub struct WindowBridge {
    /// Managed windows
    windows: HashMap<u64, ManagedWindow>,
    /// Window decorations state
    decorations_visible: bool,
    /// Workspaces
    workspaces: HashMap<String, Workspace>,
    /// Active workspace
    active_workspace: String,
    /// Window focus history
    focus_history: Vec<u64>,
    /// Bridge configuration
    config: WindowBridgeConfig,
}

/// Managed window information
#[derive(Debug, Clone)]
pub struct ManagedWindow {
    /// Window ID
    pub id: u64,
    /// Window title
    pub title: String,
    /// Application name
    pub app_name: String,
    /// Window state
    pub state: WindowState,
    /// Window geometry
    pub geometry: WindowGeometry,
    /// Decorations
    pub decorations: WindowDecorations,
    /// Graph node ID
    pub node_id: Option<u64>,
    /// Workspace ID
    pub workspace_id: String,
    /// Is focused
    pub is_focused: bool,
    /// Is visible
    pub is_visible: bool,
}

/// Window state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowState {
    /// Normal state
    Normal,
    /// Minimized
    Minimized,
    /// Maximized
    Maximized,
    /// Fullscreen
    Fullscreen,
    /// Tiled left
    TiledLeft,
    /// Tiled right
    TiledRight,
    /// Tiled top
    TiledTop,
    /// Tiled bottom
    TiledBottom,
}

/// Window geometry
#[derive(Debug, Clone)]
pub struct WindowGeometry {
    /// X position
    pub x: i32,
    /// Y position
    pub y: i32,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Minimum width
    pub min_width: u32,
    /// Minimum height
    pub min_height: u32,
    /// Maximum width
    pub max_width: u32,
    /// Maximum height
    pub max_height: u32,
}

/// Window decorations
#[derive(Debug, Clone)]
pub struct WindowDecorations {
    /// Show title bar
    pub title_bar: bool,
    /// Show border
    pub border: bool,
    /// Show minimize button
    pub minimize_button: bool,
    /// Show maximize button
    pub maximize_button: bool,
    /// Show close button
    pub close_button: bool,
    /// Show resize handles
    pub resize_handles: bool,
    /// Title bar height
    pub title_bar_height: u32,
    /// Border width
    pub border_width: u32,
}

/// Workspace
#[derive(Debug, Clone)]
pub struct Workspace {
    /// Workspace ID
    pub id: String,
    /// Display name
    pub name: String,
    /// Windows in this workspace
    pub windows: Vec<u64>,
    /// Is active
    pub is_active: bool,
    /// Layout mode
    pub layout_mode: LayoutMode,
}

/// Layout modes for workspaces
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    /// Free-form floating
    Floating,
    /// Tiling layout
    Tiling,
    /// Automatic layout
    Auto,
    /// Grid layout
    Grid,
}

/// Window bridge configuration
#[derive(Debug, Clone)]
pub struct WindowBridgeConfig {
    /// Enable window decorations
    pub enable_decorations: bool,
    /// Enable workspaces
    pub enable_workspaces: bool,
    /// Default layout mode
    pub default_layout_mode: LayoutMode,
    /// Auto-tile windows
    pub auto_tile: bool,
    /// Focus follows mouse
    pub focus_follows_mouse: bool,
    /// Click to focus
    pub click_to_focus: bool,
    /// Window animation duration
    pub animation_duration: u32,
}

/// Window events
#[derive(Debug, Clone)]
pub enum WindowEvent {
    /// Window created
    WindowCreated { window: ManagedWindow },
    /// Window destroyed
    WindowDestroyed { window_id: u64 },
    /// Window moved
    WindowMoved { window_id: u64, x: i32, y: i32 },
    /// Window resized
    WindowResized { window_id: u64, width: u32, height: u32 },
    /// Window state changed
    WindowStateChanged { window_id: u64, old_state: WindowState, new_state: WindowState },
    /// Window focused
    WindowFocused { window_id: u64 },
    /// Workspace created
    WorkspaceCreated { workspace: Workspace },
    /// Workspace switched
    WorkspaceSwitched { old_workspace: String, new_workspace: String },
}

impl WindowBridge {
    /// Create a new window bridge
    pub fn new() -> Self {
        let mut workspaces = HashMap::new();
        let default_workspace = Workspace {
            id: "default".to_string(),
            name: "Workspace 1".to_string(),
            windows: Vec::new(),
            is_active: true,
            layout_mode: LayoutMode::Floating,
        };
        workspaces.insert("default".to_string(), default_workspace);
        
        Self {
            windows: HashMap::new(),
            decorations_visible: true,
            workspaces,
            active_workspace: "default".to_string(),
            focus_history: Vec::new(),
            config: WindowBridgeConfig::default(),
        }
    }
    
    /// Initialize the window bridge
    pub fn initialize(&mut self, _engine: &mut GraphEngine, _node_manager: &mut NodeManager) -> Result<(), BridgeError> {
        log::info!("Window bridge initialized");
        Ok(())
    }
    
    /// Handle window action
    pub fn handle_action(&mut self, action: WindowAction) -> Result<(), BridgeError> {
        match action {
            WindowAction::Minimize { window_id } => self.minimize_window(window_id),
            WindowAction::Maximize { window_id } => self.maximize_window(window_id),
            WindowAction::Close { window_id } => self.close_window(window_id),
            WindowAction::Move { window_id, x, y } => self.move_window(window_id, x, y),
            WindowAction::Resize { window_id, width, height } => self.resize_window(window_id, width, height),
            WindowAction::Focus { window_id } => self.focus_window(window_id),
            WindowAction::CreateWorkspace { name } => self.create_workspace(name),
            WindowAction::SwitchWorkspace { workspace_id } => self.switch_workspace(&workspace_id),
        }
    }
    
    /// Register a new window
    pub fn register_window(&mut self, window_id: u64, title: String, app_name: String) -> Result<(), BridgeError> {
        let geometry = WindowGeometry::default();
        let decorations = if self.config.enable_decorations {
            WindowDecorations::default()
        } else {
            WindowDecorations::minimal()
        };
        
        let window = ManagedWindow {
            id: window_id,
            title,
            app_name,
            state: WindowState::Normal,
            geometry,
            decorations,
            node_id: None,
            workspace_id: self.active_workspace.clone(),
            is_focused: false,
            is_visible: true,
        };
        
        // Add to active workspace
        if let Some(workspace) = self.workspaces.get_mut(&self.active_workspace) {
            workspace.windows.push(window_id);
        }
        
        self.windows.insert(window_id, window);
        
        // Auto-focus if configured
        if self.config.click_to_focus {
            self.focus_window(window_id)?;
        }
        
        log::info!("Registered window: {} ({})", window_id, self.windows[&window_id].title);
        Ok(())
    }
    
    /// Unregister a window
    pub fn unregister_window(&mut self, window_id: u64) -> Result<(), BridgeError> {
        if let Some(window) = self.windows.remove(&window_id) {
            // Remove from workspace
            if let Some(workspace) = self.workspaces.get_mut(&window.workspace_id) {
                workspace.windows.retain(|&id| id != window_id);
            }
            
            // Remove from focus history
            self.focus_history.retain(|&id| id != window_id);
            
            // Focus next window if this was focused
            if window.is_focused && !self.focus_history.is_empty() {
                let next_window = self.focus_history[self.focus_history.len() - 1];
                self.focus_window(next_window)?;
            }
            
            log::info!("Unregistered window: {} ({})", window_id, window.title);
            Ok(())
        } else {
            Err(BridgeError::IoError(format!("Window not found: {}", window_id)))
        }
    }
    
    /// Minimize window
    pub fn minimize_window(&mut self, window_id: u64) -> Result<(), BridgeError> {
        if let Some(window) = self.windows.get_mut(&window_id) {
            let _old_state = window.state;
            window.state = WindowState::Minimized;
            window.is_visible = false;
            
            log::info!("Minimized window: {}", window_id);
            Ok(())
        } else {
            Err(BridgeError::IoError(format!("Window not found: {}", window_id)))
        }
    }
    
    /// Maximize window
    pub fn maximize_window(&mut self, window_id: u64) -> Result<(), BridgeError> {
        if let Some(window) = self.windows.get_mut(&window_id) {
            let _old_state = window.state;
            
            if window.state == WindowState::Maximized {
                // Restore to normal
                window.state = WindowState::Normal;
                // In a real implementation, you'd restore previous geometry
            } else {
                window.state = WindowState::Maximized;
                // In a real implementation, you'd set geometry to screen size
            }
            
            log::info!("Toggled maximize for window: {} (state: {:?})", window_id, window.state);
            Ok(())
        } else {
            Err(BridgeError::IoError(format!("Window not found: {}", window_id)))
        }
    }
    
    /// Close window
    pub fn close_window(&mut self, window_id: u64) -> Result<(), BridgeError> {
        // In a real implementation, this would send a close request to the application
        log::info!("Closing window: {}", window_id);
        self.unregister_window(window_id)
    }
    
    /// Move window
    pub fn move_window(&mut self, window_id: u64, x: i32, y: i32) -> Result<(), BridgeError> {
        if let Some(window) = self.windows.get_mut(&window_id) {
            window.geometry.x = x;
            window.geometry.y = y;
            
            log::debug!("Moved window {} to ({}, {})", window_id, x, y);
            Ok(())
        } else {
            Err(BridgeError::IoError(format!("Window not found: {}", window_id)))
        }
    }
    
    /// Resize window
    pub fn resize_window(&mut self, window_id: u64, width: u32, height: u32) -> Result<(), BridgeError> {
        if let Some(window) = self.windows.get_mut(&window_id) {
            // Respect minimum and maximum sizes
            let width = width.max(window.geometry.min_width).min(window.geometry.max_width);
            let height = height.max(window.geometry.min_height).min(window.geometry.max_height);
            
            window.geometry.width = width;
            window.geometry.height = height;
            
            log::debug!("Resized window {} to {}x{}", window_id, width, height);
            Ok(())
        } else {
            Err(BridgeError::IoError(format!("Window not found: {}", window_id)))
        }
    }
    
    /// Focus window
    pub fn focus_window(&mut self, window_id: u64) -> Result<(), BridgeError> {
        // Unfocus all windows first
        for window in self.windows.values_mut() {
            window.is_focused = false;
        }
        
        if let Some(window) = self.windows.get_mut(&window_id) {
            window.is_focused = true;
            
            // Update focus history
            self.focus_history.retain(|&id| id != window_id);
            self.focus_history.push(window_id);
            
            // Limit focus history size
            if self.focus_history.len() > 50 {
                self.focus_history.remove(0);
            }
            
            log::debug!("Focused window: {} ({})", window_id, window.title);
            Ok(())
        } else {
            Err(BridgeError::IoError(format!("Window not found: {}", window_id)))
        }
    }
    
    /// Create new workspace
    pub fn create_workspace(&mut self, name: String) -> Result<(), BridgeError> {
        let workspace_id = format!("workspace_{}", self.workspaces.len() + 1);
        
        let workspace = Workspace {
            id: workspace_id.clone(),
            name,
            windows: Vec::new(),
            is_active: false,
            layout_mode: self.config.default_layout_mode,
        };
        
        self.workspaces.insert(workspace_id.clone(), workspace);
        
        log::info!("Created workspace: {} ({})", workspace_id, self.workspaces[&workspace_id].name);
        Ok(())
    }
    
    /// Switch to workspace
    pub fn switch_workspace(&mut self, workspace_id: &str) -> Result<(), BridgeError> {
        if !self.workspaces.contains_key(workspace_id) {
            return Err(BridgeError::IoError(format!("Workspace not found: {}", workspace_id)));
        }
        
        let old_workspace = self.active_workspace.clone();
        
        // Deactivate current workspace
        if let Some(workspace) = self.workspaces.get_mut(&self.active_workspace) {
            workspace.is_active = false;
            // Hide windows in current workspace
            for &window_id in &workspace.windows {
                if let Some(window) = self.windows.get_mut(&window_id) {
                    window.is_visible = false;
                }
            }
        }
        
        // Activate new workspace
        self.active_workspace = workspace_id.to_string();
        if let Some(workspace) = self.workspaces.get_mut(workspace_id) {
            workspace.is_active = true;
            // Show windows in new workspace
            for &window_id in &workspace.windows {
                if let Some(window) = self.windows.get_mut(&window_id) {
                    window.is_visible = true;
                }
            }
        }
        
        log::info!("Switched workspace: {} -> {}", old_workspace, workspace_id);
        Ok(())
    }
    
    /// Show window decorations
    pub fn show_decorations(&mut self) {
        self.decorations_visible = true;
        for window in self.windows.values_mut() {
            window.decorations = WindowDecorations::default();
        }
    }
    
    /// Hide window decorations
    pub fn hide_decorations(&mut self) {
        self.decorations_visible = false;
        for window in self.windows.values_mut() {
            window.decorations = WindowDecorations::minimal();
        }
    }
    
    /// Show full decorations
    pub fn show_full_decorations(&mut self) {
        self.decorations_visible = true;
        for window in self.windows.values_mut() {
            window.decorations = WindowDecorations::full();
        }
    }
    
    /// Update window bridge
    pub fn update(&mut self, _engine: &mut GraphEngine, _node_manager: &mut NodeManager) -> Result<Vec<BridgeEvent>, BridgeError> {
        let events = Vec::new();
        
        // Auto-tile windows if enabled
        if self.config.auto_tile {
            self.auto_tile_windows()?;
        }
        
        Ok(events)
    }
    
    /// Auto-tile windows in current workspace
    fn auto_tile_windows(&mut self) -> Result<(), BridgeError> {
        if let Some(workspace) = self.workspaces.get(&self.active_workspace) {
            if workspace.layout_mode == LayoutMode::Tiling && workspace.windows.len() > 1 {
                // Simple tiling algorithm - split screen evenly
                let visible_windows: Vec<_> = workspace.windows.iter()
                    .filter(|&&id| self.windows.get(&id).map_or(false, |w| w.is_visible && w.state != WindowState::Minimized))
                    .collect();
                
                if visible_windows.len() > 1 {
                    let screen_width = 1920; // In a real implementation, get from display
                    let screen_height = 1080;
                    
                    let window_width = screen_width / visible_windows.len() as u32;
                    
                    for (i, &&window_id) in visible_windows.iter().enumerate() {
                        if let Some(window) = self.windows.get_mut(&window_id) {
                            window.geometry.x = (i as u32 * window_width) as i32;
                            window.geometry.y = 0;
                            window.geometry.width = window_width;
                            window.geometry.height = screen_height;
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Get windows in current workspace
    pub fn get_current_workspace_windows(&self) -> Vec<&ManagedWindow> {
        if let Some(workspace) = self.workspaces.get(&self.active_workspace) {
            workspace.windows.iter()
                .filter_map(|&id| self.windows.get(&id))
                .collect()
        } else {
            Vec::new()
        }
    }
    
    /// Get all workspaces
    pub fn get_workspaces(&self) -> Vec<&Workspace> {
        self.workspaces.values().collect()
    }
    
    /// Get focused window
    pub fn get_focused_window(&self) -> Option<&ManagedWindow> {
        self.windows.values().find(|w| w.is_focused)
    }
    
    /// Set configuration
    pub fn set_config(&mut self, config: WindowBridgeConfig) {
        self.config = config;
        
        // Apply decoration changes
        if self.config.enable_decorations {
            self.show_decorations();
        } else {
            self.hide_decorations();
        }
    }
    
    /// Get configuration
    pub fn config(&self) -> &WindowBridgeConfig {
        &self.config
    }
    
    /// Get window by ID
    pub fn get_window(&self, window_id: u64) -> Option<&ManagedWindow> {
        self.windows.get(&window_id)
    }
    
    /// Get active workspace
    pub fn active_workspace(&self) -> &str {
        &self.active_workspace
    }
}

impl Default for WindowBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for WindowGeometry {
    fn default() -> Self {
        Self {
            x: 100,
            y: 100,
            width: 800,
            height: 600,
            min_width: 200,
            min_height: 100,
            max_width: 3840,
            max_height: 2160,
        }
    }
}

impl Default for WindowDecorations {
    fn default() -> Self {
        Self {
            title_bar: true,
            border: true,
            minimize_button: true,
            maximize_button: true,
            close_button: true,
            resize_handles: true,
            title_bar_height: 32,
            border_width: 2,
        }
    }
}

impl WindowDecorations {
    /// Minimal decorations
    pub fn minimal() -> Self {
        Self {
            title_bar: false,
            border: true,
            minimize_button: false,
            maximize_button: false,
            close_button: false,
            resize_handles: false,
            title_bar_height: 0,
            border_width: 1,
        }
    }
    
    /// Full decorations
    pub fn full() -> Self {
        Self {
            title_bar: true,
            border: true,
            minimize_button: true,
            maximize_button: true,
            close_button: true,
            resize_handles: true,
            title_bar_height: 40,
            border_width: 3,
        }
    }
}

impl Default for WindowBridgeConfig {
    fn default() -> Self {
        Self {
            enable_decorations: true,
            enable_workspaces: true,
            default_layout_mode: LayoutMode::Floating,
            auto_tile: false,
            focus_follows_mouse: false,
            click_to_focus: true,
            animation_duration: 200,
        }
    }
}