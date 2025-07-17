//! Configuration validation system

use std::collections::HashMap;
use anyhow::{Result, Context};

use crate::GraphDesktopConfig;

/// Configuration validator
#[derive(Clone)]
pub struct ConfigValidator;

impl ConfigValidator {
    /// Create new validator
    pub fn new() -> Self {
        Self
    }
    
    /// Validate configuration
    pub fn validate(&self, config: &GraphDesktopConfig) -> Result<()> {
        // General validation
        self.validate_general(&config.general)?;
        
        // Appearance validation
        self.validate_appearance(&config.appearance)?;
        
        // Graph validation
        self.validate_graph(&config.graph)?;
        
        // Interaction validation
        self.validate_interaction(&config.interaction)?;
        
        // Performance validation
        self.validate_performance(&config.performance)?;
        
        // AI validation
        self.validate_ai(&config.ai)?;
        
        // Workspace validation
        self.validate_workspace(&config.workspace)?;
        
        // Accessibility validation
        self.validate_accessibility(&config.accessibility)?;
        
        // Keyboard shortcuts validation
        self.validate_shortcuts(&config.shortcuts)?;
        
        Ok(())
    }
    
    /// Validate general configuration
    fn validate_general(&self, config: &crate::GeneralConfig) -> Result<()> {
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&config.log_level.as_str()) {
            return Err(anyhow::anyhow!(
                "Invalid log level: {}. Must be one of: {:?}",
                config.log_level,
                valid_levels
            ));
        }
        Ok(())
    }
    
    /// Validate appearance configuration
    fn validate_appearance(&self, config: &crate::AppearanceConfig) -> Result<()> {
        // Validate font sizes
        if config.fonts.ui_size <= 0.0 {
            return Err(anyhow::anyhow!("UI font size must be positive"));
        }
        if config.fonts.mono_size <= 0.0 {
            return Err(anyhow::anyhow!("Monospace font size must be positive"));
        }
        
        // Validate animation speed
        if config.animations.speed <= 0.0 {
            return Err(anyhow::anyhow!("Animation speed must be positive"));
        }
        if config.animations.spring_stiffness <= 0.0 {
            return Err(anyhow::anyhow!("Spring stiffness must be positive"));
        }
        if config.animations.spring_damping < 0.0 {
            return Err(anyhow::anyhow!("Spring damping must be non-negative"));
        }
        
        // Validate transparency
        let validate_alpha = |value: f32, name: &str| -> Result<()> {
            if !(0.0..=1.0).contains(&value) {
                return Err(anyhow::anyhow!("{} must be between 0.0 and 1.0", name));
            }
            Ok(())
        };
        
        validate_alpha(config.transparency.windows, "Window transparency")?;
        validate_alpha(config.transparency.panels, "Panel transparency")?;
        
        if config.transparency.blur_radius < 0.0 {
            return Err(anyhow::anyhow!("Blur radius must be non-negative"));
        }
        
        Ok(())
    }
    
    /// Validate graph configuration
    fn validate_graph(&self, config: &crate::GraphConfig) -> Result<()> {
        if config.node_size <= 0.0 {
            return Err(anyhow::anyhow!("Node size must be positive"));
        }
        if config.edge_thickness <= 0.0 {
            return Err(anyhow::anyhow!("Edge thickness must be positive"));
        }
        if config.label_size <= 0.0 {
            return Err(anyhow::anyhow!("Label size must be positive"));
        }
        
        // Validate physics
        if config.physics.friction < 0.0 || config.physics.friction > 1.0 {
            return Err(anyhow::anyhow!("Friction must be between 0.0 and 1.0"));
        }
        
        Ok(())
    }
    
    /// Validate interaction configuration
    fn validate_interaction(&self, config: &crate::InteractionConfig) -> Result<()> {
        if config.mouse_sensitivity <= 0.0 {
            return Err(anyhow::anyhow!("Mouse sensitivity must be positive"));
        }
        if config.scroll_speed <= 0.0 {
            return Err(anyhow::anyhow!("Scroll speed must be positive"));
        }
        if config.double_click_interval == 0 {
            return Err(anyhow::anyhow!("Double-click interval must be positive"));
        }
        if config.drag_threshold < 0.0 {
            return Err(anyhow::anyhow!("Drag threshold must be non-negative"));
        }
        Ok(())
    }
    
    /// Validate performance configuration
    fn validate_performance(&self, config: &crate::PerformanceConfig) -> Result<()> {
        if config.max_fps == 0 || config.max_fps > 1000 {
            return Err(anyhow::anyhow!("Max FPS must be between 1 and 1000"));
        }
        if config.max_nodes == 0 {
            return Err(anyhow::anyhow!("Max nodes must be positive"));
        }
        
        // Validate LOD distances are in ascending order
        let distances = &config.lod_distances;
        if distances[0] >= distances[1] || distances[1] >= distances[2] {
            return Err(anyhow::anyhow!("LOD distances must be in ascending order"));
        }
        
        Ok(())
    }
    
    /// Validate AI configuration
    fn validate_ai(&self, config: &crate::AIConfig) -> Result<()> {
        if config.enabled {
            // Validate Ollama endpoint
            if config.ollama_endpoint.is_empty() {
                return Err(anyhow::anyhow!("Ollama endpoint cannot be empty when AI is enabled"));
            }
            
            // Basic URL validation
            if !config.ollama_endpoint.starts_with("http://") && 
               !config.ollama_endpoint.starts_with("https://") {
                return Err(anyhow::anyhow!("Ollama endpoint must be a valid HTTP(S) URL"));
            }
            
            if config.suggestion_frequency == 0 {
                return Err(anyhow::anyhow!("Suggestion frequency must be positive"));
            }
        }
        Ok(())
    }
    
    /// Validate workspace configuration
    fn validate_workspace(&self, config: &crate::WorkspaceConfig) -> Result<()> {
        if config.default_count == 0 || config.default_count > 100 {
            return Err(anyhow::anyhow!("Default workspace count must be between 1 and 100"));
        }
        Ok(())
    }
    
    /// Validate accessibility configuration
    fn validate_accessibility(&self, config: &crate::AccessibilityConfig) -> Result<()> {
        if config.magnification && config.magnification_level <= 1.0 {
            return Err(anyhow::anyhow!("Magnification level must be greater than 1.0"));
        }
        Ok(())
    }
    
    /// Validate keyboard shortcuts
    fn validate_shortcuts(&self, shortcuts: &HashMap<String, crate::KeyboardShortcut>) -> Result<()> {
        // Check for duplicate key bindings
        let mut seen_keys = HashMap::new();
        for (name, shortcut) in shortcuts {
            if let Some(existing) = seen_keys.get(&shortcut.keys) {
                return Err(anyhow::anyhow!(
                    "Duplicate key binding '{}' for '{}' and '{}'",
                    shortcut.keys,
                    name,
                    existing
                ));
            }
            seen_keys.insert(&shortcut.keys, name);
        }
        Ok(())
    }
}

/// Theme validator
pub struct ThemeValidator;

impl ThemeValidator {
    /// Validate theme
    pub fn validate(theme: &crate::theme::Theme) -> Result<()> {
        // Validate color formats
        Self::validate_colors(&theme.colors)?;
        
        // Validate UI parameters
        if theme.ui.corner_radius < 0.0 {
            return Err(anyhow::anyhow!("Corner radius must be non-negative"));
        }
        if theme.ui.border_width < 0.0 {
            return Err(anyhow::anyhow!("Border width must be non-negative"));
        }
        if theme.ui.spacing < 0.0 {
            return Err(anyhow::anyhow!("Spacing must be non-negative"));
        }
        if theme.ui.padding < 0.0 {
            return Err(anyhow::anyhow!("Padding must be non-negative"));
        }
        if theme.ui.icon_size <= 0.0 {
            return Err(anyhow::anyhow!("Icon size must be positive"));
        }
        if theme.ui.shadow_blur < 0.0 {
            return Err(anyhow::anyhow!("Shadow blur must be non-negative"));
        }
        
        // Validate grid style
        if theme.graph.background_grid.enabled {
            if theme.graph.background_grid.size <= 0.0 {
                return Err(anyhow::anyhow!("Grid size must be positive"));
            }
            if theme.graph.background_grid.subdivisions == 0 {
                return Err(anyhow::anyhow!("Grid subdivisions must be positive"));
            }
        }
        
        Ok(())
    }
    
    /// Validate color palette
    fn validate_colors(colors: &crate::theme::ColorPalette) -> Result<()> {
        let validate_color = |color: &str, name: &str| -> Result<()> {
            if !Self::is_valid_color(color) {
                return Err(anyhow::anyhow!("Invalid color format for {}: {}", name, color));
            }
            Ok(())
        };
        
        // Validate all colors
        validate_color(&colors.background, "background")?;
        validate_color(&colors.foreground, "foreground")?;
        validate_color(&colors.primary, "primary")?;
        validate_color(&colors.primary_hover, "primary_hover")?;
        validate_color(&colors.primary_active, "primary_active")?;
        validate_color(&colors.secondary, "secondary")?;
        validate_color(&colors.secondary_hover, "secondary_hover")?;
        validate_color(&colors.secondary_active, "secondary_active")?;
        validate_color(&colors.accent, "accent")?;
        validate_color(&colors.accent_hover, "accent_hover")?;
        validate_color(&colors.accent_active, "accent_active")?;
        validate_color(&colors.success, "success")?;
        validate_color(&colors.warning, "warning")?;
        validate_color(&colors.error, "error")?;
        validate_color(&colors.info, "info")?;
        validate_color(&colors.surface, "surface")?;
        validate_color(&colors.surface_variant, "surface_variant")?;
        validate_color(&colors.border, "border")?;
        validate_color(&colors.border_focus, "border_focus")?;
        validate_color(&colors.text, "text")?;
        validate_color(&colors.text_secondary, "text_secondary")?;
        validate_color(&colors.text_disabled, "text_disabled")?;
        validate_color(&colors.shadow, "shadow")?;
        
        Ok(())
    }
    
    /// Check if color string is valid
    fn is_valid_color(color: &str) -> bool {
        // Support hex colors with optional alpha
        if color.starts_with('#') {
            let hex = &color[1..];
            return hex.len() == 6 || hex.len() == 8;
        }
        
        // Support rgb/rgba functions
        if color.starts_with("rgb(") || color.starts_with("rgba(") {
            return true;
        }
        
        // Support named colors
        const NAMED_COLORS: &[&str] = &[
            "black", "white", "red", "green", "blue", "yellow", 
            "cyan", "magenta", "transparent"
        ];
        
        NAMED_COLORS.contains(&color)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_color_validation() {
        assert!(ThemeValidator::is_valid_color("#FFFFFF"));
        assert!(ThemeValidator::is_valid_color("#000000FF"));
        assert!(ThemeValidator::is_valid_color("rgb(255, 255, 255)"));
        assert!(ThemeValidator::is_valid_color("rgba(0, 0, 0, 0.5)"));
        assert!(ThemeValidator::is_valid_color("black"));
        assert!(ThemeValidator::is_valid_color("transparent"));
        
        assert!(!ThemeValidator::is_valid_color("#FFF")); // Too short
        assert!(!ThemeValidator::is_valid_color("#GGGGGG")); // Invalid hex
        assert!(!ThemeValidator::is_valid_color("unknown")); // Unknown named color
    }
    
    #[test]
    fn test_config_validation() {
        let validator = ConfigValidator::new();
        let config = GraphDesktopConfig::default();
        
        assert!(validator.validate(&config).is_ok());
    }
}