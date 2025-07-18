//! Agent task decomposition engine
//! 
//! This module provides intelligent task decomposition capabilities for breaking down
//! complex tasks into smaller, manageable subtasks that can be executed by different agents.

use crate::AIError;
use crate::agents::langchain::{AgentTask, TaskType, TaskStatus};
use crate::agents::coordinator::{SubTask, DecompositionStrategy};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use chrono::{DateTime, Utc};
use log::{info, debug};

/// Task decomposition configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecompositionConfig {
    /// Enable task decomposition
    pub enabled: bool,
    /// Maximum decomposition depth
    pub max_depth: u32,
    /// Minimum task complexity for decomposition
    pub min_complexity_threshold: f32,
    /// Maximum subtasks per decomposition
    pub max_subtasks: u32,
    /// Enable parallel decomposition
    pub enable_parallel: bool,
    /// Enable sequential decomposition
    pub enable_sequential: bool,
    /// Enable hierarchical decomposition
    pub enable_hierarchical: bool,
    /// Enable data-driven decomposition
    pub enable_data_driven: bool,
    /// Decomposition timeout (seconds)
    pub decomposition_timeout: u64,
    /// Use LLM for decomposition
    pub use_llm_decomposition: bool,
    /// LLM model for decomposition
    pub llm_model: String,
    /// Enable dependency analysis
    pub enable_dependency_analysis: bool,
    /// Enable capability matching
    pub enable_capability_matching: bool,
}

impl Default for DecompositionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_depth: 5,
            min_complexity_threshold: 0.3,
            max_subtasks: 10,
            enable_parallel: true,
            enable_sequential: true,
            enable_hierarchical: true,
            enable_data_driven: true,
            decomposition_timeout: 60,
            use_llm_decomposition: true,
            llm_model: "llama3.2:latest".to_string(),
            enable_dependency_analysis: true,
            enable_capability_matching: true,
        }
    }
}

/// Task complexity metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskComplexity {
    /// Complexity score (0.0 to 1.0)
    pub score: f32,
    /// Input complexity
    pub input_complexity: f32,
    /// Processing complexity
    pub processing_complexity: f32,
    /// Output complexity
    pub output_complexity: f32,
    /// Interdependency complexity
    pub interdependency_complexity: f32,
    /// Temporal complexity
    pub temporal_complexity: f32,
    /// Resource complexity
    pub resource_complexity: f32,
    /// Complexity factors
    pub factors: Vec<ComplexityFactor>,
}

/// Complexity factor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityFactor {
    /// Factor name
    pub name: String,
    /// Factor weight
    pub weight: f32,
    /// Factor value
    pub value: f32,
    /// Factor description
    pub description: String,
}

/// Decomposition pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecompositionPattern {
    /// Pattern name
    pub name: String,
    /// Pattern description
    pub description: String,
    /// Pattern type
    pub pattern_type: PatternType,
    /// Applicable task types
    pub applicable_task_types: Vec<TaskType>,
    /// Pattern rules
    pub rules: Vec<DecompositionRule>,
    /// Pattern metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Pattern type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternType {
    /// Functional decomposition pattern
    Functional,
    /// Data flow decomposition pattern
    DataFlow,
    /// Pipeline decomposition pattern
    Pipeline,
    /// Tree decomposition pattern
    Tree,
    /// Graph decomposition pattern
    Graph,
    /// Template-based pattern
    Template,
}

/// Decomposition rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecompositionRule {
    /// Rule name
    pub name: String,
    /// Rule condition
    pub condition: String,
    /// Rule action
    pub action: String,
    /// Rule priority
    pub priority: u32,
    /// Rule metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Decomposition result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecompositionResult {
    /// Original task ID
    pub original_task_id: String,
    /// Decomposition success
    pub success: bool,
    /// Decomposition strategy used
    pub strategy: DecompositionStrategy,
    /// Generated subtasks
    pub subtasks: Vec<SubTask>,
    /// Subtask dependencies
    pub dependencies: HashMap<String, Vec<String>>,
    /// Decomposition metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Decomposition time
    pub decomposition_time: std::time::Duration,
    /// Decomposition error
    pub error: Option<String>,
}

/// Dependency analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyAnalysis {
    /// Task dependencies
    pub dependencies: HashMap<String, Vec<String>>,
    /// Critical path
    pub critical_path: Vec<String>,
    /// Parallel execution groups
    pub parallel_groups: Vec<Vec<String>>,
    /// Dependency conflicts
    pub conflicts: Vec<DependencyConflict>,
    /// Analysis metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Dependency conflict
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyConflict {
    /// Conflicting tasks
    pub tasks: Vec<String>,
    /// Conflict type
    pub conflict_type: ConflictType,
    /// Conflict description
    pub description: String,
    /// Suggested resolution
    pub resolution: String,
}

/// Conflict type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictType {
    /// Circular dependency
    Circular,
    /// Resource contention
    Resource,
    /// Data dependency
    Data,
    /// Temporal conflict
    Temporal,
}

/// Task decomposition engine
pub struct TaskDecompositionEngine {
    /// Configuration
    config: Arc<RwLock<DecompositionConfig>>,
    /// Decomposition patterns
    patterns: Arc<RwLock<HashMap<String, DecompositionPattern>>>,
    /// Decomposition cache
    cache: Arc<RwLock<HashMap<String, DecompositionResult>>>,
    /// Engine statistics
    stats: Arc<RwLock<DecompositionStats>>,
}

/// Decomposition statistics
#[derive(Debug, Default)]
pub struct DecompositionStats {
    /// Total decompositions
    total_decompositions: u64,
    /// Successful decompositions
    successful_decompositions: u64,
    /// Failed decompositions
    failed_decompositions: u64,
    /// Average decomposition time
    avg_decomposition_time: f64,
    /// Total subtasks generated
    total_subtasks_generated: u64,
    /// Cache hits
    cache_hits: u64,
    /// Cache misses
    cache_misses: u64,
    /// Last decomposition time
    last_decomposition: Option<DateTime<Utc>>,
}

impl TaskDecompositionEngine {
    /// Create a new task decomposition engine
    pub async fn new(config: DecompositionConfig) -> Result<Self, AIError> {
        let engine = Self {
            config: Arc::new(RwLock::new(config)),
            patterns: Arc::new(RwLock::new(HashMap::new())),
            cache: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(DecompositionStats::default())),
        };
        
        // Load default patterns
        engine.load_default_patterns().await?;
        
        info!("Task decomposition engine initialized");
        Ok(engine)
    }
    
    /// Decompose a task
    pub async fn decompose_task(&self, task: &AgentTask) -> Result<DecompositionResult, AIError> {
        let start_time = std::time::Instant::now();
        
        // Check cache first
        if let Some(cached_result) = self.cache.read().get(&task.id) {
            let mut stats = self.stats.write();
            stats.cache_hits += 1;
            return Ok(cached_result.clone());
        }
        
        let mut stats = self.stats.write();
        stats.cache_misses += 1;
        stats.total_decompositions += 1;
        drop(stats);
        
        // Analyze task complexity
        let complexity = self.analyze_task_complexity(task).await?;
        
        // Check if decomposition is needed
        if complexity.score < self.config.read().min_complexity_threshold {
            return Ok(DecompositionResult {
                original_task_id: task.id.clone(),
                success: true,
                strategy: DecompositionStrategy::None,
                subtasks: Vec::new(),
                dependencies: HashMap::new(),
                metadata: HashMap::new(),
                decomposition_time: start_time.elapsed(),
                error: None,
            });
        }
        
        // Select decomposition strategy
        let strategy = self.select_decomposition_strategy(task, &complexity).await?;
        
        // Perform decomposition
        let result = match strategy {
            DecompositionStrategy::Functional => self.functional_decomposition(task).await,
            DecompositionStrategy::Data => self.data_decomposition(task).await,
            DecompositionStrategy::Pipeline => self.pipeline_decomposition(task).await,
            DecompositionStrategy::Hierarchical => self.hierarchical_decomposition(task).await,
            DecompositionStrategy::None => Ok(DecompositionResult {
                original_task_id: task.id.clone(),
                success: true,
                strategy: DecompositionStrategy::None,
                subtasks: Vec::new(),
                dependencies: HashMap::new(),
                metadata: HashMap::new(),
                decomposition_time: start_time.elapsed(),
                error: None,
            }),
        };
        
        let mut decomposition_result = result?;
        decomposition_result.decomposition_time = start_time.elapsed();
        
        // Cache the result
        self.cache.write().insert(task.id.clone(), decomposition_result.clone());
        
        // Update statistics
        let mut stats = self.stats.write();
        if decomposition_result.success {
            stats.successful_decompositions += 1;
            stats.total_subtasks_generated += decomposition_result.subtasks.len() as u64;
        } else {
            stats.failed_decompositions += 1;
        }
        stats.avg_decomposition_time = (stats.avg_decomposition_time * (stats.total_decompositions - 1) as f64 + decomposition_result.decomposition_time.as_secs_f64()) / stats.total_decompositions as f64;
        stats.last_decomposition = Some(Utc::now());
        
        info!("Task decomposed: {} -> {} subtasks", task.id, decomposition_result.subtasks.len());
        Ok(decomposition_result)
    }
    
    /// Analyze task complexity
    pub async fn analyze_task_complexity(&self, task: &AgentTask) -> Result<TaskComplexity, AIError> {
        let mut factors = Vec::new();
        
        // Analyze input complexity
        let input_complexity = self.analyze_input_complexity(task).await?;
        factors.push(ComplexityFactor {
            name: "input".to_string(),
            weight: 0.2,
            value: input_complexity,
            description: "Input data complexity".to_string(),
        });
        
        // Analyze processing complexity
        let processing_complexity = self.analyze_processing_complexity(task).await?;
        factors.push(ComplexityFactor {
            name: "processing".to_string(),
            weight: 0.3,
            value: processing_complexity,
            description: "Processing logic complexity".to_string(),
        });
        
        // Analyze output complexity
        let output_complexity = self.analyze_output_complexity(task).await?;
        factors.push(ComplexityFactor {
            name: "output".to_string(),
            weight: 0.15,
            value: output_complexity,
            description: "Output format complexity".to_string(),
        });
        
        // Analyze interdependency complexity
        let interdependency_complexity = self.analyze_interdependency_complexity(task).await?;
        factors.push(ComplexityFactor {
            name: "interdependency".to_string(),
            weight: 0.2,
            value: interdependency_complexity,
            description: "Task interdependency complexity".to_string(),
        });
        
        // Analyze temporal complexity
        let temporal_complexity = self.analyze_temporal_complexity(task).await?;
        factors.push(ComplexityFactor {
            name: "temporal".to_string(),
            weight: 0.1,
            value: temporal_complexity,
            description: "Temporal constraint complexity".to_string(),
        });
        
        // Analyze resource complexity
        let resource_complexity = self.analyze_resource_complexity(task).await?;
        factors.push(ComplexityFactor {
            name: "resource".to_string(),
            weight: 0.05,
            value: resource_complexity,
            description: "Resource requirement complexity".to_string(),
        });
        
        // Calculate overall complexity score
        let score = factors.iter()
            .map(|f| f.weight * f.value)
            .sum::<f32>();
        
        Ok(TaskComplexity {
            score,
            input_complexity,
            processing_complexity,
            output_complexity,
            interdependency_complexity,
            temporal_complexity,
            resource_complexity,
            factors,
        })
    }
    
    /// Analyze dependencies between subtasks
    pub async fn analyze_dependencies(&self, subtasks: &[SubTask]) -> Result<DependencyAnalysis, AIError> {
        let mut dependencies = HashMap::new();
        let mut conflicts = Vec::new();
        
        // Analyze data dependencies
        for subtask in subtasks {
            let mut task_deps = Vec::new();
            
            // Check for data dependencies
            for other_subtask in subtasks {
                if subtask.id != other_subtask.id {
                    if self.has_data_dependency(subtask, other_subtask).await? {
                        task_deps.push(other_subtask.id.clone());
                    }
                }
            }
            
            dependencies.insert(subtask.id.clone(), task_deps);
        }
        
        // Detect circular dependencies
        self.detect_circular_dependencies(&dependencies, &mut conflicts).await?;
        
        // Calculate critical path
        let critical_path = self.calculate_critical_path(&dependencies).await?;
        
        // Identify parallel execution groups
        let parallel_groups = self.identify_parallel_groups(&dependencies).await?;
        
        Ok(DependencyAnalysis {
            dependencies,
            critical_path,
            parallel_groups,
            conflicts,
            metadata: HashMap::new(),
        })
    }
    
    /// Update configuration
    pub async fn update_config(&self, new_config: DecompositionConfig) -> Result<(), AIError> {
        *self.config.write() = new_config;
        info!("Decomposition engine configuration updated");
        Ok(())
    }
    
    /// Add decomposition pattern
    pub async fn add_pattern(&self, pattern: DecompositionPattern) -> Result<(), AIError> {
        self.patterns.write().insert(pattern.name.clone(), pattern.clone());
        info!("Decomposition pattern added: {}", pattern.name);
        Ok(())
    }
    
    /// Clear decomposition cache
    pub async fn clear_cache(&self) -> Result<(), AIError> {
        self.cache.write().clear();
        info!("Decomposition cache cleared");
        Ok(())
    }
    
    /// Get statistics
    pub fn get_stats(&self) -> DecompositionStats {
        self.stats.read().clone()
    }
    
    /// Load default decomposition patterns
    async fn load_default_patterns(&self) -> Result<(), AIError> {
        // Load functional decomposition pattern
        let functional_pattern = DecompositionPattern {
            name: "functional".to_string(),
            description: "Decompose based on functional requirements".to_string(),
            pattern_type: PatternType::Functional,
            applicable_task_types: vec![TaskType::CodeGeneration, TaskType::SystemAdmin],
            rules: vec![
                DecompositionRule {
                    name: "separate_concerns".to_string(),
                    condition: "task_has_multiple_functions".to_string(),
                    action: "create_subtask_per_function".to_string(),
                    priority: 1,
                    metadata: HashMap::new(),
                },
            ],
            metadata: HashMap::new(),
        };
        
        self.patterns.write().insert("functional".to_string(), functional_pattern);
        
        // Load data flow pattern
        let data_flow_pattern = DecompositionPattern {
            name: "data_flow".to_string(),
            description: "Decompose based on data flow".to_string(),
            pattern_type: PatternType::DataFlow,
            applicable_task_types: vec![TaskType::DataAnalysis],
            rules: vec![
                DecompositionRule {
                    name: "data_pipeline".to_string(),
                    condition: "task_has_data_pipeline".to_string(),
                    action: "create_subtask_per_stage".to_string(),
                    priority: 1,
                    metadata: HashMap::new(),
                },
            ],
            metadata: HashMap::new(),
        };
        
        self.patterns.write().insert("data_flow".to_string(), data_flow_pattern);
        
        info!("Default decomposition patterns loaded");
        Ok(())
    }
    
    /// Select decomposition strategy
    async fn select_decomposition_strategy(&self, task: &AgentTask, complexity: &TaskComplexity) -> Result<DecompositionStrategy, AIError> {
        match task.task_type {
            TaskType::CodeGeneration => Ok(DecompositionStrategy::Functional),
            TaskType::DataAnalysis => Ok(DecompositionStrategy::Data),
            TaskType::WebAutomation => Ok(DecompositionStrategy::Pipeline),
            TaskType::SystemAdmin => Ok(DecompositionStrategy::Hierarchical),
            _ => Ok(DecompositionStrategy::Functional),
        }
    }
    
    /// Functional decomposition
    async fn functional_decomposition(&self, task: &AgentTask) -> Result<DecompositionResult, AIError> {
        let mut subtasks = Vec::new();
        
        // TODO: Implement actual functional decomposition logic
        // This would involve analyzing the task requirements and breaking them down
        // into functional components
        
        subtasks.push(SubTask {
            id: format!("{}-func-1", task.id),
            description: "Functional subtask 1".to_string(),
            required_capabilities: vec!["basic_reasoning".to_string()],
            priority: task.priority.clone(),
            input: task.input.clone(),
            output: None,
            status: TaskStatus::Queued,
            assigned_agent: None,
            metadata: HashMap::new(),
        });
        
        Ok(DecompositionResult {
            original_task_id: task.id.clone(),
            success: true,
            strategy: DecompositionStrategy::Functional,
            subtasks,
            dependencies: HashMap::new(),
            metadata: HashMap::new(),
            decomposition_time: std::time::Duration::from_millis(0),
            error: None,
        })
    }
    
    /// Data decomposition
    async fn data_decomposition(&self, task: &AgentTask) -> Result<DecompositionResult, AIError> {
        let mut subtasks = Vec::new();
        
        // TODO: Implement actual data decomposition logic
        
        subtasks.push(SubTask {
            id: format!("{}-data-1", task.id),
            description: "Data processing subtask 1".to_string(),
            required_capabilities: vec!["data_processing".to_string()],
            priority: task.priority.clone(),
            input: task.input.clone(),
            output: None,
            status: TaskStatus::Queued,
            assigned_agent: None,
            metadata: HashMap::new(),
        });
        
        Ok(DecompositionResult {
            original_task_id: task.id.clone(),
            success: true,
            strategy: DecompositionStrategy::Data,
            subtasks,
            dependencies: HashMap::new(),
            metadata: HashMap::new(),
            decomposition_time: std::time::Duration::from_millis(0),
            error: None,
        })
    }
    
    /// Pipeline decomposition
    async fn pipeline_decomposition(&self, task: &AgentTask) -> Result<DecompositionResult, AIError> {
        let mut subtasks = Vec::new();
        
        // TODO: Implement actual pipeline decomposition logic
        
        subtasks.push(SubTask {
            id: format!("{}-pipe-1", task.id),
            description: "Pipeline stage 1".to_string(),
            required_capabilities: vec!["web_automation".to_string()],
            priority: task.priority.clone(),
            input: task.input.clone(),
            output: None,
            status: TaskStatus::Queued,
            assigned_agent: None,
            metadata: HashMap::new(),
        });
        
        Ok(DecompositionResult {
            original_task_id: task.id.clone(),
            success: true,
            strategy: DecompositionStrategy::Pipeline,
            subtasks,
            dependencies: HashMap::new(),
            metadata: HashMap::new(),
            decomposition_time: std::time::Duration::from_millis(0),
            error: None,
        })
    }
    
    /// Hierarchical decomposition
    async fn hierarchical_decomposition(&self, task: &AgentTask) -> Result<DecompositionResult, AIError> {
        let mut subtasks = Vec::new();
        
        // TODO: Implement actual hierarchical decomposition logic
        
        subtasks.push(SubTask {
            id: format!("{}-hier-1", task.id),
            description: "Hierarchical subtask 1".to_string(),
            required_capabilities: vec!["system_admin".to_string()],
            priority: task.priority.clone(),
            input: task.input.clone(),
            output: None,
            status: TaskStatus::Queued,
            assigned_agent: None,
            metadata: HashMap::new(),
        });
        
        Ok(DecompositionResult {
            original_task_id: task.id.clone(),
            success: true,
            strategy: DecompositionStrategy::Hierarchical,
            subtasks,
            dependencies: HashMap::new(),
            metadata: HashMap::new(),
            decomposition_time: std::time::Duration::from_millis(0),
            error: None,
        })
    }
    
    /// Analyze input complexity
    async fn analyze_input_complexity(&self, task: &AgentTask) -> Result<f32, AIError> {
        // TODO: Implement actual input complexity analysis
        Ok(0.5)
    }
    
    /// Analyze processing complexity
    async fn analyze_processing_complexity(&self, task: &AgentTask) -> Result<f32, AIError> {
        // TODO: Implement actual processing complexity analysis
        Ok(0.7)
    }
    
    /// Analyze output complexity
    async fn analyze_output_complexity(&self, task: &AgentTask) -> Result<f32, AIError> {
        // TODO: Implement actual output complexity analysis
        Ok(0.3)
    }
    
    /// Analyze interdependency complexity
    async fn analyze_interdependency_complexity(&self, task: &AgentTask) -> Result<f32, AIError> {
        // TODO: Implement actual interdependency analysis
        Ok(0.4)
    }
    
    /// Analyze temporal complexity
    async fn analyze_temporal_complexity(&self, task: &AgentTask) -> Result<f32, AIError> {
        // TODO: Implement actual temporal complexity analysis
        Ok(0.2)
    }
    
    /// Analyze resource complexity
    async fn analyze_resource_complexity(&self, task: &AgentTask) -> Result<f32, AIError> {
        // TODO: Implement actual resource complexity analysis
        Ok(0.3)
    }
    
    /// Check if subtask has data dependency on another
    async fn has_data_dependency(&self, subtask: &SubTask, other_subtask: &SubTask) -> Result<bool, AIError> {
        // TODO: Implement actual data dependency analysis
        Ok(false)
    }
    
    /// Detect circular dependencies
    async fn detect_circular_dependencies(&self, dependencies: &HashMap<String, Vec<String>>, conflicts: &mut Vec<DependencyConflict>) -> Result<(), AIError> {
        // TODO: Implement circular dependency detection
        Ok(())
    }
    
    /// Calculate critical path
    async fn calculate_critical_path(&self, dependencies: &HashMap<String, Vec<String>>) -> Result<Vec<String>, AIError> {
        // TODO: Implement critical path calculation
        Ok(Vec::new())
    }
    
    /// Identify parallel execution groups
    async fn identify_parallel_groups(&self, dependencies: &HashMap<String, Vec<String>>) -> Result<Vec<Vec<String>>, AIError> {
        // TODO: Implement parallel group identification
        Ok(Vec::new())
    }
}

impl Clone for DecompositionStats {
    fn clone(&self) -> Self {
        Self {
            total_decompositions: self.total_decompositions,
            successful_decompositions: self.successful_decompositions,
            failed_decompositions: self.failed_decompositions,
            avg_decomposition_time: self.avg_decomposition_time,
            total_subtasks_generated: self.total_subtasks_generated,
            cache_hits: self.cache_hits,
            cache_misses: self.cache_misses,
            last_decomposition: self.last_decomposition,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    
    #[test]
    fn test_decomposition_config_default() {
        let config = DecompositionConfig::default();
        assert!(config.enabled);
        assert_eq!(config.max_depth, 5);
        assert_eq!(config.min_complexity_threshold, 0.3);
        assert_eq!(config.max_subtasks, 10);
        assert!(config.enable_parallel);
        assert!(config.enable_sequential);
        assert!(config.enable_hierarchical);
    }
    
    #[test]
    fn test_task_complexity_calculation() {
        let complexity = TaskComplexity {
            score: 0.6,
            input_complexity: 0.5,
            processing_complexity: 0.7,
            output_complexity: 0.3,
            interdependency_complexity: 0.4,
            temporal_complexity: 0.2,
            resource_complexity: 0.3,
            factors: vec![
                ComplexityFactor {
                    name: "input".to_string(),
                    weight: 0.2,
                    value: 0.5,
                    description: "Input complexity".to_string(),
                },
            ],
        };
        
        assert_eq!(complexity.score, 0.6);
        assert_eq!(complexity.input_complexity, 0.5);
        assert_eq!(complexity.processing_complexity, 0.7);
        assert_eq!(complexity.factors.len(), 1);
    }
    
    #[test]
    fn test_decomposition_pattern() {
        let pattern = DecompositionPattern {
            name: "test_pattern".to_string(),
            description: "Test decomposition pattern".to_string(),
            pattern_type: PatternType::Functional,
            applicable_task_types: vec![TaskType::CodeGeneration],
            rules: vec![
                DecompositionRule {
                    name: "test_rule".to_string(),
                    condition: "test_condition".to_string(),
                    action: "test_action".to_string(),
                    priority: 1,
                    metadata: HashMap::new(),
                },
            ],
            metadata: HashMap::new(),
        };
        
        assert_eq!(pattern.name, "test_pattern");
        assert!(matches!(pattern.pattern_type, PatternType::Functional));
        assert_eq!(pattern.applicable_task_types.len(), 1);
        assert_eq!(pattern.rules.len(), 1);
    }
    
    #[test]
    fn test_dependency_analysis() {
        let mut dependencies = HashMap::new();
        dependencies.insert("task1".to_string(), vec!["task2".to_string()]);
        dependencies.insert("task2".to_string(), vec![]);
        
        let analysis = DependencyAnalysis {
            dependencies,
            critical_path: vec!["task2".to_string(), "task1".to_string()],
            parallel_groups: vec![vec!["task2".to_string()], vec!["task1".to_string()]],
            conflicts: Vec::new(),
            metadata: HashMap::new(),
        };
        
        assert_eq!(analysis.dependencies.len(), 2);
        assert_eq!(analysis.critical_path.len(), 2);
        assert_eq!(analysis.parallel_groups.len(), 2);
        assert_eq!(analysis.conflicts.len(), 0);
    }
}