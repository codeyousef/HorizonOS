//! Setting node implementation for configuration values

use crate::{
    GraphNode, NodeVisualData, NodeAction, NodeActionResult, NodeActionType, NodeError, NodeExportData
};
use horizonos_graph_engine::{SceneNode, SceneId, NodeMetadata};
use horizonos_graph_engine::scene::{NodeType, SettingType, SettingScope};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

/// Setting node representing configuration values and preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingNode {
    /// Node ID
    pub id: SceneId,
    /// Setting key/name
    pub key: String,
    /// Setting value
    pub value: String,
    /// Type of setting
    pub setting_type: SettingType,
    /// Scope of the setting
    pub scope: SettingScope,
    /// Default value
    pub default_value: String,
    /// Minimum value (for numeric types)
    pub min_value: Option<f64>,
    /// Maximum value (for numeric types)
    pub max_value: Option<f64>,
    /// Allowed values (for enum types)
    pub allowed_values: Vec<String>,
    /// Setting description
    pub description: Option<String>,
    /// Setting category
    pub category: Option<String>,
    /// Whether the setting is read-only
    pub readonly: bool,
    /// Whether the setting is hidden from UI
    pub hidden: bool,
    /// Whether the setting requires restart
    pub requires_restart: bool,
    /// Whether the setting is deprecated
    pub deprecated: bool,
    /// Validation regex (for string types)
    pub validation_regex: Option<String>,
    /// Unit of measurement
    pub unit: Option<String>,
    /// Previous value (for undo functionality)
    pub previous_value: Option<String>,
    /// Change timestamp
    pub last_changed: Option<DateTime<Utc>>,
    /// Changed by (user/system)
    pub changed_by: Option<String>,
    /// Setting dependencies
    pub dependencies: Vec<String>,
    /// Setting affects (other settings that depend on this)
    pub affects: Vec<String>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Validation function name
    pub validator: Option<String>,
    /// Transformation function name
    pub transformer: Option<String>,
    /// Access control permissions
    pub permissions: SettingPermissions,
    /// Encryption status
    pub encrypted: bool,
    /// Sensitive data flag
    pub sensitive: bool,
    /// Node metadata
    pub metadata: NodeMetadata,
    /// Visual data for rendering
    pub visual_data: NodeVisualData,
}

/// Setting permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingPermissions {
    /// Read permission
    pub read: PermissionLevel,
    /// Write permission
    pub write: PermissionLevel,
    /// Required roles
    pub required_roles: Vec<String>,
    /// Required capabilities
    pub required_capabilities: Vec<String>,
}

/// Permission levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PermissionLevel {
    /// Anyone can access
    Public,
    /// Authenticated users
    User,
    /// Administrators only
    Admin,
    /// System only
    System,
    /// Custom permission
    Custom(String),
}

/// Setting validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the value is valid
    pub valid: bool,
    /// Error message if invalid
    pub error: Option<String>,
    /// Transformed value
    pub transformed_value: Option<String>,
}

impl SettingNode {
    /// Create a new setting node
    pub fn new(
        id: SceneId,
        key: String,
        value: String,
        setting_type: SettingType,
        scope: SettingScope,
    ) -> Self {
        // Set appropriate color based on setting type
        let color = match setting_type {
            SettingType::String => [0.4, 0.7, 0.9, 1.0],    // Light blue
            SettingType::Integer => [0.9, 0.7, 0.4, 1.0],   // Orange
            SettingType::Float => [0.9, 0.8, 0.4, 1.0],     // Yellow
            SettingType::Boolean => [0.6, 0.9, 0.6, 1.0],   // Light green
            SettingType::Color => [0.9, 0.4, 0.7, 1.0],     // Pink
            SettingType::Path => [0.7, 0.5, 0.9, 1.0],      // Purple
            SettingType::Enum => [0.5, 0.9, 0.8, 1.0],      // Cyan
            SettingType::Array => [0.9, 0.6, 0.4, 1.0],     // Coral
            SettingType::Object => [0.6, 0.6, 0.9, 1.0],    // Lavender
        };
        
        let mut visual_data = NodeVisualData::default();
        visual_data.color = color;
        visual_data.radius = 0.8;
        visual_data.icon = Some(Self::get_icon_for_type(&setting_type));
        
        let mut metadata = NodeMetadata::default();
        metadata.description = Some(format!("{:?} setting", setting_type));
        metadata.tags = vec!["setting".to_string(), format!("{:?}", scope).to_lowercase()];
        
        Self {
            id,
            key,
            value: value.clone(),
            setting_type,
            scope,
            default_value: value,
            min_value: None,
            max_value: None,
            allowed_values: Vec::new(),
            description: None,
            category: None,
            readonly: false,
            hidden: false,
            requires_restart: false,
            deprecated: false,
            validation_regex: None,
            unit: None,
            previous_value: None,
            last_changed: None,
            changed_by: None,
            dependencies: Vec::new(),
            affects: Vec::new(),
            tags: Vec::new(),
            validator: None,
            transformer: None,
            permissions: SettingPermissions::default(),
            encrypted: false,
            sensitive: false,
            metadata,
            visual_data,
        }
    }
    
    /// Get icon name for setting type
    fn get_icon_for_type(setting_type: &SettingType) -> String {
        match setting_type {
            SettingType::String => "type".to_string(),
            SettingType::Integer => "hash".to_string(),
            SettingType::Float => "minus".to_string(),
            SettingType::Boolean => "toggle".to_string(),
            SettingType::Color => "palette".to_string(),
            SettingType::Path => "folder".to_string(),
            SettingType::Enum => "list".to_string(),
            SettingType::Array => "layers".to_string(),
            SettingType::Object => "box".to_string(),
        }
    }
    
    /// Validate a value for this setting
    pub fn validate(&self, value: &str) -> ValidationResult {
        // Check if readonly
        if self.readonly {
            return ValidationResult {
                valid: false,
                error: Some("Setting is read-only".to_string()),
                transformed_value: None,
            };
        }
        
        // Type-specific validation
        match self.setting_type {
            SettingType::String => self.validate_string(value),
            SettingType::Integer => self.validate_integer(value),
            SettingType::Float => self.validate_float(value),
            SettingType::Boolean => self.validate_boolean(value),
            SettingType::Color => self.validate_color(value),
            SettingType::Path => self.validate_path(value),
            SettingType::Enum => self.validate_enum(value),
            SettingType::Array => self.validate_array(value),
            SettingType::Object => self.validate_object(value),
        }
    }
    
    /// Validate string value
    fn validate_string(&self, value: &str) -> ValidationResult {
        // Check regex validation
        if let Some(regex_pattern) = &self.validation_regex {
            if let Ok(regex) = regex::Regex::new(regex_pattern) {
                if !regex.is_match(value) {
                    return ValidationResult {
                        valid: false,
                        error: Some(format!("Value does not match pattern: {}", regex_pattern)),
                        transformed_value: None,
                    };
                }
            }
        }
        
        ValidationResult {
            valid: true,
            error: None,
            transformed_value: Some(value.to_string()),
        }
    }
    
    /// Validate integer value
    fn validate_integer(&self, value: &str) -> ValidationResult {
        match value.parse::<i64>() {
            Ok(num) => {
                // Check min/max bounds
                if let Some(min) = self.min_value {
                    if (num as f64) < min {
                        return ValidationResult {
                            valid: false,
                            error: Some(format!("Value must be >= {}", min)),
                            transformed_value: None,
                        };
                    }
                }
                
                if let Some(max) = self.max_value {
                    if (num as f64) > max {
                        return ValidationResult {
                            valid: false,
                            error: Some(format!("Value must be <= {}", max)),
                            transformed_value: None,
                        };
                    }
                }
                
                ValidationResult {
                    valid: true,
                    error: None,
                    transformed_value: Some(num.to_string()),
                }
            }
            Err(_) => ValidationResult {
                valid: false,
                error: Some("Invalid integer value".to_string()),
                transformed_value: None,
            },
        }
    }
    
    /// Validate float value
    fn validate_float(&self, value: &str) -> ValidationResult {
        match value.parse::<f64>() {
            Ok(num) => {
                // Check min/max bounds
                if let Some(min) = self.min_value {
                    if num < min {
                        return ValidationResult {
                            valid: false,
                            error: Some(format!("Value must be >= {}", min)),
                            transformed_value: None,
                        };
                    }
                }
                
                if let Some(max) = self.max_value {
                    if num > max {
                        return ValidationResult {
                            valid: false,
                            error: Some(format!("Value must be <= {}", max)),
                            transformed_value: None,
                        };
                    }
                }
                
                ValidationResult {
                    valid: true,
                    error: None,
                    transformed_value: Some(num.to_string()),
                }
            }
            Err(_) => ValidationResult {
                valid: false,
                error: Some("Invalid float value".to_string()),
                transformed_value: None,
            },
        }
    }
    
    /// Validate boolean value
    fn validate_boolean(&self, value: &str) -> ValidationResult {
        match value.to_lowercase().as_str() {
            "true" | "1" | "yes" | "on" | "enabled" => ValidationResult {
                valid: true,
                error: None,
                transformed_value: Some("true".to_string()),
            },
            "false" | "0" | "no" | "off" | "disabled" => ValidationResult {
                valid: true,
                error: None,
                transformed_value: Some("false".to_string()),
            },
            _ => ValidationResult {
                valid: false,
                error: Some("Invalid boolean value".to_string()),
                transformed_value: None,
            },
        }
    }
    
    /// Validate color value
    fn validate_color(&self, value: &str) -> ValidationResult {
        // Simple color validation (hex, rgb, hsl)
        if value.starts_with('#') && value.len() == 7 {
            // Hex color
            if value[1..].chars().all(|c| c.is_ascii_hexdigit()) {
                return ValidationResult {
                    valid: true,
                    error: None,
                    transformed_value: Some(value.to_string()),
                };
            }
        }
        
        // TODO: Add more color format validation
        
        ValidationResult {
            valid: false,
            error: Some("Invalid color format".to_string()),
            transformed_value: None,
        }
    }
    
    /// Validate path value
    fn validate_path(&self, value: &str) -> ValidationResult {
        // Basic path validation
        if value.is_empty() {
            return ValidationResult {
                valid: false,
                error: Some("Path cannot be empty".to_string()),
                transformed_value: None,
            };
        }
        
        // TODO: Add more sophisticated path validation
        
        ValidationResult {
            valid: true,
            error: None,
            transformed_value: Some(value.to_string()),
        }
    }
    
    /// Validate enum value
    fn validate_enum(&self, value: &str) -> ValidationResult {
        if self.allowed_values.is_empty() {
            return ValidationResult {
                valid: true,
                error: None,
                transformed_value: Some(value.to_string()),
            };
        }
        
        if self.allowed_values.contains(&value.to_string()) {
            ValidationResult {
                valid: true,
                error: None,
                transformed_value: Some(value.to_string()),
            }
        } else {
            ValidationResult {
                valid: false,
                error: Some(format!("Value must be one of: {:?}", self.allowed_values)),
                transformed_value: None,
            }
        }
    }
    
    /// Validate array value
    fn validate_array(&self, value: &str) -> ValidationResult {
        // Try to parse as JSON array
        match serde_json::from_str::<Vec<serde_json::Value>>(value) {
            Ok(_) => ValidationResult {
                valid: true,
                error: None,
                transformed_value: Some(value.to_string()),
            },
            Err(_) => ValidationResult {
                valid: false,
                error: Some("Invalid JSON array".to_string()),
                transformed_value: None,
            },
        }
    }
    
    /// Validate object value
    fn validate_object(&self, value: &str) -> ValidationResult {
        // Try to parse as JSON object
        match serde_json::from_str::<serde_json::Value>(value) {
            Ok(_) => ValidationResult {
                valid: true,
                error: None,
                transformed_value: Some(value.to_string()),
            },
            Err(_) => ValidationResult {
                valid: false,
                error: Some("Invalid JSON object".to_string()),
                transformed_value: None,
            },
        }
    }
    
    /// Set value with validation
    pub fn set_value(&mut self, value: String, changed_by: Option<String>) -> Result<(), NodeError> {
        let validation = self.validate(&value);
        
        if !validation.valid {
            return Err(NodeError::SystemError {
                message: validation.error.unwrap_or("Validation failed".to_string()),
            });
        }
        
        // Store previous value
        self.previous_value = Some(self.value.clone());
        
        // Set new value
        self.value = validation.transformed_value.unwrap_or(value);
        self.last_changed = Some(Utc::now());
        self.changed_by = changed_by;
        
        // Update visual feedback
        self.visual_data.glow = true;
        
        self.last_changed = Some(Utc::now());
        
        Ok(())
    }
    
    /// Reset to default value
    pub fn reset_to_default(&mut self) -> Result<(), NodeError> {
        self.set_value(self.default_value.clone(), Some("system".to_string()))
    }
    
    /// Undo last change
    pub fn undo(&mut self) -> Result<(), NodeError> {
        if let Some(prev_value) = self.previous_value.take() {
            let current_value = self.value.clone();
            self.value = prev_value;
            self.previous_value = Some(current_value);
            self.last_changed = Some(Utc::now());
            self.changed_by = Some("undo".to_string());
            
            self.last_changed = Some(Utc::now());
            Ok(())
        } else {
            Err(NodeError::SystemError {
                message: "No previous value to undo to".to_string(),
            })
        }
    }
    
    /// Get display value (masked if sensitive)
    pub fn display_value(&self) -> String {
        if self.sensitive {
            "***".to_string()
        } else {
            self.value.clone()
        }
    }
    
    /// Check if user has permission to read
    pub fn can_read(&self, user_roles: &[String]) -> bool {
        match self.permissions.read {
            PermissionLevel::Public => true,
            PermissionLevel::User => true, // Assuming authenticated
            PermissionLevel::Admin => user_roles.contains(&"admin".to_string()),
            PermissionLevel::System => false,
            PermissionLevel::Custom(_) => false, // TODO: Implement custom permissions
        }
    }
    
    /// Check if user has permission to write
    pub fn can_write(&self, user_roles: &[String]) -> bool {
        if self.readonly {
            return false;
        }
        
        match self.permissions.write {
            PermissionLevel::Public => true,
            PermissionLevel::User => true, // Assuming authenticated
            PermissionLevel::Admin => user_roles.contains(&"admin".to_string()),
            PermissionLevel::System => false,
            PermissionLevel::Custom(_) => false, // TODO: Implement custom permissions
        }
    }
}

impl GraphNode for SettingNode {
    fn id(&self) -> SceneId {
        self.id
    }
    
    fn display_name(&self) -> String {
        self.key.clone()
    }
    
    fn description(&self) -> Option<String> {
        self.description.clone().or_else(|| {
            Some(format!("{:?} setting: {}", self.setting_type, self.display_value()))
        })
    }
    
    fn visual_data(&self) -> NodeVisualData {
        self.visual_data.clone()
    }
    
    fn update(&mut self, _delta_time: f32) -> Result<(), NodeError> {
        // Reset glow effect after some time
        if self.visual_data.glow {
            self.visual_data.glow = false;
        }
        Ok(())
    }
    
    fn handle_action(&mut self, action: NodeAction) -> Result<NodeActionResult, NodeError> {
        match action {
            NodeAction::Open => {
                Ok(NodeActionResult::Success {
                    message: Some("Opening setting editor".to_string()),
                })
            }
            NodeAction::Edit => {
                Ok(NodeActionResult::Success {
                    message: Some("Editing setting properties".to_string()),
                })
            }
            NodeAction::Delete => {
                Ok(NodeActionResult::ConfirmationRequired {
                    prompt: format!("Delete setting: {}?", self.key),
                })
            }
            NodeAction::Custom { action_type, parameters } => {
                match action_type.as_str() {
                    "set_value" => {
                        if let Some(value) = parameters.get("value") {
                            self.set_value(value.clone(), Some("user".to_string()))?;
                            Ok(NodeActionResult::Success {
                                message: Some(format!("Set {} to {}", self.key, self.display_value())),
                            })
                        } else {
                            Ok(NodeActionResult::Error {
                                error: "Value parameter required".to_string(),
                            })
                        }
                    }
                    "reset" => {
                        self.reset_to_default()?;
                        Ok(NodeActionResult::Success {
                            message: Some(format!("Reset {} to default", self.key)),
                        })
                    }
                    "undo" => {
                        self.undo()?;
                        Ok(NodeActionResult::Success {
                            message: Some(format!("Undid change to {}", self.key)),
                        })
                    }
                    "validate" => {
                        if let Some(value) = parameters.get("value") {
                            let validation = self.validate(value);
                            if validation.valid {
                                Ok(NodeActionResult::Success {
                                    message: Some("Value is valid".to_string()),
                                })
                            } else {
                                Ok(NodeActionResult::Error {
                                    error: validation.error.unwrap_or("Validation failed".to_string()),
                                })
                            }
                        } else {
                            Ok(NodeActionResult::Error {
                                error: "Value parameter required".to_string(),
                            })
                        }
                    }
                    _ => Ok(NodeActionResult::Error {
                        error: format!("Unknown action: {}", action_type),
                    }),
                }
            }
            _ => Ok(NodeActionResult::Error {
                error: "Action not supported for setting nodes".to_string(),
            }),
        }
    }
    
    fn available_actions(&self) -> Vec<NodeActionType> {
        vec![
            NodeActionType::Open,
            NodeActionType::Edit,
            NodeActionType::Delete,
            NodeActionType::Custom("set_value".to_string()),
            NodeActionType::Custom("reset".to_string()),
            NodeActionType::Custom("undo".to_string()),
            NodeActionType::Custom("validate".to_string()),
        ]
    }
    
    fn export_data(&self) -> Result<NodeExportData, NodeError> {
        Ok(NodeExportData {
            node_type: "Setting".to_string(),
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
            node_type: NodeType::Setting {
                key: self.key.clone(),
                value: self.value.clone(),
                setting_type: self.setting_type.clone(),
                scope: self.scope.clone(),
            },
            metadata: self.metadata.clone(),
            visible: self.visual_data.visible,
            selected: self.visual_data.selected,
        }
    }
}

impl Default for SettingPermissions {
    fn default() -> Self {
        Self {
            read: PermissionLevel::User,
            write: PermissionLevel::User,
            required_roles: Vec::new(),
            required_capabilities: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_setting_node_creation() {
        let setting = SettingNode::new(
            1,
            "test_setting".to_string(),
            "test_value".to_string(),
            SettingType::String,
            SettingScope::User,
        );
        
        assert_eq!(setting.id(), 1);
        assert_eq!(setting.key, "test_setting");
        assert_eq!(setting.value, "test_value");
        assert_eq!(setting.display_name(), "test_setting");
    }
    
    #[test]
    fn test_setting_validation() {
        let mut setting = SettingNode::new(
            1,
            "number_setting".to_string(),
            "10".to_string(),
            SettingType::Integer,
            SettingScope::User,
        );
        
        setting.min_value = Some(5.0);
        setting.max_value = Some(15.0);
        
        // Valid value
        let result = setting.validate("10");
        assert!(result.valid);
        
        // Invalid value (too low)
        let result = setting.validate("3");
        assert!(!result.valid);
        
        // Invalid value (too high)
        let result = setting.validate("20");
        assert!(!result.valid);
        
        // Invalid value (not a number)
        let result = setting.validate("abc");
        assert!(!result.valid);
    }
    
    #[test]
    fn test_setting_boolean_validation() {
        let setting = SettingNode::new(
            1,
            "bool_setting".to_string(),
            "true".to_string(),
            SettingType::Boolean,
            SettingScope::User,
        );
        
        // Valid true values
        assert!(setting.validate("true").valid);
        assert!(setting.validate("1").valid);
        assert!(setting.validate("yes").valid);
        assert!(setting.validate("on").valid);
        assert!(setting.validate("enabled").valid);
        
        // Valid false values
        assert!(setting.validate("false").valid);
        assert!(setting.validate("0").valid);
        assert!(setting.validate("no").valid);
        assert!(setting.validate("off").valid);
        assert!(setting.validate("disabled").valid);
        
        // Invalid value
        assert!(!setting.validate("maybe").valid);
    }
    
    #[test]
    fn test_setting_enum_validation() {
        let mut setting = SettingNode::new(
            1,
            "enum_setting".to_string(),
            "option1".to_string(),
            SettingType::Enum,
            SettingScope::User,
        );
        
        setting.allowed_values = vec!["option1".to_string(), "option2".to_string(), "option3".to_string()];
        
        // Valid values
        assert!(setting.validate("option1").valid);
        assert!(setting.validate("option2").valid);
        assert!(setting.validate("option3").valid);
        
        // Invalid value
        assert!(!setting.validate("option4").valid);
    }
    
    #[test]
    fn test_setting_set_value() {
        let mut setting = SettingNode::new(
            1,
            "test_setting".to_string(),
            "old_value".to_string(),
            SettingType::String,
            SettingScope::User,
        );
        
        setting.set_value("new_value".to_string(), Some("user".to_string())).unwrap();
        
        assert_eq!(setting.value, "new_value");
        assert_eq!(setting.previous_value, Some("old_value".to_string()));
        assert_eq!(setting.changed_by, Some("user".to_string()));
        assert!(setting.last_changed.is_some());
    }
    
    #[test]
    fn test_setting_undo() {
        let mut setting = SettingNode::new(
            1,
            "test_setting".to_string(),
            "original_value".to_string(),
            SettingType::String,
            SettingScope::User,
        );
        
        setting.set_value("new_value".to_string(), Some("user".to_string())).unwrap();
        assert_eq!(setting.value, "new_value");
        
        setting.undo().unwrap();
        assert_eq!(setting.value, "original_value");
        assert_eq!(setting.previous_value, Some("new_value".to_string()));
    }
    
    #[test]
    fn test_setting_permissions() {
        let mut setting = SettingNode::new(
            1,
            "admin_setting".to_string(),
            "value".to_string(),
            SettingType::String,
            SettingScope::System,
        );
        
        setting.permissions.read = PermissionLevel::Admin;
        setting.permissions.write = PermissionLevel::Admin;
        
        let user_roles = vec!["user".to_string()];
        let admin_roles = vec!["admin".to_string()];
        
        assert!(!setting.can_read(&user_roles));
        assert!(!setting.can_write(&user_roles));
        
        assert!(setting.can_read(&admin_roles));
        assert!(setting.can_write(&admin_roles));
    }
}