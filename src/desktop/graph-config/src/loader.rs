//! Configuration file loader with support for multiple formats

use serde::{Deserialize, Serialize};
use std::path::Path;
use anyhow::{Result, Context};
use tokio::fs;

use crate::{GraphDesktopConfig, kotlindsl::KotlinDslLoader};

/// Configuration loader
#[derive(Clone)]
pub struct ConfigLoader {
    /// Supported file formats
    formats: Vec<ConfigFormat>,
}

impl ConfigLoader {
    /// Create new configuration loader
    pub fn new() -> Self {
        Self {
            formats: vec![
                ConfigFormat::Toml,
                ConfigFormat::Json,
                ConfigFormat::Yaml,
                ConfigFormat::KotlinDsl,
            ],
        }
    }
    
    /// Load configuration from file
    pub async fn load_config(&self, path: &Path) -> Result<GraphDesktopConfig> {
        let format = self.detect_format(path)?;
        
        // Handle Kotlin DSL separately as it loads from JSON output
        if matches!(format, ConfigFormat::KotlinDsl) {
            return KotlinDslLoader::load_and_convert(path).await;
        }
        
        let content = fs::read_to_string(path).await
            .context("Failed to read configuration file")?;
        
        self.parse_config(&content, format)
    }
    
    /// Save configuration to file
    pub async fn save_config(&self, config: &GraphDesktopConfig, path: &Path) -> Result<()> {
        let format = self.detect_format(path)?;
        let content = self.serialize_config(config, format)?;
        
        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await
                .context("Failed to create configuration directory")?;
        }
        
        fs::write(path, content).await
            .context("Failed to write configuration file")?;
        
        Ok(())
    }
    
    /// Load configuration with defaults
    pub async fn load_with_defaults(&self, path: &Path) -> Result<GraphDesktopConfig> {
        if path.exists() {
            match self.load_config(path).await {
                Ok(config) => Ok(config),
                Err(e) => {
                    log::warn!("Failed to load config from {:?}: {}", path, e);
                    log::info!("Using default configuration");
                    Ok(GraphDesktopConfig::default())
                }
            }
        } else {
            log::info!("Configuration file not found at {:?}, using defaults", path);
            Ok(GraphDesktopConfig::default())
        }
    }
    
    /// Merge configurations
    pub fn merge_configs(&self, base: GraphDesktopConfig, overlay: PartialConfig) -> GraphDesktopConfig {
        let mut config = base;
        
        // Merge general settings
        if let Some(general) = overlay.general {
            if let Some(autostart) = general.autostart {
                config.general.autostart = autostart;
            }
            if let Some(terminal) = general.terminal {
                config.general.terminal = terminal;
            }
            if let Some(browser) = general.browser {
                config.general.browser = browser;
            }
            if let Some(log_level) = general.log_level {
                config.general.log_level = log_level;
            }
            if let Some(debug) = general.debug {
                config.general.debug = debug;
            }
        }
        
        // Merge appearance settings
        if let Some(appearance) = overlay.appearance {
            if let Some(theme) = appearance.theme {
                config.appearance.theme = theme;
            }
            if let Some(icon_theme) = appearance.icon_theme {
                config.appearance.icon_theme = icon_theme;
            }
        }
        
        // Merge custom values
        if let Some(custom) = overlay.custom {
            config.custom.extend(custom);
        }
        
        config
    }
    
    /// Detect configuration format from file extension
    fn detect_format(&self, path: &Path) -> Result<ConfigFormat> {
        let ext = path.extension()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow::anyhow!("No file extension found"))?;
        
        match ext.to_lowercase().as_str() {
            "toml" => Ok(ConfigFormat::Toml),
            "json" => {
                // Check if this is Kotlin DSL output by looking for specific markers
                if path.to_string_lossy().contains("kotlin") || path.to_string_lossy().contains("dsl") {
                    Ok(ConfigFormat::KotlinDsl)
                } else {
                    Ok(ConfigFormat::Json)
                }
            },
            "yaml" | "yml" => Ok(ConfigFormat::Yaml),
            _ => Err(anyhow::anyhow!("Unsupported configuration format: {}", ext)),
        }
    }
    
    /// Parse configuration from string
    fn parse_config(&self, content: &str, format: ConfigFormat) -> Result<GraphDesktopConfig> {
        match format {
            ConfigFormat::Toml => {
                toml::from_str(content)
                    .context("Failed to parse TOML configuration")
            }
            ConfigFormat::Json => {
                serde_json::from_str(content)
                    .context("Failed to parse JSON configuration")
            }
            ConfigFormat::Yaml => {
                serde_yaml::from_str(content)
                    .context("Failed to parse YAML configuration")
            }
            ConfigFormat::KotlinDsl => {
                // This shouldn't be reached as KotlinDsl is handled separately
                Err(anyhow::anyhow!("Kotlin DSL format should be loaded directly"))
            }
        }
    }
    
    /// Serialize configuration to string
    fn serialize_config(&self, config: &GraphDesktopConfig, format: ConfigFormat) -> Result<String> {
        match format {
            ConfigFormat::Toml => {
                toml::to_string_pretty(config)
                    .context("Failed to serialize configuration to TOML")
            }
            ConfigFormat::Json => {
                serde_json::to_string_pretty(config)
                    .context("Failed to serialize configuration to JSON")
            }
            ConfigFormat::Yaml => {
                serde_yaml::to_string(config)
                    .context("Failed to serialize configuration to YAML")
            }
            ConfigFormat::KotlinDsl => {
                // Kotlin DSL is input-only format
                Err(anyhow::anyhow!("Cannot serialize to Kotlin DSL format"))
            }
        }
    }
    
    /// Load configuration directory
    pub async fn load_config_dir(&self, dir: &Path) -> Result<Vec<(String, GraphDesktopConfig)>> {
        let mut configs = Vec::new();
        let mut entries = fs::read_dir(dir).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            
            if path.is_file() {
                if let Ok(_format) = self.detect_format(&path) {
                    match self.load_config(&path).await {
                        Ok(config) => {
                            let name = path.file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("unknown")
                                .to_string();
                            configs.push((name, config));
                        }
                        Err(e) => {
                            log::warn!("Failed to load config {:?}: {}", path, e);
                        }
                    }
                }
            }
        }
        
        Ok(configs)
    }
}

/// Configuration file formats
#[derive(Debug, Clone, Copy)]
enum ConfigFormat {
    Toml,
    Json,
    Yaml,
    KotlinDsl,
}

/// Partial configuration for merging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialConfig {
    pub general: Option<PartialGeneralConfig>,
    pub appearance: Option<PartialAppearanceConfig>,
    pub graph: Option<crate::GraphConfig>,
    pub interaction: Option<crate::InteractionConfig>,
    pub performance: Option<crate::PerformanceConfig>,
    pub ai: Option<crate::AIConfig>,
    pub workspace: Option<crate::WorkspaceConfig>,
    pub accessibility: Option<crate::AccessibilityConfig>,
    pub shortcuts: Option<std::collections::HashMap<String, crate::KeyboardShortcut>>,
    pub custom: Option<std::collections::HashMap<String, serde_json::Value>>,
}

/// Partial general configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialGeneralConfig {
    pub autostart: Option<Vec<String>>,
    pub terminal: Option<String>,
    pub browser: Option<String>,
    pub log_level: Option<String>,
    pub debug: Option<bool>,
}

/// Partial appearance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialAppearanceConfig {
    pub theme: Option<String>,
    pub icon_theme: Option<String>,
    pub fonts: Option<crate::FontConfig>,
    pub animations: Option<crate::AnimationConfig>,
    pub transparency: Option<crate::TransparencyConfig>,
}

/// Configuration migration
pub struct ConfigMigration {
    version: u32,
}

impl ConfigMigration {
    /// Create new migration handler
    pub fn new() -> Self {
        Self {
            version: 1,
        }
    }
    
    /// Migrate configuration to latest version
    pub fn migrate(&self, config: &mut serde_json::Value) -> Result<()> {
        let current_version = config.get("version")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
        
        if current_version < self.version {
            log::info!("Migrating configuration from v{} to v{}", current_version, self.version);
            
            // Apply migrations
            for version in (current_version + 1)..=self.version {
                self.apply_migration(config, version)?;
            }
            
            // Update version
            config["version"] = serde_json::json!(self.version);
        }
        
        Ok(())
    }
    
    /// Apply specific migration
    fn apply_migration(&self, _config: &mut serde_json::Value, version: u32) -> Result<()> {
        match version {
            1 => {
                // Initial version - no migration needed
                Ok(())
            }
            _ => {
                log::warn!("Unknown migration version: {}", version);
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_config_format_detection() {
        let loader = ConfigLoader::new();
        
        assert!(matches!(
            loader.detect_format(Path::new("config.toml")).unwrap(),
            ConfigFormat::Toml
        ));
        
        assert!(matches!(
            loader.detect_format(Path::new("config.json")).unwrap(),
            ConfigFormat::Json
        ));
        
        assert!(matches!(
            loader.detect_format(Path::new("config.yaml")).unwrap(),
            ConfigFormat::Yaml
        ));
        
        assert!(loader.detect_format(Path::new("config.txt")).is_err());
    }
    
    #[test]
    fn test_config_serialization() {
        let loader = ConfigLoader::new();
        let config = GraphDesktopConfig::default();
        
        // Test TOML serialization
        let toml_result = loader.serialize_config(&config, ConfigFormat::Toml);
        assert!(toml_result.is_ok());
        
        // Test JSON serialization
        let json_result = loader.serialize_config(&config, ConfigFormat::Json);
        assert!(json_result.is_ok());
        
        // Test YAML serialization
        let yaml_result = loader.serialize_config(&config, ConfigFormat::Yaml);
        assert!(yaml_result.is_ok());
    }
}