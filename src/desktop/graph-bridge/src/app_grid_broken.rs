//! Application Grid Bridge
//!
//! Provides a traditional application launcher grid interface

use horizonos_graph_engine::GraphEngine;
use horizonos_graph_nodes::{NodeManager, ApplicationNode, NodeType};
use crate::{BridgeError, BridgeEvent};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

/// Application Grid Bridge
/// 
/// Presents applications in a traditional grid launcher while
/// mapping to graph application nodes
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

/// Application launch modes
#[derive(Debug, Clone)]
pub enum LaunchMode {
    /// Normal launch
    Normal,
    /// Launch with arguments
    WithArgs(Vec<String>),
    /// Launch in terminal
    Terminal,
    /// Launch as administrator
    Admin,
}

/// Grid events
#[derive(Debug, Clone)]
pub enum GridEvent {
    /// Application launched
    AppLaunched { app_id: String, mode: LaunchMode },
    /// Search query changed
    SearchChanged { query: String },
    /// Page changed
    PageChanged { page: usize },
    /// Category selected
    CategorySelected { category: String },
    /// Application favorited/unfavorited
    FavoriteToggled { app_id: String, is_favorite: bool },
    /// Grid shown/hidden
    VisibilityChanged { visible: bool },
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
        let app = self.applications.get(app_id)
            .ok_or_else(|| BridgeError::IoError(format!("Application not found: {}", app_id)))?;\n        \n        // Launch the application\n        let result = Command::new(&app.executable)\n            .spawn();\n        \n        match result {\n            Ok(_) => {\n                // Update launch statistics\n                if let Some(app_mut) = self.applications.get_mut(app_id) {\n                    app_mut.launch_count += 1;\n                    app_mut.last_launched = Some(std::time::SystemTime::now());\n                }\n                \n                // Update recent apps\n                self.add_to_recent(app_id.to_string());\n                \n                log::info!(\"Launched application: {} ({})\", app.name, app_id);\n                Ok(())\n            }\n            Err(e) => {\n                Err(BridgeError::IoError(format!(\"Failed to launch {}: {}\", app.name, e)))\n            }\n        }\n    }\n    \n    /// Launch application with arguments\n    pub fn launch_application_with_args(&mut self, app_id: &str, args: Vec<String>) -> Result<(), BridgeError> {\n        let app = self.applications.get(app_id)\n            .ok_or_else(|| BridgeError::IoError(format!(\"Application not found: {}\", app_id)))?;\n        \n        let result = Command::new(&app.executable)\n            .args(&args)\n            .spawn();\n        \n        match result {\n            Ok(_) => {\n                if let Some(app_mut) = self.applications.get_mut(app_id) {\n                    app_mut.launch_count += 1;\n                    app_mut.last_launched = Some(std::time::SystemTime::now());\n                }\n                self.add_to_recent(app_id.to_string());\n                log::info!(\"Launched application: {} with args: {:?}\", app.name, args);\n                Ok(())\n            }\n            Err(e) => {\n                Err(BridgeError::IoError(format!(\"Failed to launch {}: {}\", app.name, e)))\n            }\n        }\n    }\n    \n    /// Set search query\n    pub fn set_search_query(&mut self, query: String) {\n        self.search_query = query;\n        self.current_page = 0;\n        self.state = if self.search_query.is_empty() { GridState::Full } else { GridState::Search };\n        self.update_filtered_apps();\n    }\n    \n    /// Clear search\n    pub fn clear_search(&mut self) {\n        self.search_query.clear();\n        self.state = GridState::Full;\n        self.current_page = 0;\n        self.update_filtered_apps();\n    }\n    \n    /// Set current page\n    pub fn set_page(&mut self, page: usize) {\n        let max_page = self.get_page_count().saturating_sub(1);\n        self.current_page = page.min(max_page);\n    }\n    \n    /// Go to next page\n    pub fn next_page(&mut self) {\n        let max_page = self.get_page_count().saturating_sub(1);\n        if self.current_page < max_page {\n            self.current_page += 1;\n        }\n    }\n    \n    /// Go to previous page\n    pub fn prev_page(&mut self) {\n        if self.current_page > 0 {\n            self.current_page -= 1;\n        }\n    }\n    \n    /// Toggle favorite status\n    pub fn toggle_favorite(&mut self, app_id: &str) -> Result<(), BridgeError> {\n        let app = self.applications.get_mut(app_id)\n            .ok_or_else(|| BridgeError::IoError(format!(\"Application not found: {}\", app_id)))?;\n        \n        app.is_favorite = !app.is_favorite;\n        \n        if app.is_favorite {\n            if !self.favorites.contains(&app_id.to_string()) {\n                self.favorites.push(app_id.to_string());\n            }\n        } else {\n            self.favorites.retain(|id| id != app_id);\n        }\n        \n        log::info!(\"Toggled favorite for {}: {}\", app.name, app.is_favorite);\n        Ok(())\n    }\n    \n    /// Get applications for current page\n    pub fn get_current_page_apps(&self) -> Vec<&ApplicationInfo> {\n        let start = self.current_page * self.layout.items_per_page;\n        let end = (start + self.layout.items_per_page).min(self.filtered_apps.len());\n        \n        self.filtered_apps[start..end]\n            .iter()\n            .filter_map(|id| self.applications.get(id))\n            .collect()\n    }\n    \n    /// Get page count\n    pub fn get_page_count(&self) -> usize {\n        if self.layout.items_per_page == 0 {\n            1\n        } else {\n            (self.filtered_apps.len() + self.layout.items_per_page - 1) / self.layout.items_per_page\n        }\n    }\n    \n    /// Get applications by category\n    pub fn get_category_apps(&self, category: &str) -> Vec<&ApplicationInfo> {\n        if let Some(app_ids) = self.categories.get(category) {\n            app_ids.iter()\n                .filter_map(|id| self.applications.get(id))\n                .collect()\n        } else {\n            Vec::new()\n        }\n    }\n    \n    /// Get favorite applications\n    pub fn get_favorites(&self) -> Vec<&ApplicationInfo> {\n        self.favorites.iter()\n            .filter_map(|id| self.applications.get(id))\n            .collect()\n    }\n    \n    /// Get recent applications\n    pub fn get_recent(&self) -> Vec<&ApplicationInfo> {\n        self.recent_apps.iter()\n            .filter_map(|id| self.applications.get(id))\n            .collect()\n    }\n    \n    /// Get all categories\n    pub fn get_categories(&self) -> Vec<String> {\n        self.categories.keys().cloned().collect()\n    }\n    \n    /// Update the grid\n    pub fn update(&mut self, engine: &mut GraphEngine, node_manager: &mut NodeManager) -> Result<Vec<BridgeEvent>, BridgeError> {\n        let mut events = Vec::new();\n        \n        // Periodically rescan for new applications\n        // In a real implementation, this would be more intelligent\n        // For now, we skip rescanning to avoid performance issues\n        \n        Ok(events)\n    }\n    \n    /// Scan for applications\n    fn scan_applications(&mut self) -> Result<(), BridgeError> {\n        // Common application directories\n        let app_dirs = [\n            \"/usr/share/applications\",\n            \"/usr/local/share/applications\",\n            \"~/.local/share/applications\",\n        ];\n        \n        for dir in &app_dirs {\n            let expanded_dir = if dir.starts_with('~') {\n                if let Some(home) = std::env::home_dir() {\n                    PathBuf::from(dir.replace('~', &home.to_string_lossy()))\n                } else {\n                    continue;\n                }\n            } else {\n                PathBuf::from(dir)\n            };\n            \n            if expanded_dir.exists() {\n                self.scan_directory(&expanded_dir)?;\n            }\n        }\n        \n        // Create some default applications if none found\n        if self.applications.is_empty() {\n            self.create_default_applications();\n        }\n        \n        Ok(())\n    }\n    \n    /// Scan directory for desktop files\n    fn scan_directory(&mut self, dir: &PathBuf) -> Result<(), BridgeError> {\n        if let Ok(entries) = std::fs::read_dir(dir) {\n            for entry in entries {\n                if let Ok(entry) = entry {\n                    let path = entry.path();\n                    if path.extension().and_then(|s| s.to_str()) == Some(\"desktop\") {\n                        if let Ok(app_info) = self.parse_desktop_file(&path) {\n                            self.applications.insert(app_info.id.clone(), app_info);\n                        }\n                    }\n                }\n            }\n        }\n        Ok(())\n    }\n    \n    /// Parse desktop file\n    fn parse_desktop_file(&self, path: &PathBuf) -> Result<ApplicationInfo, BridgeError> {\n        // This is a simplified desktop file parser\n        // In a real implementation, you'd use a proper desktop file parser\n        \n        let content = std::fs::read_to_string(path)\n            .map_err(|e| BridgeError::IoError(e.to_string()))?;\n        \n        let mut name = path.file_stem().unwrap_or_default().to_string_lossy().to_string();\n        let mut exec = String::new();\n        let mut description = String::new();\n        let mut icon = None;\n        let mut categories = Vec::new();\n        \n        for line in content.lines() {\n            if line.starts_with(\"Name=\") {\n                name = line[5..].to_string();\n            } else if line.starts_with(\"Exec=\") {\n                exec = line[5..].to_string();\n            } else if line.starts_with(\"Comment=\") {\n                description = line[8..].to_string();\n            } else if line.starts_with(\"Icon=\") {\n                icon = Some(PathBuf::from(&line[5..]));\n            } else if line.starts_with(\"Categories=\") {\n                categories = line[11..].split(';').map(|s| s.to_string()).collect();\n            }\n        }\n        \n        // Extract executable name (remove arguments)\n        let executable = if let Some(space_pos) = exec.find(' ') {\n            PathBuf::from(&exec[..space_pos])\n        } else {\n            PathBuf::from(&exec)\n        };\n        \n        let app_id = path.file_stem().unwrap_or_default().to_string_lossy().to_string();\n        \n        Ok(ApplicationInfo {\n            id: app_id,\n            name,\n            description,\n            executable,\n            icon,\n            desktop_file: Some(path.clone()),\n            categories: categories.clone(),\n            keywords: vec![name.clone()], // Simple keyword extraction\n            launch_count: 0,\n            last_launched: None,\n            is_favorite: false,\n        })\n    }\n    \n    /// Create default applications for demonstration\n    fn create_default_applications(&mut self) {\n        let default_apps = [\n            (\"terminal\", \"Terminal\", \"Terminal emulator\", \"/usr/bin/gnome-terminal\"),\n            (\"browser\", \"Web Browser\", \"Browse the web\", \"/usr/bin/firefox\"),\n            (\"files\", \"File Manager\", \"Manage files\", \"/usr/bin/nautilus\"),\n            (\"editor\", \"Text Editor\", \"Edit text files\", \"/usr/bin/gedit\"),\n        ];\n        \n        for (id, name, description, executable) in &default_apps {\n            let app_info = ApplicationInfo {\n                id: id.to_string(),\n                name: name.to_string(),\n                description: description.to_string(),\n                executable: PathBuf::from(executable),\n                icon: None,\n                desktop_file: None,\n                categories: vec![\"Utilities\".to_string()],\n                keywords: vec![name.to_string()],\n                launch_count: 0,\n                last_launched: None,\n                is_favorite: false,\n            };\n            \n            self.applications.insert(id.to_string(), app_info);\n        }\n    }\n    \n    /// Create application nodes in the graph\n    fn create_application_nodes(&mut self, engine: &mut GraphEngine, node_manager: &mut NodeManager) -> Result<(), BridgeError> {\n        for app in self.applications.values() {\n            let node_id = node_manager.create_application_node(\n                app.name.clone(),\n                app.executable.clone(),\n            );\n            \n            log::debug!(\"Created application node: {} -> {:?}\", app.name, node_id);\n        }\n        \n        Ok(())\n    }\n    \n    /// Update filtered applications list\n    fn update_filtered_apps(&mut self) {\n        if self.search_query.is_empty() {\n            // Show all applications, sorted by name\n            self.filtered_apps = self.applications.keys().cloned().collect();\n            self.filtered_apps.sort_by(|a, b| {\n                let app_a = &self.applications[a];\n                let app_b = &self.applications[b];\n                app_a.name.cmp(&app_b.name)\n            });\n        } else {\n            // Filter and search\n            let query = self.search_query.to_lowercase();\n            self.filtered_apps = self.applications\n                .iter()\n                .filter(|(_, app)| {\n                    app.name.to_lowercase().contains(&query) ||\n                    app.description.to_lowercase().contains(&query) ||\n                    app.keywords.iter().any(|k| k.to_lowercase().contains(&query))\n                })\n                .map(|(id, _)| id.clone())\n                .collect();\n            \n            // Sort by relevance (name matches first, then others)\n            self.filtered_apps.sort_by(|a, b| {\n                let app_a = &self.applications[a];\n                let app_b = &self.applications[b];\n                \n                let name_match_a = app_a.name.to_lowercase().starts_with(&query);\n                let name_match_b = app_b.name.to_lowercase().starts_with(&query);\n                \n                match (name_match_a, name_match_b) {\n                    (true, false) => std::cmp::Ordering::Less,\n                    (false, true) => std::cmp::Ordering::Greater,\n                    _ => app_a.name.cmp(&app_b.name),\n                }\n            });\n        }\n        \n        // Update categories\n        self.update_categories();\n    }\n    \n    /// Update category mappings\n    fn update_categories(&mut self) {\n        self.categories.clear();\n        \n        for (app_id, app) in &self.applications {\n            for category in &app.categories {\n                self.categories.entry(category.clone())\n                    .or_insert_with(Vec::new)\n                    .push(app_id.clone());\n            }\n        }\n    }\n    \n    /// Add application to recent list\n    fn add_to_recent(&mut self, app_id: String) {\n        // Remove if already present\n        self.recent_apps.retain(|id| id != &app_id);\n        \n        // Add to front\n        self.recent_apps.insert(0, app_id);\n        \n        // Limit to 20 recent apps\n        if self.recent_apps.len() > 20 {\n            self.recent_apps.truncate(20);\n        }\n    }\n    \n    /// Set grid layout\n    pub fn set_layout(&mut self, layout: GridLayout) {\n        self.layout = layout;\n        self.current_page = 0; // Reset to first page\n        self.update_filtered_apps();\n    }\n    \n    /// Get current state\n    pub fn state(&self) -> GridState {\n        self.state\n    }\n    \n    /// Get search query\n    pub fn search_query(&self) -> &str {\n        &self.search_query\n    }\n    \n    /// Get current page\n    pub fn current_page(&self) -> usize {\n        self.current_page\n    }\n    \n    /// Get layout\n    pub fn layout(&self) -> &GridLayout {\n        &self.layout\n    }\n}\n\nimpl Default for ApplicationGrid {\n    fn default() -> Self {\n        Self::new()\n    }\n}\n\nimpl Default for GridLayout {\n    fn default() -> Self {\n        Self {\n            items_per_row: 6,\n            items_per_page: 24,\n            icon_size: IconSize::Large,\n            show_labels: true,\n            spacing: 16,\n        }\n    }\n}\n\nimpl IconSize {\n    /// Get pixel size\n    pub fn to_pixels(self) -> u32 {\n        match self {\n            IconSize::Small => 32,\n            IconSize::Medium => 48,\n            IconSize::Large => 64,\n            IconSize::ExtraLarge => 96,\n        }\n    }\n}"