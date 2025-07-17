//! Gesture recognition for touch and mouse inputs

use crate::{Gesture, SwipeDirection};
use winit::event::{Touch, TouchPhase};
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
}

#[derive(Clone, Debug)]
struct TouchInfo {
    id: u64,
    position: (f32, f32),
    start_position: (f32, f32),
    start_time: Instant,
    phase: TouchPhase,
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
        }
    }
    
    /// Update cursor position for mouse-based gestures
    pub fn update_cursor(&mut self, pos: (f32, f32)) {
        self.cursor_pos = pos;
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
                self.current_gesture = Some(Gesture::Pan { delta });
            }
            
            // Check for long press
            if touch.start_time.elapsed() > Duration::from_millis(500) {
                let distance = ((touch.position.0 - touch.start_position.0).powi(2) +
                               (touch.position.1 - touch.start_position.1).powi(2)).sqrt();
                if distance < 10.0 {
                    self.current_gesture = Some(Gesture::LongPress { position: touch.position });
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
                self.current_gesture = Some(Gesture::Pinch { scale, center });
            }
        }
        
        // Detect rotation
        if let Some(last_angle) = self.last_rotation_angle {
            let angle_diff = current_angle - last_angle;
            if angle_diff.abs() > 0.01 {
                self.current_gesture = Some(Gesture::Rotate { angle: angle_diff, center });
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
            
            self.current_gesture = Some(Gesture::Swipe { direction, velocity });
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