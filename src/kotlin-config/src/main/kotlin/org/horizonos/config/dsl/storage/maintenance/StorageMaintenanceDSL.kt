package org.horizonos.config.dsl.storage.maintenance

import org.horizonos.config.dsl.HorizonOSDsl
import kotlin.time.Duration
import kotlin.time.Duration.Companion.days
import kotlin.time.Duration.Companion.hours

// ===== Storage Maintenance DSL Builders =====

@HorizonOSDsl
class StorageMaintenanceContext {
    var enabled = true
    private var trim = TrimConfiguration()
    private var defragmentation = DefragmentationConfig()
    private var cleanup = CleanupConfig()
    private var verification = VerificationConfig()
    private var optimization = OptimizationConfig()
    
    fun trim(block: TrimContext.() -> Unit) {
        trim = TrimContext().apply(block).toConfig()
    }
    
    fun defragmentation(block: DefragmentationContext.() -> Unit) {
        defragmentation = DefragmentationContext().apply(block).toConfig()
    }
    
    fun cleanup(block: CleanupContext.() -> Unit) {
        cleanup = CleanupContext().apply(block).toConfig()
    }
    
    fun verification(block: VerificationContext.() -> Unit) {
        verification = VerificationContext().apply(block).toConfig()
    }
    
    fun optimization(block: OptimizationContext.() -> Unit) {
        optimization = OptimizationContext().apply(block).toConfig()
    }
    
    fun toConfig() = StorageMaintenanceConfig(
        enabled = enabled,
        trim = trim,
        defragmentation = defragmentation,
        cleanup = cleanup,
        verification = verification,
        optimization = optimization
    )
}

@HorizonOSDsl
class TrimContext {
    var enabled = true
    var schedule = "weekly"
    private val devices = mutableListOf<TrimDevice>()
    private var fstrim = FstrimConfig()
    
    fun device(device: String, block: TrimDeviceContext.() -> Unit = {}) {
        val context = TrimDeviceContext(device).apply(block)
        devices.add(context.toConfig())
    }
    
    fun fstrim(block: FstrimContext.() -> Unit) {
        fstrim = FstrimContext().apply(block).toConfig()
    }
    
    fun toConfig() = TrimConfiguration(
        enabled = enabled,
        schedule = schedule,
        devices = devices,
        fstrim = fstrim
    )
}

@HorizonOSDsl
class TrimDeviceContext(private val device: String) {
    var filesystem: String? = null
    val options = mutableListOf<String>()
    var minFreeSpace = "10%"
    
    fun option(opt: String) {
        options.add(opt)
    }
    
    fun toConfig() = TrimDevice(
        device = device,
        filesystem = filesystem,
        options = options,
        minFreeSpace = minFreeSpace
    )
}

@HorizonOSDsl
class FstrimContext {
    var verbose = true
    var allFilesystems = true
    var minExtentSize: String? = null
    val excludeFilesystems = mutableListOf<String>()
    
    fun exclude(fs: String) {
        excludeFilesystems.add(fs)
    }
    
    fun toConfig() = FstrimConfig(
        verbose = verbose,
        allFilesystems = allFilesystems,
        minExtentSize = minExtentSize,
        excludeFilesystems = excludeFilesystems
    )
}

@HorizonOSDsl
class DefragmentationContext {
    var enabled = false
    private val filesystems = mutableListOf<DefragTask>()
    var schedule = "monthly"
    private var policy = DefragPolicy()
    
    fun filesystem(fs: String, type: FilesystemType, block: DefragTaskContext.() -> Unit = {}) {
        val context = DefragTaskContext(fs, type).apply(block)
        filesystems.add(context.toConfig())
    }
    
    fun policy(block: DefragPolicyContext.() -> Unit) {
        policy = DefragPolicyContext().apply(block).toConfig()
    }
    
    fun toConfig() = DefragmentationConfig(
        enabled = enabled,
        filesystems = filesystems,
        schedule = schedule,
        policy = policy
    )
}

@HorizonOSDsl
class DefragTaskContext(
    private val filesystem: String,
    private val type: FilesystemType
) {
    private var options = DefragOptions()
    val exclude = mutableListOf<String>()
    
    fun options(block: DefragOptionsContext.() -> Unit) {
        options = DefragOptionsContext().apply(block).toConfig()
    }
    
    fun exclude(path: String) {
        exclude.add(path)
    }
    
    fun toConfig() = DefragTask(
        filesystem = filesystem,
        type = type,
        options = options,
        exclude = exclude
    )
}

@HorizonOSDsl
class DefragOptionsContext {
    var maxRuntime: Duration = 2.hours
    var threshold = 5
    var compress = false
    var flush = true
    
    fun toConfig() = DefragOptions(
        maxRuntime = maxRuntime,
        threshold = threshold,
        compress = compress,
        flush = flush
    )
}

@HorizonOSDsl
class DefragPolicyContext {
    var skipIfBattery = true
    var ionicePriority = 7
    val cpuAffinity = mutableListOf<Int>()
    var memoryLimit: String? = null
    
    fun cpuCore(core: Int) {
        cpuAffinity.add(core)
    }
    
    fun toConfig() = DefragPolicy(
        skipIfBattery = skipIfBattery,
        ionicePriority = ionicePriority,
        cpuAffinity = cpuAffinity,
        memoryLimit = memoryLimit
    )
}

@HorizonOSDsl
class CleanupContext {
    var enabled = true
    private val tasks = mutableListOf<CleanupTask>()
    private var orphaned = OrphanedCleanup()
    private var cache = CacheCleanup()
    private var logs = LogCleanup()
    
    fun task(name: String, block: CleanupTaskContext.() -> Unit) {
        val context = CleanupTaskContext(name).apply(block)
        tasks.add(context.toConfig())
    }
    
    fun orphaned(block: OrphanedCleanupContext.() -> Unit) {
        orphaned = OrphanedCleanupContext().apply(block).toConfig()
    }
    
    fun cache(block: CacheCleanupContext.() -> Unit) {
        cache = CacheCleanupContext().apply(block).toConfig()
    }
    
    fun logs(block: LogCleanupContext.() -> Unit) {
        logs = LogCleanupContext().apply(block).toConfig()
    }
    
    fun toConfig() = CleanupConfig(
        enabled = enabled,
        tasks = tasks,
        orphaned = orphaned,
        cache = cache,
        logs = logs
    )
}

@HorizonOSDsl
class CleanupTaskContext(private val name: String) {
    val paths = mutableListOf<String>()
    val patterns = mutableListOf<String>()
    var age: Duration? = null
    var size: String? = null
    var action = CleanupAction.DELETE
    
    fun path(p: String) {
        paths.add(p)
    }
    
    fun pattern(p: String) {
        patterns.add(p)
    }
    
    fun toConfig() = CleanupTask(
        name = name,
        paths = paths,
        patterns = patterns,
        age = age,
        size = size,
        action = action
    )
}

@HorizonOSDsl
class OrphanedCleanupContext {
    var enabled = true
    var packages = true
    var kernels = true
    var tempFiles = true
    var schedule = "weekly"
    
    fun toConfig() = OrphanedCleanup(
        enabled = enabled,
        packages = packages,
        kernels = kernels,
        tempFiles = tempFiles,
        schedule = schedule
    )
}

@HorizonOSDsl
class CacheCleanupContext {
    var enabled = true
    var systemCache = true
    var userCache = true
    var packageCache = true
    var maxAge: Duration = 30.days
    var maxSize: String? = null
    
    fun toConfig() = CacheCleanup(
        enabled = enabled,
        systemCache = systemCache,
        userCache = userCache,
        packageCache = packageCache,
        maxAge = maxAge,
        maxSize = maxSize
    )
}

@HorizonOSDsl
class LogCleanupContext {
    var enabled = true
    private var rotation = LogRotation()
    var compression = true
    var maxAge: Duration = 90.days
    var maxSize = "1G"
    
    fun rotation(block: LogRotationContext.() -> Unit) {
        rotation = LogRotationContext().apply(block).toConfig()
    }
    
    fun toConfig() = LogCleanup(
        enabled = enabled,
        rotation = rotation,
        compression = compression,
        maxAge = maxAge,
        maxSize = maxSize
    )
}

@HorizonOSDsl
class LogRotationContext {
    var enabled = true
    var frequency = "daily"
    var keep = 7
    var compress = true
    var delayCompress = true
    
    fun toConfig() = LogRotation(
        enabled = enabled,
        frequency = frequency,
        keep = keep,
        compress = compress,
        delayCompress = delayCompress
    )
}

@HorizonOSDsl
class VerificationContext {
    var enabled = true
    private var filesystem = FilesystemVerification()
    private var data = DataVerification()
    private var integrity = IntegrityVerification()
    
    fun filesystem(block: FilesystemVerificationContext.() -> Unit) {
        filesystem = FilesystemVerificationContext().apply(block).toConfig()
    }
    
    fun data(block: DataVerificationContext.() -> Unit) {
        data = DataVerificationContext().apply(block).toConfig()
    }
    
    fun integrity(block: IntegrityVerificationContext.() -> Unit) {
        integrity = IntegrityVerificationContext().apply(block).toConfig()
    }
    
    fun toConfig() = VerificationConfig(
        enabled = enabled,
        filesystem = filesystem,
        data = data,
        integrity = integrity
    )
}

@HorizonOSDsl
class FilesystemVerificationContext {
    var enabled = true
    var schedule = "monthly"
    private val checks = mutableListOf<FsckCheck>()
    var autoRepair = false
    
    fun check(filesystem: String, type: FilesystemType, block: FsckCheckContext.() -> Unit = {}) {
        val context = FsckCheckContext(filesystem, type).apply(block)
        checks.add(context.toConfig())
    }
    
    fun toConfig() = FilesystemVerification(
        enabled = enabled,
        schedule = schedule,
        checks = checks,
        autoRepair = autoRepair
    )
}

@HorizonOSDsl
class FsckCheckContext(
    private val filesystem: String,
    private val type: FilesystemType
) {
    val options = mutableListOf<String>()
    var forceCheck = false
    
    fun option(opt: String) {
        options.add(opt)
    }
    
    fun toConfig() = FsckCheck(
        filesystem = filesystem,
        type = type,
        options = options,
        forceCheck = forceCheck
    )
}

@HorizonOSDsl
class DataVerificationContext {
    var enabled = true
    private var checksums = ChecksumVerification()
    private var backups = BackupVerification()
    
    fun checksums(block: ChecksumVerificationContext.() -> Unit) {
        checksums = ChecksumVerificationContext().apply(block).toConfig()
    }
    
    fun backups(block: BackupVerificationContext.() -> Unit) {
        backups = BackupVerificationContext().apply(block).toConfig()
    }
    
    fun toConfig() = DataVerification(
        enabled = enabled,
        checksums = checksums,
        backups = backups
    )
}

@HorizonOSDsl
class ChecksumVerificationContext {
    var enabled = true
    var algorithm = ChecksumAlgorithm.SHA256
    val paths = mutableListOf<String>()
    var schedule = "weekly"
    
    fun path(p: String) {
        paths.add(p)
    }
    
    fun toConfig() = ChecksumVerification(
        enabled = enabled,
        algorithm = algorithm,
        paths = paths,
        schedule = schedule
    )
}

@HorizonOSDsl
class BackupVerificationContext {
    var enabled = true
    var testRestore = false
    var compareChecksums = true
    var sampleSize = 10
    
    fun toConfig() = BackupVerification(
        enabled = enabled,
        testRestore = testRestore,
        compareChecksums = compareChecksums,
        sampleSize = sampleSize
    )
}

@HorizonOSDsl
class IntegrityVerificationContext {
    var enabled = true
    private val methods = mutableListOf<IntegrityMethod>()
    var schedule = "weekly"
    
    fun method(name: String, type: IntegrityType, block: IntegrityMethodContext.() -> Unit) {
        val context = IntegrityMethodContext(name, type).apply(block)
        methods.add(context.toConfig())
    }
    
    fun toConfig() = IntegrityVerification(
        enabled = enabled,
        methods = methods,
        schedule = schedule
    )
}

@HorizonOSDsl
class IntegrityMethodContext(
    private val name: String,
    private val type: IntegrityType
) {
    val paths = mutableListOf<String>()
    val options = mutableMapOf<String, String>()
    
    fun path(p: String) {
        paths.add(p)
    }
    
    fun option(key: String, value: String) {
        options[key] = value
    }
    
    fun toConfig() = IntegrityMethod(
        name = name,
        type = type,
        paths = paths,
        options = options
    )
}

@HorizonOSDsl
class OptimizationContext {
    var enabled = true
    private var database = DatabaseOptimization()
    private var index = IndexOptimization()
    private var allocation = AllocationOptimization()
    
    fun database(block: DatabaseOptimizationContext.() -> Unit) {
        database = DatabaseOptimizationContext().apply(block).toConfig()
    }
    
    fun index(block: IndexOptimizationContext.() -> Unit) {
        index = IndexOptimizationContext().apply(block).toConfig()
    }
    
    fun allocation(block: AllocationOptimizationContext.() -> Unit) {
        allocation = AllocationOptimizationContext().apply(block).toConfig()
    }
    
    fun toConfig() = OptimizationConfig(
        enabled = enabled,
        database = database,
        index = index,
        allocation = allocation
    )
}

@HorizonOSDsl
class DatabaseOptimizationContext {
    var enabled = true
    var vacuum = true
    var analyze = true
    var reindex = false
    val databases = mutableListOf<String>()
    
    fun database(name: String) {
        databases.add(name)
    }
    
    fun toConfig() = DatabaseOptimization(
        enabled = enabled,
        vacuum = vacuum,
        analyze = analyze,
        reindex = reindex,
        databases = databases
    )
}

@HorizonOSDsl
class IndexOptimizationContext {
    var enabled = true
    var updatedb = true
    var mandb = true
    var fontCache = true
    var schedule = "daily"
    
    fun toConfig() = IndexOptimization(
        enabled = enabled,
        updatedb = updatedb,
        mandb = mandb,
        fontCache = fontCache,
        schedule = schedule
    )
}

@HorizonOSDsl
class AllocationOptimizationContext {
    var enabled = false
    private var preallocation = PreallocationConfig()
    private var alignment = AlignmentConfig()
    
    fun preallocation(block: PreallocationContext.() -> Unit) {
        preallocation = PreallocationContext().apply(block).toConfig()
    }
    
    fun alignment(block: AlignmentContext.() -> Unit) {
        alignment = AlignmentContext().apply(block).toConfig()
    }
    
    fun toConfig() = AllocationOptimization(
        enabled = enabled,
        preallocation = preallocation,
        alignment = alignment
    )
}

@HorizonOSDsl
class PreallocationContext {
    var enabled = false
    var method = PreallocationMethod.FALLOCATE
    private val patterns = mutableListOf<PreallocationPattern>()
    
    fun pattern(path: String, size: String, count: Int = 1) {
        patterns.add(PreallocationPattern(path, size, count))
    }
    
    fun toConfig() = PreallocationConfig(
        enabled = enabled,
        method = method,
        patterns = patterns
    )
}

@HorizonOSDsl
class AlignmentContext {
    var enabled = true
    var blockSize = 4096
    var stripeSize: Int? = null
    
    fun toConfig() = AlignmentConfig(
        enabled = enabled,
        blockSize = blockSize,
        stripeSize = stripeSize
    )
}