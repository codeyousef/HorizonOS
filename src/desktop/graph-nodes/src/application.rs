//! Application node implementation

use crate::{GraphNode, BaseNode, NodeVisualData, NodeAction, NodeActionResult, NodeActionType, NodeError, NodeExportData};
use horizonos_graph_engine::{SceneNode, NodeType, NodeMetadata, SceneId, Position, Vec3};
use serde::{Serialize, Deserialize};
use std::process::Command;

/// Represents a running application in the graph desktop
#[derive(Debug, Clone)]
pub struct ApplicationNode {
    base: BaseNode,
    app_data: ApplicationData,
    process: Option<u32>, // PID
    #[allow(dead_code)]
    window_id: Option<String>,
    is_running: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationData {
    pub name: String,
    pub executable_path: String,
    pub args: Vec<String>,
    pub working_directory: Option<String>,
    pub window_title: Option<String>,
    pub icon_path: Option<String>,
    pub category: ApplicationCategory,
    pub memory_usage: u64, // bytes
    pub cpu_usage: f32,    // percentage
    pub startup_time: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ApplicationCategory {
    Development,
    Graphics,
    Internet,
    Multimedia,
    Office,
    Games,
    System,
    Utilities,
    Other,
}

impl ApplicationNode {
    pub fn new(id: SceneId, name: String, executable_path: String) -> Self {
        let mut base = BaseNode::new(id)
            .with_color([0.2, 0.6, 1.0, 1.0]) // Blue for applications
            .with_radius(1.2);
        
        base.metadata.tags.push("application".to_string());
        base.metadata.description = Some(format!("Application: {}", name));
        
        ApplicationNode {
            base,
            app_data: ApplicationData {
                name: name.clone(),
                executable_path,
                args: Vec::new(),
                working_directory: None,
                window_title: Some(name),
                icon_path: None,
                category: ApplicationCategory::Other,
                memory_usage: 0,
                cpu_usage: 0.0,
                startup_time: chrono::Utc::now(),
            },
            process: None,
            window_id: None,
            is_running: false,
        }
    }
    
    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.app_data.args = args;
        self
    }
    
    pub fn with_category(mut self, category: ApplicationCategory) -> Self {
        self.app_data.category = category;
        self.update_visual_by_category();
        self
    }
    
    pub fn with_icon(mut self, icon_path: String) -> Self {
        self.app_data.icon_path = Some(icon_path.clone());
        self.base.visual_data.icon = Some(icon_path);
        self
    }
    
    pub fn start_process(&mut self) -> Result<(), NodeError> {
        if self.is_running {
            return Ok(());
        }
        
        log::info!("Starting application: {}", self.app_data.name);
        
        let mut command = Command::new(&self.app_data.executable_path);
        command.args(&self.app_data.args);
        
        if let Some(ref work_dir) = self.app_data.working_directory {
            command.current_dir(work_dir);
        }
        
        match command.spawn() {
            Ok(child) => {
                self.process = Some(child.id());
                self.is_running = true;
                self.base.visual_data.glow = true;
                self.base.update_timestamp();
                log::info!("Started process {} for {}", child.id(), self.app_data.name);
                Ok(())
            }
            Err(e) => {
                log::error!("Failed to start {}: {}", self.app_data.name, e);
                Err(NodeError::SystemError { 
                    message: format!("Failed to start application: {}", e) 
                })
            }
        }
    }
    
    pub fn stop_process(&mut self) -> Result<(), NodeError> {
        if !self.is_running || self.process.is_none() {
            return Ok(());
        }
        
        if let Some(pid) = self.process {
            log::info!("Stopping application {} (PID: {})", self.app_data.name, pid);
            
            // Try graceful termination first (SIGTERM)
            let result = std::process::Command::new("kill")
                .arg("-TERM")
                .arg(pid.to_string())
                .output();
                
            match result {
                Ok(_) => {
                    self.is_running = false;
                    self.process = None;
                    self.base.visual_data.glow = false;
                    self.base.update_timestamp();
                    Ok(())
                }
                Err(e) => Err(NodeError::SystemError { 
                    message: format!("Failed to stop application: {}", e) 
                })
            }
        } else {
            Ok(())
        }
    }
    
    pub fn get_process_info(&mut self) -> Result<(), NodeError> {
        if let Some(pid) = self.process {
            // Get memory usage using ps command
            let output = std::process::Command::new("ps")
                .args(&["-p", &pid.to_string(), "-o", "rss="])
                .output();
                
            match output {
                Ok(output) => {
                    if output.status.success() {
                        let rss_str = String::from_utf8_lossy(&output.stdout);
                        if let Ok(rss_kb) = rss_str.trim().parse::<u64>() {
                            self.app_data.memory_usage = rss_kb * 1024; // Convert KB to bytes
                        }
                    } else {
                        // Process might have died
                        self.is_running = false;
                        self.process = None;
                        self.base.visual_data.glow = false;
                    }
                }
                Err(_) => {
                    // ps command failed, assume process is dead
                    self.is_running = false;
                    self.process = None;
                    self.base.visual_data.glow = false;
                }
            }
        }
        Ok(())
    }
    
    fn update_visual_by_category(&mut self) {
        let color = match self.app_data.category {
            ApplicationCategory::Development => [0.8, 0.4, 0.8, 1.0], // Purple
            ApplicationCategory::Graphics => [0.9, 0.5, 0.1, 1.0],    // Orange
            ApplicationCategory::Internet => [0.2, 0.8, 0.2, 1.0],    // Green
            ApplicationCategory::Multimedia => [0.9, 0.2, 0.5, 1.0],  // Pink
            ApplicationCategory::Office => [0.1, 0.5, 0.9, 1.0],      // Blue
            ApplicationCategory::Games => [0.9, 0.9, 0.1, 1.0],       // Yellow
            ApplicationCategory::System => [0.6, 0.6, 0.6, 1.0],      // Gray
            ApplicationCategory::Utilities => [0.5, 0.7, 0.9, 1.0],   // Light blue
            ApplicationCategory::Other => [0.2, 0.6, 1.0, 1.0],       // Default blue
        };
        self.base.visual_data.color = color;
    }
    
    pub fn pid(&self) -> Option<u32> {
        self.process
    }
    
    pub fn is_running(&self) -> bool {
        self.is_running
    }
    
    pub fn memory_usage(&self) -> u64 {
        self.app_data.memory_usage
    }
    
    pub fn cpu_usage(&self) -> f32 {
        self.app_data.cpu_usage
    }
}

impl GraphNode for ApplicationNode {
    fn id(&self) -> SceneId {
        self.base.id
    }
    
    fn display_name(&self) -> String {
        if self.is_running {
            format!("{} (Running)", self.app_data.name)
        } else {
            self.app_data.name.clone()
        }
    }
    
    fn description(&self) -> Option<String> {
        let status = if self.is_running {
            format!("Running (PID: {})", self.process.unwrap_or(0))
        } else {
            "Stopped".to_string()
        };
        
        Some(format!(
            "{} - {} | Memory: {} MB | Category: {:?}",
            self.app_data.name,
            status,
            self.app_data.memory_usage / 1024 / 1024,
            self.app_data.category
        ))
    }
    
    fn node_type(&self) -> NodeType {
        NodeType::Application {
            pid: self.process.unwrap_or(0),
            name: self.app_data.name.clone(),
        }
    }
    
    fn metadata(&self) -> NodeMetadata {
        self.base.metadata.clone()
    }
    
    fn visual_data(&self) -> NodeVisualData {
        let mut visual = self.base.visual_data.clone();
        
        // Add status indicator
        if self.is_running {
            visual.badge = Some("▶".to_string());
            visual.glow = true;
        } else {
            visual.badge = Some("⏸".to_string());
            visual.glow = false;
        }
        
        visual
    }
    
    fn update(&mut self, _delta_time: f32) -> Result<(), NodeError> {
        // Update process information if running
        if self.is_running {
            self.get_process_info()?;
        }
        Ok(())
    }
    
    fn handle_action(&mut self, action: NodeAction) -> Result<NodeActionResult, NodeError> {
        match action {
            NodeAction::Open => {
                if !self.is_running {
                    self.start_process()?;
                    Ok(NodeActionResult::Success { 
                        message: Some(format!("Started {}", self.app_data.name)) 
                    })
                } else {
                    Ok(NodeActionResult::Success { 
                        message: Some(format!("{} is already running", self.app_data.name)) 
                    })
                }
            }
            NodeAction::Delete => {
                if self.is_running {
                    Ok(NodeActionResult::ConfirmationRequired { 
                        prompt: format!("Stop and remove {}?", self.app_data.name) 
                    })
                } else {
                    Ok(NodeActionResult::Success { 
                        message: Some(format!("Removed {}", self.app_data.name)) 
                    })
                }
            }
            NodeAction::Custom { action_type, parameters: _ } => {
                match action_type.as_str() {
                    "stop" => {
                        if self.is_running {
                            self.stop_process()?;
                            Ok(NodeActionResult::Success { 
                                message: Some(format!("Stopped {}", self.app_data.name)) 
                            })
                        } else {
                            Ok(NodeActionResult::Error { 
                                error: "Application is not running".to_string() 
                            })
                        }
                    }
                    "restart" => {
                        if self.is_running {
                            self.stop_process()?;
                            // Wait a moment for clean shutdown
                            std::thread::sleep(std::time::Duration::from_millis(500));
                        }
                        self.start_process()?;
                        Ok(NodeActionResult::Success { 
                            message: Some(format!("Restarted {}", self.app_data.name)) 
                        })
                    }
                    _ => Err(NodeError::InvalidAction { action: NodeAction::Custom { action_type, parameters: std::collections::HashMap::new() } })
                }
            }
            _ => Err(NodeError::InvalidAction { action })
        }
    }
    
    fn available_actions(&self) -> Vec<NodeActionType> {
        let mut actions = vec![
            NodeActionType::Open,
            NodeActionType::Edit,
            NodeActionType::Delete,
            NodeActionType::Copy,
            NodeActionType::Move,
            NodeActionType::Connect,
        ];
        
        if self.is_running {
            actions.push(NodeActionType::Custom("stop".to_string()));
            actions.push(NodeActionType::Custom("restart".to_string()));
        }
        
        actions
    }
    
    fn export_data(&self) -> Result<NodeExportData, NodeError> {
        Ok(NodeExportData {
            node_type: "application".to_string(),
            display_name: self.display_name(),
            description: self.description(),
            visual_data: self.visual_data(),
            metadata: self.base.metadata.clone(),
            type_specific_data: serde_json::to_value(&self.app_data)?,
        })
    }
    
    fn to_scene_node(&self) -> SceneNode {
        SceneNode {
            id: self.base.id,
            position: Position::new(
                self.base.visual_data.position[0],
                self.base.visual_data.position[1],
                self.base.visual_data.position[2],
            ),
            velocity: Vec3::zeros(),
            radius: self.base.visual_data.radius,
            color: self.base.visual_data.color,
            node_type: NodeType::Application {
                pid: self.process.unwrap_or(0),
                name: self.app_data.name.clone(),
            },
            metadata: self.base.metadata.clone(),
            visible: self.base.visual_data.visible,
            selected: self.base.visual_data.selected,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_application_node_creation() {
        let app = ApplicationNode::new(1, "Test App".to_string(), "/usr/bin/test".to_string())
            .with_category(ApplicationCategory::Development)
            .with_args(vec!["--test".to_string()]);
            
        assert_eq!(app.id(), 1);
        assert_eq!(app.display_name(), "Test App");
        assert!(!app.is_running());
        assert_eq!(app.app_data.category, ApplicationCategory::Development);
    }

    #[test]
    fn test_application_actions() {
        let mut app = ApplicationNode::new(2, "Test App".to_string(), "echo".to_string());
        
        let actions = app.available_actions();
        assert!(actions.contains(&NodeActionType::Open));
        assert!(actions.contains(&NodeActionType::Delete));
        
        // Note: We can't test actual process spawning in unit tests easily,
        // but we can test the action handling structure
        match app.handle_action(NodeAction::Delete) {
            Ok(NodeActionResult::Success { .. }) => (),
            _ => panic!("Expected success for delete action"),
        }
    }
}