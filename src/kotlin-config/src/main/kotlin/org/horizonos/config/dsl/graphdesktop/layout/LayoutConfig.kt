package org.horizonos.config.dsl.graphdesktop.layout

import kotlinx.serialization.Serializable

// ===== Layout Configuration =====

@Serializable
data class LayoutAlgorithmConfig(
    val algorithm: LayoutAlgorithm,
    val enabled: Boolean = true,
    val animationDuration: Int = 500,
    val animationEasing: EasingFunction = EasingFunction.EASE_IN_OUT,
    val padding: Int = 50,
    val nodeSpacing: Int = 50,
    val levelSpacing: Int = 100,
    val constraints: List<LayoutConstraint> = emptyList()
)

@Serializable
data class LayoutConstraint(
    val type: ConstraintType,
    val nodes: List<String> = emptyList(),
    val value: String? = null
)

// Layout Enums
@Serializable
enum class LayoutAlgorithm {
    FORCE_DIRECTED,
    HIERARCHICAL,
    CIRCULAR,
    GRID,
    TREE,
    RADIAL,
    SPECTRAL,
    CUSTOM
}

@Serializable
enum class ConstraintType {
    FIXED_POSITION,
    SAME_LEVEL,
    MINIMUM_DISTANCE,
    MAXIMUM_DISTANCE,
    ALIGNMENT,
    GROUPING
}

@Serializable
enum class EasingFunction {
    LINEAR,
    EASE_IN,
    EASE_OUT,
    EASE_IN_OUT,
    BOUNCE,
    ELASTIC,
    CUBIC_BEZIER
}