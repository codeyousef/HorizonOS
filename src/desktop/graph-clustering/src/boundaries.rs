//! Cluster boundary visualization and computation

use crate::{Cluster, ClusterId, BoundaryStyle};
use anyhow::Result;
use horizonos_graph_engine::{SceneId, Scene};
use nalgebra::{Point3, Vector3};
use std::collections::HashMap;

/// Cluster boundary representation
#[derive(Debug, Clone)]
pub struct ClusterBoundary {
    /// Cluster ID
    pub cluster_id: ClusterId,
    /// Boundary points in 3D space
    pub points: Vec<Point3<f32>>,
    /// Boundary type
    pub boundary_type: BoundaryType,
    /// Visual style
    pub style: BoundaryStyle,
    /// Computed center point
    pub center: Point3<f32>,
    /// Bounding radius
    pub radius: f32,
}

/// Types of cluster boundaries
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundaryType {
    /// Convex hull around nodes
    ConvexHull,
    /// Circle/sphere around nodes
    Circle,
    /// Minimum bounding box
    BoundingBox,
    /// Alpha shape (concave hull)
    AlphaShape,
    /// Custom polygon
    Custom,
}

/// Boundary renderer for clusters
pub struct BoundaryRenderer {
    /// Cached boundaries
    boundaries: HashMap<ClusterId, ClusterBoundary>,
    /// Default boundary padding
    padding: f32,
}

impl BoundaryRenderer {
    /// Create new boundary renderer
    pub fn new() -> Self {
        Self {
            boundaries: HashMap::new(),
            padding: 20.0,
        }
    }
    
    /// Compute boundary for a cluster
    pub fn compute_boundary(&self, cluster: &Cluster, scene: &Scene) -> Result<ClusterBoundary> {
        let node_positions: Vec<Point3<f32>> = cluster.nodes
            .iter()
            .filter_map(|&node_id| scene.get_node_position(node_id))
            .collect();
        
        if node_positions.is_empty() {
            return Err(anyhow::anyhow!("No valid positions found for cluster nodes"));
        }
        
        let boundary_type = self.determine_boundary_type(&cluster.style.boundary_style, node_positions.len());
        let points = self.compute_boundary_points(&node_positions, boundary_type)?;
        let center = self.compute_center(&node_positions);
        let radius = self.compute_radius(&node_positions, center);
        
        Ok(ClusterBoundary {
            cluster_id: cluster.id,
            points,
            boundary_type,
            style: cluster.style.boundary_style,
            center,
            radius,
        })
    }
    
    /// Determine boundary type based on style and node count
    fn determine_boundary_type(&self, style: &BoundaryStyle, node_count: usize) -> BoundaryType {
        match style {
            BoundaryStyle::Solid | BoundaryStyle::Dashed | BoundaryStyle::Dotted => {
                if node_count <= 3 {
                    BoundaryType::Circle
                } else if node_count <= 10 {
                    BoundaryType::ConvexHull
                } else {
                    BoundaryType::AlphaShape
                }
            },
            BoundaryStyle::Glow | BoundaryStyle::Particles => BoundaryType::Circle,
            BoundaryStyle::None => BoundaryType::BoundingBox,
        }
    }
    
    /// Compute boundary points based on type
    fn compute_boundary_points(&self, positions: &[Point3<f32>], boundary_type: BoundaryType) -> Result<Vec<Point3<f32>>> {
        match boundary_type {
            BoundaryType::Circle => self.compute_circle_boundary(positions),
            BoundaryType::ConvexHull => self.compute_convex_hull(positions),
            BoundaryType::BoundingBox => self.compute_bounding_box(positions),
            BoundaryType::AlphaShape => self.compute_alpha_shape(positions),
            BoundaryType::Custom => Ok(positions.to_vec()),
        }
    }
    
    /// Compute circular boundary
    fn compute_circle_boundary(&self, positions: &[Point3<f32>]) -> Result<Vec<Point3<f32>>> {
        let center = self.compute_center(positions);
        let radius = self.compute_radius(positions, center) + self.padding;
        
        let mut points = Vec::new();
        let segments = 32; // Number of circle segments
        
        for i in 0..segments {
            let angle = 2.0 * std::f32::consts::PI * i as f32 / segments as f32;
            let x = center.x + radius * angle.cos();
            let y = center.y + radius * angle.sin();
            let z = center.z; // Keep same Z level for 2D circle
            points.push(Point3::new(x, y, z));
        }
        
        Ok(points)
    }
    
    /// Compute convex hull (simplified 2D version)
    fn compute_convex_hull(&self, positions: &[Point3<f32>]) -> Result<Vec<Point3<f32>>> {
        if positions.len() < 3 {
            return self.compute_circle_boundary(positions);
        }
        
        // Project to 2D and compute convex hull using Graham scan
        let mut points_2d: Vec<(Point3<f32>, usize)> = positions.iter()
            .enumerate()
            .map(|(i, p)| (*p, i))
            .collect();
        
        // Find bottom-most point (or left-most in case of tie)
        points_2d.sort_by(|a, b| {
            let cmp_y = a.0.y.partial_cmp(&b.0.y).unwrap();
            if cmp_y == std::cmp::Ordering::Equal {
                a.0.x.partial_cmp(&b.0.x).unwrap()
            } else {
                cmp_y
            }
        });
        
        let start = points_2d[0].0;
        
        // Sort by polar angle with respect to start point
        points_2d[1..].sort_by(|a, b| {
            let angle_a = (a.0.y - start.y).atan2(a.0.x - start.x);
            let angle_b = (b.0.y - start.y).atan2(b.0.x - start.x);
            angle_a.partial_cmp(&angle_b).unwrap()
        });
        
        // Graham scan
        let mut hull = Vec::new();
        
        for (point, _) in points_2d {
            while hull.len() > 1 {
                let p1: Point3<f32> = hull[hull.len()-2];
                let p2: Point3<f32> = hull[hull.len()-1];
                
                // Cross product to determine turn direction
                let cross = (p2.x - p1.x) * (point.y - p1.y) - (p2.y - p1.y) * (point.x - p1.x);
                if cross <= 0.0 {
                    hull.pop();
                } else {
                    break;
                }
            }
            hull.push(point);
        }
        
        // Add padding by expanding outward
        let center = self.compute_center(&hull);
        let padded_hull: Vec<Point3<f32>> = hull.iter().map(|p| {
            let direction = (*p - center).normalize();
            *p + direction * self.padding
        }).collect();
        
        Ok(padded_hull)
    }
    
    /// Compute bounding box
    fn compute_bounding_box(&self, positions: &[Point3<f32>]) -> Result<Vec<Point3<f32>>> {
        if positions.is_empty() {
            return Ok(Vec::new());
        }
        
        let mut min_x = positions[0].x;
        let mut max_x = positions[0].x;
        let mut min_y = positions[0].y;
        let mut max_y = positions[0].y;
        let mut min_z = positions[0].z;
        let mut max_z = positions[0].z;
        
        for pos in positions {
            min_x = min_x.min(pos.x);
            max_x = max_x.max(pos.x);
            min_y = min_y.min(pos.y);
            max_y = max_y.max(pos.y);
            min_z = min_z.min(pos.z);
            max_z = max_z.max(pos.z);
        }
        
        // Add padding
        min_x -= self.padding;
        max_x += self.padding;
        min_y -= self.padding;
        max_y += self.padding;
        
        // Average Z for 2D bounding box
        let avg_z = (min_z + max_z) / 2.0;
        
        Ok(vec![
            Point3::new(min_x, min_y, avg_z),
            Point3::new(max_x, min_y, avg_z),
            Point3::new(max_x, max_y, avg_z),
            Point3::new(min_x, max_y, avg_z),
        ])
    }
    
    /// Compute alpha shape (simplified version)
    fn compute_alpha_shape(&self, positions: &[Point3<f32>]) -> Result<Vec<Point3<f32>>> {
        // For now, use convex hull as alpha shape implementation is complex
        // A full alpha shape would require Delaunay triangulation and edge filtering
        self.compute_convex_hull(positions)
    }
    
    /// Compute center point of positions
    fn compute_center(&self, positions: &[Point3<f32>]) -> Point3<f32> {
        if positions.is_empty() {
            return Point3::origin();
        }
        
        let sum = positions.iter().fold(Vector3::zeros(), |acc, p| acc + p.coords);
        Point3::from(sum / positions.len() as f32)
    }
    
    /// Compute radius from center to furthest point
    fn compute_radius(&self, positions: &[Point3<f32>], center: Point3<f32>) -> f32 {
        positions.iter()
            .map(|p| (p - center).magnitude())
            .fold(0.0, f32::max)
    }
    
    /// Update cluster boundary
    pub fn update_cluster_boundary(&mut self, cluster_id: ClusterId, boundary: ClusterBoundary) {
        self.boundaries.insert(cluster_id, boundary);
    }
    
    /// Get cluster boundary
    pub fn get_cluster_boundary(&self, cluster_id: ClusterId) -> Option<&ClusterBoundary> {
        self.boundaries.get(&cluster_id)
    }
    
    /// Remove cluster boundary
    pub fn remove_cluster_boundary(&mut self, cluster_id: ClusterId) -> Option<ClusterBoundary> {
        self.boundaries.remove(&cluster_id)
    }
    
    /// Get all boundaries
    pub fn get_all_boundaries(&self) -> &HashMap<ClusterId, ClusterBoundary> {
        &self.boundaries
    }
    
    /// Clear all boundaries
    pub fn clear_boundaries(&mut self) {
        self.boundaries.clear();
    }
    
    /// Set boundary padding
    pub fn set_padding(&mut self, padding: f32) {
        self.padding = padding;
    }
    
    /// Get boundary padding
    pub fn padding(&self) -> f32 {
        self.padding
    }
    
    /// Check if point is inside cluster boundary
    pub fn point_in_cluster(&self, cluster_id: ClusterId, point: Point3<f32>) -> bool {
        if let Some(boundary) = self.boundaries.get(&cluster_id) {
            match boundary.boundary_type {
                BoundaryType::Circle => {
                    let distance = (point - boundary.center).magnitude();
                    distance <= boundary.radius
                },
                BoundaryType::BoundingBox => {
                    if boundary.points.len() >= 4 {
                        let min_x = boundary.points.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
                        let max_x = boundary.points.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max);
                        let min_y = boundary.points.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
                        let max_y = boundary.points.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max);
                        
                        point.x >= min_x && point.x <= max_x && point.y >= min_y && point.y <= max_y
                    } else {
                        false
                    }
                },
                BoundaryType::ConvexHull | BoundaryType::AlphaShape | BoundaryType::Custom => {
                    // Point-in-polygon test using ray casting
                    self.point_in_polygon(point, &boundary.points)
                },
            }
        } else {
            false
        }
    }
    
    /// Point-in-polygon test using ray casting algorithm
    fn point_in_polygon(&self, point: Point3<f32>, polygon: &[Point3<f32>]) -> bool {
        if polygon.len() < 3 {
            return false;
        }
        
        let mut inside = false;
        let mut j = polygon.len() - 1;
        
        for i in 0..polygon.len() {
            let xi = polygon[i].x;
            let yi = polygon[i].y;
            let xj = polygon[j].x;
            let yj = polygon[j].y;
            
            if ((yi > point.y) != (yj > point.y)) &&
               (point.x < (xj - xi) * (point.y - yi) / (yj - yi) + xi) {
                inside = !inside;
            }
            j = i;
        }
        
        inside
    }
}

impl Default for BoundaryRenderer {
    fn default() -> Self {
        Self::new()
    }
}