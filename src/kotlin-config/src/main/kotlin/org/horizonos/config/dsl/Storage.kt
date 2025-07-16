package org.horizonos.config.dsl

import kotlinx.serialization.Serializable
import org.horizonos.config.dsl.storage.filesystem.*
import org.horizonos.config.dsl.storage.filesystem.FilesystemType
import org.horizonos.config.dsl.storage.raid.*
import org.horizonos.config.dsl.storage.encryption.*
import org.horizonos.config.dsl.storage.btrfs.*
import org.horizonos.config.dsl.storage.swap.*
import org.horizonos.config.dsl.storage.monitoring.*
import org.horizonos.config.dsl.storage.maintenance.*

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
data class AutoMountConfig(
    val enabled: Boolean = true,
    val timeout: Int = 60,
    val showInFileManager: Boolean = true,
    val allowPolkitActions: Boolean = true
)

// ===== Storage DSL Builder =====

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
class AutoMountContext {
    var enabled = true
    var timeout = 60
    var showInFileManager = true
    var allowPolkitActions = true
    
    fun toConfig() = AutoMountConfig(
        enabled = enabled,
        timeout = timeout,
        showInFileManager = showInFileManager,
        allowPolkitActions = allowPolkitActions
    )
}

// ===== Storage DSL Function =====

@HorizonOSDsl
fun storage(block: StorageContext.() -> Unit): StorageConfig =
    StorageContext().apply(block).toConfig()