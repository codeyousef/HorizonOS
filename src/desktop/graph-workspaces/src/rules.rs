//! Workspace organization rules

use crate::Workspace;
use serde::{Deserialize, Serialize};

/// Workspace rules engine
pub struct WorkspaceRules {
    /// Active rule sets
    rule_sets: Vec<RuleSet>,
}

impl WorkspaceRules {
    /// Create new rules engine
    pub fn new() -> Self {
        Self {
            rule_sets: vec![
                RuleSet::default_organization_rules(),
                RuleSet::default_grouping_rules(),
            ],
        }
    }
    
    /// Add a custom rule set
    pub fn add_rule_set(&mut self, rule_set: RuleSet) {
        self.rule_sets.push(rule_set);
    }
    
    /// Apply all rules to a workspace
    pub fn apply_to_workspace(&self, workspace: &mut Workspace) {
        for rule_set in &self.rule_sets {
            if rule_set.enabled {
                self.apply_rule_set(workspace, rule_set);
            }
        }
    }
    
    /// Apply a specific rule set
    fn apply_rule_set(&self, workspace: &mut Workspace, rule_set: &RuleSet) {
        for rule in &rule_set.rules {
            match rule {
                Rule::GroupByType { node_types: _ } => {
                    // Group nodes by type
                    // This would reorganize the layout based on node types
                    log::debug!("Applying group by type rule");
                }
                Rule::SeparateByAge { days_threshold } => {
                    // Separate old nodes from recent ones
                    log::debug!("Applying separate by age rule: {} days", days_threshold);
                }
                Rule::ClusterRelated { max_distance } => {
                    // Cluster related nodes together
                    log::debug!("Applying cluster related rule: max distance {}", max_distance);
                }
                Rule::AutoArchive { inactive_days } => {
                    // Move inactive nodes to archive workspace
                    log::debug!("Applying auto-archive rule: {} days", inactive_days);
                }
                Rule::LimitNodeCount { max_nodes } => {
                    // Limit the number of nodes in a workspace
                    if workspace.nodes.len() > *max_nodes {
                        log::warn!("Workspace exceeds node limit: {} > {}", workspace.nodes.len(), max_nodes);
                    }
                }
            }
        }
    }
}

/// Rule set
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleSet {
    /// Rule set name
    pub name: String,
    /// Whether this rule set is enabled
    pub enabled: bool,
    /// Rules in this set
    pub rules: Vec<Rule>,
}

impl RuleSet {
    /// Create default organization rules
    pub fn default_organization_rules() -> Self {
        Self {
            name: "Default Organization".to_string(),
            enabled: true,
            rules: vec![
                Rule::GroupByType {
                    node_types: vec!["Application".to_string(), "File".to_string()],
                },
                Rule::SeparateByAge {
                    days_threshold: 30,
                },
            ],
        }
    }
    
    /// Create default grouping rules
    pub fn default_grouping_rules() -> Self {
        Self {
            name: "Default Grouping".to_string(),
            enabled: true,
            rules: vec![
                Rule::ClusterRelated {
                    max_distance: 200.0,
                },
                Rule::LimitNodeCount {
                    max_nodes: 100,
                },
            ],
        }
    }
}

/// Individual workspace rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Rule {
    /// Group nodes by type
    GroupByType {
        node_types: Vec<String>,
    },
    
    /// Separate nodes by age
    SeparateByAge {
        days_threshold: u32,
    },
    
    /// Cluster related nodes
    ClusterRelated {
        max_distance: f32,
    },
    
    /// Auto-archive inactive nodes
    AutoArchive {
        inactive_days: u32,
    },
    
    /// Limit node count
    LimitNodeCount {
        max_nodes: usize,
    },
}

/// Rule conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Condition {
    /// Node type matches
    NodeType(String),
    
    /// Node age exceeds threshold
    OlderThan { days: u32 },
    
    /// Node has specific metadata
    HasMetadata { key: String, value: serde_json::Value },
    
    /// Node name matches pattern
    NameMatches { pattern: String },
}

/// Rule actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
    /// Move to position
    MoveTo { x: f32, y: f32, z: f32 },
    
    /// Group with other nodes
    GroupWith { group_id: String },
    
    /// Apply visual style
    ApplyStyle { color: [f32; 4], size: f32 },
    
    /// Add metadata
    AddMetadata { key: String, value: serde_json::Value },
    
    /// Archive node
    Archive,
}