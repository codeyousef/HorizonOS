package org.horizonos.config.dsl.graphdesktop.visual

import org.horizonos.config.dsl.HorizonOSDsl

// ===== Visual DSL Builders =====

@HorizonOSDsl
class VisualEffectContext(private val type: VisualEffectType) {
    var enabled: Boolean = true
    var duration: Int = 1000
    var delay: Int = 0
    var easing: EasingFunction = EasingFunction.EASE_IN_OUT
    var intensity: Double = 1.0
    var color: String? = null
    var blendMode: String = "normal"
    
    fun toConfig(): VisualEffectConfig {
        return VisualEffectConfig(
            type = type,
            enabled = enabled,
            duration = duration,
            delay = delay,
            easing = easing,
            intensity = intensity,
            color = color,
            blendMode = blendMode
        )
    }
}

@HorizonOSDsl
class GraphThemeContext(private val name: String) {
    var displayName: String = name
    var description: String? = null
    var isDark: Boolean = false
    private var colors = ThemeColors()
    private var typography = ThemeTypography()
    private var spacing = ThemeSpacing()
    private var effects = ThemeEffects()
    
    fun colors(block: ThemeColorsContext.() -> Unit) {
        colors = ThemeColorsContext().apply(block).toColors()
    }
    
    fun typography(block: ThemeTypographyContext.() -> Unit) {
        typography = ThemeTypographyContext().apply(block).toTypography()
    }
    
    fun spacing(block: ThemeSpacingContext.() -> Unit) {
        spacing = ThemeSpacingContext().apply(block).toSpacing()
    }
    
    fun effects(block: ThemeEffectsContext.() -> Unit) {
        effects = ThemeEffectsContext().apply(block).toEffects()
    }
    
    fun toConfig(): GraphThemeConfig {
        return GraphThemeConfig(
            name = name,
            displayName = displayName,
            description = description,
            isDark = isDark,
            colors = colors,
            typography = typography,
            spacing = spacing,
            effects = effects
        )
    }
}

@HorizonOSDsl
class ThemeColorsContext {
    var background: String = "#FFFFFF"
    var foreground: String = "#000000"
    var primary: String = "#4A90E2"
    var secondary: String = "#7B68EE"
    var accent: String = "#FF6B6B"
    var success: String = "#4CAF50"
    var warning: String = "#FFC107"
    var error: String = "#F44336"
    var nodeDefault: String = "#E0E0E0"
    var edgeDefault: String = "#999999"
    var selection: String = "#2196F3"
    var hover: String = "#FFC107"
    
    fun toColors(): ThemeColors {
        return ThemeColors(
            background = background,
            foreground = foreground,
            primary = primary,
            secondary = secondary,
            accent = accent,
            success = success,
            warning = warning,
            error = error,
            nodeDefault = nodeDefault,
            edgeDefault = edgeDefault,
            selection = selection,
            hover = hover
        )
    }
}

@HorizonOSDsl
class ThemeTypographyContext {
    var fontFamily: String = "Inter, system-ui, sans-serif"
    var fontSize: Int = 14
    var fontWeight: String = "normal"
    var lineHeight: Double = 1.5
    var nodeLabelSize: Int = 12
    var edgeLabelSize: Int = 10
    
    fun toTypography(): ThemeTypography {
        return ThemeTypography(
            fontFamily = fontFamily,
            fontSize = fontSize,
            fontWeight = fontWeight,
            lineHeight = lineHeight,
            nodeLabelSize = nodeLabelSize,
            edgeLabelSize = edgeLabelSize
        )
    }
}

@HorizonOSDsl
class ThemeSpacingContext {
    var unit: Int = 8
    var nodePadding: Int = 16
    var edgePadding: Int = 8
    var groupPadding: Int = 24
    
    fun toSpacing(): ThemeSpacing {
        return ThemeSpacing(
            unit = unit,
            nodePadding = nodePadding,
            edgePadding = edgePadding,
            groupPadding = groupPadding
        )
    }
}

@HorizonOSDsl
class ThemeEffectsContext {
    var shadows: Boolean = true
    var animations: Boolean = true
    var blur: Boolean = true
    var glow: Boolean = true
    var gradients: Boolean = true
    
    fun toEffects(): ThemeEffects {
        return ThemeEffects(
            shadows = shadows,
            animations = animations,
            blur = blur,
            glow = glow,
            gradients = gradients
        )
    }
}