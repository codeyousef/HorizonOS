//! Thumbnail generation and caching system

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use anyhow::Result;
use image::{DynamicImage, ImageFormat};
use tokio::fs;
use tokio::process::Command;
use sha2::{Sha256, Digest as Sha2Digest};
use log::debug;

/// Thumbnail sizes following XDG specification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ThumbnailSize {
    Normal = 128,
    Large = 256,
    XLarge = 512,
}

impl ThumbnailSize {
    /// Get pixel size
    pub fn pixels(&self) -> u32 {
        *self as u32
    }
    
    /// Get directory name for XDG cache
    pub fn dir_name(&self) -> &'static str {
        match self {
            ThumbnailSize::Normal => "normal",
            ThumbnailSize::Large => "large",
            ThumbnailSize::XLarge => "x-large",
        }
    }
}

/// Thumbnail generator with XDG-compliant caching
pub struct ThumbnailGenerator {
    /// Cache directory
    cache_dir: PathBuf,
    /// Memory cache for quick access
    memory_cache: Arc<RwLock<HashMap<(PathBuf, ThumbnailSize), Arc<DynamicImage>>>>,
    /// Maximum memory cache size in bytes
    max_memory_cache: usize,
    /// Current memory usage
    current_memory_usage: Arc<RwLock<usize>>,
}

impl ThumbnailGenerator {
    /// Create new thumbnail generator
    pub fn new() -> Result<Self> {
        let cache_dir = Self::get_cache_dir()?;
        
        Ok(Self {
            cache_dir,
            memory_cache: Arc::new(RwLock::new(HashMap::new())),
            max_memory_cache: 100 * 1024 * 1024, // 100MB
            current_memory_usage: Arc::new(RwLock::new(0)),
        })
    }
    
    /// Get XDG-compliant cache directory
    fn get_cache_dir() -> Result<PathBuf> {
        let cache_home = std::env::var("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
                PathBuf::from(home).join(".cache")
            });
        
        let thumbnail_dir = cache_home.join("thumbnails");
        
        // Create directories if they don't exist
        for size in &["normal", "large", "x-large"] {
            std::fs::create_dir_all(thumbnail_dir.join(size))?;
        }
        
        Ok(thumbnail_dir)
    }
    
    /// Generate or retrieve thumbnail for a file
    pub async fn get_thumbnail(
        &self,
        file_path: &Path,
        size: ThumbnailSize,
    ) -> Result<Arc<DynamicImage>> {
        // Check memory cache first
        let cache_key = (file_path.to_path_buf(), size);
        if let Some(cached) = self.memory_cache.read().unwrap().get(&cache_key) {
            return Ok(cached.clone());
        }
        
        // Check disk cache
        let thumbnail_path = self.get_thumbnail_path(file_path, size)?;
        if thumbnail_path.exists() {
            if self.is_thumbnail_valid(file_path, &thumbnail_path).await? {
                let thumb = self.load_thumbnail(&thumbnail_path).await?;
                self.add_to_memory_cache(cache_key, thumb.clone()).await;
                return Ok(thumb);
            }
        }
        
        // Generate new thumbnail
        let thumb = self.generate_thumbnail(file_path, size).await?;
        
        // Save to disk cache
        self.save_thumbnail(&thumb, &thumbnail_path, file_path).await?;
        
        // Add to memory cache
        self.add_to_memory_cache(cache_key, thumb.clone()).await;
        
        Ok(thumb)
    }
    
    /// Get thumbnail file path using URI hash
    fn get_thumbnail_path(&self, file_path: &Path, size: ThumbnailSize) -> Result<PathBuf> {
        let uri = format!("file://{}", file_path.display());
        let hash = self.hash_uri(&uri);
        
        Ok(self.cache_dir
            .join(size.dir_name())
            .join(format!("{}.png", hash)))
    }
    
    /// Hash URI for thumbnail filename
    fn hash_uri(&self, uri: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(uri.as_bytes());
        format!("{:x}", hasher.finalize())
    }
    
    /// Check if cached thumbnail is still valid
    async fn is_thumbnail_valid(&self, file_path: &Path, thumb_path: &Path) -> Result<bool> {
        let file_meta = fs::metadata(file_path).await?;
        let thumb_meta = fs::metadata(thumb_path).await?;
        
        // Thumbnail is valid if it's newer than the source file
        Ok(thumb_meta.modified()? >= file_meta.modified()?)
    }
    
    /// Load thumbnail from disk
    async fn load_thumbnail(&self, path: &Path) -> Result<Arc<DynamicImage>> {
        let data = fs::read(path).await?;
        let img = image::load_from_memory(&data)?;
        Ok(Arc::new(img))
    }
    
    /// Generate thumbnail for file
    async fn generate_thumbnail(&self, file_path: &Path, size: ThumbnailSize) -> Result<Arc<DynamicImage>> {
        let mime_type = tree_magic_mini::from_filepath(file_path)
            .unwrap_or("application/octet-stream");
        
        match mime_type {
            // Image files
            t if t.starts_with("image/") => self.generate_image_thumbnail(file_path, size).await,
            
            // Video files
            t if t.starts_with("video/") => self.generate_video_thumbnail(file_path, size).await,
            
            // PDF files
            "application/pdf" => self.generate_pdf_thumbnail(file_path, size).await,
            
            // Text files
            t if t.starts_with("text/") => self.generate_text_thumbnail(file_path, size).await,
            
            // Default fallback
            _ => self.generate_generic_thumbnail(file_path, size).await,
        }
    }
    
    /// Generate thumbnail for image files
    async fn generate_image_thumbnail(&self, path: &Path, size: ThumbnailSize) -> Result<Arc<DynamicImage>> {
        let data = fs::read(path).await?;
        let img = image::load_from_memory(&data)?;
        
        // Resize to fit within thumbnail size while maintaining aspect ratio
        let thumb_size = size.pixels();
        let thumb = img.thumbnail(thumb_size, thumb_size);
        
        Ok(Arc::new(thumb))
    }
    
    /// Generate thumbnail for video files using ffmpeg
    async fn generate_video_thumbnail(&self, path: &Path, size: ThumbnailSize) -> Result<Arc<DynamicImage>> {
        let temp_path = std::env::temp_dir().join(format!("thumb_{}.png", uuid::Uuid::new_v4()));
        let thumb_size = size.pixels();
        
        // Use ffmpeg to extract frame at 1 second
        let output = Command::new("ffmpeg")
            .args(&[
                "-i", path.to_str().unwrap(),
                "-ss", "00:00:01",
                "-vframes", "1",
                "-vf", &format!("scale={}:{}", thumb_size, thumb_size),
                "-f", "image2",
                temp_path.to_str().unwrap(),
            ])
            .output()
            .await?;
        
        if !output.status.success() {
            return Err(anyhow::anyhow!("ffmpeg failed: {:?}", output.stderr));
        }
        
        // Load generated thumbnail
        let thumb_data = fs::read(&temp_path).await?;
        fs::remove_file(&temp_path).await?;
        
        let img = image::load_from_memory(&thumb_data)?;
        Ok(Arc::new(img))
    }
    
    /// Generate thumbnail for PDF files
    async fn generate_pdf_thumbnail(&self, path: &Path, size: ThumbnailSize) -> Result<Arc<DynamicImage>> {
        let temp_path = std::env::temp_dir().join(format!("thumb_{}.png", uuid::Uuid::new_v4()));
        let thumb_size = size.pixels();
        
        // Use pdftoppm to convert first page to image
        let output = Command::new("pdftoppm")
            .args(&[
                "-png",
                "-f", "1",
                "-l", "1",
                "-scale-to", &thumb_size.to_string(),
                "-singlefile",
                path.to_str().unwrap(),
                temp_path.to_str().unwrap().trim_end_matches(".png"),
            ])
            .output()
            .await?;
        
        if !output.status.success() {
            // Fallback to ImageMagick if available
            return self.generate_pdf_thumbnail_imagemagick(path, size).await;
        }
        
        // Load generated thumbnail
        let thumb_data = fs::read(&temp_path).await?;
        fs::remove_file(&temp_path).await?;
        
        let img = image::load_from_memory(&thumb_data)?;
        Ok(Arc::new(img))
    }
    
    /// Generate PDF thumbnail using ImageMagick as fallback
    async fn generate_pdf_thumbnail_imagemagick(&self, path: &Path, size: ThumbnailSize) -> Result<Arc<DynamicImage>> {
        let temp_path = std::env::temp_dir().join(format!("thumb_{}.png", uuid::Uuid::new_v4()));
        let thumb_size = size.pixels();
        
        let output = Command::new("convert")
            .args(&[
                &format!("{}[0]", path.display()), // First page only
                "-thumbnail", &format!("{}x{}", thumb_size, thumb_size),
                temp_path.to_str().unwrap(),
            ])
            .output()
            .await?;
        
        if !output.status.success() {
            return Err(anyhow::anyhow!("ImageMagick convert failed"));
        }
        
        let thumb_data = fs::read(&temp_path).await?;
        fs::remove_file(&temp_path).await?;
        
        let img = image::load_from_memory(&thumb_data)?;
        Ok(Arc::new(img))
    }
    
    /// Generate thumbnail for text files
    async fn generate_text_thumbnail(&self, path: &Path, size: ThumbnailSize) -> Result<Arc<DynamicImage>> {
        // TODO: Implement text rendering when imageproc API stabilizes
        
        // Read first few lines of text
        let content = fs::read_to_string(path).await?;
        let lines: Vec<&str> = content.lines().take(20).collect();
        
        // Create image
        let thumb_size = size.pixels();
        let mut img = image::RgbImage::new(thumb_size, thumb_size);
        
        // Fill with white background
        for pixel in img.pixels_mut() {
            *pixel = image::Rgb([255, 255, 255]);
        }
        
        // TODO: Draw text lines when font rendering is available
        
        Ok(Arc::new(DynamicImage::ImageRgb8(img)))
    }
    
    /// Generate generic thumbnail with file type icon
    async fn generate_generic_thumbnail(&self, path: &Path, size: ThumbnailSize) -> Result<Arc<DynamicImage>> {
        // Create a simple thumbnail with file extension
        let thumb_size = size.pixels();
        let mut img = image::RgbaImage::new(thumb_size, thumb_size);
        
        // Fill with light gray background
        for pixel in img.pixels_mut() {
            *pixel = image::Rgba([240, 240, 240, 255]);
        }
        
        // TODO: Draw file type icon or extension text
        
        Ok(Arc::new(DynamicImage::ImageRgba8(img)))
    }
    
    /// Save thumbnail to disk with metadata
    async fn save_thumbnail(
        &self,
        thumb: &Arc<DynamicImage>,
        thumb_path: &Path,
        source_path: &Path,
    ) -> Result<()> {
        // Create parent directory if needed
        if let Some(parent) = thumb_path.parent() {
            fs::create_dir_all(parent).await?;
        }
        
        // Save as PNG with metadata
        let mut buffer = std::io::Cursor::new(Vec::new());
        thumb.write_to(&mut buffer, ImageFormat::Png)?;
        
        // Write to file
        fs::write(thumb_path, buffer.into_inner()).await?;
        
        // TODO: Add PNG metadata chunks for thumbnail spec compliance
        
        Ok(())
    }
    
    /// Add thumbnail to memory cache with size limit
    async fn add_to_memory_cache(&self, key: (PathBuf, ThumbnailSize), thumb: Arc<DynamicImage>) {
        let size = (thumb.width() * thumb.height() * 4) as usize;
        
        let mut cache = self.memory_cache.write().unwrap();
        let mut usage = self.current_memory_usage.write().unwrap();
        
        // Evict old entries if needed
        while *usage + size > self.max_memory_cache && !cache.is_empty() {
            // Remove oldest entry (simple FIFO for now)
            if let Some((old_key, old_thumb)) = cache.iter().next() {
                let old_size = (old_thumb.width() * old_thumb.height() * 4) as usize;
                let old_key = old_key.clone();
                cache.remove(&old_key);
                *usage -= old_size;
            }
        }
        
        // Add new entry
        cache.insert(key, thumb);
        *usage += size;
    }
    
    /// Clear memory cache
    pub fn clear_memory_cache(&self) {
        self.memory_cache.write().unwrap().clear();
        *self.current_memory_usage.write().unwrap() = 0;
    }
    
    /// Get cache statistics
    pub fn cache_stats(&self) -> CacheStats {
        let cache = self.memory_cache.read().unwrap();
        let usage = *self.current_memory_usage.read().unwrap();
        
        CacheStats {
            memory_entries: cache.len(),
            memory_usage: usage,
            max_memory: self.max_memory_cache,
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Number of entries in memory cache
    pub memory_entries: usize,
    /// Current memory usage in bytes
    pub memory_usage: usize,
    /// Maximum memory cache size
    pub max_memory: usize,
}

/// Profile picture generator for PersonNodes
pub struct ProfilePictureGenerator;

impl ProfilePictureGenerator {
    /// Generate avatar from initials
    pub fn generate_from_initials(name: &str, size: u32) -> Result<DynamicImage> {
        // TODO: Implement text rendering when imageproc API stabilizes
        
        // Extract initials
        let initials = Self::extract_initials(name);
        
        // Create image
        let mut img = image::RgbImage::new(size, size);
        
        // Generate background color from name hash
        let bg_color = Self::generate_color(name);
        for pixel in img.pixels_mut() {
            *pixel = bg_color;
        }
        
        // TODO: Draw initials when font rendering is available
        
        Ok(DynamicImage::ImageRgb8(img))
    }
    
    /// Extract initials from name
    fn extract_initials(name: &str) -> String {
        name.split_whitespace()
            .filter_map(|word| word.chars().next())
            .take(2)
            .collect::<String>()
            .to_uppercase()
    }
    
    /// Generate color from name
    fn generate_color(name: &str) -> image::Rgb<u8> {
        let mut hasher = Sha256::new();
        hasher.update(name.as_bytes());
        let hash = hasher.finalize();
        
        // Use first 3 bytes for RGB, ensure reasonable brightness
        let r = (hash[0] % 128) + 64;
        let g = (hash[1] % 128) + 64;
        let b = (hash[2] % 128) + 64;
        
        image::Rgb([r, g, b])
    }
    
    /// Load profile picture from Gravatar
    pub async fn load_from_gravatar(email: &str, size: u32) -> Result<DynamicImage> {
        let email_hash = Self::hash_email(email);
        let url = format!(
            "https://www.gravatar.com/avatar/{}?s={}&d=404",
            email_hash, size
        );
        
        let response = reqwest::get(&url).await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Gravatar not found"));
        }
        
        let bytes = response.bytes().await?;
        let img = image::load_from_memory(&bytes)?;
        
        Ok(img)
    }
    
    /// Hash email for Gravatar
    fn hash_email(email: &str) -> String {
        let normalized = email.trim().to_lowercase();
        use md5::compute;
        format!("{:x}", compute(normalized.as_bytes()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_thumbnail_sizes() {
        assert_eq!(ThumbnailSize::Normal.pixels(), 128);
        assert_eq!(ThumbnailSize::Large.pixels(), 256);
        assert_eq!(ThumbnailSize::XLarge.pixels(), 512);
        
        assert_eq!(ThumbnailSize::Normal.dir_name(), "normal");
        assert_eq!(ThumbnailSize::Large.dir_name(), "large");
        assert_eq!(ThumbnailSize::XLarge.dir_name(), "x-large");
    }
    
    #[test]
    fn test_initials_extraction() {
        assert_eq!(
            ProfilePictureGenerator::extract_initials("John Doe"),
            "JD"
        );
        assert_eq!(
            ProfilePictureGenerator::extract_initials("Alice"),
            "A"
        );
        assert_eq!(
            ProfilePictureGenerator::extract_initials("Bob Charlie David"),
            "BC"
        );
    }
}