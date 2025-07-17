use crate::{GraphNode, NodeVisualData, NodeAction, NodeActionResult, NodeActionType, NodeError, NodeExportData};
use horizonos_graph_engine::{SceneId, SceneNode, NodeMetadata};
use horizonos_graph_engine::scene::{NodeType, ConfigType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigGroupNode {
    pub id: SceneId,
    pub name: String,
    pub config_type: ConfigType,
    pub items: Vec<String>,
    pub description: Option<String>,
    pub parent_group: Option<String>,
    pub child_groups: Vec<String>,
    pub config_path: Option<PathBuf>,
    pub is_active: bool,
    pub priority: u32,
    pub validation_rules: Vec<ValidationRule>,
    pub dependencies: Vec<String>,
    pub conflicts: Vec<String>,
    pub node_metadata: HashMap<String, String>,
    pub tags: Vec<String>,
    pub access_permissions: AccessPermissions,
    pub version: String,
    pub last_modified: Option<chrono::DateTime<chrono::Utc>>,
    pub modification_history: Vec<ConfigModification>,
    pub backup_enabled: bool,
    pub export_format: Vec<ExportFormat>,
    pub import_sources: Vec<ImportSource>,
    pub metadata: NodeMetadata,
    pub visual_data: NodeVisualData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRule {
    pub rule_type: ValidationRuleType,
    pub expression: String,
    pub error_message: String,
    pub severity: ValidationSeverity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationRuleType {
    Required,
    Range,
    Pattern,
    Custom,
    Dependencies,
    Conflicts,
    Type,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessPermissions {
    pub read_users: Vec<String>,
    pub write_users: Vec<String>,
    pub admin_users: Vec<String>,
    pub read_groups: Vec<String>,
    pub write_groups: Vec<String>,
    pub admin_groups: Vec<String>,
    pub public_read: bool,
    pub public_write: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigModification {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub user: String,
    pub action: ModificationAction,
    pub item: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModificationAction {
    Add,
    Remove,
    Modify,
    Move,
    Rename,
    Duplicate,
    Import,
    Export,
    Reset,
    Backup,
    Restore,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExportFormat {
    Json,
    Yaml,
    Toml,
    Ini,
    Properties,
    Xml,
    Csv,
    Binary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportSource {
    pub source_type: ImportSourceType,
    pub path: String,
    pub format: ExportFormat,
    pub auto_sync: bool,
    pub conflict_resolution: ConflictResolution,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImportSourceType {
    File,
    Directory,
    Url,
    Database,
    Registry,
    Environment,
    CommandOutput,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictResolution {
    Overwrite,
    Skip,
    Merge,
    Prompt,
    Backup,
}

impl ConfigGroupNode {
    pub fn new(id: SceneId, name: String, config_type: ConfigType) -> Self {
        Self {
            id,
            name,
            config_type,
            items: Vec::new(),
            description: None,
            parent_group: None,
            child_groups: Vec::new(),
            config_path: None,
            is_active: true,
            priority: 0,
            validation_rules: Vec::new(),
            dependencies: Vec::new(),
            conflicts: Vec::new(),
            node_metadata: HashMap::new(),
            tags: Vec::new(),
            access_permissions: AccessPermissions::default(),
            version: "1.0.0".to_string(),
            last_modified: None,
            modification_history: Vec::new(),
            backup_enabled: false,
            export_format: vec![ExportFormat::Json],
            import_sources: Vec::new(),
            metadata: NodeMetadata::default(),
            visual_data: NodeVisualData::default(),
        }
    }

    pub fn add_item(&mut self, item: String) -> Result<(), String> {
        if self.items.contains(&item) {
            return Err(format!("Item '{}' already exists in group", item));
        }

        if !self.validate_item(&item) {
            return Err(format!("Item '{}' failed validation", item));
        }

        self.items.push(item.clone());
        self.record_modification(ModificationAction::Add, item, None, None);
        Ok(())
    }

    pub fn remove_item(&mut self, item: &str) -> Result<(), String> {
        let position = self.items.iter().position(|x| x == item)
            .ok_or_else(|| format!("Item '{}' not found in group", item))?;

        self.items.remove(position);
        self.record_modification(ModificationAction::Remove, item.to_string(), Some(item.to_string()), None);
        Ok(())
    }

    pub fn move_item(&mut self, item: &str, new_index: usize) -> Result<(), String> {
        let current_index = self.items.iter().position(|x| x == item)
            .ok_or_else(|| format!("Item '{}' not found in group", item))?;

        if new_index >= self.items.len() {
            return Err("New index out of bounds".to_string());
        }

        let item_value = self.items.remove(current_index);
        self.items.insert(new_index, item_value.clone());
        self.record_modification(ModificationAction::Move, item_value, Some(current_index.to_string()), Some(new_index.to_string()));
        Ok(())
    }

    pub fn validate_group(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        for rule in &self.validation_rules {
            if let Some(error) = self.validate_rule(rule) {
                errors.push(error);
            }
        }

        errors
    }

    fn validate_item(&self, item: &str) -> bool {
        for rule in &self.validation_rules {
            if !self.validate_item_against_rule(item, rule) {
                return false;
            }
        }
        true
    }

    fn validate_rule(&self, rule: &ValidationRule) -> Option<ValidationError> {
        match rule.rule_type {
            ValidationRuleType::Required => {
                if self.items.is_empty() {
                    Some(ValidationError {
                        rule_type: rule.rule_type.clone(),
                        message: rule.error_message.clone(),
                        severity: rule.severity.clone(),
                        item: None,
                    })
                } else {
                    None
                }
            }
            ValidationRuleType::Dependencies => {
                for dep in &self.dependencies {
                    if !self.check_dependency(dep) {
                        return Some(ValidationError {
                            rule_type: rule.rule_type.clone(),
                            message: format!("Dependency '{}' not satisfied", dep),
                            severity: rule.severity.clone(),
                            item: Some(dep.clone()),
                        });
                    }
                }
                None
            }
            ValidationRuleType::Conflicts => {
                for conflict in &self.conflicts {
                    if self.check_conflict(conflict) {
                        return Some(ValidationError {
                            rule_type: rule.rule_type.clone(),
                            message: format!("Conflict with '{}'", conflict),
                            severity: rule.severity.clone(),
                            item: Some(conflict.clone()),
                        });
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn validate_item_against_rule(&self, item: &str, rule: &ValidationRule) -> bool {
        match rule.rule_type {
            ValidationRuleType::Pattern => {
                if let Ok(regex) = regex::Regex::new(&rule.expression) {
                    regex.is_match(item)
                } else {
                    false
                }
            }
            ValidationRuleType::Type => {
                self.validate_item_type(item, &rule.expression)
            }
            _ => true,
        }
    }

    fn validate_item_type(&self, item: &str, type_expr: &str) -> bool {
        match type_expr {
            "string" => true,
            "number" => item.parse::<f64>().is_ok(),
            "integer" => item.parse::<i64>().is_ok(),
            "boolean" => item.parse::<bool>().is_ok(),
            "path" => PathBuf::from(item).is_absolute() || PathBuf::from(item).is_relative(),
            _ => true,
        }
    }

    fn check_dependency(&self, _dependency: &str) -> bool {
        true
    }

    fn check_conflict(&self, _conflict: &str) -> bool {
        false
    }

    pub fn export_config(&self, format: &ExportFormat) -> Result<String, String> {
        match format {
            ExportFormat::Json => {
                serde_json::to_string_pretty(self)
                    .map_err(|e| format!("JSON export failed: {}", e))
            }
            ExportFormat::Yaml => {
                serde_yaml::to_string(self)
                    .map_err(|e| format!("YAML export failed: {}", e))
            }
            ExportFormat::Toml => {
                toml::to_string(self)
                    .map_err(|e| format!("TOML export failed: {}", e))
            }
            _ => Err(format!("Export format {:?} not implemented", format)),
        }
    }

    pub fn import_config(&mut self, data: &str, format: &ExportFormat) -> Result<(), String> {
        match format {
            ExportFormat::Json => {
                let imported: ConfigGroupNode = serde_json::from_str(data)
                    .map_err(|e| format!("JSON import failed: {}", e))?;
                self.merge_config(imported);
                Ok(())
            }
            ExportFormat::Yaml => {
                let imported: ConfigGroupNode = serde_yaml::from_str(data)
                    .map_err(|e| format!("YAML import failed: {}", e))?;
                self.merge_config(imported);
                Ok(())
            }
            ExportFormat::Toml => {
                let imported: ConfigGroupNode = toml::from_str(data)
                    .map_err(|e| format!("TOML import failed: {}", e))?;
                self.merge_config(imported);
                Ok(())
            }
            _ => Err(format!("Import format {:?} not implemented", format)),
        }
    }

    fn merge_config(&mut self, other: ConfigGroupNode) {
        for item in other.items {
            if !self.items.contains(&item) {
                self.items.push(item);
            }
        }
        
        for (key, value) in other.node_metadata {
            self.node_metadata.insert(key, value);
        }
        
        for tag in other.tags {
            if !self.tags.contains(&tag) {
                self.tags.push(tag);
            }
        }
    }

    fn record_modification(&mut self, action: ModificationAction, item: String, old_value: Option<String>, new_value: Option<String>) {
        let modification = ConfigModification {
            timestamp: chrono::Utc::now(),
            user: std::env::var("USER").unwrap_or_else(|_| "unknown".to_string()),
            action,
            item,
            old_value,
            new_value,
            reason: None,
        };
        
        self.modification_history.push(modification);
        self.last_modified = Some(chrono::Utc::now());
    }

    pub fn backup(&self) -> Result<String, String> {
        if !self.backup_enabled {
            return Err("Backup is not enabled for this config group".to_string());
        }
        
        self.export_config(&ExportFormat::Json)
    }

    pub fn restore(&mut self, backup_data: &str) -> Result<(), String> {
        let backup: ConfigGroupNode = serde_json::from_str(backup_data)
            .map_err(|e| format!("Backup restore failed: {}", e))?;
        
        self.items = backup.items;
        self.node_metadata = backup.node_metadata;
        self.tags = backup.tags;
        self.record_modification(ModificationAction::Restore, "full_config".to_string(), None, None);
        
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub rule_type: ValidationRuleType,
    pub message: String,
    pub severity: ValidationSeverity,
    pub item: Option<String>,
}

impl Default for AccessPermissions {
    fn default() -> Self {
        Self {
            read_users: Vec::new(),
            write_users: Vec::new(),
            admin_users: Vec::new(),
            read_groups: Vec::new(),
            write_groups: Vec::new(),
            admin_groups: Vec::new(),
            public_read: true,
            public_write: false,
        }
    }
}

impl GraphNode for ConfigGroupNode {
    fn id(&self) -> SceneId {
        self.id
    }
    
    fn display_name(&self) -> String {
        self.name.clone()
    }
    
    fn description(&self) -> Option<String> {
        self.description.clone()
    }
    
    fn node_type(&self) -> NodeType {
        NodeType::ConfigGroup {
            name: self.name.clone(),
            config_type: self.config_type.clone(),
            items: self.items.clone(),
        }
    }
    
    fn metadata(&self) -> NodeMetadata {
        let properties: HashMap<String, String> = self.node_metadata.clone();
        
        NodeMetadata {
            created_at: chrono::Utc::now(),
            updated_at: self.last_modified.unwrap_or_else(|| chrono::Utc::now()),
            description: self.description.clone(),
            tags: self.tags.clone(),
            properties,
        }
    }
    
    fn visual_data(&self) -> NodeVisualData {
        self.visual_data.clone()
    }
    
    fn update(&mut self, _delta_time: f32) -> Result<(), NodeError> {
        Ok(())
    }
    
    fn handle_action(&mut self, action: NodeAction) -> Result<NodeActionResult, NodeError> {
        match action {
            NodeAction::Open => {
                Ok(NodeActionResult::Success { 
                    message: Some(format!("Opened config group: {}", self.name)) 
                })
            }
            NodeAction::Edit => {
                Ok(NodeActionResult::Success { 
                    message: Some("Config group opened for editing".to_string()) 
                })
            }
            _ => Err(NodeError::InvalidAction { action })
        }
    }
    
    fn available_actions(&self) -> Vec<NodeActionType> {
        vec![
            NodeActionType::Open,
            NodeActionType::Edit,
            NodeActionType::Copy,
        ]
    }
    
    fn export_data(&self) -> Result<NodeExportData, NodeError> {
        Ok(NodeExportData {
            node_type: "ConfigGroup".to_string(),
            display_name: self.display_name(),
            description: self.description(),
            visual_data: self.visual_data(),
            metadata: self.metadata.clone(),
            type_specific_data: serde_json::to_value(self)?,
        })
    }
    
    fn to_scene_node(&self) -> SceneNode {
        SceneNode {
            id: self.id,
            position: self.visual_data.position.into(),
            velocity: nalgebra::Vector3::zeros(),
            radius: self.visual_data.radius,
            color: self.visual_data.color,
            node_type: NodeType::ConfigGroup {
                name: self.name.clone(),
                config_type: self.config_type.clone(),
                items: self.items.clone(),
            },
            metadata: self.metadata.clone(),
            visible: self.visual_data.visible,
            selected: self.visual_data.selected,
        }
    }
}