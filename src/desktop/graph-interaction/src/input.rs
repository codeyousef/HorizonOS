//! Low-level input handling and state tracking

use winit::event::{WindowEvent, ElementState, MouseButton};
use winit::keyboard::KeyCode;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

/// Handles raw input events and maintains input state
pub struct InputHandler {
    /// Currently pressed keys
    pressed_keys: HashSet<KeyCode>,
    /// Currently pressed mouse buttons
    pressed_mouse_buttons: HashSet<MouseButton>,
    /// Current cursor position
    cursor_pos: (f32, f32),
    /// Last cursor position
    last_cursor_pos: (f32, f32),
    /// Double-click detection
    last_click_time: Option<Instant>,
    last_click_pos: Option<(f32, f32)>,
    /// Mouse button press times for gestures
    button_press_times: HashMap<MouseButton, Instant>,
    /// Key press times for held key detection
    key_press_times: HashMap<KeyCode, Instant>,
}

impl InputHandler {
    /// Create a new input handler
    pub fn new() -> Self {
        Self {
            pressed_keys: HashSet::new(),
            pressed_mouse_buttons: HashSet::new(),
            cursor_pos: (0.0, 0.0),
            last_cursor_pos: (0.0, 0.0),
            last_click_time: None,
            last_click_pos: None,
            button_press_times: HashMap::new(),
            key_press_times: HashMap::new(),
        }
    }
    
    /// Process a window event
    pub fn handle_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.last_cursor_pos = self.cursor_pos;
                self.cursor_pos = (position.x as f32, position.y as f32);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                match state {
                    ElementState::Pressed => {
                        self.pressed_mouse_buttons.insert(*button);
                        self.button_press_times.insert(*button, Instant::now());
                        
                        // Check for double-click
                        if *button == MouseButton::Left {
                            let now = Instant::now();
                            let is_double_click = if let (Some(last_time), Some(last_pos)) = 
                                (self.last_click_time, self.last_click_pos) {
                                let time_diff = now.duration_since(last_time);
                                let pos_diff = ((self.cursor_pos.0 - last_pos.0).powi(2) + 
                                               (self.cursor_pos.1 - last_pos.1).powi(2)).sqrt();
                                time_diff < Duration::from_millis(500) && pos_diff < 5.0
                            } else {
                                false
                            };
                            
                            if !is_double_click {
                                self.last_click_time = Some(now);
                                self.last_click_pos = Some(self.cursor_pos);
                            }
                        }
                    }
                    ElementState::Released => {
                        self.pressed_mouse_buttons.remove(button);
                        self.button_press_times.remove(button);
                    }
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let winit::keyboard::PhysicalKey::Code(key_code) = event.physical_key {
                    match event.state {
                        ElementState::Pressed => {
                            if !self.pressed_keys.contains(&key_code) {
                                self.pressed_keys.insert(key_code);
                                self.key_press_times.insert(key_code, Instant::now());
                            }
                        }
                        ElementState::Released => {
                            self.pressed_keys.remove(&key_code);
                            self.key_press_times.remove(&key_code);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    
    /// Check if a key is currently pressed
    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.pressed_keys.contains(&key)
    }
    
    /// Check if a mouse button is currently pressed
    pub fn is_mouse_pressed(&self, button: MouseButton) -> bool {
        self.pressed_mouse_buttons.contains(&button)
    }
    
    /// Get current cursor position
    pub fn cursor_pos(&self) -> (f32, f32) {
        self.cursor_pos
    }
    
    /// Get last cursor position
    pub fn last_cursor_pos(&self) -> (f32, f32) {
        self.last_cursor_pos
    }
    
    /// Get cursor delta since last frame
    pub fn cursor_delta(&self) -> (f32, f32) {
        (
            self.cursor_pos.0 - self.last_cursor_pos.0,
            self.cursor_pos.1 - self.last_cursor_pos.1,
        )
    }
    
    /// Check if the last mouse click was a double-click
    pub fn is_double_click(&self) -> bool {
        if let (Some(last_time), Some(last_pos)) = (self.last_click_time, self.last_click_pos) {
            let now = Instant::now();
            let time_diff = now.duration_since(last_time);
            let pos_diff = ((self.cursor_pos.0 - last_pos.0).powi(2) + 
                           (self.cursor_pos.1 - last_pos.1).powi(2)).sqrt();
            time_diff < Duration::from_millis(500) && pos_diff < 5.0
        } else {
            false
        }
    }
    
    /// Get how long a key has been held
    pub fn key_held_duration(&self, key: KeyCode) -> Option<Duration> {
        self.key_press_times.get(&key).map(|&time| time.elapsed())
    }
    
    /// Get how long a mouse button has been held
    pub fn button_held_duration(&self, button: MouseButton) -> Option<Duration> {
        self.button_press_times.get(&button).map(|&time| time.elapsed())
    }
    
    /// Check if any modifier key is pressed
    pub fn any_modifier_pressed(&self) -> bool {
        self.is_key_pressed(KeyCode::ControlLeft) ||
        self.is_key_pressed(KeyCode::ControlRight) ||
        self.is_key_pressed(KeyCode::ShiftLeft) ||
        self.is_key_pressed(KeyCode::ShiftRight) ||
        self.is_key_pressed(KeyCode::AltLeft) ||
        self.is_key_pressed(KeyCode::AltRight) ||
        self.is_key_pressed(KeyCode::SuperLeft) ||
        self.is_key_pressed(KeyCode::SuperRight)
    }
    
    /// Reset input state
    pub fn reset(&mut self) {
        self.pressed_keys.clear();
        self.pressed_mouse_buttons.clear();
        self.button_press_times.clear();
        self.key_press_times.clear();
        self.last_click_time = None;
        self.last_click_pos = None;
    }
}