package org.horizonos.config.dsl.boot.plymouth

import kotlinx.serialization.Serializable

// ===== Plymouth Configuration =====

@Serializable
data class PlymouthConfig(
    val enabled: Boolean = true,
    val theme: String = "spinner",
    val showDelay: Int = 0,
    val deviceTimeout: Int = 8,
    val debug: Boolean = false,
    val forceSplash: Boolean = false,
    val ignoreSerialConsoles: Boolean = false,
    val modules: List<String> = emptyList(),
    val customThemes: List<PlymouthTheme> = emptyList()
)

@Serializable
data class PlymouthTheme(
    val name: String,
    val displayName: String,
    val description: String? = null,
    val scriptPath: String,
    val imagePath: String? = null,
    val configPath: String? = null,
    val colors: Map<String, String> = emptyMap()
)