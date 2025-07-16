package org.horizonos.config.dsl.graphdesktop.workspace

import kotlinx.serialization.Serializable

// ===== Workspace Configuration =====

@Serializable
data class GraphWorkspaceConfig(
    val name: String,
    val displayName: String,
    val description: String? = null,
    val icon: String? = null,
    val isDefault: Boolean = false,
    val persistent: Boolean = true,
    val maxNodes: Int = 1000,
    val filters: List<WorkspaceFilter> = emptyList(),
    val layout: String? = null,
    val viewState: Map<String, String> = emptyMap()
)

@Serializable
data class WorkspaceFilter(
    val type: FilterType,
    val field: String,
    val operator: String,
    val value: String,
    val enabled: Boolean = true
)

@Serializable
data class SemanticRule(
    val name: String,
    val description: String? = null,
    val trigger: SemanticTrigger,
    val conditions: List<SemanticCondition> = emptyList(),
    val actions: List<SemanticAction> = emptyList(),
    val priority: Int = 0,
    val enabled: Boolean = true
)

@Serializable
data class SemanticTrigger(
    val event: String,
    val target: String? = null,
    val debounce: Int? = null
)

@Serializable
data class SemanticCondition(
    val field: String,
    val operator: String,
    val value: String
)

// Workspace Enums
@Serializable
enum class FilterType {
    NODE_TYPE,
    EDGE_TYPE,
    PROPERTY,
    TAG,
    CREATION_DATE,
    MODIFICATION_DATE,
    CUSTOM
}

@Serializable
enum class SemanticAction {
    CREATE_NODE,
    CREATE_EDGE,
    UPDATE_PROPERTY,
    DELETE_NODE,
    DELETE_EDGE,
    TRIGGER_LAYOUT,
    SHOW_NOTIFICATION,
    EXECUTE_SCRIPT
}