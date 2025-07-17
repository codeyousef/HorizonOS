//! Edge and relationship system for HorizonOS graph desktop
//! 
//! This module provides the implementation for managing relationships between nodes
//! in the graph desktop environment.

pub use horizonos_graph_engine::{SceneEdge, EdgeType, SceneId};

pub mod manager;
pub mod relationship;
pub mod discovery;

pub use manager::*;
pub use relationship::*;
pub use discovery::*;

use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Enhanced edge with additional relationship data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub id: SceneId,
    pub source: SceneId,
    pub target: SceneId,
    pub edge_type: EdgeType,
    pub relationship_data: RelationshipData,
    pub visual_style: EdgeVisualStyle,
    pub metadata: EdgeMetadata,
}

/// Data describing the relationship between nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipData {
    pub strength: f32,           // 0.0 to 1.0
    pub confidence: f32,         // 0.0 to 1.0 (how certain we are about this relationship)
    pub frequency: u32,          // How often this relationship is used/accessed
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_accessed: chrono::DateTime<chrono::Utc>,
    pub bidirectional: bool,     // Can the relationship work both ways?
    pub properties: HashMap<String, String>, // Additional key-value properties
}

/// Visual styling for edges
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeVisualStyle {
    pub color: [f32; 4],         // RGBA
    pub thickness: f32,
    pub opacity: f32,
    pub dash_pattern: Option<Vec<f32>>, // For dashed lines
    pub animation_speed: f32,    // For animated edges
    pub glow: bool,
    pub visible: bool,
}

/// Metadata for edges
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeMetadata {
    pub labels: Vec<String>,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub user_created: bool,      // vs automatically discovered
    pub pinned: bool,            // User wants to keep this relationship visible
    pub temporary: bool,         // Should expire after some time
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Errors that can occur during edge operations
#[derive(Debug)]
pub enum EdgeError {
    EdgeNotFound { id: SceneId },
    InvalidRelationship { source: SceneId, target: SceneId },
    CircularDependency,
    MaxEdgesExceeded { node_id: SceneId },
    SerializationError(serde_json::Error),
    SystemError { message: String },
}

impl std::fmt::Display for EdgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EdgeError::EdgeNotFound { id } => write!(f, "Edge not found: {}", id),
            EdgeError::InvalidRelationship { source, target } => write!(f, "Invalid relationship: {} -> {}", source, target),
            EdgeError::CircularDependency => write!(f, "Circular dependency detected"),
            EdgeError::MaxEdgesExceeded { node_id } => write!(f, "Maximum edges exceeded for node: {}", node_id),
            EdgeError::SerializationError(e) => write!(f, "Serialization error: {}", e),
            EdgeError::SystemError { message } => write!(f, "System error: {}", message),
        }
    }
}

impl std::error::Error for EdgeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            EdgeError::SerializationError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<serde_json::Error> for EdgeError {
    fn from(err: serde_json::Error) -> Self {
        EdgeError::SerializationError(err)
    }
}

impl Default for RelationshipData {
    fn default() -> Self {
        let now = chrono::Utc::now();
        RelationshipData {
            strength: 0.5,
            confidence: 0.8,
            frequency: 1,
            created_at: now,
            last_accessed: now,
            bidirectional: false,
            properties: HashMap::new(),
        }
    }
}

impl Default for EdgeVisualStyle {
    fn default() -> Self {
        EdgeVisualStyle {
            color: [0.8, 0.8, 0.8, 0.8],
            thickness: 1.0,
            opacity: 0.8,
            dash_pattern: None,
            animation_speed: 0.0,
            glow: false,
            visible: true,
        }
    }
}

impl Default for EdgeMetadata {
    fn default() -> Self {
        EdgeMetadata {
            labels: Vec::new(),
            description: None,
            tags: Vec::new(),
            user_created: false,
            pinned: false,
            temporary: false,
            expires_at: None,
        }
    }
}

impl GraphEdge {
    /// Create a new graph edge
    pub fn new(id: SceneId, source: SceneId, target: SceneId, edge_type: EdgeType) -> Self {
        let mut visual_style = EdgeVisualStyle::default();
        
        // Set default colors based on edge type
        visual_style.color = match edge_type {
            EdgeType::Contains => [0.4, 0.8, 0.4, 0.8],        // Green
            EdgeType::DependsOn => [0.8, 0.4, 0.4, 0.8],       // Red
            EdgeType::CommunicatesWith => [0.4, 0.4, 0.8, 0.8], // Blue
            EdgeType::CreatedBy => [0.8, 0.6, 0.2, 0.8],       // Orange
            EdgeType::RelatedTo { .. } => [0.6, 0.6, 0.6, 0.6], // Gray
            EdgeType::Temporal { .. } => [0.8, 0.2, 0.8, 0.8], // Purple
            EdgeType::TaggedAs { .. } => [0.2, 0.8, 0.8, 0.8], // Cyan
            EdgeType::WorksOn => [0.9, 0.9, 0.2, 0.8],         // Yellow
        };
        
        GraphEdge {
            id,
            source,
            target,
            edge_type,
            relationship_data: RelationshipData::default(),
            visual_style,
            metadata: EdgeMetadata::default(),
        }
    }
    
    /// Update the relationship strength
    pub fn update_strength(&mut self, new_strength: f32) {
        self.relationship_data.strength = new_strength.clamp(0.0, 1.0);
        self.relationship_data.last_accessed = chrono::Utc::now();
        self.update_visual_style();
    }
    
    /// Record that this relationship was accessed
    pub fn record_access(&mut self) {
        self.relationship_data.frequency += 1;
        self.relationship_data.last_accessed = chrono::Utc::now();
        
        // Increase strength slightly for frequently accessed relationships
        let strength_boost = 0.01 * (self.relationship_data.frequency as f32).ln();
        self.relationship_data.strength = (self.relationship_data.strength + strength_boost).min(1.0);
        
        self.update_visual_style();
    }
    
    /// Update visual style based on relationship data
    fn update_visual_style(&mut self) {
        // Make thickness proportional to strength
        self.visual_style.thickness = 0.5 + (self.relationship_data.strength * 2.0);
        
        // Make opacity proportional to confidence
        self.visual_style.opacity = 0.3 + (self.relationship_data.confidence * 0.7);
        
        // Add glow for strong relationships
        self.visual_style.glow = self.relationship_data.strength > 0.8;
        
        // Add animation for frequently accessed relationships
        if self.relationship_data.frequency > 10 {
            self.visual_style.animation_speed = 1.0;
        }
    }
    
    /// Check if the edge should expire
    pub fn should_expire(&self) -> bool {
        if let Some(expires_at) = self.metadata.expires_at {
            chrono::Utc::now() > expires_at
        } else {
            false
        }
    }
    
    /// Set the edge to expire after a duration
    pub fn set_expiry(&mut self, duration: chrono::Duration) {
        self.metadata.temporary = true;
        self.metadata.expires_at = Some(chrono::Utc::now() + duration);
    }
    
    /// Convert to scene edge for rendering
    pub fn to_scene_edge(&self) -> SceneEdge {
        SceneEdge {
            id: self.id,
            source: self.source,
            target: self.target,
            edge_type: self.edge_type.clone(),
            weight: self.relationship_data.strength,
            color: self.visual_style.color,
            visible: self.visual_style.visible,
            animated: self.visual_style.animation_speed > 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_edge_creation() {
        let edge = GraphEdge::new(1, 100, 200, EdgeType::Contains);
        assert_eq!(edge.id, 1);
        assert_eq!(edge.source, 100);
        assert_eq!(edge.target, 200);
        assert!(matches!(edge.edge_type, EdgeType::Contains));
        assert_eq!(edge.visual_style.color, [0.4, 0.8, 0.4, 0.8]); // Green for Contains
    }

    #[test]
    fn test_strength_update() {
        let mut edge = GraphEdge::new(1, 100, 200, EdgeType::DependsOn);
        let initial_strength = edge.relationship_data.strength;
        
        edge.update_strength(0.9);
        assert_eq!(edge.relationship_data.strength, 0.9);
        assert!(edge.visual_style.thickness > 1.0); // Should increase with strength
    }

    #[test]
    fn test_access_recording() {
        let mut edge = GraphEdge::new(1, 100, 200, EdgeType::RelatedTo { similarity: 0.5 });
        let initial_frequency = edge.relationship_data.frequency;
        
        edge.record_access();
        assert_eq!(edge.relationship_data.frequency, initial_frequency + 1);
        assert!(edge.relationship_data.strength >= 0.5); // Should increase slightly
    }

    #[test]
    fn test_edge_expiry() {
        let mut edge = GraphEdge::new(1, 100, 200, EdgeType::Temporal { sequence_order: 1 });
        assert!(!edge.should_expire());
        
        edge.set_expiry(chrono::Duration::milliseconds(1));
        std::thread::sleep(std::time::Duration::from_millis(2));
        assert!(edge.should_expire());
    }

    #[test]
    fn test_scene_edge_conversion() {
        let graph_edge = GraphEdge::new(1, 100, 200, EdgeType::WorksOn);
        let scene_edge = graph_edge.to_scene_edge();
        
        assert_eq!(scene_edge.id, graph_edge.id);
        assert_eq!(scene_edge.source, graph_edge.source);
        assert_eq!(scene_edge.target, graph_edge.target);
        assert_eq!(scene_edge.weight, graph_edge.relationship_data.strength);
    }
}