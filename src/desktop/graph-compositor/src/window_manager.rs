//! Window management system for HorizonOS graph compositor
//! 
//! This module provides comprehensive window management including:
//! - Window positioning and sizing
//! - Focus management and activation
//! - Window stacking and layering
//! - Workspace management
//! - Window rules and policies
//! - Integration with graph layout system

use crate::{AppState, protocols::ProtocolManager};
use smithay::desktop::Window;
use horizonos_graph_engine::{SceneId, Position};
use smithay::{
    desktop::{Space, Window as SmithayWindow},
    output::Output,
    utils::{Logical, Point, Rectangle, Size},
    wayland::shell::xdg::XdgToplevelSurfaceData,
};
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};

/// Window management system
#[derive(Debug)]
pub struct WindowManager {
    /// Window stack organized by workspace
    workspaces: HashMap<WorkspaceId, Workspace>,
    /// Current active workspace
    active_workspace: WorkspaceId,
    /// Window focus history
    focus_history: VecDeque<WindowId>,
    /// Window rules for automatic management
    window_rules: Vec<WindowRule>,
    /// Window positioning strategy
    positioning_strategy: PositioningStrategy,
    /// Window layout policies
    layout_policies: Vec<LayoutPolicy>,
    /// Floating windows
    floating_windows: HashSet<WindowId>,
    /// Window animations
    animations: HashMap<WindowId, WindowAnimation>,
    /// Window manager configuration
    config: WindowManagerConfig,
}

/// Workspace identifier
pub type WorkspaceId = u32;

/// Window identifier
pub type WindowId = u32;

/// Workspace containing windows and layout
#[derive(Debug)]
pub struct Workspace {
    /// Workspace ID
    pub id: WorkspaceId,
    /// Workspace name
    pub name: String,
    /// Windows in this workspace
    pub windows: Vec<WindowId>,
    /// Tiled windows layout
    pub tiled_layout: TiledLayout,
    /// Workspace-specific graph scene
    pub graph_scene_id: Option<SceneId>,
    /// Output assignment
    pub output: Option<Output>,
    /// Workspace state
    pub state: WorkspaceState,
}

/// Workspace state
#[derive(Debug, Clone)]
pub struct WorkspaceState {
    /// Whether workspace is visible
    pub visible: bool,
    /// Last access time
    pub last_accessed: Instant,
    /// Number of windows
    pub window_count: usize,
    /// Focus mode active
    pub focus_mode: bool,
}

/// Tiled layout for workspace
#[derive(Debug, Clone)]
pub struct TiledLayout {
    /// Layout type
    pub layout_type: LayoutType,
    /// Main window ratio (for layouts with main area)
    pub main_ratio: f32,
    /// Number of windows in main area
    pub main_count: usize,
    /// Layout orientation
    pub orientation: Orientation,
    /// Gaps between windows
    pub gaps: Gaps,
}

/// Layout types for tiled windows
#[derive(Debug, Clone, Copy)]
pub enum LayoutType {
    /// Master-stack layout
    MasterStack,
    /// Grid layout
    Grid,
    /// Monocle (fullscreen) layout
    Monocle,
    /// Spiral layout
    Spiral,
    /// Dwindle layout
    Dwindle,
    /// Graph-based layout (positions from graph engine)
    GraphBased,
}

/// Layout orientation
#[derive(Debug, Clone, Copy)]
pub enum Orientation {
    Horizontal,
    Vertical,
}

/// Gaps configuration
#[derive(Debug, Clone)]
pub struct Gaps {
    /// Inner gaps between windows
    pub inner: u32,
    /// Outer gaps from screen edges
    pub outer: u32,
    /// Top gap (for panels)
    pub top: u32,
    /// Bottom gap
    pub bottom: u32,
    /// Left gap
    pub left: u32,
    /// Right gap
    pub right: u32,
}

/// Window positioning strategy
#[derive(Debug, Clone, Copy)]
pub enum PositioningStrategy {
    /// Position based on graph layout
    GraphBased,
    /// Traditional tiling
    Tiled,
    /// Floating positioning
    Floating,
    /// Smart positioning (mix of strategies)
    Smart,
}

/// Window rule for automatic management
#[derive(Debug, Clone)]
pub struct WindowRule {
    /// Rule name
    pub name: String,
    /// Window matcher
    pub matcher: WindowMatcher,
    /// Actions to apply
    pub actions: Vec<WindowAction>,
    /// Rule priority
    pub priority: u32,
}

/// Window matcher for rules
#[derive(Debug, Clone)]
pub struct WindowMatcher {
    /// App ID pattern
    pub app_id: Option<String>,
    /// Title pattern  
    pub title: Option<String>,
    /// Window class pattern
    pub class: Option<String>,
    /// Instance pattern
    pub instance: Option<String>,
}

/// Actions to apply to windows
#[derive(Debug, Clone)]
pub enum WindowAction {
    /// Set workspace
    SetWorkspace(WorkspaceId),
    /// Set floating state
    SetFloating(bool),
    /// Set position
    SetPosition(Point<i32, Logical>),
    /// Set size
    SetSize(Size<i32, Logical>),
    /// Set opacity
    SetOpacity(f32),
    /// Set focus
    SetFocus(bool),
    /// Set fullscreen
    SetFullscreen(bool),
    /// Set maximized
    SetMaximized(bool),
    /// Set minimized
    SetMinimized(bool),
    /// Map to graph node
    MapToNode(SceneId),
}

/// Layout policy for window arrangement
#[derive(Debug, Clone)]
pub struct LayoutPolicy {
    /// Policy name
    pub name: String,
    /// When to apply policy
    pub trigger: PolicyTrigger,
    /// Layout modifications
    pub modifications: LayoutModifications,
}

/// Policy trigger conditions
#[derive(Debug, Clone)]
pub enum PolicyTrigger {
    /// Number of windows threshold
    WindowCount { min: usize, max: Option<usize> },
    /// Window type
    WindowType(String),
    /// Time-based trigger
    TimeOfDay { start: u8, end: u8 },
    /// User activity level
    ActivityLevel(ActivityLevel),
}

/// User activity level
#[derive(Debug, Clone, Copy)]
pub enum ActivityLevel {
    Low,
    Medium,
    High,
}

/// Layout modifications
#[derive(Debug, Clone)]
pub struct LayoutModifications {
    /// Change layout type
    pub layout_type: Option<LayoutType>,
    /// Adjust gaps
    pub gaps: Option<Gaps>,
    /// Change positioning strategy
    pub positioning: Option<PositioningStrategy>,
    /// Enable/disable animations
    pub animations: Option<bool>,
}

/// Window animation
#[derive(Debug, Clone)]
pub struct WindowAnimation {
    /// Animation type
    pub animation_type: AnimationType,
    /// Animation duration
    pub duration: Duration,
    /// Start time
    pub start_time: Instant,
    /// Start position/size
    pub start_rect: Rectangle<i32, Logical>,
    /// Target position/size
    pub target_rect: Rectangle<i32, Logical>,
    /// Easing function
    pub easing: EasingFunction,
}

/// Animation types
#[derive(Debug, Clone, Copy)]
pub enum AnimationType {
    Move,
    Resize,
    Fade,
    Scale,
    Slide,
}

/// Easing functions for animations
#[derive(Debug, Clone, Copy)]
pub enum EasingFunction {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    Bounce,
    Elastic,
}

/// Window manager configuration
#[derive(Debug, Clone)]
pub struct WindowManagerConfig {
    /// Default layout type
    pub default_layout: LayoutType,
    /// Default gaps
    pub default_gaps: Gaps,
    /// Animation settings
    pub animations: AnimationConfig,
    /// Focus settings
    pub focus: FocusConfig,
    /// Workspace settings
    pub workspaces: WorkspaceConfig,
}

/// Animation configuration
#[derive(Debug, Clone)]
pub struct AnimationConfig {
    /// Enable animations
    pub enabled: bool,
    /// Default duration
    pub duration: Duration,
    /// Default easing
    pub easing: EasingFunction,
    /// Performance mode (reduce animations when performance is poor)
    pub performance_mode: bool,
}

/// Focus configuration
#[derive(Debug, Clone)]
pub struct FocusConfig {
    /// Follow mouse focus
    pub follow_mouse: bool,
    /// Focus new windows
    pub focus_new_windows: bool,
    /// Warp mouse to focused window
    pub warp_mouse: bool,
    /// Focus history size
    pub history_size: usize,
}

/// Workspace configuration
#[derive(Debug, Clone)]
pub struct WorkspaceConfig {
    /// Number of workspaces
    pub count: u32,
    /// Auto-create workspaces
    pub auto_create: bool,
    /// Auto-switch to new windows
    pub auto_switch: bool,
    /// Workspace names
    pub names: Vec<String>,
}

impl WindowManager {
    /// Create a new window manager
    pub fn new(config: WindowManagerConfig) -> Self {
        let mut workspaces = HashMap::new();
        
        // Create initial workspaces
        for i in 0..config.workspaces.count {
            let name = config.workspaces.names.get(i as usize)
                .cloned()
                .unwrap_or_else(|| format!("Workspace {}", i + 1));
            
            let workspace = Workspace {
                id: i,
                name,
                windows: Vec::new(),
                tiled_layout: TiledLayout {
                    layout_type: config.default_layout,
                    main_ratio: 0.6,
                    main_count: 1,
                    orientation: Orientation::Horizontal,
                    gaps: config.default_gaps.clone(),
                },
                graph_scene_id: None,
                output: None,
                state: WorkspaceState {
                    visible: i == 0,
                    last_accessed: Instant::now(),
                    window_count: 0,
                    focus_mode: false,
                },
            };
            
            workspaces.insert(i, workspace);
        }
        
        Self {
            workspaces,
            active_workspace: 0,
            focus_history: VecDeque::new(),
            window_rules: Vec::new(),
            positioning_strategy: PositioningStrategy::Smart,
            layout_policies: Vec::new(),
            floating_windows: HashSet::new(),
            animations: HashMap::new(),
            config,
        }
    }
    
    /// Add a window to the manager
    pub fn add_window(&mut self, window_id: WindowId, app_id: Option<String>, title: Option<String>) -> anyhow::Result<()> {
        // Apply window rules
        let actions = self.evaluate_window_rules(app_id.as_deref(), title.as_deref());
        
        // Determine target workspace
        let target_workspace = actions.iter()
            .find_map(|action| match action {
                WindowAction::SetWorkspace(ws) => Some(*ws),
                _ => None,
            })
            .unwrap_or(self.active_workspace);
        
        // Add to workspace
        if let Some(workspace) = self.workspaces.get_mut(&target_workspace) {
            workspace.windows.push(window_id);
            workspace.state.window_count += 1;
            workspace.state.last_accessed = Instant::now();
        }
        
        // Apply other actions
        for action in actions {
            self.apply_window_action(window_id, action)?;
        }
        
        // Update layout
        self.update_workspace_layout(target_workspace)?;
        
        // Focus if configured
        if self.config.focus.focus_new_windows {
            self.focus_window(window_id)?;
        }
        
        log::debug!("Added window {} to workspace {}", window_id, target_workspace);
        Ok(())
    }
    
    /// Remove a window from the manager
    pub fn remove_window(&mut self, window_id: WindowId) -> anyhow::Result<()> {
        // Find and remove from workspace
        let mut target_workspace = None;
        for (ws_id, workspace) in &mut self.workspaces {
            if let Some(pos) = workspace.windows.iter().position(|&id| id == window_id) {
                workspace.windows.remove(pos);
                workspace.state.window_count -= 1;
                target_workspace = Some(*ws_id);
                break;
            }
        }
        
        // Remove from focus history
        self.focus_history.retain(|&id| id != window_id);
        
        // Remove from floating windows
        self.floating_windows.remove(&window_id);
        
        // Remove animations
        self.animations.remove(&window_id);
        
        // Update layout if needed
        if let Some(ws_id) = target_workspace {
            self.update_workspace_layout(ws_id)?;
        }
        
        log::debug!("Removed window {}", window_id);
        Ok(())
    }
    
    /// Focus a window
    pub fn focus_window(&mut self, window_id: WindowId) -> anyhow::Result<()> {
        // Add to focus history
        self.focus_history.retain(|&id| id != window_id);
        self.focus_history.push_front(window_id);
        
        // Limit history size
        if self.focus_history.len() > self.config.focus.history_size {
            self.focus_history.pop_back();
        }
        
        log::debug!("Focused window {}", window_id);
        Ok(())
    }
    
    /// Switch to workspace
    pub fn switch_workspace(&mut self, workspace_id: WorkspaceId) -> anyhow::Result<()> {
        if !self.workspaces.contains_key(&workspace_id) {
            return Err(anyhow::anyhow!("Workspace {} does not exist", workspace_id));
        }
        
        // Update visibility
        if let Some(current) = self.workspaces.get_mut(&self.active_workspace) {
            current.state.visible = false;
        }
        
        if let Some(target) = self.workspaces.get_mut(&workspace_id) {
            target.state.visible = true;
            target.state.last_accessed = Instant::now();
        }
        
        self.active_workspace = workspace_id;
        
        log::debug!("Switched to workspace {}", workspace_id);
        Ok(())
    }
    
    /// Set layout for workspace
    pub fn set_workspace_layout(&mut self, workspace_id: WorkspaceId, layout_type: LayoutType) -> anyhow::Result<()> {
        if let Some(workspace) = self.workspaces.get_mut(&workspace_id) {
            workspace.tiled_layout.layout_type = layout_type;
            self.update_workspace_layout(workspace_id)?;
            log::debug!("Set workspace {} layout to {:?}", workspace_id, layout_type);
        }
        Ok(())
    }
    
    /// Update workspace layout
    fn update_workspace_layout(&mut self, workspace_id: WorkspaceId) -> anyhow::Result<()> {
        if let Some(workspace) = self.workspaces.get(&workspace_id) {
            match workspace.tiled_layout.layout_type {
                LayoutType::MasterStack => self.apply_master_stack_layout(workspace_id)?,
                LayoutType::Grid => self.apply_grid_layout(workspace_id)?,
                LayoutType::Monocle => self.apply_monocle_layout(workspace_id)?,
                LayoutType::GraphBased => self.apply_graph_based_layout(workspace_id)?,
                _ => {} // TODO: Implement other layouts
            }
        }
        Ok(())
    }
    
    /// Apply master-stack layout
    fn apply_master_stack_layout(&mut self, workspace_id: WorkspaceId) -> anyhow::Result<()> {
        // TODO: Implement master-stack layout logic
        log::debug!("Applied master-stack layout to workspace {}", workspace_id);
        Ok(())
    }
    
    /// Apply grid layout
    fn apply_grid_layout(&mut self, workspace_id: WorkspaceId) -> anyhow::Result<()> {
        // TODO: Implement grid layout logic
        log::debug!("Applied grid layout to workspace {}", workspace_id);
        Ok(())
    }
    
    /// Apply monocle layout
    fn apply_monocle_layout(&mut self, workspace_id: WorkspaceId) -> anyhow::Result<()> {
        // TODO: Implement monocle layout logic
        log::debug!("Applied monocle layout to workspace {}", workspace_id);
        Ok(())
    }
    
    /// Apply graph-based layout
    fn apply_graph_based_layout(&mut self, workspace_id: WorkspaceId) -> anyhow::Result<()> {
        // TODO: Integrate with graph engine for positioning
        log::debug!("Applied graph-based layout to workspace {}", workspace_id);
        Ok(())
    }
    
    /// Evaluate window rules for a window
    fn evaluate_window_rules(&self, app_id: Option<&str>, title: Option<&str>) -> Vec<WindowAction> {
        let mut actions = Vec::new();
        
        for rule in &self.window_rules {
            if self.matches_window(&rule.matcher, app_id, title) {
                actions.extend(rule.actions.clone());
            }
        }
        
        // Sort by priority (higher priority first)
        actions.sort_by_key(|_| 0); // Simplified for now
        
        actions
    }
    
    /// Check if window matches rule
    fn matches_window(&self, matcher: &WindowMatcher, app_id: Option<&str>, title: Option<&str>) -> bool {
        if let Some(expected_app_id) = &matcher.app_id {
            if app_id.map(|id| id.contains(expected_app_id)).unwrap_or(false) {
                return true;
            }
        }
        
        if let Some(expected_title) = &matcher.title {
            if title.map(|t| t.contains(expected_title)).unwrap_or(false) {
                return true;
            }
        }
        
        false
    }
    
    /// Apply window action
    fn apply_window_action(&mut self, window_id: WindowId, action: WindowAction) -> anyhow::Result<()> {
        match action {
            WindowAction::SetFloating(floating) => {
                if floating {
                    self.floating_windows.insert(window_id);
                } else {
                    self.floating_windows.remove(&window_id);
                }
            }
            WindowAction::SetWorkspace(workspace_id) => {
                // Already handled in add_window
            }
            _ => {
                // TODO: Implement other actions
            }
        }
        Ok(())
    }
    
    /// Add window rule
    pub fn add_window_rule(&mut self, rule: WindowRule) {
        self.window_rules.push(rule);
        // Sort by priority
        self.window_rules.sort_by(|a, b| b.priority.cmp(&a.priority));
    }
    
    /// Get current workspace
    pub fn current_workspace(&self) -> Option<&Workspace> {
        self.workspaces.get(&self.active_workspace)
    }
    
    /// Get workspace by ID
    pub fn get_workspace(&self, workspace_id: WorkspaceId) -> Option<&Workspace> {
        self.workspaces.get(&workspace_id)
    }
    
    /// Get focused window
    pub fn focused_window(&self) -> Option<WindowId> {
        self.focus_history.front().copied()
    }
    
    /// Check if window is floating
    pub fn is_floating(&self, window_id: WindowId) -> bool {
        self.floating_windows.contains(&window_id)
    }
    
    /// Update animations
    pub fn update_animations(&mut self, delta_time: Duration) {
        let now = Instant::now();
        let mut completed_animations = Vec::new();
        
        for (window_id, animation) in &mut self.animations {
            let elapsed = now.duration_since(animation.start_time);
            if elapsed >= animation.duration {
                completed_animations.push(*window_id);
            } else {
                // Update animation progress
                let progress = elapsed.as_secs_f32() / animation.duration.as_secs_f32();
                let eased_progress = apply_easing(animation.easing, progress);
                
                // TODO: Apply animation transform
            }
        }
        
        // Remove completed animations
        for window_id in completed_animations {
            self.animations.remove(&window_id);
        }
    }
}

/// Apply easing function to progress value
fn apply_easing(easing: EasingFunction, t: f32) -> f32 {
    match easing {
        EasingFunction::Linear => t,
        EasingFunction::EaseIn => t * t,
        EasingFunction::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
        EasingFunction::EaseInOut => {
            if t < 0.5 {
                2.0 * t * t
            } else {
                1.0 - 2.0 * (1.0 - t) * (1.0 - t)
            }
        }
        EasingFunction::Bounce => {
            if t < 1.0 / 2.75 {
                7.5625 * t * t
            } else if t < 2.0 / 2.75 {
                let t = t - 1.5 / 2.75;
                7.5625 * t * t + 0.75
            } else if t < 2.5 / 2.75 {
                let t = t - 2.25 / 2.75;
                7.5625 * t * t + 0.9375
            } else {
                let t = t - 2.625 / 2.75;
                7.5625 * t * t + 0.984375
            }
        }
        EasingFunction::Elastic => {
            if t == 0.0 || t == 1.0 {
                t
            } else {
                let p = 0.3;
                let s = p / 4.0;
                -(2.0_f32.powf(10.0 * (t - 1.0)) * ((t - 1.0 - s) * (2.0 * std::f32::consts::PI) / p).sin())
            }
        }
    }
}

impl Default for WindowManagerConfig {
    fn default() -> Self {
        Self {
            default_layout: LayoutType::MasterStack,
            default_gaps: Gaps {
                inner: 5,
                outer: 10,
                top: 0,
                bottom: 0,
                left: 0,
                right: 0,
            },
            animations: AnimationConfig {
                enabled: true,
                duration: Duration::from_millis(250),
                easing: EasingFunction::EaseInOut,
                performance_mode: true,
            },
            focus: FocusConfig {
                follow_mouse: false,
                focus_new_windows: true,
                warp_mouse: false,
                history_size: 10,
            },
            workspaces: WorkspaceConfig {
                count: 4,
                auto_create: false,
                auto_switch: false,
                names: vec![
                    "Main".to_string(),
                    "Work".to_string(),
                    "Web".to_string(),
                    "Media".to_string(),
                ],
            },
        }
    }
}