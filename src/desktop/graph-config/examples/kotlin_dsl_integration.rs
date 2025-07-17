//! Example of integrating Kotlin DSL configuration with the graph desktop
//! 
//! This example shows how to:
//! 1. Load configuration from Kotlin DSL JSON output
//! 2. Access node and edge type definitions
//! 3. Apply configuration to the graph desktop

use anyhow::Result;
use std::path::Path;
use horizonos_graph_config::{ConfigManager, kotlindsl::KotlinDslLoader};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    
    // Example 1: Load Kotlin DSL configuration directly
    let kotlin_config_path = Path::new("../../kotlin-config/output/json/config.json");
    if kotlin_config_path.exists() {
        println!("Loading Kotlin DSL configuration...");
        let config = KotlinDslLoader::load_and_convert(kotlin_config_path).await?;
        
        println!("Configuration loaded:");
        println!("  Theme: {}", config.appearance.theme);
        println!("  Physics enabled: {}", config.graph.physics_enabled);
        println!("  AI enabled: {}", config.ai.enabled);
        println!("  Max nodes: {}", config.performance.max_nodes);
        
        // Access node type definitions
        println!("\nNode types defined:");
        for (key, value) in &config.custom {
            if key.starts_with("node_type.") {
                let node_name = key.strip_prefix("node_type.").unwrap();
                println!("  - {}", node_name);
            }
        }
        
        // Access edge type definitions
        println!("\nEdge types defined:");
        for (key, value) in &config.custom {
            if key.starts_with("edge_type.") {
                let edge_name = key.strip_prefix("edge_type.").unwrap();
                println!("  - {}", edge_name);
            }
        }
    }
    
    // Example 2: Use ConfigManager with automatic Kotlin DSL detection
    println!("\n\nUsing ConfigManager...");
    let (mut config_manager, mut change_rx) = ConfigManager::new();
    
    // Initialize from a directory that may contain Kotlin DSL output
    let config_dir = Path::new("../../kotlin-config");
    config_manager.initialize(config_dir).await?;
    
    // Access configuration
    let current_config = config_manager.config();
    println!("Current theme: {}", current_config.appearance.theme);
    
    // Access custom node type configuration
    if let Some(app_node_type) = config_manager.get::<serde_json::Value>("node_type.application") {
        println!("\nApplication node configuration:");
        println!("{}", serde_json::to_string_pretty(&app_node_type)?);
    }
    
    // Listen for configuration changes
    tokio::spawn(async move {
        while let Ok(_) = change_rx.changed().await {
            let event = change_rx.borrow();
            println!("Configuration changed: {:?}", event);
        }
    });
    
    // Example 3: Apply configuration to graph components
    println!("\n\nApplying configuration to graph components...");
    
    // Get layout configuration
    if let Some(force_directed_layout) = config_manager.get::<serde_json::Value>("layout.FORCE_DIRECTED") {
        println!("Force-directed layout parameters:");
        if let Some(repulsion) = force_directed_layout.get("nodeRepulsion") {
            println!("  Node repulsion: {}", repulsion);
        }
        if let Some(attraction) = force_directed_layout.get("edgeAttraction") {
            println!("  Edge attraction: {}", attraction);
        }
    }
    
    // Get AI integration settings
    let ai_config = current_config.ai;
    if ai_config.enabled {
        println!("\nAI Integration:");
        println!("  Endpoint: {}", ai_config.ollama_endpoint);
        println!("  Model: {}", ai_config.default_model);
        println!("  Suggestions enabled: {}", ai_config.suggestions_enabled);
    }
    
    // Get keyboard shortcuts
    println!("\nKeyboard shortcuts:");
    for (name, shortcut) in &current_config.shortcuts {
        println!("  {} - {} ({})", shortcut.keys, name, shortcut.description);
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_kotlin_dsl_integration() {
        // Create a sample Kotlin DSL JSON for testing
        let sample_config = r#"{
            "graphDesktop": {
                "enabled": true,
                "renderingEngine": "WEBGPU",
                "enablePhysics": true,
                "enableGestures": true,
                "enableKeyboardNavigation": true,
                "enableVoiceControl": false,
                "maxNodes": 10000,
                "maxEdges": 50000,
                "performanceMode": "BALANCED",
                "nodeTypes": {
                    "application": {
                        "displayName": "Application",
                        "description": "Running applications",
                        "category": "APPLICATION",
                        "icon": "window",
                        "color": "#4A90E2",
                        "shape": "ROUNDED_RECTANGLE",
                        "size": "MEDIUM",
                        "visual": {
                            "useActualIcon": true,
                            "glowOnActivity": true
                        },
                        "behavior": {
                            "doubleClickToLaunch": true,
                            "contextMenuItems": ["Open", "Close"]
                        }
                    }
                },
                "edgeTypes": {},
                "layouts": {},
                "interactions": {},
                "visualEffects": {},
                "workspaces": {},
                "themes": {
                    "dark": {
                        "displayName": "Dark Theme",
                        "isDark": true,
                        "colors": {
                            "background": "#1E1E1E",
                            "foreground": "#E0E0E0",
                            "primary": "#64B5F6",
                            "secondary": "#BA68C8",
                            "accent": "#FFB74D",
                            "nodeDefault": "#424242",
                            "edgeDefault": "#666666",
                            "selection": "#2196F3"
                        }
                    }
                },
                "semanticRules": [],
                "gestures": {},
                "aiIntegration": {
                    "enabled": true,
                    "provider": "ollama",
                    "model": "llama2",
                    "features": {
                        "enableRelationshipDiscovery": true,
                        "enableContentAnalysis": true,
                        "enableSmartClustering": true,
                        "enableWorkflowPrediction": true,
                        "enableSemanticSearch": true
                    },
                    "analysisRules": {}
                }
            }
        }"#;
        
        // Write to temp file
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("kotlin-dsl-config.json");
        tokio::fs::write(&config_path, sample_config).await.unwrap();
        
        // Test loading
        let config = KotlinDslLoader::load_and_convert(&config_path).await.unwrap();
        
        // Verify configuration was loaded correctly
        assert!(config.graph.physics_enabled);
        assert!(config.ai.enabled);
        assert_eq!(config.performance.max_nodes, 10000);
        
        // Check custom node type was stored
        assert!(config.custom.contains_key("node_type.application"));
    }
}