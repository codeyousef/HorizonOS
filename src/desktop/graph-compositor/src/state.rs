//! Compositor state management - updated for Smithay 0.7

use smithay::{
    delegate_compositor, delegate_shm, delegate_xdg_shell, delegate_seat,
    delegate_data_device, delegate_output,
    desktop::{Space, Window, PopupManager},
    input::{Seat, SeatHandler, SeatState, pointer::CursorImageStatus},
    reexports::{
        wayland_server::{
            backend::ClientData,
            protocol::{wl_surface::WlSurface, wl_seat::WlSeat, wl_data_source::WlDataSource},
            Client, DisplayHandle,
        },
    },
    wayland::{
        buffer::BufferHandler,
        compositor::{CompositorClientState, CompositorHandler, CompositorState},
        selection::{
            data_device::{DataDeviceHandler, DataDeviceState, ServerDndGrabHandler, ClientDndGrabHandler},
            SelectionHandler,
        },
        output::{OutputHandler, OutputManagerState},
        seat,
        shell::xdg::{ToplevelSurface, XdgShellHandler, XdgShellState, PopupSurface},
        shm::{ShmHandler, ShmState},
    },
};
use calloop::LoopHandle;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use horizonos_graph_engine::{Scene, SceneId};
use horizonos_graph_nodes::manager::NodeManager;
use horizonos_graph_interaction::InteractionManager;
use crate::protocols::ProtocolManager;
use crate::window_manager::{WindowManager, WindowManagerConfig};

/// Client data stored per connected client
#[derive(Default)]
pub struct ClientState {
    pub compositor_state: CompositorClientState,
}

impl ClientData for ClientState {
    fn initialized(&self, _client_id: smithay::reexports::wayland_server::backend::ClientId) {}
    fn disconnected(&self, _client_id: smithay::reexports::wayland_server::backend::ClientId, _reason: smithay::reexports::wayland_server::backend::DisconnectReason) {}
}

/// Main compositor state
pub struct AppState {
    /// Running flag
    pub running: bool,
    /// Event loop handle
    pub loop_handle: LoopHandle<'static, Self>,
    /// Display handle
    pub display_handle: DisplayHandle,
    
    // Wayland state
    pub compositor_state: CompositorState,
    pub xdg_shell_state: XdgShellState,
    pub shm_state: ShmState,
    pub output_manager: OutputManagerState,
    pub seat_state: SeatState<Self>,
    pub data_device_state: DataDeviceState,
    
    // Desktop management
    pub space: Space<Window>,
    pub popups: PopupManager,
    
    // Graph integration
    pub graph_scene: Arc<Mutex<Scene>>,
    pub node_manager: Arc<Mutex<NodeManager>>,
    pub interaction_manager: Arc<Mutex<InteractionManager>>,
    pub surface_to_node: HashMap<WlSurface, SceneId>,
    
    // Protocol extensions
    pub protocol_manager: ProtocolManager,
    
    // Window management
    pub window_manager: WindowManager,
    
    // Input
    pub seat: Seat<Self>,
}

impl AppState {
    pub fn new(
        display_handle: DisplayHandle,
        loop_handle: LoopHandle<'static, Self>,
    ) -> Result<Self, anyhow::Error> {
        // Initialize Wayland protocols
        let compositor_state = CompositorState::new::<Self>(&display_handle);
        let xdg_shell_state = XdgShellState::new::<Self>(&display_handle);
        let shm_state = ShmState::new::<Self>(&display_handle, vec![]);
        let output_manager = OutputManagerState::new_with_xdg_output::<Self>(&display_handle);
        let mut seat_state = SeatState::new();
        let data_device_state = DataDeviceState::new::<Self>(&display_handle);
        
        // Create seat
        let mut seat = seat_state.new_wl_seat(&display_handle, "seat0");
        seat.add_keyboard(Default::default(), 200, 25)?;
        seat.add_pointer();
        
        // Create desktop space
        let space = Space::<Window>::default();
        let popups = PopupManager::default();
        
        // Initialize graph components
        let graph_scene = Arc::new(Mutex::new(Scene::new()));
        let node_manager = Arc::new(Mutex::new(NodeManager::new()));
        let interaction_manager = Arc::new(Mutex::new(InteractionManager::new()));
        
        // Initialize protocol manager
        let protocol_manager = ProtocolManager::new();
        
        // Initialize window manager
        let window_manager = WindowManager::new(WindowManagerConfig::default());
        
        Ok(Self {
            running: true,
            loop_handle,
            display_handle,
            compositor_state,
            xdg_shell_state,
            shm_state,
            output_manager,
            seat_state,
            data_device_state,
            space,
            popups,
            graph_scene,
            node_manager,
            interaction_manager,
            surface_to_node: HashMap::new(),
            protocol_manager,
            window_manager,
            seat,
        })
    }
}

// Handler implementations
impl CompositorHandler for AppState {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }
    
    fn client_compositor_state<'a>(&self, client: &'a Client) -> &'a CompositorClientState {
        &client.get_data::<ClientState>().unwrap().compositor_state
    }
    
    fn commit(&mut self, surface: &WlSurface) {
        // Handle surface commits
        // Space and popup handling in Smithay 0.7
        // TODO: Check if specific handling needed
    }
}

impl BufferHandler for AppState {
    fn buffer_destroyed(&mut self, _buffer: &smithay::reexports::wayland_server::protocol::wl_buffer::WlBuffer) {}
}

impl ShmHandler for AppState {
    fn shm_state(&self) -> &ShmState {
        &self.shm_state
    }
}

impl XdgShellHandler for AppState {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }
    
    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        // Create a window for the toplevel
        let window = Window::new_wayland_window(surface);
        self.space.map_element(window.clone(), (0, 0), true);
        
        // Create a graph node for the window
        let node = horizonos_graph_engine::SceneNode {
            id: 0, // Will be set by Scene
            position: nalgebra::Point3::new(0.0, 0.0, 0.0),
            velocity: nalgebra::Vector3::zeros(),
            node_type: horizonos_graph_engine::NodeType::Application { 
                pid: 0, 
                name: "Window".to_string() 
            },
            radius: 1.0,
            color: [0.5, 0.5, 0.5, 1.0],
            metadata: horizonos_graph_engine::NodeMetadata {
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                tags: vec![],
                description: None,
                properties: std::collections::HashMap::new(),
            },
            visible: true,
            selected: false,
        };
        let node_id = self.graph_scene.lock().unwrap().add_node(node);
        
        // Map surface to node
        if let Some(toplevel) = window.toplevel() {
            let wl_surface = toplevel.wl_surface();
            self.surface_to_node.insert(wl_surface.clone(), node_id);
        }
    }
    
    fn toplevel_destroyed(&mut self, surface: ToplevelSurface) {
        // Find and remove the window
        let window = self.space.elements()
            .find(|w| w.toplevel() == Some(&surface))
            .cloned();
        
        if let Some(window) = window {
            self.space.unmap_elem(&window);
            
            // Remove graph node
            if let Some(toplevel) = window.toplevel() {
                let wl_surface = toplevel.wl_surface();
                if let Some(node_id) = self.surface_to_node.remove(wl_surface) {
                    self.graph_scene.lock().unwrap().remove_node(node_id);
                }
            }
        }
    }
    
    fn new_popup(&mut self, _surface: PopupSurface, _positioner: smithay::wayland::shell::xdg::PositionerState) {
        // TODO: Handle popups
    }
    
    fn move_request(&mut self, _surface: ToplevelSurface, _seat: WlSeat, _serial: smithay::utils::Serial) {
        // TODO: Implement window movement
    }
    
    fn resize_request(&mut self, _surface: ToplevelSurface, _seat: WlSeat, _serial: smithay::utils::Serial, _edges: smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::ResizeEdge) {
        // TODO: Implement window resizing
    }
    
    fn grab(&mut self, _surface: PopupSurface, _seat: WlSeat, _serial: smithay::utils::Serial) {
        // TODO: Implement popup grab
    }
    
    fn reposition_request(&mut self, _surface: PopupSurface, _positioner: smithay::wayland::shell::xdg::PositionerState, _token: u32) {
        // TODO: Implement popup repositioning
    }
}

impl SeatHandler for AppState {
    type KeyboardFocus = WlSurface;
    type PointerFocus = WlSurface;
    type TouchFocus = WlSurface;
    
    fn seat_state(&mut self) -> &mut SeatState<Self> {
        &mut self.seat_state
    }
    
    fn cursor_image(&mut self, _seat: &Seat<Self>, _image: CursorImageStatus) {
        // TODO: Handle cursor image changes
    }
    
    fn focus_changed(&mut self, _seat: &Seat<Self>, _focused: Option<&WlSurface>) {
        // TODO: Handle focus changes
    }
}

impl DataDeviceHandler for AppState {
    fn data_device_state(&self) -> &DataDeviceState {
        &self.data_device_state
    }
}

impl SelectionHandler for AppState {
    type SelectionUserData = ();
}

impl ClientDndGrabHandler for AppState {}

impl ServerDndGrabHandler for AppState {}

impl OutputHandler for AppState {}

// Delegate macro implementations
delegate_compositor!(AppState);
delegate_shm!(AppState);
delegate_xdg_shell!(AppState);
delegate_seat!(AppState);
delegate_data_device!(AppState);
delegate_output!(AppState);