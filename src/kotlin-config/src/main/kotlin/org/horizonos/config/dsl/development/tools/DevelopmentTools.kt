package org.horizonos.config.dsl.development.tools

import kotlinx.serialization.Serializable
import org.horizonos.config.dsl.HorizonOSDsl

// ===== Development Tools =====

@HorizonOSDsl
class DevelopmentToolContext(private val name: String) {
    var enabled: Boolean = true
    var version: String? = null
    var category: ToolCategory = ToolCategory.GENERAL
    val dependencies = mutableListOf<String>()
    val environmentVariables = mutableMapOf<String, String>()
    val configurations = mutableMapOf<String, String>()

    fun dependency(name: String) {
        dependencies.add(name)
    }

    fun env(key: String, value: String) {
        environmentVariables[key] = value
    }

    fun config(key: String, value: String) {
        configurations[key] = value
    }

    fun toTool(): DevelopmentTool {
        return DevelopmentTool(
            name = name,
            enabled = enabled,
            version = version,
            category = category,
            dependencies = dependencies,
            environmentVariables = environmentVariables,
            configurations = configurations
        )
    }
}

@Serializable
data class DevelopmentTool(
    val name: String,
    val enabled: Boolean,
    val version: String?,
    val category: ToolCategory,
    val dependencies: List<String>,
    val environmentVariables: Map<String, String>,
    val configurations: Map<String, String>
)

@Serializable
enum class ToolCategory {
    GENERAL, BUILD, DEBUG, PROFILING, TESTING, DOCUMENTATION, 
    LINTING, FORMATTING, VERSION_CONTROL, DATABASE, NETWORKING, 
    MONITORING, CONTAINERIZATION, VIRTUALIZATION, SECURITY
}