package org.horizonos.config.dsl

import kotlinx.serialization.Serializable
import org.horizonos.config.dsl.development.languages.*
import org.horizonos.config.dsl.development.ide.*
import org.horizonos.config.dsl.development.EditorConfiguration
import org.horizonos.config.dsl.development.EditorConfigurationContext
import org.horizonos.config.dsl.development.EditorType
import org.horizonos.config.dsl.development.tools.*
import org.horizonos.config.dsl.development.containers.*
import org.horizonos.config.dsl.development.vcs.*

// ===== Development Environment Configuration DSL =====

@HorizonOSDsl
class DevelopmentContext {
    internal val languages = mutableListOf<LanguageRuntime>()
    internal val ides = mutableListOf<IDEConfiguration>()
    internal val editors = mutableListOf<EditorConfiguration>()
    internal val tools = mutableListOf<DevelopmentTool>()
    internal val packageManagers = mutableListOf<PackageManagerConfig>()
    internal val containerDev = mutableListOf<ContainerDevEnvironment>()
    internal val versionControl = mutableListOf<VCSConfiguration>()

    fun language(type: LanguageType, block: LanguageRuntimeContext.() -> Unit) {
        languages.add(LanguageRuntimeContext(type).apply(block).toRuntime())
    }

    fun ide(type: IDEType, block: IDEConfigurationContext.() -> Unit) {
        ides.add(IDEConfigurationContext(type).apply(block).toConfiguration())
    }

    fun editor(type: EditorType, block: EditorConfigurationContext.() -> Unit) {
        editors.add(EditorConfigurationContext(type).apply(block).toConfiguration())
    }

    fun tool(name: String, block: DevelopmentToolContext.() -> Unit) {
        tools.add(DevelopmentToolContext(name).apply(block).toTool())
    }

    fun packageManager(type: PackageManagerType, block: PackageManagerContext.() -> Unit) {
        packageManagers.add(PackageManagerContext(type).apply(block).toConfig())
    }

    fun containerDev(name: String, block: ContainerDevContext.() -> Unit) {
        containerDev.add(ContainerDevContext(name).apply(block).toEnvironment())
    }

    fun versionControl(type: VCSType, block: VCSConfigurationContext.() -> Unit) {
        versionControl.add(VCSConfigurationContext(type).apply(block).toConfiguration())
    }

    fun toConfig(): DevelopmentConfig {
        return DevelopmentConfig(
            languages = languages,
            ides = ides,
            editors = editors,
            tools = tools,
            packageManagers = packageManagers,
            containerDev = containerDev,
            versionControl = versionControl
        )
    }
}

// ===== Main Development Configuration =====

@Serializable
data class DevelopmentConfig(
    val languages: List<LanguageRuntime> = emptyList(),
    val ides: List<IDEConfiguration> = emptyList(),
    val editors: List<EditorConfiguration> = emptyList(),
    val tools: List<DevelopmentTool> = emptyList(),
    val packageManagers: List<PackageManagerConfig> = emptyList(),
    val containerDev: List<ContainerDevEnvironment> = emptyList(),
    val versionControl: List<VCSConfiguration> = emptyList()
)