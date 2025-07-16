package org.horizonos.config.dsl.graphdesktop.edges

import kotlinx.serialization.Serializable

// ===== Edge Configuration =====

@Serializable
data class EdgeTypeDefinition(
    val name: String,
    val displayName: String,
    val description: String? = null,
    val category: EdgeCategory,
    val style: EdgeStyle = EdgeStyle.SOLID,
    val color: String = "#999999",
    val width: Int = 2,
    val animated: Boolean = false,
    val bidirectional: Boolean = false,
    val showLabel: Boolean = true,
    val showArrow: Boolean = true,
    val arrowSize: Int = 10,
    val physics: EdgePhysics = EdgePhysics(),
    val routing: EdgeRouting = EdgeRouting(),
    val interaction: EdgeInteraction = EdgeInteraction()
)

@Serializable
data class EdgePhysics(
    val springLength: Double = 100.0,
    val springStrength: Double = 0.1,
    val damping: Double = 0.8
)

@Serializable
data class EdgeRouting(
    val algorithm: RoutingAlgorithm = RoutingAlgorithm.STRAIGHT,
    val avoidNodes: Boolean = true,
    val avoidOverlap: Boolean = true,
    val cornerRadius: Int = 10
)

@Serializable
data class EdgeInteraction(
    val selectable: Boolean = true,
    val deletable: Boolean = true,
    val editable: Boolean = true,
    val hoverable: Boolean = true,
    val contextMenu: List<String> = emptyList()
)

// Edge Enums
@Serializable
enum class EdgeCategory {
    DATA_FLOW,
    CONTROL_FLOW,
    DEPENDENCY,
    ASSOCIATION,
    HIERARCHY,
    NETWORK,
    SEMANTIC,
    CUSTOM
}

@Serializable
enum class EdgeStyle {
    SOLID,
    DASHED,
    DOTTED,
    DOUBLE,
    WAVY,
    GRADIENT
}

@Serializable
enum class RoutingAlgorithm {
    STRAIGHT,
    CURVED,
    ORTHOGONAL,
    BEZIER,
    SMOOTH,
    AVOID_OBSTACLES
}