package org.horizonos.config.dsl.storage.filesystem

import kotlinx.serialization.Serializable
import org.horizonos.config.dsl.HorizonOSDsl

/**
 * Filesystem Configuration for HorizonOS Storage
 * 
 * This module provides comprehensive filesystem configuration including mount options,
 * security settings, performance tuning, and automatic mounting for various filesystem
 * types and storage devices.
 * 
 * ## Key Features:
 * - Support for multiple filesystem types (ext4, btrfs, xfs, etc.)
 * - Granular mount options and security settings
 * - Performance optimization for different use cases
 * - Automatic mounting of removable devices and network shares
 * - User-controlled mounting with security restrictions
 * 
 * ## Usage Example:
 * ```kotlin
 * storage {
 *     filesystem {
 *         mount("/dev/sda1", "/") {
 *             type = FilesystemType.BTRFS
 *             options {
 *                 security {
 *                     noexec = false
 *                     nosuid = false
 *                     relatime = true
 *                 }
 *                 performance {
 *                     noatime = true
 *                     commit = 30
 *                 }
 *             }
 *         }
 *     }
 * }
 * ```
 * 
 * @since 1.0
 */

// ===== Filesystem Configuration =====

/**
 * Filesystem configuration for a single mount point
 * 
 * This class defines a complete filesystem mount configuration including
 * device, mount point, filesystem type, and various mount options.
 */
@Serializable
data class FilesystemConfig(
    /** Device path or identifier (e.g., "/dev/sda1", "UUID=...") */
    val device: String,
    
    /** Mount point path where the filesystem will be mounted */
    val mountPoint: String,
    
    /** Filesystem type (ext4, btrfs, xfs, etc.) */
    val type: FilesystemType,
    
    /** Mount options for security, performance, and behavior */
    val options: MountOptions = MountOptions(),
    
    /** Whether this filesystem is enabled for mounting */
    val enabled: Boolean = true,
    
    /** Whether to mount this filesystem at boot time */
    val bootMount: Boolean = false,
    
    /** Whether users can mount/unmount this filesystem */
    val userMount: Boolean = false,
    
    /** Whether to mount as read-only */
    val readOnly: Boolean = false,
    
    /** Filesystem label for identification */
    val label: String? = null,
    
    /** Filesystem UUID for identification */
    val uuid: String? = null,
    
    /** Dump backup frequency (0=never, 1=daily, 2=every other day) */
    val backupFrequency: Int = 0,
    
    /** Filesystem check order (0=no check, 1=root fs, 2=others) */
    val fsckOrder: Int = 0
)

/**
 * Mount options for filesystem configuration
 * 
 * This class organizes mount options into standard, filesystem-specific,
 * security, and performance categories.
 */
@Serializable
data class MountOptions(
    /** Standard mount options (e.g., "defaults", "noauto") */
    val standard: List<String> = emptyList(),
    
    /** Filesystem-specific options as key-value pairs */
    val filesystem: Map<String, String> = emptyMap(),
    
    /** Security-related mount options */
    val security: SecurityOptions = SecurityOptions(),
    
    /** Performance-related mount options */
    val performance: PerformanceOptions = PerformanceOptions()
)

/**
 * Security-related mount options
 * 
 * This class defines security options that control execution permissions,
 * device access, and file access time behavior.
 */
@Serializable
data class SecurityOptions(
    /** Prevent execution of binaries from this filesystem */
    val noexec: Boolean = false,
    
    /** Prevent setuid/setgid bit execution from this filesystem */
    val nosuid: Boolean = false,
    
    /** Prevent device file access from this filesystem */
    val nodev: Boolean = false,
    
    /** Update access times relative to modify/change times */
    val relatime: Boolean = true,
    
    /** Always update access times (strict POSIX behavior) */
    val strictatime: Boolean = false,
    
    /** Synchronous I/O operations (slower but safer) */
    val sync: Boolean = false,
    
    /** Mount as read-only */
    val ro: Boolean = false
)

/**
 * Performance-related mount options
 * 
 * This class defines options that affect filesystem performance,
 * including access time updates and journaling behavior.
 */
@Serializable
data class PerformanceOptions(
    /** Never update access times (fastest) */
    val noatime: Boolean = false,
    
    /** Don't update directory access times */
    val nodiratime: Boolean = false,
    
    /** Journal commit interval in seconds (ext3/4, btrfs) */
    val commit: Int? = null,
    
    /** Use write barriers for data integrity */
    val barrier: Boolean = true,
    
    /** Data journaling mode for ext3/4 filesystems */
    val dataMode: DataMode = DataMode.ORDERED,
    
    /** Journal mode for transaction handling */
    val journalMode: JournalMode = JournalMode.ORDERED
)

/**
 * Automatic mounting configuration
 * 
 * This class configures automatic mounting of removable devices and network shares.
 */
@Serializable
data class AutoMountConfig(
    /** Whether automatic mounting is enabled */
    val enabled: Boolean = true,
    
    /** Configuration for removable devices (USB, CD, SD cards) */
    val removableDevices: RemovableDeviceConfig = RemovableDeviceConfig(),
    
    /** Configuration for network shares (NFS, CIFS, SSHFS) */
    val networkShares: NetworkShareConfig = NetworkShareConfig(),
    
    /** Options for user-controlled mounting */
    val userMountOptions: UserMountOptions = UserMountOptions()
)

/**
 * Configuration for removable device mounting
 * 
 * This class defines how removable devices like USB drives, CDs, and SD cards
 * are automatically mounted by the system.
 */
@Serializable
data class RemovableDeviceConfig(
    /** Whether to automatically mount USB devices */
    val autoMountUSB: Boolean = true,
    
    /** Whether to automatically mount CD/DVD devices */
    val autoMountCD: Boolean = true,
    
    /** Whether to automatically mount SD cards */
    val autoMountSDCard: Boolean = true,
    
    /** Default mount options for removable devices */
    val mountOptions: List<String> = listOf("noexec", "nosuid"),
    
    /** Base path where removable devices are mounted */
    val mountPath: String = "/media",
    
    /** Whether mounted devices are owned by the user */
    val ownedByUser: Boolean = true
)

/**
 * Configuration for network share mounting
 * 
 * This class defines automatic mounting behavior for network filesystems.
 */
@Serializable
data class NetworkShareConfig(
    /** Whether to automatically mount NFS shares */
    val autoMountNFS: Boolean = false,
    
    /** Whether to automatically mount CIFS/SMB shares */
    val autoMountCIFS: Boolean = false,
    
    /** Whether to automatically mount SSHFS shares */
    val autoMountSSHFS: Boolean = false,
    
    /** Network timeout in seconds for mount operations */
    val timeout: Int = 30,
    
    /** Number of mount retries on failure */
    val retryCount: Int = 3
)

/**
 * User mount permission configuration
 * 
 * This class defines what filesystems and mount points users are allowed
 * to mount and unmount through user-space tools.
 */
@Serializable
data class UserMountOptions(
    /** Whether users can mount/unmount filesystems */
    val allowUserMount: Boolean = true,
    
    /** List of filesystem types users are allowed to mount */
    val allowedFilesystems: List<FilesystemType> = listOf(
        FilesystemType.EXT4,
        FilesystemType.VFAT,
        FilesystemType.NTFS,
        FilesystemType.EXFAT
    ),
    
    /** Maximum number of mount points per user */
    val maxMountPoints: Int = 10,
    
    /** System paths users are not allowed to mount over */
    val restrictedPaths: List<String> = listOf("/", "/boot", "/etc", "/usr", "/var")
)

// Filesystem Type Enum

/**
 * Supported filesystem types in HorizonOS
 * 
 * This enum covers all filesystem types that can be configured and mounted,
 * including traditional filesystems, network filesystems, and virtual filesystems.
 */
@Serializable
enum class FilesystemType {
    /** ext4 - Fourth extended filesystem (default Linux filesystem) */
    EXT4,
    
    /** ext3 - Third extended filesystem (journaled) */
    EXT3,
    
    /** ext2 - Second extended filesystem (no journaling) */
    EXT2,
    
    /** XFS - High-performance 64-bit journaling filesystem */
    XFS,
    
    /** Btrfs - B-tree filesystem with snapshots and subvolumes */
    BTRFS,
    
    /** F2FS - Flash-friendly filesystem for SSDs */
    F2FS,
    
    /** NILFS2 - New implementation of log-structured filesystem */
    NILFS2,
    
    /** ReiserFS - Journaling filesystem (legacy) */
    REISERFS,
    
    /** JFS - Journaled File System from IBM */
    JFS,
    
    /** NTFS - Windows NT File System */
    NTFS,
    
    /** VFAT - Virtual File Allocation Table (FAT32) */
    VFAT,
    
    /** exFAT - Extended File Allocation Table */
    EXFAT,
    
    /** ISO9660 - CD-ROM filesystem */
    ISO9660,
    
    /** UDF - Universal Disk Format (DVD/Blu-ray) */
    UDF,
    
    /** tmpfs - Temporary filesystem in RAM */
    TMPFS,
    
    /** ramfs - RAM-based filesystem */
    RAMFS,
    
    /** proc - Process information virtual filesystem */
    PROC,
    
    /** sysfs - System information virtual filesystem */
    SYSFS,
    
    /** devtmpfs - Device nodes temporary filesystem */
    DEVTMPFS,
    
    /** cgroup - Control group filesystem (v1) */
    CGROUP,
    
    /** cgroup2 - Control group filesystem (v2) */
    CGROUP2,
    
    /** FUSE - Filesystem in Userspace */
    FUSE,
    
    /** NFS - Network File System */
    NFS,
    
    /** CIFS - Common Internet File System (SMB/CIFS) */
    CIFS,
    
    /** SSHFS - SSH Filesystem */
    SSHFS,
    
    /** overlay - Overlay filesystem for containers */
    OVERLAY,
    
    /** aufs - Advanced multi-layered unification filesystem */
    AUFS,
    
    /** bind - Bind mount (remount part of filesystem tree) */
    BIND
}

/**
 * Data journaling modes for ext3/4 filesystems
 * 
 * These modes control how data is journaled for crash recovery.
 */
@Serializable
enum class DataMode {
    /** Journal all data - slowest but safest */
    JOURNAL,
    
    /** Journal metadata only, ordered data writes - balanced */
    ORDERED,
    
    /** Journal metadata only, no data ordering - fastest */
    WRITEBACK
}

/**
 * Journal transaction modes
 * 
 * These modes control how filesystem transactions are handled.
 */
@Serializable
enum class JournalMode {
    /** Full journaling of all changes */
    JOURNAL,
    
    /** Ordered journaling - metadata journaled, data ordered */
    ORDERED,
    
    /** Writeback journaling - metadata only, no data ordering */
    WRITEBACK
}