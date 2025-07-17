//! Icon loading and management system

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use anyhow::{Result, Context};
use image::{DynamicImage, ImageFormat};
use tokio::fs;
use log::{debug, warn};
use dirs;

/// Icon sizes supported by the system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IconSize {
    Small = 16,
    Medium = 32,
    Large = 64,
    XLarge = 128,
    XXLarge = 256,
}

impl IconSize {
    /// Get all standard sizes
    pub fn all() -> &'static [IconSize] {
        &[
            IconSize::Small,
            IconSize::Medium,
            IconSize::Large,
            IconSize::XLarge,
            IconSize::XXLarge,
        ]
    }
    
    /// Get the pixel size
    pub fn pixels(&self) -> u32 {
        *self as u32
    }
}

/// Icon theme following freedesktop.org specification
#[derive(Debug, Clone)]
pub struct IconTheme {
    /// Theme name
    pub name: String,
    /// Theme directories
    pub directories: Vec<PathBuf>,
    /// Inherits from these themes
    pub inherits: Vec<String>,
}

impl IconTheme {
    /// Load system icon themes
    pub fn load_system_themes() -> Vec<Self> {
        let mut themes = Vec::new();
        
        // Standard icon theme directories
        let mut theme_dirs = vec![
            "/usr/share/icons".to_string(),
            "/usr/local/share/icons".to_string(),
        ];
        
        if let Some(home) = dirs::home_dir() {
            theme_dirs.push(home.join(".local/share/icons").to_string_lossy().to_string());
        }
        
        for dir in theme_dirs {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                        let theme_path = entry.path();
                        let theme_name = theme_path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or_default()
                            .to_string();
                        
                        themes.push(IconTheme {
                            name: theme_name,
                            directories: vec![theme_path],
                            inherits: vec!["hicolor".to_string()],
                        });
                    }
                }
            }
        }
        
        themes
    }
}

/// Icon loader with caching
pub struct IconLoader {
    /// Icon cache: (icon_name, size) -> image data
    cache: Arc<RwLock<HashMap<(String, IconSize), Arc<DynamicImage>>>>,
    /// Current icon theme
    theme: IconTheme,
    /// Fallback icon
    fallback_icon: Option<Arc<DynamicImage>>,
}

impl IconLoader {
    /// Create new icon loader
    pub fn new(theme_name: Option<&str>) -> Self {
        let themes = IconTheme::load_system_themes();
        let theme = theme_name
            .and_then(|name| themes.into_iter().find(|t| t.name == name))
            .unwrap_or_else(|| IconTheme {
                name: "hicolor".to_string(),
                directories: vec![PathBuf::from("/usr/share/icons/hicolor")],
                inherits: vec![],
            });
        
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            theme,
            fallback_icon: None,
        }
    }
    
    /// Load an icon by name
    pub async fn load_icon(&self, icon_name: &str, size: IconSize) -> Result<Arc<DynamicImage>> {
        // Check cache first
        let cache_key = (icon_name.to_string(), size);
        if let Some(cached) = self.cache.read().unwrap().get(&cache_key) {
            return Ok(cached.clone());
        }
        
        // Search for icon in theme directories
        let icon = self.find_and_load_icon(icon_name, size).await?;
        
        // Cache the result
        let icon_arc = Arc::new(icon);
        self.cache.write().unwrap().insert(cache_key, icon_arc.clone());
        
        Ok(icon_arc)
    }
    
    /// Find and load icon from theme directories
    async fn find_and_load_icon(&self, icon_name: &str, size: IconSize) -> Result<DynamicImage> {
        let size_str = size.pixels().to_string();
        let extensions = ["svg", "png", "xpm"];
        
        // Search in theme directories
        for dir in &self.theme.directories {
            // Try size-specific directories first
            let size_dirs = vec![
                dir.join(&size_str).join(&size_str),
                dir.join(format!("{}x{}", size_str, size_str)),
                dir.join("scalable"),
            ];
            
            for size_dir in size_dirs {
                for ext in &extensions {
                    let icon_path = size_dir.join(format!("{}.{}", icon_name, ext));
                    if icon_path.exists() {
                        return self.load_icon_file(&icon_path, size).await;
                    }
                }
            }
        }
        
        // Fallback to generic icon
        if let Some(fallback) = &self.fallback_icon {
            return Ok((**fallback).clone());
        }
        
        Err(anyhow::anyhow!("Icon '{}' not found", icon_name))
    }
    
    /// Load icon file and resize if needed
    async fn load_icon_file(&self, path: &Path, target_size: IconSize) -> Result<DynamicImage> {
        let data = fs::read(path).await
            .context("Failed to read icon file")?;
        
        // Handle SVG files specially
        if path.extension().and_then(|e| e.to_str()) == Some("svg") {
            return self.load_svg(&data, target_size);
        }
        
        // Load other formats with image crate
        let img = image::load_from_memory(&data)
            .context("Failed to decode icon")?;
        
        // Resize if needed
        let target_pixels = target_size.pixels();
        if img.width() != target_pixels || img.height() != target_pixels {
            Ok(img.resize_exact(
                target_pixels,
                target_pixels,
                image::imageops::FilterType::Lanczos3
            ))
        } else {
            Ok(img)
        }
    }
    
    /// Load SVG file
    fn load_svg(&self, data: &[u8], size: IconSize) -> Result<DynamicImage> {
        // TODO: Implement proper SVG rendering when resvg API stabilizes
        // For now, return a placeholder image
        let target_size = size.pixels();
        let img = image::RgbaImage::new(target_size, target_size);
        Ok(DynamicImage::ImageRgba8(img))
    }
    
    /// Set fallback icon
    pub fn set_fallback_icon(&mut self, icon: DynamicImage) {
        self.fallback_icon = Some(Arc::new(icon));
    }
    
    /// Clear icon cache
    pub fn clear_cache(&self) {
        self.cache.write().unwrap().clear();
    }
    
    /// Get cache statistics
    pub fn cache_stats(&self) -> CacheStats {
        let cache = self.cache.read().unwrap();
        CacheStats {
            total_icons: cache.len(),
            memory_usage: cache.values()
                .map(|img| (img.width() * img.height() * 4) as usize)
                .sum(),
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Total number of cached icons
    pub total_icons: usize,
    /// Approximate memory usage in bytes
    pub memory_usage: usize,
}

/// Application icon extractor
pub struct AppIconExtractor;

impl AppIconExtractor {
    /// Extract icon from .desktop file
    pub async fn extract_from_desktop_file(desktop_path: &Path) -> Result<String> {
        let content = fs::read_to_string(desktop_path).await
            .context("Failed to read .desktop file")?;
        
        // Parse .desktop file
        for line in content.lines() {
            if let Some(icon) = line.strip_prefix("Icon=") {
                return Ok(icon.trim().to_string());
            }
        }
        
        Err(anyhow::anyhow!("No Icon field found in .desktop file"))
    }
    
    /// Find .desktop file for application
    pub async fn find_desktop_file(app_name: &str) -> Result<PathBuf> {
        let mut desktop_dirs = vec![
            "/usr/share/applications".to_string(),
            "/usr/local/share/applications".to_string(),
        ];
        
        if let Some(home) = dirs::home_dir() {
            desktop_dirs.push(home.join(".local/share/applications").to_string_lossy().to_string());
        }
        
        for dir in desktop_dirs {
            let desktop_path = PathBuf::from(dir).join(format!("{}.desktop", app_name));
            if desktop_path.exists() {
                return Ok(desktop_path);
            }
        }
        
        Err(anyhow::anyhow!("Desktop file not found for application: {}", app_name))
    }
}

/// File type icon mapper
pub struct FileTypeIconMapper {
    /// MIME type to icon name mapping
    mime_icons: HashMap<String, String>,
    /// Extension to icon name mapping
    extension_icons: HashMap<String, String>,
}

impl FileTypeIconMapper {
    /// Create new file type icon mapper
    pub fn new() -> Self {
        let mut mapper = Self {
            mime_icons: HashMap::new(),
            extension_icons: HashMap::new(),
        };
        
        // Initialize default mappings
        mapper.init_default_mappings();
        mapper
    }
    
    /// Initialize default MIME type mappings
    fn init_default_mappings(&mut self) {
        // Document types
        self.mime_icons.insert("application/pdf".to_string(), "application-pdf".to_string());
        self.mime_icons.insert("application/msword".to_string(), "application-msword".to_string());
        self.mime_icons.insert("text/plain".to_string(), "text-plain".to_string());
        self.mime_icons.insert("text/html".to_string(), "text-html".to_string());
        
        // Image types
        self.mime_icons.insert("image/png".to_string(), "image-png".to_string());
        self.mime_icons.insert("image/jpeg".to_string(), "image-jpeg".to_string());
        self.mime_icons.insert("image/svg+xml".to_string(), "image-svg+xml".to_string());
        
        // Audio/Video types
        self.mime_icons.insert("audio/mpeg".to_string(), "audio-x-generic".to_string());
        self.mime_icons.insert("video/mp4".to_string(), "video-x-generic".to_string());
        
        // Archive types
        self.mime_icons.insert("application/zip".to_string(), "package-x-generic".to_string());
        self.mime_icons.insert("application/x-tar".to_string(), "package-x-generic".to_string());
        
        // Extension fallbacks
        self.extension_icons.insert("rs".to_string(), "text-x-rust".to_string());
        self.extension_icons.insert("py".to_string(), "text-x-python".to_string());
        self.extension_icons.insert("js".to_string(), "text-x-javascript".to_string());
        self.extension_icons.insert("cpp".to_string(), "text-x-c++".to_string());
        self.extension_icons.insert("c".to_string(), "text-x-c".to_string());
        self.extension_icons.insert("h".to_string(), "text-x-c-header".to_string());
    }
    
    /// Get icon name for MIME type
    pub fn icon_for_mime_type(&self, mime_type: &str) -> Option<&str> {
        self.mime_icons.get(mime_type).map(|s| s.as_str())
    }
    
    /// Get icon name for file extension
    pub fn icon_for_extension(&self, extension: &str) -> Option<&str> {
        self.extension_icons.get(extension).map(|s| s.as_str())
    }
    
    /// Get icon name for file path
    pub fn icon_for_file(&self, path: &Path) -> &str {
        // Try to get MIME type first
        if let Some(mime) = tree_magic_mini::from_filepath(path) {
            if let Some(icon) = self.icon_for_mime_type(mime) {
                return icon;
            }
        }
        
        // Fall back to extension
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if let Some(icon) = self.icon_for_extension(ext) {
                return icon;
            }
        }
        
        // Default icon
        "text-x-generic"
    }
    
    /// Add custom MIME type mapping
    pub fn add_mime_mapping(&mut self, mime_type: String, icon_name: String) {
        self.mime_icons.insert(mime_type, icon_name);
    }
    
    /// Add custom extension mapping
    pub fn add_extension_mapping(&mut self, extension: String, icon_name: String) {
        self.extension_icons.insert(extension, icon_name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_icon_sizes() {
        assert_eq!(IconSize::Small.pixels(), 16);
        assert_eq!(IconSize::Medium.pixels(), 32);
        assert_eq!(IconSize::Large.pixels(), 64);
        assert_eq!(IconSize::XLarge.pixels(), 128);
        assert_eq!(IconSize::XXLarge.pixels(), 256);
    }
    
    #[test]
    fn test_file_type_mapper() {
        let mapper = FileTypeIconMapper::new();
        
        assert_eq!(mapper.icon_for_mime_type("application/pdf"), Some("application-pdf"));
        assert_eq!(mapper.icon_for_extension("rs"), Some("text-x-rust"));
        assert_eq!(mapper.icon_for_file(Path::new("test.py")), "text-x-python");
    }
}