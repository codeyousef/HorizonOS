package org.horizonos.config.dsl.graphdesktop.edges

import org.horizonos.config.dsl.HorizonOSDsl

// ===== Edge DSL Builders =====

@HorizonOSDsl
class EdgeTypeContext(private val name: String) {
    var displayName: String = name
    var description: String? = null
    var category: EdgeCategory = EdgeCategory.CUSTOM
    var style: EdgeStyle = EdgeStyle.SOLID
    var color: String = "#999999"
    var width: Int = 2
    var animated: Boolean = false
    var bidirectional: Boolean = false
    var showLabel: Boolean = true
    var showArrow: Boolean = true
    var arrowSize: Int = 10
    
    private var physics = EdgePhysics()
    private var routing = EdgeRouting()
    private var interaction = EdgeInteraction()
    
    fun physics(block: EdgePhysicsContext.() -> Unit) {
        physics = EdgePhysicsContext().apply(block).toPhysics()
    }
    
    fun routing(block: EdgeRoutingContext.() -> Unit) {
        routing = EdgeRoutingContext().apply(block).toRouting()
    }
    
    fun interaction(block: EdgeInteractionContext.() -> Unit) {
        interaction = EdgeInteractionContext().apply(block).toInteraction()
    }
    
    fun toDefinition(): EdgeTypeDefinition {
        return EdgeTypeDefinition(
            name = name,
            displayName = displayName,
            description = description,
            category = category,
            style = style,
            color = color,
            width = width,
            animated = animated,
            bidirectional = bidirectional,
            showLabel = showLabel,
            showArrow = showArrow,
            arrowSize = arrowSize,
            physics = physics,
            routing = routing,
            interaction = interaction
        )
    }
}

@HorizonOSDsl
class EdgePhysicsContext {
    var springLength: Double = 100.0
    var springStrength: Double = 0.1
    var damping: Double = 0.8
    
    fun toPhysics(): EdgePhysics {
        return EdgePhysics(
            springLength = springLength,
            springStrength = springStrength,
            damping = damping
        )
    }
}

@HorizonOSDsl
class EdgeRoutingContext {
    var algorithm: RoutingAlgorithm = RoutingAlgorithm.STRAIGHT
    var avoidNodes: Boolean = true
    var avoidOverlap: Boolean = true
    var cornerRadius: Int = 10
    
    fun toRouting(): EdgeRouting {
        return EdgeRouting(
            algorithm = algorithm,
            avoidNodes = avoidNodes,
            avoidOverlap = avoidOverlap,
            cornerRadius = cornerRadius
        )
    }
}

@HorizonOSDsl
class EdgeInteractionContext {
    var selectable: Boolean = true
    var deletable: Boolean = true
    var editable: Boolean = true
    var hoverable: Boolean = true
    private val contextMenu = mutableListOf<String>()
    
    fun contextMenuItem(item: String) {
        contextMenu.add(item)
    }
    
    fun toInteraction(): EdgeInteraction {
        return EdgeInteraction(
            selectable = selectable,
            deletable = deletable,
            editable = editable,
            hoverable = hoverable,
            contextMenu = contextMenu
        )
    }
}