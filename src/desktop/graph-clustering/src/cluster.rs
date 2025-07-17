//! Core cluster data structures

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use horizonos_graph_engine::SceneId;

/// Unique identifier for a cluster
pub type ClusterId = Uuid;

/// A cluster of related nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cluster {
    /// Unique identifier
    pub id: ClusterId,
    /// Display name
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// Cluster type
    pub cluster_type: ClusterType,
    /// Cluster creation method
    pub creation_method: CreationMethod,
    /// Node IDs in this cluster
    pub nodes: HashSet<SceneId>,
    /// Visual style
    pub style: ClusterStyle,
    /// Metadata
    pub metadata: ClusterMetadata,
    /// Whether the cluster is currently visible
    pub visible: bool,
    /// Whether the cluster is currently expanded
    pub expanded: bool,
}

/// Type of cluster
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ClusterType {
    /// Connected components
    Connected,
    /// Proximity-based clustering
    Proximity,
    /// Semantic clustering (same type, similar properties)
    Semantic,
    /// Temporal clustering (same time period)
    Temporal,
    /// Manual clustering (user-created)
    Manual,
    /// AI-suggested clustering
    AISuggested,
}

/// How the cluster was created
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CreationMethod {
    /// Automatically generated
    Automatic,
    /// Manually created by user
    Manual,
    /// AI suggestion accepted
    AISuggestion,
    /// Imported from external source
    Imported,
}

/// Visual styling for clusters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterStyle {
    /// Boundary color
    pub boundary_color: [f32; 4],
    /// Fill color (for background)
    pub fill_color: [f32; 4],
    /// Boundary style
    pub boundary_style: BoundaryStyle,
    /// Whether to show label
    pub show_label: bool,
    /// Label position
    pub label_position: LabelPosition,
    /// Boundary width
    pub boundary_width: f32,
}

/// Boundary rendering style
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BoundaryStyle {
    /// Solid line boundary
    Solid,
    /// Dashed line boundary
    Dashed,
    /// Dotted line boundary
    Dotted,
    /// Soft glow boundary
    Glow,
    /// Particle boundary
    Particles,
    /// No visible boundary
    None,
}

/// Label position relative to cluster
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LabelPosition {
    /// Top of cluster
    Top,
    /// Bottom of cluster
    Bottom,
    /// Center of cluster
    Center,
    /// Top-left corner
    TopLeft,
    /// Top-right corner
    TopRight,
    /// Floating above cluster
    Floating,
}

/// Cluster metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterMetadata {
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last modified timestamp
    pub modified_at: DateTime<Utc>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Custom properties
    pub properties: std::collections::HashMap<String, String>,
    /// Confidence score for auto-generated clusters
    pub confidence: Option<f32>,
}

impl Cluster {
    /// Create a new automatically generated cluster
    pub fn new_auto(name: String, nodes: Vec<SceneId>, cluster_type: ClusterType) -> Self {
        let id = Uuid::new_v4();
        let now = Utc::now();
        
        Self {
            id,
            name,
            description: None,
            cluster_type,
            creation_method: CreationMethod::Automatic,
            nodes: nodes.into_iter().collect(),
            style: ClusterStyle::default_for_type(cluster_type),
            metadata: ClusterMetadata {
                created_at: now,
                modified_at: now,
                tags: Vec::new(),
                properties: std::collections::HashMap::new(),
                confidence: Some(0.8), // Default confidence for auto clusters
            },
            visible: true,
            expanded: true,
        }
    }
    
    /// Create a new manually created cluster
    pub fn new_manual(name: String, nodes: Vec<SceneId>) -> Self {
        let id = Uuid::new_v4();
        let now = Utc::now();
        
        Self {
            id,
            name,
            description: None,
            cluster_type: ClusterType::Manual,
            creation_method: CreationMethod::Manual,
            nodes: nodes.into_iter().collect(),
            style: ClusterStyle::default_for_type(ClusterType::Manual),
            metadata: ClusterMetadata {
                created_at: now,
                modified_at: now,
                tags: Vec::new(),
                properties: std::collections::HashMap::new(),
                confidence: None, // Manual clusters don't have confidence scores
            },
            visible: true,
            expanded: true,
        }
    }
    
    /// Add a node to the cluster
    pub fn add_node(&mut self, node_id: SceneId) {
        if self.nodes.insert(node_id) {
            self.metadata.modified_at = Utc::now();
        }
    }
    
    /// Remove a node from the cluster
    pub fn remove_node(&mut self, node_id: SceneId) -> bool {
        if self.nodes.remove(&node_id) {
            self.metadata.modified_at = Utc::now();
            true
        } else {
            false
        }
    }
    
    /// Check if cluster contains a node
    pub fn contains_node(&self, node_id: SceneId) -> bool {
        self.nodes.contains(&node_id)
    }
    
    /// Get number of nodes in cluster
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
    
    /// Check if cluster is empty
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
    
    /// Merge another cluster into this one
    pub fn merge_with(&mut self, other: &Cluster) {
        for &node_id in &other.nodes {
            self.nodes.insert(node_id);
        }
        self.metadata.modified_at = Utc::now();
        
        // Merge tags
        for tag in &other.metadata.tags {
            if !self.metadata.tags.contains(tag) {
                self.metadata.tags.push(tag.clone());
            }
        }
        
        // If this cluster was auto-generated and we're merging a manual cluster,
        // upgrade to manual
        if other.creation_method == CreationMethod::Manual {
            self.creation_method = CreationMethod::Manual;
            self.cluster_type = ClusterType::Manual;
        }
    }
    
    /// Set cluster style
    pub fn set_style(&mut self, style: ClusterStyle) {
        self.style = style;
        self.metadata.modified_at = Utc::now();
    }
    
    /// Add a tag to the cluster
    pub fn add_tag(&mut self, tag: String) {
        if !self.metadata.tags.contains(&tag) {
            self.metadata.tags.push(tag);
            self.metadata.modified_at = Utc::now();
        }
    }
    
    /// Remove a tag from the cluster
    pub fn remove_tag(&mut self, tag: &str) -> bool {
        if let Some(pos) = self.metadata.tags.iter().position(|t| t == tag) {
            self.metadata.tags.remove(pos);
            self.metadata.modified_at = Utc::now();
            true
        } else {
            false
        }
    }
    
    /// Set a custom property
    pub fn set_property(&mut self, key: String, value: String) {
        self.metadata.properties.insert(key, value);
        self.metadata.modified_at = Utc::now();
    }
    
    /// Get a custom property
    pub fn get_property(&self, key: &str) -> Option<&String> {
        self.metadata.properties.get(key)
    }
}

impl ClusterStyle {
    /// Default style for a cluster type
    pub fn default_for_type(cluster_type: ClusterType) -> Self {
        match cluster_type {
            ClusterType::Connected => Self {
                boundary_color: [0.2, 0.6, 1.0, 0.8],
                fill_color: [0.2, 0.6, 1.0, 0.1],
                boundary_style: BoundaryStyle::Solid,
                show_label: true,
                label_position: LabelPosition::TopLeft,
                boundary_width: 2.0,
            },
            ClusterType::Proximity => Self {
                boundary_color: [0.6, 0.8, 0.2, 0.8],
                fill_color: [0.6, 0.8, 0.2, 0.1],
                boundary_style: BoundaryStyle::Dashed,
                show_label: true,
                label_position: LabelPosition::Top,
                boundary_width: 1.5,
            },
            ClusterType::Semantic => Self {
                boundary_color: [1.0, 0.6, 0.2, 0.8],
                fill_color: [1.0, 0.6, 0.2, 0.1],
                boundary_style: BoundaryStyle::Dotted,
                show_label: true,
                label_position: LabelPosition::Center,
                boundary_width: 2.0,
            },
            ClusterType::Temporal => Self {
                boundary_color: [0.8, 0.2, 0.8, 0.8],
                fill_color: [0.8, 0.2, 0.8, 0.1],
                boundary_style: BoundaryStyle::Glow,
                show_label: true,
                label_position: LabelPosition::Floating,
                boundary_width: 1.0,
            },
            ClusterType::Manual => Self {
                boundary_color: [0.9, 0.9, 0.9, 0.9],
                fill_color: [0.9, 0.9, 0.9, 0.15],
                boundary_style: BoundaryStyle::Solid,
                show_label: true,
                label_position: LabelPosition::TopLeft,
                boundary_width: 2.5,
            },
            ClusterType::AISuggested => Self {
                boundary_color: [0.6, 0.9, 0.6, 0.7],
                fill_color: [0.6, 0.9, 0.6, 0.1],
                boundary_style: BoundaryStyle::Particles,
                show_label: true,
                label_position: LabelPosition::Floating,
                boundary_width: 1.5,
            },
        }
    }
}