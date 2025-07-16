package org.horizonos.config.dsl.desktop

import kotlinx.serialization.Serializable
import org.horizonos.config.dsl.HorizonOSDsl

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

// ===== Enums =====

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

// ===== Data Classes =====

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

@Serializable
data class WallpaperConfig(
    val type: WallpaperType,
    val path: String?,
    val mode: WallpaperMode,
    val changeInterval: Int?,
    val directory: String?,
    val randomOrder: Boolean
)