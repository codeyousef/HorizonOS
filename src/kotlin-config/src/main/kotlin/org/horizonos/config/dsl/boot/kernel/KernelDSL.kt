package org.horizonos.config.dsl.boot.kernel

import org.horizonos.config.dsl.HorizonOSDsl

// ===== Kernel DSL Builders =====

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
    
    fun variant(name: String, version: String, path: String, block: KernelVariantContext.() -> Unit = {}) {
        variants.add(KernelVariantContext(name, version, path).apply(block).toVariant())
    }
    
    fun debugging(block: KernelDebuggingContext.() -> Unit) {
        debugging = KernelDebuggingContext().apply(block).toConfig()
    }
    
    fun security(block: KernelSecurityContext.() -> Unit) {
        security = KernelSecurityContext().apply(block).toConfig()
    }
    
    // Convenience methods for common kernel parameters
    fun quiet() = parameter("quiet")
    fun splash() = parameter("splash")
    fun nomodeset() = parameter("nomodeset")
    fun acpiOff() = parameter("acpi", "off")
    fun rootDevice(device: String) = parameter("root", device)
    fun rootfsType(type: String) = parameter("rootfstype", type)
    fun resume(device: String) = parameter("resume", device)
    fun cryptDevice(device: String) = parameter("cryptdevice", device)
    fun nvidia() = parameter("nvidia-drm.modeset", "1")
    fun intel() = parameter("i915.modeset", "1")
    fun amd() = parameter("amdgpu.modeset", "1")
    fun crashkernel(size: String) = parameter("crashkernel", size)
    
    // Security mitigation convenience methods
    fun spectre_v2(mitigation: SpectreV2Mitigation) {
        security { spectreV2 = mitigation }
    }
    
    fun meltdown(enabled: Boolean) {
        security { pti = enabled }
    }
    
    fun l1tf(mitigation: L1TFMitigation) {
        security { l1tf = mitigation }
    }
    
    fun mds(mitigation: MDSMitigation) {
        security { mds = mitigation }
    }
    
    fun selinux(enabled: Boolean) = parameter("selinux", if (enabled) "1" else "0")
    fun apparmor(enabled: Boolean) = parameter("apparmor", if (enabled) "1" else "0")
    
    fun toConfig(): KernelConfig {
        return KernelConfig(
            parameters = parameters,
            modules = modules,
            version = version,
            variants = variants,
            compression = compression,
            debugging = debugging,
            security = security
        )
    }
}

@HorizonOSDsl
class KernelModuleContext {
    private val blacklist = mutableListOf<String>()
    private val load = mutableListOf<String>()
    private val options = mutableMapOf<String, String>()
    var autoLoad: Boolean = true
    var compression: ModuleCompression = ModuleCompression.XZ
    
    fun blacklist(module: String) {
        blacklist.add(module)
    }
    
    fun load(module: String) {
        load.add(module)
    }
    
    fun option(module: String, options: String) {
        this.options[module] = options
    }
    
    fun toConfig(): KernelModuleConfig {
        return KernelModuleConfig(
            blacklist = blacklist,
            load = load,
            options = options,
            autoLoad = autoLoad,
            compression = compression
        )
    }
}

@HorizonOSDsl
class KernelVariantContext(
    private val name: String,
    private val version: String,
    private val path: String
) {
    var enabled: Boolean = true
    var description: String? = null
    var isDefault: Boolean = false
    
    fun toVariant(): KernelVariant {
        return KernelVariant(
            name = name,
            version = version,
            path = path,
            enabled = enabled,
            description = description,
            isDefault = isDefault
        )
    }
}

@HorizonOSDsl
class KernelDebuggingContext {
    var enabled: Boolean = false
    var kgdb: Boolean = false
    var kdb: Boolean = false
    var earlyPrintk: Boolean = false
    var debugLevel: Int = 0
    var ftrace: Boolean = false
    var kprobes: Boolean = false
    var dynamicDebug: Boolean = false
    
    fun toConfig(): KernelDebugging {
        return KernelDebugging(
            enabled = enabled,
            kgdb = kgdb,
            kdb = kdb,
            earlyPrintk = earlyPrintk,
            debugLevel = debugLevel,
            ftrace = ftrace,
            kprobes = kprobes,
            dynamicDebug = dynamicDebug
        )
    }
}

@HorizonOSDsl
class KernelSecurityContext {
    var kaslr: Boolean = true
    var smep: Boolean = true
    var smap: Boolean = true
    var pti: Boolean = true
    var kpti: Boolean = true
    var spectreV2: SpectreV2Mitigation = SpectreV2Mitigation.AUTO
    var spectreV4: Boolean = true
    var l1tf: L1TFMitigation = L1TFMitigation.FLUSH
    var mds: MDSMitigation = MDSMitigation.FULL
    var tsx: Boolean = false
    var ibrs: Boolean = true
    var ibpb: Boolean = true
    var stibp: Boolean = true
    var ssbd: Boolean = true
    var retpoline: Boolean = true
    
    fun toConfig(): KernelSecurity {
        return KernelSecurity(
            kaslr = kaslr,
            smep = smep,
            smap = smap,
            pti = pti,
            kpti = kpti,
            spectreV2 = spectreV2,
            spectreV4 = spectreV4,
            l1tf = l1tf,
            mds = mds,
            tsx = tsx,
            ibrs = ibrs,
            ibpb = ibpb,
            stibp = stibp,
            ssbd = ssbd,
            retpoline = retpoline
        )
    }
}