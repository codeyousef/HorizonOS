package org.horizonos.config.dsl.hardware

import kotlinx.serialization.Serializable
import kotlin.time.Duration
import kotlin.time.Duration.Companion.minutes

// ===== Display Configuration =====

@Serializable
data class DisplayConfig(
    val monitors: List<MonitorConfig> = emptyList(),
    val layout: DisplayLayout = DisplayLayout.SINGLE,
    val scaling: DisplayScaling = DisplayScaling(),
    val color: ColorManagement = ColorManagement(),
    val nightLight: NightLightConfig = NightLightConfig(),
    val dpms: DPMSConfig = DPMSConfig(),
    val compositing: CompositingConfig = CompositingConfig()
)

@Serializable
data class MonitorConfig(
    val name: String,
    val enabled: Boolean = true,
    val primary: Boolean = false,
    val resolution: Resolution? = null,
    val refreshRate: Double? = null,
    val position: Position = Position(0, 0),
    val rotation: ScreenRotation = ScreenRotation.NORMAL,
    val scale: Double = 1.0,
    val colorProfile: String? = null,
    val brightness: Double = 1.0,
    val gamma: GammaConfig = GammaConfig()
)

@Serializable
data class Resolution(
    val width: Int,
    val height: Int
)

@Serializable
data class Position(
    val x: Int,
    val y: Int
)

@Serializable
data class GammaConfig(
    val red: Double = 1.0,
    val green: Double = 1.0,
    val blue: Double = 1.0
)

@Serializable
data class DisplayScaling(
    val mode: ScalingMode = ScalingMode.AUTO,
    val factor: Double = 1.0,
    val perMonitor: Boolean = true,
    val fractionalScaling: Boolean = true
)

@Serializable
data class ColorManagement(
    val enabled: Boolean = true,
    val defaultProfile: String? = null,
    val profiles: List<ColorProfile> = emptyList(),
    val adaptation: ColorAdaptation = ColorAdaptation.BRADFORD
)

@Serializable
data class ColorProfile(
    val name: String,
    val path: String,
    val description: String? = null
)

@Serializable
data class NightLightConfig(
    val enabled: Boolean = true,
    val temperature: Int = 4000, // Kelvin
    val schedule: NightLightSchedule = NightLightSchedule.AUTOMATIC,
    val manualStart: String = "20:00",
    val manualEnd: String = "06:00",
    val transition: Duration = 30.minutes
)

@Serializable
data class DPMSConfig(
    val enabled: Boolean = true,
    val standbyTime: Duration = 10.minutes,
    val suspendTime: Duration = 15.minutes,
    val offTime: Duration = 20.minutes
)

@Serializable
data class CompositingConfig(
    val enabled: Boolean = true,
    val backend: CompositingBackend = CompositingBackend.OPENGL,
    val vsync: Boolean = true,
    val unredirectFullscreen: Boolean = true,
    val animations: Boolean = true,
    val effects: List<String> = emptyList()
)

// ===== Enums =====

@Serializable
enum class DisplayLayout {
    SINGLE,        // Single monitor
    EXTEND,        // Extended desktop
    MIRROR,        // Mirror displays
    CUSTOM         // Custom arrangement
}

@Serializable
enum class ScalingMode {
    AUTO,          // Automatic scaling
    MANUAL,        // Manual scaling factor
    INTEGER,       // Integer scaling only
    FRACTIONAL     // Fractional scaling
}

@Serializable
enum class ColorAdaptation {
    BRADFORD,      // Bradford adaptation
    VON_KRIES,     // Von Kries adaptation
    XYZ_SCALING    // XYZ scaling
}

@Serializable
enum class NightLightSchedule {
    AUTOMATIC,     // Automatic sunset/sunrise
    MANUAL,        // Manual start/end times
    DISABLED       // Night light disabled
}

@Serializable
enum class ScreenRotation {
    NORMAL,        // 0 degrees
    LEFT,          // 90 degrees counter-clockwise
    INVERTED,      // 180 degrees
    RIGHT          // 90 degrees clockwise
}

@Serializable
enum class CompositingBackend {
    OPENGL,        // OpenGL backend
    XRENDER,       // XRender backend
    SOFTWARE       // Software rendering
}