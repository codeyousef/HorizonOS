package org.horizonos.config.dsl.hardware

import kotlinx.serialization.Serializable
import kotlin.time.Duration
import kotlin.time.Duration.Companion.seconds

// ===== Input Configuration =====

@Serializable
data class InputConfig(
    val keyboard: KeyboardConfig = KeyboardConfig(),
    val mouse: MouseConfig = MouseConfig(),
    val touchpad: TouchpadConfig = TouchpadConfig(),
    val touchscreen: TouchscreenConfig = TouchscreenConfig(),
    val gameController: GameControllerConfig = GameControllerConfig(),
    val accessibility: AccessibilityConfig = AccessibilityConfig()
)

@Serializable
data class KeyboardConfig(
    val layout: String = "us",
    val variant: String? = null,
    val model: String? = null,
    val options: List<String> = emptyList(),
    val repeatDelay: Duration = 500.seconds,
    val repeatRate: Int = 25,
    val numLock: Boolean = true,
    val capsLock: CapsLockBehavior = CapsLockBehavior.CAPS_LOCK
)

@Serializable
data class MouseConfig(
    val acceleration: Double = 1.0,
    val threshold: Double = 4.0,
    val leftHanded: Boolean = false,
    val middleButtonEmulation: Boolean = false,
    val scrollMethod: ScrollMethod = ScrollMethod.TWO_FINGER,
    val naturalScrolling: Boolean = false,
    val clickMethod: ClickMethod = ClickMethod.BUTTON_AREAS
)

@Serializable
data class TouchpadConfig(
    val enabled: Boolean = true,
    val tapToClick: Boolean = true,
    val twoFingerScroll: Boolean = true,
    val edgeScrolling: Boolean = false,
    val naturalScrolling: Boolean = true,
    val palmDetection: Boolean = true,
    val disableWhileTyping: Boolean = true,
    val acceleration: Double = 1.0,
    val sensitivity: Double = 1.0,
    val gestures: GestureConfig = GestureConfig()
)

@Serializable
data class GestureConfig(
    val enabled: Boolean = true,
    val threeFingerSwipe: Boolean = true,
    val fourFingerSwipe: Boolean = true,
    val pinchToZoom: Boolean = true,
    val rotateGesture: Boolean = false,
    val customGestures: List<CustomGesture> = emptyList()
)

@Serializable
data class CustomGesture(
    val name: String,
    val fingers: Int,
    val direction: GestureDirection,
    val action: String
)

@Serializable
data class TouchscreenConfig(
    val enabled: Boolean = true,
    val calibration: TouchCalibration? = null,
    val multitouch: Boolean = true,
    val gestures: Boolean = true,
    val rotation: ScreenRotation = ScreenRotation.NORMAL
)

@Serializable
data class TouchCalibration(
    val minX: Int,
    val maxX: Int,
    val minY: Int,
    val maxY: Int,
    val swapXY: Boolean = false,
    val invertX: Boolean = false,
    val invertY: Boolean = false
)

@Serializable
data class GameControllerConfig(
    val enabled: Boolean = true,
    val deadzone: Double = 0.1,
    val triggerThreshold: Double = 0.3,
    val rumble: Boolean = true,
    val mappings: List<ControllerMapping> = emptyList()
)

@Serializable
data class ControllerMapping(
    val controllerId: String,
    val mapping: String,
    val name: String? = null
)

@Serializable
data class AccessibilityConfig(
    val enabled: Boolean = false,
    val stickyKeys: Boolean = false,
    val slowKeys: Duration? = null,
    val bounceKeys: Duration? = null,
    val mouseKeys: Boolean = false,
    val toggleKeys: Boolean = false,
    val filterKeys: Boolean = false,
    val highContrast: Boolean = false,
    val largeText: Boolean = false,
    val screenReader: Boolean = false,
    val magnifier: MagnifierConfig = MagnifierConfig()
)

@Serializable
data class MagnifierConfig(
    val enabled: Boolean = false,
    val magnification: Double = 2.0,
    val followFocus: Boolean = true,
    val followCursor: Boolean = true,
    val invertColors: Boolean = false
)

// ===== Enums =====

@Serializable
enum class CapsLockBehavior {
    CAPS_LOCK,     // Standard Caps Lock behavior
    CTRL,          // Caps Lock acts as Ctrl
    ESC,           // Caps Lock acts as Escape
    DISABLED       // Caps Lock disabled
}

@Serializable
enum class ScrollMethod {
    TWO_FINGER,    // Two-finger scrolling
    EDGE,          // Edge scrolling
    BUTTON,        // Button scrolling
    NO_SCROLL      // Scrolling disabled
}

@Serializable
enum class ClickMethod {
    BUTTON_AREAS,  // Physical button areas
    CLICK_FINGER,  // Click anywhere with finger
    NONE          // No clicking
}

@Serializable
enum class GestureDirection {
    UP,           // Upward gesture
    DOWN,         // Downward gesture
    LEFT,         // Left gesture
    RIGHT,        // Right gesture
    CLOCKWISE,    // Clockwise rotation
    COUNTER_CLOCKWISE, // Counter-clockwise rotation
    PINCH_IN,     // Pinch inward
    PINCH_OUT     // Pinch outward
}

@Serializable
enum class ScreenRotation {
    NORMAL,       // 0 degrees
    LEFT,         // 90 degrees counter-clockwise
    INVERTED,     // 180 degrees
    RIGHT         // 90 degrees clockwise
}