package org.horizonos.config.dsl.graphdesktop.ai

import kotlinx.serialization.Serializable

// ===== AI Integration Configuration =====

@Serializable
data class GraphAIConfig(
    val enabled: Boolean = false,
    val provider: GraphAIProvider = GraphAIProvider.LOCAL_OLLAMA,
    val model: String = "llama3.2",
    val apiEndpoint: String? = null,
    val apiKey: String? = null,
    val features: List<AIFeature> = emptyList(),
    val suggestions: AISuggestionConfig = AISuggestionConfig(),
    val clustering: AIClusteringConfig = AIClusteringConfig(),
    val search: AISearchConfig = AISearchConfig()
)

@Serializable
data class AISuggestionConfig(
    val enabled: Boolean = true,
    val maxSuggestions: Int = 5,
    val triggerDelay: Int = 500,
    val contextWindow: Int = 10,
    val suggestionTypes: List<String> = listOf("connections", "nodes", "layouts")
)

@Serializable
data class AIClusteringConfig(
    val enabled: Boolean = false,
    val algorithm: ClusteringAlgorithm = ClusteringAlgorithm.HIERARCHICAL,
    val minClusterSize: Int = 3,
    val maxClusters: Int = 10,
    val similarity: Double = 0.7
)

@Serializable
data class AISearchConfig(
    val enabled: Boolean = true,
    val fuzzySearch: Boolean = true,
    val semanticSearch: Boolean = true,
    val maxResults: Int = 20,
    val includeMetadata: Boolean = true
)

// AI Enums
@Serializable
enum class GraphAIProvider {
    LOCAL_OLLAMA,
    OPENAI,
    ANTHROPIC,
    GOOGLE_VERTEX,
    CUSTOM
}

@Serializable
enum class AIFeature {
    AUTO_LAYOUT,
    NODE_SUGGESTIONS,
    EDGE_PREDICTIONS,
    CLUSTERING,
    ANOMALY_DETECTION,
    PATTERN_RECOGNITION,
    NATURAL_LANGUAGE_QUERY,
    SEMANTIC_SEARCH
}

@Serializable
enum class ClusteringAlgorithm {
    HIERARCHICAL,
    K_MEANS,
    DBSCAN,
    SPECTRAL,
    COMMUNITY_DETECTION
}