package org.horizonos.config.dsl.boot.secureboot

import kotlinx.serialization.Serializable

// ===== Secure Boot Configuration =====

@Serializable
data class SecureBootConfig(
    val enabled: Boolean = false,
    val enrollKeys: Boolean = false,
    val setupMode: Boolean = false,
    val allowUnsignedDrivers: Boolean = false,
    val mokManager: Boolean = false,
    val signKernel: Boolean = false,
    val signModules: Boolean = false,
    val keys: SecureBootKeys = SecureBootKeys()
)

@Serializable
data class SecureBootKeys(
    val pk: String? = null,
    val kek: List<String> = emptyList(),
    val db: List<String> = emptyList(),
    val dbx: List<String> = emptyList(),
    val mok: List<String> = emptyList(),
    val mokListRT: List<String> = emptyList(),
    val platform: String? = null,
    val keyExchange: String? = null,
    val signature: String? = null,
    val forbidden: List<String> = emptyList()
)