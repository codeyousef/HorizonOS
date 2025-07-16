package org.horizonos.config.dsl.boot.kernel

import kotlinx.serialization.Serializable

// ===== Kernel Configuration =====

@Serializable
data class KernelConfig(
    val parameters: List<KernelParameter> = emptyList(),
    val modules: KernelModuleConfig = KernelModuleConfig(),
    val version: String? = null,
    val variants: List<KernelVariant> = emptyList(),
    val compression: KernelCompression = KernelCompression.ZSTD,
    val debugging: KernelDebugging = KernelDebugging(),
    val security: KernelSecurity = KernelSecurity()
)

@Serializable
data class KernelParameter(
    val name: String,
    val value: String? = null,
    val condition: String? = null
)

@Serializable
data class KernelModuleConfig(
    val blacklist: List<String> = emptyList(),
    val load: List<String> = emptyList(),
    val options: Map<String, String> = emptyMap(),
    val autoLoad: Boolean = true,
    val compression: ModuleCompression = ModuleCompression.XZ
)

@Serializable
data class KernelVariant(
    val name: String,
    val version: String,
    val path: String,
    val enabled: Boolean = true,
    val description: String? = null,
    val isDefault: Boolean = false
)

@Serializable
data class KernelDebugging(
    val enabled: Boolean = false,
    val kgdb: Boolean = false,
    val kdb: Boolean = false,
    val earlyPrintk: Boolean = false,
    val debugLevel: Int = 0,
    val ftrace: Boolean = false,
    val kprobes: Boolean = false,
    val dynamicDebug: Boolean = false
)

@Serializable
data class KernelSecurity(
    val kaslr: Boolean = true,
    val smep: Boolean = true,
    val smap: Boolean = true,
    val pti: Boolean = true,
    val kpti: Boolean = true,
    val spectreV2: SpectreV2Mitigation = SpectreV2Mitigation.AUTO,
    val spectreV4: Boolean = true,
    val l1tf: L1TFMitigation = L1TFMitigation.FLUSH,
    val mds: MDSMitigation = MDSMitigation.FULL,
    val tsx: Boolean = false,
    val ibrs: Boolean = true,
    val ibpb: Boolean = true,
    val stibp: Boolean = true,
    val ssbd: Boolean = true,
    val retpoline: Boolean = true
)

// Kernel Enums
@Serializable
enum class KernelCompression {
    NONE,
    GZIP,
    BZIP2,
    LZMA,
    XZ,
    LZO,
    LZ4,
    ZSTD
}

@Serializable
enum class ModuleCompression {
    NONE,
    GZIP,
    XZ,
    ZSTD
}

@Serializable
enum class SpectreV2Mitigation {
    OFF,
    AUTO,
    ON,
    RETPOLINE,
    IBRS,
    IBRS_ENHANCED,
    EIBRS,
    EIBRS_RETPOLINE,
    EIBRS_LFENCE
}

@Serializable
enum class L1TFMitigation {
    OFF,
    FLUSH,
    FLUSH_NOSMT,
    FLUSH_NOWARN,
    FULL,
    FULL_FORCE
}

@Serializable
enum class MDSMitigation {
    OFF,
    FULL,
    FULL_NOSMT
}