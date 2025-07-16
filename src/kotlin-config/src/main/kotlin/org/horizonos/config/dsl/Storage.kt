package org.horizonos.config.dsl

import kotlinx.serialization.Serializable
import kotlin.time.Duration
import kotlin.time.Duration.Companion.hours
import kotlin.time.Duration.Companion.minutes

/**
 * Storage Configuration DSL for HorizonOS
 * 
 * Provides type-safe configuration for storage systems including
 * filesystems, RAID, LUKS encryption, Btrfs subvolumes, and swap.
 */

// ===== Storage Configuration =====

@Serializable
data class StorageConfig(
    val filesystems: List<FilesystemConfig> = emptyList(),
    val raid: RAIDStorageConfig = RAIDStorageConfig(),
    val encryption: EncryptionConfig = EncryptionConfig(),
    val btrfs: BtrfsConfig = BtrfsConfig(),
    val swap: SwapConfig = SwapConfig(),
    val autoMount: AutoMountConfig = AutoMountConfig(),
    val maintenance: StorageMaintenanceConfig = StorageMaintenanceConfig(),
    val monitoring: StorageMonitoringConfig = StorageMonitoringConfig()
)

@Serializable
data class FilesystemConfig(
    val device: String,
    val mountPoint: String,
    val type: FilesystemType,
    val options: MountOptions = MountOptions(),
    val enabled: Boolean = true,
    val bootMount: Boolean = false,
    val userMount: Boolean = false,
    val readOnly: Boolean = false,
    val label: String? = null,
    val uuid: String? = null,
    val backupFrequency: Int = 0,
    val fsckOrder: Int = 0
)

@Serializable
data class MountOptions(
    val standard: List<String> = emptyList(),
    val filesystem: Map<String, String> = emptyMap(),
    val security: SecurityOptions = SecurityOptions(),
    val performance: PerformanceOptions = PerformanceOptions()
)

@Serializable
data class SecurityOptions(
    val noexec: Boolean = false,
    val nosuid: Boolean = false,
    val nodev: Boolean = false,
    val relatime: Boolean = true,
    val strictatime: Boolean = false,
    val sync: Boolean = false,
    val ro: Boolean = false
)

@Serializable
data class PerformanceOptions(
    val noatime: Boolean = false,
    val nodiratime: Boolean = false,
    val commit: Int? = null,
    val barrier: Boolean = true,
    val dataMode: DataMode = DataMode.ORDERED,
    val journalMode: JournalMode = JournalMode.ORDERED
)

@Serializable
data class RAIDStorageConfig(
    val enabled: Boolean = false,
    val arrays: List<RAIDArray> = emptyList(),
    val monitoring: RAIDMonitoring = RAIDMonitoring(),
    val hotspares: List<String> = emptyList(),
    val notifications: RAIDNotifications = RAIDNotifications()
)

@Serializable
data class RAIDArray(
    val name: String,
    val level: RAIDLevel,
    val devices: List<String>,
    val spares: List<String> = emptyList(),
    val chunkSize: String? = null,
    val metadata: RAIDMetadata = RAIDMetadata.DEFAULT,
    val bitmap: RAIDBitmap? = null,
    val layout: RAIDLayout? = null,
    val writePolicy: WritePolicy = WritePolicy.WRITE_BACK,
    val readPolicy: ReadPolicy = ReadPolicy.ADAPTIVE,
    val rebuildPriority: RebuildPriority = RebuildPriority.NORMAL
)

@Serializable
data class RAIDBitmap(
    val enabled: Boolean = true,
    val location: String = "internal",
    val chunkSize: String = "4096"
)

@Serializable
data class RAIDMonitoring(
    val enabled: Boolean = true,
    val checkInterval: Duration = 24.hours,
    val scanSpeed: ScanSpeed = ScanSpeed.NORMAL,
    val emailNotifications: Boolean = true,
    val emailAddress: String? = null,
    val slackWebhook: String? = null
)

@Serializable
data class RAIDNotifications(
    val degraded: Boolean = true,
    val failed: Boolean = true,
    val spareActive: Boolean = true,
    val rebuildStarted: Boolean = true,
    val rebuildFinished: Boolean = true,
    val testFinished: Boolean = false
)

@Serializable
data class EncryptionConfig(
    val enabled: Boolean = false,
    val volumes: List<EncryptedVolume> = emptyList(),
    val keyfiles: List<Keyfile> = emptyList(),
    val tpm: TPMConfig = TPMConfig(),
    val yubikey: YubikeyConfig = YubikeyConfig(),
    val keyManagement: KeyManagementConfig = KeyManagementConfig()
)

@Serializable
data class EncryptedVolume(
    val name: String,
    val device: String,
    val cipher: EncryptionCipher = EncryptionCipher.AES_XTS_PLAIN64,
    val keySize: Int = 512,
    val hashAlgorithm: HashAlgorithm = HashAlgorithm.SHA256,
    val keyFile: String? = null,
    val keySlots: List<KeySlot> = emptyList(),
    val pbkdf: PBKDFConfig = PBKDFConfig(),
    val headerBackup: Boolean = true,
    val allowDiscards: Boolean = true,
    val readOnly: Boolean = false
)

@Serializable
data class KeySlot(
    val slot: Int,
    val keyFile: String? = null,
    val passphrase: Boolean = false,
    val tpm: Boolean = false,
    val yubikey: Boolean = false,
    val priority: Int = 0
)

@Serializable
data class PBKDFConfig(
    val algorithm: PBKDFAlgorithm = PBKDFAlgorithm.ARGON2I,
    val iterations: Int? = null,
    val memory: Int? = null,
    val parallelism: Int? = null,
    val salt: String? = null
)

@Serializable
data class Keyfile(
    val path: String,
    val size: Int = 4096,
    val randomSource: String = "/dev/urandom",
    val permissions: String = "600",
    val owner: String = "root",
    val group: String = "root"
)

@Serializable
data class TPMConfig(
    val enabled: Boolean = false,
    val version: TPMVersion = TPMVersion.TPM2,
    val pcrBanks: List<Int> = listOf(0, 1, 2, 3, 4, 5, 6, 7),
    val sealingPolicy: SealingPolicy = SealingPolicy.SECURE_BOOT,
    val keyHandle: String? = null
)

@Serializable
data class YubikeyConfig(
    val enabled: Boolean = false,
    val slot: Int = 2,
    val challenge: String? = null,
    val timeout: Duration = 15.minutes,
    val touch: Boolean = false
)

@Serializable
data class KeyManagementConfig(
    val autoBackup: Boolean = true,
    val backupLocation: String = "/etc/luks-keys",
    val backupEncryption: Boolean = true,
    val keyRotation: KeyRotationConfig = KeyRotationConfig(),
    val escrow: EscrowConfig = EscrowConfig()
)

@Serializable
data class KeyRotationConfig(
    val enabled: Boolean = false,
    val interval: Duration = 30.hours * 24, // 30 days
    val keepOldKeys: Int = 3,
    val automaticRotation: Boolean = false
)

@Serializable
data class EscrowConfig(
    val enabled: Boolean = false,
    val provider: EscrowProvider = EscrowProvider.NONE,
    val threshold: Int = 3,
    val shares: Int = 5
)

@Serializable
data class BtrfsConfig(
    val enabled: Boolean = false,
    val filesystems: List<BtrfsFilesystem> = emptyList(),
    val snapshots: SnapshotConfig = SnapshotConfig(),
    val compression: CompressionConfig = CompressionConfig(),
    val deduplication: Boolean = false,
    val scrubbing: ScrubConfig = ScrubConfig(),
    val balancing: BalanceConfig = BalanceConfig()
)

@Serializable
data class BtrfsFilesystem(
    val label: String,
    val devices: List<String>,
    val dataProfile: BtrfsProfile = BtrfsProfile.SINGLE,
    val metadataProfile: BtrfsProfile = BtrfsProfile.DUP,
    val systemProfile: BtrfsProfile = BtrfsProfile.DUP,
    val subvolumes: List<Subvolume> = emptyList(),
    val quotas: Boolean = false,
    val qgroups: List<Qgroup> = emptyList()
)

@Serializable
data class Subvolume(
    val name: String,
    val path: String,
    val parent: String? = null,
    val mountPoint: String? = null,
    val mountOptions: List<String> = emptyList(),
    val defaultSubvolume: Boolean = false,
    val readOnly: Boolean = false,
    val quota: SubvolumeQuota? = null,
    val snapshots: SubvolumeSnapshotConfig = SubvolumeSnapshotConfig()
)

@Serializable
data class SubvolumeQuota(
    val enabled: Boolean = false,
    val sizeLimit: String? = null,
    val exclusiveLimit: String? = null
)

@Serializable
data class SubvolumeSnapshotConfig(
    val enabled: Boolean = true,
    val retention: RetentionPolicy = RetentionPolicy(),
    val schedule: SnapshotSchedule = SnapshotSchedule()
)

@Serializable
data class Qgroup(
    val id: String,
    val limit: String? = null,
    val exclusiveLimit: String? = null
)

@Serializable
data class SnapshotConfig(
    val enabled: Boolean = true,
    val location: String = "/.snapshots",
    val retention: RetentionPolicy = RetentionPolicy(),
    val schedule: SnapshotSchedule = SnapshotSchedule(),
    val compression: Boolean = true,
    val verification: Boolean = true
)

@Serializable
data class RetentionPolicy(
    val hourly: Int = 24,
    val daily: Int = 7,
    val weekly: Int = 4,
    val monthly: Int = 12,
    val yearly: Int = 3
)

@Serializable
data class SnapshotSchedule(
    val hourly: Boolean = true,
    val daily: Boolean = true,
    val weekly: Boolean = true,
    val monthly: Boolean = true,
    val yearly: Boolean = true,
    val customCron: String? = null
)

@Serializable
data class CompressionConfig(
    val enabled: Boolean = true,
    val algorithm: CompressionAlgorithm = CompressionAlgorithm.ZSTD,
    val level: Int = 3,
    val autoCompress: Boolean = true,
    val compressibleTypes: List<String> = listOf("text", "application", "image")
)

@Serializable
data class ScrubConfig(
    val enabled: Boolean = true,
    val schedule: String = "0 2 * * 0", // Weekly at 2 AM
    val priority: ScrubbingPriority = ScrubbingPriority.NORMAL,
    val readOnly: Boolean = false,
    val forceCheck: Boolean = false
)

@Serializable
data class BalanceConfig(
    val enabled: Boolean = false,
    val schedule: String = "0 3 1 * *", // Monthly at 3 AM
    val dataThreshold: Int = 85,
    val metadataThreshold: Int = 90,
    val autoBalance: Boolean = true
)

@Serializable
data class SwapConfig(
    val enabled: Boolean = true,
    val type: SwapType = SwapType.ZRAM,
    val size: String = "auto",
    val priority: Int = 10,
    val files: List<SwapFile> = emptyList(),
    val partitions: List<SwapPartition> = emptyList(),
    val zram: ZramConfig = ZramConfig(),
    val zswap: ZswapConfig = ZswapConfig(),
    val swappiness: Int = 10,
    val vfsCache: Int = 50
)

@Serializable
data class SwapFile(
    val path: String,
    val size: String,
    val priority: Int = 0,
    val preallocate: Boolean = true,
    val permissions: String = "600"
)

@Serializable
data class SwapPartition(
    val device: String,
    val priority: Int = 0,
    val uuid: String? = null,
    val label: String? = null
)

@Serializable
data class ZramConfig(
    val enabled: Boolean = true,
    val size: String = "50%",
    val algorithm: CompressionAlgorithm = CompressionAlgorithm.LZ4,
    val streams: Int = 0, // Auto-detect
    val priority: Int = 100,
    val disksize: String = "auto"
)

@Serializable
data class ZswapConfig(
    val enabled: Boolean = false,
    val compressor: CompressionAlgorithm = CompressionAlgorithm.LZ4,
    val zpool: String = "z3fold",
    val maxPoolPercent: Int = 20,
    val acceptThreshold: Int = 90,
    val enabledParam: Boolean = false
)

@Serializable
data class AutoMountConfig(
    val enabled: Boolean = true,
    val removableMedia: RemovableMediaConfig = RemovableMediaConfig(),
    val networkShares: NetworkSharesConfig = NetworkSharesConfig(),
    val encryptedVolumes: EncryptedAutoMountConfig = EncryptedAutoMountConfig()
)

@Serializable
data class RemovableMediaConfig(
    val enabled: Boolean = true,
    val mountPoint: String = "/media",
    val fileManager: Boolean = true,
    val desktop: Boolean = true,
    val userMount: Boolean = true,
    val umask: String = "022",
    val options: List<String> = emptyList()
)

@Serializable
data class NetworkSharesConfig(
    val enabled: Boolean = true,
    val samba: SambaAutoMountConfig = SambaAutoMountConfig(),
    val nfs: NFSAutoMountConfig = NFSAutoMountConfig(),
    val ssh: SSHAutoMountConfig = SSHAutoMountConfig()
)

@Serializable
data class SambaAutoMountConfig(
    val enabled: Boolean = true,
    val workgroup: String = "WORKGROUP",
    val credentials: String? = null,
    val version: String = "3.0",
    val encryption: Boolean = true
)

@Serializable
data class NFSAutoMountConfig(
    val enabled: Boolean = true,
    val version: String = "4.0",
    val timeout: Duration = 30.minutes,
    val retrans: Int = 3,
    val rsize: Int = 32768,
    val wsize: Int = 32768
)

@Serializable
data class SSHAutoMountConfig(
    val enabled: Boolean = false,
    val compression: Boolean = true,
    val identityFile: String? = null,
    val port: Int = 22,
    val followSymlinks: Boolean = true
)

@Serializable
data class EncryptedAutoMountConfig(
    val enabled: Boolean = true,
    val keyring: Boolean = true,
    val passwordPrompt: Boolean = true,
    val timeout: Duration = 5.minutes
)

@Serializable
data class StorageMaintenanceConfig(
    val enabled: Boolean = true,
    val fsck: FsckConfig = FsckConfig(),
    val defragmentation: DefragmentationConfig = DefragmentationConfig(),
    val trim: TrimConfig = TrimConfig(),
    val healthChecks: HealthCheckConfig = HealthCheckConfig()
)

@Serializable
data class FsckConfig(
    val enabled: Boolean = true,
    val schedule: String = "0 4 * * 0", // Weekly at 4 AM
    val forceCheck: Boolean = false,
    val autoFix: Boolean = false,
    val skipRoot: Boolean = false
)

@Serializable
data class DefragmentationConfig(
    val enabled: Boolean = false,
    val schedule: String = "0 5 1 * *", // Monthly at 5 AM
    val filesystems: List<String> = listOf("ext4", "btrfs"),
    val threshold: Int = 10, // Fragmentation percentage
    val maxFiles: Int = 1000
)

@Serializable
data class TrimConfig(
    val enabled: Boolean = true,
    val schedule: String = "0 6 * * 0", // Weekly at 6 AM
    val continuous: Boolean = false,
    val filesystems: List<String> = listOf("ext4", "xfs", "btrfs")
)

@Serializable
data class HealthCheckConfig(
    val enabled: Boolean = true,
    val schedule: String = "0 7 * * *", // Daily at 7 AM
    val smart: SmartConfig = SmartConfig(),
    val badBlocks: BadBlocksConfig = BadBlocksConfig()
)

@Serializable
data class SmartConfig(
    val enabled: Boolean = true,
    val testSchedule: String = "0 8 * * 0", // Weekly at 8 AM
    val testType: SmartTestType = SmartTestType.LONG,
    val temperature: SmartTemperatureConfig = SmartTemperatureConfig(),
    val attributes: List<SmartAttribute> = emptyList()
)

@Serializable
data class SmartTemperatureConfig(
    val warning: Int = 60,
    val critical: Int = 70,
    val monitoring: Boolean = true
)

@Serializable
data class SmartAttribute(
    val id: Int,
    val name: String,
    val threshold: Int,
    val critical: Boolean = false
)

@Serializable
data class BadBlocksConfig(
    val enabled: Boolean = false,
    val schedule: String = "0 9 1 * *", // Monthly at 9 AM
    val destructive: Boolean = false,
    val pattern: BadBlockPattern = BadBlockPattern.RANDOM
)

@Serializable
data class StorageMonitoringConfig(
    val enabled: Boolean = true,
    val diskUsage: DiskUsageConfig = DiskUsageConfig(),
    val performance: PerformanceMonitoringConfig = PerformanceMonitoringConfig(),
    val notifications: StorageNotifications = StorageNotifications()
)

@Serializable
data class DiskUsageConfig(
    val enabled: Boolean = true,
    val interval: Duration = 5.minutes,
    val warningThreshold: Int = 80,
    val criticalThreshold: Int = 90,
    val ignoreFilesystems: List<String> = listOf("tmpfs", "devtmpfs", "proc", "sys")
)

@Serializable
data class PerformanceMonitoringConfig(
    val enabled: Boolean = true,
    val interval: Duration = 30.minutes,
    val ioStats: Boolean = true,
    val latencyTracking: Boolean = true,
    val queueDepth: Boolean = true
)

@Serializable
data class StorageNotifications(
    val email: Boolean = false,
    val desktop: Boolean = true,
    val syslog: Boolean = true,
    val webhook: String? = null
)

// ===== Enums =====

@Serializable
enum class FilesystemType {
    EXT4,
    EXT3,
    EXT2,
    XFS,
    BTRFS,
    F2FS,
    NILFS2,
    REISERFS,
    JFS,
    NTFS,
    VFAT,
    EXFAT,
    ISO9660,
    UDF,
    TMPFS,
    RAMFS,
    PROC,
    SYSFS,
    DEVTMPFS,
    CGROUP,
    CGROUP2,
    FUSE,
    NFS,
    CIFS,
    SSHFS,
    OVERLAY,
    AUFS,
    BIND
}

@Serializable
enum class DataMode {
    JOURNAL,
    ORDERED,
    WRITEBACK
}

@Serializable
enum class JournalMode {
    JOURNAL,
    ORDERED,
    WRITEBACK
}

@Serializable
enum class RAIDLevel {
    RAID0,
    RAID1,
    RAID4,
    RAID5,
    RAID6,
    RAID10,
    LINEAR
}

@Serializable
enum class RAIDMetadata {
    DEFAULT,
    V0_90,
    V1_0,
    V1_1,
    V1_2
}

@Serializable
enum class RAIDLayout {
    LEFT_ASYMMETRIC,
    LEFT_SYMMETRIC,
    RIGHT_ASYMMETRIC,
    RIGHT_SYMMETRIC,
    NEAR,
    FAR,
    OFFSET
}

@Serializable
enum class WritePolicy {
    WRITE_THROUGH,
    WRITE_BACK,
    WRITE_AROUND
}

@Serializable
enum class ReadPolicy {
    READ_BALANCE,
    ADAPTIVE,
    ROUND_ROBIN
}

@Serializable
enum class RebuildPriority {
    LOW,
    NORMAL,
    HIGH
}

@Serializable
enum class ScanSpeed {
    VERY_LOW,
    LOW,
    NORMAL,
    HIGH,
    VERY_HIGH
}

@Serializable
enum class EncryptionCipher {
    AES_XTS_PLAIN64,
    AES_CBC_ESSIV,
    AES_LRW_BENBI,
    TWOFISH_XTS_PLAIN64,
    SERPENT_XTS_PLAIN64,
    CAMELLIA_XTS_PLAIN64
}

@Serializable
enum class HashAlgorithm {
    SHA1,
    SHA256,
    SHA512,
    RIPEMD160,
    WHIRLPOOL
}

@Serializable
enum class PBKDFAlgorithm {
    PBKDF2,
    ARGON2I,
    ARGON2ID
}

@Serializable
enum class TPMVersion {
    TPM1,
    TPM2
}

@Serializable
enum class SealingPolicy {
    SECURE_BOOT,
    MEASURED_BOOT,
    CUSTOM
}

@Serializable
enum class EscrowProvider {
    NONE,
    SHAMIR,
    VAULT,
    CUSTOM
}

@Serializable
enum class BtrfsProfile {
    SINGLE,
    DUP,
    RAID0,
    RAID1,
    RAID5,
    RAID6,
    RAID10,
    RAID1C3,
    RAID1C4
}

@Serializable
enum class CompressionAlgorithm {
    NONE,
    LZO,
    ZLIB,
    ZSTD,
    LZ4
}

@Serializable
enum class ScrubbingPriority {
    LOW,
    NORMAL,
    HIGH
}

@Serializable
enum class SwapType {
    PARTITION,
    FILE,
    ZRAM,
    ZSWAP,
    HYBRID
}

@Serializable
enum class SmartTestType {
    SHORT,
    LONG,
    CONVEYANCE,
    OFFLINE
}

@Serializable
enum class BadBlockPattern {
    RANDOM,
    SEQUENTIAL,
    PATTERN_AA,
    PATTERN_55,
    PATTERN_FF
}

// ===== DSL Builders =====

@HorizonOSDsl
class StorageContext {
    private val filesystems = mutableListOf<FilesystemConfig>()
    private var raid = RAIDStorageConfig()
    private var encryption = EncryptionConfig()
    private var btrfs = BtrfsConfig()
    private var swap = SwapConfig()
    private var autoMount = AutoMountConfig()
    private var maintenance = StorageMaintenanceConfig()
    private var monitoring = StorageMonitoringConfig()
    
    fun filesystem(device: String, mountPoint: String, type: FilesystemType, block: FilesystemContext.() -> Unit = {}) {
        val context = FilesystemContext(device, mountPoint, type).apply(block)
        filesystems.add(context.toConfig())
    }
    
    fun raid(block: RAIDStorageContext.() -> Unit) {
        raid = RAIDStorageContext().apply(block).toConfig()
    }
    
    fun encryption(block: EncryptionContext.() -> Unit) {
        encryption = EncryptionContext().apply(block).toConfig()
    }
    
    fun btrfs(block: BtrfsContext.() -> Unit) {
        btrfs = BtrfsContext().apply(block).toConfig()
    }
    
    fun swap(block: SwapContext.() -> Unit) {
        swap = SwapContext().apply(block).toConfig()
    }
    
    fun autoMount(block: AutoMountContext.() -> Unit) {
        autoMount = AutoMountContext().apply(block).toConfig()
    }
    
    fun maintenance(block: StorageMaintenanceContext.() -> Unit) {
        maintenance = StorageMaintenanceContext().apply(block).toConfig()
    }
    
    fun monitoring(block: StorageMonitoringContext.() -> Unit) {
        monitoring = StorageMonitoringContext().apply(block).toConfig()
    }
    
    fun toConfig() = StorageConfig(
        filesystems = filesystems,
        raid = raid,
        encryption = encryption,
        btrfs = btrfs,
        swap = swap,
        autoMount = autoMount,
        maintenance = maintenance,
        monitoring = monitoring
    )
}

@HorizonOSDsl
class FilesystemContext(
    private val device: String,
    private val mountPoint: String,
    private val type: FilesystemType
) {
    private var options = MountOptions()
    var enabled: Boolean = true
    var bootMount: Boolean = false
    var userMount: Boolean = false
    var readOnly: Boolean = false
    var label: String? = null
    var uuid: String? = null
    var backupFrequency: Int = 0
    var fsckOrder: Int = 0
    
    fun options(block: MountOptionsContext.() -> Unit) {
        options = MountOptionsContext().apply(block).toConfig()
    }
    
    fun toConfig() = FilesystemConfig(
        device = device,
        mountPoint = mountPoint,
        type = type,
        options = options,
        enabled = enabled,
        bootMount = bootMount,
        userMount = userMount,
        readOnly = readOnly,
        label = label,
        uuid = uuid,
        backupFrequency = backupFrequency,
        fsckOrder = fsckOrder
    )
}

@HorizonOSDsl
class MountOptionsContext {
    private val standard = mutableListOf<String>()
    private val filesystem = mutableMapOf<String, String>()
    private var security = SecurityOptions()
    private var performance = PerformanceOptions()
    
    fun standard(vararg options: String) {
        standard.addAll(options)
    }
    
    fun filesystem(key: String, value: String) {
        filesystem[key] = value
    }
    
    fun security(block: SecurityOptionsContext.() -> Unit) {
        security = SecurityOptionsContext().apply(block).toConfig()
    }
    
    fun performance(block: PerformanceOptionsContext.() -> Unit) {
        performance = PerformanceOptionsContext().apply(block).toConfig()
    }
    
    fun toConfig() = MountOptions(
        standard = standard,
        filesystem = filesystem,
        security = security,
        performance = performance
    )
}

// Placeholder context classes for comprehensive DSL structure
@HorizonOSDsl class SecurityOptionsContext { fun toConfig() = SecurityOptions() }
@HorizonOSDsl class PerformanceOptionsContext { fun toConfig() = PerformanceOptions() }
@HorizonOSDsl class RAIDStorageContext { fun toConfig() = RAIDStorageConfig() }
@HorizonOSDsl class EncryptionContext { fun toConfig() = EncryptionConfig() }
@HorizonOSDsl class BtrfsContext { fun toConfig() = BtrfsConfig() }
@HorizonOSDsl class SwapContext { fun toConfig() = SwapConfig() }
@HorizonOSDsl class AutoMountContext { fun toConfig() = AutoMountConfig() }
@HorizonOSDsl class StorageMaintenanceContext { fun toConfig() = StorageMaintenanceConfig() }
@HorizonOSDsl class StorageMonitoringContext { fun toConfig() = StorageMonitoringConfig() }

// ===== Extension Functions =====

fun CompiledConfig.hasStorage(): Boolean = storage != null

fun CompiledConfig.getFilesystem(mountPoint: String): FilesystemConfig? = 
    storage?.filesystems?.find { it.mountPoint == mountPoint }

fun CompiledConfig.getRAIDArray(name: String): RAIDArray? = 
    storage?.raid?.arrays?.find { it.name == name }

fun CompiledConfig.getEncryptedVolume(name: String): EncryptedVolume? = 
    storage?.encryption?.volumes?.find { it.name == name }

fun CompiledConfig.getBtrfsFilesystem(label: String): BtrfsFilesystem? = 
    storage?.btrfs?.filesystems?.find { it.label == label }

fun CompiledConfig.getSubvolume(name: String): Subvolume? = 
    storage?.btrfs?.filesystems?.flatMap { it.subvolumes }?.find { it.name == name }

fun CompiledConfig.hasEncryption(): Boolean = 
    storage?.encryption?.enabled == true

fun CompiledConfig.hasRAID(): Boolean = 
    storage?.raid?.enabled == true

fun CompiledConfig.hasBtrfs(): Boolean = 
    storage?.btrfs?.enabled == true

fun CompiledConfig.hasSwap(): Boolean = 
    storage?.swap?.enabled == true