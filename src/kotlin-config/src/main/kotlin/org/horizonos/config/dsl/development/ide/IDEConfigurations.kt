package org.horizonos.config.dsl.development.ide

import kotlinx.serialization.Serializable
import org.horizonos.config.dsl.HorizonOSDsl

// ===== IDE Configuration =====

@HorizonOSDsl
class IDEConfigurationContext(private val type: IDEType) {
    var enabled: Boolean = true
    val plugins = mutableListOf<String>()
    val extensions = mutableListOf<String>()
    val settings = mutableMapOf<String, String>()
    val keybindings = mutableMapOf<String, String>()
    
    // IDE-specific configurations
    var vscodeConfig: VSCodeConfig? = null
    var intellijConfig: IntelliJConfig? = null
    var vimConfig: VimConfig? = null

    fun plugin(name: String) {
        plugins.add(name)
    }

    fun extension(name: String) {
        extensions.add(name)
    }

    fun setting(key: String, value: String) {
        settings[key] = value
    }

    fun keybinding(key: String, command: String) {
        keybindings[key] = command
    }

    fun vscode(block: VSCodeContext.() -> Unit) {
        vscodeConfig = VSCodeContext().apply(block).toConfig()
    }

    fun intellij(block: IntelliJContext.() -> Unit) {
        intellijConfig = IntelliJContext().apply(block).toConfig()
    }

    fun vim(block: VimContext.() -> Unit) {
        vimConfig = VimContext().apply(block).toConfig()
    }

    fun toConfiguration(): IDEConfiguration {
        return IDEConfiguration(
            type = type,
            enabled = enabled,
            plugins = plugins,
            extensions = extensions,
            settings = settings,
            keybindings = keybindings,
            vscodeConfig = vscodeConfig,
            intellijConfig = intellijConfig,
            vimConfig = vimConfig
        )
    }
}

@HorizonOSDsl
class VSCodeContext {
    var enableTelemetry: Boolean = false
    var enableSyncSettings: Boolean = true
    var colorTheme: String = "Default Dark+"
    var iconTheme: String = "vscode-icons"
    val extensions = mutableListOf<String>()
    val userSettings = mutableMapOf<String, String>()

    fun extension(id: String) {
        extensions.add(id)
    }

    fun setting(key: String, value: String) {
        userSettings[key] = value
    }

    fun toConfig(): VSCodeConfig {
        return VSCodeConfig(
            enableTelemetry = enableTelemetry,
            enableSyncSettings = enableSyncSettings,
            colorTheme = colorTheme,
            iconTheme = iconTheme,
            extensions = extensions,
            userSettings = userSettings
        )
    }
}

@HorizonOSDsl
class IntelliJContext {
    var edition: IntelliJEdition = IntelliJEdition.COMMUNITY
    var enableStatistics: Boolean = false
    var heapSize: String = "2048m"
    val plugins = mutableListOf<String>()
    val vmOptions = mutableListOf<String>()

    fun plugin(id: String) {
        plugins.add(id)
    }

    fun vmOption(option: String) {
        vmOptions.add(option)
    }

    fun toConfig(): IntelliJConfig {
        return IntelliJConfig(
            edition = edition,
            enableStatistics = enableStatistics,
            heapSize = heapSize,
            plugins = plugins,
            vmOptions = vmOptions
        )
    }
}

@HorizonOSDsl
class VimContext {
    var enableSyntaxHighlighting: Boolean = true
    var enableLineNumbers: Boolean = true
    var tabWidth: Int = 4
    var expandTabs: Boolean = true
    var colorScheme: String = "default"
    val plugins = mutableListOf<String>()
    val mappings = mutableMapOf<String, String>()

    fun plugin(name: String) {
        plugins.add(name)
    }

    fun map(key: String, command: String) {
        mappings[key] = command
    }

    fun toConfig(): VimConfig {
        return VimConfig(
            enableSyntaxHighlighting = enableSyntaxHighlighting,
            enableLineNumbers = enableLineNumbers,
            tabWidth = tabWidth,
            expandTabs = expandTabs,
            colorScheme = colorScheme,
            plugins = plugins,
            mappings = mappings
        )
    }
}

// Data Classes
@Serializable
data class IDEConfiguration(
    val type: IDEType,
    val enabled: Boolean,
    val plugins: List<String>,
    val extensions: List<String>,
    val settings: Map<String, String>,
    val keybindings: Map<String, String>,
    val vscodeConfig: VSCodeConfig?,
    val intellijConfig: IntelliJConfig?,
    val vimConfig: VimConfig?
)

@Serializable
data class VSCodeConfig(
    val enableTelemetry: Boolean,
    val enableSyncSettings: Boolean,
    val colorTheme: String,
    val iconTheme: String,
    val extensions: List<String>,
    val userSettings: Map<String, String>
)

@Serializable
data class IntelliJConfig(
    val edition: IntelliJEdition,
    val enableStatistics: Boolean,
    val heapSize: String,
    val plugins: List<String>,
    val vmOptions: List<String>
)

@Serializable
data class VimConfig(
    val enableSyntaxHighlighting: Boolean,
    val enableLineNumbers: Boolean,
    val tabWidth: Int,
    val expandTabs: Boolean,
    val colorScheme: String,
    val plugins: List<String>,
    val mappings: Map<String, String>
)

@Serializable
enum class IDEType {
    VSCODE, VSCODIUM, INTELLIJ, PYCHARM, WEBSTORM, CLION, RIDER, 
    GOLAND, PHPSTORM, RUBYMINE, DATAGRIP, VIM, NEOVIM, EMACS, 
    SUBLIME, ATOM, ECLIPSE, NETBEANS, KATE, GEDIT, FLEET
}

@Serializable
enum class IntelliJEdition {
    COMMUNITY, ULTIMATE
}