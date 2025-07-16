package org.horizonos.config.dsl.storage.swap

import kotlinx.serialization.Serializable

// ===== Swap Configuration =====

@Serializable
data class SwapConfig(
    val enabled: Boolean = true,
    val devices: List<SwapDevice> = emptyList(),
    val files: List<SwapFile> = emptyList(),
    val zram: ZramConfig = ZramConfig(),
    val zswap: ZswapConfig = ZswapConfig(),
    val swappiness: Int = 60,
    val vfsCachePressure: Int = 100,
    val minFreeKbytes: Long? = null,
    val watermarkScaleFactor: Int = 10
)

@Serializable
data class SwapDevice(
    val device: String,
    val priority: Int = -1,
    val discardPolicy: DiscardPolicy = DiscardPolicy.NONE,
    val label: String? = null,
    val uuid: String? = null
)

@Serializable
data class SwapFile(
    val path: String,
    val size: String,
    val priority: Int = -1,
    val permissions: String = "0600",
    val allocateMode: AllocateMode = AllocateMode.FALLOCATE,
    val filesystem: String? = null
)

@Serializable
data class ZramConfig(
    val enabled: Boolean = false,
    val devices: List<ZramDevice> = emptyList(),
    val algorithm: CompressionAlgorithm = CompressionAlgorithm.LZ4,
    val streams: Int? = null
)

@Serializable
data class ZramDevice(
    val name: String = "zram0",
    val size: String = "50%",
    val priority: Int = 100,
    val disksize: String? = null,
    val memLimit: String? = null,
    val backingDev: String? = null,
    val writebackPercent: Int = 10
)

@Serializable
data class ZswapConfig(
    val enabled: Boolean = false,
    val compressor: CompressionAlgorithm = CompressionAlgorithm.LZ4,
    val maxPoolPercent: Int = 20,
    val acceptThresholdPercent: Int = 90,
    val zpool: ZpoolType = ZpoolType.Z3FOLD,
    val sameFilled: Boolean = true,
    val nonSameFilled: Boolean = true,
    val exclusiveLoads: Boolean = false,
    val shrinkerEnabled: Boolean = true
)

// Swap Enums
@Serializable
enum class DiscardPolicy {
    NONE,
    ONCE,
    PAGES,
    BOTH
}

@Serializable
enum class AllocateMode {
    FALLOCATE,
    DD,
    TRUNCATE
}

@Serializable
enum class CompressionAlgorithm {
    LZO,
    LZ4,
    LZ4HC,
    ZSTD,
    DEFLATE,
    ZLIB
}

@Serializable
enum class ZpoolType {
    ZBUD,
    Z3FOLD,
    ZSMALLOC
}