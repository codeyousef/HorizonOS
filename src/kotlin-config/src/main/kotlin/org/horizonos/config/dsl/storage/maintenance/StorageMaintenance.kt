package org.horizonos.config.dsl.storage.maintenance

import kotlinx.serialization.Serializable
import kotlin.time.Duration
import kotlin.time.Duration.Companion.days
import kotlin.time.Duration.Companion.hours

// ===== Storage Maintenance Configuration =====

@Serializable
data class StorageMaintenanceConfig(
    val enabled: Boolean = true,
    val trim: TrimConfiguration = TrimConfiguration(),
    val defragmentation: DefragmentationConfig = DefragmentationConfig(),
    val cleanup: CleanupConfig = CleanupConfig(),
    val verification: VerificationConfig = VerificationConfig(),
    val optimization: OptimizationConfig = OptimizationConfig()
)

@Serializable
data class TrimConfiguration(
    val enabled: Boolean = true,
    val schedule: String = "weekly",
    val devices: List<TrimDevice> = emptyList(),
    val fstrim: FstrimConfig = FstrimConfig()
)

@Serializable
data class TrimDevice(
    val device: String,
    val filesystem: String? = null,
    val options: List<String> = emptyList(),
    val minFreeSpace: String = "10%"
)

@Serializable
data class FstrimConfig(
    val verbose: Boolean = true,
    val allFilesystems: Boolean = true,
    val minExtentSize: String? = null,
    val excludeFilesystems: List<String> = emptyList()
)

@Serializable
data class DefragmentationConfig(
    val enabled: Boolean = false,
    val filesystems: List<DefragTask> = emptyList(),
    val schedule: String = "monthly",
    val policy: DefragPolicy = DefragPolicy()
)

@Serializable
data class DefragTask(
    val filesystem: String,
    val type: FilesystemType,
    val options: DefragOptions = DefragOptions(),
    val exclude: List<String> = emptyList()
)

@Serializable
data class DefragOptions(
    val maxRuntime: Duration = 2.hours,
    val threshold: Int = 5, // fragmentation percentage
    val compress: Boolean = false,
    val flush: Boolean = true
)

@Serializable
data class DefragPolicy(
    val skipIfBattery: Boolean = true,
    val ionicePriority: Int = 7,
    val cpuAffinity: List<Int> = emptyList(),
    val memoryLimit: String? = null
)

@Serializable
data class CleanupConfig(
    val enabled: Boolean = true,
    val tasks: List<CleanupTask> = emptyList(),
    val orphaned: OrphanedCleanup = OrphanedCleanup(),
    val cache: CacheCleanup = CacheCleanup(),
    val logs: LogCleanup = LogCleanup()
)

@Serializable
data class CleanupTask(
    val name: String,
    val paths: List<String>,
    val patterns: List<String> = emptyList(),
    val age: Duration? = null,
    val size: String? = null,
    val action: CleanupAction = CleanupAction.DELETE
)

@Serializable
data class OrphanedCleanup(
    val enabled: Boolean = true,
    val packages: Boolean = true,
    val kernels: Boolean = true,
    val tempFiles: Boolean = true,
    val schedule: String = "weekly"
)

@Serializable
data class CacheCleanup(
    val enabled: Boolean = true,
    val systemCache: Boolean = true,
    val userCache: Boolean = true,
    val packageCache: Boolean = true,
    val maxAge: Duration = 30.days,
    val maxSize: String? = null
)

@Serializable
data class LogCleanup(
    val enabled: Boolean = true,
    val rotation: LogRotation = LogRotation(),
    val compression: Boolean = true,
    val maxAge: Duration = 90.days,
    val maxSize: String = "1G"
)

@Serializable
data class LogRotation(
    val enabled: Boolean = true,
    val frequency: String = "daily",
    val keep: Int = 7,
    val compress: Boolean = true,
    val delayCompress: Boolean = true
)

@Serializable
data class VerificationConfig(
    val enabled: Boolean = true,
    val filesystem: FilesystemVerification = FilesystemVerification(),
    val data: DataVerification = DataVerification(),
    val integrity: IntegrityVerification = IntegrityVerification()
)

@Serializable
data class FilesystemVerification(
    val enabled: Boolean = true,
    val schedule: String = "monthly",
    val checks: List<FsckCheck> = emptyList(),
    val autoRepair: Boolean = false
)

@Serializable
data class FsckCheck(
    val filesystem: String,
    val type: FilesystemType,
    val options: List<String> = emptyList(),
    val forceCheck: Boolean = false
)

@Serializable
data class DataVerification(
    val enabled: Boolean = true,
    val checksums: ChecksumVerification = ChecksumVerification(),
    val backups: BackupVerification = BackupVerification()
)

@Serializable
data class ChecksumVerification(
    val enabled: Boolean = true,
    val algorithm: ChecksumAlgorithm = ChecksumAlgorithm.SHA256,
    val paths: List<String> = emptyList(),
    val schedule: String = "weekly"
)

@Serializable
data class BackupVerification(
    val enabled: Boolean = true,
    val testRestore: Boolean = false,
    val compareChecksums: Boolean = true,
    val sampleSize: Int = 10 // percentage
)

@Serializable
data class IntegrityVerification(
    val enabled: Boolean = true,
    val methods: List<IntegrityMethod> = emptyList(),
    val schedule: String = "weekly"
)

@Serializable
data class IntegrityMethod(
    val name: String,
    val type: IntegrityType,
    val paths: List<String>,
    val options: Map<String, String> = emptyMap()
)

@Serializable
data class OptimizationConfig(
    val enabled: Boolean = true,
    val database: DatabaseOptimization = DatabaseOptimization(),
    val index: IndexOptimization = IndexOptimization(),
    val allocation: AllocationOptimization = AllocationOptimization()
)

@Serializable
data class DatabaseOptimization(
    val enabled: Boolean = true,
    val vacuum: Boolean = true,
    val analyze: Boolean = true,
    val reindex: Boolean = false,
    val databases: List<String> = emptyList()
)

@Serializable
data class IndexOptimization(
    val enabled: Boolean = true,
    val updatedb: Boolean = true,
    val mandb: Boolean = true,
    val fontCache: Boolean = true,
    val schedule: String = "daily"
)

@Serializable
data class AllocationOptimization(
    val enabled: Boolean = false,
    val preallocation: PreallocationConfig = PreallocationConfig(),
    val alignment: AlignmentConfig = AlignmentConfig()
)

@Serializable
data class PreallocationConfig(
    val enabled: Boolean = false,
    val method: PreallocationMethod = PreallocationMethod.FALLOCATE,
    val patterns: List<PreallocationPattern> = emptyList()
)

@Serializable
data class PreallocationPattern(
    val path: String,
    val size: String,
    val count: Int = 1
)

@Serializable
data class AlignmentConfig(
    val enabled: Boolean = true,
    val blockSize: Int = 4096,
    val stripeSize: Int? = null
)

// Maintenance Enums
@Serializable
enum class FilesystemType {
    EXT4,
    BTRFS,
    XFS,
    F2FS,
    ZFS
}

@Serializable
enum class CleanupAction {
    DELETE,
    COMPRESS,
    ARCHIVE,
    MOVE
}

@Serializable
enum class ChecksumAlgorithm {
    MD5,
    SHA1,
    SHA256,
    SHA512,
    BLAKE2B,
    XXHASH
}

@Serializable
enum class IntegrityType {
    AIDE,
    TRIPWIRE,
    SAMHAIN,
    CUSTOM
}

@Serializable
enum class PreallocationMethod {
    FALLOCATE,
    POSIX_FALLOCATE,
    TRUNCATE
}