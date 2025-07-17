//! XWayland integration for running X11 applications

use smithay::{
    xwayland::{
        xwm::{Reorder, ResizeEdge as X11ResizeEdge, WmWindowProperty, XwmId},
        X11Surface, X11Wm, XWayland, XWaylandEvent,
    },
    utils::{Logical, Point, Rectangle, Size},
    reexports::wayland_server::{protocol::wl_surface::WlSurface, DisplayHandle},
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    process::Stdio,
};
use tracing::{debug, error, info, warn};

/// XWayland manager for X11 application support
pub struct XWaylandManager {
    /// XWayland instance
    xwayland: Option<XWayland>,
    /// X11 window manager
    x11_wm: Option<X11Wm>,
    /// Mapping from X11 windows to Wayland surfaces
    x11_to_wayland: Arc<Mutex<HashMap<u32, WlSurface>>>,
    /// Mapping from Wayland surfaces to X11 windows
    wayland_to_x11: Arc<Mutex<HashMap<WlSurface, u32>>>,
    /// Display number for X11
    display_number: u32,
    /// Whether XWayland is enabled
    enabled: bool,
}

impl XWaylandManager {
    /// Create a new XWayland manager
    pub fn new() -> Self {
        Self {
            xwayland: None,
            x11_wm: None,
            x11_to_wayland: Arc::new(Mutex::new(HashMap::new())),
            wayland_to_x11: Arc::new(Mutex::new(HashMap::new())),
            display_number: 0,
            enabled: true,
        }
    }

    /// Initialize XWayland
    pub fn init(
        &mut self,
        display: &DisplayHandle,
        loop_handle: &smithay::reexports::calloop::LoopHandle<'static, crate::state::AppState>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if !self.enabled {
            info!("XWayland is disabled");
            return Ok(());
        }

        info!("Initializing XWayland...");

        // Create XWayland instance using Smithay 0.7 API
        match XWayland::spawn(
            display,
            None, // display number
            Vec::<(String, String)>::new(), // environment variables
            true, // client_fd
            Stdio::null(), // stdout
            Stdio::null(), // stderr
            |_user_data| {}, // user data closure
        ) {
            Ok((xwayland, _client)) => {
                info!("XWayland spawned successfully");
                self.xwayland = Some(xwayland);
                info!("XWayland initialized successfully");
            }
            Err(e) => {
                error!("Failed to spawn XWayland: {}", e);
                return Err(Box::new(e));
            }
        }
        
        Ok(())
    }

    /// Get X11 window manager
    pub fn x11_wm(&mut self) -> Option<&mut X11Wm> {
        self.x11_wm.as_mut()
    }

    /// Handle X11 surface creation
    pub fn handle_x11_surface_created(&mut self, surface: &X11Surface) {
        let window_id = surface.window_id();
        info!("X11 surface created: window_id={}", window_id);

        // Set default properties
        if let Some(x11_wm) = &mut self.x11_wm {
            // Configure the window
            let _ = surface.configure(Rectangle {
                loc: Point::from((100, 100)),
                size: Size::from((800, 600)),
            });

            // Map the window
            surface.set_mapped(true).ok();
        }
    }

    /// Handle X11 surface destruction
    pub fn handle_x11_surface_destroyed(&mut self, window_id: u32) {
        info!("X11 surface destroyed: window_id={}", window_id);

        // Clean up mappings
        self.x11_to_wayland.lock().unwrap().remove(&window_id);
        
        // Also remove from reverse mapping
        let mut wayland_to_x11 = self.wayland_to_x11.lock().unwrap();
        wayland_to_x11.retain(|_, &mut v| v != window_id);
    }

    /// Map X11 window to Wayland surface
    pub fn map_x11_to_wayland(&self, window_id: u32, surface: WlSurface) {
        debug!("Mapping X11 window {} to Wayland surface", window_id);
        
        self.x11_to_wayland.lock().unwrap().insert(window_id, surface.clone());
        self.wayland_to_x11.lock().unwrap().insert(surface, window_id);
    }

    /// Get Wayland surface for X11 window
    pub fn get_wayland_surface(&self, window_id: u32) -> Option<WlSurface> {
        self.x11_to_wayland.lock().unwrap().get(&window_id).cloned()
    }

    /// Get X11 window for Wayland surface
    pub fn get_x11_window(&self, surface: &WlSurface) -> Option<u32> {
        self.wayland_to_x11.lock().unwrap().get(surface).copied()
    }

    /// Handle X11 window property change
    pub fn handle_property_change(&mut self, window_id: u32, property: WmWindowProperty) {
        debug!("X11 window {} property changed: {:?}", window_id, property);

        match property {
            WmWindowProperty::Title => {
                info!("X11 window {} title changed", window_id);
            }
            WmWindowProperty::Class => {
                info!("X11 window {} class changed", window_id);
            }
            WmWindowProperty::WindowType => {
                info!("X11 window {} type changed", window_id);
            }
            _ => {}
        }
    }

    /// Handle X11 configure request
    pub fn handle_configure_request(
        &mut self,
        surface: &X11Surface,
        x: Option<i32>,
        y: Option<i32>,
        width: Option<u32>,
        height: Option<u32>,
        _reorder: Option<Reorder>,
    ) {
        let window_id = surface.window_id();
        debug!(
            "X11 configure request for window {}: pos=({:?}, {:?}), size=({:?}, {:?})",
            window_id, x, y, width, height
        );

        // Create the geometry based on the request
        let loc = Point::from((
            x.unwrap_or(surface.geometry().loc.x),
            y.unwrap_or(surface.geometry().loc.y),
        ));
        
        let size = Size::from((
            width.unwrap_or(surface.geometry().size.w as u32) as i32,
            height.unwrap_or(surface.geometry().size.h as u32) as i32,
        ));

        let new_geometry = Rectangle::new(loc, size);
        
        // Configure the X11 surface
        if let Err(e) = surface.configure(new_geometry) {
            warn!("Failed to configure X11 surface {}: {}", window_id, e);
        } else {
            debug!("Configured X11 surface {} to {:?}", window_id, new_geometry);
        }
    }

    /// Check if XWayland is running
    pub fn is_running(&self) -> bool {
        self.xwayland.is_some()
    }

    /// Get display number
    pub fn display_number(&self) -> Option<u32> {
        if self.is_running() {
            Some(self.display_number)
        } else {
            None
        }
    }

    /// Shutdown XWayland
    pub fn shutdown(&mut self) {
        info!("Shutting down XWayland");
        
        // Clear mappings
        self.x11_to_wayland.lock().unwrap().clear();
        self.wayland_to_x11.lock().unwrap().clear();
        
        // Drop XWayland instance (will terminate the process)
        self.xwayland = None;
        self.x11_wm = None;
    }
}

/// Handle XWayland events - simplified for basic functionality
pub fn handle_xwayland_ready(state: &mut crate::state::AppState, display_number: u32) {
    info!("XWayland ready on display {}", display_number);
    
    // Set DISPLAY environment variable for applications
    std::env::set_var("DISPLAY", format!(":{}", display_number));
    
    // Store the display number
    state.xwayland_manager.display_number = display_number;
    info!("XWayland setup complete, ready for X11 applications");
}

/// X11 window state for graph nodes
#[derive(Debug, Clone)]
pub struct X11WindowNode {
    /// X11 window ID
    pub window_id: u32,
    /// Window title
    pub title: Option<String>,
    /// Window class
    pub class: Option<String>,
    /// Window geometry
    pub geometry: Rectangle<i32, Logical>,
    /// Whether the window is mapped
    pub mapped: bool,
    /// Whether the window is override redirect
    pub override_redirect: bool,
}

impl X11WindowNode {
    /// Create a new X11 window node
    pub fn new(window_id: u32) -> Self {
        Self {
            window_id,
            title: None,
            class: None,
            geometry: Rectangle::new(Point::from((0, 0)), Size::from((800, 600))),
            mapped: false,
            override_redirect: false,
        }
    }

    /// Update window properties from X11 surface
    pub fn update_from_surface(&mut self, surface: &X11Surface) {
        self.title = Some(surface.title());
        self.class = Some(surface.class());
        self.geometry = surface.geometry();
        self.mapped = surface.is_mapped();
        self.override_redirect = surface.is_override_redirect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xwayland_manager_creation() {
        let manager = XWaylandManager::new();
        assert!(!manager.is_running());
        assert_eq!(manager.display_number(), None);
    }

    #[test]
    fn test_x11_window_node() {
        let mut node = X11WindowNode::new(12345);
        assert_eq!(node.window_id, 12345);
        assert!(node.title.is_none());
        assert!(!node.mapped);
        
        node.title = Some("Test Window".to_string());
        assert_eq!(node.title.as_ref().unwrap(), "Test Window");
    }
}