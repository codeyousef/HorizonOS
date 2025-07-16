package org.horizonos.config.dsl

import kotlinx.serialization.Serializable

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

// ===== Node Type Configuration =====

@HorizonOSDsl
class NodeTypeContext(private val name: String) {
    var displayName: String = name
    var description: String? = null
    var category: NodeCategory = NodeCategory.GENERIC
    var shape: NodeShape = NodeShape.CIRCLE
    var size: NodeSize = NodeSize.MEDIUM
    var color: String = "#3498db"
    var icon: String? = null
    var draggable: Boolean = true
    var selectable: Boolean = true
    var resizable: Boolean = false
    var deletable: Boolean = true
    var connectable: Boolean = true
    var maxConnections: Int = -1
    var physics: NodePhysics? = null
    var content: NodeContent? = null
    var behavior: NodeBehavior? = null

    fun physics(block: NodePhysicsContext.() -> Unit) {
        physics = NodePhysicsContext().apply(block).toPhysics()
    }

    fun content(block: NodeContentContext.() -> Unit) {
        content = NodeContentContext().apply(block).toContent()
    }

    fun behavior(block: NodeBehaviorContext.() -> Unit) {
        behavior = NodeBehaviorContext().apply(block).toBehavior()
    }

    fun toDefinition(): NodeTypeDefinition {
        return NodeTypeDefinition(
            name = name,
            displayName = displayName,
            description = description,
            category = category,
            shape = shape,
            size = size,
            color = color,
            icon = icon,
            draggable = draggable,
            selectable = selectable,
            resizable = resizable,
            deletable = deletable,
            connectable = connectable,
            maxConnections = maxConnections,
            physics = physics,
            content = content,
            behavior = behavior
        )
    }
}

@HorizonOSDsl
class NodePhysicsContext {
    var mass: Double = 1.0
    var charge: Double = -300.0
    var gravity: Double = 0.1
    var friction: Double = 0.9
    var fixed: Boolean = false

    fun toPhysics(): NodePhysics {
        return NodePhysics(
            mass = mass,
            charge = charge,
            gravity = gravity,
            friction = friction,
            fixed = fixed
        )
    }
}

@HorizonOSDsl
class NodeContentContext {
    var type: ContentType = ContentType.TEXT
    var data: String? = null
    var template: String? = null
    var editable: Boolean = false
    var richText: Boolean = false
    var maxLength: Int? = null

    fun toContent(): NodeContent {
        return NodeContent(
            type = type,
            data = data,
            template = template,
            editable = editable,
            richText = richText,
            maxLength = maxLength
        )
    }
}

@HorizonOSDsl
class NodeBehaviorContext {
    var autoLayout: Boolean = true
    var cluster: Boolean = false
    var expandable: Boolean = false
    var collapsible: Boolean = false
    var animations: List<NodeAnimation> = emptyList()
    var events = mutableMapOf<String, String>()

    fun event(trigger: String, action: String) {
        events[trigger] = action
    }

    fun toBehavior(): NodeBehavior {
        return NodeBehavior(
            autoLayout = autoLayout,
            cluster = cluster,
            expandable = expandable,
            collapsible = collapsible,
            animations = animations,
            events = events.toMap()
        )
    }
}

// ===== Edge Type Configuration =====

@HorizonOSDsl
class EdgeTypeContext(private val name: String) {
    var displayName: String = name
    var description: String? = null
    var category: EdgeCategory = EdgeCategory.RELATIONSHIP
    var style: EdgeStyle = EdgeStyle.SOLID
    var width: Double = 2.0
    var color: String = "#7f8c8d"
    var curved: Boolean = true
    var directed: Boolean = false
    var animated: Boolean = false
    var label: String? = null
    var weight: Double = 1.0
    var bidirectional: Boolean = false
    var selectable: Boolean = true
    var deletable: Boolean = true

    fun toDefinition(): EdgeTypeDefinition {
        return EdgeTypeDefinition(
            name = name,
            displayName = displayName,
            description = description,
            category = category,
            style = style,
            width = width,
            color = color,
            curved = curved,
            directed = directed,
            animated = animated,
            label = label,
            weight = weight,
            bidirectional = bidirectional,
            selectable = selectable,
            deletable = deletable
        )
    }
}

// ===== Layout Algorithm Configuration =====

@HorizonOSDsl
class LayoutAlgorithmContext(private val algorithm: LayoutAlgorithm) {
    var enabled: Boolean = true
    var primary: Boolean = false
    var parameters = mutableMapOf<String, Double>()
    var constraints = mutableListOf<LayoutConstraint>()

    fun parameter(name: String, value: Double) {
        parameters[name] = value
    }

    fun constraint(type: ConstraintType, value: Double) {
        constraints.add(LayoutConstraint(type, value))
    }

    fun toConfig(): LayoutAlgorithmConfig {
        return LayoutAlgorithmConfig(
            algorithm = algorithm,
            enabled = enabled,
            primary = primary,
            parameters = parameters.toMap(),
            constraints = constraints
        )
    }
}

// ===== Interaction Configuration =====

@HorizonOSDsl
class InteractionContext(private val type: InteractionType) {
    var enabled: Boolean = true
    var sensitivity: Double = 1.0
    var multiTouch: Boolean = true
    var gestures = mutableListOf<String>()
    var keyBindings = mutableMapOf<String, String>()

    fun gesture(name: String) {
        gestures.add(name)
    }

    fun keyBinding(key: String, action: String) {
        keyBindings[key] = action
    }

    fun toConfig(): InteractionConfig {
        return InteractionConfig(
            type = type,
            enabled = enabled,
            sensitivity = sensitivity,
            multiTouch = multiTouch,
            gestures = gestures,
            keyBindings = keyBindings.toMap()
        )
    }
}

// ===== Visual Effects Configuration =====

@HorizonOSDsl
class VisualEffectContext(private val type: VisualEffectType) {
    var enabled: Boolean = true
    var intensity: Double = 1.0
    var duration: Int = 300
    var easing: EasingFunction = EasingFunction.EASE_IN_OUT
    var settings = mutableMapOf<String, String>()

    fun setting(key: String, value: String) {
        settings[key] = value
    }

    fun toConfig(): VisualEffectConfig {
        return VisualEffectConfig(
            type = type,
            enabled = enabled,
            intensity = intensity,
            duration = duration,
            easing = easing,
            settings = settings.toMap()
        )
    }
}

// ===== Semantic Rules =====

@HorizonOSDsl
class SemanticRuleContext {
    var name: String = ""
    var condition: String = ""
    var action: SemanticAction = SemanticAction.AUTO_CONNECT
    var priority: Int = 50
    var enabled: Boolean = true
    var parameters = mutableMapOf<String, String>()

    fun parameter(key: String, value: String) {
        parameters[key] = value
    }

    fun toRule(): SemanticRule {
        return SemanticRule(
            name = name,
            condition = condition,
            action = action,
            priority = priority,
            enabled = enabled,
            parameters = parameters.toMap()
        )
    }
}

// ===== Graph AI Integration =====

@HorizonOSDsl
class GraphAIContext {
    var enabled: Boolean = true
    var provider: GraphAIProvider = GraphAIProvider.LOCAL_LLM
    var model: String = "llama2"
    var features = mutableListOf<AIFeature>()
    var suggestions: AISuggestionConfig? = null
    var clustering: AIClusteringConfig? = null
    var search: AISearchConfig? = null

    fun feature(feature: AIFeature) {
        features.add(feature)
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
    var threshold: Double = 0.7
    var maxSuggestions: Int = 5
    var contextWindow: Int = 100

    fun toConfig(): AISuggestionConfig {
        return AISuggestionConfig(
            enabled = enabled,
            threshold = threshold,
            maxSuggestions = maxSuggestions,
            contextWindow = contextWindow
        )
    }
}

@HorizonOSDsl
class AIClusteringContext {
    var enabled: Boolean = true
    var algorithm: ClusteringAlgorithm = ClusteringAlgorithm.DBSCAN
    var minClusterSize: Int = 3
    var maxClusters: Int = 20

    fun toConfig(): AIClusteringConfig {
        return AIClusteringConfig(
            enabled = enabled,
            algorithm = algorithm,
            minClusterSize = minClusterSize,
            maxClusters = maxClusters
        )
    }
}

@HorizonOSDsl
class AISearchContext {
    var enabled: Boolean = true
    var semantic: Boolean = true
    var fuzzy: Boolean = true
    var maxResults: Int = 50

    fun toConfig(): AISearchConfig {
        return AISearchConfig(
            enabled = enabled,
            semantic = semantic,
            fuzzy = fuzzy,
            maxResults = maxResults
        )
    }
}

// ===== Graph Workspace Configuration =====

@HorizonOSDsl
class GraphWorkspaceContext(private val name: String) {
    var enabled: Boolean = true
    var defaultLayout: LayoutAlgorithm = LayoutAlgorithm.FORCE_DIRECTED
    var maxNodes: Int = 1000
    var persistence: Boolean = true
    var collaborative: Boolean = false
    var filters = mutableListOf<WorkspaceFilter>()

    fun filter(type: FilterType, condition: String) {
        filters.add(WorkspaceFilter(type, condition))
    }

    fun toConfig(): GraphWorkspaceConfig {
        return GraphWorkspaceConfig(
            name = name,
            enabled = enabled,
            defaultLayout = defaultLayout,
            maxNodes = maxNodes,
            persistence = persistence,
            collaborative = collaborative,
            filters = filters
        )
    }
}

// ===== Graph Theme Configuration =====

@HorizonOSDsl
class GraphThemeContext(private val name: String) {
    var enabled: Boolean = true
    var darkMode: Boolean = false
    var backgroundColor: String = "#ffffff"
    var gridColor: String = "#f0f0f0"
    var selectionColor: String = "#3498db"
    var highlightColor: String = "#e74c3c"
    var nodeColors = mutableMapOf<String, String>()
    var edgeColors = mutableMapOf<String, String>()
    var fonts = mutableMapOf<String, String>()

    fun nodeColor(category: String, color: String) {
        nodeColors[category] = color
    }

    fun edgeColor(category: String, color: String) {
        edgeColors[category] = color
    }

    fun font(element: String, font: String) {
        fonts[element] = font
    }

    fun toConfig(): GraphThemeConfig {
        return GraphThemeConfig(
            name = name,
            enabled = enabled,
            darkMode = darkMode,
            backgroundColor = backgroundColor,
            gridColor = gridColor,
            selectionColor = selectionColor,
            highlightColor = highlightColor,
            nodeColors = nodeColors.toMap(),
            edgeColors = edgeColors.toMap(),
            fonts = fonts.toMap()
        )
    }
}

// ===== Gesture Configuration =====

@HorizonOSDsl
class GestureContext(private val name: String) {
    var enabled: Boolean = true
    var fingers: Int = 2
    var direction: GestureDirection = GestureDirection.ANY
    var action: String = ""
    var sensitivity: Double = 1.0

    fun toConfig(): GestureConfig {
        return GestureConfig(
            name = name,
            enabled = enabled,
            fingers = fingers,
            direction = direction,
            action = action,
            sensitivity = sensitivity
        )
    }
}

// ===== Enums =====

@Serializable
enum class RenderingEngine {
    WEBGPU, OPENGL, VULKAN, SOFTWARE
}

@Serializable
enum class PerformanceMode {
    HIGH_PERFORMANCE, BALANCED, POWER_SAVE
}

@Serializable
enum class NodeCategory {
    GENERIC, APPLICATION, FILE, FOLDER, DOCUMENT, IMAGE, VIDEO, AUDIO, LINK, BOOKMARK, TASK, CONTACT, LOCATION, SYSTEM
}

@Serializable
enum class NodeShape {
    CIRCLE, SQUARE, RECTANGLE, TRIANGLE, DIAMOND, HEXAGON, CUSTOM
}

@Serializable
enum class NodeSize {
    TINY, SMALL, MEDIUM, LARGE, HUGE, CUSTOM
}

@Serializable
enum class ContentType {
    TEXT, HTML, MARKDOWN, IMAGE, VIDEO, AUDIO, CUSTOM
}

@Serializable
enum class NodeAnimation {
    PULSE, GLOW, ROTATE, BOUNCE, SHAKE, FADE
}

@Serializable
enum class EdgeCategory {
    RELATIONSHIP, DEPENDENCY, FLOW, HIERARCHY, ASSOCIATION, COMMUNICATION
}

@Serializable
enum class EdgeStyle {
    SOLID, DASHED, DOTTED, THICK, THIN
}

@Serializable
enum class LayoutAlgorithm {
    FORCE_DIRECTED, HIERARCHICAL, CIRCULAR, GRID, TREE, DAGRE, COLA
}

@Serializable
enum class ConstraintType {
    MIN_DISTANCE, MAX_DISTANCE, ALIGNMENT, HIERARCHY_LEVEL
}

@Serializable
enum class InteractionType {
    MOUSE, TOUCH, KEYBOARD, VOICE, EYE_TRACKING
}

@Serializable
enum class VisualEffectType {
    PARTICLES, TRAILS, GLOW, SHADOW, OUTLINE, HIGHLIGHT
}

@Serializable
enum class EasingFunction {
    LINEAR, EASE_IN, EASE_OUT, EASE_IN_OUT, BOUNCE, ELASTIC
}

@Serializable
enum class SemanticAction {
    AUTO_CONNECT, AUTO_CLUSTER, AUTO_LAYOUT, SUGGEST_RELATIONSHIP, HIGHLIGHT_PATTERN
}

@Serializable
enum class GraphAIProvider {
    LOCAL_LLM, OPENAI, ANTHROPIC, CUSTOM
}

@Serializable
enum class AIFeature {
    SUGGESTIONS, CLUSTERING, SEARCH, SUMMARIZATION, CLASSIFICATION, RELATIONSHIP_DETECTION
}

@Serializable
enum class ClusteringAlgorithm {
    DBSCAN, KMEANS, HIERARCHICAL, SPECTRAL
}

@Serializable
enum class FilterType {
    NODE_TYPE, EDGE_TYPE, CATEGORY, PROPERTY, DATE_RANGE
}

@Serializable
enum class GestureDirection {
    UP, DOWN, LEFT, RIGHT, CLOCKWISE, COUNTERCLOCKWISE, PINCH, SPREAD, ANY
}

// ===== Data Classes =====

@Serializable
data class GraphDesktopConfig(
    val enabled: Boolean,
    val renderingEngine: RenderingEngine,
    val enablePhysics: Boolean,
    val enableGestures: Boolean,
    val enableKeyboardNavigation: Boolean,
    val enableVoiceControl: Boolean,
    val maxNodes: Int,
    val maxEdges: Int,
    val performanceMode: PerformanceMode,
    val nodeTypes: List<NodeTypeDefinition>,
    val edgeTypes: List<EdgeTypeDefinition>,
    val layouts: List<LayoutAlgorithmConfig>,
    val interactions: List<InteractionConfig>,
    val visualEffects: List<VisualEffectConfig>,
    val semanticRules: List<SemanticRule>,
    val aiIntegration: List<GraphAIConfig>,
    val workspaces: List<GraphWorkspaceConfig>,
    val themes: List<GraphThemeConfig>,
    val gestures: List<GestureConfig>
)

// Node Data Classes
@Serializable
data class NodeTypeDefinition(
    val name: String,
    val displayName: String,
    val description: String?,
    val category: NodeCategory,
    val shape: NodeShape,
    val size: NodeSize,
    val color: String,
    val icon: String?,
    val draggable: Boolean,
    val selectable: Boolean,
    val resizable: Boolean,
    val deletable: Boolean,
    val connectable: Boolean,
    val maxConnections: Int,
    val physics: NodePhysics?,
    val content: NodeContent?,
    val behavior: NodeBehavior?
)

@Serializable
data class NodePhysics(
    val mass: Double,
    val charge: Double,
    val gravity: Double,
    val friction: Double,
    val fixed: Boolean
)

@Serializable
data class NodeContent(
    val type: ContentType,
    val data: String?,
    val template: String?,
    val editable: Boolean,
    val richText: Boolean,
    val maxLength: Int?
)

@Serializable
data class NodeBehavior(
    val autoLayout: Boolean,
    val cluster: Boolean,
    val expandable: Boolean,
    val collapsible: Boolean,
    val animations: List<NodeAnimation>,
    val events: Map<String, String>
)

// Edge Data Classes
@Serializable
data class EdgeTypeDefinition(
    val name: String,
    val displayName: String,
    val description: String?,
    val category: EdgeCategory,
    val style: EdgeStyle,
    val width: Double,
    val color: String,
    val curved: Boolean,
    val directed: Boolean,
    val animated: Boolean,
    val label: String?,
    val weight: Double,
    val bidirectional: Boolean,
    val selectable: Boolean,
    val deletable: Boolean
)

// Layout Data Classes
@Serializable
data class LayoutAlgorithmConfig(
    val algorithm: LayoutAlgorithm,
    val enabled: Boolean,
    val primary: Boolean,
    val parameters: Map<String, Double>,
    val constraints: List<LayoutConstraint>
)

@Serializable
data class LayoutConstraint(
    val type: ConstraintType,
    val value: Double
)

// Interaction Data Classes
@Serializable
data class InteractionConfig(
    val type: InteractionType,
    val enabled: Boolean,
    val sensitivity: Double,
    val multiTouch: Boolean,
    val gestures: List<String>,
    val keyBindings: Map<String, String>
)

// Visual Effects Data Classes
@Serializable
data class VisualEffectConfig(
    val type: VisualEffectType,
    val enabled: Boolean,
    val intensity: Double,
    val duration: Int,
    val easing: EasingFunction,
    val settings: Map<String, String>
)

// Semantic Rules Data Classes
@Serializable
data class SemanticRule(
    val name: String,
    val condition: String,
    val action: SemanticAction,
    val priority: Int,
    val enabled: Boolean,
    val parameters: Map<String, String>
)

// AI Integration Data Classes
@Serializable
data class GraphAIConfig(
    val enabled: Boolean,
    val provider: GraphAIProvider,
    val model: String,
    val features: List<AIFeature>,
    val suggestions: AISuggestionConfig?,
    val clustering: AIClusteringConfig?,
    val search: AISearchConfig?
)

@Serializable
data class AISuggestionConfig(
    val enabled: Boolean,
    val threshold: Double,
    val maxSuggestions: Int,
    val contextWindow: Int
)

@Serializable
data class AIClusteringConfig(
    val enabled: Boolean,
    val algorithm: ClusteringAlgorithm,
    val minClusterSize: Int,
    val maxClusters: Int
)

@Serializable
data class AISearchConfig(
    val enabled: Boolean,
    val semantic: Boolean,
    val fuzzy: Boolean,
    val maxResults: Int
)

// Workspace Data Classes
@Serializable
data class GraphWorkspaceConfig(
    val name: String,
    val enabled: Boolean,
    val defaultLayout: LayoutAlgorithm,
    val maxNodes: Int,
    val persistence: Boolean,
    val collaborative: Boolean,
    val filters: List<WorkspaceFilter>
)

@Serializable
data class WorkspaceFilter(
    val type: FilterType,
    val condition: String
)

// Theme Data Classes
@Serializable
data class GraphThemeConfig(
    val name: String,
    val enabled: Boolean,
    val darkMode: Boolean,
    val backgroundColor: String,
    val gridColor: String,
    val selectionColor: String,
    val highlightColor: String,
    val nodeColors: Map<String, String>,
    val edgeColors: Map<String, String>,
    val fonts: Map<String, String>
)

// Gesture Data Classes
@Serializable
data class GestureConfig(
    val name: String,
    val enabled: Boolean,
    val fingers: Int,
    val direction: GestureDirection,
    val action: String,
    val sensitivity: Double
)