//! High contrast and color accessibility management

use crate::{AccessibilitySettings, ColorBlindMode};
use anyhow::Result;
use std::collections::HashMap;

/// High contrast and color accessibility manager
#[derive(Debug)]
pub struct ContrastManager {
    /// High contrast mode enabled
    high_contrast: bool,
    /// Current contrast theme
    contrast_theme: ContrastTheme,
    /// Color blind mode
    color_blind_mode: ColorBlindMode,
    /// Custom color filters
    color_filters: Vec<ColorFilter>,
    /// Brightness adjustment
    brightness: f32,
    /// Contrast adjustment
    contrast: f32,
    /// Gamma adjustment
    gamma: f32,
    /// Color temperature
    color_temperature: f32,
    /// Saturation adjustment
    saturation: f32,
}

/// High contrast themes
#[derive(Debug, Clone)]
pub struct ContrastTheme {
    /// Theme name
    pub name: String,
    /// Background color
    pub background: [f32; 4],
    /// Foreground color
    pub foreground: [f32; 4],
    /// Accent color
    pub accent: [f32; 4],
    /// Selection color
    pub selection: [f32; 4],
    /// Link color
    pub link: [f32; 4],
    /// Visited link color
    pub visited_link: [f32; 4],
    /// Border color
    pub border: [f32; 4],
    /// Focus color
    pub focus: [f32; 4],
    /// Success color
    pub success: [f32; 4],
    /// Warning color
    pub warning: [f32; 4],
    /// Error color
    pub error: [f32; 4],
    /// Info color
    pub info: [f32; 4],
}

/// Color filters for accessibility
#[derive(Debug, Clone)]
pub struct ColorFilter {
    /// Filter name
    pub name: String,
    /// Filter type
    pub filter_type: ColorFilterType,
    /// Filter parameters
    pub parameters: HashMap<String, f32>,
    /// Enabled state
    pub enabled: bool,
}

/// Types of color filters
#[derive(Debug, Clone)]
pub enum ColorFilterType {
    /// Protanopia (red-blind) filter
    Protanopia,
    /// Deuteranopia (green-blind) filter
    Deuteranopia,
    /// Tritanopia (blue-blind) filter
    Tritanopia,
    /// Monochrome filter
    Monochrome,
    /// Sepia filter
    Sepia,
    /// Blue light filter
    BlueLight,
    /// Custom matrix filter
    Custom(ColorMatrix),
}

/// Color transformation matrix
#[derive(Debug, Clone)]
pub struct ColorMatrix {
    /// 4x4 transformation matrix
    pub matrix: [[f32; 4]; 4],
}

/// Color accessibility report
#[derive(Debug)]
pub struct ColorAccessibilityReport {
    /// Color contrast ratios
    pub contrast_ratios: Vec<ContrastRatio>,
    /// Color blind simulation results
    pub color_blind_tests: Vec<ColorBlindTest>,
    /// Accessibility violations
    pub violations: Vec<AccessibilityViolation>,
    /// Recommendations
    pub recommendations: Vec<String>,
}

/// Contrast ratio measurement
#[derive(Debug)]
pub struct ContrastRatio {
    /// Foreground color
    pub foreground: [f32; 4],
    /// Background color
    pub background: [f32; 4],
    /// Calculated contrast ratio
    pub ratio: f32,
    /// WCAG AA compliance
    pub wcag_aa: bool,
    /// WCAG AAA compliance
    pub wcag_aaa: bool,
}

/// Color blind test result
#[derive(Debug)]
pub struct ColorBlindTest {
    /// Color blind type
    pub color_blind_type: ColorBlindMode,
    /// Original colors
    pub original_colors: Vec<[f32; 4]>,
    /// Simulated colors
    pub simulated_colors: Vec<[f32; 4]>,
    /// Distinguishability score
    pub distinguishability: f32,
}

/// Accessibility violation
#[derive(Debug)]
pub struct AccessibilityViolation {
    /// Violation type
    pub violation_type: ViolationType,
    /// Severity level
    pub severity: Severity,
    /// Description
    pub description: String,
    /// Affected colors
    pub affected_colors: Vec<[f32; 4]>,
    /// Suggested fixes
    pub suggested_fixes: Vec<String>,
}

/// Types of accessibility violations
#[derive(Debug)]
pub enum ViolationType {
    /// Low contrast
    LowContrast,
    /// Color only differentiation
    ColorOnly,
    /// Insufficient color difference
    InsufficientColorDifference,
    /// Missing alternative text
    MissingAltText,
}

/// Severity levels
#[derive(Debug)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

impl ContrastManager {
    /// Create a new contrast manager
    pub fn new() -> Self {
        Self {
            high_contrast: false,
            contrast_theme: ContrastTheme::default_dark(),
            color_blind_mode: ColorBlindMode::None,
            color_filters: Vec::new(),
            brightness: 1.0,
            contrast: 1.0,
            gamma: 1.0,
            color_temperature: 6500.0,
            saturation: 1.0,
        }
    }

    /// Enable high contrast mode
    pub fn enable_high_contrast(&mut self, theme: Option<ContrastTheme>) -> Result<()> {
        self.high_contrast = true;
        
        if let Some(theme) = theme {
            self.contrast_theme = theme;
        }
        
        log::info!("High contrast mode enabled with theme: {}", self.contrast_theme.name);
        Ok(())
    }

    /// Disable high contrast mode
    pub fn disable_high_contrast(&mut self) -> Result<()> {
        self.high_contrast = false;
        log::info!("High contrast mode disabled");
        Ok(())
    }

    /// Set contrast theme
    pub fn set_contrast_theme(&mut self, theme: ContrastTheme) -> Result<()> {
        self.contrast_theme = theme;
        log::debug!("Contrast theme set to: {}", self.contrast_theme.name);
        Ok(())
    }

    /// Set color blind mode
    pub fn set_color_blind_mode(&mut self, mode: ColorBlindMode) -> Result<()> {
        self.color_blind_mode = mode;
        
        // Update color filters based on mode
        self.update_color_blind_filters()?;
        
        log::debug!("Color blind mode set to: {:?}", mode);
        Ok(())
    }

    /// Add color filter
    pub fn add_color_filter(&mut self, filter: ColorFilter) -> Result<()> {
        self.color_filters.push(filter);
        log::debug!("Color filter added");
        Ok(())
    }

    /// Remove color filter
    pub fn remove_color_filter(&mut self, name: &str) -> Result<()> {
        self.color_filters.retain(|f| f.name != name);
        log::debug!("Color filter removed: {}", name);
        Ok(())
    }

    /// Enable/disable color filter
    pub fn toggle_color_filter(&mut self, name: &str, enabled: bool) -> Result<()> {
        if let Some(filter) = self.color_filters.iter_mut().find(|f| f.name == name) {
            filter.enabled = enabled;
            log::debug!("Color filter '{}' {}", name, if enabled { "enabled" } else { "disabled" });
        }
        Ok(())
    }

    /// Apply color transformations to a color
    pub fn transform_color(&self, color: [f32; 4]) -> [f32; 4] {
        let mut result = color;
        
        // Apply color blind mode filter
        result = self.apply_color_blind_filter(result);
        
        // Apply custom color filters
        for filter in &self.color_filters {
            if filter.enabled {
                result = self.apply_color_filter(result, filter);
            }
        }
        
        // Apply brightness, contrast, gamma adjustments
        result = self.apply_brightness_contrast_gamma(result);
        
        // Apply high contrast theme if enabled
        if self.high_contrast {
            result = self.apply_high_contrast_theme(result);
        }
        
        result
    }

    /// Calculate contrast ratio between two colors
    pub fn calculate_contrast_ratio(&self, foreground: [f32; 4], background: [f32; 4]) -> f32 {
        let fg_luminance = self.calculate_luminance(foreground);
        let bg_luminance = self.calculate_luminance(background);
        
        let lighter = fg_luminance.max(bg_luminance);
        let darker = fg_luminance.min(bg_luminance);
        
        (lighter + 0.05) / (darker + 0.05)
    }

    /// Check WCAG contrast compliance
    pub fn check_wcag_compliance(&self, foreground: [f32; 4], background: [f32; 4]) -> (bool, bool) {
        let ratio = self.calculate_contrast_ratio(foreground, background);
        
        // WCAG AA requires 4.5:1 for normal text, 3:1 for large text
        // WCAG AAA requires 7:1 for normal text, 4.5:1 for large text
        let aa_compliant = ratio >= 4.5;
        let aaa_compliant = ratio >= 7.0;
        
        (aa_compliant, aaa_compliant)
    }

    /// Generate color accessibility report
    pub fn generate_accessibility_report(&self, colors: &[[f32; 4]]) -> ColorAccessibilityReport {
        let mut contrast_ratios = Vec::new();
        let mut color_blind_tests = Vec::new();
        let mut violations = Vec::new();
        let mut recommendations = Vec::new();
        
        // Test all color combinations for contrast
        for (i, &fg) in colors.iter().enumerate() {
            for (j, &bg) in colors.iter().enumerate() {
                if i != j {
                    let ratio = self.calculate_contrast_ratio(fg, bg);
                    let (aa, aaa) = self.check_wcag_compliance(fg, bg);
                    
                    contrast_ratios.push(ContrastRatio {
                        foreground: fg,
                        background: bg,
                        ratio,
                        wcag_aa: aa,
                        wcag_aaa: aaa,
                    });
                    
                    // Check for violations
                    if !aa {
                        violations.push(AccessibilityViolation {
                            violation_type: ViolationType::LowContrast,
                            severity: Severity::High,
                            description: format!("Low contrast ratio: {:.2}", ratio),
                            affected_colors: vec![fg, bg],
                            suggested_fixes: vec!["Increase contrast between colors".to_string()],
                        });
                    }
                }
            }
        }
        
        // Test color blind accessibility
        for mode in [ColorBlindMode::Protanopia, ColorBlindMode::Deuteranopia, ColorBlindMode::Tritanopia] {
            let simulated: Vec<[f32; 4]> = colors.iter()
                .map(|&color| self.simulate_color_blindness(color, mode))
                .collect();
            
            let distinguishability = self.calculate_distinguishability(&simulated);
            
            color_blind_tests.push(ColorBlindTest {
                color_blind_type: mode,
                original_colors: colors.to_vec(),
                simulated_colors: simulated,
                distinguishability,
            });
            
            if distinguishability < 0.7 {
                violations.push(AccessibilityViolation {
                    violation_type: ViolationType::InsufficientColorDifference,
                    severity: Severity::Medium,
                    description: format!("Colors may be difficult to distinguish for {:?}", mode),
                    affected_colors: colors.to_vec(),
                    suggested_fixes: vec!["Add non-color indicators (patterns, text, etc.)".to_string()],
                });
            }
        }
        
        // Generate recommendations
        if violations.iter().any(|v| matches!(v.violation_type, ViolationType::LowContrast)) {
            recommendations.push("Consider using darker backgrounds or lighter text".to_string());
        }
        
        if violations.iter().any(|v| matches!(v.violation_type, ViolationType::InsufficientColorDifference)) {
            recommendations.push("Add patterns, shapes, or text labels to distinguish elements".to_string());
        }
        
        ColorAccessibilityReport {
            contrast_ratios,
            color_blind_tests,
            violations,
            recommendations,
        }
    }

    /// Set brightness adjustment
    pub fn set_brightness(&mut self, brightness: f32) -> Result<()> {
        self.brightness = brightness.max(0.0).min(2.0);
        log::debug!("Brightness set to: {}", self.brightness);
        Ok(())
    }

    /// Set contrast adjustment
    pub fn set_contrast(&mut self, contrast: f32) -> Result<()> {
        self.contrast = contrast.max(0.0).min(2.0);
        log::debug!("Contrast set to: {}", self.contrast);
        Ok(())
    }

    /// Set gamma adjustment
    pub fn set_gamma(&mut self, gamma: f32) -> Result<()> {
        self.gamma = gamma.max(0.5).min(2.0);
        log::debug!("Gamma set to: {}", self.gamma);
        Ok(())
    }

    /// Set color temperature
    pub fn set_color_temperature(&mut self, temperature: f32) -> Result<()> {
        self.color_temperature = temperature.max(2000.0).min(10000.0);
        log::debug!("Color temperature set to: {}", self.color_temperature);
        Ok(())
    }

    /// Set saturation adjustment
    pub fn set_saturation(&mut self, saturation: f32) -> Result<()> {
        self.saturation = saturation.max(0.0).min(2.0);
        log::debug!("Saturation set to: {}", self.saturation);
        Ok(())
    }

    /// Update settings from accessibility settings
    pub fn update_settings(&mut self, settings: &AccessibilitySettings) -> Result<()> {
        // Update high contrast mode
        if settings.high_contrast_enabled != self.high_contrast {
            if settings.high_contrast_enabled {
                self.enable_high_contrast(None)?;
            } else {
                self.disable_high_contrast()?;
            }
        }
        
        // Update color blind mode
        if settings.color_blind_mode as u8 != self.color_blind_mode as u8 {
            self.set_color_blind_mode(settings.color_blind_mode)?;
        }
        
        Ok(())
    }

    /// Get available contrast themes
    pub fn get_available_themes(&self) -> Vec<ContrastTheme> {
        vec![
            ContrastTheme::default_dark(),
            ContrastTheme::default_light(),
            ContrastTheme::high_contrast_black(),
            ContrastTheme::high_contrast_white(),
            ContrastTheme::yellow_on_black(),
            ContrastTheme::green_on_black(),
        ]
    }

    /// Apply color blind filter to a color
    fn apply_color_blind_filter(&self, color: [f32; 4]) -> [f32; 4] {
        match self.color_blind_mode {
            ColorBlindMode::None => color,
            mode => self.simulate_color_blindness(color, mode),
        }
    }

    /// Simulate color blindness for a color
    fn simulate_color_blindness(&self, color: [f32; 4], mode: ColorBlindMode) -> [f32; 4] {
        let [r, g, b, a] = color;
        
        match mode {
            ColorBlindMode::None => color,
            ColorBlindMode::Protanopia => {
                // Protanopia (red-blind) simulation
                let new_r = 0.567 * r + 0.433 * g;
                let new_g = 0.558 * r + 0.442 * g;
                let new_b = 0.242 * g + 0.758 * b;
                [new_r, new_g, new_b, a]
            }
            ColorBlindMode::Deuteranopia => {
                // Deuteranopia (green-blind) simulation
                let new_r = 0.625 * r + 0.375 * g;
                let new_g = 0.7 * r + 0.3 * g;
                let new_b = 0.3 * g + 0.7 * b;
                [new_r, new_g, new_b, a]
            }
            ColorBlindMode::Tritanopia => {
                // Tritanopia (blue-blind) simulation
                let new_r = 0.95 * r + 0.05 * g;
                let new_g = 0.433 * g + 0.567 * b;
                let new_b = 0.475 * g + 0.525 * b;
                [new_r, new_g, new_b, a]
            }
            ColorBlindMode::Monochrome => {
                // Convert to grayscale
                let gray = 0.299 * r + 0.587 * g + 0.114 * b;
                [gray, gray, gray, a]
            }
        }
    }

    /// Apply custom color filter
    fn apply_color_filter(&self, color: [f32; 4], filter: &ColorFilter) -> [f32; 4] {
        match &filter.filter_type {
            ColorFilterType::Sepia => {
                let [r, g, b, a] = color;
                let new_r = (0.393 * r + 0.769 * g + 0.189 * b).min(1.0);
                let new_g = (0.349 * r + 0.686 * g + 0.168 * b).min(1.0);
                let new_b = (0.272 * r + 0.534 * g + 0.131 * b).min(1.0);
                [new_r, new_g, new_b, a]
            }
            ColorFilterType::BlueLight => {
                let [r, g, b, a] = color;
                let strength = filter.parameters.get("strength").unwrap_or(&0.5);
                let new_b = b * (1.0 - strength);
                [r, g, new_b, a]
            }
            ColorFilterType::Custom(matrix) => {
                self.apply_color_matrix(color, matrix)
            }
            _ => color,
        }
    }

    /// Apply color matrix transformation
    fn apply_color_matrix(&self, color: [f32; 4], matrix: &ColorMatrix) -> [f32; 4] {
        let [r, g, b, a] = color;
        let m = &matrix.matrix;
        
        let new_r = m[0][0] * r + m[0][1] * g + m[0][2] * b + m[0][3] * a;
        let new_g = m[1][0] * r + m[1][1] * g + m[1][2] * b + m[1][3] * a;
        let new_b = m[2][0] * r + m[2][1] * g + m[2][2] * b + m[2][3] * a;
        let new_a = m[3][0] * r + m[3][1] * g + m[3][2] * b + m[3][3] * a;
        
        [new_r.max(0.0).min(1.0), new_g.max(0.0).min(1.0), new_b.max(0.0).min(1.0), new_a.max(0.0).min(1.0)]
    }

    /// Apply brightness, contrast, and gamma adjustments
    fn apply_brightness_contrast_gamma(&self, color: [f32; 4]) -> [f32; 4] {
        let [r, g, b, a] = color;
        
        // Apply gamma correction
        let gamma_r = r.powf(1.0 / self.gamma);
        let gamma_g = g.powf(1.0 / self.gamma);
        let gamma_b = b.powf(1.0 / self.gamma);
        
        // Apply brightness and contrast
        let bright_r = ((gamma_r - 0.5) * self.contrast + 0.5) * self.brightness;
        let bright_g = ((gamma_g - 0.5) * self.contrast + 0.5) * self.brightness;
        let bright_b = ((gamma_b - 0.5) * self.contrast + 0.5) * self.brightness;
        
        // Apply saturation
        let gray = 0.299 * bright_r + 0.587 * bright_g + 0.114 * bright_b;
        let sat_r = gray + self.saturation * (bright_r - gray);
        let sat_g = gray + self.saturation * (bright_g - gray);
        let sat_b = gray + self.saturation * (bright_b - gray);
        
        [sat_r.max(0.0).min(1.0), sat_g.max(0.0).min(1.0), sat_b.max(0.0).min(1.0), a]
    }

    /// Apply high contrast theme
    fn apply_high_contrast_theme(&self, color: [f32; 4]) -> [f32; 4] {
        // Simplified high contrast application
        // In a real implementation, this would map colors to theme colors
        // based on their semantic meaning
        
        let luminance = self.calculate_luminance(color);
        
        if luminance > 0.5 {
            self.contrast_theme.foreground
        } else {
            self.contrast_theme.background
        }
    }

    /// Calculate luminance of a color
    fn calculate_luminance(&self, color: [f32; 4]) -> f32 {
        let [r, g, b, _] = color;
        
        // Convert to linear RGB
        let linear_r = if r <= 0.03928 { r / 12.92 } else { ((r + 0.055) / 1.055).powf(2.4) };
        let linear_g = if g <= 0.03928 { g / 12.92 } else { ((g + 0.055) / 1.055).powf(2.4) };
        let linear_b = if b <= 0.03928 { b / 12.92 } else { ((b + 0.055) / 1.055).powf(2.4) };
        
        // Calculate luminance
        0.2126 * linear_r + 0.7152 * linear_g + 0.0722 * linear_b
    }

    /// Calculate distinguishability between colors
    fn calculate_distinguishability(&self, colors: &[[f32; 4]]) -> f32 {
        if colors.len() < 2 {
            return 1.0;
        }
        
        let mut total_distance = 0.0;
        let mut pairs = 0;
        
        for (i, &color1) in colors.iter().enumerate() {
            for &color2 in colors.iter().skip(i + 1) {
                let distance = self.calculate_color_distance(color1, color2);
                total_distance += distance;
                pairs += 1;
            }
        }
        
        if pairs > 0 {
            total_distance / pairs as f32
        } else {
            1.0
        }
    }

    /// Calculate perceptual distance between two colors
    fn calculate_color_distance(&self, color1: [f32; 4], color2: [f32; 4]) -> f32 {
        let [r1, g1, b1, _] = color1;
        let [r2, g2, b2, _] = color2;
        
        // Simple Euclidean distance in RGB space
        // In a real implementation, you might use Delta E or other perceptual distance metrics
        ((r1 - r2).powi(2) + (g1 - g2).powi(2) + (b1 - b2).powi(2)).sqrt()
    }

    /// Update color blind filters based on current mode
    fn update_color_blind_filters(&mut self) -> Result<()> {
        // Remove existing color blind filters
        self.color_filters.retain(|f| !matches!(f.filter_type, 
            ColorFilterType::Protanopia | ColorFilterType::Deuteranopia | ColorFilterType::Tritanopia | ColorFilterType::Monochrome));
        
        // Add appropriate filter based on mode
        match self.color_blind_mode {
            ColorBlindMode::None => {}
            ColorBlindMode::Protanopia => {
                self.color_filters.push(ColorFilter {
                    name: "Protanopia".to_string(),
                    filter_type: ColorFilterType::Protanopia,
                    parameters: HashMap::new(),
                    enabled: true,
                });
            }
            ColorBlindMode::Deuteranopia => {
                self.color_filters.push(ColorFilter {
                    name: "Deuteranopia".to_string(),
                    filter_type: ColorFilterType::Deuteranopia,
                    parameters: HashMap::new(),
                    enabled: true,
                });
            }
            ColorBlindMode::Tritanopia => {
                self.color_filters.push(ColorFilter {
                    name: "Tritanopia".to_string(),
                    filter_type: ColorFilterType::Tritanopia,
                    parameters: HashMap::new(),
                    enabled: true,
                });
            }
            ColorBlindMode::Monochrome => {
                self.color_filters.push(ColorFilter {
                    name: "Monochrome".to_string(),
                    filter_type: ColorFilterType::Monochrome,
                    parameters: HashMap::new(),
                    enabled: true,
                });
            }
        }
        
        Ok(())
    }
}

impl ContrastTheme {
    /// Default dark theme
    pub fn default_dark() -> Self {
        Self {
            name: "Dark".to_string(),
            background: [0.1, 0.1, 0.1, 1.0],
            foreground: [0.9, 0.9, 0.9, 1.0],
            accent: [0.2, 0.6, 1.0, 1.0],
            selection: [0.2, 0.4, 0.8, 1.0],
            link: [0.4, 0.7, 1.0, 1.0],
            visited_link: [0.6, 0.4, 1.0, 1.0],
            border: [0.3, 0.3, 0.3, 1.0],
            focus: [1.0, 0.8, 0.0, 1.0],
            success: [0.0, 0.8, 0.0, 1.0],
            warning: [1.0, 0.6, 0.0, 1.0],
            error: [1.0, 0.2, 0.2, 1.0],
            info: [0.0, 0.7, 1.0, 1.0],
        }
    }

    /// Default light theme
    pub fn default_light() -> Self {
        Self {
            name: "Light".to_string(),
            background: [1.0, 1.0, 1.0, 1.0],
            foreground: [0.0, 0.0, 0.0, 1.0],
            accent: [0.0, 0.4, 0.8, 1.0],
            selection: [0.7, 0.8, 1.0, 1.0],
            link: [0.0, 0.0, 0.8, 1.0],
            visited_link: [0.5, 0.0, 0.5, 1.0],
            border: [0.7, 0.7, 0.7, 1.0],
            focus: [1.0, 0.6, 0.0, 1.0],
            success: [0.0, 0.6, 0.0, 1.0],
            warning: [0.8, 0.4, 0.0, 1.0],
            error: [0.8, 0.0, 0.0, 1.0],
            info: [0.0, 0.4, 0.8, 1.0],
        }
    }

    /// High contrast black theme
    pub fn high_contrast_black() -> Self {
        Self {
            name: "High Contrast Black".to_string(),
            background: [0.0, 0.0, 0.0, 1.0],
            foreground: [1.0, 1.0, 1.0, 1.0],
            accent: [1.0, 1.0, 0.0, 1.0],
            selection: [1.0, 1.0, 0.0, 1.0],
            link: [0.0, 1.0, 1.0, 1.0],
            visited_link: [1.0, 0.0, 1.0, 1.0],
            border: [1.0, 1.0, 1.0, 1.0],
            focus: [1.0, 1.0, 0.0, 1.0],
            success: [0.0, 1.0, 0.0, 1.0],
            warning: [1.0, 1.0, 0.0, 1.0],
            error: [1.0, 0.0, 0.0, 1.0],
            info: [0.0, 1.0, 1.0, 1.0],
        }
    }

    /// High contrast white theme
    pub fn high_contrast_white() -> Self {
        Self {
            name: "High Contrast White".to_string(),
            background: [1.0, 1.0, 1.0, 1.0],
            foreground: [0.0, 0.0, 0.0, 1.0],
            accent: [0.0, 0.0, 1.0, 1.0],
            selection: [0.0, 0.0, 1.0, 1.0],
            link: [0.0, 0.0, 1.0, 1.0],
            visited_link: [0.8, 0.0, 0.8, 1.0],
            border: [0.0, 0.0, 0.0, 1.0],
            focus: [0.0, 0.0, 1.0, 1.0],
            success: [0.0, 0.8, 0.0, 1.0],
            warning: [0.8, 0.4, 0.0, 1.0],
            error: [0.8, 0.0, 0.0, 1.0],
            info: [0.0, 0.0, 0.8, 1.0],
        }
    }

    /// Yellow on black theme
    pub fn yellow_on_black() -> Self {
        Self {
            name: "Yellow on Black".to_string(),
            background: [0.0, 0.0, 0.0, 1.0],
            foreground: [1.0, 1.0, 0.0, 1.0],
            accent: [1.0, 1.0, 0.0, 1.0],
            selection: [1.0, 1.0, 0.0, 1.0],
            link: [1.0, 1.0, 0.0, 1.0],
            visited_link: [0.8, 0.8, 0.0, 1.0],
            border: [1.0, 1.0, 0.0, 1.0],
            focus: [1.0, 1.0, 0.0, 1.0],
            success: [1.0, 1.0, 0.0, 1.0],
            warning: [1.0, 1.0, 0.0, 1.0],
            error: [1.0, 1.0, 0.0, 1.0],
            info: [1.0, 1.0, 0.0, 1.0],
        }
    }

    /// Green on black theme
    pub fn green_on_black() -> Self {
        Self {
            name: "Green on Black".to_string(),
            background: [0.0, 0.0, 0.0, 1.0],
            foreground: [0.0, 1.0, 0.0, 1.0],
            accent: [0.0, 1.0, 0.0, 1.0],
            selection: [0.0, 1.0, 0.0, 1.0],
            link: [0.0, 1.0, 0.0, 1.0],
            visited_link: [0.0, 0.8, 0.0, 1.0],
            border: [0.0, 1.0, 0.0, 1.0],
            focus: [0.0, 1.0, 0.0, 1.0],
            success: [0.0, 1.0, 0.0, 1.0],
            warning: [0.0, 1.0, 0.0, 1.0],
            error: [0.0, 1.0, 0.0, 1.0],
            info: [0.0, 1.0, 0.0, 1.0],
        }
    }
}