//! Visual design system for HorizonOS graph desktop
//! 
//! This module handles:
//! - Icon loading and management (app icons, file type icons)
//! - Thumbnail generation for files
//! - Edge visual styles
//! - Visual effects and animations
//! - Theming support

pub mod icons;
pub mod thumbnails;
pub mod effects;
pub mod theme;

use anyhow::Result;
use std::sync::Arc;

pub use icons::{IconLoader, IconSize, FileTypeIconMapper, AppIconExtractor};
pub use thumbnails::{ThumbnailGenerator, ThumbnailSize, ProfilePictureGenerator};
pub use theme::{Theme as NewTheme, ThemeSystem, ThemeObserver, Color};

/// Visual resource manager
pub struct VisualManager {
    /// Current theme (legacy)
    theme: Arc<Theme>,
    /// New theme system
    theme_system: Arc<ThemeSystem>,
}

/// Simple theme configuration
#[derive(Debug, Clone)]
pub struct Theme {
    /// Theme name
    pub name: String,
    /// Whether this is a dark theme
    pub is_dark: bool,
    /// Primary color
    pub primary_color: [f32; 4],
    /// Secondary color
    pub secondary_color: [f32; 4],
    /// Background color
    pub background_color: [f32; 4],
    /// Text color
    pub text_color: [f32; 4],
}

impl VisualManager {
    /// Create a new visual manager
    pub fn new() -> Result<Self> {
        Ok(Self {
            theme: Arc::new(Theme::default()),
            theme_system: Arc::new(ThemeSystem::new()),
        })
    }
    
    /// Get current theme (legacy)
    pub fn theme(&self) -> &Theme {
        &self.theme
    }
    
    /// Set theme (legacy)
    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = Arc::new(theme);
    }
    
    /// Get new theme system
    pub fn theme_system(&self) -> &ThemeSystem {
        &self.theme_system
    }
    
    /// Get mutable theme system
    pub fn theme_system_mut(&mut self) -> &mut ThemeSystem {
        Arc::get_mut(&mut self.theme_system).unwrap()
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            is_dark: true,
            primary_color: [0.3, 0.6, 1.0, 1.0],
            secondary_color: [0.8, 0.4, 0.8, 1.0],
            background_color: [0.1, 0.1, 0.1, 1.0],
            text_color: [0.9, 0.9, 0.9, 1.0],
        }
    }
}

/// Visual element types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VisualElement {
    /// Application icon
    AppIcon,
    /// File type icon
    FileIcon,
    /// Profile picture
    ProfilePicture,
    /// Thumbnail preview
    Thumbnail,
    /// Custom icon
    Custom,
}

/// Visual priority for rendering order
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum VisualPriority {
    /// Background elements
    Background = 0,
    /// Normal priority
    Normal = 1,
    /// Elevated priority (selected items)
    Elevated = 2,
    /// Foreground elements
    Foreground = 3,
    /// Overlay elements (tooltips, menus)
    Overlay = 4,
}

/// Edge style configuration
#[derive(Debug, Clone)]
pub struct EdgeStyle {
    /// Style name
    pub name: String,
    /// Line color
    pub color: [f32; 4],
    /// Line width
    pub width: f32,
    /// Animated
    pub animated: bool,
    /// Has arrow
    pub has_arrow: bool,
}

impl EdgeStyle {
    /// Create data flow style
    pub fn data_flow() -> Self {
        Self {
            name: "data-flow".to_string(),
            color: [0.3, 0.7, 0.3, 1.0],
            width: 3.0,
            animated: true,
            has_arrow: true,
        }
    }
    
    /// Create dependency style
    pub fn dependency() -> Self {
        Self {
            name: "dependency".to_string(),
            color: [1.0, 0.6, 0.0, 1.0],
            width: 2.0,
            animated: false,
            has_arrow: true,
        }
    }
    
    /// Create relationship style
    pub fn relationship() -> Self {
        Self {
            name: "relationship".to_string(),
            color: [0.6, 0.6, 0.6, 0.8],
            width: 1.5,
            animated: false,
            has_arrow: false,
        }
    }
}