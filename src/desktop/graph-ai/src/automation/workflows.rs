//! Workflow definitions and management
//! 
//! This module provides structures and utilities for defining, managing,
//! and executing automation workflows.

use crate::AIError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use log::{info, warn, error, debug};

/// Workflow definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    /// Workflow ID
    pub id: Option<String>,
    /// Workflow name
    pub name: String,
    /// Workflow description
    pub description: Option<String>,
    /// Workflow version
    pub version: String,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Updated timestamp
    pub updated_at: DateTime<Utc>,
    /// Whether the workflow is active
    pub active: bool,
    /// Workflow steps
    pub steps: Vec<WorkflowStep>,
    /// Workflow triggers
    pub triggers: Vec<WorkflowTrigger>,
    /// Workflow variables
    pub variables: HashMap<String, serde_json::Value>,
    /// Workflow settings
    pub settings: serde_json::Value,
}

/// Workflow step definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    /// Step ID
    pub id: String,
    /// Step name
    pub name: String,
    /// Step type
    pub step_type: StepType,
    /// Step configuration
    pub config: serde_json::Value,
    /// Step position (for visual workflows)
    pub position: Option<Position>,
    /// Step dependencies
    pub dependencies: Vec<String>,
    /// Step conditions
    pub conditions: Vec<StepCondition>,
    /// Step timeout in seconds
    pub timeout: Option<u64>,
    /// Step retry configuration
    pub retry: Option<RetryConfig>,
    /// Step enabled status
    pub enabled: bool,
}

/// Workflow step types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepType {
    /// HTTP request step
    HttpRequest,
    /// Browser automation step
    BrowserAutomation,
    /// UI automation step
    UIAutomation,
    /// Data transformation step
    DataTransformation,
    /// Conditional logic step
    Conditional,
    /// Loop step
    Loop,
    /// Delay step
    Delay,
    /// Email step
    Email,
    /// File operation step
    FileOperation,
    /// Database operation step
    DatabaseOperation,
    /// API call step
    ApiCall,
    /// Script execution step
    ScriptExecution,
    /// Webhook step
    Webhook,
    /// Custom step
    Custom(String),
}

/// Step position for visual workflows
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Position {
    /// X coordinate
    pub x: f64,
    /// Y coordinate
    pub y: f64,
}

/// Step execution condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepCondition {
    /// Condition type
    pub condition_type: ConditionType,
    /// Variable to check
    pub variable: String,
    /// Comparison operator
    pub operator: ComparisonOperator,
    /// Expected value
    pub value: serde_json::Value,
}

/// Condition types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConditionType {
    /// Check if variable exists
    Exists,
    /// Check variable value
    Value,
    /// Check variable type
    Type,
    /// Check array/object length
    Length,
    /// Custom condition
    Custom(String),
}

/// Comparison operators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComparisonOperator {
    /// Equal
    Equal,
    /// Not equal
    NotEqual,
    /// Greater than
    GreaterThan,
    /// Less than
    LessThan,
    /// Greater than or equal
    GreaterThanOrEqual,
    /// Less than or equal
    LessThanOrEqual,
    /// Contains
    Contains,
    /// Not contains
    NotContains,
    /// Starts with
    StartsWith,
    /// Ends with
    EndsWith,
    /// Matches regex
    Regex,
}

/// Step retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum retry attempts
    pub max_attempts: u32,
    /// Delay between retries in seconds
    pub delay: u64,
    /// Exponential backoff factor
    pub backoff_factor: f64,
    /// Maximum delay between retries
    pub max_delay: u64,
    /// Retry on specific error types
    pub retry_on: Vec<String>,
}

/// Workflow trigger definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTrigger {
    /// Trigger ID
    pub id: String,
    /// Trigger name
    pub name: String,
    /// Trigger type
    pub trigger_type: TriggerType,
    /// Trigger configuration
    pub config: serde_json::Value,
    /// Trigger enabled status
    pub enabled: bool,
}

/// Workflow trigger types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TriggerType {
    /// Manual trigger
    Manual,
    /// Scheduled trigger (cron)
    Schedule,
    /// Webhook trigger
    Webhook,
    /// File system event trigger
    FileSystemEvent,
    /// Database event trigger
    DatabaseEvent,
    /// Email trigger
    Email,
    /// HTTP request trigger
    HttpRequest,
    /// System event trigger
    SystemEvent,
    /// User action trigger
    UserAction,
    /// Custom trigger
    Custom(String),
}

/// Workflow template for common automation patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTemplate {
    /// Template ID
    pub id: String,
    /// Template name
    pub name: String,
    /// Template description
    pub description: String,
    /// Template category
    pub category: String,
    /// Template tags
    pub tags: Vec<String>,
    /// Template definition
    pub definition: WorkflowDefinition,
    /// Template parameters
    pub parameters: Vec<TemplateParameter>,
    /// Template examples
    pub examples: Vec<TemplateExample>,
}

/// Template parameter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateParameter {
    /// Parameter name
    pub name: String,
    /// Parameter description
    pub description: String,
    /// Parameter type
    pub param_type: ParameterType,
    /// Default value
    pub default_value: Option<serde_json::Value>,
    /// Required parameter
    pub required: bool,
    /// Parameter validation
    pub validation: Option<ParameterValidation>,
}

/// Parameter types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParameterType {
    /// String parameter
    String,
    /// Number parameter
    Number,
    /// Boolean parameter
    Boolean,
    /// Array parameter
    Array,
    /// Object parameter
    Object,
    /// URL parameter
    Url,
    /// Email parameter
    Email,
    /// File path parameter
    FilePath,
    /// Custom parameter type
    Custom(String),
}

/// Parameter validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterValidation {
    /// Minimum value/length
    pub min: Option<f64>,
    /// Maximum value/length
    pub max: Option<f64>,
    /// Pattern (regex)
    pub pattern: Option<String>,
    /// Allowed values
    pub allowed_values: Option<Vec<serde_json::Value>>,
}

/// Template example
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateExample {
    /// Example name
    pub name: String,
    /// Example description
    pub description: String,
    /// Example parameters
    pub parameters: HashMap<String, serde_json::Value>,
}

/// Workflow builder for creating workflows programmatically
pub struct WorkflowBuilder {
    /// Workflow being built
    workflow: WorkflowDefinition,
}

impl WorkflowBuilder {
    /// Create a new workflow builder
    pub fn new(name: &str) -> Self {
        let now = Utc::now();
        
        Self {
            workflow: WorkflowDefinition {
                id: Some(Uuid::new_v4().to_string()),
                name: name.to_string(),
                description: None,
                version: "1.0".to_string(),
                created_at: now,
                updated_at: now,
                active: true,
                steps: Vec::new(),
                triggers: Vec::new(),
                variables: HashMap::new(),
                settings: serde_json::json!({}),
            },
        }
    }
    
    /// Set workflow description
    pub fn description(mut self, description: &str) -> Self {
        self.workflow.description = Some(description.to_string());
        self
    }
    
    /// Set workflow version
    pub fn version(mut self, version: &str) -> Self {
        self.workflow.version = version.to_string();
        self
    }
    
    /// Set workflow active status
    pub fn active(mut self, active: bool) -> Self {
        self.workflow.active = active;
        self
    }
    
    /// Add a step to the workflow
    pub fn add_step(mut self, step: WorkflowStep) -> Self {
        self.workflow.steps.push(step);
        self
    }
    
    /// Add a trigger to the workflow
    pub fn add_trigger(mut self, trigger: WorkflowTrigger) -> Self {
        self.workflow.triggers.push(trigger);
        self
    }
    
    /// Add a variable to the workflow
    pub fn add_variable(mut self, name: &str, value: serde_json::Value) -> Self {
        self.workflow.variables.insert(name.to_string(), value);
        self
    }
    
    /// Set workflow settings
    pub fn settings(mut self, settings: serde_json::Value) -> Self {
        self.workflow.settings = settings;
        self
    }
    
    /// Build the workflow
    pub fn build(self) -> WorkflowDefinition {
        self.workflow
    }
}

/// Step builder for creating workflow steps
pub struct StepBuilder {
    /// Step being built
    step: WorkflowStep,
}

impl StepBuilder {
    /// Create a new step builder
    pub fn new(name: &str, step_type: StepType) -> Self {
        Self {
            step: WorkflowStep {
                id: Uuid::new_v4().to_string(),
                name: name.to_string(),
                step_type,
                config: serde_json::json!({}),
                position: None,
                dependencies: Vec::new(),
                conditions: Vec::new(),
                timeout: None,
                retry: None,
                enabled: true,
            },
        }
    }
    
    /// Set step configuration
    pub fn config(mut self, config: serde_json::Value) -> Self {
        self.step.config = config;
        self
    }
    
    /// Set step position
    pub fn position(mut self, x: f64, y: f64) -> Self {
        self.step.position = Some(Position { x, y });
        self
    }
    
    /// Add step dependency
    pub fn depends_on(mut self, step_id: &str) -> Self {
        self.step.dependencies.push(step_id.to_string());
        self
    }
    
    /// Add step condition
    pub fn condition(mut self, condition: StepCondition) -> Self {
        self.step.conditions.push(condition);
        self
    }
    
    /// Set step timeout
    pub fn timeout(mut self, seconds: u64) -> Self {
        self.step.timeout = Some(seconds);
        self
    }
    
    /// Set step retry configuration
    pub fn retry(mut self, retry: RetryConfig) -> Self {
        self.step.retry = Some(retry);
        self
    }
    
    /// Set step enabled status
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.step.enabled = enabled;
        self
    }
    
    /// Build the step
    pub fn build(self) -> WorkflowStep {
        self.step
    }
}

/// Workflow validator
pub struct WorkflowValidator;

impl WorkflowValidator {
    /// Validate a workflow definition
    pub fn validate(workflow: &WorkflowDefinition) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();
        
        // Validate basic fields
        if workflow.name.is_empty() {
            errors.push(ValidationError {
                field: "name".to_string(),
                message: "Workflow name cannot be empty".to_string(),
            });
        }
        
        if workflow.version.is_empty() {
            errors.push(ValidationError {
                field: "version".to_string(),
                message: "Workflow version cannot be empty".to_string(),
            });
        }
        
        // Validate steps
        for (i, step) in workflow.steps.iter().enumerate() {
            if let Err(step_errors) = Self::validate_step(step) {
                for error in step_errors {
                    errors.push(ValidationError {
                        field: format!("steps[{}].{}", i, error.field),
                        message: error.message,
                    });
                }
            }
        }
        
        // Validate triggers
        for (i, trigger) in workflow.triggers.iter().enumerate() {
            if let Err(trigger_errors) = Self::validate_trigger(trigger) {
                for error in trigger_errors {
                    errors.push(ValidationError {
                        field: format!("triggers[{}].{}", i, error.field),
                        message: error.message,
                    });
                }
            }
        }
        
        // Validate step dependencies
        Self::validate_dependencies(workflow, &mut errors);
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
    
    /// Validate a workflow step
    fn validate_step(step: &WorkflowStep) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();
        
        if step.name.is_empty() {
            errors.push(ValidationError {
                field: "name".to_string(),
                message: "Step name cannot be empty".to_string(),
            });
        }
        
        if step.id.is_empty() {
            errors.push(ValidationError {
                field: "id".to_string(),
                message: "Step ID cannot be empty".to_string(),
            });
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
    
    /// Validate a workflow trigger
    fn validate_trigger(trigger: &WorkflowTrigger) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();
        
        if trigger.name.is_empty() {
            errors.push(ValidationError {
                field: "name".to_string(),
                message: "Trigger name cannot be empty".to_string(),
            });
        }
        
        if trigger.id.is_empty() {
            errors.push(ValidationError {
                field: "id".to_string(),
                message: "Trigger ID cannot be empty".to_string(),
            });
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
    
    /// Validate step dependencies
    fn validate_dependencies(workflow: &WorkflowDefinition, errors: &mut Vec<ValidationError>) {
        let step_ids: std::collections::HashSet<String> = workflow.steps.iter()
            .map(|s| s.id.clone())
            .collect();
        
        for step in &workflow.steps {
            for dep in &step.dependencies {
                if !step_ids.contains(dep) {
                    errors.push(ValidationError {
                        field: format!("steps.{}.dependencies", step.id),
                        message: format!("Dependency '{}' not found", dep),
                    });
                }
            }
        }
        
        // Check for circular dependencies
        if let Err(cycle_error) = Self::check_circular_dependencies(workflow) {
            errors.push(cycle_error);
        }
    }
    
    /// Check for circular dependencies
    fn check_circular_dependencies(_workflow: &WorkflowDefinition) -> Result<(), ValidationError> {
        // TODO: Implement proper cycle detection algorithm
        Ok(())
    }
}

/// Validation error
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// Field that failed validation
    pub field: String,
    /// Error message
    pub message: String,
}

impl Default for WorkflowDefinition {
    fn default() -> Self {
        let now = Utc::now();
        
        Self {
            id: Some(Uuid::new_v4().to_string()),
            name: "New Workflow".to_string(),
            description: None,
            version: "1.0".to_string(),
            created_at: now,
            updated_at: now,
            active: true,
            steps: Vec::new(),
            triggers: Vec::new(),
            variables: HashMap::new(),
            settings: serde_json::json!({}),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_workflow_builder() {
        let workflow = WorkflowBuilder::new("Test Workflow")
            .description("A test workflow")
            .version("1.0")
            .active(true)
            .add_variable("test_var", serde_json::json!("test_value"))
            .build();
        
        assert_eq!(workflow.name, "Test Workflow");
        assert_eq!(workflow.description, Some("A test workflow".to_string()));
        assert_eq!(workflow.version, "1.0");
        assert!(workflow.active);
        assert_eq!(workflow.variables.get("test_var"), Some(&serde_json::json!("test_value")));
    }
    
    #[test]
    fn test_step_builder() {
        let step = StepBuilder::new("Test Step", StepType::HttpRequest)
            .config(serde_json::json!({"url": "https://example.com"}))
            .position(100.0, 200.0)
            .depends_on("previous_step")
            .timeout(30)
            .enabled(true)
            .build();
        
        assert_eq!(step.name, "Test Step");
        assert!(matches!(step.step_type, StepType::HttpRequest));
        assert_eq!(step.config["url"], "https://example.com");
        assert_eq!(step.position, Some(Position { x: 100.0, y: 200.0 }));
        assert_eq!(step.dependencies, vec!["previous_step"]);
        assert_eq!(step.timeout, Some(30));
        assert!(step.enabled);
    }
    
    #[test]
    fn test_workflow_validation() {
        let workflow = WorkflowBuilder::new("Valid Workflow")
            .description("A valid workflow")
            .build();
        
        assert!(WorkflowValidator::validate(&workflow).is_ok());
    }
    
    #[test]
    fn test_workflow_validation_errors() {
        let workflow = WorkflowBuilder::new("")
            .version("")
            .build();
        
        let errors = WorkflowValidator::validate(&workflow).unwrap_err();
        assert_eq!(errors.len(), 2);
        assert!(errors.iter().any(|e| e.field == "name"));
        assert!(errors.iter().any(|e| e.field == "version"));
    }
    
    #[test]
    fn test_step_types_serialization() {
        let step_types = vec![
            StepType::HttpRequest,
            StepType::BrowserAutomation,
            StepType::UIAutomation,
            StepType::Custom("custom_type".to_string()),
        ];
        
        for step_type in step_types {
            let serialized = serde_json::to_string(&step_type).unwrap();
            let deserialized: StepType = serde_json::from_str(&serialized).unwrap();
            assert_eq!(
                std::mem::discriminant(&step_type),
                std::mem::discriminant(&deserialized)
            );
        }
    }
}