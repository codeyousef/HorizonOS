package org.horizonos.config.dsl.storage.btrfs

import org.horizonos.config.dsl.HorizonOSDsl
import kotlin.time.Duration
import kotlin.time.Duration.Companion.hours

// ===== Btrfs DSL Builders =====

@HorizonOSDsl
class BtrfsContext {
    var enabled = false
    private val subvolumes = mutableListOf<BtrfsSubvolume>()
    private var snapshots = SnapshotConfig()
    private var quotas = QuotaConfig()
    private var scrub = ScrubConfig()
    private var balance = BalanceConfig()
    private var compression = CompressionConfig()
    private var dedupe = DedupeConfig()
    
    fun subvolume(name: String, path: String, block: BtrfsSubvolumeContext.() -> Unit = {}) {
        val context = BtrfsSubvolumeContext(name, path).apply(block)
        subvolumes.add(context.toConfig())
    }
    
    fun snapshots(block: SnapshotConfigContext.() -> Unit) {
        snapshots = SnapshotConfigContext().apply(block).toConfig()
    }
    
    fun quotas(block: QuotaConfigContext.() -> Unit) {
        quotas = QuotaConfigContext().apply(block).toConfig()
    }
    
    fun scrub(block: ScrubConfigContext.() -> Unit) {
        scrub = ScrubConfigContext().apply(block).toConfig()
    }
    
    fun balance(block: BalanceConfigContext.() -> Unit) {
        balance = BalanceConfigContext().apply(block).toConfig()
    }
    
    fun compression(block: CompressionConfigContext.() -> Unit) {
        compression = CompressionConfigContext().apply(block).toConfig()
    }
    
    fun dedupe(block: DedupeConfigContext.() -> Unit) {
        dedupe = DedupeConfigContext().apply(block).toConfig()
    }
    
    fun toConfig() = BtrfsConfig(
        enabled = enabled,
        subvolumes = subvolumes,
        snapshots = snapshots,
        quotas = quotas,
        scrub = scrub,
        balance = balance,
        compression = compression,
        dedupe = dedupe
    )
}

@HorizonOSDsl
class BtrfsSubvolumeContext(
    private val name: String,
    private val path: String
) {
    val mountOptions = mutableListOf<String>()
    var defaultSubvolume = false
    private var snapshots = SubvolumeSnapshots()
    private var quota: SubvolumeQuota? = null
    var compression: CompressionType? = null
    var copyOnWrite = true
    var autoDefrag = false
    
    fun mountOption(opt: String) {
        mountOptions.add(opt)
    }
    
    fun snapshots(block: SubvolumeSnapshotsContext.() -> Unit) {
        snapshots = SubvolumeSnapshotsContext().apply(block).toConfig()
    }
    
    fun quota(referenced: String? = null, exclusive: String? = null, compressed: String? = null, sizeLimit: String? = null) {
        quota = SubvolumeQuota(referenced, exclusive, compressed, sizeLimit)
    }
    
    fun toConfig() = BtrfsSubvolume(
        name = name,
        path = path,
        mountOptions = mountOptions,
        defaultSubvolume = defaultSubvolume,
        snapshots = snapshots,
        quota = quota,
        compression = compression,
        copyOnWrite = copyOnWrite,
        autoDefrag = autoDefrag
    )
}

@HorizonOSDsl
class SubvolumeSnapshotsContext {
    var enabled = true
    var location = "/.snapshots"
    private var schedule = SnapshotSchedule()
    private var retention = SnapshotRetention()
    var naming = "{name}-{timestamp}"
    
    fun schedule(block: SnapshotScheduleContext.() -> Unit) {
        schedule = SnapshotScheduleContext().apply(block).toConfig()
    }
    
    fun retention(block: SnapshotRetentionContext.() -> Unit) {
        retention = SnapshotRetentionContext().apply(block).toConfig()
    }
    
    fun toConfig() = SubvolumeSnapshots(
        enabled = enabled,
        location = location,
        schedule = schedule,
        retention = retention,
        naming = naming
    )
}

@HorizonOSDsl
class SnapshotScheduleContext {
    var hourly = 0
    var daily = 7
    var weekly = 4
    var monthly = 12
    var yearly = 0
    
    fun toConfig() = SnapshotSchedule(
        hourly = hourly,
        daily = daily,
        weekly = weekly,
        monthly = monthly,
        yearly = yearly
    )
}

@HorizonOSDsl
class SnapshotRetentionContext {
    var hourly = 24
    var daily = 7
    var weekly = 4
    var monthly = 12
    var yearly = 2
    var minimumFreeSpace = "20%"
    
    fun toConfig() = SnapshotRetention(
        hourly = hourly,
        daily = daily,
        weekly = weekly,
        monthly = monthly,
        yearly = yearly,
        minimumFreeSpace = minimumFreeSpace
    )
}

@HorizonOSDsl
class SnapshotConfigContext {
    var enabled = true
    var tool = SnapshotTool.SNAPPER
    var bootSnapshots = true
    var prePostSnapshots = true
    var timeline = true
    var cleanupAlgorithm = CleanupAlgorithm.NUMBER
    
    fun toConfig() = SnapshotConfig(
        enabled = enabled,
        tool = tool,
        bootSnapshots = bootSnapshots,
        prePostSnapshots = prePostSnapshots,
        timeline = timeline,
        cleanupAlgorithm = cleanupAlgorithm
    )
}

@HorizonOSDsl
class QuotaConfigContext {
    var enabled = false
    private val groups = mutableListOf<QuotaGroup>()
    var rescan: Duration = 24.hours
    
    fun group(id: String, limit: String, block: QuotaGroupContext.() -> Unit = {}) {
        val context = QuotaGroupContext(id, limit).apply(block)
        groups.add(context.toConfig())
    }
    
    fun toConfig() = QuotaConfig(
        enabled = enabled,
        groups = groups,
        rescan = rescan
    )
}

@HorizonOSDsl
class QuotaGroupContext(
    private val id: String,
    private val limit: String
) {
    private val subvolumes = mutableListOf<String>()
    
    fun subvolume(name: String) {
        subvolumes.add(name)
    }
    
    fun toConfig() = QuotaGroup(
        id = id,
        limit = limit,
        subvolumes = subvolumes
    )
}

@HorizonOSDsl
class ScrubConfigContext {
    var enabled = true
    var schedule = "monthly"
    var priority = IOPriority.IDLE
    var readOnly = false
    var dataOnly = false
    
    fun toConfig() = ScrubConfig(
        enabled = enabled,
        schedule = schedule,
        priority = priority,
        readOnly = readOnly,
        dataOnly = dataOnly
    )
}

@HorizonOSDsl
class BalanceConfigContext {
    var enabled = false
    var schedule = "quarterly"
    private var filters = BalanceFilters()
    var pauseOnBattery = true
    var resumeAfterBoot = true
    
    fun filters(block: BalanceFiltersContext.() -> Unit) {
        filters = BalanceFiltersContext().apply(block).toConfig()
    }
    
    fun toConfig() = BalanceConfig(
        enabled = enabled,
        schedule = schedule,
        filters = filters,
        pauseOnBattery = pauseOnBattery,
        resumeAfterBoot = resumeAfterBoot
    )
}

@HorizonOSDsl
class BalanceFiltersContext {
    var dataUsage: String? = null
    var metadataUsage: String? = null
    var systemUsage: String? = null
    private var convert: ProfileConversion? = null
    
    fun convert(data: String? = null, metadata: String? = null, system: String? = null) {
        convert = ProfileConversion(data, metadata, system)
    }
    
    fun toConfig() = BalanceFilters(
        dataUsage = dataUsage,
        metadataUsage = metadataUsage,
        systemUsage = systemUsage,
        convert = convert
    )
}

@HorizonOSDsl
class CompressionConfigContext {
    var algorithm = CompressionType.ZSTD
    var level: Int? = null
    var forceCompress = false
    var threads: Int? = null
    
    fun toConfig() = CompressionConfig(
        algorithm = algorithm,
        level = level,
        forceCompress = forceCompress,
        threads = threads
    )
}

@HorizonOSDsl
class DedupeConfigContext {
    var enabled = false
    var backend = DedupeBackend.DUPEREMOVE
    var hashAlgorithm = "sha256"
    var blockSize = "128K"
    var schedule = "weekly"
    
    fun toConfig() = DedupeConfig(
        enabled = enabled,
        backend = backend,
        hashAlgorithm = hashAlgorithm,
        blockSize = blockSize,
        schedule = schedule
    )
}