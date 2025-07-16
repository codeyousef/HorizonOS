package org.horizonos.config.dsl.boot.plymouth

import org.horizonos.config.dsl.HorizonOSDsl

// ===== Plymouth DSL Builders =====

@HorizonOSDsl
class PlymouthContext {
    var enabled: Boolean = true
    var theme: String = "spinner"
    var showDelay: Int = 0
    var deviceTimeout: Int = 8
    var debug: Boolean = false
    var forceSplash: Boolean = false
    var ignoreSerialConsoles: Boolean = false
    private val modules = mutableListOf<String>()
    private val customThemes = mutableListOf<PlymouthTheme>()
    
    fun module(name: String) {
        modules.add(name)
    }
    
    // Convenience methods for common Plymouth configurations
    fun quietBoot() {
        forceSplash = false
        showDelay = 0
    }
    
    fun showSplash() {
        forceSplash = true
    }
    
    fun plugins(vararg pluginNames: String) {
        pluginNames.forEach { module(it) }
    }
    
    fun customTheme(name: String, scriptPath: String, block: PlymouthThemeContext.() -> Unit = {}) {
        customThemes.add(PlymouthThemeContext(name, scriptPath).apply(block).toTheme())
    }
    
    fun toConfig(): PlymouthConfig {
        return PlymouthConfig(
            enabled = enabled,
            theme = theme,
            showDelay = showDelay,
            deviceTimeout = deviceTimeout,
            debug = debug,
            forceSplash = forceSplash,
            ignoreSerialConsoles = ignoreSerialConsoles,
            modules = modules,
            customThemes = customThemes
        )
    }
}

@HorizonOSDsl
class PlymouthThemeContext(
    private val name: String,
    private val scriptPath: String
) {
    var displayName: String = name
    var description: String? = null
    var imagePath: String? = null
    var configPath: String? = null
    private val colors = mutableMapOf<String, String>()
    
    fun color(name: String, value: String) {
        colors[name] = value
    }
    
    fun toTheme(): PlymouthTheme {
        return PlymouthTheme(
            name = name,
            displayName = displayName,
            description = description,
            scriptPath = scriptPath,
            imagePath = imagePath,
            configPath = configPath,
            colors = colors
        )
    }
}