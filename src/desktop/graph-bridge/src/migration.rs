//! Migration Tools
//!
//! Helps users transition from traditional desktop environments to the graph desktop

use horizonos_graph_engine::GraphEngine;
use horizonos_graph_nodes::NodeManager;
use crate::{BridgeError, BridgeEvent};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Migration Tools
/// 
/// Analyzes existing desktop configuration and helps migrate to graph desktop
pub struct MigrationTools {
    /// Migration status
    status: MigrationStatus,
    /// Migration steps
    steps: Vec<MigrationStep>,
    /// Current step index
    current_step: usize,
    /// Migration configuration
    config: MigrationConfig,
    /// Discovered configuration
    discovered_config: DiscoveredConfig,
    /// Migration progress
    progress: f32,
}

/// Migration status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationStatus {
    /// Not started
    NotStarted,
    /// Scanning existing configuration
    Scanning,
    /// Ready to migrate
    Ready,
    /// Migrating
    InProgress,
    /// Migration completed successfully
    Completed,
    /// Migration failed
    Failed,
    /// Migration paused
    Paused,
}

/// Migration step
#[derive(Debug, Clone)]
pub struct MigrationStep {
    /// Step ID
    pub id: String,
    /// Step name
    pub name: String,
    /// Step description
    pub description: String,
    /// Step status
    pub status: StepStatus,
    /// Step progress (0.0 to 1.0)
    pub progress: f32,
    /// Estimated time (seconds)
    pub estimated_time: u32,
    /// Is skippable
    pub skippable: bool,
    /// Step type
    pub step_type: StepType,
}

/// Step status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepStatus {
    /// Pending
    Pending,
    /// In progress
    InProgress,
    /// Completed
    Completed,
    /// Failed
    Failed,
    /// Skipped
    Skipped,
}

/// Step types
#[derive(Debug, Clone)]
pub enum StepType {
    /// Scan desktop environment
    ScanDesktopEnvironment,
    /// Import bookmarks
    ImportBookmarks,
    /// Import applications
    ImportApplications,
    /// Import files and folders
    ImportFiles,
    /// Import desktop settings
    ImportSettings,
    /// Create graph relationships
    CreateRelationships,
    /// Migrate workspaces
    MigrateWorkspaces,
    /// Setup graph layout
    SetupLayout,
    /// Create welcome tour
    CreateWelcomeTour,
}

/// Migration configuration
#[derive(Debug, Clone)]
pub struct MigrationConfig {
    /// Auto-detect desktop environment
    pub auto_detect_de: bool,
    /// Import browser bookmarks
    pub import_bookmarks: bool,
    /// Import application settings
    pub import_app_settings: bool,
    /// Import file associations
    pub import_file_associations: bool,
    /// Create semantic relationships
    pub create_relationships: bool,
    /// Import wallpapers and themes
    pub import_themes: bool,
    /// Show migration wizard
    pub show_wizard: bool,
    /// Create backup
    pub create_backup: bool,
}

/// Discovered desktop configuration
#[derive(Debug, Clone)]
pub struct DiscoveredConfig {
    /// Desktop environment
    pub desktop_environment: Option<DesktopEnvironment>,
    /// Installed applications
    pub applications: Vec<DiscoveredApplication>,
    /// Browser bookmarks
    pub bookmarks: Vec<Bookmark>,
    /// File associations
    pub file_associations: HashMap<String, String>,
    /// Desktop settings
    pub desktop_settings: DesktopSettings,
    /// Workspaces/virtual desktops
    pub workspaces: Vec<DiscoveredWorkspace>,
    /// Theme information
    pub theme_info: ThemeInfo,
}

/// Detected desktop environment
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopEnvironment {
    /// GNOME
    Gnome,
    /// KDE Plasma
    Kde,
    /// XFCE
    Xfce,
    /// Cinnamon
    Cinnamon,
    /// MATE
    Mate,
    /// LXQt
    LxQt,
    /// i3
    I3,
    /// Sway
    Sway,
    /// Hyprland
    Hyprland,
    /// Other/Unknown
    Other(String),
}

/// Discovered application
#[derive(Debug, Clone)]
pub struct DiscoveredApplication {
    /// Application name
    pub name: String,
    /// Executable path
    pub executable: PathBuf,
    /// Desktop file path
    pub desktop_file: Option<PathBuf>,
    /// Categories
    pub categories: Vec<String>,
    /// Usage frequency (if available)
    pub usage_frequency: Option<u32>,
    /// Last used time
    pub last_used: Option<std::time::SystemTime>,
}

/// Browser bookmark
#[derive(Debug, Clone)]
pub struct Bookmark {
    /// Bookmark title
    pub title: String,
    /// URL
    pub url: String,
    /// Folder path
    pub folder: Vec<String>,
    /// Tags
    pub tags: Vec<String>,
    /// Browser source
    pub browser: String,
}

/// Desktop settings
#[derive(Debug, Clone)]
pub struct DesktopSettings {
    /// Wallpaper path
    pub wallpaper: Option<PathBuf>,
    /// Theme name
    pub theme: Option<String>,
    /// Icon theme
    pub icon_theme: Option<String>,
    /// Font settings
    pub font_settings: FontSettings,
    /// Shortcuts
    pub shortcuts: Vec<Shortcut>,
}

/// Font settings
#[derive(Debug, Clone)]
pub struct FontSettings {
    /// System font
    pub system_font: Option<String>,
    /// Document font
    pub document_font: Option<String>,
    /// Monospace font
    pub monospace_font: Option<String>,
    /// Font size
    pub font_size: Option<u32>,
}

/// Keyboard shortcut
#[derive(Debug, Clone)]
pub struct Shortcut {
    /// Key combination
    pub keys: String,
    /// Action description
    pub action: String,
    /// Command to execute
    pub command: Option<String>,
}

/// Discovered workspace
#[derive(Debug, Clone)]
pub struct DiscoveredWorkspace {
    /// Workspace name
    pub name: String,
    /// Workspace number
    pub number: u32,
    /// Applications in workspace
    pub applications: Vec<String>,
}

/// Theme information
#[derive(Debug, Clone)]
pub struct ThemeInfo {
    /// Color scheme
    pub color_scheme: Option<String>,
    /// Dark mode enabled
    pub dark_mode: bool,
    /// Accent color
    pub accent_color: Option<String>,
    /// Window decoration theme
    pub window_theme: Option<String>,
}

impl MigrationTools {
    /// Create new migration tools
    pub fn new() -> Self {
        Self {
            status: MigrationStatus::NotStarted,
            steps: Vec::new(),
            current_step: 0,
            config: MigrationConfig::default(),
            discovered_config: DiscoveredConfig::default(),
            progress: 0.0,
        }
    }
    
    /// Start migration process
    pub fn start_migration(&mut self, engine: &mut GraphEngine, node_manager: &mut NodeManager) -> Result<(), BridgeError> {
        if self.status != MigrationStatus::NotStarted {
            return Err(BridgeError::MigrationFailed("Migration already started".to_string()));
        }
        
        self.status = MigrationStatus::Scanning;
        self.create_migration_steps();
        
        // Start scanning existing configuration
        self.scan_desktop_environment()?;
        
        self.status = MigrationStatus::Ready;
        log::info!("Migration scan completed, ready to migrate");
        
        // If auto-migration is enabled, start immediately
        if self.config.auto_detect_de {
            self.execute_migration(engine, node_manager)?;
        }
        
        Ok(())
    }
    
    /// Execute migration
    pub fn execute_migration(&mut self, engine: &mut GraphEngine, node_manager: &mut NodeManager) -> Result<(), BridgeError> {
        if self.status != MigrationStatus::Ready {
            return Err(BridgeError::MigrationFailed("Migration not ready".to_string()));
        }
        
        self.status = MigrationStatus::InProgress;
        self.current_step = 0;
        
        // Execute each migration step
        for (index, step) in self.steps.clone().iter().enumerate() {
            self.current_step = index;
            
            if let Err(e) = self.execute_step(step, engine, node_manager) {\n                self.status = MigrationStatus::Failed;\n                return Err(e);\n            }\n            \n            // Update progress\n            self.progress = (index + 1) as f32 / self.steps.len() as f32;\n        }\n        \n        self.status = MigrationStatus::Completed;\n        self.progress = 1.0;\n        \n        log::info!("Migration completed successfully");\n        Ok(())\n    }\n    \n    /// Get migration status\n    pub fn status(&self) -> MigrationStatus {\n        self.status\n    }\n    \n    /// Get migration progress\n    pub fn progress(&self) -> f32 {\n        self.progress\n    }\n    \n    /// Get current step\n    pub fn current_step(&self) -> Option<&MigrationStep> {\n        self.steps.get(self.current_step)\n    }\n    \n    /// Get all steps\n    pub fn steps(&self) -> &[MigrationStep] {\n        &self.steps\n    }\n    \n    /// Update migration tools\n    pub fn update(&mut self, engine: &mut GraphEngine, node_manager: &mut NodeManager) -> Result<Vec<BridgeEvent>, BridgeError> {\n        let mut events = Vec::new();\n        \n        // Check for migration completion or failure\n        match self.status {\n            MigrationStatus::InProgress => {\n                // Continue migration if needed\n            }\n            MigrationStatus::Completed => {\n                events.push(BridgeEvent::MigrationCompleted { success: true });\n            }\n            MigrationStatus::Failed => {\n                events.push(BridgeEvent::MigrationCompleted { success: false });\n            }\n            _ => {}\n        }\n        \n        Ok(events)\n    }\n    \n    /// Scan desktop environment\n    fn scan_desktop_environment(&mut self) -> Result<(), BridgeError> {\n        // Detect desktop environment\n        self.discovered_config.desktop_environment = self.detect_desktop_environment();\n        \n        // Scan applications\n        self.discovered_config.applications = self.scan_applications()?;\n        \n        // Scan bookmarks\n        if self.config.import_bookmarks {\n            self.discovered_config.bookmarks = self.scan_bookmarks()?;\n        }\n        \n        // Scan settings\n        self.discovered_config.desktop_settings = self.scan_desktop_settings()?;\n        \n        // Scan workspaces\n        self.discovered_config.workspaces = self.scan_workspaces()?;\n        \n        // Scan theme info\n        self.discovered_config.theme_info = self.scan_theme_info()?;\n        \n        log::info!("Desktop environment scan completed: {:?}", self.discovered_config.desktop_environment);\n        Ok(())\n    }\n    \n    /// Detect desktop environment\n    fn detect_desktop_environment(&self) -> Option<DesktopEnvironment> {\n        // Check environment variables\n        if let Ok(desktop) = std::env::var("XDG_CURRENT_DESKTOP") {\n            match desktop.to_lowercase().as_str() {\n                "gnome" => return Some(DesktopEnvironment::Gnome),\n                "kde" => return Some(DesktopEnvironment::Kde),\n                "xfce" => return Some(DesktopEnvironment::Xfce),\n                "x-cinnamon" => return Some(DesktopEnvironment::Cinnamon),\n                "mate" => return Some(DesktopEnvironment::Mate),\n                "lxqt" => return Some(DesktopEnvironment::LxQt),\n                "i3" => return Some(DesktopEnvironment::I3),\n                "sway" => return Some(DesktopEnvironment::Sway),\n                "hyprland" => return Some(DesktopEnvironment::Hyprland),\n                _ => return Some(DesktopEnvironment::Other(desktop)),\n            }\n        }\n        \n        // Check for specific desktop files or processes\n        let desktop_indicators = [\n            ("/usr/bin/gnome-shell", DesktopEnvironment::Gnome),\n            ("/usr/bin/plasmashell", DesktopEnvironment::Kde),\n            ("/usr/bin/xfce4-session", DesktopEnvironment::Xfce),\n            ("/usr/bin/cinnamon", DesktopEnvironment::Cinnamon),\n            ("/usr/bin/mate-session", DesktopEnvironment::Mate),\n            ("/usr/bin/lxqt-session", DesktopEnvironment::LxQt),\n            ("/usr/bin/i3", DesktopEnvironment::I3),\n            ("/usr/bin/sway", DesktopEnvironment::Sway),\n            ("/usr/bin/Hyprland", DesktopEnvironment::Hyprland),\n        ];\n        \n        for (path, de) in &desktop_indicators {\n            if Path::new(path).exists() {\n                return Some(*de);\n            }\n        }\n        \n        None\n    }\n    \n    /// Scan applications\n    fn scan_applications(&self) -> Result<Vec<DiscoveredApplication>, BridgeError> {\n        let mut applications = Vec::new();\n        \n        // Scan standard application directories\n        let app_dirs = [\n            "/usr/share/applications",\n            "/usr/local/share/applications",\n            "~/.local/share/applications",\n        ];\n        \n        for dir in &app_dirs {\n            let expanded_dir = if dir.starts_with('~') {\n                if let Some(home) = std::env::home_dir() {\n                    PathBuf::from(dir.replace('~', &home.to_string_lossy()))\n                } else {\n                    continue;\n                }\n            } else {\n                PathBuf::from(dir)\n            };\n            \n            if expanded_dir.exists() {\n                if let Ok(entries) = std::fs::read_dir(&expanded_dir) {\n                    for entry in entries {\n                        if let Ok(entry) = entry {\n                            let path = entry.path();\n                            if path.extension().and_then(|s| s.to_str()) == Some("desktop") {\n                                if let Ok(app) = self.parse_desktop_file_for_migration(&path) {\n                                    applications.push(app);\n                                }\n                            }\n                        }\n                    }\n                }\n            }\n        }\n        \n        log::info!("Discovered {} applications", applications.len());\n        Ok(applications)\n    }\n    \n    /// Parse desktop file for migration\n    fn parse_desktop_file_for_migration(&self, path: &PathBuf) -> Result<DiscoveredApplication, BridgeError> {\n        let content = std::fs::read_to_string(path)\n            .map_err(|e| BridgeError::IoError(e.to_string()))?;\n        \n        let mut name = String::new();\n        let mut exec = String::new();\n        let mut categories = Vec::new();\n        \n        for line in content.lines() {\n            if line.starts_with("Name=") {\n                name = line[5..].to_string();\n            } else if line.starts_with("Exec=") {\n                exec = line[5..].to_string();\n            } else if line.starts_with("Categories=") {\n                categories = line[11..].split(';').map(|s| s.to_string()).collect();\n            }\n        }\n        \n        let executable = if let Some(space_pos) = exec.find(' ') {\n            PathBuf::from(&exec[..space_pos])\n        } else {\n            PathBuf::from(&exec)\n        };\n        \n        Ok(DiscoveredApplication {\n            name,\n            executable,\n            desktop_file: Some(path.clone()),\n            categories,\n            usage_frequency: None,\n            last_used: None,\n        })\n    }\n    \n    /// Scan browser bookmarks\n    fn scan_bookmarks(&self) -> Result<Vec<Bookmark>, BridgeError> {\n        let mut bookmarks = Vec::new();\n        \n        // Firefox bookmarks\n        if let Some(firefox_bookmarks) = self.scan_firefox_bookmarks()? {\n            bookmarks.extend(firefox_bookmarks);\n        }\n        \n        // Chrome/Chromium bookmarks\n        if let Some(chrome_bookmarks) = self.scan_chrome_bookmarks()? {\n            bookmarks.extend(chrome_bookmarks);\n        }\n        \n        log::info!("Discovered {} bookmarks", bookmarks.len());\n        Ok(bookmarks)\n    }\n    \n    /// Scan Firefox bookmarks\n    fn scan_firefox_bookmarks(&self) -> Result<Option<Vec<Bookmark>>, BridgeError> {\n        // This is a simplified implementation\n        // In practice, you'd parse Firefox's places.sqlite database\n        Ok(None)\n    }\n    \n    /// Scan Chrome bookmarks\n    fn scan_chrome_bookmarks(&self) -> Result<Option<Vec<Bookmark>>, BridgeError> {\n        // This is a simplified implementation\n        // In practice, you'd parse Chrome's Bookmarks JSON file\n        Ok(None)\n    }\n    \n    /// Scan desktop settings\n    fn scan_desktop_settings(&self) -> Result<DesktopSettings, BridgeError> {\n        // This would scan various configuration files depending on the DE\n        Ok(DesktopSettings {\n            wallpaper: None,\n            theme: None,\n            icon_theme: None,\n            font_settings: FontSettings {\n                system_font: None,\n                document_font: None,\n                monospace_font: None,\n                font_size: None,\n            },\n            shortcuts: Vec::new(),\n        })\n    }\n    \n    /// Scan workspaces\n    fn scan_workspaces(&self) -> Result<Vec<DiscoveredWorkspace>, BridgeError> {\n        // This would scan workspace configuration\n        Ok(Vec::new())\n    }\n    \n    /// Scan theme information\n    fn scan_theme_info(&self) -> Result<ThemeInfo, BridgeError> {\n        Ok(ThemeInfo {\n            color_scheme: None,\n            dark_mode: false,\n            accent_color: None,\n            window_theme: None,\n        })\n    }\n    \n    /// Create migration steps\n    fn create_migration_steps(&mut self) {\n        self.steps = vec![\n            MigrationStep {\n                id: "scan_desktop".to_string(),\n                name: "Scan Desktop Environment".to_string(),\n                description: "Detecting current desktop environment and configuration".to_string(),\n                status: StepStatus::Pending,\n                progress: 0.0,\n                estimated_time: 30,\n                skippable: false,\n                step_type: StepType::ScanDesktopEnvironment,\n            },\n            MigrationStep {\n                id: "import_apps".to_string(),\n                name: "Import Applications".to_string(),\n                description: "Importing installed applications and creating graph nodes".to_string(),\n                status: StepStatus::Pending,\n                progress: 0.0,\n                estimated_time: 60,\n                skippable: false,\n                step_type: StepType::ImportApplications,\n            },\n            MigrationStep {\n                id: "import_bookmarks".to_string(),\n                name: "Import Bookmarks".to_string(),\n                description: "Importing browser bookmarks and web resources".to_string(),\n                status: StepStatus::Pending,\n                progress: 0.0,\n                estimated_time: 45,\n                skippable: true,\n                step_type: StepType::ImportBookmarks,\n            },\n            MigrationStep {\n                id: "import_files".to_string(),\n                name: "Import Files".to_string(),\n                description: "Importing important files and folders".to_string(),\n                status: StepStatus::Pending,\n                progress: 0.0,\n                estimated_time: 120,\n                skippable: true,\n                step_type: StepType::ImportFiles,\n            },\n            MigrationStep {\n                id: "create_relationships".to_string(),\n                name: "Create Relationships".to_string(),\n                description: "Analyzing and creating semantic relationships between items".to_string(),\n                status: StepStatus::Pending,\n                progress: 0.0,\n                estimated_time: 90,\n                skippable: true,\n                step_type: StepType::CreateRelationships,\n            },\n            MigrationStep {\n                id: "setup_layout".to_string(),\n                name: "Setup Graph Layout".to_string(),\n                description: "Arranging graph nodes in an optimal layout".to_string(),\n                status: StepStatus::Pending,\n                progress: 0.0,\n                estimated_time: 30,\n                skippable: false,\n                step_type: StepType::SetupLayout,\n            },\n            MigrationStep {\n                id: "welcome_tour".to_string(),\n                name: "Create Welcome Tour".to_string(),\n                description: "Setting up interactive tour of the graph desktop".to_string(),\n                status: StepStatus::Pending,\n                progress: 0.0,\n                estimated_time: 15,\n                skippable: true,\n                step_type: StepType::CreateWelcomeTour,\n            },\n        ];\n    }\n    \n    /// Execute a migration step\n    fn execute_step(&mut self, step: &MigrationStep, engine: &mut GraphEngine, node_manager: &mut NodeManager) -> Result<(), BridgeError> {\n        log::info!("Executing migration step: {}", step.name);\n        \n        // Update step status\n        if let Some(step_mut) = self.steps.get_mut(self.current_step) {\n            step_mut.status = StepStatus::InProgress;\n        }\n        \n        match &step.step_type {\n            StepType::ScanDesktopEnvironment => {\n                // Already done in start_migration\n            }\n            StepType::ImportApplications => {\n                self.import_applications(engine, node_manager)?;\n            }\n            StepType::ImportBookmarks => {\n                self.import_bookmarks(engine, node_manager)?;\n            }\n            StepType::ImportFiles => {\n                self.import_files(engine, node_manager)?;\n            }\n            StepType::CreateRelationships => {\n                self.create_relationships(engine, node_manager)?;\n            }\n            StepType::SetupLayout => {\n                self.setup_layout(engine, node_manager)?;\n            }\n            StepType::CreateWelcomeTour => {\n                self.create_welcome_tour(engine, node_manager)?;\n            }\n            _ => {\n                // Other step types not implemented yet\n            }\n        }\n        \n        // Mark step as completed\n        if let Some(step_mut) = self.steps.get_mut(self.current_step) {\n            step_mut.status = StepStatus::Completed;\n            step_mut.progress = 1.0;\n        }\n        \n        Ok(())\n    }\n    \n    /// Import applications into graph\n    fn import_applications(&self, engine: &mut GraphEngine, node_manager: &mut NodeManager) -> Result<(), BridgeError> {\n        for app in &self.discovered_config.applications {\n            let node_id = node_manager.create_application_node(\n                app.name.clone(),\n                app.executable.clone(),\n            );\n            \n            log::debug!("Imported application: {} -> {:?}", app.name, node_id);\n        }\n        \n        Ok(())\n    }\n    \n    /// Import bookmarks into graph\n    fn import_bookmarks(&self, engine: &mut GraphEngine, node_manager: &mut NodeManager) -> Result<(), BridgeError> {\n        for bookmark in &self.discovered_config.bookmarks {\n            // Create URL node for bookmark\n            let node_id = node_manager.create_url_node(\n                bookmark.title.clone(),\n                bookmark.url.clone(),\n            );\n            \n            log::debug!("Imported bookmark: {} -> {:?}", bookmark.title, node_id);\n        }\n        \n        Ok(())\n    }\n    \n    /// Import files into graph\n    fn import_files(&self, engine: &mut GraphEngine, node_manager: &mut NodeManager) -> Result<(), BridgeError> {\n        // Import important directories\n        let important_dirs = [\n            std::env::home_dir().unwrap_or_else(|| PathBuf::from("/")),\n            PathBuf::from("/usr/share/applications"),\n        ];\n        \n        for dir in &important_dirs {\n            if dir.exists() {\n                let node_id = node_manager.create_file_node(\n                    dir.file_name().unwrap_or_default().to_string_lossy().to_string(),\n                    dir.clone(),\n                );\n                \n                log::debug!("Imported directory: {} -> {:?}", dir.display(), node_id);\n            }\n        }\n        \n        Ok(())\n    }\n    \n    /// Create relationships between imported items\n    fn create_relationships(&self, engine: &mut GraphEngine, node_manager: &mut NodeManager) -> Result<(), BridgeError> {\n        // This would analyze patterns and create semantic relationships\n        // For now, just log that we're creating relationships\n        log::info!("Creating semantic relationships between {} items", \n                   self.discovered_config.applications.len() + self.discovered_config.bookmarks.len());\n        \n        Ok(())\n    }\n    \n    /// Setup graph layout\n    fn setup_layout(&self, engine: &mut GraphEngine, node_manager: &mut NodeManager) -> Result<(), BridgeError> {\n        // This would arrange nodes in an optimal layout\n        log::info!("Setting up graph layout");\n        \n        Ok(())\n    }\n    \n    /// Create welcome tour\n    fn create_welcome_tour(&self, engine: &mut GraphEngine, node_manager: &mut NodeManager) -> Result<(), BridgeError> {\n        // This would create an interactive tour\n        log::info!("Creating welcome tour");\n        \n        Ok(())\n    }\n    \n    /// Get discovered configuration\n    pub fn discovered_config(&self) -> &DiscoveredConfig {\n        &self.discovered_config\n    }\n    \n    /// Set migration configuration\n    pub fn set_config(&mut self, config: MigrationConfig) {\n        self.config = config;\n    }\n    \n    /// Get migration configuration\n    pub fn config(&self) -> &MigrationConfig {\n        &self.config\n    }\n}\n\nimpl Default for MigrationTools {\n    fn default() -> Self {\n        Self::new()\n    }\n}\n\nimpl Default for MigrationConfig {\n    fn default() -> Self {\n        Self {\n            auto_detect_de: true,\n            import_bookmarks: true,\n            import_app_settings: true,\n            import_file_associations: true,\n            create_relationships: true,\n            import_themes: true,\n            show_wizard: true,\n            create_backup: true,\n        }\n    }\n}\n\nimpl Default for DiscoveredConfig {\n    fn default() -> Self {\n        Self {\n            desktop_environment: None,\n            applications: Vec::new(),\n            bookmarks: Vec::new(),\n            file_associations: HashMap::new(),\n            desktop_settings: DesktopSettings {\n                wallpaper: None,\n                theme: None,\n                icon_theme: None,\n                font_settings: FontSettings {\n                    system_font: None,\n                    document_font: None,\n                    monospace_font: None,\n                    font_size: None,\n                },\n                shortcuts: Vec::new(),\n            },\n            workspaces: Vec::new(),\n            theme_info: ThemeInfo {\n                color_scheme: None,\n                dark_mode: false,\n                accent_color: None,\n                window_theme: None,\n            },\n        }\n    }\n}"