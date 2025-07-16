package org.horizonos.config.dsl.storage.filesystem

import org.horizonos.config.dsl.HorizonOSDsl

// ===== Filesystem DSL Builders =====

@HorizonOSDsl
class FilesystemContext(
    private val device: String,
    private val mountPoint: String,
    private val type: FilesystemType
) {
    var enabled = true
    var bootMount = false
    var userMount = false
    var readOnly = false
    var label: String? = null
    var uuid: String? = null
    var backupFrequency = 0
    var fsckOrder = 0
    
    private val standardOptions = mutableListOf<String>()
    private val filesystemOptions = mutableMapOf<String, String>()
    private var securityOptions = SecurityOptions()
    private var performanceOptions = PerformanceOptions()
    
    fun option(opt: String) {
        standardOptions.add(opt)
    }
    
    fun fsOption(key: String, value: String) {
        filesystemOptions[key] = value
    }
    
    fun security(block: SecurityOptionsContext.() -> Unit) {
        securityOptions = SecurityOptionsContext().apply(block).toConfig()
    }
    
    fun performance(block: PerformanceOptionsContext.() -> Unit) {
        performanceOptions = PerformanceOptionsContext().apply(block).toConfig()
    }
    
    fun toConfig() = FilesystemConfig(
        device = device,
        mountPoint = mountPoint,
        type = type,
        options = MountOptions(
            standard = standardOptions,
            filesystem = filesystemOptions,
            security = securityOptions,
            performance = performanceOptions
        ),
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
class SecurityOptionsContext {
    var noexec = false
    var nosuid = false
    var nodev = false
    var relatime = true
    var strictatime = false
    var sync = false
    var ro = false
    
    fun toConfig() = SecurityOptions(
        noexec = noexec,
        nosuid = nosuid,
        nodev = nodev,
        relatime = relatime,
        strictatime = strictatime,
        sync = sync,
        ro = ro
    )
}

@HorizonOSDsl
class PerformanceOptionsContext {
    var noatime = false
    var nodiratime = false
    var commit: Int? = null
    var barrier = true
    var dataMode = DataMode.ORDERED
    var journalMode = JournalMode.ORDERED
    
    fun toConfig() = PerformanceOptions(
        noatime = noatime,
        nodiratime = nodiratime,
        commit = commit,
        barrier = barrier,
        dataMode = dataMode,
        journalMode = journalMode
    )
}

@HorizonOSDsl
class AutoMountContext {
    var enabled = true
    private var removableDevices = RemovableDeviceConfig()
    private var networkShares = NetworkShareConfig()
    private var userMountOptions = UserMountOptions()
    
    fun removableDevices(block: RemovableDeviceContext.() -> Unit) {
        removableDevices = RemovableDeviceContext().apply(block).toConfig()
    }
    
    fun networkShares(block: NetworkShareContext.() -> Unit) {
        networkShares = NetworkShareContext().apply(block).toConfig()
    }
    
    fun userMountOptions(block: UserMountOptionsContext.() -> Unit) {
        userMountOptions = UserMountOptionsContext().apply(block).toConfig()
    }
    
    fun toConfig() = AutoMountConfig(
        enabled = enabled,
        removableDevices = removableDevices,
        networkShares = networkShares,
        userMountOptions = userMountOptions
    )
}

@HorizonOSDsl
class RemovableDeviceContext {
    var autoMountUSB = true
    var autoMountCD = true
    var autoMountSDCard = true
    val mountOptions = mutableListOf("noexec", "nosuid")
    var mountPath = "/media"
    var ownedByUser = true
    
    fun mountOption(opt: String) {
        mountOptions.add(opt)
    }
    
    fun toConfig() = RemovableDeviceConfig(
        autoMountUSB = autoMountUSB,
        autoMountCD = autoMountCD,
        autoMountSDCard = autoMountSDCard,
        mountOptions = mountOptions,
        mountPath = mountPath,
        ownedByUser = ownedByUser
    )
}

@HorizonOSDsl
class NetworkShareContext {
    var autoMountNFS = false
    var autoMountCIFS = false
    var autoMountSSHFS = false
    var timeout = 30
    var retryCount = 3
    
    fun toConfig() = NetworkShareConfig(
        autoMountNFS = autoMountNFS,
        autoMountCIFS = autoMountCIFS,
        autoMountSSHFS = autoMountSSHFS,
        timeout = timeout,
        retryCount = retryCount
    )
}

@HorizonOSDsl
class UserMountOptionsContext {
    var allowUserMount = true
    val allowedFilesystems = mutableListOf(
        FilesystemType.EXT4,
        FilesystemType.VFAT,
        FilesystemType.NTFS,
        FilesystemType.EXFAT
    )
    var maxMountPoints = 10
    val restrictedPaths = mutableListOf("/", "/boot", "/etc", "/usr", "/var")
    
    fun allowFilesystem(type: FilesystemType) {
        allowedFilesystems.add(type)
    }
    
    fun restrictPath(path: String) {
        restrictedPaths.add(path)
    }
    
    fun toConfig() = UserMountOptions(
        allowUserMount = allowUserMount,
        allowedFilesystems = allowedFilesystems,
        maxMountPoints = maxMountPoints,
        restrictedPaths = restrictedPaths
    )
}