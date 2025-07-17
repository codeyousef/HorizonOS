//! Graph Desktop Compositor Executable

use horizonos_graph_compositor::{AppState, backend};
use smithay::reexports::wayland_server::Display;
use calloop::EventLoop;
use anyhow::Result;

fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();
    
    log::info!("Starting HorizonOS Graph Desktop Compositor");
    
    // For development, use winit backend
    run_winit_compositor()
}

fn run_winit_compositor() -> Result<()> {
    // Initialize backend
    let (backend, winit_event_loop) = backend::init_winit_backend()?;
    
    // Create event loop
    let event_loop = EventLoop::<AppState>::try_new()?;
    let loop_handle = event_loop.handle();
    
    // Create display
    let display: Display<AppState> = Display::new()?;
    let display_handle = display.handle();
    
    // Create compositor state  
    let state = AppState::new(display_handle, loop_handle)?;
    
    // Socket handling is automatic in Smithay 0.7
    log::info!("Starting Wayland compositor");
    
    // Run winit backend with integrated event loop
    backend::run_winit(state, backend, winit_event_loop)?;
    
    Ok(())
}