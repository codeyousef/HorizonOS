package org.horizonos.config.dsl.graphdesktop.workspace

import org.horizonos.config.dsl.HorizonOSDsl

// ===== Workspace DSL Builders =====

@HorizonOSDsl
class GraphWorkspaceContext(private val name: String) {
    var displayName: String = name
    var description: String? = null
    var icon: String? = null
    var isDefault: Boolean = false
    var persistent: Boolean = true
    var maxNodes: Int = 1000
    private val filters = mutableListOf<WorkspaceFilter>()
    var layout: String? = null
    private val viewState = mutableMapOf<String, String>()
    
    fun filter(type: FilterType, block: WorkspaceFilterContext.() -> Unit) {
        filters.add(WorkspaceFilterContext(type).apply(block).toFilter())
    }
    
    fun viewState(key: String, value: String) {
        viewState[key] = value
    }
    
    fun toConfig(): GraphWorkspaceConfig {
        return GraphWorkspaceConfig(
            name = name,
            displayName = displayName,
            description = description,
            icon = icon,
            isDefault = isDefault,
            persistent = persistent,
            maxNodes = maxNodes,
            filters = filters,
            layout = layout,
            viewState = viewState
        )
    }
}

@HorizonOSDsl
class WorkspaceFilterContext(private val type: FilterType) {
    var field: String = ""
    var operator: String = "equals"
    var value: String = ""
    var enabled: Boolean = true
    
    fun toFilter(): WorkspaceFilter {
        return WorkspaceFilter(
            type = type,
            field = field,
            operator = operator,
            value = value,
            enabled = enabled
        )
    }
}

@HorizonOSDsl
class SemanticRuleContext {
    var name: String = ""
    var description: String? = null
    private lateinit var trigger: SemanticTrigger
    private val conditions = mutableListOf<SemanticCondition>()
    private val actions = mutableListOf<SemanticAction>()
    var priority: Int = 0
    var enabled: Boolean = true
    
    fun trigger(event: String, block: SemanticTriggerContext.() -> Unit = {}) {
        trigger = SemanticTriggerContext(event).apply(block).toTrigger()
    }
    
    fun condition(field: String, operator: String, value: String) {
        conditions.add(SemanticCondition(field, operator, value))
    }
    
    fun action(action: SemanticAction) {
        actions.add(action)
    }
    
    fun toRule(): SemanticRule {
        return SemanticRule(
            name = name,
            description = description,
            trigger = trigger,
            conditions = conditions,
            actions = actions,
            priority = priority,
            enabled = enabled
        )
    }
}

@HorizonOSDsl
class SemanticTriggerContext(private val event: String) {
    var target: String? = null
    var debounce: Int? = null
    
    fun toTrigger(): SemanticTrigger {
        return SemanticTrigger(
            event = event,
            target = target,
            debounce = debounce
        )
    }
}