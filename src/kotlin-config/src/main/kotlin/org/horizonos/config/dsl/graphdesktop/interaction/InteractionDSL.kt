package org.horizonos.config.dsl.graphdesktop.interaction

import org.horizonos.config.dsl.HorizonOSDsl

// ===== Interaction DSL Builders =====

@HorizonOSDsl
class InteractionContext(private val type: InteractionType) {
    var enabled: Boolean = true
    private val modifierKeys = mutableListOf<String>()
    var preventDefault: Boolean = false
    var stopPropagation: Boolean = false
    var debounceMs: Int? = null
    var throttleMs: Int? = null
    private var feedback = InteractionFeedback()
    
    fun modifierKey(key: String) {
        modifierKeys.add(key)
    }
    
    fun feedback(block: InteractionFeedbackContext.() -> Unit) {
        feedback = InteractionFeedbackContext().apply(block).toFeedback()
    }
    
    fun toConfig(): InteractionConfig {
        return InteractionConfig(
            type = type,
            enabled = enabled,
            modifierKeys = modifierKeys,
            preventDefault = preventDefault,
            stopPropagation = stopPropagation,
            debounceMs = debounceMs,
            throttleMs = throttleMs,
            feedback = feedback
        )
    }
}

@HorizonOSDsl
class InteractionFeedbackContext {
    var visual: Boolean = true
    var haptic: Boolean = false
    var audio: Boolean = false
    var cursor: String? = null
    
    fun toFeedback(): InteractionFeedback {
        return InteractionFeedback(
            visual = visual,
            haptic = haptic,
            audio = audio,
            cursor = cursor
        )
    }
}

@HorizonOSDsl
class GestureContext(private val name: String) {
    var enabled: Boolean = true
    var fingers: Int = 1
    var direction: GestureDirection? = null
    var threshold: Double = 10.0
    var velocity: Double? = null
    var action: String = ""
    
    fun toConfig(): GestureConfig {
        return GestureConfig(
            name = name,
            enabled = enabled,
            fingers = fingers,
            direction = direction,
            threshold = threshold,
            velocity = velocity,
            action = action
        )
    }
}