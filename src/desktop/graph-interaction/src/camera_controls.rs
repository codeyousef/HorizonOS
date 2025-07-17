//! Camera control and manipulation

use horizonos_graph_engine::{Camera, GraphEngine, SceneId, Position, Ray};
use nalgebra::{Point3, Vector3, Unit};

/// Camera state for rendering
#[derive(Debug, Clone)]
pub struct CameraState {
    pub position: Point3<f32>,
    pub forward: Vector3<f32>,
    pub up: Vector3<f32>,
    pub right: Vector3<f32>,
    pub fov: f32,
}

/// Controls camera movement and orientation
pub struct CameraController {
    /// Camera movement speed
    move_speed: f32,
    /// Camera rotation speed
    rotate_speed: f32,
    /// Zoom speed
    zoom_speed: f32,
    /// Minimum zoom distance
    min_zoom: f32,
    /// Maximum zoom distance
    max_zoom: f32,
    /// Smooth animation factor
    smoothing: f32,
}

impl CameraController {
    /// Create a new camera controller
    pub fn new() -> Self {
        Self {
            move_speed: 0.01,
            rotate_speed: 0.005,
            zoom_speed: 1.1,
            min_zoom: 1.0,
            max_zoom: 100.0,
            smoothing: 0.1,
        }
    }
    
    /// Pan the camera based on mouse movement
    pub fn pan(&self, camera: &mut Camera, current_pos: (f32, f32), previous_pos: (f32, f32)) {
        let delta_x = (current_pos.0 - previous_pos.0) * self.move_speed;
        let delta_y = (current_pos.1 - previous_pos.1) * self.move_speed;
        
        // Move camera using its coordinate system
        camera.move_right(delta_x);
        camera.move_up(delta_y);
    }
    
    /// Rotate the camera around the target
    pub fn rotate(&self, camera: &mut Camera, current_pos: (f32, f32), previous_pos: (f32, f32)) {
        let delta_x = (current_pos.0 - previous_pos.0) * self.rotate_speed;
        let delta_y = (current_pos.1 - previous_pos.1) * self.rotate_speed;
        
        // Use camera's rotation methods
        camera.rotate_horizontal(-delta_x);
        camera.rotate_vertical(-delta_y);
    }
    
    /// Rotate camera around a specific point
    pub fn rotate_around(&self, camera: &mut Camera, center: (f32, f32), angle: f32) {
        // For now, just rotate horizontally
        camera.rotate_horizontal(angle);
    }
    
    /// Zoom the camera
    pub fn zoom(&self, camera: &mut Camera, delta: f32) {
        // Use camera's zoom method
        camera.zoom(delta * 0.1);
    }
    
    /// Focus camera on a specific node
    pub fn focus_on_node(&self, node_id: SceneId, engine: &mut GraphEngine) {
        if let Some(node) = engine.scene().get_node(node_id) {
            self.focus_on_position(node.position, engine.camera_mut());
        }
    }
    
    /// Focus camera on a specific position
    pub fn focus_on_position(&self, position: Position, camera: &mut Camera) {
        // Use camera's look_at method
        camera.look_at(position);
    }
    
    /// Convert screen coordinates to a ray in world space
    pub fn screen_to_ray(&self, screen_pos: (f32, f32), camera: &Camera, window_size: (u32, u32)) -> Ray {
        // Use camera's screen_to_ray method
        camera.screen_to_ray(screen_pos.0, screen_pos.1, window_size.0 as f32, window_size.1 as f32)
    }
    
    /// Convert screen coordinates to world position at a specific depth
    pub fn screen_to_world(&self, screen_pos: (f32, f32), camera: &Camera, window_size: (u32, u32)) -> Position {
        let ray = self.screen_to_ray(screen_pos, camera, window_size);
        
        // Intersect ray with a plane at the target depth
        let plane_distance = 10.0; // Default distance for interaction plane
        let plane_normal = -camera.forward;
        let plane_point = camera.position + camera.forward * plane_distance;
        
        // Ray-plane intersection
        let denom = plane_normal.dot(&ray.direction);
        if denom.abs() > 0.0001 {
            let t = (plane_point - ray.origin).dot(&plane_normal) / denom;
            if t > 0.0 {
                return ray.origin + ray.direction * t;
            }
        }
        
        // Fallback to ray origin if no intersection
        ray.origin
    }
    
    /// Set camera movement speed
    pub fn set_move_speed(&mut self, speed: f32) {
        self.move_speed = speed;
    }
    
    /// Set camera rotation speed
    pub fn set_rotate_speed(&mut self, speed: f32) {
        self.rotate_speed = speed;
    }
    
    /// Set zoom limits
    pub fn set_zoom_limits(&mut self, min: f32, max: f32) {
        self.min_zoom = min;
        self.max_zoom = max;
    }
    
    /// Get current camera state
    pub fn get_state(&self) -> CameraState {
        // This is a placeholder - in a real implementation, we'd store
        // a reference to the camera and get its actual state
        CameraState {
            position: Point3::new(0.0, 0.0, 10.0),
            forward: Vector3::new(0.0, 0.0, -1.0),
            up: Vector3::new(0.0, 1.0, 0.0),
            right: Vector3::new(1.0, 0.0, 0.0),
            fov: 60.0_f32.to_radians(),
        }
    }
}