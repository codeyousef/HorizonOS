package org.horizonos.config.dsl.storage.swap

import org.horizonos.config.dsl.HorizonOSDsl

// ===== Swap DSL Builders =====

@HorizonOSDsl
class SwapContext {
    var enabled = true
    private val devices = mutableListOf<SwapDevice>()
    private val files = mutableListOf<SwapFile>()
    private var zram = ZramConfig()
    private var zswap = ZswapConfig()
    var swappiness = 60
    var vfsCachePressure = 100
    var minFreeKbytes: Long? = null
    var watermarkScaleFactor = 10
    
    fun device(device: String, block: SwapDeviceContext.() -> Unit = {}) {
        val context = SwapDeviceContext(device).apply(block)
        devices.add(context.toConfig())
    }
    
    fun file(path: String, size: String, block: SwapFileContext.() -> Unit = {}) {
        val context = SwapFileContext(path, size).apply(block)
        files.add(context.toConfig())
    }
    
    fun zram(block: ZramConfigContext.() -> Unit) {
        zram = ZramConfigContext().apply(block).toConfig()
    }
    
    fun zswap(block: ZswapConfigContext.() -> Unit) {
        zswap = ZswapConfigContext().apply(block).toConfig()
    }
    
    fun toConfig() = SwapConfig(
        enabled = enabled,
        devices = devices,
        files = files,
        zram = zram,
        zswap = zswap,
        swappiness = swappiness,
        vfsCachePressure = vfsCachePressure,
        minFreeKbytes = minFreeKbytes,
        watermarkScaleFactor = watermarkScaleFactor
    )
}

@HorizonOSDsl
class SwapDeviceContext(private val device: String) {
    var priority = -1
    var discardPolicy = DiscardPolicy.NONE
    var label: String? = null
    var uuid: String? = null
    
    fun toConfig() = SwapDevice(
        device = device,
        priority = priority,
        discardPolicy = discardPolicy,
        label = label,
        uuid = uuid
    )
}

@HorizonOSDsl
class SwapFileContext(
    private val path: String,
    private val size: String
) {
    var priority = -1
    var permissions = "0600"
    var allocateMode = AllocateMode.FALLOCATE
    var filesystem: String? = null
    
    fun toConfig() = SwapFile(
        path = path,
        size = size,
        priority = priority,
        permissions = permissions,
        allocateMode = allocateMode,
        filesystem = filesystem
    )
}

@HorizonOSDsl
class ZramConfigContext {
    var enabled = false
    private val devices = mutableListOf<ZramDevice>()
    var algorithm = CompressionAlgorithm.LZ4
    var streams: Int? = null
    
    fun device(name: String = "zram0", size: String = "50%", block: ZramDeviceContext.() -> Unit = {}) {
        val context = ZramDeviceContext(name, size).apply(block)
        devices.add(context.toConfig())
    }
    
    fun toConfig() = ZramConfig(
        enabled = enabled,
        devices = devices,
        algorithm = algorithm,
        streams = streams
    )
}

@HorizonOSDsl
class ZramDeviceContext(
    private val name: String,
    private val size: String
) {
    var priority = 100
    var disksize: String? = null
    var memLimit: String? = null
    var backingDev: String? = null
    var writebackPercent = 10
    
    fun toConfig() = ZramDevice(
        name = name,
        size = size,
        priority = priority,
        disksize = disksize,
        memLimit = memLimit,
        backingDev = backingDev,
        writebackPercent = writebackPercent
    )
}

@HorizonOSDsl
class ZswapConfigContext {
    var enabled = false
    var compressor = CompressionAlgorithm.LZ4
    var maxPoolPercent = 20
    var acceptThresholdPercent = 90
    var zpool = ZpoolType.Z3FOLD
    var sameFilled = true
    var nonSameFilled = true
    var exclusiveLoads = false
    var shrinkerEnabled = true
    
    fun toConfig() = ZswapConfig(
        enabled = enabled,
        compressor = compressor,
        maxPoolPercent = maxPoolPercent,
        acceptThresholdPercent = acceptThresholdPercent,
        zpool = zpool,
        sameFilled = sameFilled,
        nonSameFilled = nonSameFilled,
        exclusiveLoads = exclusiveLoads,
        shrinkerEnabled = shrinkerEnabled
    )
}