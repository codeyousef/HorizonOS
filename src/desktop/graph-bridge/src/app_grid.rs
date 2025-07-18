//! Application Grid Bridge
//!
//! Provides a traditional application launcher grid interface

use horizonos_graph_engine::GraphEngine;
use horizonos_graph_nodes::NodeManager;
use crate::{BridgeError, BridgeEvent};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

/// Application Grid Bridge
/// 
/// Presents applications in a traditional grid launcher while
/// mapping to graph application nodes
#[allow(dead_code)]
pub struct ApplicationGrid {
    /// Available applications
    applications: HashMap<String, ApplicationInfo>,
    /// Grid layout configuration
    layout: GridLayout,
    /// Current page
    current_page: usize,
    /// Search query
    search_query: String,
    /// Filtered applications
    filtered_apps: Vec<String>,
    /// Grid state
    state: GridState,
    /// Categories
    categories: HashMap<String, Vec<String>>,
    /// Favorites
    favorites: Vec<String>,
    /// Recently used applications
    recent_apps: Vec<String>,
}

/// Application information
#[derive(Debug, Clone)]
pub struct ApplicationInfo {
    /// Application ID
    pub id: String,
    /// Display name
    pub name: String,
    /// Description
    pub description: String,
    /// Executable path
    pub executable: PathBuf,
    /// Icon path
    pub icon: Option<PathBuf>,
    /// Desktop file path
    pub desktop_file: Option<PathBuf>,
    /// Categories
    pub categories: Vec<String>,
    /// Keywords for searching
    pub keywords: Vec<String>,
    /// Launch count
    pub launch_count: u32,
    /// Last launched time
    pub last_launched: Option<std::time::SystemTime>,
    /// Is favorite
    pub is_favorite: bool,
}

/// Grid layout configuration
#[derive(Debug, Clone)]
pub struct GridLayout {
    /// Items per row
    pub items_per_row: usize,
    /// Items per page
    pub items_per_page: usize,
    /// Icon size
    pub icon_size: IconSize,
    /// Show labels
    pub show_labels: bool,
    /// Grid spacing
    pub spacing: u32,
}

/// Icon sizes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconSize {
    /// Small icons (32x32)
    Small,
    /// Medium icons (48x48)
    Medium,
    /// Large icons (64x64)
    Large,
    /// Extra large icons (96x96)
    ExtraLarge,
}

/// Grid state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridState {
    /// Hidden
    Hidden,
    /// Overlay mode (transparent over graph)
    Overlay,
    /// Full mode (traditional app grid)
    Full,
    /// Search mode
    Search,
}

impl ApplicationGrid {
    /// Create a new application grid
    pub fn new() -> Self {
        Self {
            applications: HashMap::new(),
            layout: GridLayout::default(),
            current_page: 0,
            search_query: String::new(),
            filtered_apps: Vec::new(),
            state: GridState::Hidden,
            categories: HashMap::new(),
            favorites: Vec::new(),
            recent_apps: Vec::new(),
        }
    }
    
    /// Initialize the application grid
    pub fn initialize(&mut self, engine: &mut GraphEngine, node_manager: &mut NodeManager) -> Result<(), BridgeError> {
        // Scan for applications
        self.scan_applications()?;
        
        // Create application nodes in the graph
        self.create_application_nodes(engine, node_manager)?;
        
        // Update filtered list
        self.update_filtered_apps();
        
        log::info!("Application grid initialized with {} applications", self.applications.len());
        Ok(())
    }
    
    /// Show the application grid
    pub fn show(&mut self) -> Result<(), BridgeError> {
        self.state = GridState::Full;
        self.update_filtered_apps();
        log::info!("Application grid shown");
        Ok(())
    }
    
    /// Hide the application grid
    pub fn hide(&mut self) {
        self.state = GridState::Hidden;
        self.search_query.clear();
        self.current_page = 0;
    }
    
    /// Show as overlay
    pub fn show_overlay(&mut self) {
        self.state = GridState::Overlay;
        self.update_filtered_apps();
    }
    
    /// Launch application
    pub fn launch_application(&mut self, app_id: &str) -> Result<(), BridgeError> {
        let app_executable = {
            let app = self.applications.get(app_id)
                .ok_or_else(|| BridgeError::IoError(format!("Application not found: {}", app_id)))?;
            app.executable.clone()
        };
        
        // Launch the application
        let result = Command::new(&app_executable)
            .spawn();
        
        match result {
            Ok(_) => {
                // Update launch statistics
                if let Some(app_mut) = self.applications.get_mut(app_id) {
                    app_mut.launch_count += 1;
                    app_mut.last_launched = Some(std::time::SystemTime::now());
                }
                
                // Update recent apps
                self.add_to_recent(app_id.to_string());
                
                log::info!("Launched application: {}", app_id);
                Ok(())
            }
            Err(e) => {
                Err(BridgeError::IoError(format!("Failed to launch {}: {}", app_id, e)))
            }
        }
    }
    
    /// Update the grid
    pub fn update(&mut self, _engine: &mut GraphEngine, _node_manager: &mut NodeManager) -> Result<Vec<BridgeEvent>, BridgeError> {
        Ok(Vec::new())
    }
    
    /// Scan for applications
    fn scan_applications(&mut self) -> Result<(), BridgeError> {
        // Create some default applications for now
        self.create_default_applications();
        Ok(())
    }
    
    /// Create default applications for demonstration
    fn create_default_applications(&mut self) {
        let default_apps = [
            ("terminal", "Terminal", "Terminal emulator", "/usr/bin/gnome-terminal"),
            ("browser", "Web Browser", "Browse the web", "/usr/bin/firefox"),
            ("files", "File Manager", "Manage files", "/usr/bin/nautilus"),
            ("editor", "Text Editor", "Edit text files", "/usr/bin/gedit"),
        ];
        
        for (id, name, description, executable) in &default_apps {
            let app_info = ApplicationInfo {
                id: id.to_string(),
                name: name.to_string(),
                description: description.to_string(),
                executable: PathBuf::from(executable),
                icon: None,
                desktop_file: None,
                categories: vec!["Utilities".to_string()],
                keywords: vec![name.to_string()],
                launch_count: 0,
                last_launched: None,
                is_favorite: false,
            };
            
            self.applications.insert(id.to_string(), app_info);
        }
    }
    
    /// Create application nodes in the graph
    fn create_application_nodes(&mut self, _engine: &mut GraphEngine, node_manager: &mut NodeManager) -> Result<(), BridgeError> {
        for app in self.applications.values() {
            let node_id = node_manager.create_application(
                app.name.clone(),
                app.executable.to_string_lossy().to_string(),
            );
            
            log::debug!("Created application node: {} -> {:?}", app.name, node_id);
        }
        
        Ok(())
    }
    
    /// Update filtered applications list
    fn update_filtered_apps(&mut self) {
        self.filtered_apps = self.applications.keys().cloned().collect();
        self.filtered_apps.sort_by(|a, b| {
            let app_a = &self.applications[a];
            let app_b = &self.applications[b];
            app_a.name.cmp(&app_b.name)
        });
    }
    
    /// Add application to recent list
    fn add_to_recent(&mut self, app_id: String) {
        // Remove if already present
        self.recent_apps.retain(|id| id != &app_id);
        
        // Add to front
        self.recent_apps.insert(0, app_id);
        
        // Limit to 20 recent apps
        if self.recent_apps.len() > 20 {
            self.recent_apps.truncate(20);
        }
    }
}

impl Default for ApplicationGrid {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for GridLayout {
    fn default() -> Self {
        Self {
            items_per_row: 6,
            items_per_page: 24,
            icon_size: IconSize::Large,
            show_labels: true,
            spacing: 16,
        }
    }
}

impl IconSize {
    /// Get pixel size
    pub fn to_pixels(self) -> u32 {
        match self {
            IconSize::Small => 32,
            IconSize::Medium => 48,
            IconSize::Large => 64,
            IconSize::ExtraLarge => 96,
        }
    }
}