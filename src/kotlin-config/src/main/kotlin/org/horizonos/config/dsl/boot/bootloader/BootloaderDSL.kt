package org.horizonos.config.dsl.boot.bootloader

import org.horizonos.config.dsl.HorizonOSDsl
import kotlin.time.Duration
import kotlin.time.Duration.Companion.seconds

// ===== Bootloader DSL Builders =====

@HorizonOSDsl
class BootloaderContext {
    var type: BootloaderType = BootloaderType.SYSTEMD_BOOT
    var timeout: Duration = 5.seconds
    var defaultEntry: String? = null
    var fallbackEntry: String? = null
    private val entries = mutableListOf<BootEntry>()
    var theme: String? = null
    var resolution: String? = null
    var consoleMode: ConsoleMode = ConsoleMode.AUTO
    var editor: Boolean = true
    var autoEntries: Boolean = true
    var autoFirmware: Boolean = true
    private var grubConfig: GrubConfig? = null
    
    fun entry(title: String, linux: String, block: BootEntryContext.() -> Unit = {}) {
        val context = BootEntryContext(title, linux).apply(block)
        entries.add(context.toEntry())
    }
    
    fun grub(block: GrubContext.() -> Unit) {
        grubConfig = GrubContext().apply(block).toConfig()
    }
    
    fun toConfig(): BootloaderConfig {
        return BootloaderConfig(
            type = type,
            timeout = timeout,
            defaultEntry = defaultEntry,
            fallbackEntry = fallbackEntry,
            entries = entries,
            theme = theme,
            resolution = resolution,
            consoleMode = consoleMode,
            editor = editor,
            autoEntries = autoEntries,
            autoFirmware = autoFirmware,
            grubConfig = grubConfig
        )
    }
}

@HorizonOSDsl
class BootEntryContext(
    private val title: String,
    private val linux: String
) {
    var initrd: String? = null
    private val options = mutableListOf<String>()
    var devicetree: String? = null
    var architecture: String? = null
    var version: String? = null
    var machineId: String? = null
    var sort: Int? = null
    
    fun option(opt: String) {
        options.add(opt)
    }
    
    fun options(vararg opts: String) {
        options.addAll(opts)
    }
    
    fun toEntry(): BootEntry {
        return BootEntry(
            title = title,
            linux = linux,
            initrd = initrd,
            options = options,
            devicetree = devicetree,
            architecture = architecture,
            version = version,
            machineId = machineId,
            sort = sort
        )
    }
}

@HorizonOSDsl
class GrubContext {
    var distributor: String = "HorizonOS"
    var defaultTimeout: Duration = 5.seconds
    var theme: String? = null
    var background: String? = null
    var gfxMode: String = "auto"
    var gfxPayload: String = "keep"
    var recordFailCount: Boolean = true
    var submenuShowOnce: Boolean = true
    var disableRecovery: Boolean = false
    var disableOsProber: Boolean = false
    private val customEntries = mutableListOf<String>()
    
    fun customEntry(entry: String) {
        customEntries.add(entry)
    }
    
    fun toConfig(): GrubConfig {
        return GrubConfig(
            distributor = distributor,
            defaultTimeout = defaultTimeout,
            theme = theme,
            background = background,
            gfxMode = gfxMode,
            gfxPayload = gfxPayload,
            recordFailCount = recordFailCount,
            submenuShowOnce = submenuShowOnce,
            disableRecovery = disableRecovery,
            disableOsProber = disableOsProber,
            customEntries = customEntries
        )
    }
}