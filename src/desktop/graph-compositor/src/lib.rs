//! HorizonOS Graph Desktop Wayland Compositor
//! 
//! This module implements a native Wayland compositor where windows are managed
//! as nodes in a graph structure, providing a novel window management paradigm.

pub mod compositor;
pub mod backend;
pub mod input;
pub mod output;
pub mod window;
pub mod protocols;
pub mod state;
pub mod render;

pub use compositor::*;
pub use backend::*;
pub use state::*;

/// Error types for the compositor
#[derive(Debug, thiserror::Error)]
pub enum CompositorError {
    #[error("Backend initialization failed: {0}")]
    BackendInit(String),
    
    #[error("Display creation failed: {0}")]
    DisplayCreation(String),
    
    #[error("Event loop error: {0}")]
    EventLoop(#[from] calloop::Error),
    
    #[error("Input error: {0}")]
    Input(String),
    
    #[error("Output error: {0}")]
    Output(String),
    
    #[error("Protocol error: {0}")]
    Protocol(String),
    
    #[error("System error: {0}")]
    System(#[from] std::io::Error),
}