//! Interaction system for HorizonOS graph desktop
//! 
//! This module provides comprehensive input handling including:
//! - Mouse/trackpad interactions (click, drag, scroll)
//! - Keyboard navigation and shortcuts
//! - Touch gestures (pinch, swipe, rotate)
//! - Voice command integration
//! - Node selection and manipulation
//! - Camera controls

pub mod input;
pub mod selection;
pub mod gestures;
pub mod camera_controls;
pub mod context_menu;
pub mod drag_drop;
pub mod advanced;

pub use input::*;
pub use selection::*;
pub use gestures::*;
pub use camera_controls::*;
pub use context_menu::*;
pub use drag_drop::*;
pub use advanced::*;

use horizonos_graph_engine::{GraphEngine, SceneId, Position, Camera, Ray};
use horizonos_graph_nodes::GraphNode;
use std::sync::{Arc, RwLock};
use winit::event::{Event, WindowEvent, ElementState};
use winit::keyboard::{KeyCode, PhysicalKey};

/// Main interaction manager that coordinates all input handling
pub struct InteractionManager {
    /// Input handler for low-level events
    input_handler: InputHandler,
    /// Selection system for nodes
    selection_manager: SelectionManager,
    /// Gesture recognizer
    gesture_recognizer: GestureRecognizer,
    /// Camera controller
    camera_controller: CameraController,
    /// Context menu system
    context_menu: ContextMenuManager,
    /// Drag and drop handler
    drag_drop_handler: DragDropHandler,
    /// Advanced interaction features
    advanced_manager: AdvancedInteractionManager,
    /// Current interaction mode
    mode: InteractionMode,
    /// Callback handlers
    callbacks: Arc<RwLock<InteractionCallbacks>>,
}

/// Different interaction modes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InteractionMode {
    /// Normal selection and navigation
    Normal,
    /// Panning the camera
    Pan,
    /// Rotating the camera
    Rotate,
    /// Dragging nodes
    Drag,
    /// Box selection
    BoxSelect,
    /// Creating edges
    EdgeCreate,
}

/// Callbacks for interaction events
#[derive(Default)]
pub struct InteractionCallbacks {
    pub on_node_click: Option<Box<dyn Fn(SceneId) + Send + Sync>>,
    pub on_node_double_click: Option<Box<dyn Fn(SceneId) + Send + Sync>>,
    pub on_node_drag: Option<Box<dyn Fn(SceneId, Position) + Send + Sync>>,
    pub on_selection_changed: Option<Box<dyn Fn(Vec<SceneId>) + Send + Sync>>,
    pub on_edge_create: Option<Box<dyn Fn(SceneId, SceneId) + Send + Sync>>,
    pub on_context_menu: Option<Box<dyn Fn(SceneId, Position) + Send + Sync>>,
}

impl InteractionManager {
    /// Create a new interaction manager
    pub fn new() -> Self {
        Self {
            input_handler: InputHandler::new(),
            selection_manager: SelectionManager::new(),
            gesture_recognizer: GestureRecognizer::new(),
            camera_controller: CameraController::new(),
            context_menu: ContextMenuManager::new(),
            drag_drop_handler: DragDropHandler::new(),
            advanced_manager: AdvancedInteractionManager::new(),
            mode: InteractionMode::Normal,
            callbacks: Arc::new(RwLock::new(InteractionCallbacks::default())),
        }
    }
    
    /// Handle a winit event
    pub fn handle_event(&mut self, event: &Event<()>, engine: &mut GraphEngine) -> bool {
        match event {
            Event::WindowEvent { event, .. } => {
                self.handle_window_event(event, engine)
            }
            _ => false,
        }
    }
    
    /// Handle window events
    fn handle_window_event(&mut self, event: &WindowEvent, engine: &mut GraphEngine) -> bool {
        // First, update input state
        self.input_handler.handle_event(event);
        
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.handle_cursor_moved(*position, engine);
                true
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.handle_mouse_input(*state, *button, engine);
                true
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.handle_mouse_wheel(*delta, engine);
                true
            }
            WindowEvent::KeyboardInput { event, .. } => {
                self.handle_keyboard_input(event, engine);
                true
            }
            WindowEvent::Touch(touch) => {
                self.handle_touch(touch, engine);
                true
            }
            _ => false,
        }
    }
    
    /// Handle cursor movement
    fn handle_cursor_moved(&mut self, position: winit::dpi::PhysicalPosition<f64>, engine: &mut GraphEngine) {
        let pos = (position.x as f32, position.y as f32);
        
        // Update gesture recognizer
        self.gesture_recognizer.update_cursor(pos);
        
        // Handle different modes
        match self.mode {
            InteractionMode::Pan => {
                if self.input_handler.is_mouse_pressed(winit::event::MouseButton::Middle) {
                    self.camera_controller.pan(engine.camera_mut(), pos, self.input_handler.last_cursor_pos());
                }
            }
            InteractionMode::Rotate => {
                if self.input_handler.is_mouse_pressed(winit::event::MouseButton::Right) {
                    self.camera_controller.rotate(engine.camera_mut(), pos, self.input_handler.last_cursor_pos());
                }
            }
            InteractionMode::Drag => {
                if let Some(selected) = self.selection_manager.get_primary_selection() {
                    self.drag_drop_handler.update_drag(selected, pos, engine);
                }
            }
            InteractionMode::BoxSelect => {
                self.selection_manager.update_box_selection(pos);
            }
            _ => {
                // Update hover state
                if let Some(node_id) = self.pick_node_at(pos, engine) {
                    self.selection_manager.set_hover(Some(node_id));
                } else {
                    self.selection_manager.set_hover(None);
                }
            }
        }
    }
    
    /// Handle mouse button input
    fn handle_mouse_input(&mut self, state: ElementState, button: winit::event::MouseButton, engine: &mut GraphEngine) {
        let cursor_pos = self.input_handler.cursor_pos();
        
        match (state, button) {
            (ElementState::Pressed, winit::event::MouseButton::Left) => {
                // Check for node selection
                if let Some(node_id) = self.pick_node_at(cursor_pos, engine) {
                    let shift_pressed = self.input_handler.is_key_pressed(KeyCode::ShiftLeft);
                    let ctrl_pressed = self.input_handler.is_key_pressed(KeyCode::ControlLeft);
                    
                    if ctrl_pressed {
                        // Toggle selection
                        self.selection_manager.toggle_selection(node_id);
                    } else if shift_pressed {
                        // Add to selection
                        self.selection_manager.add_to_selection(node_id);
                    } else {
                        // Single selection
                        self.selection_manager.set_selection(vec![node_id]);
                        
                        // Advanced: Handle node selection for context awareness
                        self.advanced_manager.handle_node_selection(node_id, engine);
                        
                        // Check for double-click
                        if self.input_handler.is_double_click() {
                            if let Some(callback) = &self.callbacks.read().unwrap().on_node_double_click {
                                callback(node_id);
                            }
                        } else {
                            if let Some(callback) = &self.callbacks.read().unwrap().on_node_click {
                                callback(node_id);
                            }
                        }
                    }
                    
                    // Start drag mode
                    self.mode = InteractionMode::Drag;
                    self.drag_drop_handler.start_drag(node_id, cursor_pos);
                } else {
                    // Start box selection
                    self.mode = InteractionMode::BoxSelect;
                    self.selection_manager.start_box_selection(cursor_pos);
                }
            }
            (ElementState::Released, winit::event::MouseButton::Left) => {
                match self.mode {
                    InteractionMode::Drag => {
                        self.drag_drop_handler.end_drag();
                        self.mode = InteractionMode::Normal;
                    }
                    InteractionMode::BoxSelect => {
                        let selected = self.selection_manager.finish_box_selection(engine);
                        if let Some(callback) = &self.callbacks.read().unwrap().on_selection_changed {
                            callback(selected);
                        }
                        self.mode = InteractionMode::Normal;
                    }
                    _ => {}
                }
            }
            (ElementState::Pressed, winit::event::MouseButton::Right) => {
                // Context menu or camera rotation
                if let Some(node_id) = self.pick_node_at(cursor_pos, engine) {
                    self.context_menu.show_for_node(node_id, cursor_pos);
                    if let Some(callback) = &self.callbacks.read().unwrap().on_context_menu {
                        let world_pos = self.screen_to_world(cursor_pos, engine);
                        callback(node_id, world_pos);
                    }
                } else {
                    self.mode = InteractionMode::Rotate;
                }
            }
            (ElementState::Released, winit::event::MouseButton::Right) => {
                if self.mode == InteractionMode::Rotate {
                    self.mode = InteractionMode::Normal;
                }
            }
            (ElementState::Pressed, winit::event::MouseButton::Middle) => {
                self.mode = InteractionMode::Pan;
            }
            (ElementState::Released, winit::event::MouseButton::Middle) => {
                if self.mode == InteractionMode::Pan {
                    self.mode = InteractionMode::Normal;
                }
            }
            _ => {}
        }
    }
    
    /// Handle mouse wheel input
    fn handle_mouse_wheel(&mut self, delta: winit::event::MouseScrollDelta, engine: &mut GraphEngine) {
        use winit::event::MouseScrollDelta;
        
        let zoom_delta = match delta {
            MouseScrollDelta::LineDelta(_, y) => y * 0.1,
            MouseScrollDelta::PixelDelta(pos) => (pos.y as f32) * 0.001,
        };
        
        self.camera_controller.zoom(engine.camera_mut(), zoom_delta);
    }
    
    /// Handle keyboard input
    fn handle_keyboard_input(&mut self, event: &winit::event::KeyEvent, engine: &mut GraphEngine) {
        if event.state != ElementState::Pressed {
            return;
        }
        
        match event.physical_key {
            PhysicalKey::Code(KeyCode::Delete) => {
                // Delete selected nodes
                let selected = self.selection_manager.get_selection();
                // TODO: Implement node deletion
            }
            PhysicalKey::Code(KeyCode::KeyA) if self.input_handler.is_key_pressed(KeyCode::ControlLeft) => {
                // Select all
                self.selection_manager.select_all(engine);
            }
            PhysicalKey::Code(KeyCode::Escape) => {
                // Clear selection
                self.selection_manager.clear_selection();
                self.mode = InteractionMode::Normal;
            }
            PhysicalKey::Code(KeyCode::KeyF) => {
                // Focus on selected nodes
                if let Some(selected) = self.selection_manager.get_selection().first() {
                    self.camera_controller.focus_on_node(*selected, engine);
                }
            }
            _ => {}
        }
    }
    
    /// Handle touch input
    fn handle_touch(&mut self, touch: &winit::event::Touch, engine: &mut GraphEngine) {
        self.gesture_recognizer.process_touch(touch);
        
        // Check for recognized gestures
        if let Some(gesture) = self.gesture_recognizer.get_gesture() {
            match gesture {
                Gesture::Pinch { scale, center } => {
                    self.camera_controller.zoom(engine.camera_mut(), scale - 1.0);
                }
                Gesture::Pan { delta } => {
                    let current = self.input_handler.cursor_pos();
                    let previous = (current.0 - delta.0, current.1 - delta.1);
                    self.camera_controller.pan(engine.camera_mut(), current, previous);
                }
                Gesture::Rotate { angle, center } => {
                    self.camera_controller.rotate_around(engine.camera_mut(), center, angle);
                }
                _ => {}
            }
        }
    }
    
    /// Pick a node at screen coordinates
    fn pick_node_at(&self, screen_pos: (f32, f32), engine: &GraphEngine) -> Option<SceneId> {
        // Convert screen coordinates to ray
        let ray = self.camera_controller.screen_to_ray(screen_pos, engine.camera(), engine.window_size());
        
        // Perform ray-node intersection test
        self.selection_manager.ray_pick_node(&ray, engine.scene())
    }
    
    /// Convert screen coordinates to world position
    fn screen_to_world(&self, screen_pos: (f32, f32), engine: &GraphEngine) -> Position {
        self.camera_controller.screen_to_world(screen_pos, engine.camera(), engine.window_size())
    }
    
    /// Set a callback for node clicks
    pub fn on_node_click<F>(&mut self, callback: F) 
    where
        F: Fn(SceneId) + Send + Sync + 'static,
    {
        self.callbacks.write().unwrap().on_node_click = Some(Box::new(callback));
    }
    
    /// Set a callback for node double-clicks
    pub fn on_node_double_click<F>(&mut self, callback: F)
    where
        F: Fn(SceneId) + Send + Sync + 'static,
    {
        self.callbacks.write().unwrap().on_node_double_click = Some(Box::new(callback));
    }
    
    /// Set a callback for selection changes
    pub fn on_selection_changed<F>(&mut self, callback: F)
    where
        F: Fn(Vec<SceneId>) + Send + Sync + 'static,
    {
        self.callbacks.write().unwrap().on_selection_changed = Some(Box::new(callback));
    }
    
    /// Get current interaction mode
    pub fn mode(&self) -> InteractionMode {
        self.mode
    }
    
    /// Set interaction mode
    pub fn set_mode(&mut self, mode: InteractionMode) {
        self.mode = mode;
    }
    
    /// Get the selection manager
    pub fn selection(&self) -> &SelectionManager {
        &self.selection_manager
    }
    
    /// Get the camera controller
    pub fn camera_controller(&self) -> &CameraController {
        &self.camera_controller
    }
    
    /// Get selected nodes
    pub fn get_selected_nodes(&self) -> Vec<SceneId> {
        self.selection_manager.get_selection()
    }
    
    /// Get current camera state for rendering
    pub fn get_camera_state(&self) -> Option<CameraState> {
        Some(self.camera_controller.get_state())
    }
    
    // Advanced interaction features
    
    /// Update advanced interaction systems with current state
    pub fn update_advanced_features(&mut self, engine: &GraphEngine, cluster_manager: &horizonos_graph_clustering::ClusterManager) {
        self.advanced_manager.update(engine, cluster_manager);
    }
    
    /// Bring a node to the foreground (advanced feature)
    pub fn bring_to_foreground(&mut self, node_id: SceneId, engine: &mut GraphEngine) {
        self.advanced_manager.bring_to_foreground(node_id, engine);
    }
    
    /// Auto-flatten overlapping nodes in a specific area
    pub fn auto_flatten_area(&mut self, center: Position, radius: f32, engine: &mut GraphEngine) {
        self.advanced_manager.auto_flatten_area(center, radius, engine);
    }
    
    /// Navigate to a semantically related node
    pub fn navigate_to_related(&mut self, from_node: SceneId, engine: &mut GraphEngine) -> Option<SceneId> {
        self.advanced_manager.navigate_to_related(from_node, engine)
    }
    
    /// Get the current focus stack (most recently focused nodes)
    pub fn get_focus_stack(&self) -> &std::collections::VecDeque<SceneId> {
        self.advanced_manager.get_focus_stack()
    }
    
    /// Get auto-flatten settings for configuration
    pub fn auto_flatten_settings(&mut self) -> &mut AutoFlattenManager {
        self.advanced_manager.auto_flatten_settings()
    }
    
    /// Get focus manager for configuration
    pub fn focus_manager(&mut self) -> &mut FocusManager {
        self.advanced_manager.focus_manager()
    }
    
    /// Get navigation manager for configuration
    pub fn navigation_manager(&mut self) -> &mut SmartNavigationManager {
        self.advanced_manager.navigation_manager()
    }
    
    /// Enable or disable auto-flatten feature
    pub fn set_auto_flatten_enabled(&mut self, enabled: bool) {
        self.advanced_manager.auto_flatten_settings().set_enabled(enabled);
    }
    
    /// Set the overlap threshold for auto-flatten detection
    pub fn set_auto_flatten_threshold(&mut self, threshold: f32) {
        self.advanced_manager.auto_flatten_settings().set_overlap_threshold(threshold);
    }
    
    /// Configure auto-focus behavior
    pub fn configure_auto_focus(&mut self, focus_on_click: bool, focus_on_hover: bool, fade_unfocused: bool) {
        let settings = self.advanced_manager.focus_manager().settings();
        settings.focus_on_click = focus_on_click;
        settings.focus_on_hover = focus_on_hover;
        settings.fade_unfocused = fade_unfocused;
    }
}

/// Gesture types recognized by the system
#[derive(Debug, Clone)]
pub enum Gesture {
    Tap { position: (f32, f32) },
    DoubleTap { position: (f32, f32) },
    LongPress { position: (f32, f32) },
    Pan { delta: (f32, f32) },
    Pinch { scale: f32, center: (f32, f32) },
    Rotate { angle: f32, center: (f32, f32) },
    Swipe { direction: SwipeDirection, velocity: f32 },
}

/// Swipe directions
#[derive(Debug, Clone, Copy)]
pub enum SwipeDirection {
    Up,
    Down,
    Left,
    Right,
}

