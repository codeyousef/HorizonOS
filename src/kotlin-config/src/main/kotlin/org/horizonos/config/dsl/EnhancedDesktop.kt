package org.horizonos.config.dsl

import kotlinx.serialization.Serializable
import org.horizonos.config.dsl.desktop.*

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

// Window manager, compositor, workspace, and hotkey contexts are now imported from desktop package

// Theme, panel, launcher, notification, and wallpaper contexts are now imported from desktop package

// Accessibility, input method, display profile, workspace, and hotkey contexts are now imported from desktop package

// Enums are now imported from desktop package

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