package org.horizonos.config.dsl.graphdesktop.layout

import org.horizonos.config.dsl.HorizonOSDsl

// ===== Layout DSL Builders =====

@HorizonOSDsl
class LayoutAlgorithmContext(private val algorithm: LayoutAlgorithm) {
    var enabled: Boolean = true
    var animationDuration: Int = 500
    var animationEasing: EasingFunction = EasingFunction.EASE_IN_OUT
    var padding: Int = 50
    var nodeSpacing: Int = 50
    var levelSpacing: Int = 100
    private val constraints = mutableListOf<LayoutConstraint>()
    
    fun constraint(type: ConstraintType, block: LayoutConstraintContext.() -> Unit) {
        constraints.add(LayoutConstraintContext(type).apply(block).toConstraint())
    }
    
    fun toConfig(): LayoutAlgorithmConfig {
        return LayoutAlgorithmConfig(
            algorithm = algorithm,
            enabled = enabled,
            animationDuration = animationDuration,
            animationEasing = animationEasing,
            padding = padding,
            nodeSpacing = nodeSpacing,
            levelSpacing = levelSpacing,
            constraints = constraints
        )
    }
}

@HorizonOSDsl
class LayoutConstraintContext(private val type: ConstraintType) {
    private val nodes = mutableListOf<String>()
    var value: String? = null
    
    fun node(nodeId: String) {
        nodes.add(nodeId)
    }
    
    fun nodes(vararg nodeIds: String) {
        nodes.addAll(nodeIds)
    }
    
    fun toConstraint(): LayoutConstraint {
        return LayoutConstraint(
            type = type,
            nodes = nodes,
            value = value
        )
    }
}