//! Gesture recognition for touch and mouse inputs

use crate::{Gesture, SwipeDirection, KeyboardModifiers, MouseButton};
use winit::event::{Touch, TouchPhase, MouseButton as WinitMouseButton, ElementState};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Recognizes gestures from touch and mouse inputs
pub struct GestureRecognizer {
    /// Active touch points
    touches: HashMap<u64, TouchInfo>,
    /// Current gesture being recognized
    current_gesture: Option<Gesture>,
    /// Gesture start time
    gesture_start_time: Option<Instant>,
    /// Previous pinch distance
    last_pinch_distance: Option<f32>,
    /// Previous rotation angle
    last_rotation_angle: Option<f32>,
    /// Cursor position for mouse gestures
    cursor_pos: (f32, f32),
    /// Mouse button states
    mouse_buttons: HashMap<MouseButton, bool>,
    /// Current keyboard modifiers
    modifiers: KeyboardModifiers,
    /// Gesture history for pattern recognition
    gesture_history: Vec<GesturePoint>,
    /// Last tap time for multi-tap detection
    last_tap_time: Option<Instant>,
    /// Last tap position for multi-tap detection
    last_tap_position: Option<(f32, f32)>,
    /// Tap count for multi-tap gestures
    tap_count: u32,
    /// Velocity tracking for fling gestures
    velocity_tracker: VelocityTracker,
    /// Path tracking for shape recognition
    path_tracker: PathTracker,
}

#[derive(Clone, Debug)]
struct TouchInfo {
    id: u64,
    position: (f32, f32),
    start_position: (f32, f32),
    start_time: Instant,
    phase: TouchPhase,
}

/// Point in gesture history
#[derive(Clone, Debug)]
struct GesturePoint {
    position: (f32, f32),
    time: Instant,
    pressure: f32,
}

/// Velocity tracking for fling gestures
#[derive(Debug)]
struct VelocityTracker {
    points: Vec<GesturePoint>,
    max_history: usize,
}

impl VelocityTracker {
    fn new() -> Self {
        Self {
            points: Vec::new(),
            max_history: 10,
        }
    }
    
    fn add_point(&mut self, position: (f32, f32), pressure: f32) {
        self.points.push(GesturePoint {
            position,
            time: Instant::now(),
            pressure,
        });
        
        // Keep only recent points
        if self.points.len() > self.max_history {
            self.points.remove(0);
        }
    }
    
    fn get_velocity(&self) -> (f32, f32) {
        if self.points.len() < 2 {
            return (0.0, 0.0);
        }
        
        let last = &self.points[self.points.len() - 1];
        let first = &self.points[0];
        
        let dt = last.time.duration_since(first.time).as_secs_f32();
        if dt <= 0.0 {
            return (0.0, 0.0);
        }
        
        let dx = last.position.0 - first.position.0;
        let dy = last.position.1 - first.position.1;
        
        (dx / dt, dy / dt)
    }
}

/// Path tracking for shape recognition
#[derive(Debug)]
struct PathTracker {
    points: Vec<(f32, f32)>,
    max_points: usize,
}

impl PathTracker {
    fn new() -> Self {
        Self {
            points: Vec::new(),
            max_points: 100,
        }
    }
    
    fn add_point(&mut self, position: (f32, f32)) {
        self.points.push(position);
        
        // Keep only recent points
        if self.points.len() > self.max_points {
            self.points.remove(0);
        }
    }
    
    fn clear(&mut self) {
        self.points.clear();
    }
    
    /// Detect if path forms a circle
    fn detect_circle(&self) -> Option<(f32, f32, f32, bool)> {
        if self.points.len() < 10 {
            return None;
        }
        
        // Simple circle detection algorithm
        let center = self.calculate_centroid();
        let avg_radius = self.calculate_average_radius(center);
        let is_clockwise = self.is_clockwise();
        
        // Check if points roughly form a circle
        let radius_variance = self.calculate_radius_variance(center, avg_radius);
        if radius_variance < 0.3 && avg_radius > 20.0 {
            Some((center.0, center.1, avg_radius, is_clockwise))
        } else {
            None
        }
    }
    
    /// Detect if path forms a line
    fn detect_line(&self) -> Option<((f32, f32), (f32, f32), f32)> {
        if self.points.len() < 5 {
            return None;
        }
        
        let start = self.points[0];
        let end = self.points[self.points.len() - 1];
        let straightness = self.calculate_straightness();
        
        if straightness > 0.8 {
            Some((start, end, straightness))
        } else {
            None
        }
    }
    
    fn calculate_centroid(&self) -> (f32, f32) {
        let sum_x: f32 = self.points.iter().map(|p| p.0).sum();
        let sum_y: f32 = self.points.iter().map(|p| p.1).sum();
        let count = self.points.len() as f32;
        (sum_x / count, sum_y / count)
    }
    
    fn calculate_average_radius(&self, center: (f32, f32)) -> f32 {
        let sum_radius: f32 = self.points.iter()
            .map(|p| distance(*p, center))
            .sum();
        sum_radius / self.points.len() as f32
    }
    
    fn calculate_radius_variance(&self, center: (f32, f32), avg_radius: f32) -> f32 {
        let variance: f32 = self.points.iter()
            .map(|p| {
                let radius = distance(*p, center);
                let diff = radius - avg_radius;
                diff * diff
            })
            .sum();
        variance / self.points.len() as f32 / (avg_radius * avg_radius)
    }
    
    fn is_clockwise(&self) -> bool {
        if self.points.len() < 3 {
            return true;
        }
        
        let mut signed_area = 0.0;
        for i in 0..self.points.len() {
            let j = (i + 1) % self.points.len();
            signed_area += (self.points[j].0 - self.points[i].0) * (self.points[j].1 + self.points[i].1);
        }
        signed_area > 0.0
    }
    
    fn calculate_straightness(&self) -> f32 {
        if self.points.len() < 3 {
            return 1.0;
        }
        
        let start = self.points[0];
        let end = self.points[self.points.len() - 1];
        let direct_distance = distance(start, end);
        
        if direct_distance < 1.0 {
            return 0.0;
        }
        
        let path_length: f32 = self.points.windows(2)
            .map(|pair| distance(pair[0], pair[1]))
            .sum();
        
        direct_distance / path_length
    }
}

impl GestureRecognizer {
    /// Create a new gesture recognizer
    pub fn new() -> Self {
        Self {
            touches: HashMap::new(),
            current_gesture: None,
            gesture_start_time: None,
            last_pinch_distance: None,
            last_rotation_angle: None,
            cursor_pos: (0.0, 0.0),
            mouse_buttons: HashMap::new(),
            modifiers: KeyboardModifiers::default(),
            gesture_history: Vec::new(),
            last_tap_time: None,
            last_tap_position: None,
            tap_count: 0,
            velocity_tracker: VelocityTracker::new(),
            path_tracker: PathTracker::new(),
        }
    }
    
    /// Update cursor position for mouse-based gestures
    pub fn update_cursor(&mut self, pos: (f32, f32)) {
        self.cursor_pos = pos;
        self.velocity_tracker.add_point(pos, 1.0);
        self.path_tracker.add_point(pos);
    }
    
    /// Update keyboard modifiers
    pub fn update_modifiers(&mut self, modifiers: KeyboardModifiers) {
        self.modifiers = modifiers;
    }
    
    /// Handle mouse button press/release
    pub fn handle_mouse_button(&mut self, button: WinitMouseButton, state: ElementState) -> Option<Gesture> {
        let mouse_button = match button {
            WinitMouseButton::Left => MouseButton::Left,
            WinitMouseButton::Right => MouseButton::Right,
            WinitMouseButton::Middle => MouseButton::Middle,
            WinitMouseButton::Back => MouseButton::Other(4),
            WinitMouseButton::Forward => MouseButton::Other(5),
            WinitMouseButton::Other(id) => MouseButton::Other(id),
        };
        
        let pressed = state == ElementState::Pressed;
        self.mouse_buttons.insert(mouse_button, pressed);
        
        match (mouse_button, pressed) {
            (MouseButton::Left, true) => {
                self.handle_tap_detection();
                None
            }
            (MouseButton::Right, true) => {
                Some(Gesture::RightClick { position: self.cursor_pos })
            }
            (MouseButton::Middle, true) => {
                Some(Gesture::MiddleClick { position: self.cursor_pos })
            }
            _ => None,
        }
    }
    
    /// Handle mouse scroll
    pub fn handle_scroll(&mut self, delta: (f32, f32)) -> Option<Gesture> {
        Some(Gesture::Scroll {
            delta,
            modifiers: self.modifiers,
        })
    }
    
    /// Handle tap detection for multi-tap gestures
    fn handle_tap_detection(&mut self) -> Option<Gesture> {
        let now = Instant::now();
        let position = self.cursor_pos;
        
        // Check if this is part of a multi-tap sequence
        if let (Some(last_time), Some(last_pos)) = (self.last_tap_time, self.last_tap_position) {
            let time_diff = now.duration_since(last_time);
            let distance = distance(position, last_pos);
            
            // Multi-tap detection thresholds
            if time_diff < Duration::from_millis(300) && distance < 30.0 {
                self.tap_count += 1;
            } else {
                self.tap_count = 1;
            }
        } else {
            self.tap_count = 1;
        }
        
        self.last_tap_time = Some(now);
        self.last_tap_position = Some(position);
        
        // Return appropriate gesture based on tap count
        match self.tap_count {
            1 => Some(Gesture::Tap { position }),
            2 => Some(Gesture::DoubleTap { position }),
            3 => Some(Gesture::TripleTap { position }),
            _ => None,
        }
    }
    
    /// Process a touch event
    pub fn process_touch(&mut self, touch: &Touch) {
        let touch_info = TouchInfo {
            id: touch.id,
            position: (touch.location.x as f32, touch.location.y as f32),
            start_position: (touch.location.x as f32, touch.location.y as f32),
            start_time: Instant::now(),
            phase: touch.phase,
        };
        
        match touch.phase {
            TouchPhase::Started => {
                self.touches.insert(touch.id, touch_info);
                self.gesture_start_time = Some(Instant::now());
                self.analyze_gesture();
            }
            TouchPhase::Moved => {
                if let Some(existing) = self.touches.get_mut(&touch.id) {
                    existing.position = (touch.location.x as f32, touch.location.y as f32);
                    existing.phase = touch.phase;
                }
                self.analyze_gesture();
            }
            TouchPhase::Ended | TouchPhase::Cancelled => {
                if let Some(touch_info) = self.touches.remove(&touch.id) {
                    // Check for tap or swipe on release
                    self.check_tap_or_swipe(&touch_info);
                }
                
                if self.touches.is_empty() {
                    self.reset();
                }
            }
        }
    }
    
    /// Analyze current touches to determine gesture
    fn analyze_gesture(&mut self) {
        match self.touches.len() {
            1 => self.analyze_single_touch(),
            2 => self.analyze_two_touch(),
            _ => {}
        }
    }
    
    /// Analyze single touch gestures
    fn analyze_single_touch(&mut self) {
        if let Some(touch) = self.touches.values().next() {
            let delta = (
                touch.position.0 - touch.start_position.0,
                touch.position.1 - touch.start_position.1,
            );
            
            // Check for pan gesture
            if delta.0.abs() > 5.0 || delta.1.abs() > 5.0 {
                let velocity = self.velocity_tracker.get_velocity();
                self.current_gesture = Some(Gesture::Pan { delta, velocity });
            }
            
            // Check for long press
            if touch.start_time.elapsed() > Duration::from_millis(500) {
                let distance = ((touch.position.0 - touch.start_position.0).powi(2) +
                               (touch.position.1 - touch.start_position.1).powi(2)).sqrt();
                if distance < 10.0 {
                    let duration = touch.start_time.elapsed().as_secs_f32();
                    self.current_gesture = Some(Gesture::LongPress { 
                        position: touch.position,
                        duration,
                    });
                }
            }
        }
    }
    
    /// Analyze two-touch gestures (pinch and rotate)
    fn analyze_two_touch(&mut self) {
        let touches: Vec<&TouchInfo> = self.touches.values().collect();
        if touches.len() != 2 {
            return;
        }
        
        let touch1 = touches[0];
        let touch2 = touches[1];
        
        // Calculate current distance and angle
        let current_distance = distance(touch1.position, touch2.position);
        let current_angle = angle(touch1.position, touch2.position);
        
        // Calculate center point
        let center = (
            (touch1.position.0 + touch2.position.0) / 2.0,
            (touch1.position.1 + touch2.position.1) / 2.0,
        );
        
        // Detect pinch
        if let Some(last_distance) = self.last_pinch_distance {
            let scale = current_distance / last_distance;
            if (scale - 1.0).abs() > 0.01 {
                let velocity = (current_distance - last_distance) * 60.0; // Approximate velocity
                self.current_gesture = Some(Gesture::Pinch { scale, center, velocity });
            }
        }
        
        // Detect rotation
        if let Some(last_angle) = self.last_rotation_angle {
            let angle_diff = current_angle - last_angle;
            if angle_diff.abs() > 0.01 {
                let angular_velocity = angle_diff * 60.0; // Approximate angular velocity
                self.current_gesture = Some(Gesture::Rotate { 
                    angle: angle_diff, 
                    center, 
                    angular_velocity 
                });
            }
        }
        
        self.last_pinch_distance = Some(current_distance);
        self.last_rotation_angle = Some(current_angle);
    }
    
    /// Check for tap or swipe when touch ends
    fn check_tap_or_swipe(&mut self, touch_info: &TouchInfo) {
        let distance = ((touch_info.position.0 - touch_info.start_position.0).powi(2) +
                       (touch_info.position.1 - touch_info.start_position.1).powi(2)).sqrt();
        let duration = touch_info.start_time.elapsed();
        
        if distance < 10.0 && duration < Duration::from_millis(300) {
            // Check for double tap
            if self.gesture_start_time.map(|t| t.elapsed() < Duration::from_millis(500)).unwrap_or(false) {
                self.current_gesture = Some(Gesture::DoubleTap { position: touch_info.position });
            } else {
                self.current_gesture = Some(Gesture::Tap { position: touch_info.position });
            }
        } else if distance > 50.0 && duration < Duration::from_millis(500) {
            // Swipe gesture
            let velocity = distance / duration.as_secs_f32();
            let dx = touch_info.position.0 - touch_info.start_position.0;
            let dy = touch_info.position.1 - touch_info.start_position.1;
            
            let direction = if dx.abs() > dy.abs() {
                if dx > 0.0 { SwipeDirection::Right } else { SwipeDirection::Left }
            } else {
                if dy > 0.0 { SwipeDirection::Down } else { SwipeDirection::Up }
            };
            
            self.current_gesture = Some(Gesture::Swipe { direction, velocity, distance });
        }
    }
    
    /// Get the current recognized gesture
    pub fn get_gesture(&mut self) -> Option<Gesture> {
        self.current_gesture.take()
    }
    
    /// Reset gesture recognition state
    fn reset(&mut self) {
        self.current_gesture = None;
        self.last_pinch_distance = None;
        self.last_rotation_angle = None;
    }
}

/// Calculate distance between two points
fn distance(p1: (f32, f32), p2: (f32, f32)) -> f32 {
    ((p2.0 - p1.0).powi(2) + (p2.1 - p1.1).powi(2)).sqrt()
}

/// Calculate angle between two points
fn angle(p1: (f32, f32), p2: (f32, f32)) -> f32 {
    (p2.1 - p1.1).atan2(p2.0 - p1.0)
}