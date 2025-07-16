package org.horizonos.config.dsl.graphdesktop.interaction

import kotlinx.serialization.Serializable

// ===== Interaction Configuration =====

@Serializable
data class InteractionConfig(
    val type: InteractionType,
    val enabled: Boolean = true,
    val modifierKeys: List<String> = emptyList(),
    val preventDefault: Boolean = false,
    val stopPropagation: Boolean = false,
    val debounceMs: Int? = null,
    val throttleMs: Int? = null,
    val feedback: InteractionFeedback = InteractionFeedback()
)

@Serializable
data class InteractionFeedback(
    val visual: Boolean = true,
    val haptic: Boolean = false,
    val audio: Boolean = false,
    val cursor: String? = null
)

@Serializable
data class GestureConfig(
    val name: String,
    val enabled: Boolean = true,
    val fingers: Int = 1,
    val direction: GestureDirection? = null,
    val threshold: Double = 10.0,
    val velocity: Double? = null,
    val action: String
)

// Interaction Enums
@Serializable
enum class InteractionType {
    CLICK,
    DOUBLE_CLICK,
    RIGHT_CLICK,
    HOVER,
    DRAG,
    DROP,
    ZOOM,
    PAN,
    ROTATE,
    MULTI_SELECT
}

@Serializable
enum class GestureDirection {
    UP,
    DOWN,
    LEFT,
    RIGHT,
    CLOCKWISE,
    COUNTER_CLOCKWISE,
    PINCH_IN,
    PINCH_OUT
}