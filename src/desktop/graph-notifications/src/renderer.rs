//! Notification rendering for the graph desktop

use crate::{
    Notification, NotificationPosition, NotificationPriority, NotificationRenderTarget,
    NotificationAnimation, SlideDirection
};
use anyhow::Result;
use nalgebra::{Point3, Vector3};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;
use horizonos_graph_nodes::NodeVisualData;

/// Graph-integrated notification renderer
pub struct NotificationRenderer {
    /// Active notification visuals
    active_visuals: Arc<RwLock<HashMap<Uuid, NotificationVisual>>>,
    /// Notification layout manager
    layout: NotificationLayout,
    /// Animation states
    animations: Arc<RwLock<HashMap<Uuid, AnimationState>>>,
}

/// Visual representation of a notification
#[derive(Debug, Clone)]
pub struct NotificationVisual {
    /// Notification ID
    pub id: Uuid,
    /// Position in 3D space
    pub position: Point3<f32>,
    /// Size
    pub size: Vector3<f32>,
    /// Visual data
    pub visual_data: NodeVisualData,
    /// Opacity
    pub opacity: f32,
    /// Scale
    pub scale: f32,
    /// Z-index for layering
    pub z_index: u32,
    /// Attached to node
    pub attached_node: Option<u64>,
}

/// Notification layout manager
pub struct NotificationLayout {
    /// Current position setting
    position: NotificationPosition,
    /// Screen dimensions
    screen_size: (f32, f32),
    /// Notification spacing
    spacing: f32,
    /// Maximum stack size
    max_stack: usize,
    /// Current stack
    stack: Vec<Uuid>,
}

/// Animation state
#[derive(Debug, Clone)]
struct AnimationState {
    /// Current animation
    animation: NotificationAnimation,
    /// Start time
    start_time: std::time::Instant,
    /// Initial state
    initial: AnimationFrame,
    /// Target state
    target: AnimationFrame,
}

/// Animation frame
#[derive(Debug, Clone, Copy)]
struct AnimationFrame {
    position: Point3<f32>,
    opacity: f32,
    scale: f32,
}

impl NotificationRenderer {
    /// Create a new notification renderer
    pub fn new(screen_size: (f32, f32)) -> Self {
        Self {
            active_visuals: Arc::new(RwLock::new(HashMap::new())),
            layout: NotificationLayout::new(screen_size),
            animations: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Update screen size
    pub fn update_screen_size(&mut self, width: f32, height: f32) {
        self.layout.screen_size = (width, height);
        self.relayout();
    }
    
    /// Update notification position setting
    pub fn set_position(&mut self, position: NotificationPosition) {
        self.layout.position = position;
        self.relayout();
    }
    
    /// Create visual for notification
    pub fn create_visual(&self, notification: &Notification) -> NotificationVisual {
        let mut visual_data = NodeVisualData::default();
        
        // Set color based on notification type and priority
        visual_data.color = match notification.priority {
            NotificationPriority::Critical => [1.0, 0.2, 0.2, 0.95], // Red
            NotificationPriority::High => [1.0, 0.6, 0.0, 0.95],     // Orange
            NotificationPriority::Normal => [0.2, 0.6, 1.0, 0.95],   // Blue
            NotificationPriority::Low => [0.6, 0.6, 0.6, 0.95],      // Gray
        };
        
        // Set icon
        visual_data.icon = notification.icon.clone();
        
        // Set size based on content
        let has_image = notification.image.is_some();
        let action_count = notification.actions.len();
        let base_size = if has_image { 2.5 } else { 2.0 };
        let height = base_size + (action_count as f32 * 0.3);
        
        NotificationVisual {
            id: notification.id,
            position: Point3::new(0.0, 0.0, 0.0), // Will be set by layout
            size: Vector3::new(4.0, height, 0.1),
            visual_data,
            opacity: 1.0,
            scale: 1.0,
            z_index: 1000, // Notifications on top
            attached_node: notification.node_id,
        }
    }
    
    /// Add notification to renderer
    pub fn add_notification(&mut self, notification: &Notification) {
        let visual = self.create_visual(notification);
        let position = self.layout.get_position_for_notification(&visual);
        
        let mut visual = visual;
        visual.position = position;
        
        self.active_visuals.write().unwrap().insert(notification.id, visual);
        self.layout.stack.push(notification.id);
        
        // Limit stack size
        if self.layout.stack.len() > self.layout.max_stack {
            let old_id = self.layout.stack.remove(0);
            self.active_visuals.write().unwrap().remove(&old_id);
        }
    }
    
    /// Remove notification from renderer
    pub fn remove_notification(&mut self, id: Uuid) {
        self.active_visuals.write().unwrap().remove(&id);
        self.layout.stack.retain(|&nid| nid != id);
        self.animations.write().unwrap().remove(&id);
        self.relayout();
    }
    
    /// Start animation for notification
    pub fn start_animation(&mut self, id: Uuid, animation: NotificationAnimation) {
        if let Some(visual) = self.active_visuals.read().unwrap().get(&id) {
            let initial = AnimationFrame {
                position: visual.position,
                opacity: visual.opacity,
                scale: visual.scale,
            };
            
            let target = match &animation {
                NotificationAnimation::SlideIn { direction, .. } => {
                    let offset = self.get_slide_offset(*direction);
                    AnimationFrame {
                        position: visual.position,
                        opacity: 1.0,
                        scale: 1.0,
                    }
                }
                NotificationAnimation::SlideOut { direction, .. } => {
                    let offset = self.get_slide_offset(*direction);
                    AnimationFrame {
                        position: visual.position + offset,
                        opacity: 0.0,
                        scale: 1.0,
                    }
                }
                NotificationAnimation::FadeIn { .. } => AnimationFrame {
                    position: visual.position,
                    opacity: 1.0,
                    scale: 1.0,
                },
                NotificationAnimation::FadeOut { .. } => AnimationFrame {
                    position: visual.position,
                    opacity: 0.0,
                    scale: 1.0,
                },
                NotificationAnimation::Shake { .. } => initial, // Handled differently
                NotificationAnimation::Pulse { .. } => initial,  // Handled differently
                NotificationAnimation::ProgressUpdate { .. } => initial,
            };
            
            self.animations.write().unwrap().insert(id, AnimationState {
                animation,
                start_time: std::time::Instant::now(),
                initial,
                target,
            });
        }
    }
    
    /// Update animations
    pub fn update_animations(&mut self, delta_time: f32) {
        let mut completed = Vec::new();
        let mut animations = self.animations.write().unwrap();
        let mut visuals = self.active_visuals.write().unwrap();
        
        for (id, state) in animations.iter() {
            if let Some(visual) = visuals.get_mut(id) {
                let elapsed = state.start_time.elapsed();
                
                match &state.animation {
                    NotificationAnimation::SlideIn { duration, .. } |
                    NotificationAnimation::SlideOut { duration, .. } |
                    NotificationAnimation::FadeIn { duration } |
                    NotificationAnimation::FadeOut { duration } => {
                        let progress = (elapsed.as_secs_f32() / duration.as_secs_f32()).min(1.0);
                        let t = self.ease_in_out(progress);
                        
                        // Interpolate
                        visual.position = Point3::from(
                            state.initial.position.coords.lerp(&state.target.position.coords, t)
                        );
                        visual.opacity = state.initial.opacity + (state.target.opacity - state.initial.opacity) * t;
                        visual.scale = state.initial.scale + (state.target.scale - state.initial.scale) * t;
                        
                        if progress >= 1.0 {
                            completed.push(*id);
                        }
                    }
                    NotificationAnimation::Shake { duration, intensity } => {
                        let progress = elapsed.as_secs_f32() / duration.as_secs_f32();
                        if progress < 1.0 {
                            let shake = (progress * 20.0).sin() * intensity * (1.0 - progress);
                            visual.position.x = state.initial.position.x + shake;
                        } else {
                            visual.position = state.initial.position;
                            completed.push(*id);
                        }
                    }
                    NotificationAnimation::Pulse { duration, count } => {
                        let progress = elapsed.as_secs_f32() / duration.as_secs_f32();
                        let cycles = progress * (*count as f32);
                        if cycles < *count as f32 {
                            let pulse = (cycles * std::f32::consts::PI * 2.0).sin() * 0.5 + 0.5;
                            visual.scale = 1.0 + pulse * 0.2;
                            visual.visual_data.glow = pulse > 0.5;
                        } else {
                            visual.scale = 1.0;
                            visual.visual_data.glow = false;
                            completed.push(*id);
                        }
                    }
                    NotificationAnimation::ProgressUpdate { value, duration } => {
                        let progress = (elapsed.as_secs_f32() / duration.as_secs_f32()).min(1.0);
                        // Update progress bar visual
                        if progress >= 1.0 {
                            completed.push(*id);
                        }
                    }
                }
            }
        }
        
        // Remove completed animations
        for id in completed {
            animations.remove(&id);
        }
    }
    
    /// Get slide offset for direction
    fn get_slide_offset(&self, direction: SlideDirection) -> Vector3<f32> {
        let distance = 10.0;
        match direction {
            SlideDirection::Left => Vector3::new(-distance, 0.0, 0.0),
            SlideDirection::Right => Vector3::new(distance, 0.0, 0.0),
            SlideDirection::Top => Vector3::new(0.0, distance, 0.0),
            SlideDirection::Bottom => Vector3::new(0.0, -distance, 0.0),
        }
    }
    
    /// Ease in-out animation curve
    fn ease_in_out(&self, t: f32) -> f32 {
        if t < 0.5 {
            2.0 * t * t
        } else {
            -1.0 + (4.0 - 2.0 * t) * t
        }
    }
    
    /// Relayout all notifications
    fn relayout(&mut self) {
        let stack = self.layout.stack.clone();
        let mut visuals = self.active_visuals.write().unwrap();
        
        for (i, id) in stack.iter().enumerate() {
            if let Some(visual) = visuals.get_mut(id) {
                visual.position = self.layout.get_position_for_index(i);
            }
        }
    }
    
    /// Get all active visuals
    pub fn get_visuals(&self) -> Vec<NotificationVisual> {
        self.active_visuals.read().unwrap().values().cloned().collect()
    }
}

impl NotificationLayout {
    /// Create a new layout manager
    fn new(screen_size: (f32, f32)) -> Self {
        Self {
            position: NotificationPosition::TopRight,
            screen_size,
            spacing: 0.5,
            max_stack: 5,
            stack: Vec::new(),
        }
    }
    
    /// Get position for a notification
    fn get_position_for_notification(&self, visual: &NotificationVisual) -> Point3<f32> {
        let index = self.stack.len();
        self.get_position_for_index(index)
    }
    
    /// Get position for stack index
    fn get_position_for_index(&self, index: usize) -> Point3<f32> {
        let (width, height) = self.screen_size;
        let margin = 1.0;
        let notification_height = 2.5; // Average height
        let y_offset = index as f32 * (notification_height + self.spacing);
        
        match self.position {
            NotificationPosition::TopLeft => {
                Point3::new(-width / 2.0 + margin, height / 2.0 - margin - y_offset, 10.0)
            }
            NotificationPosition::TopCenter => {
                Point3::new(0.0, height / 2.0 - margin - y_offset, 10.0)
            }
            NotificationPosition::TopRight => {
                Point3::new(width / 2.0 - margin, height / 2.0 - margin - y_offset, 10.0)
            }
            NotificationPosition::BottomLeft => {
                Point3::new(-width / 2.0 + margin, -height / 2.0 + margin + y_offset, 10.0)
            }
            NotificationPosition::BottomCenter => {
                Point3::new(0.0, -height / 2.0 + margin + y_offset, 10.0)
            }
            NotificationPosition::BottomRight => {
                Point3::new(width / 2.0 - margin, -height / 2.0 + margin + y_offset, 10.0)
            }
            NotificationPosition::Center => {
                Point3::new(0.0, -y_offset, 10.0)
            }
            NotificationPosition::GraphIntegrated => {
                // For graph-integrated notifications, position near the associated node
                Point3::new(0.0, 0.0, 10.0) // Will be overridden based on node position
            }
        }
    }
}

#[async_trait::async_trait]
impl NotificationRenderTarget for NotificationRenderer {
    async fn render(&mut self, notification: &Notification, position: NotificationPosition) -> Result<()> {
        self.set_position(position);
        self.add_notification(notification);
        Ok(())
    }
    
    async fn update_render(&mut self, notification: &Notification) -> Result<()> {
        if let Some(mut visual) = self.active_visuals.write().unwrap().get_mut(&notification.id) {
            // Update visual properties based on notification changes
            visual.visual_data.badge = notification.progress.map(|p| format!("{}%", p));
            
            // Update color for priority changes
            visual.visual_data.color = match notification.priority {
                NotificationPriority::Critical => [1.0, 0.2, 0.2, 0.95],
                NotificationPriority::High => [1.0, 0.6, 0.0, 0.95],
                NotificationPriority::Normal => [0.2, 0.6, 1.0, 0.95],
                NotificationPriority::Low => [0.6, 0.6, 0.6, 0.95],
            };
        }
        Ok(())
    }
    
    async fn remove_render(&mut self, id: Uuid) -> Result<()> {
        self.remove_notification(id);
        Ok(())
    }
    
    async fn animate(&mut self, id: Uuid, animation: NotificationAnimation) -> Result<()> {
        self.start_animation(id, animation);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_notification_renderer_creation() {
        let renderer = NotificationRenderer::new((1920.0, 1080.0));
        assert_eq!(renderer.get_visuals().len(), 0);
    }
    
    #[test]
    fn test_notification_visual_creation() {
        let renderer = NotificationRenderer::new((1920.0, 1080.0));
        let notification = crate::Notification::new(
            "Test".to_string(),
            "Body".to_string()
        );
        
        let visual = renderer.create_visual(&notification);
        assert_eq!(visual.id, notification.id);
        assert_eq!(visual.size.y, 2.0); // Base size without image or actions
    }
}