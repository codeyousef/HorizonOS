//! Error types for the graph engine

use thiserror::Error;

#[derive(Error, Debug)]
pub enum GraphEngineError {
    #[error("WebGPU adapter not found")]
    AdapterNotFound,
    
    #[error("WebGPU device error: {0}")]
    DeviceError(#[from] wgpu::RequestDeviceError),
    
    #[error("Surface error: {0}")]
    SurfaceError(#[from] wgpu::CreateSurfaceError),
    
    #[error("Rendering error: {0}")]
    RenderError(String),
    
    #[error("Physics error: {0}")]
    PhysicsError(String),
    
    #[error("Scene error: {0}")]
    SceneError(String),
    
    #[error("Camera error: {0}")]
    CameraError(String),
    
    #[error("Shader compilation error: {0}")]
    ShaderError(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("System error: {0}")]
    System(String),
    
    #[error("Node not found: {0}")]
    NodeNotFound(crate::SceneId),
    
    #[error("Thread pool error: {0}")]
    ThreadPoolError(String),
    
    #[error("Scheduler shutdown")]
    SchedulerShutdown,
    
    #[error("Lock error: {0}")]
    LockError(String),
}