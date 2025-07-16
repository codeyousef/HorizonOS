package org.horizonos.config.dsl.graphdesktop.nodes

import kotlinx.serialization.Serializable

// ===== Node Configuration =====

@Serializable
data class NodeTypeDefinition(
    val name: String,
    val displayName: String,
    val description: String? = null,
    val category: NodeCategory,
    val icon: String? = null,
    val color: String = "#4A90E2",
    val shape: NodeShape = NodeShape.ROUNDED_RECTANGLE,
    val size: NodeSize = NodeSize.MEDIUM,
    val minSize: Pair<Int, Int> = 50 to 50,
    val maxSize: Pair<Int, Int> = 500 to 500,
    val resizable: Boolean = true,
    val physics: NodePhysics = NodePhysics(),
    val content: NodeContent = NodeContent(),
    val behavior: NodeBehavior = NodeBehavior()
)

@Serializable
data class NodePhysics(
    val mass: Double = 1.0,
    val charge: Double = -30.0,
    val friction: Double = 0.9,
    val repulsionStrength: Double = 100.0,
    val attractionStrength: Double = 0.1,
    val collisionRadius: Double = 20.0,
    val fixedPosition: Boolean = false
)

@Serializable
data class NodeContent(
    val contentType: ContentType = ContentType.TEXT,
    val showLabel: Boolean = true,
    val showIcon: Boolean = true,
    val showMetadata: Boolean = false,
    val maxTextLength: Int = 100,
    val textWrapping: Boolean = true,
    val customRenderer: String? = null
)

@Serializable
data class NodeBehavior(
    val selectable: Boolean = true,
    val draggable: Boolean = true,
    val editable: Boolean = true,
    val deletable: Boolean = true,
    val connectableAsSource: Boolean = true,
    val connectableAsTarget: Boolean = true,
    val maxConnections: Int? = null,
    val animations: List<NodeAnimation> = emptyList(),
    val contextMenu: List<String> = emptyList()
)

// Node Enums
@Serializable
enum class NodeCategory {
    SYSTEM,
    APPLICATION,
    FILE,
    PROCESS,
    NETWORK,
    DATA,
    AI,
    USER,
    CUSTOM
}

@Serializable
enum class NodeShape {
    CIRCLE,
    RECTANGLE,
    ROUNDED_RECTANGLE,
    DIAMOND,
    HEXAGON,
    OCTAGON,
    STAR,
    CUSTOM
}

@Serializable
enum class NodeSize {
    TINY,
    SMALL,
    MEDIUM,
    LARGE,
    EXTRA_LARGE,
    DYNAMIC
}

@Serializable
enum class ContentType {
    TEXT,
    ICON,
    IMAGE,
    CODE,
    MARKDOWN,
    HTML,
    CUSTOM
}

@Serializable
enum class NodeAnimation {
    PULSE,
    GLOW,
    SHAKE,
    BOUNCE,
    FADE,
    SCALE,
    ROTATE
}