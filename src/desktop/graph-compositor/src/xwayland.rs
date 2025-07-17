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

        // Create XWayland instance
        let (xwayland, channel) = XWayland::new(display, loop_handle.clone());
        
        // Start XWayland
        match xwayland.start(
            loop_handle.clone(),
            None, // No specific display number
            |display_number| {
                info!("XWayland starting on display :{}", display_number);
                std::env::set_var("DISPLAY", format!(":{}", display_number));
                
                // Start window manager process
                std::process::Command::new("Xwayland")
                    .arg(format!(":{}", display_number))
                    .arg("-rootless")
                    .arg("-terminate")
                    .arg("-listen")
                    .arg("4")
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                    .ok()
            },
        ) {
            Ok(display_number) => {
                self.display_number = display_number;
                self.xwayland = Some(xwayland);
                info!("XWayland started on display :{}", display_number);
                
                // Set up event handling
                self.setup_event_handling(loop_handle, channel)?;
                
                Ok(())
            }
            Err(e) => {
                error!("Failed to start XWayland: {}", e);
                Err(Box::new(e))
            }
        }
    }

    /// Set up XWayland event handling
    fn setup_event_handling(
        &mut self,
        loop_handle: &smithay::reexports::calloop::LoopHandle<'static, crate::state::AppState>,
        channel: smithay::reexports::calloop::channel::Channel<XWaylandEvent>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let x11_to_wayland = self.x11_to_wayland.clone();
        let wayland_to_x11 = self.wayland_to_x11.clone();

        loop_handle
            .insert_source(channel, move |event, _, state| {
                match event {
                    smithay::reexports::calloop::channel::Event::Msg(msg) => {
                        handle_xwayland_event(msg, state, &x11_to_wayland, &wayland_to_x11);
                    }
                    smithay::reexports::calloop::channel::Event::Closed => {
                        warn!("XWayland channel closed");
                    }
                }
            })
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

        Ok(())
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
            WmWindowProperty::Title(title) => {
                info!("X11 window {} title: {:?}", window_id, title);
            }
            WmWindowProperty::Class(class) => {
                info!("X11 window {} class: {:?}", window_id, class);
            }
            WmWindowProperty::WindowType(window_type) => {
                info!("X11 window {} type: {:?}", window_id, window_type);
            }
            _ => {}
        }
    }

    /// Handle X11 configure request
    pub fn handle_configure_request(
        &mut self,
        window_id: u32,
        x: Option<i32>,
        y: Option<i32>,
        width: Option<u32>,
        height: Option<u32>,
        reorder: Option<Reorder>,
    ) {
        debug!(
            "X11 configure request for window {}: pos=({:?}, {:?}), size=({:?}, {:?}), reorder={:?}",
            window_id, x, y, width, height, reorder
        );

        if let Some(x11_wm) = &mut self.x11_wm {
            if let Some(surface) = x11_wm.surfaces().find(|s| s.window_id() == window_id) {
                let current_geo = surface.geometry();
                
                let new_geo = Rectangle {
                    loc: Point::from((
                        x.unwrap_or(current_geo.loc.x),
                        y.unwrap_or(current_geo.loc.y),
                    )),
                    size: Size::from((
                        width.map(|w| w as i32).unwrap_or(current_geo.size.w),
                        height.map(|h| h as i32).unwrap_or(current_geo.size.h),
                    )),
                };

                let _ = surface.configure(new_geo);
            }
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

/// Handle XWayland events
fn handle_xwayland_event(
    event: XWaylandEvent,
    state: &mut crate::state::AppState,
    x11_to_wayland: &Arc<Mutex<HashMap<u32, WlSurface>>>,
    wayland_to_x11: &Arc<Mutex<HashMap<WlSurface, u32>>>,
) {
    match event {
        XWaylandEvent::Ready { x11_wm, xwm_id } => {
            info!("XWayland ready with XWM id: {:?}", xwm_id);
            // Store the X11 window manager reference
            // state.xwayland_manager.x11_wm = Some(x11_wm);
        }
        XWaylandEvent::Error => {
            error!("XWayland error occurred");
        }
    }
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
            geometry: Rectangle::from_loc_and_size((0, 0), (800, 600)),
            mapped: false,
            override_redirect: false,
        }
    }

    /// Update window properties from X11 surface
    pub fn update_from_surface(&mut self, surface: &X11Surface) {
        self.title = surface.title();
        self.class = surface.class();
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