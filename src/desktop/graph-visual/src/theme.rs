//! Theme system for HorizonOS graph desktop
//!
//! This module provides a comprehensive theme system that integrates with
//! the Kotlin DSL configuration to provide rich theming capabilities.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Comprehensive theme system
pub struct ThemeSystem {
    /// Available themes
    themes: HashMap<String, Theme>,
    /// Current active theme
    current_theme: String,
    /// Theme observers
    observers: Vec<Box<dyn ThemeObserver>>,
}

/// Complete theme definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    /// Theme metadata
    pub metadata: ThemeMetadata,
    /// Color palette
    pub colors: ThemeColors,
    /// Node styling
    pub node_styles: NodeStyles,
    /// Edge styling
    pub edge_styles: EdgeStyles,
    /// Visual effects
    pub effects: ThemeEffects,
    /// Animation settings
    pub animations: AnimationSettings,
    /// Typography
    pub typography: Typography,
    /// Layout preferences
    pub layout: LayoutStyles,
}

/// Theme metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeMetadata {
    /// Theme name
    pub name: String,
    /// Display name
    pub display_name: String,
    /// Author
    pub author: String,
    /// Version
    pub version: String,
    /// Description
    pub description: String,
    /// Whether this is a dark theme
    pub is_dark: bool,
    /// Tags
    pub tags: Vec<String>,
}

/// Theme color palette
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeColors {
    /// Primary brand color
    pub primary: Color,
    /// Secondary brand color
    pub secondary: Color,
    /// Accent color
    pub accent: Color,
    /// Background color
    pub background: Color,
    /// Surface color
    pub surface: Color,
    /// Text colors
    pub text: TextColors,
    /// Status colors
    pub status: StatusColors,
    /// Node colors
    pub nodes: NodeColors,
    /// Edge colors
    pub edges: EdgeColors,
    /// UI element colors
    pub ui: UiColors,
}

/// Text colors for different contexts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextColors {
    /// Primary text color
    pub primary: Color,
    /// Secondary text color
    pub secondary: Color,
    /// Muted text color
    pub muted: Color,
    /// Disabled text color
    pub disabled: Color,
    /// Link text color
    pub link: Color,
}

/// Status indication colors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusColors {
    /// Success color
    pub success: Color,
    /// Warning color
    pub warning: Color,
    /// Error color
    pub error: Color,
    /// Info color
    pub info: Color,
}

/// Node-specific colors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeColors {
    /// Default node color
    pub default: Color,
    /// Selected node color
    pub selected: Color,
    /// Hovered node color
    pub hovered: Color,
    /// Active node color
    pub active: Color,
    /// File node color
    pub file: Color,
    /// Application node color
    pub application: Color,
    /// Person node color
    pub person: Color,
    /// Task node color
    pub task: Color,
    /// Concept node color
    pub concept: Color,
}

/// Edge-specific colors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeColors {
    /// Default edge color
    pub default: Color,
    /// Selected edge color
    pub selected: Color,
    /// Data flow edge color
    pub data_flow: Color,
    /// Dependency edge color
    pub dependency: Color,
    /// Relationship edge color
    pub relationship: Color,
    /// Temporal edge color
    pub temporal: Color,
}

/// UI element colors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiColors {
    /// Border color
    pub border: Color,
    /// Focus outline color
    pub focus: Color,
    /// Selection background color
    pub selection_bg: Color,
    /// Hover background color
    pub hover_bg: Color,
    /// Tooltip background color
    pub tooltip_bg: Color,
    /// Menu background color
    pub menu_bg: Color,
}

/// Color representation
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

/// Node styling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStyles {
    /// Default node style
    pub default: NodeStyle,
    /// Node styles by type
    pub by_type: HashMap<String, NodeStyle>,
    /// Node styles by category
    pub by_category: HashMap<String, NodeStyle>,
}

/// Individual node style
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStyle {
    /// Base color
    pub color: Color,
    /// Border color
    pub border_color: Color,
    /// Border width
    pub border_width: f32,
    /// Corner radius
    pub corner_radius: f32,
    /// Shadow configuration
    pub shadow: Option<ShadowConfig>,
    /// Glow configuration
    pub glow: Option<GlowConfig>,
    /// Size scaling
    pub size_scale: f32,
    /// Icon configuration
    pub icon: IconConfig,
    /// Text configuration
    pub text: TextConfig,
}

/// Edge styling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeStyles {
    /// Default edge style
    pub default: EdgeStyle,
    /// Edge styles by type
    pub by_type: HashMap<String, EdgeStyle>,
    /// Edge styles by category
    pub by_category: HashMap<String, EdgeStyle>,
}

/// Individual edge style
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeStyle {
    /// Line color
    pub color: Color,
    /// Line width
    pub width: f32,
    /// Line pattern
    pub pattern: LinePattern,
    /// Arrow configuration
    pub arrow: Option<ArrowConfig>,
    /// Animation configuration
    pub animation: Option<EdgeAnimation>,
    /// Glow configuration
    pub glow: Option<GlowConfig>,
}

/// Line pattern types
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum LinePattern {
    Solid,
    Dashed,
    Dotted,
    DashDot,
}

/// Arrow configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArrowConfig {
    /// Arrow size
    pub size: f32,
    /// Arrow color
    pub color: Color,
    /// Arrow style
    pub style: ArrowStyle,
}

/// Arrow style types
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ArrowStyle {
    Triangle,
    Open,
    Diamond,
    Circle,
}

/// Edge animation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeAnimation {
    /// Animation type
    pub animation_type: EdgeAnimationType,
    /// Animation speed
    pub speed: f32,
    /// Animation direction
    pub direction: AnimationDirection,
}

/// Edge animation types
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum EdgeAnimationType {
    Flow,
    Pulse,
    Glow,
    Dash,
}

/// Animation direction
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AnimationDirection {
    Forward,
    Backward,
    Bidirectional,
}

/// Visual effects configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeEffects {
    /// Global glow settings
    pub glow: GlobalGlowConfig,
    /// Shadow settings
    pub shadows: ShadowConfig,
    /// Particle effects
    pub particles: ParticleConfig,
    /// Blur effects
    pub blur: BlurConfig,
    /// Transparency effects
    pub transparency: TransparencyConfig,
}

/// Global glow configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalGlowConfig {
    /// Whether glow is enabled
    pub enabled: bool,
    /// Global glow intensity
    pub intensity: f32,
    /// Glow color
    pub color: Color,
    /// Glow blur radius
    pub blur_radius: f32,
}

/// Shadow configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowConfig {
    /// Whether shadows are enabled
    pub enabled: bool,
    /// Shadow color
    pub color: Color,
    /// Shadow offset
    pub offset: (f32, f32),
    /// Shadow blur radius
    pub blur_radius: f32,
    /// Shadow spread
    pub spread: f32,
}

/// Glow configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlowConfig {
    /// Whether glow is enabled
    pub enabled: bool,
    /// Glow color
    pub color: Color,
    /// Glow intensity
    pub intensity: f32,
    /// Glow blur radius
    pub blur_radius: f32,
}

/// Particle effects configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticleConfig {
    /// Whether particles are enabled
    pub enabled: bool,
    /// Particle count
    pub count: u32,
    /// Particle size
    pub size: f32,
    /// Particle color
    pub color: Color,
    /// Particle lifetime
    pub lifetime: f32,
    /// Particle speed
    pub speed: f32,
}

/// Blur effects configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlurConfig {
    /// Whether blur is enabled
    pub enabled: bool,
    /// Blur radius
    pub radius: f32,
    /// Blur quality
    pub quality: BlurQuality,
}

/// Blur quality levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum BlurQuality {
    Low,
    Medium,
    High,
}

/// Transparency configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransparencyConfig {
    /// Background transparency
    pub background: f32,
    /// Inactive node transparency
    pub inactive_nodes: f32,
    /// Inactive edge transparency
    pub inactive_edges: f32,
}

/// Animation settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationSettings {
    /// Whether animations are enabled
    pub enabled: bool,
    /// Global animation speed multiplier
    pub speed_multiplier: f32,
    /// Easing function
    pub easing: EasingFunction,
    /// Node animations
    pub nodes: NodeAnimations,
    /// Edge animations
    pub edges: EdgeAnimations,
    /// UI animations
    pub ui: UiAnimations,
}

/// Easing functions
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum EasingFunction {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    Bounce,
    Elastic,
}

/// Node animation settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeAnimations {
    /// Node creation animation
    pub creation: AnimationConfig,
    /// Node selection animation
    pub selection: AnimationConfig,
    /// Node hover animation
    pub hover: AnimationConfig,
    /// Node movement animation
    pub movement: AnimationConfig,
}

/// Edge animation settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeAnimations {
    /// Edge creation animation
    pub creation: AnimationConfig,
    /// Edge selection animation
    pub selection: AnimationConfig,
    /// Edge data flow animation
    pub data_flow: AnimationConfig,
}

/// UI animation settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiAnimations {
    /// Menu animation
    pub menu: AnimationConfig,
    /// Tooltip animation
    pub tooltip: AnimationConfig,
    /// Modal animation
    pub modal: AnimationConfig,
}

/// Animation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationConfig {
    /// Animation duration in seconds
    pub duration: f32,
    /// Animation delay in seconds
    pub delay: f32,
    /// Easing function
    pub easing: EasingFunction,
}

/// Typography configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Typography {
    /// Font family
    pub font_family: String,
    /// Font sizes
    pub sizes: FontSizes,
    /// Font weights
    pub weights: FontWeights,
    /// Line heights
    pub line_heights: LineHeights,
}

/// Font sizes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontSizes {
    /// Extra small
    pub xs: f32,
    /// Small
    pub sm: f32,
    /// Medium
    pub md: f32,
    /// Large
    pub lg: f32,
    /// Extra large
    pub xl: f32,
    /// Extra extra large
    pub xxl: f32,
}

/// Font weights
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontWeights {
    /// Light weight
    pub light: u32,
    /// Normal weight
    pub normal: u32,
    /// Medium weight
    pub medium: u32,
    /// Bold weight
    pub bold: u32,
}

/// Line heights
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineHeights {
    /// Tight line height
    pub tight: f32,
    /// Normal line height
    pub normal: f32,
    /// Loose line height
    pub loose: f32,
}

/// Icon configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IconConfig {
    /// Icon size
    pub size: f32,
    /// Icon color
    pub color: Color,
    /// Icon opacity
    pub opacity: f32,
}

/// Text configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextConfig {
    /// Text color
    pub color: Color,
    /// Font size
    pub size: f32,
    /// Font weight
    pub weight: u32,
    /// Text opacity
    pub opacity: f32,
}

/// Layout styling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutStyles {
    /// Grid settings
    pub grid: GridConfig,
    /// Spacing settings
    pub spacing: SpacingConfig,
    /// Container settings
    pub containers: ContainerConfig,
}

/// Grid configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridConfig {
    /// Grid color
    pub color: Color,
    /// Grid opacity
    pub opacity: f32,
    /// Grid size
    pub size: f32,
    /// Whether grid is visible
    pub visible: bool,
}

/// Spacing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpacingConfig {
    /// Node spacing
    pub node_spacing: f32,
    /// Edge spacing
    pub edge_spacing: f32,
    /// Container padding
    pub container_padding: f32,
    /// Container margin
    pub container_margin: f32,
}

/// Container configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerConfig {
    /// Container background
    pub background: Color,
    /// Container border
    pub border: Color,
    /// Container border width
    pub border_width: f32,
    /// Container corner radius
    pub corner_radius: f32,
}

/// Theme observer trait
pub trait ThemeObserver: Send + Sync {
    /// Called when theme changes
    fn on_theme_changed(&self, theme: &Theme);
}

impl ThemeSystem {
    /// Create new theme system
    pub fn new() -> Self {
        let mut themes = HashMap::new();
        
        // Add default themes
        themes.insert("horizon-dark".to_string(), Theme::horizon_dark());
        themes.insert("horizon-light".to_string(), Theme::horizon_light());
        themes.insert("neon".to_string(), Theme::neon());
        themes.insert("minimal".to_string(), Theme::minimal());
        
        Self {
            themes,
            current_theme: "horizon-dark".to_string(),
            observers: Vec::new(),
        }
    }
    
    /// Get current theme
    pub fn current_theme(&self) -> &Theme {
        self.themes.get(&self.current_theme).unwrap()
    }
    
    /// Set current theme
    pub fn set_theme(&mut self, theme_name: &str) -> Result<()> {
        if !self.themes.contains_key(theme_name) {
            return Err(anyhow::anyhow!("Theme {} not found", theme_name));
        }
        
        self.current_theme = theme_name.to_string();
        let theme = self.themes.get(theme_name).unwrap();
        
        // Notify observers
        for observer in &self.observers {
            observer.on_theme_changed(theme);
        }
        
        Ok(())
    }
    
    /// Add theme
    pub fn add_theme(&mut self, theme: Theme) {
        self.themes.insert(theme.metadata.name.clone(), theme);
    }
    
    /// Remove theme
    pub fn remove_theme(&mut self, theme_name: &str) -> Result<()> {
        if theme_name == self.current_theme {
            return Err(anyhow::anyhow!("Cannot remove current theme"));
        }
        
        self.themes.remove(theme_name);
        Ok(())
    }
    
    /// Get available themes
    pub fn available_themes(&self) -> Vec<&str> {
        self.themes.keys().map(|s| s.as_str()).collect()
    }
    
    /// Add theme observer
    pub fn add_observer(&mut self, observer: Box<dyn ThemeObserver>) {
        self.observers.push(observer);
    }
    
    /// Load theme from Kotlin DSL configuration
    pub fn load_theme_from_kotlin_dsl(
        &mut self,
        kotlin_theme: &horizonos_graph_config::kotlindsl::ThemeDef,
    ) -> Result<()> {
        let theme = Theme::from_kotlin_dsl(kotlin_theme)?;
        self.add_theme(theme);
        Ok(())
    }
}

impl Color {
    /// Create color from RGBA values
    pub fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }
    
    /// Create color from RGB values (alpha = 1.0)
    pub fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }
    
    /// Create color from hex string
    pub fn from_hex(hex: &str) -> Result<Self> {
        let hex = hex.trim_start_matches('#');
        
        if hex.len() != 6 && hex.len() != 8 {
            return Err(anyhow::anyhow!("Invalid hex color format"));
        }
        
        let r = u8::from_str_radix(&hex[0..2], 16)? as f32 / 255.0;
        let g = u8::from_str_radix(&hex[2..4], 16)? as f32 / 255.0;
        let b = u8::from_str_radix(&hex[4..6], 16)? as f32 / 255.0;
        let a = if hex.len() == 8 {
            u8::from_str_radix(&hex[6..8], 16)? as f32 / 255.0
        } else {
            1.0
        };
        
        Ok(Self { r, g, b, a })
    }
    
    /// Convert to array format
    pub fn to_array(&self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

impl Default for AnimationConfig {
    fn default() -> Self {
        Self {
            duration: 0.3,
            delay: 0.0,
            easing: EasingFunction::EaseInOut,
        }
    }
}

impl Theme {
    /// Create Horizon Dark theme
    pub fn horizon_dark() -> Self {
        Self {
            metadata: ThemeMetadata {
                name: "horizon-dark".to_string(),
                display_name: "Horizon Dark".to_string(),
                author: "HorizonOS Team".to_string(),
                version: "1.0.0".to_string(),
                description: "Dark theme for HorizonOS graph desktop".to_string(),
                is_dark: true,
                tags: vec!["dark".to_string(), "modern".to_string()],
            },
            colors: ThemeColors {
                primary: Color::from_hex("#4A90E2").unwrap(),
                secondary: Color::from_hex("#7B68EE").unwrap(),
                accent: Color::from_hex("#FF6B6B").unwrap(),
                background: Color::from_hex("#1E1E1E").unwrap(),
                surface: Color::from_hex("#2D2D2D").unwrap(),
                text: TextColors {
                    primary: Color::from_hex("#FFFFFF").unwrap(),
                    secondary: Color::from_hex("#B0B0B0").unwrap(),
                    muted: Color::from_hex("#808080").unwrap(),
                    disabled: Color::from_hex("#505050").unwrap(),
                    link: Color::from_hex("#4A90E2").unwrap(),
                },
                status: StatusColors {
                    success: Color::from_hex("#4CAF50").unwrap(),
                    warning: Color::from_hex("#FF9800").unwrap(),
                    error: Color::from_hex("#F44336").unwrap(),
                    info: Color::from_hex("#2196F3").unwrap(),
                },
                nodes: NodeColors {
                    default: Color::from_hex("#404040").unwrap(),
                    selected: Color::from_hex("#4A90E2").unwrap(),
                    hovered: Color::from_hex("#505050").unwrap(),
                    active: Color::from_hex("#FF6B6B").unwrap(),
                    file: Color::from_hex("#8BC34A").unwrap(),
                    application: Color::from_hex("#FF9800").unwrap(),
                    person: Color::from_hex("#E91E63").unwrap(),
                    task: Color::from_hex("#9C27B0").unwrap(),
                    concept: Color::from_hex("#00BCD4").unwrap(),
                },
                edges: EdgeColors {
                    default: Color::from_hex("#606060").unwrap(),
                    selected: Color::from_hex("#4A90E2").unwrap(),
                    data_flow: Color::from_hex("#4CAF50").unwrap(),
                    dependency: Color::from_hex("#FF9800").unwrap(),
                    relationship: Color::from_hex("#9C27B0").unwrap(),
                    temporal: Color::from_hex("#00BCD4").unwrap(),
                },
                ui: UiColors {
                    border: Color::from_hex("#404040").unwrap(),
                    focus: Color::from_hex("#4A90E2").unwrap(),
                    selection_bg: Color::rgba(0.29, 0.56, 0.89, 0.2),
                    hover_bg: Color::rgba(1.0, 1.0, 1.0, 0.1),
                    tooltip_bg: Color::from_hex("#2D2D2D").unwrap(),
                    menu_bg: Color::from_hex("#2D2D2D").unwrap(),
                },
            },
            node_styles: NodeStyles {
                default: NodeStyle {
                    color: Color::from_hex("#404040").unwrap(),
                    border_color: Color::from_hex("#606060").unwrap(),
                    border_width: 1.0,
                    corner_radius: 8.0,
                    shadow: Some(ShadowConfig {
                        enabled: true,
                        color: Color::rgba(0.0, 0.0, 0.0, 0.3),
                        offset: (0.0, 2.0),
                        blur_radius: 4.0,
                        spread: 0.0,
                    }),
                    glow: None,
                    size_scale: 1.0,
                    icon: IconConfig {
                        size: 24.0,
                        color: Color::from_hex("#FFFFFF").unwrap(),
                        opacity: 1.0,
                    },
                    text: TextConfig {
                        color: Color::from_hex("#FFFFFF").unwrap(),
                        size: 12.0,
                        weight: 400,
                        opacity: 1.0,
                    },
                },
                by_type: HashMap::new(),
                by_category: HashMap::new(),
            },
            edge_styles: EdgeStyles {
                default: EdgeStyle {
                    color: Color::from_hex("#606060").unwrap(),
                    width: 2.0,
                    pattern: LinePattern::Solid,
                    arrow: Some(ArrowConfig {
                        size: 8.0,
                        color: Color::from_hex("#606060").unwrap(),
                        style: ArrowStyle::Triangle,
                    }),
                    animation: None,
                    glow: None,
                },
                by_type: HashMap::new(),
                by_category: HashMap::new(),
            },
            effects: ThemeEffects {
                glow: GlobalGlowConfig {
                    enabled: true,
                    intensity: 0.8,
                    color: Color::from_hex("#4A90E2").unwrap(),
                    blur_radius: 8.0,
                },
                shadows: ShadowConfig {
                    enabled: true,
                    color: Color::rgba(0.0, 0.0, 0.0, 0.3),
                    offset: (0.0, 2.0),
                    blur_radius: 4.0,
                    spread: 0.0,
                },
                particles: ParticleConfig {
                    enabled: true,
                    count: 50,
                    size: 2.0,
                    color: Color::from_hex("#4A90E2").unwrap(),
                    lifetime: 3.0,
                    speed: 1.0,
                },
                blur: BlurConfig {
                    enabled: true,
                    radius: 4.0,
                    quality: BlurQuality::Medium,
                },
                transparency: TransparencyConfig {
                    background: 0.95,
                    inactive_nodes: 0.5,
                    inactive_edges: 0.3,
                },
            },
            animations: AnimationSettings {
                enabled: true,
                speed_multiplier: 1.0,
                easing: EasingFunction::EaseInOut,
                nodes: NodeAnimations {
                    creation: AnimationConfig::default(),
                    selection: AnimationConfig::default(),
                    hover: AnimationConfig { duration: 0.1, ..Default::default() },
                    movement: AnimationConfig { duration: 0.5, ..Default::default() },
                },
                edges: EdgeAnimations {
                    creation: AnimationConfig::default(),
                    selection: AnimationConfig::default(),
                    data_flow: AnimationConfig { duration: 2.0, ..Default::default() },
                },
                ui: UiAnimations {
                    menu: AnimationConfig { duration: 0.2, ..Default::default() },
                    tooltip: AnimationConfig { duration: 0.1, ..Default::default() },
                    modal: AnimationConfig { duration: 0.3, ..Default::default() },
                },
            },
            typography: Typography {
                font_family: "Inter".to_string(),
                sizes: FontSizes {
                    xs: 10.0,
                    sm: 12.0,
                    md: 14.0,
                    lg: 16.0,
                    xl: 18.0,
                    xxl: 20.0,
                },
                weights: FontWeights {
                    light: 300,
                    normal: 400,
                    medium: 500,
                    bold: 700,
                },
                line_heights: LineHeights {
                    tight: 1.2,
                    normal: 1.5,
                    loose: 1.8,
                },
            },
            layout: LayoutStyles {
                grid: GridConfig {
                    color: Color::rgba(1.0, 1.0, 1.0, 0.1),
                    opacity: 0.5,
                    size: 20.0,
                    visible: false,
                },
                spacing: SpacingConfig {
                    node_spacing: 50.0,
                    edge_spacing: 10.0,
                    container_padding: 20.0,
                    container_margin: 10.0,
                },
                containers: ContainerConfig {
                    background: Color::from_hex("#2D2D2D").unwrap(),
                    border: Color::from_hex("#404040").unwrap(),
                    border_width: 1.0,
                    corner_radius: 8.0,
                },
            },
        }
    }
    
    /// Create Horizon Light theme
    pub fn horizon_light() -> Self {
        let mut theme = Self::horizon_dark();
        theme.metadata.name = "horizon-light".to_string();
        theme.metadata.display_name = "Horizon Light".to_string();
        theme.metadata.is_dark = false;
        theme.colors.background = Color::from_hex("#FFFFFF").unwrap();
        theme.colors.surface = Color::from_hex("#F5F5F5").unwrap();
        theme.colors.text.primary = Color::from_hex("#212121").unwrap();
        theme.colors.text.secondary = Color::from_hex("#757575").unwrap();
        theme.colors.nodes.default = Color::from_hex("#E0E0E0").unwrap();
        theme.colors.edges.default = Color::from_hex("#BDBDBD").unwrap();
        theme
    }
    
    /// Create Neon theme
    pub fn neon() -> Self {
        let mut theme = Self::horizon_dark();
        theme.metadata.name = "neon".to_string();
        theme.metadata.display_name = "Neon".to_string();
        theme.colors.primary = Color::from_hex("#00FFFF").unwrap();
        theme.colors.secondary = Color::from_hex("#FF00FF").unwrap();
        theme.colors.accent = Color::from_hex("#FFFF00").unwrap();
        theme.colors.background = Color::from_hex("#000000").unwrap();
        theme.effects.glow.enabled = true;
        theme.effects.glow.intensity = 1.5;
        theme
    }
    
    /// Create Minimal theme
    pub fn minimal() -> Self {
        let mut theme = Self::horizon_light();
        theme.metadata.name = "minimal".to_string();
        theme.metadata.display_name = "Minimal".to_string();
        theme.effects.glow.enabled = false;
        theme.effects.shadows.enabled = false;
        theme.effects.particles.enabled = false;
        theme.animations.enabled = false;
        theme
    }
    
    /// Create theme from Kotlin DSL configuration
    pub fn from_kotlin_dsl(kotlin_theme: &horizonos_graph_config::kotlindsl::ThemeDef) -> Result<Self> {
        let mut theme = Theme::horizon_dark(); // Start with default
        
        // Update metadata
        theme.metadata.display_name = kotlin_theme.display_name.clone();
        theme.metadata.is_dark = kotlin_theme.is_dark;
        theme.metadata.name = kotlin_theme.display_name.to_lowercase().replace(" ", "-");
        
        // Convert colors
        let colors = &kotlin_theme.colors;
        theme.colors.background = Color::from_hex(&colors.background)?;
        theme.colors.text.primary = Color::from_hex(&colors.foreground)?;
        theme.colors.primary = Color::from_hex(&colors.primary)?;
        theme.colors.secondary = Color::from_hex(&colors.secondary)?;
        theme.colors.accent = Color::from_hex(&colors.accent)?;
        theme.colors.nodes.default = Color::from_hex(&colors.node_default)?;
        theme.colors.edges.default = Color::from_hex(&colors.edge_default)?;
        theme.colors.ui.selection_bg = Color::from_hex(&colors.selection)?;
        
        // Apply theme effects if present
        if let Some(effects) = &kotlin_theme.effects {
            theme.effects.glow.intensity = effects.glow_intensity;
            theme.effects.shadows.color = Color::rgba(0.0, 0.0, 0.0, effects.shadow_opacity);
            theme.animations.speed_multiplier = effects.animation_speed;
        }
        
        Ok(theme)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_theme_system() {
        let mut theme_system = ThemeSystem::new();
        
        // Test getting current theme
        let current = theme_system.current_theme();
        assert_eq!(current.metadata.name, "horizon-dark");
        
        // Test switching themes
        theme_system.set_theme("horizon-light").unwrap();
        assert_eq!(theme_system.current_theme().metadata.name, "horizon-light");
        
        // Test available themes
        let themes = theme_system.available_themes();
        assert!(themes.contains(&"horizon-dark"));
        assert!(themes.contains(&"horizon-light"));
        assert!(themes.contains(&"neon"));
        assert!(themes.contains(&"minimal"));
    }
    
    #[test]
    fn test_color_from_hex() {
        let color = Color::from_hex("#FF0000").unwrap();
        assert_eq!(color.r, 1.0);
        assert_eq!(color.g, 0.0);
        assert_eq!(color.b, 0.0);
        assert_eq!(color.a, 1.0);
        
        let color_alpha = Color::from_hex("#FF0000AA").unwrap();
        assert_eq!(color_alpha.a, 170.0 / 255.0);
    }
    
    #[test]
    fn test_theme_creation() {
        let theme = Theme::horizon_dark();
        assert_eq!(theme.metadata.name, "horizon-dark");
        assert!(theme.metadata.is_dark);
        assert!(theme.effects.glow.enabled);
        assert!(theme.animations.enabled);
    }
}