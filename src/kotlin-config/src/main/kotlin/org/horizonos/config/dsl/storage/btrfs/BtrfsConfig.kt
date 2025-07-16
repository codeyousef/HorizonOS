package org.horizonos.config.dsl.storage.btrfs

import kotlinx.serialization.Serializable
import kotlin.time.Duration
import kotlin.time.Duration.Companion.hours

// ===== Btrfs Configuration =====

@Serializable
data class BtrfsConfig(
    val enabled: Boolean = false,
    val subvolumes: List<BtrfsSubvolume> = emptyList(),
    val snapshots: SnapshotConfig = SnapshotConfig(),
    val quotas: QuotaConfig = QuotaConfig(),
    val scrub: ScrubConfig = ScrubConfig(),
    val balance: BalanceConfig = BalanceConfig(),
    val compression: CompressionConfig = CompressionConfig(),
    val dedupe: DedupeConfig = DedupeConfig()
)

@Serializable
data class BtrfsSubvolume(
    val name: String,
    val path: String,
    val mountOptions: List<String> = emptyList(),
    val defaultSubvolume: Boolean = false,
    val snapshots: SubvolumeSnapshots = SubvolumeSnapshots(),
    val quota: SubvolumeQuota? = null,
    val compression: CompressionType? = null,
    val copyOnWrite: Boolean = true,
    val autoDefrag: Boolean = false
)

@Serializable
data class SubvolumeSnapshots(
    val enabled: Boolean = true,
    val location: String = "/.snapshots",
    val schedule: SnapshotSchedule = SnapshotSchedule(),
    val retention: SnapshotRetention = SnapshotRetention(),
    val naming: String = "{name}-{timestamp}"
)

@Serializable
data class SnapshotSchedule(
    val hourly: Int = 0,
    val daily: Int = 7,
    val weekly: Int = 4,
    val monthly: Int = 12,
    val yearly: Int = 0
)

@Serializable
data class SnapshotRetention(
    val hourly: Int = 24,
    val daily: Int = 7,
    val weekly: Int = 4,
    val monthly: Int = 12,
    val yearly: Int = 2,
    val minimumFreeSpace: String = "20%"
)

@Serializable
data class SnapshotConfig(
    val enabled: Boolean = true,
    val tool: SnapshotTool = SnapshotTool.SNAPPER,
    val bootSnapshots: Boolean = true,
    val prePostSnapshots: Boolean = true,
    val timeline: Boolean = true,
    val cleanupAlgorithm: CleanupAlgorithm = CleanupAlgorithm.NUMBER
)

@Serializable
data class SubvolumeQuota(
    val referenced: String? = null,
    val exclusive: String? = null,
    val compressed: String? = null,
    val sizeLimit: String? = null
)

@Serializable
data class QuotaConfig(
    val enabled: Boolean = false,
    val groups: List<QuotaGroup> = emptyList(),
    val rescan: Duration = 24.hours
)

@Serializable
data class QuotaGroup(
    val id: String,
    val limit: String,
    val subvolumes: List<String> = emptyList()
)

@Serializable
data class ScrubConfig(
    val enabled: Boolean = true,
    val schedule: String = "monthly",
    val priority: IOPriority = IOPriority.IDLE,
    val readOnly: Boolean = false,
    val dataOnly: Boolean = false
)

@Serializable
data class BalanceConfig(
    val enabled: Boolean = false,
    val schedule: String = "quarterly",
    val filters: BalanceFilters = BalanceFilters(),
    val pauseOnBattery: Boolean = true,
    val resumeAfterBoot: Boolean = true
)

@Serializable
data class BalanceFilters(
    val dataUsage: String? = null,
    val metadataUsage: String? = null,
    val systemUsage: String? = null,
    val convert: ProfileConversion? = null
)

@Serializable
data class ProfileConversion(
    val data: String? = null,
    val metadata: String? = null,
    val system: String? = null
)

@Serializable
data class CompressionConfig(
    val algorithm: CompressionType = CompressionType.ZSTD,
    val level: Int? = null,
    val forceCompress: Boolean = false,
    val threads: Int? = null
)

@Serializable
data class DedupeConfig(
    val enabled: Boolean = false,
    val backend: DedupeBackend = DedupeBackend.DUPEREMOVE,
    val hashAlgorithm: String = "sha256",
    val blockSize: String = "128K",
    val schedule: String = "weekly"
)

// Btrfs Enums
@Serializable
enum class CompressionType {
    NONE,
    ZLIB,
    LZO,
    ZSTD
}

@Serializable
enum class SnapshotTool {
    SNAPPER,
    TIMESHIFT,
    BTRBK,
    CUSTOM
}

@Serializable
enum class CleanupAlgorithm {
    NUMBER,
    TIMELINE,
    EMPTY_PRE_POST
}

@Serializable
enum class IOPriority {
    IDLE,
    BEST_EFFORT,
    REALTIME
}

@Serializable
enum class DedupeBackend {
    DUPEREMOVE,
    BEES,
    CUSTOM
}