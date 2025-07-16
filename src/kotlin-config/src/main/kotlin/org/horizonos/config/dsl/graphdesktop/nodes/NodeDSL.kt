package org.horizonos.config.dsl.graphdesktop.nodes

import org.horizonos.config.dsl.HorizonOSDsl

// ===== Node DSL Builders =====

@HorizonOSDsl
class NodeTypeContext(private val name: String) {
    var displayName: String = name
    var description: String? = null
    var category: NodeCategory = NodeCategory.CUSTOM
    var icon: String? = null
    var color: String = "#4A90E2"
    var shape: NodeShape = NodeShape.ROUNDED_RECTANGLE
    var size: NodeSize = NodeSize.MEDIUM
    var minSize: Pair<Int, Int> = 50 to 50
    var maxSize: Pair<Int, Int> = 500 to 500
    var resizable: Boolean = true
    
    private var physics = NodePhysics()
    private var content = NodeContent()
    private var behavior = NodeBehavior()
    
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
            icon = icon,
            color = color,
            shape = shape,
            size = size,
            minSize = minSize,
            maxSize = maxSize,
            resizable = resizable,
            physics = physics,
            content = content,
            behavior = behavior
        )
    }
}

@HorizonOSDsl
class NodePhysicsContext {
    var mass: Double = 1.0
    var charge: Double = -30.0
    var friction: Double = 0.9
    var repulsionStrength: Double = 100.0
    var attractionStrength: Double = 0.1
    var collisionRadius: Double = 20.0
    var fixedPosition: Boolean = false
    
    fun toPhysics(): NodePhysics {
        return NodePhysics(
            mass = mass,
            charge = charge,
            friction = friction,
            repulsionStrength = repulsionStrength,
            attractionStrength = attractionStrength,
            collisionRadius = collisionRadius,
            fixedPosition = fixedPosition
        )
    }
}

@HorizonOSDsl
class NodeContentContext {
    var contentType: ContentType = ContentType.TEXT
    var showLabel: Boolean = true
    var showIcon: Boolean = true
    var showMetadata: Boolean = false
    var maxTextLength: Int = 100
    var textWrapping: Boolean = true
    var customRenderer: String? = null
    
    fun toContent(): NodeContent {
        return NodeContent(
            contentType = contentType,
            showLabel = showLabel,
            showIcon = showIcon,
            showMetadata = showMetadata,
            maxTextLength = maxTextLength,
            textWrapping = textWrapping,
            customRenderer = customRenderer
        )
    }
}

@HorizonOSDsl
class NodeBehaviorContext {
    var selectable: Boolean = true
    var draggable: Boolean = true
    var editable: Boolean = true
    var deletable: Boolean = true
    var connectableAsSource: Boolean = true
    var connectableAsTarget: Boolean = true
    var maxConnections: Int? = null
    private val animations = mutableListOf<NodeAnimation>()
    private val contextMenu = mutableListOf<String>()
    
    fun animation(anim: NodeAnimation) {
        animations.add(anim)
    }
    
    fun contextMenuItem(item: String) {
        contextMenu.add(item)
    }
    
    fun toBehavior(): NodeBehavior {
        return NodeBehavior(
            selectable = selectable,
            draggable = draggable,
            editable = editable,
            deletable = deletable,
            connectableAsSource = connectableAsSource,
            connectableAsTarget = connectableAsTarget,
            maxConnections = maxConnections,
            animations = animations,
            contextMenu = contextMenu
        )
    }
}