package org.horizonos.config.dsl

import kotlinx.serialization.Serializable
import kotlin.time.Duration
import kotlin.time.Duration.Companion.seconds

/**
 * Boot and Kernel Configuration DSL for HorizonOS
 * 
 * Provides type-safe configuration for bootloader settings, kernel parameters,
 * module management, initramfs configuration, and boot themes.
 */

// ===== Boot Configuration =====

@Serializable
data class BootConfig(
    val bootloader: BootloaderConfig = BootloaderConfig(),
    val kernel: KernelConfig = KernelConfig(),
    val initramfs: InitramfsConfig = InitramfsConfig(),
    val plymouth: PlymouthConfig = PlymouthConfig(),
    val secureBoot: SecureBootConfig = SecureBootConfig(),
    val recovery: RecoveryConfig = RecoveryConfig()
)

@Serializable
data class BootloaderConfig(
    val type: BootloaderType = BootloaderType.SYSTEMD_BOOT,
    val timeout: Duration = 5.seconds,
    val defaultEntry: String? = null,
    val fallbackEntry: String? = null,
    val entries: List<BootEntry> = emptyList(),
    val theme: String? = null,
    val resolution: String? = null,
    val consoleMode: ConsoleMode = ConsoleMode.AUTO,
    val editor: Boolean = true,
    val autoEntries: Boolean = true,
    val autoFirmware: Boolean = true,
    val grubConfig: GrubConfig? = null
)

@Serializable
data class BootEntry(
    val title: String,
    val linux: String,
    val initrd: String? = null,
    val options: List<String> = emptyList(),
    val devicetree: String? = null,
    val architecture: String? = null,
    val version: String? = null,
    val machineId: String? = null,
    val sort: Int? = null
)

@Serializable
data class GrubConfig(
    val distributor: String = "HorizonOS",
    val defaultTimeout: Duration = 5.seconds,
    val theme: String? = null,
    val background: String? = null,
    val gfxMode: String = "auto",
    val gfxPayload: String = "keep",
    val recordFailCount: Boolean = true,
    val submenuShowOnce: Boolean = true,
    val disableRecovery: Boolean = false,
    val disableOsProber: Boolean = false,
    val customEntries: List<String> = emptyList()
)

@Serializable
data class KernelConfig(
    val parameters: List<KernelParameter> = emptyList(),
    val modules: KernelModuleConfig = KernelModuleConfig(),
    val version: String? = null,
    val variants: List<KernelVariant> = emptyList(),
    val compression: KernelCompression = KernelCompression.ZSTD,
    val debugging: KernelDebugging = KernelDebugging(),
    val security: KernelSecurity = KernelSecurity()
)

@Serializable
data class KernelParameter(
    val name: String,
    val value: String? = null,
    val condition: String? = null
)

@Serializable
data class KernelModuleConfig(
    val blacklist: List<String> = emptyList(),
    val load: List<String> = emptyList(),
    val options: Map<String, String> = emptyMap(),
    val autoLoad: Boolean = true,
    val compression: ModuleCompression = ModuleCompression.XZ
)

@Serializable
data class KernelVariant(
    val name: String,
    val version: String,
    val parameters: List<KernelParameter> = emptyList(),
    val initrd: String? = null,
    val description: String? = null
)

@Serializable
data class KernelDebugging(
    val enabled: Boolean = false,
    val debugLevel: Int = 0,
    val earlyPrintk: Boolean = false,
    val ignore_loglevel: Boolean = false,
    val printk_time: Boolean = false,
    val crashkernel: String? = null
)

@Serializable
data class KernelSecurity(
    val kaslr: Boolean = true,
    val smep: Boolean = true,
    val smap: Boolean = true,
    val pti: Boolean = true,
    val spectre_v2: SpectreV2Mitigation = SpectreV2Mitigation.AUTO,
    val meltdown: Boolean = true,
    val l1tf: L1TFMitigation = L1TFMitigation.FLUSH,
    val mds: MDSMitigation = MDSMitigation.FULL,
    val selinux: Boolean = false,
    val apparmor: Boolean = true
)

@Serializable
data class InitramfsConfig(
    val generator: InitramfsGenerator = InitramfsGenerator.MKINITCPIO,
    val modules: List<String> = emptyList(),
    val hooks: List<String> = emptyList(),
    val files: List<String> = emptyList(),
    val compression: InitramfsCompression = InitramfsCompression.ZSTD,
    val microcode: MicrocodeConfig = MicrocodeConfig(),
    val encryption: InitramfsEncryption? = null,
    val customScripts: List<String> = emptyList()
)

@Serializable
data class MicrocodeConfig(
    val enabled: Boolean = true,
    val intel: Boolean = true,
    val amd: Boolean = true,
    val early: Boolean = true
)

@Serializable
data class InitramfsEncryption(
    val method: EncryptionMethod = EncryptionMethod.LUKS2,
    val keyfile: String? = null,
    val keyslot: Int = 0,
    val tries: Int = 3,
    val timeout: Duration = 30.seconds
)

@Serializable
data class PlymouthConfig(
    val enabled: Boolean = true,
    val theme: String = "horizonos",
    val showDelay: Duration = 0.seconds,
    val deviceTimeout: Duration = 5.seconds,
    val modules: List<String> = listOf("drm"),
    val plugins: List<String> = emptyList(),
    val quietBoot: Boolean = true,
    val showSplash: Boolean = true
)

@Serializable
data class SecureBootConfig(
    val enabled: Boolean = false,
    val keys: SecureBootKeys? = null,
    val mokManager: Boolean = false,
    val signKernel: Boolean = false,
    val signModules: Boolean = false,
    val enrollKeys: Boolean = false
)

@Serializable
data class SecureBootKeys(
    val platform: String? = null,
    val keyExchange: String? = null,
    val signature: String? = null,
    val forbidden: String? = null
)

@Serializable
data class RecoveryConfig(
    val enabled: Boolean = true,
    val kernelParameters: List<KernelParameter> = emptyList(),
    val timeout: Duration = 30.seconds,
    val autoSelect: Boolean = false,
    val hideFromMenu: Boolean = false
)

// ===== Enums =====

@Serializable
enum class BootloaderType {
    GRUB,
    SYSTEMD_BOOT,
    REFIND,
    SYSLINUX,
    LIMINE,
    EFISTUB
}

@Serializable
enum class ConsoleMode {
    AUTO,
    MAX,
    KEEP,
    TEXT
}

@Serializable
enum class KernelCompression {
    GZIP,
    BZIP2,
    LZMA,
    XZ,
    LZO,
    LZ4,
    ZSTD
}

@Serializable
enum class ModuleCompression {
    NONE,
    GZIP,
    XZ,
    ZSTD
}

@Serializable
enum class SpectreV2Mitigation {
    OFF,
    ON,
    AUTO,
    RETPOLINE,
    LFENCE,
    EIBRS,
    EIBRS_RETPOLINE,
    EIBRS_LFENCE
}

@Serializable
enum class L1TFMitigation {
    OFF,
    FLUSH,
    FLUSH_NOSMT,
    FLUSH_NOWARN,
    FULL,
    FULL_FORCE
}

@Serializable
enum class MDSMitigation {
    OFF,
    FULL,
    FULL_NOSMT
}

@Serializable
enum class InitramfsGenerator {
    MKINITCPIO,
    DRACUT,
    INITRAMFS_TOOLS,
    CUSTOM
}

@Serializable
enum class InitramfsCompression {
    GZIP,
    BZIP2,
    LZMA,
    XZ,
    LZO,
    LZ4,
    ZSTD,
    CAT
}

@Serializable
enum class EncryptionMethod {
    LUKS1,
    LUKS2,
    PLAIN
}

// ===== DSL Builders =====

@HorizonOSDsl
class BootContext {
    private var bootloader = BootloaderConfig()
    private var kernel = KernelConfig()
    private var initramfs = InitramfsConfig()
    private var plymouth = PlymouthConfig()
    private var secureBoot = SecureBootConfig()
    private var recovery = RecoveryConfig()
    
    fun bootloader(block: BootloaderContext.() -> Unit) {
        bootloader = BootloaderContext().apply(block).toConfig()
    }
    
    fun kernel(block: KernelContext.() -> Unit) {
        kernel = KernelContext().apply(block).toConfig()
    }
    
    fun initramfs(block: InitramfsContext.() -> Unit) {
        initramfs = InitramfsContext().apply(block).toConfig()
    }
    
    fun plymouth(block: PlymouthContext.() -> Unit) {
        plymouth = PlymouthContext().apply(block).toConfig()
    }
    
    fun secureBoot(block: SecureBootContext.() -> Unit) {
        secureBoot = SecureBootContext().apply(block).toConfig()
    }
    
    fun recovery(block: RecoveryContext.() -> Unit) {
        recovery = RecoveryContext().apply(block).toConfig()
    }
    
    fun toConfig() = BootConfig(
        bootloader = bootloader,
        kernel = kernel,
        initramfs = initramfs,
        plymouth = plymouth,
        secureBoot = secureBoot,
        recovery = recovery
    )
}

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
    
    fun entry(title: String, block: BootEntryContext.() -> Unit) {
        val context = BootEntryContext().apply {
            this.title = title
            block()
        }
        entries.add(context.toEntry())
    }
    
    fun grub(block: GrubContext.() -> Unit) {
        grubConfig = GrubContext().apply(block).toConfig()
    }
    
    fun toConfig() = BootloaderConfig(
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

@HorizonOSDsl
class BootEntryContext {
    var title: String = ""
    var linux: String = ""
    var initrd: String? = null
    private val options = mutableListOf<String>()
    var devicetree: String? = null
    var architecture: String? = null
    var version: String? = null
    var machineId: String? = null
    var sort: Int? = null
    
    fun options(vararg opts: String) {
        options.addAll(opts)
    }
    
    fun toEntry() = BootEntry(
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
    
    fun toConfig() = GrubConfig(
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

@HorizonOSDsl
class KernelContext {
    private val parameters = mutableListOf<KernelParameter>()
    private var modules = KernelModuleConfig()
    var version: String? = null
    private val variants = mutableListOf<KernelVariant>()
    var compression: KernelCompression = KernelCompression.ZSTD
    private var debugging = KernelDebugging()
    private var security = KernelSecurity()
    
    fun parameter(name: String, value: String? = null, condition: String? = null) {
        parameters.add(KernelParameter(name, value, condition))
    }
    
    fun modules(block: KernelModuleContext.() -> Unit) {
        modules = KernelModuleContext().apply(block).toConfig()
    }
    
    fun variant(name: String, version: String, block: KernelVariantContext.() -> Unit = {}) {
        val context = KernelVariantContext().apply {
            this.name = name
            this.version = version
            block()
        }
        variants.add(context.toVariant())
    }
    
    fun debugging(block: KernelDebuggingContext.() -> Unit) {
        debugging = KernelDebuggingContext().apply(block).toConfig()
    }
    
    fun security(block: KernelSecurityContext.() -> Unit) {
        security = KernelSecurityContext().apply(block).toConfig()
    }
    
    // Common kernel parameters helpers
    fun quiet() = parameter("quiet")
    fun splash() = parameter("splash")
    fun nomodeset() = parameter("nomodeset")
    fun acpiOff() = parameter("acpi", "off")
    fun rootDevice(device: String) = parameter("root", device)
    fun rootfsType(type: String) = parameter("rootfstype", type)
    fun resume(device: String) = parameter("resume", device)
    fun cryptDevice(uuid: String, name: String) = parameter("cryptdevice", "$uuid:$name")
    fun nvidia(mode: String = "drm.modeset=1") = parameter("nvidia-drm.modeset", "1")
    
    fun toConfig() = KernelConfig(
        parameters = parameters,
        modules = modules,
        version = version,
        variants = variants,
        compression = compression,
        debugging = debugging,
        security = security
    )
}

@HorizonOSDsl
class KernelModuleContext {
    private val blacklist = mutableListOf<String>()
    private val load = mutableListOf<String>()
    private val options = mutableMapOf<String, String>()
    var autoLoad: Boolean = true
    var compression: ModuleCompression = ModuleCompression.XZ
    
    fun blacklist(vararg modules: String) {
        blacklist.addAll(modules)
    }
    
    fun load(vararg modules: String) {
        load.addAll(modules)
    }
    
    fun option(module: String, option: String) {
        options[module] = option
    }
    
    fun toConfig() = KernelModuleConfig(
        blacklist = blacklist,
        load = load,
        options = options,
        autoLoad = autoLoad,
        compression = compression
    )
}

@HorizonOSDsl
class KernelVariantContext {
    var name: String = ""
    var version: String = ""
    private val parameters = mutableListOf<KernelParameter>()
    var initrd: String? = null
    var description: String? = null
    
    fun parameter(name: String, value: String? = null, condition: String? = null) {
        parameters.add(KernelParameter(name, value, condition))
    }
    
    fun toVariant() = KernelVariant(
        name = name,
        version = version,
        parameters = parameters,
        initrd = initrd,
        description = description
    )
}

@HorizonOSDsl
class KernelDebuggingContext {
    var enabled: Boolean = false
    var debugLevel: Int = 0
    var earlyPrintk: Boolean = false
    var ignore_loglevel: Boolean = false
    var printk_time: Boolean = false
    var crashkernel: String? = null
    
    fun toConfig() = KernelDebugging(
        enabled = enabled,
        debugLevel = debugLevel,
        earlyPrintk = earlyPrintk,
        ignore_loglevel = ignore_loglevel,
        printk_time = printk_time,
        crashkernel = crashkernel
    )
}

@HorizonOSDsl
class KernelSecurityContext {
    var kaslr: Boolean = true
    var smep: Boolean = true
    var smap: Boolean = true
    var pti: Boolean = true
    var spectre_v2: SpectreV2Mitigation = SpectreV2Mitigation.AUTO
    var meltdown: Boolean = true
    var l1tf: L1TFMitigation = L1TFMitigation.FLUSH
    var mds: MDSMitigation = MDSMitigation.FULL
    var selinux: Boolean = false
    var apparmor: Boolean = true
    
    fun toConfig() = KernelSecurity(
        kaslr = kaslr,
        smep = smep,
        smap = smap,
        pti = pti,
        spectre_v2 = spectre_v2,
        meltdown = meltdown,
        l1tf = l1tf,
        mds = mds,
        selinux = selinux,
        apparmor = apparmor
    )
}

@HorizonOSDsl
class InitramfsContext {
    var generator: InitramfsGenerator = InitramfsGenerator.MKINITCPIO
    private val modules = mutableListOf<String>()
    private val hooks = mutableListOf<String>()
    private val files = mutableListOf<String>()
    var compression: InitramfsCompression = InitramfsCompression.ZSTD
    private var microcode = MicrocodeConfig()
    private var encryption: InitramfsEncryption? = null
    private val customScripts = mutableListOf<String>()
    
    fun modules(vararg moduleList: String) {
        modules.addAll(moduleList)
    }
    
    fun hooks(vararg hookList: String) {
        hooks.addAll(hookList)
    }
    
    fun files(vararg fileList: String) {
        files.addAll(fileList)
    }
    
    fun microcode(block: MicrocodeContext.() -> Unit) {
        microcode = MicrocodeContext().apply(block).toConfig()
    }
    
    fun encryption(block: InitramfsEncryptionContext.() -> Unit) {
        encryption = InitramfsEncryptionContext().apply(block).toConfig()
    }
    
    fun customScript(script: String) {
        customScripts.add(script)
    }
    
    fun toConfig() = InitramfsConfig(
        generator = generator,
        modules = modules,
        hooks = hooks,
        files = files,
        compression = compression,
        microcode = microcode,
        encryption = encryption,
        customScripts = customScripts
    )
}

@HorizonOSDsl
class MicrocodeContext {
    var enabled: Boolean = true
    var intel: Boolean = true
    var amd: Boolean = true
    var early: Boolean = true
    
    fun toConfig() = MicrocodeConfig(
        enabled = enabled,
        intel = intel,
        amd = amd,
        early = early
    )
}

@HorizonOSDsl
class InitramfsEncryptionContext {
    var method: EncryptionMethod = EncryptionMethod.LUKS2
    var keyfile: String? = null
    var keyslot: Int = 0
    var tries: Int = 3
    var timeout: Duration = 30.seconds
    
    fun toConfig() = InitramfsEncryption(
        method = method,
        keyfile = keyfile,
        keyslot = keyslot,
        tries = tries,
        timeout = timeout
    )
}

@HorizonOSDsl
class PlymouthContext {
    var enabled: Boolean = true
    var theme: String = "horizonos"
    var showDelay: Duration = 0.seconds
    var deviceTimeout: Duration = 5.seconds
    private val modules = mutableListOf("drm")
    private val plugins = mutableListOf<String>()
    var quietBoot: Boolean = true
    var showSplash: Boolean = true
    
    fun modules(vararg moduleList: String) {
        modules.clear()
        modules.addAll(moduleList)
    }
    
    fun plugins(vararg pluginList: String) {
        plugins.addAll(pluginList)
    }
    
    fun toConfig() = PlymouthConfig(
        enabled = enabled,
        theme = theme,
        showDelay = showDelay,
        deviceTimeout = deviceTimeout,
        modules = modules,
        plugins = plugins,
        quietBoot = quietBoot,
        showSplash = showSplash
    )
}

@HorizonOSDsl
class SecureBootContext {
    var enabled: Boolean = false
    private var keys: SecureBootKeys? = null
    var mokManager: Boolean = false
    var signKernel: Boolean = false
    var signModules: Boolean = false
    var enrollKeys: Boolean = false
    
    fun keys(block: SecureBootKeysContext.() -> Unit) {
        keys = SecureBootKeysContext().apply(block).toKeys()
    }
    
    fun toConfig() = SecureBootConfig(
        enabled = enabled,
        keys = keys,
        mokManager = mokManager,
        signKernel = signKernel,
        signModules = signModules,
        enrollKeys = enrollKeys
    )
}

@HorizonOSDsl
class SecureBootKeysContext {
    var platform: String? = null
    var keyExchange: String? = null
    var signature: String? = null
    var forbidden: String? = null
    
    fun toKeys() = SecureBootKeys(
        platform = platform,
        keyExchange = keyExchange,
        signature = signature,
        forbidden = forbidden
    )
}

@HorizonOSDsl
class RecoveryContext {
    var enabled: Boolean = true
    private val kernelParameters = mutableListOf<KernelParameter>()
    var timeout: Duration = 30.seconds
    var autoSelect: Boolean = false
    var hideFromMenu: Boolean = false
    
    fun parameter(name: String, value: String? = null, condition: String? = null) {
        kernelParameters.add(KernelParameter(name, value, condition))
    }
    
    fun toConfig() = RecoveryConfig(
        enabled = enabled,
        kernelParameters = kernelParameters,
        timeout = timeout,
        autoSelect = autoSelect,
        hideFromMenu = hideFromMenu
    )
}

// ===== Extension Functions =====

fun CompiledConfig.hasBoot(): Boolean = boot != null

fun CompiledConfig.getBootEntry(title: String): BootEntry? = 
    boot?.bootloader?.entries?.find { it.title == title }

fun CompiledConfig.getKernelVariant(name: String): KernelVariant? = 
    boot?.kernel?.variants?.find { it.name == name }

fun CompiledConfig.getKernelParameter(name: String): KernelParameter? = 
    boot?.kernel?.parameters?.find { it.name == name }