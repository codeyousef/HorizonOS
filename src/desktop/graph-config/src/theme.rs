//! Theme management system

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use anyhow::Result;

/// Theme manager
pub struct ThemeManager {
    /// Available themes
    themes: HashMap<String, Theme>,
    /// Default theme
    default_theme: Theme,
}

impl ThemeManager {
    /// Create new theme manager
    pub fn new() -> Self {
        let mut manager = Self {
            themes: HashMap::new(),
            default_theme: Theme::default_dark(),
        };
        
        // Add built-in themes
        manager.add_builtin_themes();
        
        manager
    }
    
    /// Add built-in themes
    fn add_builtin_themes(&mut self) {
        // Horizon Dark theme
        self.themes.insert(
            "horizon-dark".to_string(),
            Theme::default_dark()
        );
        
        // Horizon Light theme
        self.themes.insert(
            "horizon-light".to_string(),
            Theme::default_light()
        );
        
        // High contrast theme
        self.themes.insert(
            "high-contrast".to_string(),
            Theme::high_contrast()
        );
        
        // Solarized Dark
        self.themes.insert(
            "solarized-dark".to_string(),
            Theme::solarized_dark()
        );
    }
    
    /// Load themes from directory
    pub async fn load_themes(&mut self, themes_dir: &Path) -> Result<()> {
        use tokio::fs;
        
        let mut entries = fs::read_dir(themes_dir).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("toml") {
                match fs::read_to_string(&path).await {
                    Ok(content) => {
                        match toml::from_str::<Theme>(&content) {
                            Ok(theme) => {
                                let name = path.file_stem()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("unknown")
                                    .to_string();
                                
                                self.themes.insert(name, theme);
                            }
                            Err(e) => {
                                log::warn!("Failed to parse theme file {:?}: {}", path, e);
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!("Failed to read theme file {:?}: {}", path, e);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Get theme by name
    pub fn get_theme(&self, name: &str) -> Option<Theme> {
        self.themes.get(name).cloned()
    }
    
    /// Check if theme exists
    pub fn has_theme(&self, name: &str) -> bool {
        self.themes.contains_key(name)
    }
    
    /// Get default theme
    pub fn default_theme(&self) -> Theme {
        self.default_theme.clone()
    }
    
    /// List available themes
    pub fn list_themes(&self) -> Vec<String> {
        self.themes.keys().cloned().collect()
    }
}

/// Theme definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    /// Theme metadata
    pub metadata: ThemeMetadata,
    /// Color palette
    pub colors: ColorPalette,
    /// UI styles
    pub ui: UIStyles,
    /// Graph styles
    pub graph: GraphStyles,
}

impl Theme {
    /// Default dark theme
    pub fn default_dark() -> Self {
        Self {
            metadata: ThemeMetadata {
                name: "Horizon Dark".to_string(),
                author: "HorizonOS Team".to_string(),
                version: "1.0.0".to_string(),
                description: Some("Default dark theme for HorizonOS".to_string()),
            },
            colors: ColorPalette {
                // Base colors
                background: "#1C1E26".to_string(),
                foreground: "#E0E0E0".to_string(),
                
                // Primary colors
                primary: "#6C6F93".to_string(),
                primary_hover: "#7E82A6".to_string(),
                primary_active: "#5A5D7A".to_string(),
                
                // Secondary colors
                secondary: "#E95678".to_string(),
                secondary_hover: "#EC6A88".to_string(),
                secondary_active: "#D84667".to_string(),
                
                // Accent colors
                accent: "#26BBD9".to_string(),
                accent_hover: "#3BC9E7".to_string(),
                accent_active: "#1EACC7".to_string(),
                
                // Status colors
                success: "#29D398".to_string(),
                warning: "#FAB795".to_string(),
                error: "#E95678".to_string(),
                info: "#26BBD9".to_string(),
                
                // Surface colors
                surface: "#232530".to_string(),
                surface_variant: "#2E303E".to_string(),
                
                // Border colors
                border: "#3E4157".to_string(),
                border_focus: "#6C6F93".to_string(),
                
                // Text colors
                text: "#E0E0E0".to_string(),
                text_secondary: "#AAAAB2".to_string(),
                text_disabled: "#6C6F80".to_string(),
                
                // Shadow color
                shadow: "#00000040".to_string(),
            },
            ui: UIStyles {
                corner_radius: 8.0,
                border_width: 1.0,
                spacing: 8.0,
                padding: 12.0,
                icon_size: 24.0,
                shadow_blur: 20.0,
                shadow_offset: [0.0, 4.0],
            },
            graph: GraphStyles {
                node_colors: NodeColors {
                    application: "#E95678".to_string(),
                    file: "#FAB795".to_string(),
                    person: "#29D398".to_string(),
                    task: "#26BBD9".to_string(),
                    device: "#6C6F93".to_string(),
                    ai_agent: "#B877DB".to_string(),
                    concept: "#FABFB7".to_string(),
                    system: "#9CA1B3".to_string(),
                    url: "#26BBD9".to_string(),
                    automation: "#FAC29A".to_string(),
                    setting: "#6C6F93".to_string(),
                    config_group: "#9CA1B3".to_string(),
                },
                edge_colors: EdgeColors {
                    relationship: "#6C6F93".to_string(),
                    dependency: "#E95678".to_string(),
                    data_flow: "#26BBD9".to_string(),
                    hierarchy: "#29D398".to_string(),
                    temporal: "#FAB795".to_string(),
                    similarity: "#B877DB".to_string(),
                },
                background_grid: GridStyle {
                    enabled: true,
                    color: "#2E303E40".to_string(),
                    size: 50.0,
                    subdivisions: 5,
                },
            },
        }
    }
    
    /// Default light theme
    pub fn default_light() -> Self {
        let mut theme = Self::default_dark();
        
        theme.metadata.name = "Horizon Light".to_string();
        theme.metadata.description = Some("Default light theme for HorizonOS".to_string());
        
        // Update colors for light theme
        theme.colors = ColorPalette {
            background: "#FAFAFA".to_string(),
            foreground: "#1A1C23".to_string(),
            
            primary: "#6C6F93".to_string(),
            primary_hover: "#5A5D7A".to_string(),
            primary_active: "#7E82A6".to_string(),
            
            secondary: "#DA103F".to_string(),
            secondary_hover: "#E6224F".to_string(),
            secondary_active: "#C40F39".to_string(),
            
            accent: "#1EB2D8".to_string(),
            accent_hover: "#30BFE3".to_string(),
            accent_active: "#1A9FC7".to_string(),
            
            success: "#07B57A".to_string(),
            warning: "#F77D26".to_string(),
            error: "#DA103F".to_string(),
            info: "#1EB2D8".to_string(),
            
            surface: "#FFFFFF".to_string(),
            surface_variant: "#F5F5F5".to_string(),
            
            border: "#E0E0E0".to_string(),
            border_focus: "#6C6F93".to_string(),
            
            text: "#1A1C23".to_string(),
            text_secondary: "#6C6F80".to_string(),
            text_disabled: "#AAAAB2".to_string(),
            
            shadow: "#00000020".to_string(),
        };
        
        theme
    }
    
    /// High contrast theme
    pub fn high_contrast() -> Self {
        let mut theme = Self::default_dark();
        
        theme.metadata.name = "High Contrast".to_string();
        theme.metadata.description = Some("High contrast theme for accessibility".to_string());
        
        theme.colors = ColorPalette {
            background: "#000000".to_string(),
            foreground: "#FFFFFF".to_string(),
            
            primary: "#FFFF00".to_string(),
            primary_hover: "#FFFF33".to_string(),
            primary_active: "#CCCC00".to_string(),
            
            secondary: "#FF00FF".to_string(),
            secondary_hover: "#FF33FF".to_string(),
            secondary_active: "#CC00CC".to_string(),
            
            accent: "#00FFFF".to_string(),
            accent_hover: "#33FFFF".to_string(),
            accent_active: "#00CCCC".to_string(),
            
            success: "#00FF00".to_string(),
            warning: "#FFFF00".to_string(),
            error: "#FF0000".to_string(),
            info: "#00FFFF".to_string(),
            
            surface: "#1A1A1A".to_string(),
            surface_variant: "#333333".to_string(),
            
            border: "#FFFFFF".to_string(),
            border_focus: "#FFFF00".to_string(),
            
            text: "#FFFFFF".to_string(),
            text_secondary: "#CCCCCC".to_string(),
            text_disabled: "#666666".to_string(),
            
            shadow: "#FFFFFF40".to_string(),
        };
        
        theme.ui.border_width = 2.0;
        
        theme
    }
    
    /// Solarized dark theme
    pub fn solarized_dark() -> Self {
        let mut theme = Self::default_dark();
        
        theme.metadata.name = "Solarized Dark".to_string();
        theme.metadata.description = Some("Solarized dark color scheme".to_string());
        
        theme.colors.background = "#002B36".to_string();
        theme.colors.foreground = "#839496".to_string();
        theme.colors.primary = "#268BD2".to_string();
        theme.colors.secondary = "#DC322F".to_string();
        theme.colors.accent = "#2AA198".to_string();
        theme.colors.surface = "#073642".to_string();
        theme.colors.surface_variant = "#073642".to_string();
        
        theme
    }
}

/// Theme metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeMetadata {
    /// Theme name
    pub name: String,
    /// Theme author
    pub author: String,
    /// Theme version
    pub version: String,
    /// Theme description
    pub description: Option<String>,
}

/// Color palette
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorPalette {
    // Base colors
    pub background: String,
    pub foreground: String,
    
    // Primary colors
    pub primary: String,
    pub primary_hover: String,
    pub primary_active: String,
    
    // Secondary colors
    pub secondary: String,
    pub secondary_hover: String,
    pub secondary_active: String,
    
    // Accent colors
    pub accent: String,
    pub accent_hover: String,
    pub accent_active: String,
    
    // Status colors
    pub success: String,
    pub warning: String,
    pub error: String,
    pub info: String,
    
    // Surface colors
    pub surface: String,
    pub surface_variant: String,
    
    // Border colors
    pub border: String,
    pub border_focus: String,
    
    // Text colors
    pub text: String,
    pub text_secondary: String,
    pub text_disabled: String,
    
    // Shadow color
    pub shadow: String,
}

/// UI styles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIStyles {
    /// Corner radius for UI elements
    pub corner_radius: f32,
    /// Border width
    pub border_width: f32,
    /// Default spacing
    pub spacing: f32,
    /// Default padding
    pub padding: f32,
    /// Icon size
    pub icon_size: f32,
    /// Shadow blur radius
    pub shadow_blur: f32,
    /// Shadow offset
    pub shadow_offset: [f32; 2],
}

/// Graph styles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStyles {
    /// Node colors by type
    pub node_colors: NodeColors,
    /// Edge colors by type
    pub edge_colors: EdgeColors,
    /// Background grid style
    pub background_grid: GridStyle,
}

/// Node colors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeColors {
    pub application: String,
    pub file: String,
    pub person: String,
    pub task: String,
    pub device: String,
    pub ai_agent: String,
    pub concept: String,
    pub system: String,
    pub url: String,
    pub automation: String,
    pub setting: String,
    pub config_group: String,
}

/// Edge colors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeColors {
    pub relationship: String,
    pub dependency: String,
    pub data_flow: String,
    pub hierarchy: String,
    pub temporal: String,
    pub similarity: String,
}

/// Grid style
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridStyle {
    /// Enable grid
    pub enabled: bool,
    /// Grid color
    pub color: String,
    /// Grid size
    pub size: f32,
    /// Grid subdivisions
    pub subdivisions: u32,
}