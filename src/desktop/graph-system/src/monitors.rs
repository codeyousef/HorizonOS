//! Multi-monitor support for the graph desktop

use anyhow::Result;
use nalgebra::Point3;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Monitor manager for multi-display support
pub struct MonitorManager {
    /// Active monitors
    monitors: Arc<RwLock<HashMap<String, Monitor>>>,
    /// Primary monitor ID
    primary: Arc<RwLock<Option<String>>>,
    /// Monitor layout
    layout: Arc<RwLock<MonitorLayout>>,
    /// Graph viewport assignments
    viewports: Arc<RwLock<HashMap<String, GraphViewport>>>,
}

/// Individual monitor information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Monitor {
    /// Unique monitor ID
    pub id: String,
    /// Monitor name
    pub name: String,
    /// Manufacturer
    pub manufacturer: Option<String>,
    /// Model
    pub model: Option<String>,
    /// Serial number
    pub serial: Option<String>,
    /// Physical size in millimeters
    pub physical_size: (u32, u32),
    /// Current resolution
    pub resolution: (u32, u32),
    /// Refresh rate in Hz
    pub refresh_rate: f32,
    /// Position in global coordinate space
    pub position: (i32, i32),
    /// Rotation
    pub rotation: MonitorRotation,
    /// Scale factor
    pub scale: f32,
    /// Connection type
    pub connection: ConnectionType,
    /// Supported modes
    pub modes: Vec<DisplayMode>,
    /// Is primary display
    pub is_primary: bool,
    /// Is enabled
    pub enabled: bool,
}

/// Monitor rotation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MonitorRotation {
    Normal,
    Left,
    Right,
    Inverted,
}

/// Connection type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionType {
    HDMI,
    DisplayPort,
    DVI,
    VGA,
    LVDS,
    Virtual,
    Unknown,
}

/// Display mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayMode {
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Refresh rate in Hz
    pub refresh_rate: f32,
    /// Is preferred mode
    pub preferred: bool,
}

/// Monitor layout configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorLayout {
    /// Layout mode
    pub mode: LayoutMode,
    /// Custom positions for each monitor
    pub positions: HashMap<String, (i32, i32)>,
    /// Alignment for automatic layouts
    pub alignment: LayoutAlignment,
}

/// Layout modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayoutMode {
    /// Automatic side-by-side
    Automatic,
    /// Mirror all displays
    Mirror,
    /// Extend desktop
    Extend,
    /// Custom positioning
    Custom,
}

/// Layout alignment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayoutAlignment {
    Top,
    Middle,
    Bottom,
}

/// Graph viewport assigned to a monitor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphViewport {
    /// Monitor ID
    pub monitor_id: String,
    /// Viewport bounds in graph space
    pub bounds: ViewportBounds,
    /// Camera position
    pub camera_position: Point3<f32>,
    /// Camera target
    pub camera_target: Point3<f32>,
    /// Zoom level
    pub zoom: f32,
    /// Workspace assignment
    pub workspace: Option<String>,
}

/// Viewport bounds
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ViewportBounds {
    /// Minimum coordinates
    pub min: Point3<f32>,
    /// Maximum coordinates
    pub max: Point3<f32>,
}

impl MonitorManager {
    /// Create new monitor manager
    pub fn new() -> Self {
        Self {
            monitors: Arc::new(RwLock::new(HashMap::new())),
            primary: Arc::new(RwLock::new(None)),
            layout: Arc::new(RwLock::new(MonitorLayout::default())),
            viewports: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Detect available monitors
    pub async fn detect_monitors(&self) -> Result<Vec<Monitor>> {
        let mut detected = Vec::new();
        
        // TODO: Integrate with Smithay's output management
        // For now, create a mock primary monitor
        let primary = Monitor {
            id: "primary".to_string(),
            name: "Built-in Display".to_string(),
            manufacturer: Some("Generic".to_string()),
            model: Some("LCD".to_string()),
            serial: None,
            physical_size: (344, 194), // 15.6" 16:9
            resolution: (1920, 1080),
            refresh_rate: 60.0,
            position: (0, 0),
            rotation: MonitorRotation::Normal,
            scale: 1.0,
            connection: ConnectionType::LVDS,
            modes: vec![
                DisplayMode {
                    width: 1920,
                    height: 1080,
                    refresh_rate: 60.0,
                    preferred: true,
                },
                DisplayMode {
                    width: 1600,
                    height: 900,
                    refresh_rate: 60.0,
                    preferred: false,
                },
            ],
            is_primary: true,
            enabled: true,
        };
        
        detected.push(primary.clone());
        self.add_monitor(primary)?;
        
        Ok(detected)
    }
    
    /// Add a monitor
    pub fn add_monitor(&self, monitor: Monitor) -> Result<()> {
        let id = monitor.id.clone();
        let is_primary = monitor.is_primary;
        
        self.monitors.write().unwrap().insert(id.clone(), monitor);
        
        if is_primary {
            *self.primary.write().unwrap() = Some(id.clone());
        }
        
        // Create default viewport for the monitor
        self.create_viewport_for_monitor(&id)?;
        
        // Update layout
        self.update_layout()?;
        
        Ok(())
    }
    
    /// Remove a monitor
    pub fn remove_monitor(&self, id: &str) -> Result<()> {
        self.monitors.write().unwrap().remove(id);
        self.viewports.write().unwrap().remove(id);
        
        // If this was primary, select a new one
        if self.primary.read().unwrap().as_ref() == Some(&id.to_string()) {
            if let Some(new_primary) = self.monitors.read().unwrap().keys().next() {
                *self.primary.write().unwrap() = Some(new_primary.clone());
            } else {
                *self.primary.write().unwrap() = None;
            }
        }
        
        self.update_layout()?;
        Ok(())
    }
    
    /// Set primary monitor
    pub fn set_primary(&self, id: &str) -> Result<()> {
        if self.monitors.read().unwrap().contains_key(id) {
            // Update old primary
            if let Some(old_primary_id) = self.primary.read().unwrap().as_ref() {
                if let Some(old_primary) = self.monitors.write().unwrap().get_mut(old_primary_id) {
                    old_primary.is_primary = false;
                }
            }
            
            // Set new primary
            if let Some(monitor) = self.monitors.write().unwrap().get_mut(id) {
                monitor.is_primary = true;
            }
            
            *self.primary.write().unwrap() = Some(id.to_string());
            Ok(())
        } else {
            Err(anyhow::anyhow!("Monitor {} not found", id))
        }
    }
    
    /// Update monitor configuration
    pub fn update_monitor(&self, id: &str, resolution: (u32, u32), position: (i32, i32), scale: f32) -> Result<()> {
        if let Some(monitor) = self.monitors.write().unwrap().get_mut(id) {
            monitor.resolution = resolution;
            monitor.position = position;
            monitor.scale = scale;
            
            // Update viewport
            self.update_viewport_for_monitor(id)?;
            self.update_layout()?;
            
            Ok(())
        } else {
            Err(anyhow::anyhow!("Monitor {} not found", id))
        }
    }
    
    /// Set layout mode
    pub fn set_layout_mode(&self, mode: LayoutMode) -> Result<()> {
        self.layout.write().unwrap().mode = mode;
        self.update_layout()?;
        Ok(())
    }
    
    /// Get all monitors
    pub fn get_monitors(&self) -> Vec<Monitor> {
        self.monitors.read().unwrap().values().cloned().collect()
    }
    
    /// Get monitor by ID
    pub fn get_monitor(&self, id: &str) -> Option<Monitor> {
        self.monitors.read().unwrap().get(id).cloned()
    }
    
    /// Get primary monitor
    pub fn get_primary(&self) -> Option<Monitor> {
        let primary_id = self.primary.read().unwrap();
        primary_id.as_ref().and_then(|id| self.get_monitor(id))
    }
    
    /// Get viewport for monitor
    pub fn get_viewport(&self, monitor_id: &str) -> Option<GraphViewport> {
        self.viewports.read().unwrap().get(monitor_id).cloned()
    }
    
    /// Create viewport for monitor
    fn create_viewport_for_monitor(&self, monitor_id: &str) -> Result<()> {
        if let Some(monitor) = self.get_monitor(monitor_id) {
            let (width, height) = monitor.resolution;
            let aspect = width as f32 / height as f32;
            
            // Create viewport bounds based on monitor size
            let viewport = GraphViewport {
                monitor_id: monitor_id.to_string(),
                bounds: ViewportBounds {
                    min: Point3::new(-aspect * 10.0, -10.0, -100.0),
                    max: Point3::new(aspect * 10.0, 10.0, 100.0),
                },
                camera_position: Point3::new(0.0, 0.0, 20.0),
                camera_target: Point3::new(0.0, 0.0, 0.0),
                zoom: 1.0,
                workspace: None,
            };
            
            self.viewports.write().unwrap().insert(monitor_id.to_string(), viewport);
        }
        Ok(())
    }
    
    /// Update viewport for monitor changes
    fn update_viewport_for_monitor(&self, monitor_id: &str) -> Result<()> {
        if let Some(monitor) = self.get_monitor(monitor_id) {
            if let Some(viewport) = self.viewports.write().unwrap().get_mut(monitor_id) {
                let (width, height) = monitor.resolution;
                let aspect = width as f32 / height as f32;
                
                // Update bounds based on new resolution
                viewport.bounds = ViewportBounds {
                    min: Point3::new(-aspect * 10.0, -10.0, -100.0),
                    max: Point3::new(aspect * 10.0, 10.0, 100.0),
                };
            }
        }
        Ok(())
    }
    
    /// Update monitor layout
    fn update_layout(&self) -> Result<()> {
        let layout = self.layout.read().unwrap();
        let mut monitors = self.monitors.write().unwrap();
        
        match layout.mode {
            LayoutMode::Automatic => {
                // Arrange monitors side by side
                let mut x_offset = 0;
                for (_, monitor) in monitors.iter_mut() {
                    monitor.position = (x_offset, 0);
                    x_offset += monitor.resolution.0 as i32;
                }
            }
            LayoutMode::Mirror => {
                // All monitors at same position
                for (_, monitor) in monitors.iter_mut() {
                    monitor.position = (0, 0);
                }
            }
            LayoutMode::Extend => {
                // Similar to automatic but with alignment
                self.apply_extended_layout(&mut monitors, layout.alignment);
            }
            LayoutMode::Custom => {
                // Use positions from layout
                for (id, position) in &layout.positions {
                    if let Some(monitor) = monitors.get_mut(id) {
                        monitor.position = *position;
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Apply extended layout with alignment
    fn apply_extended_layout(&self, monitors: &mut HashMap<String, Monitor>, alignment: LayoutAlignment) {
        let mut x_offset = 0;
        
        for (_, monitor) in monitors.iter_mut() {
            let y_offset = match alignment {
                LayoutAlignment::Top => 0,
                LayoutAlignment::Middle => {
                    // Align to middle of primary monitor
                    if let Some(primary) = self.get_primary() {
                        (primary.resolution.1 as i32 - monitor.resolution.1 as i32) / 2
                    } else {
                        0
                    }
                }
                LayoutAlignment::Bottom => {
                    // Align to bottom of primary monitor
                    if let Some(primary) = self.get_primary() {
                        primary.resolution.1 as i32 - monitor.resolution.1 as i32
                    } else {
                        0
                    }
                }
            };
            
            monitor.position = (x_offset, y_offset);
            x_offset += monitor.resolution.0 as i32;
        }
    }
    
    /// Get monitor at position
    pub fn get_monitor_at_position(&self, x: i32, y: i32) -> Option<Monitor> {
        let monitors = self.monitors.read().unwrap();
        
        for monitor in monitors.values() {
            let (mx, my) = monitor.position;
            let (width, height) = monitor.resolution;
            
            if x >= mx && x < mx + width as i32 && y >= my && y < my + height as i32 {
                return Some(monitor.clone());
            }
        }
        
        None
    }
    
    /// Transform global position to monitor-relative
    pub fn global_to_monitor_position(&self, monitor_id: &str, global_x: i32, global_y: i32) -> Option<(i32, i32)> {
        self.get_monitor(monitor_id).map(|monitor| {
            let (mx, my) = monitor.position;
            (global_x - mx, global_y - my)
        })
    }
    
    /// Transform monitor-relative position to global
    pub fn monitor_to_global_position(&self, monitor_id: &str, local_x: i32, local_y: i32) -> Option<(i32, i32)> {
        self.get_monitor(monitor_id).map(|monitor| {
            let (mx, my) = monitor.position;
            (local_x + mx, local_y + my)
        })
    }
}

impl Default for MonitorLayout {
    fn default() -> Self {
        Self {
            mode: LayoutMode::Automatic,
            positions: HashMap::new(),
            alignment: LayoutAlignment::Middle,
        }
    }
}

impl MonitorRotation {
    /// Get rotation angle in degrees
    pub fn angle(&self) -> f32 {
        match self {
            MonitorRotation::Normal => 0.0,
            MonitorRotation::Left => 90.0,
            MonitorRotation::Right => 270.0,
            MonitorRotation::Inverted => 180.0,
        }
    }
    
    /// Get transformation matrix for rotation
    pub fn transform_matrix(&self) -> [[f32; 2]; 2] {
        let angle = self.angle().to_radians();
        let cos = angle.cos();
        let sin = angle.sin();
        
        [[cos, -sin], [sin, cos]]
    }
}

/// Monitor configuration for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorConfig {
    /// Monitor configurations by ID
    pub monitors: HashMap<String, MonitorSettings>,
    /// Layout configuration
    pub layout: MonitorLayout,
    /// Primary monitor ID
    pub primary: Option<String>,
}

/// Individual monitor settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorSettings {
    /// Resolution
    pub resolution: (u32, u32),
    /// Position
    pub position: (i32, i32),
    /// Rotation
    pub rotation: MonitorRotation,
    /// Scale factor
    pub scale: f32,
    /// Enabled state
    pub enabled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_monitor_manager() {
        let manager = MonitorManager::new();
        let monitors = manager.detect_monitors().await.unwrap();
        
        assert!(!monitors.is_empty());
        assert_eq!(monitors[0].id, "primary");
        assert_eq!(monitors[0].resolution, (1920, 1080));
    }
    
    #[test]
    fn test_monitor_layout() {
        let manager = MonitorManager::new();
        
        // Add two monitors
        let monitor1 = Monitor {
            id: "monitor1".to_string(),
            name: "Monitor 1".to_string(),
            resolution: (1920, 1080),
            position: (0, 0),
            is_primary: true,
            ..Default::default()
        };
        
        let monitor2 = Monitor {
            id: "monitor2".to_string(),
            name: "Monitor 2".to_string(),
            resolution: (1920, 1080),
            position: (0, 0),
            is_primary: false,
            ..Default::default()
        };
        
        manager.add_monitor(monitor1).unwrap();
        manager.add_monitor(monitor2).unwrap();
        
        // Check automatic layout
        let monitors = manager.get_monitors();
        assert_eq!(monitors.len(), 2);
        
        // Second monitor should be positioned to the right
        let monitor2_updated = manager.get_monitor("monitor2").unwrap();
        assert_eq!(monitor2_updated.position.0, 1920);
    }
    
    #[test]
    fn test_monitor_position_detection() {
        let manager = MonitorManager::new();
        
        let monitor = Monitor {
            id: "test".to_string(),
            name: "Test".to_string(),
            resolution: (1920, 1080),
            position: (0, 0),
            ..Default::default()
        };
        
        manager.add_monitor(monitor).unwrap();
        
        // Test position detection
        assert!(manager.get_monitor_at_position(100, 100).is_some());
        assert!(manager.get_monitor_at_position(2000, 100).is_none());
        
        // Test coordinate transformation
        let (local_x, local_y) = manager.global_to_monitor_position("test", 100, 200).unwrap();
        assert_eq!(local_x, 100);
        assert_eq!(local_y, 200);
    }
}

impl Default for Monitor {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            manufacturer: None,
            model: None,
            serial: None,
            physical_size: (0, 0),
            resolution: (1920, 1080),
            refresh_rate: 60.0,
            position: (0, 0),
            rotation: MonitorRotation::Normal,
            scale: 1.0,
            connection: ConnectionType::Unknown,
            modes: Vec::new(),
            is_primary: false,
            enabled: true,
        }
    }
}