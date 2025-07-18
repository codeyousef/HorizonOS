//! Graph Desktop Compositor Executable

use horizonos_graph_compositor::{AppState, backend};
use smithay::reexports::wayland_server::Display;
use calloop::EventLoop;
use anyhow::Result;

fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();
    
    log::info!("Starting HorizonOS Graph Desktop Compositor");
    
    // Check for command line arguments
    let args: Vec<String> = std::env::args().collect();
    
    if args.contains(&"--software-render".to_string()) {
        std::env::set_var("LIBGL_ALWAYS_SOFTWARE", "1");
        std::env::set_var("WGPU_BACKEND", "gl");
        log::info!("Software rendering enabled");
    }
    
    if args.contains(&"--help".to_string()) {
        println!("HorizonOS Graph Desktop Compositor");
        println!("");
        println!("Options:");
        println!("  --software-render  Use software rendering (no GPU required)");
        println!("  --help            Show this help message");
        return Ok(());
    }
    
    // For development, use winit backend with error handling
    match run_winit_compositor() {
        Ok(()) => Ok(()),
        Err(e) => {
            log::error!("Compositor error: {}", e);
            
            // Check for common EGL errors
            if e.to_string().contains("EGL") || e.to_string().contains("BAD_ALLOC") {
                log::error!("EGL initialization failed. Try running with --software-render");
                log::error!("Or set environment variables:");
                log::error!("  export LIBGL_ALWAYS_SOFTWARE=1");
                log::error!("  export WLR_RENDERER=pixman");
            }
            
            Err(e)
        }
    }
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