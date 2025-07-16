package org.horizonos.config.dsl.desktop

import kotlinx.serialization.Serializable
import org.horizonos.config.dsl.HorizonOSDsl

// ===== Accessibility Configuration =====

@HorizonOSDsl
class AccessibilityContext {
    var highContrast: Boolean = false
    var largeText: Boolean = false
    var screenReader: Boolean = false
    var magnifier: Boolean = false
    var onScreenKeyboard: Boolean = false
    var stickyKeys: Boolean = false
    var slowKeys: Boolean = false
    var bounceKeys: Boolean = false
    var mouseKeys: Boolean = false
    var clickAssist: Boolean = false

    fun toConfig(): AccessibilityConfig {
        return AccessibilityConfig(
            highContrast = highContrast,
            largeText = largeText,
            screenReader = screenReader,
            magnifier = magnifier,
            onScreenKeyboard = onScreenKeyboard,
            stickyKeys = stickyKeys,
            slowKeys = slowKeys,
            bounceKeys = bounceKeys,
            mouseKeys = mouseKeys,
            clickAssist = clickAssist
        )
    }
}

// ===== Input Method Configuration =====

@HorizonOSDsl
class InputMethodContext(private val type: InputMethodType) {
    var enabled: Boolean = true
    var defaultEngine: String? = null
    var autoStart: Boolean = true
    var engines = mutableListOf<InputEngine>()
    var hotkeys = mutableMapOf<String, String>()

    fun engine(name: String, language: String, enabled: Boolean = true) {
        engines.add(InputEngine(name, language, enabled))
    }

    fun hotkey(action: String, key: String) {
        hotkeys[action] = key
    }

    fun toConfig(): InputMethodConfig {
        return InputMethodConfig(
            type = type,
            enabled = enabled,
            defaultEngine = defaultEngine,
            autoStart = autoStart,
            engines = engines,
            hotkeys = hotkeys.toMap()
        )
    }
}

// ===== Display Profile Configuration =====

@HorizonOSDsl
class DisplayProfileContext(private val name: String) {
    var primary: String? = null
    var monitors = mutableListOf<MonitorConfig>()
    var arrangement: DisplayArrangement = DisplayArrangement.HORIZONTAL
    var scaling: DisplayScaling = DisplayScaling.AUTO

    fun monitor(name: String, block: MonitorConfigContext.() -> Unit) {
        monitors.add(MonitorConfigContext(name).apply(block).toConfig())
    }

    fun toProfile(): DisplayProfile {
        return DisplayProfile(
            name = name,
            primary = primary,
            monitors = monitors,
            arrangement = arrangement,
            scaling = scaling
        )
    }
}

@HorizonOSDsl
class MonitorConfigContext(private val name: String) {
    var enabled: Boolean = true
    var resolution: String? = null
    var refreshRate: Int? = null
    var position: MonitorPosition? = null
    var rotation: MonitorRotation = MonitorRotation.NORMAL
    var scale: Double = 1.0
    var colorProfile: String? = null

    fun position(x: Int, y: Int) {
        position = MonitorPosition(x, y)
    }

    fun toConfig(): MonitorConfig {
        return MonitorConfig(
            name = name,
            enabled = enabled,
            resolution = resolution,
            refreshRate = refreshRate,
            position = position,
            rotation = rotation,
            scale = scale,
            colorProfile = colorProfile
        )
    }
}

// ===== Enums =====

@Serializable
enum class InputMethodType {
    IBUS, FCITX, FCITX5, SCIM
}

@Serializable
enum class DisplayArrangement {
    HORIZONTAL, VERTICAL, CUSTOM
}

@Serializable
enum class DisplayScaling {
    AUTO, MANUAL, PER_MONITOR
}

@Serializable
enum class MonitorRotation {
    NORMAL, LEFT, RIGHT, INVERTED
}

// ===== Data Classes =====

@Serializable
data class AccessibilityConfig(
    val highContrast: Boolean,
    val largeText: Boolean,
    val screenReader: Boolean,
    val magnifier: Boolean,
    val onScreenKeyboard: Boolean,
    val stickyKeys: Boolean,
    val slowKeys: Boolean,
    val bounceKeys: Boolean,
    val mouseKeys: Boolean,
    val clickAssist: Boolean
)

@Serializable
data class InputMethodConfig(
    val type: InputMethodType,
    val enabled: Boolean,
    val defaultEngine: String?,
    val autoStart: Boolean,
    val engines: List<InputEngine>,
    val hotkeys: Map<String, String>
)

@Serializable
data class InputEngine(
    val name: String,
    val language: String,
    val enabled: Boolean
)

@Serializable
data class DisplayProfile(
    val name: String,
    val primary: String?,
    val monitors: List<MonitorConfig>,
    val arrangement: DisplayArrangement,
    val scaling: DisplayScaling
)

@Serializable
data class MonitorConfig(
    val name: String,
    val enabled: Boolean,
    val resolution: String?,
    val refreshRate: Int?,
    val position: MonitorPosition?,
    val rotation: MonitorRotation,
    val scale: Double,
    val colorProfile: String?
)

@Serializable
data class MonitorPosition(
    val x: Int,
    val y: Int
)