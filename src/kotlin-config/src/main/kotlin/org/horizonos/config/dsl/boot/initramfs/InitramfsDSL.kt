package org.horizonos.config.dsl.boot.initramfs

import org.horizonos.config.dsl.HorizonOSDsl

// ===== Initramfs DSL Builders =====

@HorizonOSDsl
class InitramfsContext {
    var generator: InitramfsGenerator = InitramfsGenerator.MKINITCPIO
    var compression: InitramfsCompression = InitramfsCompression.ZSTD
    private val hooks = mutableListOf<String>()
    private val modules = mutableListOf<String>()
    private val binaries = mutableListOf<String>()
    private val files = mutableListOf<String>()
    private val customScripts = mutableListOf<String>()
    var includeKernel: Boolean = false
    var includeSystemd: Boolean = true
    private var microcode = MicrocodeConfig()
    private var encryption = InitramfsEncryption()
    
    fun hook(name: String) {
        hooks.add(name)
    }
    
    fun module(name: String) {
        modules.add(name)
    }
    
    fun binary(path: String) {
        binaries.add(path)
    }
    
    fun file(path: String) {
        files.add(path)
    }
    
    fun customScript(path: String) {
        customScripts.add(path)
    }
    
    fun microcode(block: MicrocodeContext.() -> Unit) {
        microcode = MicrocodeContext().apply(block).toConfig()
    }
    
    fun encryption(block: InitramfsEncryptionContext.() -> Unit) {
        encryption = InitramfsEncryptionContext().apply(block).toConfig()
    }
    
    fun toConfig(): InitramfsConfig {
        return InitramfsConfig(
            generator = generator,
            compression = compression,
            hooks = hooks,
            modules = modules,
            binaries = binaries,
            files = files,
            customScripts = customScripts,
            includeKernel = includeKernel,
            includeSystemd = includeSystemd,
            microcode = microcode,
            encryption = encryption
        )
    }
}

@HorizonOSDsl
class MicrocodeContext {
    var enabled: Boolean = true
    var intel: Boolean = true
    var amd: Boolean = true
    var earlyLoad: Boolean = true
    var updatePath: String? = null
    
    fun toConfig(): MicrocodeConfig {
        return MicrocodeConfig(
            enabled = enabled,
            intel = intel,
            amd = amd,
            earlyLoad = earlyLoad,
            updatePath = updatePath
        )
    }
}

@HorizonOSDsl
class InitramfsEncryptionContext {
    var enabled: Boolean = false
    var method: EncryptionMethod = EncryptionMethod.LUKS
    var keyfile: String? = null
    var tpm: Boolean = false
    var yubikey: Boolean = false
    
    fun toConfig(): InitramfsEncryption {
        return InitramfsEncryption(
            enabled = enabled,
            method = method,
            keyfile = keyfile,
            tpm = tpm,
            yubikey = yubikey
        )
    }
}