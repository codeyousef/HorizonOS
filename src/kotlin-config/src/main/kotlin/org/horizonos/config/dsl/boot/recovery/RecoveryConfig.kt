package org.horizonos.config.dsl.boot.recovery

import kotlinx.serialization.Serializable

// ===== Recovery Configuration =====

@Serializable
data class RecoveryConfig(
    val enabled: Boolean = true,
    val autoboot: Boolean = false,
    val timeout: Int = 0,
    val kernel: String? = null,
    val initrd: String? = null,
    val options: List<String> = emptyList(),
    val services: List<String> = emptyList(),
    val environment: Map<String, String> = emptyMap()
)