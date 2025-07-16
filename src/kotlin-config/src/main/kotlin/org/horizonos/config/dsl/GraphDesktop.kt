package org.horizonos.config.dsl

import kotlinx.serialization.Serializable
import org.horizonos.config.dsl.graphdesktop.*
import org.horizonos.config.dsl.graphdesktop.nodes.*
import org.horizonos.config.dsl.graphdesktop.edges.*
import org.horizonos.config.dsl.graphdesktop.layout.*
import org.horizonos.config.dsl.graphdesktop.interaction.*
import org.horizonos.config.dsl.graphdesktop.visual.*
import org.horizonos.config.dsl.graphdesktop.ai.*
import org.horizonos.config.dsl.graphdesktop.workspace.*

// ===== Graph Desktop Configuration DSL (FLAGSHIP FEATURE) =====

@HorizonOSDsl
class GraphDesktopContext {
    var enabled: Boolean = true
    var renderingEngine: RenderingEngine = RenderingEngine.WEBGPU
    var enablePhysics: Boolean = true
    var enableGestures: Boolean = true
    var enableKeyboardNavigation: Boolean = true
    var enableVoiceControl: Boolean = false
    var maxNodes: Int = 10000
    var maxEdges: Int = 50000
    var performanceMode: PerformanceMode = PerformanceMode.BALANCED
    
    internal val nodeTypes = mutableListOf<NodeTypeDefinition>()
    internal val edgeTypes = mutableListOf<EdgeTypeDefinition>()
    internal val layouts = mutableListOf<LayoutAlgorithmConfig>()
    internal val interactions = mutableListOf<InteractionConfig>()
    internal val visualEffects = mutableListOf<VisualEffectConfig>()
    internal val semanticRules = mutableListOf<SemanticRule>()
    internal val aiIntegration = mutableListOf<GraphAIConfig>()
    internal val workspaces = mutableListOf<GraphWorkspaceConfig>()
    internal val themes = mutableListOf<GraphThemeConfig>()
    internal val gestures = mutableListOf<GestureConfig>()

    fun nodeType(name: String, block: NodeTypeContext.() -> Unit) {
        nodeTypes.add(NodeTypeContext(name).apply(block).toDefinition())
    }

    fun edgeType(name: String, block: EdgeTypeContext.() -> Unit) {
        edgeTypes.add(EdgeTypeContext(name).apply(block).toDefinition())
    }

    fun layout(algorithm: LayoutAlgorithm, block: LayoutAlgorithmContext.() -> Unit) {
        layouts.add(LayoutAlgorithmContext(algorithm).apply(block).toConfig())
    }

    fun interaction(type: InteractionType, block: InteractionContext.() -> Unit) {
        interactions.add(InteractionContext(type).apply(block).toConfig())
    }

    fun visualEffect(type: VisualEffectType, block: VisualEffectContext.() -> Unit) {
        visualEffects.add(VisualEffectContext(type).apply(block).toConfig())
    }

    fun semanticRule(block: SemanticRuleContext.() -> Unit) {
        semanticRules.add(SemanticRuleContext().apply(block).toRule())
    }

    fun aiIntegration(block: GraphAIContext.() -> Unit) {
        aiIntegration.add(GraphAIContext().apply(block).toConfig())
    }

    fun workspace(name: String, block: GraphWorkspaceContext.() -> Unit) {
        workspaces.add(GraphWorkspaceContext(name).apply(block).toConfig())
    }

    fun theme(name: String, block: GraphThemeContext.() -> Unit) {
        themes.add(GraphThemeContext(name).apply(block).toConfig())
    }

    fun gesture(name: String, block: GestureContext.() -> Unit) {
        gestures.add(GestureContext(name).apply(block).toConfig())
    }

    fun toConfig(): GraphDesktopConfig {
        return GraphDesktopConfig(
            enabled = enabled,
            renderingEngine = renderingEngine,
            enablePhysics = enablePhysics,
            enableGestures = enableGestures,
            enableKeyboardNavigation = enableKeyboardNavigation,
            enableVoiceControl = enableVoiceControl,
            maxNodes = maxNodes,
            maxEdges = maxEdges,
            performanceMode = performanceMode,
            nodeTypes = nodeTypes,
            edgeTypes = edgeTypes,
            layouts = layouts,
            interactions = interactions,
            visualEffects = visualEffects,
            semanticRules = semanticRules,
            aiIntegration = aiIntegration,
            workspaces = workspaces,
            themes = themes,
            gestures = gestures
        )
    }
}

// ===== Graph Desktop DSL Function =====

@HorizonOSDsl
fun graphDesktop(block: GraphDesktopContext.() -> Unit): GraphDesktopConfig =
    GraphDesktopContext().apply(block).toConfig()