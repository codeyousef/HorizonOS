//! Workspace templates for quick setup

use crate::{Workspace, WorkspaceSettings, layout::{WorkspaceLayout, LayoutType}};
use serde::{Deserialize, Serialize};

/// Workspace template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceTemplate {
    /// Template name
    pub name: String,
    /// Template description
    pub description: String,
    /// Template category
    pub category: TemplateCategory,
    /// Predefined settings
    pub settings: WorkspaceSettings,
    /// Predefined layout
    pub layout: WorkspaceLayout,
    /// Initial metadata
    pub metadata: serde_json::Value,
}

impl WorkspaceTemplate {
    /// Create a workspace from this template
    pub fn instantiate(&self) -> Workspace {
        let mut workspace = Workspace::new(&self.name, &self.description);
        workspace.settings = self.settings.clone();
        workspace.layout = self.layout.clone();
        
        // Add template metadata
        workspace.metadata.insert(
            "template".to_string(),
            serde_json::json!({
                "name": self.name,
                "category": self.category,
            }),
        );
        
        workspace
    }
    
    /// Development workspace template
    pub fn development() -> Self {
        Self {
            name: "Development".to_string(),
            description: "Software development workspace".to_string(),
            category: TemplateCategory::Productivity,
            settings: WorkspaceSettings {
                background_color: [0.05, 0.05, 0.1, 1.0],
                show_grid: true,
                grid_size: 25.0,
                auto_save: true,
                auto_arrange: true,
                node_spacing: 120.0,
            },
            layout: WorkspaceLayout {
                layout_type: LayoutType::Hierarchical,
                ..Default::default()
            },
            metadata: serde_json::json!({
                "tools": ["editor", "terminal", "debugger"],
                "languages": ["rust", "typescript", "python"],
            }),
        }
    }
    
    /// Research workspace template
    pub fn research() -> Self {
        Self {
            name: "Research".to_string(),
            description: "Academic research workspace".to_string(),
            category: TemplateCategory::Academic,
            settings: WorkspaceSettings {
                background_color: [0.1, 0.08, 0.05, 1.0],
                show_grid: false,
                grid_size: 20.0,
                auto_save: true,
                auto_arrange: false,
                node_spacing: 150.0,
            },
            layout: WorkspaceLayout {
                layout_type: LayoutType::ForceDirected,
                ..Default::default()
            },
            metadata: serde_json::json!({
                "tools": ["browser", "notes", "references"],
                "focus": "knowledge-graph",
            }),
        }
    }
    
    /// Creative workspace template
    pub fn creative() -> Self {
        Self {
            name: "Creative".to_string(),
            description: "Creative projects workspace".to_string(),
            category: TemplateCategory::Creative,
            settings: WorkspaceSettings {
                background_color: [0.08, 0.05, 0.1, 1.0],
                show_grid: false,
                grid_size: 10.0,
                auto_save: true,
                auto_arrange: false,
                node_spacing: 100.0,
            },
            layout: WorkspaceLayout {
                layout_type: LayoutType::Manual,
                ..Default::default()
            },
            metadata: serde_json::json!({
                "tools": ["design", "media", "inspiration"],
                "style": "freeform",
            }),
        }
    }
    
    /// Communication workspace template
    pub fn communication() -> Self {
        Self {
            name: "Communication".to_string(),
            description: "Email and messaging workspace".to_string(),
            category: TemplateCategory::Communication,
            settings: WorkspaceSettings {
                background_color: [0.05, 0.08, 0.08, 1.0],
                show_grid: true,
                grid_size: 30.0,
                auto_save: true,
                auto_arrange: true,
                node_spacing: 80.0,
            },
            layout: WorkspaceLayout {
                layout_type: LayoutType::Grid,
                ..Default::default()
            },
            metadata: serde_json::json!({
                "tools": ["email", "chat", "calendar"],
                "organization": "priority-based",
            }),
        }
    }
    
    /// Planning workspace template
    pub fn planning() -> Self {
        Self {
            name: "Planning".to_string(),
            description: "Project planning workspace".to_string(),
            category: TemplateCategory::Productivity,
            settings: WorkspaceSettings {
                background_color: [0.05, 0.1, 0.05, 1.0],
                show_grid: true,
                grid_size: 40.0,
                auto_save: true,
                auto_arrange: false,
                node_spacing: 100.0,
            },
            layout: WorkspaceLayout {
                layout_type: LayoutType::Timeline,
                ..Default::default()
            },
            metadata: serde_json::json!({
                "tools": ["tasks", "calendar", "gantt"],
                "methodology": "agile",
            }),
        }
    }
}

/// Template categories
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TemplateCategory {
    Productivity,
    Creative,
    Academic,
    Communication,
    Entertainment,
    System,
    Custom,
}

/// Template manager
pub struct TemplateManager {
    /// Available templates
    templates: Vec<WorkspaceTemplate>,
}

impl TemplateManager {
    /// Create new template manager with built-in templates
    pub fn new() -> Self {
        Self {
            templates: vec![
                WorkspaceTemplate::development(),
                WorkspaceTemplate::research(),
                WorkspaceTemplate::creative(),
                WorkspaceTemplate::communication(),
                WorkspaceTemplate::planning(),
            ],
        }
    }
    
    /// Get all templates
    pub fn get_templates(&self) -> &[WorkspaceTemplate] {
        &self.templates
    }
    
    /// Get templates by category
    pub fn get_by_category(&self, category: TemplateCategory) -> Vec<&WorkspaceTemplate> {
        self.templates
            .iter()
            .filter(|t| t.category == category)
            .collect()
    }
    
    /// Add a custom template
    pub fn add_template(&mut self, template: WorkspaceTemplate) {
        self.templates.push(template);
    }
    
    /// Create template from existing workspace
    pub fn create_from_workspace(workspace: &Workspace, name: String, category: TemplateCategory) -> WorkspaceTemplate {
        WorkspaceTemplate {
            name,
            description: format!("Template based on {}", workspace.name),
            category,
            settings: workspace.settings.clone(),
            layout: workspace.layout.clone(),
            metadata: serde_json::json!({
                "source_workspace": workspace.id,
                "created_from": workspace.name,
            }),
        }
    }
}