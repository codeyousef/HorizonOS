package org.horizonos.config.dsl.graphdesktop

import kotlinx.serialization.Serializable
import org.horizonos.config.dsl.graphdesktop.nodes.*
import org.horizonos.config.dsl.graphdesktop.edges.*
import org.horizonos.config.dsl.graphdesktop.layout.*
import org.horizonos.config.dsl.graphdesktop.interaction.*
import org.horizonos.config.dsl.graphdesktop.visual.*
import org.horizonos.config.dsl.graphdesktop.ai.*
import org.horizonos.config.dsl.graphdesktop.workspace.*

// ===== Graph Desktop Configuration =====

@Serializable
data class GraphDesktopConfig(
    val enabled: Boolean = true,
    val renderingEngine: RenderingEngine = RenderingEngine.WEBGPU,
    val enablePhysics: Boolean = true,
    val enableGestures: Boolean = true,
    val enableKeyboardNavigation: Boolean = true,
    val enableVoiceControl: Boolean = false,
    val maxNodes: Int = 10000,
    val maxEdges: Int = 50000,
    val performanceMode: PerformanceMode = PerformanceMode.BALANCED,
    val nodeTypes: List<NodeTypeDefinition> = emptyList(),
    val edgeTypes: List<EdgeTypeDefinition> = emptyList(),
    val layouts: List<LayoutAlgorithmConfig> = emptyList(),
    val interactions: List<InteractionConfig> = emptyList(),
    val visualEffects: List<VisualEffectConfig> = emptyList(),
    val semanticRules: List<SemanticRule> = emptyList(),
    val aiIntegration: List<GraphAIConfig> = emptyList(),
    val workspaces: List<GraphWorkspaceConfig> = emptyList(),
    val themes: List<GraphThemeConfig> = emptyList(),
    val gestures: List<GestureConfig> = emptyList()
)

// Core Enums
@Serializable
enum class RenderingEngine {
    WEBGPU,
    WEBGL,
    CANVAS2D,
    SVG,
    HYBRID
}

@Serializable
enum class PerformanceMode {
    LOW_POWER,
    BALANCED,
    HIGH_PERFORMANCE,
    ADAPTIVE
}