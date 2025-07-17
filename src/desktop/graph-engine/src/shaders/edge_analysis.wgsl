// Edge content analysis compute shader for GPU acceleration

struct EdgeData {
    source_id: u32,
    target_id: u32,
    edge_type: u32,
    strength: f32,
    source_pos: vec3<f32>,
    target_pos: vec3<f32>,
    source_created_at: u64,
    target_created_at: u64,
    source_content_hash: u32,
    target_content_hash: u32,
    source_tag_hash: u32,
    target_tag_hash: u32,
}

struct AnalysisResult {
    edge_id: u32,
    relationship_type: u32,
    strength: f32,
    confidence: f32,
    directionality: f32,
    temporal_score: f32,
    spatial_score: f32,
    semantic_score: f32,
}

struct AnalysisConfig {
    temporal_weight: f32,
    spatial_weight: f32,
    semantic_weight: f32,
    max_analysis_distance: f32,
    min_edge_strength: f32,
    camera_pos: vec3<f32>,
    current_time: u64,
}

@group(0) @binding(0)
var<storage, read> edge_data: array<EdgeData>;

@group(0) @binding(1)
var<storage, read_write> results: array<AnalysisResult>;

@group(0) @binding(2)
var<uniform> config: AnalysisConfig;

// Hash function for content comparison
fn hash_similarity(hash1: u32, hash2: u32) -> f32 {
    let xor_result = hash1 ^ hash2;
    let similarity = f32(32u - countOneBits(xor_result)) / 32.0;
    return similarity;
}

// Calculate temporal similarity
fn calculate_temporal_similarity(created1: u64, created2: u64, current_time: u64) -> f32 {
    let time_diff = abs(i64(created1) - i64(created2));
    let hours_diff = f32(time_diff) / 3600.0;
    let similarity = 1.0 / (1.0 + hours_diff / 24.0);
    return min(similarity, 1.0);
}

// Calculate spatial similarity
fn calculate_spatial_similarity(pos1: vec3<f32>, pos2: vec3<f32>, camera_pos: vec3<f32>) -> f32 {
    let distance = length(pos1 - pos2);
    let avg_pos = (pos1 + pos2) / 2.0;
    let view_distance = length(camera_pos - avg_pos);
    let normalized_distance = distance / max(view_distance, 1.0);
    let similarity = 1.0 / (1.0 + normalized_distance);
    return min(similarity, 1.0);
}

// Calculate semantic similarity
fn calculate_semantic_similarity(content_hash1: u32, content_hash2: u32, tag_hash1: u32, tag_hash2: u32) -> f32 {
    let content_similarity = hash_similarity(content_hash1, content_hash2);
    let tag_similarity = hash_similarity(tag_hash1, tag_hash2);
    return (content_similarity * 0.7 + tag_similarity * 0.3);
}

// Classify relationship type
fn classify_relationship_type(edge_type: u32, temporal_score: f32, spatial_score: f32, semantic_score: f32) -> u32 {
    switch edge_type {
        case 0u: { return 0u; } // DataFlow
        case 1u: { return 1u; } // Dependency
        case 2u: { return 2u; } // Hierarchical
        case 3u: { 
            // Relationship - use scores to determine specific type
            if temporal_score > 0.7 {
                return 4u; // Temporal
            } else if spatial_score > 0.7 {
                return 5u; // Spatial
            } else if semantic_score > 0.7 {
                return 3u; // Associative
            } else {
                return 6u; // Unknown
            }
        }
        default: { return 6u; } // Unknown
    }
}

// Calculate directionality
fn calculate_directionality(source_created: u64, target_created: u64) -> f32 {
    let time_diff = i64(source_created) - i64(target_created);
    let time_directionality = sign(f32(time_diff)) * tanh(abs(f32(time_diff)) / 3600.0);
    return time_directionality;
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    
    if index >= arrayLength(&edge_data) {
        return;
    }
    
    let edge = edge_data[index];
    
    // Calculate similarity scores
    let temporal_score = calculate_temporal_similarity(
        edge.source_created_at, 
        edge.target_created_at, 
        config.current_time
    );
    
    let spatial_score = calculate_spatial_similarity(
        edge.source_pos, 
        edge.target_pos, 
        config.camera_pos
    );
    
    let semantic_score = calculate_semantic_similarity(
        edge.source_content_hash, 
        edge.target_content_hash, 
        edge.source_tag_hash, 
        edge.target_tag_hash
    );
    
    // Classify relationship type
    let relationship_type = classify_relationship_type(
        edge.edge_type, 
        temporal_score, 
        spatial_score, 
        semantic_score
    );
    
    // Calculate overall strength
    let strength = (temporal_score * config.temporal_weight +
                   spatial_score * config.spatial_weight +
                   semantic_score * config.semantic_weight) / 
                  (config.temporal_weight + config.spatial_weight + config.semantic_weight);
    
    // Calculate confidence based on consistency of scores
    let score_variance = ((temporal_score - strength) * (temporal_score - strength) +
                         (spatial_score - strength) * (spatial_score - strength) +
                         (semantic_score - strength) * (semantic_score - strength)) / 3.0;
    let confidence = 1.0 - sqrt(score_variance);
    
    // Calculate directionality
    let directionality = calculate_directionality(edge.source_created_at, edge.target_created_at);
    
    // Only process if strength meets minimum threshold
    if strength >= config.min_edge_strength {
        results[index] = AnalysisResult(
            edge.source_id, // Using source_id as edge identifier
            relationship_type,
            strength,
            confidence,
            directionality,
            temporal_score,
            spatial_score,
            semantic_score
        );
    } else {
        // Mark as invalid/filtered out
        results[index] = AnalysisResult(
            0u,
            6u, // Unknown
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0
        );
    }
}