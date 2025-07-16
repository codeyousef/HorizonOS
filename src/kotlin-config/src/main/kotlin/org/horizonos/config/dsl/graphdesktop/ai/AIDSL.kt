package org.horizonos.config.dsl.graphdesktop.ai

import org.horizonos.config.dsl.HorizonOSDsl

// ===== AI DSL Builders =====

@HorizonOSDsl
class GraphAIContext {
    var enabled: Boolean = false
    var provider: GraphAIProvider = GraphAIProvider.LOCAL_OLLAMA
    var model: String = "llama3.2"
    var apiEndpoint: String? = null
    var apiKey: String? = null
    private val features = mutableListOf<AIFeature>()
    private var suggestions = AISuggestionConfig()
    private var clustering = AIClusteringConfig()
    private var search = AISearchConfig()
    
    fun feature(feat: AIFeature) {
        features.add(feat)
    }
    
    fun suggestions(block: AISuggestionContext.() -> Unit) {
        suggestions = AISuggestionContext().apply(block).toConfig()
    }
    
    fun clustering(block: AIClusteringContext.() -> Unit) {
        clustering = AIClusteringContext().apply(block).toConfig()
    }
    
    fun search(block: AISearchContext.() -> Unit) {
        search = AISearchContext().apply(block).toConfig()
    }
    
    fun toConfig(): GraphAIConfig {
        return GraphAIConfig(
            enabled = enabled,
            provider = provider,
            model = model,
            apiEndpoint = apiEndpoint,
            apiKey = apiKey,
            features = features,
            suggestions = suggestions,
            clustering = clustering,
            search = search
        )
    }
}

@HorizonOSDsl
class AISuggestionContext {
    var enabled: Boolean = true
    var maxSuggestions: Int = 5
    var triggerDelay: Int = 500
    var contextWindow: Int = 10
    private val suggestionTypes = mutableListOf("connections", "nodes", "layouts")
    
    fun suggestionType(type: String) {
        suggestionTypes.add(type)
    }
    
    fun toConfig(): AISuggestionConfig {
        return AISuggestionConfig(
            enabled = enabled,
            maxSuggestions = maxSuggestions,
            triggerDelay = triggerDelay,
            contextWindow = contextWindow,
            suggestionTypes = suggestionTypes
        )
    }
}

@HorizonOSDsl
class AIClusteringContext {
    var enabled: Boolean = false
    var algorithm: ClusteringAlgorithm = ClusteringAlgorithm.HIERARCHICAL
    var minClusterSize: Int = 3
    var maxClusters: Int = 10
    var similarity: Double = 0.7
    
    fun toConfig(): AIClusteringConfig {
        return AIClusteringConfig(
            enabled = enabled,
            algorithm = algorithm,
            minClusterSize = minClusterSize,
            maxClusters = maxClusters,
            similarity = similarity
        )
    }
}

@HorizonOSDsl
class AISearchContext {
    var enabled: Boolean = true
    var fuzzySearch: Boolean = true
    var semanticSearch: Boolean = true
    var maxResults: Int = 20
    var includeMetadata: Boolean = true
    
    fun toConfig(): AISearchConfig {
        return AISearchConfig(
            enabled = enabled,
            fuzzySearch = fuzzySearch,
            semanticSearch = semanticSearch,
            maxResults = maxResults,
            includeMetadata = includeMetadata
        )
    }
}