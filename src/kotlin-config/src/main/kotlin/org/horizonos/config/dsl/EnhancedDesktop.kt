package org.horizonos.config.dsl

import kotlinx.serialization.Serializable

// ===== Enhanced Desktop Configuration DSL =====

@HorizonOSDsl
class EnhancedDesktopContext {
    internal val windowManagers = mutableListOf<WindowManagerConfig>()
    internal val compositors = mutableListOf<CompositorConfig>()
    internal val themes = mutableListOf<ThemeConfig>()
    internal val panels = mutableListOf<PanelConfig>()
    internal val launchers = mutableListOf<LauncherConfig>()
    internal val notifications = mutableListOf<NotificationConfig>()
    internal val wallpapers = mutableListOf<WallpaperConfig>()
    internal val accessibility = mutableListOf<AccessibilityConfig>()
    internal val inputMethods = mutableListOf<InputMethodConfig>()
    internal val displayProfiles = mutableListOf<DisplayProfile>()
    internal val workspaces = mutableListOf<WorkspaceConfig>()
    internal val hotkeys = mutableListOf<HotkeyConfig>()

    fun windowManager(type: WindowManagerType, block: WindowManagerContext.() -> Unit) {
        windowManagers.add(WindowManagerContext(type).apply(block).toConfig())
    }

    fun compositor(type: CompositorType, block: CompositorContext.() -> Unit) {
        compositors.add(CompositorContext(type).apply(block).toConfig())
    }

    fun theme(name: String, block: ThemeContext.() -> Unit) {
        themes.add(ThemeContext(name).apply(block).toConfig())
    }

    fun panel(type: PanelType, block: PanelContext.() -> Unit) {
        panels.add(PanelContext(type).apply(block).toConfig())
    }

    fun launcher(type: LauncherType, block: LauncherContext.() -> Unit) {
        launchers.add(LauncherContext(type).apply(block).toConfig())
    }

    fun notifications(type: NotificationType, block: NotificationContext.() -> Unit) {
        notifications.add(NotificationContext(type).apply(block).toConfig())
    }

    fun wallpaper(block: WallpaperContext.() -> Unit) {
        wallpapers.add(WallpaperContext().apply(block).toConfig())
    }

    fun accessibility(block: AccessibilityContext.() -> Unit) {
        accessibility.add(AccessibilityContext().apply(block).toConfig())
    }

    fun inputMethod(type: InputMethodType, block: InputMethodContext.() -> Unit) {
        inputMethods.add(InputMethodContext(type).apply(block).toConfig())
    }

    fun displayProfile(name: String, block: DisplayProfileContext.() -> Unit) {
        displayProfiles.add(DisplayProfileContext(name).apply(block).toProfile())
    }

    fun workspace(number: Int, block: WorkspaceContext.() -> Unit) {
        workspaces.add(WorkspaceContext(number).apply(block).toConfig())
    }

    fun hotkey(key: String, command: String, block: HotkeyContext.() -> Unit = {}) {
        hotkeys.add(HotkeyContext(key, command).apply(block).toConfig())
    }

    fun toConfig(): EnhancedDesktopConfig {
        return EnhancedDesktopConfig(
            windowManagers = windowManagers,
            compositors = compositors,
            themes = themes,
            panels = panels,
            launchers = launchers,
            notifications = notifications,
            wallpapers = wallpapers,
            accessibility = accessibility,
            inputMethods = inputMethods,
            displayProfiles = displayProfiles,
            workspaces = workspaces,
            hotkeys = hotkeys
        )
    }
}

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

// ===== Theme Configuration =====

@HorizonOSDsl
class ThemeContext(private val name: String) {
    var enabled: Boolean = true
    var type: ThemeType = ThemeType.GTK
    var darkMode: Boolean = false
    var iconTheme: String? = null
    var cursorTheme: String? = null
    var fontFamily: String = "Sans"
    var fontSize: Int = 11
    var accentColor: String? = null
    var customColors = mutableMapOf<String, String>()

    fun color(element: String, color: String) {
        customColors[element] = color
    }

    fun toConfig(): ThemeConfig {
        return ThemeConfig(
            name = name,
            enabled = enabled,
            type = type,
            darkMode = darkMode,
            iconTheme = iconTheme,
            cursorTheme = cursorTheme,
            fontFamily = fontFamily,
            fontSize = fontSize,
            accentColor = accentColor,
            customColors = customColors.toMap()
        )
    }
}

// ===== Panel Configuration =====

@HorizonOSDsl
class PanelContext(private val type: PanelType) {
    var enabled: Boolean = true
    var position: PanelPosition = PanelPosition.BOTTOM
    var height: Int = 32
    var autohide: Boolean = false
    var widgets = mutableListOf<PanelWidget>()
    var transparency: Double = 1.0

    fun widget(type: WidgetType, block: PanelWidgetContext.() -> Unit = {}) {
        widgets.add(PanelWidgetContext(type).apply(block).toWidget())
    }

    fun toConfig(): PanelConfig {
        return PanelConfig(
            type = type,
            enabled = enabled,
            position = position,
            height = height,
            autohide = autohide,
            widgets = widgets,
            transparency = transparency
        )
    }
}

@HorizonOSDsl
class PanelWidgetContext(private val type: WidgetType) {
    var enabled: Boolean = true
    var position: Int = 0
    var settings = mutableMapOf<String, String>()

    fun setting(key: String, value: String) {
        settings[key] = value
    }

    fun toWidget(): PanelWidget {
        return PanelWidget(
            type = type,
            enabled = enabled,
            position = position,
            settings = settings.toMap()
        )
    }
}

// ===== Launcher Configuration =====

@HorizonOSDsl
class LauncherContext(private val type: LauncherType) {
    var enabled: Boolean = true
    var hotkey: String? = null
    var theme: String? = null
    var fuzzySearch: Boolean = true
    var showIcons: Boolean = true
    var maxResults: Int = 10
    var settings = mutableMapOf<String, String>()

    fun setting(key: String, value: String) {
        settings[key] = value
    }

    fun toConfig(): LauncherConfig {
        return LauncherConfig(
            type = type,
            enabled = enabled,
            hotkey = hotkey,
            theme = theme,
            fuzzySearch = fuzzySearch,
            showIcons = showIcons,
            maxResults = maxResults,
            settings = settings.toMap()
        )
    }
}

// ===== Notification Configuration =====

@HorizonOSDsl
class NotificationContext(private val type: NotificationType) {
    var enabled: Boolean = true
    var position: NotificationPosition = NotificationPosition.TOP_RIGHT
    var timeout: Int = 5000
    var maxNotifications: Int = 5
    var soundEnabled: Boolean = true
    var iconEnabled: Boolean = true
    var theme: String? = null

    fun toConfig(): NotificationConfig {
        return NotificationConfig(
            type = type,
            enabled = enabled,
            position = position,
            timeout = timeout,
            maxNotifications = maxNotifications,
            soundEnabled = soundEnabled,
            iconEnabled = iconEnabled,
            theme = theme
        )
    }
}

// ===== Wallpaper Configuration =====

@HorizonOSDsl
class WallpaperContext {
    var type: WallpaperType = WallpaperType.STATIC
    var path: String? = null
    var mode: WallpaperMode = WallpaperMode.FILL
    var changeInterval: Int? = null
    var directory: String? = null
    var randomOrder: Boolean = false

    fun toConfig(): WallpaperConfig {
        return WallpaperConfig(
            type = type,
            path = path,
            mode = mode,
            changeInterval = changeInterval,
            directory = directory,
            randomOrder = randomOrder
        )
    }
}

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
enum class ThemeType {
    GTK, QT, BOTH
}

@Serializable
enum class PanelType {
    POLYBAR, WAYBAR, I3BAR, XFCE_PANEL, KDE_PANEL, GNOME_PANEL
}

@Serializable
enum class PanelPosition {
    TOP, BOTTOM, LEFT, RIGHT
}

@Serializable
enum class WidgetType {
    CLOCK, BATTERY, NETWORK, VOLUME, WORKSPACES, WINDOW_TITLE, SYSTEM_TRAY, LAUNCHER, CUSTOM
}

@Serializable
enum class LauncherType {
    ROFI, DMENU, ALBERT, ULAUNCHER, KRUNNER
}

@Serializable
enum class NotificationType {
    DUNST, MAKO, NOTIFY_OSD, KDE_NOTIFICATIONS
}

@Serializable
enum class NotificationPosition {
    TOP_LEFT, TOP_RIGHT, TOP_CENTER, BOTTOM_LEFT, BOTTOM_RIGHT, BOTTOM_CENTER
}

@Serializable
enum class WallpaperType {
    STATIC, SLIDESHOW, DYNAMIC, LIVE
}

@Serializable
enum class WallpaperMode {
    FILL, FIT, STRETCH, CENTER, TILE
}

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
data class EnhancedDesktopConfig(
    val windowManagers: List<WindowManagerConfig> = emptyList(),
    val compositors: List<CompositorConfig> = emptyList(),
    val themes: List<ThemeConfig> = emptyList(),
    val panels: List<PanelConfig> = emptyList(),
    val launchers: List<LauncherConfig> = emptyList(),
    val notifications: List<NotificationConfig> = emptyList(),
    val wallpapers: List<WallpaperConfig> = emptyList(),
    val accessibility: List<AccessibilityConfig> = emptyList(),
    val inputMethods: List<InputMethodConfig> = emptyList(),
    val displayProfiles: List<DisplayProfile> = emptyList(),
    val workspaces: List<WorkspaceConfig> = emptyList(),
    val hotkeys: List<HotkeyConfig> = emptyList()
)

// Window Manager Data Classes
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

// Compositor Data Classes
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

// Theme Data Classes
@Serializable
data class ThemeConfig(
    val name: String,
    val enabled: Boolean,
    val type: ThemeType,
    val darkMode: Boolean,
    val iconTheme: String?,
    val cursorTheme: String?,
    val fontFamily: String,
    val fontSize: Int,
    val accentColor: String?,
    val customColors: Map<String, String>
)

// Panel Data Classes
@Serializable
data class PanelConfig(
    val type: PanelType,
    val enabled: Boolean,
    val position: PanelPosition,
    val height: Int,
    val autohide: Boolean,
    val widgets: List<PanelWidget>,
    val transparency: Double
)

@Serializable
data class PanelWidget(
    val type: WidgetType,
    val enabled: Boolean,
    val position: Int,
    val settings: Map<String, String>
)

// Launcher Data Classes
@Serializable
data class LauncherConfig(
    val type: LauncherType,
    val enabled: Boolean,
    val hotkey: String?,
    val theme: String?,
    val fuzzySearch: Boolean,
    val showIcons: Boolean,
    val maxResults: Int,
    val settings: Map<String, String>
)

// Notification Data Classes
@Serializable
data class NotificationConfig(
    val type: NotificationType,
    val enabled: Boolean,
    val position: NotificationPosition,
    val timeout: Int,
    val maxNotifications: Int,
    val soundEnabled: Boolean,
    val iconEnabled: Boolean,
    val theme: String?
)

// Wallpaper Data Classes
@Serializable
data class WallpaperConfig(
    val type: WallpaperType,
    val path: String?,
    val mode: WallpaperMode,
    val changeInterval: Int?,
    val directory: String?,
    val randomOrder: Boolean
)

// Accessibility Data Classes
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

// Input Method Data Classes
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

// Display Data Classes
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

// Workspace Data Classes
@Serializable
data class WorkspaceConfig(
    val number: Int,
    val name: String?,
    val monitor: String?,
    val persistent: Boolean,
    val layout: WorkspaceLayout,
    val wallpaper: String?
)

// Hotkey Data Classes
@Serializable
data class HotkeyConfig(
    val key: String,
    val command: String,
    val description: String?,
    val mode: HotkeyMode,
    val repeat: Boolean
)