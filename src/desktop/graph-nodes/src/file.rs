//! File node implementation

use crate::{GraphNode, BaseNode, NodeVisualData, NodeAction, NodeActionResult, NodeActionType, NodeError, NodeExportData};
use horizonos_graph_engine::{SceneNode, NodeType, NodeMetadata, FileType, SceneId, Position, Vec3};
use serde::{Serialize, Deserialize};
use std::path::{Path, PathBuf};
use std::fs;

/// Represents a file or directory in the graph desktop
#[derive(Debug, Clone)]
pub struct FileNode {
    base: BaseNode,
    file_data: FileData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileData {
    pub path: PathBuf,
    pub file_type: FileType,
    pub size: u64,
    pub permissions: u32,
    pub last_modified: chrono::DateTime<chrono::Utc>,
    pub last_accessed: chrono::DateTime<chrono::Utc>,
    pub mime_type: Option<String>,
    pub encoding: Option<String>,
    pub is_symlink: bool,
    pub target_path: Option<PathBuf>, // For symlinks
}

impl FileNode {
    pub fn new(id: SceneId, path: PathBuf) -> Result<Self, NodeError> {
        let metadata = fs::metadata(&path)?;
        
        let file_type = if metadata.is_dir() {
            FileType::Directory
        } else {
            Self::detect_file_type(&path)
        };
        
        let mut base = BaseNode::new(id);
        base.metadata.tags.push("file".to_string());
        base.metadata.description = Some(format!("File: {}", path.display()));
        
        let file_data = FileData {
            path: path.clone(),
            file_type: file_type.clone(),
            size: metadata.len(),
            permissions: Self::get_permissions(&metadata),
            last_modified: Self::system_time_to_datetime(metadata.modified().unwrap_or(std::time::SystemTime::now())),
            last_accessed: Self::system_time_to_datetime(metadata.accessed().unwrap_or(std::time::SystemTime::now())),
            mime_type: Self::detect_mime_type(&path),
            encoding: None,
            is_symlink: metadata.file_type().is_symlink(),
            target_path: if metadata.file_type().is_symlink() {
                fs::read_link(&path).ok()
            } else {
                None
            },
        };
        
        let mut node = FileNode {
            base,
            file_data,
        };
        
        node.update_visual_by_type();
        Ok(node)
    }
    
    fn detect_file_type(path: &Path) -> FileType {
        if let Some(extension) = path.extension() {
            match extension.to_string_lossy().to_lowercase().as_str() {
                "jpg" | "jpeg" | "png" | "gif" | "bmp" | "svg" | "webp" => FileType::Image,
                "mp4" | "avi" | "mkv" | "mov" | "wmv" | "flv" | "webm" => FileType::Video,
                "mp3" | "wav" | "flac" | "ogg" | "aac" | "m4a" => FileType::Audio,
                "pdf" | "doc" | "docx" | "odt" | "txt" | "rtf" | "md" => FileType::Document,
                "rs" | "c" | "cpp" | "h" | "py" | "js" | "ts" | "html" | "css" | "java" | "go" => FileType::Code,
                "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" => FileType::Archive,
                _ => FileType::RegularFile,
            }
        } else {
            FileType::RegularFile
        }
    }
    
    fn detect_mime_type(path: &Path) -> Option<String> {
        // Simple MIME type detection based on extension
        if let Some(extension) = path.extension() {
            match extension.to_string_lossy().to_lowercase().as_str() {
                "txt" => Some("text/plain".to_string()),
                "html" => Some("text/html".to_string()),
                "css" => Some("text/css".to_string()),
                "js" => Some("text/javascript".to_string()),
                "json" => Some("application/json".to_string()),
                "pdf" => Some("application/pdf".to_string()),
                "jpg" | "jpeg" => Some("image/jpeg".to_string()),
                "png" => Some("image/png".to_string()),
                "gif" => Some("image/gif".to_string()),
                "mp4" => Some("video/mp4".to_string()),
                "mp3" => Some("audio/mpeg".to_string()),
                "zip" => Some("application/zip".to_string()),
                _ => None,
            }
        } else {
            None
        }
    }
    
    fn get_permissions(metadata: &fs::Metadata) -> u32 {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            metadata.permissions().mode()
        }
        #[cfg(not(unix))]
        {
            // Windows fallback
            if metadata.permissions().readonly() { 0o444 } else { 0o644 }
        }
    }
    
    fn system_time_to_datetime(time: std::time::SystemTime) -> chrono::DateTime<chrono::Utc> {
        time.duration_since(std::time::UNIX_EPOCH)
            .map(|duration| {
                chrono::DateTime::from_timestamp(duration.as_secs() as i64, duration.subsec_nanos())
                    .unwrap_or_else(chrono::Utc::now)
            })
            .unwrap_or_else(|_| chrono::Utc::now())
    }
    
    fn update_visual_by_type(&mut self) {
        let (color, radius) = match self.file_data.file_type {
            FileType::Directory => ([1.0, 0.8, 0.3, 1.0], 1.5), // Yellow folder
            FileType::Image => ([0.8, 0.2, 0.8, 1.0], 1.0),     // Purple
            FileType::Video => ([0.9, 0.4, 0.1, 1.0], 1.2),     // Orange
            FileType::Audio => ([0.1, 0.9, 0.4, 1.0], 1.0),     // Green
            FileType::Document => ([0.2, 0.5, 0.9, 1.0], 1.0),  // Blue
            FileType::Code => ([0.9, 0.1, 0.1, 1.0], 1.0),      // Red
            FileType::Archive => ([0.6, 0.4, 0.2, 1.0], 1.1),   // Brown
            FileType::RegularFile => ([0.7, 0.7, 0.7, 1.0], 0.8), // Gray
        };
        
        self.base.visual_data.color = color;
        self.base.visual_data.radius = radius;
        
        // Set icon based on type
        let icon = match self.file_data.file_type {
            FileType::Directory => "📁",
            FileType::Image => "🖼️",
            FileType::Video => "🎥",
            FileType::Audio => "🎵",
            FileType::Document => "📄",
            FileType::Code => "💻",
            FileType::Archive => "📦",
            FileType::RegularFile => "📄",
        };
        
        self.base.visual_data.icon = Some(icon.to_string());
    }
    
    pub fn read_content(&self) -> Result<Vec<u8>, NodeError> {
        if self.file_data.file_type == FileType::Directory {
            return Err(NodeError::InvalidAction { 
                action: NodeAction::Custom { 
                    action_type: "read".to_string(), 
                    parameters: std::collections::HashMap::new() 
                } 
            });
        }
        
        fs::read(&self.file_data.path).map_err(NodeError::IoError)
    }
    
    pub fn read_text(&self) -> Result<String, NodeError> {
        let content = self.read_content()?;
        String::from_utf8(content)
            .map_err(|e| NodeError::SystemError { 
                message: format!("Invalid UTF-8: {}", e) 
            })
    }
    
    pub fn list_directory(&self) -> Result<Vec<PathBuf>, NodeError> {
        if self.file_data.file_type != FileType::Directory {
            return Err(NodeError::InvalidAction { 
                action: NodeAction::Custom { 
                    action_type: "list".to_string(), 
                    parameters: std::collections::HashMap::new() 
                } 
            });
        }
        
        let mut entries = Vec::new();
        for entry in fs::read_dir(&self.file_data.path)? {
            let entry = entry?;
            entries.push(entry.path());
        }
        entries.sort();
        Ok(entries)
    }
    
    pub fn open_with_default_app(&self) -> Result<(), NodeError> {
        let path_str = self.file_data.path.to_string_lossy();
        
        #[cfg(target_os = "linux")]
        {
            std::process::Command::new("xdg-open")
                .arg(&*path_str)
                .spawn()
                .map_err(|e| NodeError::SystemError { 
                    message: format!("Failed to open file: {}", e) 
                })?;
        }
        
        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("open")
                .arg(&*path_str)
                .spawn()
                .map_err(|e| NodeError::SystemError { 
                    message: format!("Failed to open file: {}", e) 
                })?;
        }
        
        #[cfg(target_os = "windows")]
        {
            std::process::Command::new("start")
                .arg(&*path_str)
                .spawn()
                .map_err(|e| NodeError::SystemError { 
                    message: format!("Failed to open file: {}", e) 
                })?;
        }
        
        Ok(())
    }
    
    pub fn file_size_human(&self) -> String {
        let size = self.file_data.size;
        if size < 1024 {
            format!("{} B", size)
        } else if size < 1024 * 1024 {
            format!("{:.1} KB", size as f64 / 1024.0)
        } else if size < 1024 * 1024 * 1024 {
            format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
        } else {
            format!("{:.1} GB", size as f64 / (1024.0 * 1024.0 * 1024.0))
        }
    }
    
    pub fn path(&self) -> &Path {
        &self.file_data.path
    }
    
    pub fn file_name(&self) -> Option<&str> {
        self.file_data.path.file_name()
            .and_then(|name| name.to_str())
    }
    
    pub fn parent_dir(&self) -> Option<&Path> {
        self.file_data.path.parent()
    }
}

impl GraphNode for FileNode {
    fn id(&self) -> SceneId {
        self.base.id
    }
    
    fn display_name(&self) -> String {
        self.file_name()
            .unwrap_or("Unknown")
            .to_string()
    }
    
    fn description(&self) -> Option<String> {
        let type_str = match self.file_data.file_type {
            FileType::Directory => "Directory",
            FileType::Image => "Image File",
            FileType::Video => "Video File", 
            FileType::Audio => "Audio File",
            FileType::Document => "Document",
            FileType::Code => "Source Code",
            FileType::Archive => "Archive",
            FileType::RegularFile => "File",
        };
        
        Some(format!(
            "{} | {} | Modified: {}",
            type_str,
            self.file_size_human(),
            self.file_data.last_modified.format("%Y-%m-%d %H:%M")
        ))
    }
    
    fn node_type(&self) -> NodeType {
        NodeType::File {
            path: self.file_data.path.to_string_lossy().to_string(),
            file_type: self.file_data.file_type.clone(),
        }
    }
    
    fn metadata(&self) -> NodeMetadata {
        self.base.metadata.clone()
    }
    
    fn visual_data(&self) -> NodeVisualData {
        let mut visual = self.base.visual_data.clone();
        
        // Add size indicator for large files
        if self.file_data.size > 100 * 1024 * 1024 { // > 100MB
            visual.badge = Some("!".to_string());
        }
        
        // Add symlink indicator
        if self.file_data.is_symlink {
            visual.badge = Some("→".to_string());
        }
        
        visual
    }
    
    fn update(&mut self, _delta_time: f32) -> Result<(), NodeError> {
        // Check if file still exists and update metadata
        if let Ok(metadata) = fs::metadata(&self.file_data.path) {
            let new_modified = Self::system_time_to_datetime(
                metadata.modified().unwrap_or(std::time::SystemTime::now())
            );
            
            if new_modified != self.file_data.last_modified {
                self.file_data.last_modified = new_modified;
                self.file_data.size = metadata.len();
                self.base.update_timestamp();
                
                // Update visual if size changed significantly
                self.update_visual_by_type();
            }
        } else {
            // File was deleted
            self.base.visual_data.visible = false;
        }
        Ok(())
    }
    
    fn handle_action(&mut self, action: NodeAction) -> Result<NodeActionResult, NodeError> {
        match action {
            NodeAction::Open => {
                self.open_with_default_app()?;
                Ok(NodeActionResult::Success { 
                    message: Some(format!("Opened {}", self.display_name())) 
                })
            }
            NodeAction::Delete => {
                let prompt = if self.file_data.file_type == FileType::Directory {
                    format!("Delete directory {} and all its contents?", self.display_name())
                } else {
                    format!("Delete file {}?", self.display_name())
                };
                Ok(NodeActionResult::ConfirmationRequired { prompt })
            }
            NodeAction::Copy => {
                // Copy file path to clipboard (if available)
                Ok(NodeActionResult::Success { 
                    message: Some(format!("Copied path: {}", self.file_data.path.display())) 
                })
            }
            NodeAction::Custom { ref action_type, .. } => {
                match action_type.as_str() {
                    "show_in_folder" => {
                        if let Some(parent) = self.parent_dir() {
                            #[cfg(target_os = "linux")]
                            {
                                std::process::Command::new("xdg-open")
                                    .arg(parent)
                                    .spawn()
                                    .map_err(|e| NodeError::SystemError { 
                                        message: format!("Failed to open folder: {}", e) 
                                    })?;
                            }
                            Ok(NodeActionResult::Success { 
                                message: Some("Opened parent folder".to_string()) 
                            })
                        } else {
                            Ok(NodeActionResult::Error { 
                                error: "No parent directory".to_string() 
                            })
                        }
                    }
                    "properties" => {
                        Ok(NodeActionResult::Success { 
                            message: Some(format!("File properties: {:#?}", self.file_data)) 
                        })
                    }
                    _ => Err(NodeError::InvalidAction { action })
                }
            }
            _ => Err(NodeError::InvalidAction { action })
        }
    }
    
    fn available_actions(&self) -> Vec<NodeActionType> {
        let mut actions = vec![
            NodeActionType::Open,
            NodeActionType::Copy,
            NodeActionType::Delete,
            NodeActionType::Move,
            NodeActionType::Connect,
            NodeActionType::Custom("show_in_folder".to_string()),
            NodeActionType::Custom("properties".to_string()),
        ];
        
        if self.file_data.file_type != FileType::Directory {
            actions.push(NodeActionType::Edit);
        }
        
        actions
    }
    
    fn export_data(&self) -> Result<NodeExportData, NodeError> {
        Ok(NodeExportData {
            node_type: "file".to_string(),
            display_name: self.display_name(),
            description: self.description(),
            visual_data: self.visual_data(),
            metadata: self.base.metadata.clone(),
            type_specific_data: serde_json::to_value(&self.file_data)?,
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
            node_type: NodeType::File {
                path: self.file_data.path.to_string_lossy().to_string(),
                file_type: self.file_data.file_type.clone(),
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
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_file_node_creation() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        
        {
            let mut file = File::create(&file_path).unwrap();
            writeln!(file, "Hello, world!").unwrap();
        }
        
        let file_node = FileNode::new(1, file_path.clone()).unwrap();
        assert_eq!(file_node.id(), 1);
        assert_eq!(file_node.display_name(), "test.txt");
        assert_eq!(file_node.file_data.file_type, FileType::Document);
        assert!(file_node.file_data.size > 0);
    }

    #[test]
    fn test_directory_node() {
        let dir = tempdir().unwrap();
        let dir_node = FileNode::new(2, dir.path().to_path_buf()).unwrap();
        
        assert_eq!(dir_node.file_data.file_type, FileType::Directory);
        assert!(dir_node.list_directory().is_ok());
    }

    #[test]
    fn test_file_type_detection() {
        assert_eq!(FileNode::detect_file_type(Path::new("test.jpg")), FileType::Image);
        assert_eq!(FileNode::detect_file_type(Path::new("video.mp4")), FileType::Video);
        assert_eq!(FileNode::detect_file_type(Path::new("code.rs")), FileType::Code);
        assert_eq!(FileNode::detect_file_type(Path::new("doc.pdf")), FileType::Document);
    }
}