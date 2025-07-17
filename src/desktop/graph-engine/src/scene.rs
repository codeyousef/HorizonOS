//! Scene graph management for nodes and edges

use nalgebra::{Point3, Vector3};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unique identifier for scene objects
pub type SceneId = u64;

/// 3D position in the graph space
pub type Position = Point3<f32>;

/// 3D vector for directions and forces
pub type Vec3 = Vector3<f32>;

/// Scene graph containing all nodes and edges
#[derive(Debug, Default)]
pub struct Scene {
    /// All nodes in the scene
    nodes: HashMap<SceneId, SceneNode>,
    /// All edges in the scene
    edges: HashMap<SceneId, SceneEdge>,
    /// Spatial index for efficient queries
    spatial_index: SpatialIndex,
    /// Next available ID
    next_id: SceneId,
}

/// A node in the scene graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneNode {
    pub id: SceneId,
    pub position: Position,
    pub velocity: Vec3,
    pub radius: f32,
    pub color: [f32; 4], // RGBA
    pub node_type: NodeType,
    pub metadata: NodeMetadata,
    pub visible: bool,
    pub selected: bool,
}

/// Types of nodes in the graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeType {
    Application { pid: u32, name: String },
    File { path: String, file_type: FileType },
    Person { name: String, contact_info: ContactInfo },
    Task { title: String, status: TaskStatus },
    Device { name: String, device_type: DeviceType },
    AIAgent { name: String, model: String },
    Concept { title: String, content: String },
    System { component: String, status: SystemStatus },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FileType {
    Directory,
    RegularFile,
    Image,
    Video,
    Audio,
    Document,
    Code,
    Archive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactInfo {
    pub email: Option<String>,
    pub phone: Option<String>,
    pub social: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    Todo,
    InProgress,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeviceType {
    Computer,
    Phone,
    Tablet,
    IoTDevice,
    NetworkDevice,
    Storage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemStatus {
    Running,
    Stopped,
    Error,
    Warning,
}

/// Additional metadata for nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetadata {
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub tags: Vec<String>,
    pub description: Option<String>,
    pub properties: HashMap<String, String>,
}

/// An edge connecting two nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneEdge {
    pub id: SceneId,
    pub source: SceneId,
    pub target: SceneId,
    pub edge_type: EdgeType,
    pub weight: f32,
    pub color: [f32; 4], // RGBA
    pub visible: bool,
    pub animated: bool,
}

/// Types of relationships between nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EdgeType {
    Contains,
    DependsOn,
    CommunicatesWith,
    CreatedBy,
    RelatedTo { similarity: f32 },
    Temporal { sequence_order: u32 },
    TaggedAs { tag: String },
    WorksOn,
}

/// Spatial indexing for efficient queries
#[derive(Debug, Default)]
pub struct SpatialIndex {
    // Simple implementation - can be replaced with more sophisticated structures
    bounds: HashMap<SceneId, BoundingBox>,
}

#[derive(Debug, Clone)]
pub struct BoundingBox {
    pub min: Position,
    pub max: Position,
}

impl Scene {
    /// Create a new empty scene
    pub fn new() -> Self {
        Scene::default()
    }
    
    /// Add a node to the scene
    pub fn add_node(&mut self, mut node: SceneNode) -> SceneId {
        node.id = self.next_id;
        let id = node.id;
        self.next_id += 1;
        
        // Update spatial index
        let bbox = BoundingBox {
            min: Point3::new(
                node.position.x - node.radius,
                node.position.y - node.radius,
                node.position.z - node.radius,
            ),
            max: Point3::new(
                node.position.x + node.radius,
                node.position.y + node.radius,
                node.position.z + node.radius,
            ),
        };
        self.spatial_index.bounds.insert(id, bbox);
        
        self.nodes.insert(id, node);
        id
    }
    
    /// Add an edge to the scene
    pub fn add_edge(&mut self, mut edge: SceneEdge) -> SceneId {
        edge.id = self.next_id;
        let id = edge.id;
        self.next_id += 1;
        
        self.edges.insert(id, edge);
        id
    }
    
    /// Get a node by ID
    pub fn get_node(&self, id: SceneId) -> Option<&SceneNode> {
        self.nodes.get(&id)
    }
    
    /// Get a mutable node by ID
    pub fn get_node_mut(&mut self, id: SceneId) -> Option<&mut SceneNode> {
        self.nodes.get_mut(&id)
    }
    
    /// Get an edge by ID
    pub fn get_edge(&self, id: SceneId) -> Option<&SceneEdge> {
        self.edges.get(&id)
    }
    
    /// Get all nodes
    pub fn nodes(&self) -> impl Iterator<Item = (&SceneId, &SceneNode)> {
        self.nodes.iter()
    }
    
    /// Get all edges
    pub fn edges(&self) -> impl Iterator<Item = &SceneEdge> {
        self.edges.values()
    }
    
    /// Update scene animations
    pub fn update(&mut self, _delta_time: f32) {
        // Update node positions, animations, etc.
        // This will be expanded with actual animation logic
    }
    
    /// Find nodes within a radius of a position
    pub fn find_nodes_in_radius(&self, center: Position, radius: f32) -> Vec<SceneId> {
        self.nodes
            .iter()
            .filter_map(|(id, node)| {
                let distance = (node.position - center).magnitude();
                if distance <= radius {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect()
    }
    
    /// Get edges connected to a node
    pub fn get_connected_edges(&self, node_id: SceneId) -> Vec<&SceneEdge> {
        self.edges
            .values()
            .filter(|edge| edge.source == node_id || edge.target == node_id)
            .collect()
    }
    
    /// Remove a node and all connected edges
    pub fn remove_node(&mut self, node_id: SceneId) -> Option<SceneNode> {
        // Remove connected edges
        let connected_edges: Vec<SceneId> = self.edges
            .iter()
            .filter_map(|(id, edge)| {
                if edge.source == node_id || edge.target == node_id {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect();
            
        for edge_id in connected_edges {
            self.edges.remove(&edge_id);
        }
        
        // Remove from spatial index
        self.spatial_index.bounds.remove(&node_id);
        
        // Remove the node
        self.nodes.remove(&node_id)
    }
    
    /// Get node position by ID
    pub fn get_node_position(&self, node_id: SceneId) -> Option<Position> {
        self.nodes.get(&node_id).map(|node| node.position)
    }
    
    /// Get all node IDs
    pub fn get_all_nodes(&self) -> Vec<SceneId> {
        self.nodes.keys().copied().collect()
    }
    
    /// Get all edges
    pub fn get_all_edges(&self) -> Vec<&SceneEdge> {
        self.edges.values().collect()
    }
    
    /// Clear the entire scene
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.edges.clear();
        self.spatial_index.bounds.clear();
        self.next_id = 0;
    }
}

impl Default for NodeMetadata {
    fn default() -> Self {
        let now = chrono::Utc::now();
        NodeMetadata {
            created_at: now,
            updated_at: now,
            tags: Vec::new(),
            description: None,
            properties: HashMap::new(),
        }
    }
}