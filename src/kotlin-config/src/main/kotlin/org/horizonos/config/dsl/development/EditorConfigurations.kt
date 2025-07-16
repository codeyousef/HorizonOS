package org.horizonos.config.dsl.development

import kotlinx.serialization.Serializable
import org.horizonos.config.dsl.HorizonOSDsl

// ===== Editor Configuration =====

@HorizonOSDsl
class EditorConfigurationContext(private val type: EditorType) {
    var enabled: Boolean = true
    val settings = mutableMapOf<String, String>()
    val plugins = mutableListOf<String>()
    val themes = mutableListOf<String>()
    val keybindings = mutableMapOf<String, String>()

    fun setting(key: String, value: String) {
        settings[key] = value
    }

    fun plugin(name: String) {
        plugins.add(name)
    }

    fun theme(name: String) {
        themes.add(name)
    }

    fun keybinding(key: String, command: String) {
        keybindings[key] = command
    }

    fun toConfiguration(): EditorConfiguration {
        return EditorConfiguration(
            type = type,
            enabled = enabled,
            settings = settings,
            plugins = plugins,
            themes = themes,
            keybindings = keybindings
        )
    }
}

@Serializable
data class EditorConfiguration(
    val type: EditorType,
    val enabled: Boolean,
    val settings: Map<String, String>,
    val plugins: List<String>,
    val themes: List<String>,
    val keybindings: Map<String, String>
)

@Serializable
enum class EditorType {
    NANO, VIM, NEOVIM, EMACS, MICRO, HELIX, KAKOUNE, XI, NE, JOE, LITE, LITE_XL
}