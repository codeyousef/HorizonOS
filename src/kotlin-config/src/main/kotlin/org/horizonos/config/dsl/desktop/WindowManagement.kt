package org.horizonos.config.dsl.desktop

import kotlinx.serialization.Serializable
import org.horizonos.config.dsl.HorizonOSDsl

// ===== Window Manager Configuration =====

@HorizonOSDsl
class WindowManagerContext(private val type: WindowManagerType) {
    var enabled: Boolean = true
    var configFile: String? = null
    var autoStart: Boolean = true
    var settings = mutableMapOf<String, String>()
    
    // Specific configurations
    var i3Config: I3Config? = null
    var swayConfig: SwayConfig? = null
    var awesomeConfig: AwesomeConfig? = null
    var bspwmConfig: BspwmConfig? = null

    fun i3(block: I3Context.() -> Unit) {
        i3Config = I3Context().apply(block).toConfig()
    }

    fun sway(block: SwayContext.() -> Unit) {
        swayConfig = SwayContext().apply(block).toConfig()
    }

    fun awesome(block: AwesomeContext.() -> Unit) {
        awesomeConfig = AwesomeContext().apply(block).toConfig()
    }

    fun bspwm(block: BspwmContext.() -> Unit) {
        bspwmConfig = BspwmContext().apply(block).toConfig()
    }

    fun setting(key: String, value: String) {
        settings[key] = value
    }

    fun toConfig(): WindowManagerConfig {
        return WindowManagerConfig(
            type = type,
            enabled = enabled,
            configFile = configFile ?: getDefaultConfigFile(type),
            autoStart = autoStart,
            settings = settings.toMap(),
            i3Config = i3Config,
            swayConfig = swayConfig,
            awesomeConfig = awesomeConfig,
            bspwmConfig = bspwmConfig
        )
    }

    private fun getDefaultConfigFile(type: WindowManagerType): String = when (type) {
        WindowManagerType.I3 -> "~/.config/i3/config"
        WindowManagerType.SWAY -> "~/.config/sway/config"
        WindowManagerType.AWESOME -> "~/.config/awesome/rc.lua"
        WindowManagerType.BSPWM -> "~/.config/bspwm/bspwmrc"
        WindowManagerType.XMONAD -> "~/.xmonad/xmonad.hs"
        WindowManagerType.DWM -> "~/.dwm/config.h"
    }
}

@HorizonOSDsl
class I3Context {
    var modKey: String = "Mod4"
    var font: String = "pango:DejaVu Sans Mono 8"
    var borderWidth: Int = 1
    var gaps: I3Gaps? = null
    var workspaceNames = mutableListOf<String>()
    var autoBack: Boolean = true

    fun gaps(inner: Int, outer: Int = 0) {
        gaps = I3Gaps(inner, outer)
    }

    fun workspace(name: String) {
        workspaceNames.add(name)
    }

    fun toConfig(): I3Config {
        return I3Config(
            modKey = modKey,
            font = font,
            borderWidth = borderWidth,
            gaps = gaps,
            workspaceNames = workspaceNames,
            autoBack = autoBack
        )
    }
}

@HorizonOSDsl
class SwayContext {
    var modKey: String = "Mod4"
    var font: String = "pango:DejaVu Sans Mono 8"
    var output = mutableMapOf<String, SwayOutput>()
    var input = mutableMapOf<String, SwayInput>()
    var gaps: SwayGaps? = null

    fun output(name: String, block: SwayOutputContext.() -> Unit) {
        output[name] = SwayOutputContext().apply(block).toOutput()
    }

    fun input(name: String, block: SwayInputContext.() -> Unit) {
        input[name] = SwayInputContext().apply(block).toInput()
    }

    fun gaps(inner: Int, outer: Int = 0) {
        gaps = SwayGaps(inner, outer)
    }

    fun toConfig(): SwayConfig {
        return SwayConfig(
            modKey = modKey,
            font = font,
            output = output.toMap(),
            input = input.toMap(),
            gaps = gaps
        )
    }
}

@HorizonOSDsl
class SwayOutputContext {
    var resolution: String? = null
    var position: String? = null
    var scale: Double = 1.0
    var transform: String? = null
    var background: String? = null

    fun toOutput(): SwayOutput {
        return SwayOutput(
            resolution = resolution,
            position = position,
            scale = scale,
            transform = transform,
            background = background
        )
    }
}

@HorizonOSDsl
class SwayInputContext {
    var naturalScroll: Boolean = false
    var tapToClick: Boolean = true
    var dwt: Boolean = true
    var middleEmulation: Boolean = false

    fun toInput(): SwayInput {
        return SwayInput(
            naturalScroll = naturalScroll,
            tapToClick = tapToClick,
            dwt = dwt,
            middleEmulation = middleEmulation
        )
    }
}

@HorizonOSDsl
class AwesomeContext {
    var theme: String = "default"
    var modKey: String = "Mod4"
    var terminal: String = "xterm"
    var editor: String = "nano"
    var layouts = mutableListOf<String>()

    fun layout(name: String) {
        layouts.add(name)
    }

    fun toConfig(): AwesomeConfig {
        return AwesomeConfig(
            theme = theme,
            modKey = modKey,
            terminal = terminal,
            editor = editor,
            layouts = layouts
        )
    }
}

@HorizonOSDsl
class BspwmContext {
    var borderWidth: Int = 2
    var windowGap: Int = 12
    var splitRatio: Double = 0.52
    var borderlessMonocle: Boolean = true
    var gaplessMonocle: Boolean = true
    var focusFollowsPointer: Boolean = false

    fun toConfig(): BspwmConfig {
        return BspwmConfig(
            borderWidth = borderWidth,
            windowGap = windowGap,
            splitRatio = splitRatio,
            borderlessMonocle = borderlessMonocle,
            gaplessMonocle = gaplessMonocle,
            focusFollowsPointer = focusFollowsPointer
        )
    }
}

// ===== Compositor Configuration =====

@HorizonOSDsl
class CompositorContext(private val type: CompositorType) {
    var enabled: Boolean = true
    var backend: CompositorBackend = CompositorBackend.AUTO
    var vsync: Boolean = true
    var animations: Boolean = true
    var shadows: Boolean = true
    var transparency: Boolean = true
    var blur: Boolean = false
    var settings = mutableMapOf<String, String>()

    fun setting(key: String, value: String) {
        settings[key] = value
    }

    fun toConfig(): CompositorConfig {
        return CompositorConfig(
            type = type,
            enabled = enabled,
            backend = backend,
            vsync = vsync,
            animations = animations,
            shadows = shadows,
            transparency = transparency,
            blur = blur,
            settings = settings.toMap()
        )
    }
}

// ===== Workspace Configuration =====

@HorizonOSDsl
class WorkspaceContext(private val number: Int) {
    var name: String? = null
    var monitor: String? = null
    var persistent: Boolean = false
    var layout: WorkspaceLayout = WorkspaceLayout.TILED
    var wallpaper: String? = null

    fun toConfig(): WorkspaceConfig {
        return WorkspaceConfig(
            number = number,
            name = name,
            monitor = monitor,
            persistent = persistent,
            layout = layout,
            wallpaper = wallpaper
        )
    }
}

// ===== Hotkey Configuration =====

@HorizonOSDsl
class HotkeyContext(private val key: String, private val command: String) {
    var description: String? = null
    var mode: HotkeyMode = HotkeyMode.GLOBAL
    var repeat: Boolean = false

    fun toConfig(): HotkeyConfig {
        return HotkeyConfig(
            key = key,
            command = command,
            description = description,
            mode = mode,
            repeat = repeat
        )
    }
}

// ===== Enums =====

@Serializable
enum class WindowManagerType {
    I3, SWAY, AWESOME, BSPWM, XMONAD, DWM
}

@Serializable
enum class CompositorType {
    PICOM, COMPTON, XCOMPMGR, MUTTER, KWIN
}

@Serializable
enum class CompositorBackend {
    AUTO, XRENDER, GLX, EGL
}

@Serializable
enum class WorkspaceLayout {
    TILED, FLOATING, MONOCLE, GRID
}

@Serializable
enum class HotkeyMode {
    GLOBAL, WINDOW_MANAGER, APPLICATION
}

// ===== Data Classes =====

@Serializable
data class WindowManagerConfig(
    val type: WindowManagerType,
    val enabled: Boolean,
    val configFile: String,
    val autoStart: Boolean,
    val settings: Map<String, String>,
    val i3Config: I3Config?,
    val swayConfig: SwayConfig?,
    val awesomeConfig: AwesomeConfig?,
    val bspwmConfig: BspwmConfig?
)

@Serializable
data class I3Config(
    val modKey: String,
    val font: String,
    val borderWidth: Int,
    val gaps: I3Gaps?,
    val workspaceNames: List<String>,
    val autoBack: Boolean
)

@Serializable
data class I3Gaps(
    val inner: Int,
    val outer: Int
)

@Serializable
data class SwayConfig(
    val modKey: String,
    val font: String,
    val output: Map<String, SwayOutput>,
    val input: Map<String, SwayInput>,
    val gaps: SwayGaps?
)

@Serializable
data class SwayOutput(
    val resolution: String?,
    val position: String?,
    val scale: Double,
    val transform: String?,
    val background: String?
)

@Serializable
data class SwayInput(
    val naturalScroll: Boolean,
    val tapToClick: Boolean,
    val dwt: Boolean,
    val middleEmulation: Boolean
)

@Serializable
data class SwayGaps(
    val inner: Int,
    val outer: Int
)

@Serializable
data class AwesomeConfig(
    val theme: String,
    val modKey: String,
    val terminal: String,
    val editor: String,
    val layouts: List<String>
)

@Serializable
data class BspwmConfig(
    val borderWidth: Int,
    val windowGap: Int,
    val splitRatio: Double,
    val borderlessMonocle: Boolean,
    val gaplessMonocle: Boolean,
    val focusFollowsPointer: Boolean
)

@Serializable
data class CompositorConfig(
    val type: CompositorType,
    val enabled: Boolean,
    val backend: CompositorBackend,
    val vsync: Boolean,
    val animations: Boolean,
    val shadows: Boolean,
    val transparency: Boolean,
    val blur: Boolean,
    val settings: Map<String, String>
)

@Serializable
data class WorkspaceConfig(
    val number: Int,
    val name: String?,
    val monitor: String?,
    val persistent: Boolean,
    val layout: WorkspaceLayout,
    val wallpaper: String?
)

@Serializable
data class HotkeyConfig(
    val key: String,
    val command: String,
    val description: String?,
    val mode: HotkeyMode,
    val repeat: Boolean
)