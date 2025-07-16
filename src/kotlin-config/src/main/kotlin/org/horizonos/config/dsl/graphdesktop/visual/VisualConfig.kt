package org.horizonos.config.dsl.graphdesktop.visual

import kotlinx.serialization.Serializable

// ===== Visual Configuration =====

@Serializable
data class VisualEffectConfig(
    val type: VisualEffectType,
    val enabled: Boolean = true,
    val duration: Int = 1000,
    val delay: Int = 0,
    val easing: EasingFunction = EasingFunction.EASE_IN_OUT,
    val intensity: Double = 1.0,
    val color: String? = null,
    val blendMode: String = "normal"
)

@Serializable
data class GraphThemeConfig(
    val name: String,
    val displayName: String,
    val description: String? = null,
    val isDark: Boolean = false,
    val colors: ThemeColors = ThemeColors(),
    val typography: ThemeTypography = ThemeTypography(),
    val spacing: ThemeSpacing = ThemeSpacing(),
    val effects: ThemeEffects = ThemeEffects()
)

@Serializable
data class ThemeColors(
    val background: String = "#FFFFFF",
    val foreground: String = "#000000",
    val primary: String = "#4A90E2",
    val secondary: String = "#7B68EE",
    val accent: String = "#FF6B6B",
    val success: String = "#4CAF50",
    val warning: String = "#FFC107",
    val error: String = "#F44336",
    val nodeDefault: String = "#E0E0E0",
    val edgeDefault: String = "#999999",
    val selection: String = "#2196F3",
    val hover: String = "#FFC107"
)

@Serializable
data class ThemeTypography(
    val fontFamily: String = "Inter, system-ui, sans-serif",
    val fontSize: Int = 14,
    val fontWeight: String = "normal",
    val lineHeight: Double = 1.5,
    val nodeLabelSize: Int = 12,
    val edgeLabelSize: Int = 10
)

@Serializable
data class ThemeSpacing(
    val unit: Int = 8,
    val nodePadding: Int = 16,
    val edgePadding: Int = 8,
    val groupPadding: Int = 24
)

@Serializable
data class ThemeEffects(
    val shadows: Boolean = true,
    val animations: Boolean = true,
    val blur: Boolean = true,
    val glow: Boolean = true,
    val gradients: Boolean = true
)

// Visual Enums
@Serializable
enum class VisualEffectType {
    GLOW,
    SHADOW,
    BLUR,
    PARTICLE,
    TRAIL,
    RIPPLE,
    MORPH,
    DISTORTION
}

@Serializable
enum class EasingFunction {
    LINEAR,
    EASE_IN,
    EASE_OUT,
    EASE_IN_OUT,
    BOUNCE,
    ELASTIC,
    CUBIC_BEZIER
}