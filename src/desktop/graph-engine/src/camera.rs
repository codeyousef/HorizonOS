//! Camera system for 3D navigation in the graph space

use nalgebra::{Matrix4, Point3, Vector3, Perspective3};

/// Camera for navigating the 3D graph space
#[derive(Debug, Clone)]
pub struct Camera {
    /// Camera position in world space
    pub position: Point3<f32>,
    /// Camera forward direction (normalized)
    pub forward: Vector3<f32>,
    /// Camera up direction (normalized)
    pub up: Vector3<f32>,
    /// Camera right direction (normalized)
    pub right: Vector3<f32>,
    
    /// Field of view in radians
    pub fov: f32,
    /// Aspect ratio (width / height)
    pub aspect_ratio: f32,
    /// Near clipping plane
    pub near: f32,
    /// Far clipping plane
    pub far: f32,
    
    /// Camera movement speed
    pub movement_speed: f32,
    /// Mouse sensitivity for look around
    pub mouse_sensitivity: f32,
    /// Zoom speed
    pub zoom_speed: f32,
    
    /// Target position for smooth movement
    target_position: Option<Point3<f32>>,
    /// Target look direction for smooth rotation
    target_forward: Option<Vector3<f32>>,
    /// Movement interpolation speed
    interpolation_speed: f32,
}

impl Camera {
    /// Create a new camera with default settings
    pub fn new() -> Self {
        let position = Point3::new(0.0, 0.0, 10.0);
        let forward = Vector3::new(0.0, 0.0, -1.0);
        let up = Vector3::new(0.0, 1.0, 0.0);
        let right = forward.cross(&up).normalize();
        
        Camera {
            position,
            forward,
            up,
            right,
            fov: std::f32::consts::FRAC_PI_4, // 45 degrees
            aspect_ratio: 16.0 / 9.0,
            near: 0.1,
            far: 1000.0,
            movement_speed: 10.0,
            mouse_sensitivity: 0.002,
            zoom_speed: 1.0,
            target_position: None,
            target_forward: None,
            interpolation_speed: 5.0,
        }
    }
    
    /// Update camera state (interpolation, etc.)
    pub fn update(&mut self, delta_time: f32) {
        // Smooth movement to target position
        if let Some(target) = self.target_position {
            let diff = target - self.position;
            if diff.magnitude() < 0.01 {
                self.position = target;
                self.target_position = None;
            } else {
                let movement = diff * self.interpolation_speed * delta_time;
                self.position += movement;
            }
        }
        
        // Smooth rotation to target direction
        if let Some(target) = self.target_forward {
            let diff = target - self.forward;
            if diff.magnitude() < 0.01 {
                self.forward = target.normalize();
                self.target_forward = None;
                self.update_coordinate_system();
            } else {
                let rotation = diff * self.interpolation_speed * delta_time;
                self.forward = (self.forward + rotation).normalize();
                self.update_coordinate_system();
            }
        }
    }
    
    /// Get the view matrix
    pub fn view_matrix(&self) -> Matrix4<f32> {
        Matrix4::look_at_rh(&self.position, &(self.position + self.forward), &self.up)
    }
    
    /// Get the projection matrix
    pub fn projection_matrix(&self) -> Matrix4<f32> {
        let perspective = Perspective3::new(self.aspect_ratio, self.fov, self.near, self.far);
        perspective.into_inner()
    }
    
    /// Get the combined view-projection matrix
    pub fn view_projection_matrix(&self) -> Matrix4<f32> {
        self.projection_matrix() * self.view_matrix()
    }
    
    /// Move the camera forward/backward
    pub fn move_forward(&mut self, distance: f32) {
        self.position += self.forward * distance;
    }
    
    /// Move the camera right/left
    pub fn move_right(&mut self, distance: f32) {
        self.position += self.right * distance;
    }
    
    /// Move the camera up/down
    pub fn move_up(&mut self, distance: f32) {
        self.position += self.up * distance;
    }
    
    /// Rotate the camera horizontally (yaw)
    pub fn rotate_horizontal(&mut self, angle: f32) {
        let axis = nalgebra::Unit::new_normalize(self.up);
        let rotation = nalgebra::Rotation3::from_axis_angle(&axis, angle);
        self.forward = rotation * self.forward;
        self.update_coordinate_system();
    }
    
    /// Rotate the camera vertically (pitch)
    pub fn rotate_vertical(&mut self, angle: f32) {
        let axis = nalgebra::Unit::new_normalize(self.right);
        let rotation = nalgebra::Rotation3::from_axis_angle(&axis, angle);
        let new_forward = rotation * self.forward;
        
        // Prevent camera from flipping upside down
        let dot_product = new_forward.dot(&Vector3::new(0.0, 1.0, 0.0));
        if dot_product.abs() < 0.99 {
            self.forward = new_forward;
            self.update_coordinate_system();
        }
    }
    
    /// Look at a specific point
    pub fn look_at(&mut self, target: Point3<f32>) {
        self.forward = (target - self.position).normalize();
        self.update_coordinate_system();
    }
    
    /// Smoothly move to a target position
    pub fn move_to(&mut self, target: Point3<f32>) {
        self.target_position = Some(target);
    }
    
    /// Smoothly look towards a target direction
    pub fn look_towards(&mut self, direction: Vector3<f32>) {
        self.target_forward = Some(direction.normalize());
    }
    
    /// Set the aspect ratio
    pub fn set_aspect_ratio(&mut self, aspect_ratio: f32) {
        self.aspect_ratio = aspect_ratio;
    }
    
    /// Zoom in/out by adjusting FOV
    pub fn zoom(&mut self, delta: f32) {
        self.fov = (self.fov + delta * self.zoom_speed).clamp(
            std::f32::consts::FRAC_PI_6, // 30 degrees min
            std::f32::consts::FRAC_PI_2, // 90 degrees max
        );
    }
    
    /// Get a ray from the camera through a screen position
    pub fn screen_to_ray(&self, screen_x: f32, screen_y: f32, screen_width: f32, screen_height: f32) -> Ray {
        // Convert screen coordinates to normalized device coordinates
        let ndc_x = (2.0 * screen_x) / screen_width - 1.0;
        let ndc_y = 1.0 - (2.0 * screen_y) / screen_height;
        
        // Convert to view space
        let tan_half_fov = (self.fov * 0.5).tan();
        let view_x = ndc_x * tan_half_fov * self.aspect_ratio;
        let view_y = ndc_y * tan_half_fov;
        
        // Convert to world space
        let direction = (self.forward + self.right * view_x + self.up * view_y).normalize();
        
        Ray {
            origin: self.position,
            direction,
        }
    }
    
    /// Focus on a specific node or region
    pub fn focus_on_bounds(&mut self, center: Point3<f32>, radius: f32) {
        // Calculate optimal distance to view the bounds
        let distance = radius / (self.fov * 0.5).tan() * 1.5; // 1.5x for padding
        
        // Move camera back along current forward direction
        let target_position = center - self.forward * distance;
        self.move_to(target_position);
        self.look_at(center);
    }
    
    /// Update the right and up vectors based on the forward vector
    fn update_coordinate_system(&mut self) {
        let world_up = Vector3::new(0.0, 1.0, 0.0);
        self.right = self.forward.cross(&world_up).normalize();
        self.up = self.right.cross(&self.forward).normalize();
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self::new()
    }
}

/// A ray in 3D space (for mouse picking)
#[derive(Debug, Clone)]
pub struct Ray {
    pub origin: Point3<f32>,
    pub direction: Vector3<f32>,
}

impl Ray {
    /// Get a point along the ray at distance t
    pub fn point_at(&self, t: f32) -> Point3<f32> {
        self.origin + self.direction * t
    }
    
    /// Test intersection with a sphere
    pub fn intersect_sphere(&self, center: Point3<f32>, radius: f32) -> Option<f32> {
        let oc = self.origin - center;
        let a = self.direction.dot(&self.direction);
        let b = 2.0 * oc.dot(&self.direction);
        let c = oc.dot(&oc) - radius * radius;
        
        let discriminant = b * b - 4.0 * a * c;
        
        if discriminant < 0.0 {
            None
        } else {
            let sqrt_discriminant = discriminant.sqrt();
            let t1 = (-b - sqrt_discriminant) / (2.0 * a);
            let t2 = (-b + sqrt_discriminant) / (2.0 * a);
            
            // Return the closest positive intersection
            if t1 > 0.0 {
                Some(t1)
            } else if t2 > 0.0 {
                Some(t2)
            } else {
                None
            }
        }
    }
}