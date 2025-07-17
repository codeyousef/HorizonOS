//! Example program to test the configuration system

use horizonos_graph_config::{ConfigManager, ConfigChangeEvent};
use std::path::Path;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    env_logger::init();
    
    println!("Testing HorizonOS Graph Desktop Configuration System");
    println!("====================================================\n");
    
    // Create configuration manager
    let (mut manager, mut change_rx) = ConfigManager::new();
    
    // Initialize from configuration directory
    let config_dir = Path::new("../config");
    if config_dir.exists() {
        println!("Loading configuration from: {:?}", config_dir);
        manager.initialize(config_dir).await?;
    } else {
        println!("Using default configuration (config directory not found)");
    }
    
    // Print current configuration
    let config = manager.config();
    println!("\nCurrent Configuration:");
    println!("  Theme: {}", config.appearance.theme);
    println!("  Terminal: {}", config.general.terminal);
    println!("  Browser: {}", config.general.browser);
    println!("  Log Level: {}", config.general.log_level);
    println!("  AI Enabled: {}", config.ai.enabled);
    println!("  Max FPS: {}", config.performance.max_fps);
    
    // Print current theme
    let theme = manager.current_theme();
    println!("\nCurrent Theme: {}", theme.metadata.name);
    println!("  Primary Color: {}", theme.colors.primary);
    println!("  Background: {}", theme.colors.background);
    println!("  UI Corner Radius: {}", theme.ui.corner_radius);
    
    // Test setting a custom value
    manager.set("test_key", "test_value")?;
    let value: String = manager.get("test_key").unwrap();
    println!("\nCustom value test: {}", value);
    
    // Test theme switching
    println!("\nAttempting to switch to cyberpunk theme...");
    match manager.switch_theme("cyberpunk") {
        Ok(_) => {
            println!("Theme switched successfully!");
            let new_theme = manager.current_theme();
            println!("  New theme: {}", new_theme.metadata.name);
            println!("  Primary Color: {}", new_theme.colors.primary);
        }
        Err(e) => println!("Failed to switch theme: {}", e),
    }
    
    // Test keyboard shortcuts
    println!("\nKeyboard Shortcuts:");
    for (name, shortcut) in &config.shortcuts {
        println!("  {}: {} - {}", name, shortcut.keys, shortcut.description);
    }
    
    // Spawn task to watch for configuration changes
    tokio::spawn(async move {
        loop {
            if change_rx.changed().await.is_ok() {
                let event = change_rx.borrow().clone();
                match event {
                    ConfigChangeEvent::ConfigReloaded => {
                        println!("\n[Event] Configuration reloaded");
                    }
                    ConfigChangeEvent::ThemeChanged(theme) => {
                        println!("\n[Event] Theme changed to: {}", theme);
                    }
                    ConfigChangeEvent::ValueChanged(key) => {
                        println!("\n[Event] Configuration value changed: {}", key);
                    }
                    ConfigChangeEvent::Initialized => {
                        println!("\n[Event] Configuration initialized");
                    }
                }
            }
        }
    });
    
    // Save configuration
    let save_path = Path::new("/tmp/horizonos-config-test.toml");
    println!("\nSaving configuration to: {:?}", save_path);
    manager.save(save_path).await?;
    println!("Configuration saved successfully!");
    
    // Keep the program running briefly to see any file watch events
    println!("\nWatching for configuration changes (press Ctrl+C to exit)...");
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    
    Ok(())
}