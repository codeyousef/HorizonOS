package org.horizonos.config.dsl.storage.filesystem

import kotlinx.serialization.Serializable
import org.horizonos.config.dsl.HorizonOSDsl

// ===== Filesystem Configuration =====

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
data class AutoMountConfig(
    val enabled: Boolean = true,
    val removableDevices: RemovableDeviceConfig = RemovableDeviceConfig(),
    val networkShares: NetworkShareConfig = NetworkShareConfig(),
    val userMountOptions: UserMountOptions = UserMountOptions()
)

@Serializable
data class RemovableDeviceConfig(
    val autoMountUSB: Boolean = true,
    val autoMountCD: Boolean = true,
    val autoMountSDCard: Boolean = true,
    val mountOptions: List<String> = listOf("noexec", "nosuid"),
    val mountPath: String = "/media",
    val ownedByUser: Boolean = true
)

@Serializable
data class NetworkShareConfig(
    val autoMountNFS: Boolean = false,
    val autoMountCIFS: Boolean = false,
    val autoMountSSHFS: Boolean = false,
    val timeout: Int = 30,
    val retryCount: Int = 3
)

@Serializable
data class UserMountOptions(
    val allowUserMount: Boolean = true,
    val allowedFilesystems: List<FilesystemType> = listOf(
        FilesystemType.EXT4,
        FilesystemType.VFAT,
        FilesystemType.NTFS,
        FilesystemType.EXFAT
    ),
    val maxMountPoints: Int = 10,
    val restrictedPaths: List<String> = listOf("/", "/boot", "/etc", "/usr", "/var")
)

// Filesystem Type Enum
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