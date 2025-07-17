//! Relationship analysis and discovery for graph edges

use crate::{GraphEdge};
use horizonos_graph_engine::{SceneId, EdgeType};
use horizonos_graph_nodes::{GraphNode};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

/// Analyzes relationships between nodes to determine edge types and strengths
pub struct RelationshipAnalyzer {
    relationship_rules: Vec<RelationshipRule>,
    temporal_window: chrono::Duration, // How far back to look for temporal relationships
}

/// A rule for discovering relationships between nodes
#[derive(Debug, Clone)]
pub struct RelationshipRule {
    pub name: String,
    pub edge_type: EdgeType,
    pub confidence_threshold: f32,
    pub rule_function: RelationshipRuleFunction,
}

/// Function type for relationship discovery rules
#[derive(Debug, Clone)]
pub enum RelationshipRuleFunction {
    FileParentChild,
    ApplicationFileAccess,
    CoLocationTemporal,
    SimilarNames,
    TagSimilarity,
    ProcessHierarchy,
    ContentSimilarity,
    UserWorkflow,
}

/// Result of relationship analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipAnalysis {
    pub source_id: SceneId,
    pub target_id: SceneId,
    pub suggested_edge_type: EdgeType,
    pub confidence: f32,
    pub strength: f32,
    pub evidence: Vec<String>,
    pub bidirectional: bool,
}

/// Statistics about discovered relationships
#[derive(Debug, Clone)]
pub struct RelationshipStatistics {
    pub total_analyzed: usize,
    pub relationships_found: usize,
    pub high_confidence_count: usize,
    pub edge_type_distribution: HashMap<String, usize>,
    pub average_confidence: f32,
}

impl RelationshipAnalyzer {
    pub fn new() -> Self {
        let mut analyzer = RelationshipAnalyzer {
            relationship_rules: Vec::new(),
            temporal_window: chrono::Duration::hours(24),
        };
        
        analyzer.add_default_rules();
        analyzer
    }
    
    fn add_default_rules(&mut self) {
        // File parent-child relationships
        self.relationship_rules.push(RelationshipRule {
            name: "File Parent-Child".to_string(),
            edge_type: EdgeType::Contains,
            confidence_threshold: 0.9,
            rule_function: RelationshipRuleFunction::FileParentChild,
        });
        
        // Application-file access patterns
        self.relationship_rules.push(RelationshipRule {
            name: "Application File Access".to_string(),
            edge_type: EdgeType::WorksOn,
            confidence_threshold: 0.7,
            rule_function: RelationshipRuleFunction::ApplicationFileAccess,
        });
        
        // Co-location in time (temporal proximity)
        self.relationship_rules.push(RelationshipRule {
            name: "Temporal Co-location".to_string(),
            edge_type: EdgeType::Temporal { sequence_order: 0 },
            confidence_threshold: 0.6,
            rule_function: RelationshipRuleFunction::CoLocationTemporal,
        });
        
        // Similar names/content
        self.relationship_rules.push(RelationshipRule {
            name: "Name Similarity".to_string(),
            edge_type: EdgeType::RelatedTo { similarity: 0.0 },
            confidence_threshold: 0.5,
            rule_function: RelationshipRuleFunction::SimilarNames,
        });
        
        // Tag-based relationships
        self.relationship_rules.push(RelationshipRule {
            name: "Tag Similarity".to_string(),
            edge_type: EdgeType::TaggedAs { tag: String::new() },
            confidence_threshold: 0.8,
            rule_function: RelationshipRuleFunction::TagSimilarity,
        });
    }
    
    /// Analyze potential relationships between two nodes
    pub fn analyze_relationship(&self, source: &dyn GraphNode, target: &dyn GraphNode) -> Option<RelationshipAnalysis> {
        for rule in &self.relationship_rules {
            if let Some(analysis) = self.apply_rule(rule, source, target) {
                if analysis.confidence >= rule.confidence_threshold {
                    return Some(analysis);
                }
            }
        }
        None
    }
    
    /// Analyze relationships across a set of nodes
    pub fn analyze_relationships(&self, nodes: &[&dyn GraphNode]) -> Vec<RelationshipAnalysis> {
        let mut relationships = Vec::new();
        
        for i in 0..nodes.len() {
            for j in (i + 1)..nodes.len() {
                if let Some(analysis) = self.analyze_relationship(nodes[i], nodes[j]) {
                    relationships.push(analysis);
                }
                
                // Check reverse direction for non-symmetric relationships
                if let Some(analysis) = self.analyze_relationship(nodes[j], nodes[i]) {
                    if !analysis.bidirectional {
                        relationships.push(analysis);
                    }
                }
            }
        }
        
        relationships
    }
    
    fn apply_rule(&self, rule: &RelationshipRule, source: &dyn GraphNode, target: &dyn GraphNode) -> Option<RelationshipAnalysis> {
        match &rule.rule_function {
            RelationshipRuleFunction::FileParentChild => {
                self.analyze_file_parent_child(source, target, &rule.edge_type)
            }
            RelationshipRuleFunction::ApplicationFileAccess => {
                self.analyze_app_file_access(source, target, &rule.edge_type)
            }
            RelationshipRuleFunction::CoLocationTemporal => {
                self.analyze_temporal_colocation(source, target, &rule.edge_type)
            }
            RelationshipRuleFunction::SimilarNames => {
                self.analyze_name_similarity(source, target, &rule.edge_type)
            }
            RelationshipRuleFunction::TagSimilarity => {
                self.analyze_tag_similarity(source, target, &rule.edge_type)
            }
            RelationshipRuleFunction::ProcessHierarchy => {
                self.analyze_process_hierarchy(source, target, &rule.edge_type)
            }
            RelationshipRuleFunction::ContentSimilarity => {
                self.analyze_content_similarity(source, target, &rule.edge_type)
            }
            RelationshipRuleFunction::UserWorkflow => {
                self.analyze_user_workflow(source, target, &rule.edge_type)
            }
        }
    }
    
    fn analyze_file_parent_child(&self, source: &dyn GraphNode, target: &dyn GraphNode, edge_type: &EdgeType) -> Option<RelationshipAnalysis> {
        // This would require access to the actual node types
        // For now, return a placeholder implementation
        let source_name = source.display_name();
        let target_name = target.display_name();
        
        // Simple heuristic: if one name is a prefix of another with separator
        if source_name.len() < target_name.len() && target_name.starts_with(&source_name) {
            let remainder = &target_name[source_name.len()..];
            if remainder.starts_with('/') || remainder.starts_with('\\') {
                return Some(RelationshipAnalysis {
                    source_id: source.id(),
                    target_id: target.id(),
                    suggested_edge_type: edge_type.clone(),
                    confidence: 0.95,
                    strength: 0.9,
                    evidence: vec!["File path hierarchy detected".to_string()],
                    bidirectional: false,
                });
            }
        }
        
        None
    }
    
    fn analyze_app_file_access(&self, source: &dyn GraphNode, target: &dyn GraphNode, edge_type: &EdgeType) -> Option<RelationshipAnalysis> {
        // Check if source is application and target is file
        // This would require more sophisticated type checking in a real implementation
        let source_desc = source.description().unwrap_or_default();
        let target_desc = target.description().unwrap_or_default();
        
        if source_desc.contains("Application") && target_desc.contains("File") {
            // Heuristic: applications often work on files with similar names
            let source_name = source.display_name().to_lowercase();
            let target_name = target.display_name().to_lowercase();
            
            let similarity = self.calculate_string_similarity(&source_name, &target_name);
            if similarity > 0.3 {
                return Some(RelationshipAnalysis {
                    source_id: source.id(),
                    target_id: target.id(),
                    suggested_edge_type: edge_type.clone(),
                    confidence: similarity.min(0.8),
                    strength: similarity,
                    evidence: vec![format!("Application-file name similarity: {:.2}", similarity)],
                    bidirectional: false,
                });
            }
        }
        
        None
    }
    
    fn analyze_temporal_colocation(&self, source: &dyn GraphNode, target: &dyn GraphNode, edge_type: &EdgeType) -> Option<RelationshipAnalysis> {
        // This would analyze access times, creation times, etc.
        // For now, return a simple heuristic based on ID proximity (simulating temporal proximity)
        let id_diff = (source.id() as i64 - target.id() as i64).abs();
        
        if id_diff <= 5 { // IDs close together suggest temporal proximity
            let confidence = (6.0 - id_diff as f32) / 6.0;
            return Some(RelationshipAnalysis {
                source_id: source.id(),
                target_id: target.id(),
                suggested_edge_type: EdgeType::Temporal { sequence_order: id_diff as u32 },
                confidence,
                strength: confidence,
                evidence: vec![format!("Temporal proximity detected (ID diff: {})", id_diff)],
                bidirectional: true,
            });
        }
        
        None
    }
    
    fn analyze_name_similarity(&self, source: &dyn GraphNode, target: &dyn GraphNode, edge_type: &EdgeType) -> Option<RelationshipAnalysis> {
        let source_name = source.display_name().to_lowercase();
        let target_name = target.display_name().to_lowercase();
        
        let similarity = self.calculate_string_similarity(&source_name, &target_name);
        
        if similarity > 0.5 {
            return Some(RelationshipAnalysis {
                source_id: source.id(),
                target_id: target.id(),
                suggested_edge_type: EdgeType::RelatedTo { similarity },
                confidence: similarity,
                strength: similarity,
                evidence: vec![format!("Name similarity: {:.2}", similarity)],
                bidirectional: true,
            });
        }
        
        None
    }
    
    fn analyze_tag_similarity(&self, source: &dyn GraphNode, target: &dyn GraphNode, edge_type: &EdgeType) -> Option<RelationshipAnalysis> {
        // This would require access to node metadata/tags
        // For now, simple heuristic based on description content
        let source_desc = source.description().unwrap_or_default().to_lowercase();
        let target_desc = target.description().unwrap_or_default().to_lowercase();
        
        let common_words = self.find_common_words(&source_desc, &target_desc);
        
        if !common_words.is_empty() {
            let confidence = (common_words.len() as f32 / 10.0).min(1.0);
            return Some(RelationshipAnalysis {
                source_id: source.id(),
                target_id: target.id(),
                suggested_edge_type: EdgeType::TaggedAs { tag: common_words.join(",") },
                confidence,
                strength: confidence,
                evidence: vec![format!("Common tags: {}", common_words.join(", "))],
                bidirectional: true,
            });
        }
        
        None
    }
    
    fn analyze_process_hierarchy(&self, _source: &dyn GraphNode, _target: &dyn GraphNode, _edge_type: &EdgeType) -> Option<RelationshipAnalysis> {
        // Would analyze parent-child process relationships
        None
    }
    
    fn analyze_content_similarity(&self, _source: &dyn GraphNode, _target: &dyn GraphNode, _edge_type: &EdgeType) -> Option<RelationshipAnalysis> {
        // Would analyze file content, application functionality, etc.
        None
    }
    
    fn analyze_user_workflow(&self, _source: &dyn GraphNode, _target: &dyn GraphNode, _edge_type: &EdgeType) -> Option<RelationshipAnalysis> {
        // Would analyze user interaction patterns
        None
    }
    
    /// Calculate similarity between two strings using Levenshtein distance
    fn calculate_string_similarity(&self, s1: &str, s2: &str) -> f32 {
        if s1.is_empty() && s2.is_empty() {
            return 1.0;
        }
        
        let len1 = s1.chars().count();
        let len2 = s2.chars().count();
        
        if len1 == 0 || len2 == 0 {
            return 0.0;
        }
        
        let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];
        
        // Initialize first row and column
        for i in 0..=len1 {
            matrix[i][0] = i;
        }
        for j in 0..=len2 {
            matrix[0][j] = j;
        }
        
        let chars1: Vec<char> = s1.chars().collect();
        let chars2: Vec<char> = s2.chars().collect();
        
        // Fill the matrix
        for i in 1..=len1 {
            for j in 1..=len2 {
                let cost = if chars1[i - 1] == chars2[j - 1] { 0 } else { 1 };
                matrix[i][j] = (matrix[i - 1][j] + 1)
                    .min(matrix[i][j - 1] + 1)
                    .min(matrix[i - 1][j - 1] + cost);
            }
        }
        
        let distance = matrix[len1][len2];
        let max_len = len1.max(len2);
        
        1.0 - (distance as f32 / max_len as f32)
    }
    
    /// Find common words between two texts
    fn find_common_words(&self, text1: &str, text2: &str) -> Vec<String> {
        let words1: std::collections::HashSet<&str> = text1.split_whitespace().collect();
        let words2: std::collections::HashSet<&str> = text2.split_whitespace().collect();
        
        words1.intersection(&words2)
            .filter(|word| word.len() > 3) // Only meaningful words
            .map(|&word| word.to_string())
            .collect()
    }
    
    /// Generate statistics about relationship analysis
    pub fn generate_statistics(&self, analyses: &[RelationshipAnalysis]) -> RelationshipStatistics {
        let mut edge_type_distribution = HashMap::new();
        let mut total_confidence = 0.0;
        let mut high_confidence_count = 0;
        
        for analysis in analyses {
            let type_name = format!("{:?}", analysis.suggested_edge_type);
            *edge_type_distribution.entry(type_name).or_insert(0) += 1;
            
            total_confidence += analysis.confidence;
            if analysis.confidence > 0.8 {
                high_confidence_count += 1;
            }
        }
        
        let average_confidence = if !analyses.is_empty() {
            total_confidence / analyses.len() as f32
        } else {
            0.0
        };
        
        RelationshipStatistics {
            total_analyzed: analyses.len(),
            relationships_found: analyses.len(),
            high_confidence_count,
            edge_type_distribution,
            average_confidence,
        }
    }
    
    /// Create a graph edge from relationship analysis
    pub fn create_edge_from_analysis(&self, edge_id: SceneId, analysis: &RelationshipAnalysis) -> GraphEdge {
        let mut edge = GraphEdge::new(edge_id, analysis.source_id, analysis.target_id, analysis.suggested_edge_type.clone());
        
        edge.relationship_data.strength = analysis.strength;
        edge.relationship_data.confidence = analysis.confidence;
        edge.relationship_data.bidirectional = analysis.bidirectional;
        
        // Add evidence as properties
        for (i, evidence) in analysis.evidence.iter().enumerate() {
            edge.relationship_data.properties.insert(format!("evidence_{}", i), evidence.clone());
        }
        
        edge.metadata.description = Some(format!("Auto-discovered relationship: {}", analysis.evidence.join("; ")));
        edge.metadata.user_created = false;
        
        edge
    }
}

impl Default for RelationshipAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use horizonos_graph_nodes::{BaseNode, NodeVisualData, NodeAction, NodeActionResult, NodeActionType, NodeError, NodeExportData};
    use horizonos_graph_engine::{SceneNode, NodeType, Position, Vec3};
    
    // Mock node for testing
    struct MockNode {
        id: SceneId,
        name: String,
        description: String,
    }
    
    impl GraphNode for MockNode {
        fn id(&self) -> SceneId { self.id }
        fn display_name(&self) -> String { self.name.clone() }
        fn description(&self) -> Option<String> { Some(self.description.clone()) }
        fn visual_data(&self) -> NodeVisualData { NodeVisualData::default() }
        fn update(&mut self, _delta_time: f32) -> Result<(), NodeError> { Ok(()) }
        fn handle_action(&mut self, _action: NodeAction) -> Result<NodeActionResult, NodeError> { 
            Ok(NodeActionResult::Success { message: None }) 
        }
        fn available_actions(&self) -> Vec<NodeActionType> { Vec::new() }
        fn export_data(&self) -> Result<NodeExportData, NodeError> { 
            Ok(NodeExportData {
                node_type: "mock".to_string(),
                display_name: self.name.clone(),
                description: Some(self.description.clone()),
                visual_data: NodeVisualData::default(),
                metadata: horizonos_graph_engine::NodeMetadata::default(),
                type_specific_data: serde_json::Value::Null,
            })
        }
        fn to_scene_node(&self) -> SceneNode {
            SceneNode {
                id: self.id,
                position: Position::new(0.0, 0.0, 0.0),
                velocity: Vec3::zeros(),
                radius: 1.0,
                color: [1.0, 1.0, 1.0, 1.0],
                node_type: NodeType::System { component: "mock".to_string(), status: horizonos_graph_engine::SystemStatus::Running },
                metadata: horizonos_graph_engine::NodeMetadata::default(),
                visible: true,
                selected: false,
            }
        }
    }
    
    #[test]
    fn test_relationship_analyzer_creation() {
        let analyzer = RelationshipAnalyzer::new();
        assert!(!analyzer.relationship_rules.is_empty());
        assert_eq!(analyzer.temporal_window, chrono::Duration::hours(24));
    }
    
    #[test]
    fn test_string_similarity() {
        let analyzer = RelationshipAnalyzer::new();
        
        assert_eq!(analyzer.calculate_string_similarity("hello", "hello"), 1.0);
        assert_eq!(analyzer.calculate_string_similarity("", ""), 1.0);
        assert_eq!(analyzer.calculate_string_similarity("abc", ""), 0.0);
        assert!(analyzer.calculate_string_similarity("hello", "hallo") > 0.7);
    }
    
    #[test]
    fn test_name_similarity_analysis() {
        let analyzer = RelationshipAnalyzer::new();
        
        let node1 = MockNode {
            id: 1,
            name: "document.txt".to_string(),
            description: "A text file".to_string(),
        };
        
        let node2 = MockNode {
            id: 2,
            name: "document_backup.txt".to_string(),
            description: "A backup file".to_string(),
        };
        
        let analysis = analyzer.analyze_relationship(&node1, &node2);
        assert!(analysis.is_some());
        
        let analysis = analysis.unwrap();
        assert!(analysis.confidence > 0.5);
        assert!(matches!(analysis.suggested_edge_type, EdgeType::RelatedTo { .. }));
    }
    
    #[test]
    fn test_temporal_colocation() {
        let analyzer = RelationshipAnalyzer::new();
        
        let node1 = MockNode {
            id: 1,
            name: "node1".to_string(),
            description: "First node".to_string(),
        };
        
        let node2 = MockNode {
            id: 3,
            name: "node2".to_string(),
            description: "Second node".to_string(),
        };
        
        let analysis = analyzer.analyze_relationship(&node1, &node2);
        assert!(analysis.is_some());
        
        let analysis = analysis.unwrap();
        assert!(matches!(analysis.suggested_edge_type, EdgeType::Temporal { .. }));
        assert!(analysis.bidirectional);
    }
    
    #[test]
    fn test_statistics_generation() {
        let analyzer = RelationshipAnalyzer::new();
        
        let analyses = vec![
            RelationshipAnalysis {
                source_id: 1,
                target_id: 2,
                suggested_edge_type: EdgeType::Contains,
                confidence: 0.9,
                strength: 0.8,
                evidence: vec!["test".to_string()],
                bidirectional: false,
            },
            RelationshipAnalysis {
                source_id: 2,
                target_id: 3,
                suggested_edge_type: EdgeType::RelatedTo { similarity: 0.7 },
                confidence: 0.6,
                strength: 0.7,
                evidence: vec!["test2".to_string()],
                bidirectional: true,
            },
        ];
        
        let stats = analyzer.generate_statistics(&analyses);
        assert_eq!(stats.total_analyzed, 2);
        assert_eq!(stats.relationships_found, 2);
        assert_eq!(stats.high_confidence_count, 1);
        assert!(stats.edge_type_distribution.contains_key("Contains"));
    }
}