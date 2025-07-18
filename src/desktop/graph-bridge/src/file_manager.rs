//! File Manager Bridge
//!
//! Provides a traditional file manager interface that maps to graph nodes

use horizonos_graph_engine::GraphEngine;
use horizonos_graph_nodes::NodeManager;
use crate::{BridgeError, BridgeEvent};
use std::path::{Path, PathBuf};
use std::collections::HashMap;

/// File Manager Bridge
/// 
/// Presents file system as a traditional hierarchical tree while
/// mapping operations to graph nodes and relationships
pub struct FileManagerBridge {
    /// Current location in file system
    current_path: PathBuf,
    /// View mode (list, grid, details)
    view_mode: ViewMode,
    /// Selected files/folders
    selection: Vec<PathBuf>,
    /// File manager windows
    windows: HashMap<u64, FileManagerWindow>,
    /// Next window ID
    next_window_id: u64,
    /// Bridge state
    state: FileManagerState,
}

/// File manager view modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    /// List view
    List,
    /// Grid/icon view
    Grid,
    /// Details view with metadata
    Details,
    /// Thumbnail view for images
    Thumbnails,
}

/// File manager window
#[derive(Debug, Clone)]
pub struct FileManagerWindow {
    /// Window ID
    pub id: u64,
    /// Current path
    pub path: PathBuf,
    /// View mode
    pub view_mode: ViewMode,
    /// Selection
    pub selection: Vec<PathBuf>,
    /// Visibility
    pub visible: bool,
    /// Window dimensions
    pub dimensions: WindowDimensions,
}

/// Window dimensions
#[derive(Debug, Clone)]
pub struct WindowDimensions {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// File manager state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileManagerState {
    /// Hidden
    Hidden,
    /// Overlay mode (transparent over graph)
    Overlay,
    /// Full mode (traditional file manager)
    Full,
}

/// File operation types
#[derive(Debug, Clone)]
pub enum FileOperation {
    /// Copy files
    Copy { source: Vec<PathBuf>, destination: PathBuf },
    /// Move files
    Move { source: Vec<PathBuf>, destination: PathBuf },
    /// Delete files
    Delete { paths: Vec<PathBuf> },
    /// Create directory
    CreateDirectory { path: PathBuf },
    /// Rename file/directory
    Rename { old_path: PathBuf, new_path: PathBuf },
    /// Open file/directory
    Open { path: PathBuf },
}

/// File manager events
#[derive(Debug, Clone)]
pub enum FileManagerEvent {
    /// Location changed
    LocationChanged { old_path: PathBuf, new_path: PathBuf },
    /// Selection changed
    SelectionChanged { paths: Vec<PathBuf> },
    /// File operation performed
    OperationPerformed { operation: FileOperation },
    /// Window opened
    WindowOpened { window_id: u64, path: PathBuf },
    /// Window closed
    WindowClosed { window_id: u64 },
    /// View mode changed
    ViewModeChanged { mode: ViewMode },
}

impl FileManagerBridge {
    /// Create a new file manager bridge
    pub fn new() -> Self {
        Self {
            current_path: std::env::home_dir().unwrap_or_else(|| PathBuf::from("/")),
            view_mode: ViewMode::Grid,
            selection: Vec::new(),
            windows: HashMap::new(),
            next_window_id: 1,
            state: FileManagerState::Hidden,
        }
    }
    
    /// Initialize the file manager bridge
    pub fn initialize(&mut self, engine: &mut GraphEngine, node_manager: &mut NodeManager) -> Result<(), BridgeError> {
        // Scan initial directory and create file nodes
        self.scan_directory(&self.current_path.clone(), engine, node_manager)?;
        
        log::info!("File manager bridge initialized at: {}", self.current_path.display());
        Ok(())
    }
    
    /// Open file manager at specific location
    pub fn open_location(&mut self, path: &Path) -> Result<(), BridgeError> {
        if !path.exists() {
            return Err(BridgeError::IoError(format!("Path does not exist: {}", path.display())));
        }
        
        let _old_path = self.current_path.clone();
        self.current_path = path.to_path_buf();
        self.selection.clear();
        
        // Create new window
        let window_id = self.next_window_id;
        self.next_window_id += 1;
        
        let window = FileManagerWindow {
            id: window_id,
            path: self.current_path.clone(),
            view_mode: self.view_mode,
            selection: Vec::new(),
            visible: true,
            dimensions: WindowDimensions {
                x: 100,
                y: 100,
                width: 800,
                height: 600,
            },
        };
        
        self.windows.insert(window_id, window);
        self.state = FileManagerState::Full;
        
        log::info!("Opened file manager at: {}", path.display());
        Ok(())
    }
    
    /// Navigate to path
    pub fn navigate_to(&mut self, path: &Path) -> Result<(), BridgeError> {
        if !path.exists() {
            return Err(BridgeError::IoError(format!("Path does not exist: {}", path.display())));
        }
        
        let _old_path = self.current_path.clone();
        self.current_path = path.to_path_buf();
        self.selection.clear();
        
        log::info!("Navigated to: {}", path.display());
        Ok(())
    }
    
    /// Navigate up one directory
    pub fn navigate_up(&mut self) -> Result<(), BridgeError> {
        let current_path = self.current_path.clone();
        if let Some(parent) = current_path.parent() {
            self.navigate_to(parent)
        } else {
            Err(BridgeError::IoError("Cannot navigate up from root".to_string()))
        }
    }
    
    /// Navigate to home directory
    pub fn navigate_home(&mut self) -> Result<(), BridgeError> {
        let home = std::env::home_dir().ok_or_else(|| BridgeError::IoError("No home directory".to_string()))?;
        self.navigate_to(&home)
    }
    
    /// Set view mode
    pub fn set_view_mode(&mut self, mode: ViewMode) {
        self.view_mode = mode;
        
        // Update all windows
        for window in self.windows.values_mut() {
            window.view_mode = mode;
        }
        
        log::debug!("View mode changed to: {:?}", mode);
    }
    
    /// Select files/directories
    pub fn select(&mut self, paths: Vec<PathBuf>) {
        self.selection = paths;
    }
    
    /// Add to selection
    pub fn add_to_selection(&mut self, path: PathBuf) {
        if !self.selection.contains(&path) {
            self.selection.push(path);
        }
    }
    
    /// Clear selection
    pub fn clear_selection(&mut self) {
        self.selection.clear();
    }
    
    /// Copy selected files
    pub fn copy_selection(&mut self, destination: &Path) -> Result<(), BridgeError> {
        if self.selection.is_empty() {
            return Err(BridgeError::IoError("No files selected".to_string()));
        }
        
        // In a real implementation, this would perform the actual copy
        // For now, we just log the operation
        log::info!("Copying {} files to: {}", self.selection.len(), destination.display());
        
        // Clear selection after operation
        self.selection.clear();
        Ok(())
    }
    
    /// Move selected files
    pub fn move_selection(&mut self, destination: &Path) -> Result<(), BridgeError> {
        if self.selection.is_empty() {
            return Err(BridgeError::IoError("No files selected".to_string()));
        }
        
        // In a real implementation, this would perform the actual move
        log::info!("Moving {} files to: {}", self.selection.len(), destination.display());
        
        self.selection.clear();
        Ok(())
    }
    
    /// Delete selected files
    pub fn delete_selection(&mut self) -> Result<(), BridgeError> {
        if self.selection.is_empty() {
            return Err(BridgeError::IoError("No files selected".to_string()));
        }
        
        // In a real implementation, this would perform the actual deletion
        log::info!("Deleting {} files", self.selection.len());
        
        self.selection.clear();
        Ok(())
    }
    
    /// Create new directory
    pub fn create_directory(&mut self, name: &str) -> Result<(), BridgeError> {
        let path = self.current_path.join(name);
        
        // In a real implementation, this would create the directory
        log::info!("Creating directory: {}", path.display());
        
        Ok(())
    }
    
    /// Rename file/directory
    pub fn rename(&mut self, old_path: &Path, new_name: &str) -> Result<(), BridgeError> {
        let new_path = old_path.parent()
            .ok_or_else(|| BridgeError::IoError("Cannot determine parent directory".to_string()))?
            .join(new_name);
        
        // In a real implementation, this would perform the rename
        log::info!("Renaming {} to {}", old_path.display(), new_path.display());
        
        Ok(())
    }
    
    /// Get current directory contents
    pub fn get_directory_contents(&self) -> Result<Vec<FileEntry>, BridgeError> {
        let mut entries = Vec::new();
        
        // Read directory contents
        if let Ok(dir_entries) = std::fs::read_dir(&self.current_path) {
            for entry in dir_entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    let metadata = entry.metadata().unwrap_or_else(|_| {
                        // Create dummy metadata for error cases
                        std::fs::metadata("/dev/null").unwrap()
                    });
                    
                    let file_entry = FileEntry {
                        path: path.clone(),
                        name: path.file_name().unwrap_or_default().to_string_lossy().to_string(),
                        is_directory: metadata.is_dir(),
                        size: metadata.len(),
                        modified: metadata.modified().ok(),
                        permissions: FilePermissions::from_metadata(&metadata),
                    };
                    
                    entries.push(file_entry);
                }
            }
        }
        
        // Sort entries (directories first, then alphabetically)
        entries.sort_by(|a, b| {
            match (a.is_directory, b.is_directory) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            }
        });
        
        Ok(entries)
    }
    
    /// Show file manager
    pub fn show(&mut self) {
        self.state = FileManagerState::Full;
        for window in self.windows.values_mut() {
            window.visible = true;
        }
    }
    
    /// Hide file manager
    pub fn hide(&mut self) {
        self.state = FileManagerState::Hidden;
        for window in self.windows.values_mut() {
            window.visible = false;
        }
    }
    
    /// Show as overlay
    pub fn show_overlay(&mut self) {
        self.state = FileManagerState::Overlay;
        for window in self.windows.values_mut() {
            window.visible = true;
        }
    }
    
    /// Close window
    pub fn close_window(&mut self, window_id: u64) -> Result<(), BridgeError> {
        if self.windows.remove(&window_id).is_some() {
            log::info!("Closed file manager window: {}", window_id);
            Ok(())
        } else {
            Err(BridgeError::IoError(format!("Window not found: {}", window_id)))
        }
    }
    
    /// Update file manager
    pub fn update(&mut self, engine: &mut GraphEngine, node_manager: &mut NodeManager) -> Result<Vec<BridgeEvent>, BridgeError> {
        let events = Vec::new();
        
        // Update graph nodes for current directory
        if self.state != FileManagerState::Hidden {
            self.update_graph_nodes(engine, node_manager)?;
        }
        
        Ok(events)
    }
    
    /// Scan directory and create/update graph nodes
    fn scan_directory(&mut self, path: &Path, _engine: &mut GraphEngine, node_manager: &mut NodeManager) -> Result<(), BridgeError> {
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    
                    // Create file node if it doesn't exist
                    if let Some(_file_name) = path.file_name() {
                        let node_id = node_manager.create_file(
                            path.clone(),
                        );
                        
                        log::debug!("Created/updated file node: {} -> {:?}", path.display(), node_id);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Update graph nodes for current state
    fn update_graph_nodes(&mut self, engine: &mut GraphEngine, node_manager: &mut NodeManager) -> Result<(), BridgeError> {
        // This would update the graph representation based on file manager state
        // For now, we just ensure the current directory is represented
        self.scan_directory(&self.current_path.clone(), engine, node_manager)?;
        
        Ok(())
    }
    
    /// Get current path
    pub fn current_path(&self) -> &Path {
        &self.current_path
    }
    
    /// Get current selection
    pub fn selection(&self) -> &[PathBuf] {
        &self.selection
    }
    
    /// Get view mode
    pub fn view_mode(&self) -> ViewMode {
        self.view_mode
    }
    
    /// Get state
    pub fn state(&self) -> FileManagerState {
        self.state
    }
    
    /// Get windows
    pub fn windows(&self) -> &HashMap<u64, FileManagerWindow> {
        &self.windows
    }
}

/// File entry information
#[derive(Debug, Clone)]
pub struct FileEntry {
    /// Full path
    pub path: PathBuf,
    /// File name
    pub name: String,
    /// Is directory
    pub is_directory: bool,
    /// File size in bytes
    pub size: u64,
    /// Last modified time
    pub modified: Option<std::time::SystemTime>,
    /// File permissions
    pub permissions: FilePermissions,
}

/// File permissions
#[derive(Debug, Clone)]
pub struct FilePermissions {
    /// Readable
    pub readable: bool,
    /// Writable
    pub writable: bool,
    /// Executable
    pub executable: bool,
}

impl FilePermissions {
    /// Create from metadata
    fn from_metadata(metadata: &std::fs::Metadata) -> Self {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = metadata.permissions().mode();
            Self {
                readable: mode & 0o400 != 0,
                writable: mode & 0o200 != 0,
                executable: mode & 0o100 != 0,
            }
        }
        
        #[cfg(not(unix))]
        {
            Self {
                readable: true,
                writable: !metadata.permissions().readonly(),
                executable: false,
            }
        }
    }
}

impl Default for FileManagerBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for WindowDimensions {
    fn default() -> Self {
        Self {
            x: 100,
            y: 100,
            width: 800,
            height: 600,
        }
    }
}