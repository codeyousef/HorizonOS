//! Edge content analysis for semantic relationship detection
//!
//! This module provides advanced analysis of edge content to determine semantic
//! relationships between nodes in the graph, enabling better visualization and
//! navigation of complex data structures.

use crate::{Scene, Camera, GraphEngineError, SceneId, SceneNode, SceneEdge};
use nalgebra::Point3;
use std::collections::HashMap;
use std::sync::Arc;
use wgpu::{Device, Buffer, ComputePipeline, BindGroup, BindGroupLayout};

/// Configuration for edge content analysis
#[derive(Debug, Clone)]
pub struct EdgeContentConfig {
    /// Maximum distance for semantic similarity analysis
    pub max_analysis_distance: f32,
    /// Minimum edge strength to consider for analysis
    pub min_edge_strength: f32,
    /// Weight for temporal relationships (edges created close in time)
    pub temporal_weight: f32,
    /// Weight for spatial relationships (edges close in space)
    pub spatial_weight: f32,
    /// Weight for semantic relationships (edges with similar content)
    pub semantic_weight: f32,
    /// Enable clustering of similar edges
    pub enable_clustering: bool,
    /// Maximum number of edges to analyze per frame
    pub max_edges_per_frame: usize,
    /// Enable GPU-accelerated analysis
    pub use_gpu_acceleration: bool,
}

impl Default for EdgeContentConfig {
    fn default() -> Self {
        Self {
            max_analysis_distance: 1000.0,
            min_edge_strength: 0.1,
            temporal_weight: 0.3,
            spatial_weight: 0.4,
            semantic_weight: 0.3,
            enable_clustering: true,
            max_edges_per_frame: 100,
            use_gpu_acceleration: true,
        }
    }
}

/// Types of semantic relationships between nodes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SemanticRelationshipType {
    /// Data flow relationship (A provides data to B)
    DataFlow,
    /// Dependency relationship (A depends on B)
    Dependency,
    /// Hierarchical relationship (A is parent of B)
    Hierarchical,
    /// Associative relationship (A is associated with B)
    Associative,
    /// Temporal relationship (A and B are related by time)
    Temporal,
    /// Spatial relationship (A and B are related by location)
    Spatial,
    /// Unknown or unclassified relationship
    Unknown,
}

/// Semantic relationship strength and metadata
#[derive(Debug, Clone)]
pub struct SemanticRelationship {
    /// Type of relationship
    pub relationship_type: SemanticRelationshipType,
    /// Strength of the relationship (0.0 to 1.0)
    pub strength: f32,
    /// Confidence in the analysis (0.0 to 1.0)
    pub confidence: f32,
    /// Directional strength (positive means source->target, negative means target->source)
    pub directionality: f32,
    /// Temporal score (how related they are in time)
    pub temporal_score: f32,
    /// Spatial score (how related they are in space)
    pub spatial_score: f32,
    /// Semantic score (how related they are in content)
    pub semantic_score: f32,
    /// Last analysis timestamp
    pub last_analyzed: chrono::DateTime<chrono::Utc>,
}

/// Cluster of semantically related edges
#[derive(Debug, Clone)]
pub struct EdgeCluster {
    /// Unique identifier for the cluster
    pub id: String,
    /// Edges in this cluster
    pub edges: Vec<SceneId>,
    /// Centroid position of the cluster
    pub centroid: Point3<f32>,
    /// Average relationship strength
    pub avg_strength: f32,
    /// Dominant relationship type
    pub dominant_type: SemanticRelationshipType,
    /// Cluster radius
    pub radius: f32,
}

/// Results of edge content analysis
#[derive(Debug, Clone)]
pub struct EdgeContentAnalysis {
    /// Individual edge relationships
    pub relationships: HashMap<SceneId, SemanticRelationship>,
    /// Edge clusters
    pub clusters: Vec<EdgeCluster>,
    /// Global statistics
    pub statistics: EdgeAnalysisStatistics,
    /// Analysis timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Statistics from edge content analysis
#[derive(Debug, Clone)]
pub struct EdgeAnalysisStatistics {
    /// Total edges analyzed
    pub total_edges: usize,
    /// Number of strong relationships found
    pub strong_relationships: usize,
    /// Number of clusters detected
    pub cluster_count: usize,
    /// Average relationship strength
    pub avg_relationship_strength: f32,
    /// Distribution of relationship types
    pub relationship_distribution: HashMap<SemanticRelationshipType, usize>,
    /// Analysis time in milliseconds
    pub analysis_time_ms: f32,
}

/// GPU compute buffer for edge analysis
#[derive(Debug)]
#[allow(dead_code)]
struct EdgeAnalysisBuffer {
    /// Input buffer for edge data
    input_buffer: Buffer,
    /// Output buffer for analysis results
    output_buffer: Buffer,
    /// Staging buffer for CPU readback
    staging_buffer: Buffer,
    /// Bind group for compute shader
    bind_group: BindGroup,
}

/// Main edge content analyzer
pub struct EdgeContentAnalyzer {
    /// Configuration
    config: EdgeContentConfig,
    /// Analysis results
    analysis: EdgeContentAnalysis,
    /// GPU device
    #[allow(dead_code)]
    device: Arc<Device>,
    /// Compute pipeline for GPU analysis
    #[allow(dead_code)]
    compute_pipeline: Option<ComputePipeline>,
    /// Bind group layout
    #[allow(dead_code)]
    bind_group_layout: BindGroupLayout,
    /// GPU buffers
    #[allow(dead_code)]
    gpu_buffers: Option<EdgeAnalysisBuffer>,
    /// Frame counter for paced analysis
    frame_counter: u64,
    /// Cache for semantic similarity calculations
    similarity_cache: HashMap<(SceneId, SceneId), f32>,
    /// Performance tracking
    last_analysis_time: std::time::Instant,
}

impl EdgeContentAnalyzer {
    /// Create a new edge content analyzer
    pub fn new(device: Arc<Device>) -> Result<Self, GraphEngineError> {
        let config = EdgeContentConfig::default();
        
        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Edge Analysis Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        
        // Initialize compute pipeline if GPU acceleration is enabled
        let compute_pipeline = if config.use_gpu_acceleration {
            Some(Self::create_compute_pipeline(&device, &bind_group_layout)?)
        } else {
            None
        };
        
        let analysis = EdgeContentAnalysis {
            relationships: HashMap::new(),
            clusters: Vec::new(),
            statistics: EdgeAnalysisStatistics {
                total_edges: 0,
                strong_relationships: 0,
                cluster_count: 0,
                avg_relationship_strength: 0.0,
                relationship_distribution: HashMap::new(),
                analysis_time_ms: 0.0,
            },
            timestamp: chrono::Utc::now(),
        };
        
        Ok(Self {
            config,
            analysis,
            device,
            compute_pipeline,
            bind_group_layout,
            gpu_buffers: None,
            frame_counter: 0,
            similarity_cache: HashMap::new(),
            last_analysis_time: std::time::Instant::now(),
        })
    }
    
    /// Analyze edges in the scene
    pub fn analyze_edges(&mut self, scene: &Scene, camera: &Camera) -> Result<(), GraphEngineError> {
        let start_time = std::time::Instant::now();
        self.frame_counter += 1;
        
        // Perform analysis every few frames to avoid performance impact
        if self.frame_counter % 10 != 0 {
            return Ok(());
        }
        
        // Get edges from scene (simplified - would use actual view culling)
        let edges: Vec<_> = scene.edges().take(self.config.max_edges_per_frame).collect();
        
        // Limit number of edges to analyze per frame
        let edges_to_analyze = if edges.len() > self.config.max_edges_per_frame {
            &edges[..self.config.max_edges_per_frame]
        } else {
            &edges
        };
        
        // Analyze relationships
        let mut relationships = HashMap::new();
        
        for edge in edges_to_analyze {
            let relationship = self.analyze_edge_relationship(edge, scene, camera)?;
            if relationship.strength >= self.config.min_edge_strength {
                relationships.insert(edge.id, relationship);
            }
        }
        
        // Detect clusters if enabled
        let clusters = if self.config.enable_clustering {
            self.detect_edge_clusters(&relationships)?
        } else {
            Vec::new()
        };
        
        // Update statistics
        let statistics = self.calculate_statistics(&relationships, &clusters, start_time.elapsed());
        
        // Update analysis results
        self.analysis = EdgeContentAnalysis {
            relationships,
            clusters,
            statistics,
            timestamp: chrono::Utc::now(),
        };
        
        self.last_analysis_time = std::time::Instant::now();
        
        Ok(())
    }
    
    /// Analyze relationship for a single edge
    fn analyze_edge_relationship(
        &mut self,
        edge: &SceneEdge,
        scene: &Scene,
        camera: &Camera,
    ) -> Result<SemanticRelationship, GraphEngineError> {
        let source_node = scene.get_node(edge.source)
            .ok_or_else(|| GraphEngineError::NodeNotFound(edge.source))?;
        let target_node = scene.get_node(edge.target)
            .ok_or_else(|| GraphEngineError::NodeNotFound(edge.target))?;
        
        // Calculate temporal score
        let temporal_score = self.calculate_temporal_similarity(source_node, target_node)?;
        
        // Calculate spatial score
        let spatial_score = self.calculate_spatial_similarity(source_node, target_node, camera)?;
        
        // Calculate semantic score
        let semantic_score = self.calculate_semantic_similarity(source_node, target_node)?;
        
        // Determine relationship type
        let relationship_type = self.classify_relationship_type(edge, temporal_score, spatial_score, semantic_score)?;
        
        // Calculate overall strength
        let strength = (temporal_score * self.config.temporal_weight +
                       spatial_score * self.config.spatial_weight +
                       semantic_score * self.config.semantic_weight) / 
                      (self.config.temporal_weight + self.config.spatial_weight + self.config.semantic_weight);
        
        // Calculate confidence based on consistency of scores
        let score_variance = ((temporal_score - strength).powi(2) +
                             (spatial_score - strength).powi(2) +
                             (semantic_score - strength).powi(2)) / 3.0;
        let confidence = 1.0 - score_variance.sqrt();
        
        // Calculate directionality
        let directionality = self.calculate_directionality(edge, source_node, target_node)?;
        
        Ok(SemanticRelationship {
            relationship_type,
            strength,
            confidence,
            directionality,
            temporal_score,
            spatial_score,
            semantic_score,
            last_analyzed: chrono::Utc::now(),
        })
    }
    
    /// Calculate temporal similarity between two nodes
    fn calculate_temporal_similarity(
        &self,
        source: &SceneNode,
        target: &SceneNode,
    ) -> Result<f32, GraphEngineError> {
        // Get creation times from metadata
        let source_time = source.metadata.created_at;
        let target_time = target.metadata.created_at;
        
        // Calculate time difference in hours
        let time_diff = (source_time - target_time).num_seconds().abs() as f32 / 3600.0;
        
        // Convert to similarity score (closer in time = higher score)
        let similarity = (1.0 / (1.0 + time_diff / 24.0)).min(1.0);
        
        Ok(similarity)
    }
    
    /// Calculate spatial similarity between two nodes
    fn calculate_spatial_similarity(
        &self,
        source: &SceneNode,
        target: &SceneNode,
        camera: &Camera,
    ) -> Result<f32, GraphEngineError> {
        let source_pos = source.position;
        let target_pos = target.position;
        
        // Calculate distance
        let distance = (source_pos - target_pos).norm();
        
        // Calculate view distance from camera
        let camera_pos = camera.position;
        let avg_pos = (source_pos + target_pos.coords) / 2.0;
        let view_distance = (camera_pos - avg_pos).norm();
        
        // Normalize by view distance (closer edges in view are more important)
        let normalized_distance = distance / view_distance.max(1.0);
        
        // Convert to similarity score
        let similarity = (1.0 / (1.0 + normalized_distance)).min(1.0);
        
        Ok(similarity)
    }
    
    /// Calculate semantic similarity between two nodes
    fn calculate_semantic_similarity(
        &mut self,
        source: &SceneNode,
        target: &SceneNode,
    ) -> Result<f32, GraphEngineError> {
        let cache_key = (source.id, target.id);
        
        // Check cache first
        if let Some(&cached_similarity) = self.similarity_cache.get(&cache_key) {
            return Ok(cached_similarity);
        }
        
        // Calculate similarity based on node types and content
        let mut similarity = 0.0;
        
        // Type similarity (simplified - would check actual node types)
        similarity += 0.3;
        
        // Content similarity using description field
        let source_content = source.metadata.description.as_deref().unwrap_or("");
        let target_content = target.metadata.description.as_deref().unwrap_or("");
        let content_similarity = self.calculate_content_similarity(source_content, target_content)?;
        similarity += content_similarity * 0.4;
        
        // Tag similarity
        let tag_similarity = self.calculate_tag_similarity(&source.metadata.tags, &target.metadata.tags)?;
        similarity += tag_similarity * 0.3;
        
        // Cache result
        self.similarity_cache.insert(cache_key, similarity);
        
        Ok(similarity)
    }
    
    /// Calculate content similarity between two strings
    fn calculate_content_similarity(&self, content1: &str, content2: &str) -> Result<f32, GraphEngineError> {
        // Simple word-based similarity (in practice would use more sophisticated NLP)
        let content1_lower = content1.to_lowercase();
        let content2_lower = content2.to_lowercase();
        let words1: std::collections::HashSet<&str> = content1_lower.split_whitespace().collect();
        let words2: std::collections::HashSet<&str> = content2_lower.split_whitespace().collect();
        
        let intersection = words1.intersection(&words2).count();
        let union = words1.union(&words2).count();
        
        if union == 0 {
            return Ok(0.0);
        }
        
        let similarity = intersection as f32 / union as f32;
        Ok(similarity)
    }
    
    /// Calculate tag similarity between two tag sets
    fn calculate_tag_similarity(&self, tags1: &[String], tags2: &[String]) -> Result<f32, GraphEngineError> {
        let set1: std::collections::HashSet<&String> = tags1.iter().collect();
        let set2: std::collections::HashSet<&String> = tags2.iter().collect();
        
        let intersection = set1.intersection(&set2).count();
        let union = set1.union(&set2).count();
        
        if union == 0 {
            return Ok(0.0);
        }
        
        let similarity = intersection as f32 / union as f32;
        Ok(similarity)
    }
    
    /// Classify relationship type based on scores
    fn classify_relationship_type(
        &self,
        _edge: &SceneEdge,
        temporal_score: f32,
        spatial_score: f32,
        semantic_score: f32,
    ) -> Result<SemanticRelationshipType, GraphEngineError> {
        // Use scores to determine relationship type since SceneEdge doesn't have edge_type
        if temporal_score > 0.7 {
            Ok(SemanticRelationshipType::Temporal)
        } else if spatial_score > 0.7 {
            Ok(SemanticRelationshipType::Spatial)
        } else if semantic_score > 0.7 {
            Ok(SemanticRelationshipType::Associative)
        } else if temporal_score > spatial_score && temporal_score > semantic_score {
            Ok(SemanticRelationshipType::Dependency)
        } else if spatial_score > semantic_score {
            Ok(SemanticRelationshipType::Hierarchical)
        } else {
            Ok(SemanticRelationshipType::DataFlow)
        }
    }
    
    /// Calculate directionality of relationship
    fn calculate_directionality(
        &self,
        _edge: &SceneEdge,
        source: &SceneNode,
        target: &SceneNode,
    ) -> Result<f32, GraphEngineError> {
        // Simple heuristic based on creation time and node importance
        let time_diff = (source.metadata.created_at - target.metadata.created_at).num_seconds() as f32;
        let time_directionality = time_diff.signum() * (time_diff.abs() / 3600.0).tanh();
        
        // Consider edge direction (all edges in scene graph are considered directed)
        let edge_directionality = 1.0;
        
        Ok((time_directionality + edge_directionality) / 2.0)
    }
    
    /// Detect clusters of semantically related edges
    fn detect_edge_clusters(
        &self,
        relationships: &HashMap<SceneId, SemanticRelationship>,
    ) -> Result<Vec<EdgeCluster>, GraphEngineError> {
        let mut clusters = Vec::new();
        let mut processed_edges = std::collections::HashSet::new();
        
        for (edge_id, relationship) in relationships {
            if processed_edges.contains(edge_id) {
                continue;
            }
            
            // Find nearby edges with similar relationships
            let cluster_edges = self.find_similar_edges(*edge_id, relationship, relationships)?;
            
            if cluster_edges.len() >= 2 {
                // Calculate cluster properties
                let centroid = self.calculate_cluster_centroid(&cluster_edges)?;
                let avg_strength = cluster_edges.iter()
                    .map(|&edge_id| relationships.get(&edge_id).unwrap().strength)
                    .sum::<f32>() / cluster_edges.len() as f32;
                
                let dominant_type = self.find_dominant_relationship_type(&cluster_edges, relationships)?;
                let radius = self.calculate_cluster_radius(&cluster_edges, &centroid)?;
                
                clusters.push(EdgeCluster {
                    id: format!("cluster_{}", clusters.len()),
                    edges: cluster_edges.clone(),
                    centroid,
                    avg_strength,
                    dominant_type,
                    radius,
                });
                
                // Mark edges as processed
                for edge_id in cluster_edges {
                    processed_edges.insert(edge_id);
                }
            }
        }
        
        Ok(clusters)
    }
    
    /// Find edges similar to a given edge
    fn find_similar_edges(
        &self,
        target_edge_id: SceneId,
        target_relationship: &SemanticRelationship,
        relationships: &HashMap<SceneId, SemanticRelationship>,
    ) -> Result<Vec<SceneId>, GraphEngineError> {
        let mut similar_edges = vec![target_edge_id];
        
        for (edge_id, relationship) in relationships {
            if *edge_id == target_edge_id {
                continue;
            }
            
            // Check if relationships are similar
            if relationship.relationship_type == target_relationship.relationship_type {
                let strength_diff = (relationship.strength - target_relationship.strength).abs();
                if strength_diff < 0.2 {
                    similar_edges.push(*edge_id);
                }
            }
        }
        
        Ok(similar_edges)
    }
    
    /// Calculate centroid of a cluster
    fn calculate_cluster_centroid(&self, _edges: &[SceneId]) -> Result<Point3<f32>, GraphEngineError> {
        // This would need access to edge positions - simplified for now
        Ok(Point3::new(0.0, 0.0, 0.0))
    }
    
    /// Find dominant relationship type in a cluster
    fn find_dominant_relationship_type(
        &self,
        edges: &[SceneId],
        relationships: &HashMap<SceneId, SemanticRelationship>,
    ) -> Result<SemanticRelationshipType, GraphEngineError> {
        let mut type_counts = HashMap::new();
        
        for edge_id in edges {
            if let Some(relationship) = relationships.get(edge_id) {
                *type_counts.entry(relationship.relationship_type).or_insert(0) += 1;
            }
        }
        
        let dominant_type = type_counts.into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(type_, _)| type_)
            .unwrap_or(SemanticRelationshipType::Unknown);
        
        Ok(dominant_type)
    }
    
    /// Calculate radius of a cluster
    fn calculate_cluster_radius(&self, _edges: &[SceneId], _centroid: &Point3<f32>) -> Result<f32, GraphEngineError> {
        // This would need access to edge positions - simplified for now
        Ok(10.0)
    }
    
    /// Calculate analysis statistics
    fn calculate_statistics(
        &self,
        relationships: &HashMap<SceneId, SemanticRelationship>,
        clusters: &[EdgeCluster],
        analysis_time: std::time::Duration,
    ) -> EdgeAnalysisStatistics {
        let total_edges = relationships.len();
        let strong_relationships = relationships.values()
            .filter(|r| r.strength > 0.7)
            .count();
        
        let avg_relationship_strength = if total_edges > 0 {
            relationships.values().map(|r| r.strength).sum::<f32>() / total_edges as f32
        } else {
            0.0
        };
        
        let mut relationship_distribution = HashMap::new();
        for relationship in relationships.values() {
            *relationship_distribution.entry(relationship.relationship_type).or_insert(0) += 1;
        }
        
        EdgeAnalysisStatistics {
            total_edges,
            strong_relationships,
            cluster_count: clusters.len(),
            avg_relationship_strength,
            relationship_distribution,
            analysis_time_ms: analysis_time.as_secs_f32() * 1000.0,
        }
    }
    
    /// Create compute pipeline for GPU acceleration
    fn create_compute_pipeline(
        device: &Device,
        bind_group_layout: &BindGroupLayout,
    ) -> Result<ComputePipeline, GraphEngineError> {
        // Simplified - would need actual compute shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Edge Analysis Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/edge_analysis.wgsl").into()),
        });
        
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Edge Analysis Pipeline Layout"),
            bind_group_layouts: &[bind_group_layout],
            push_constant_ranges: &[],
        });
        
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Edge Analysis Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });
        
        Ok(pipeline)
    }
    
    /// Get current analysis results
    pub fn get_analysis(&self) -> EdgeContentAnalysis {
        self.analysis.clone()
    }
    
    /// Get semantic strength for a specific edge
    pub fn get_semantic_strength(&self, edge_id: SceneId) -> f32 {
        self.analysis.relationships.get(&edge_id)
            .map(|r| r.strength)
            .unwrap_or(0.0)
    }
    
    /// Update configuration
    pub fn update_config(&mut self, new_config: EdgeContentConfig) {
        // Clear cache if settings changed significantly
        if self.config.semantic_weight != new_config.semantic_weight ||
           self.config.temporal_weight != new_config.temporal_weight ||
           self.config.spatial_weight != new_config.spatial_weight {
            self.similarity_cache.clear();
        }
        
        self.config = new_config;
    }
    
    /// Clear analysis cache
    pub fn clear_cache(&mut self) {
        self.similarity_cache.clear();
    }
    
    /// Get performance statistics
    pub fn get_performance_stats(&self) -> HashMap<String, f32> {
        let mut stats = HashMap::new();
        stats.insert("last_analysis_time_ms".to_string(), self.analysis.statistics.analysis_time_ms);
        stats.insert("total_edges".to_string(), self.analysis.statistics.total_edges as f32);
        stats.insert("strong_relationships".to_string(), self.analysis.statistics.strong_relationships as f32);
        stats.insert("cluster_count".to_string(), self.analysis.statistics.cluster_count as f32);
        stats.insert("avg_relationship_strength".to_string(), self.analysis.statistics.avg_relationship_strength);
        stats.insert("cache_size".to_string(), self.similarity_cache.len() as f32);
        stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_edge_content_config() {
        let config = EdgeContentConfig::default();
        assert_eq!(config.max_analysis_distance, 1000.0);
        assert_eq!(config.min_edge_strength, 0.1);
        assert!(config.enable_clustering);
        assert!(config.use_gpu_acceleration);
    }
    
    #[test]
    fn test_semantic_relationship_type() {
        let relationship = SemanticRelationship {
            relationship_type: SemanticRelationshipType::DataFlow,
            strength: 0.8,
            confidence: 0.9,
            directionality: 0.5,
            temporal_score: 0.7,
            spatial_score: 0.8,
            semantic_score: 0.9,
            last_analyzed: chrono::Utc::now(),
        };
        
        assert_eq!(relationship.relationship_type, SemanticRelationshipType::DataFlow);
        assert!(relationship.strength > 0.5);
        assert!(relationship.confidence > 0.5);
    }
    
    #[test]
    fn test_edge_cluster() {
        let cluster = EdgeCluster {
            id: "test_cluster".to_string(),
            edges: vec![1, 2],
            centroid: Point3::new(0.0, 0.0, 0.0),
            avg_strength: 0.75,
            dominant_type: SemanticRelationshipType::Associative,
            radius: 10.0,
        };
        
        assert_eq!(cluster.id, "test_cluster");
        assert_eq!(cluster.edges.len(), 2);
        assert_eq!(cluster.avg_strength, 0.75);
        assert_eq!(cluster.dominant_type, SemanticRelationshipType::Associative);
    }
}