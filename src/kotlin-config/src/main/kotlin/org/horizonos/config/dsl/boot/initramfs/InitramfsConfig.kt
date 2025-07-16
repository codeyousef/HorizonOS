package org.horizonos.config.dsl.boot.initramfs

import kotlinx.serialization.Serializable

// ===== Initramfs Configuration =====

@Serializable
data class InitramfsConfig(
    val generator: InitramfsGenerator = InitramfsGenerator.MKINITCPIO,
    val compression: InitramfsCompression = InitramfsCompression.ZSTD,
    val hooks: List<String> = emptyList(),
    val modules: List<String> = emptyList(),
    val binaries: List<String> = emptyList(),
    val files: List<String> = emptyList(),
    val customScripts: List<String> = emptyList(),
    val includeKernel: Boolean = false,
    val includeSystemd: Boolean = true,
    val microcode: MicrocodeConfig = MicrocodeConfig(),
    val encryption: InitramfsEncryption = InitramfsEncryption()
)

@Serializable
data class MicrocodeConfig(
    val enabled: Boolean = true,
    val intel: Boolean = true,
    val amd: Boolean = true,
    val earlyLoad: Boolean = true,
    val updatePath: String? = null
)

@Serializable
data class InitramfsEncryption(
    val enabled: Boolean = false,
    val method: EncryptionMethod = EncryptionMethod.LUKS,
    val keyfile: String? = null,
    val tpm: Boolean = false,
    val yubikey: Boolean = false
)

// Initramfs Enums
@Serializable
enum class InitramfsGenerator {
    MKINITCPIO,
    DRACUT,
    INITRAMFS_TOOLS,
    CUSTOM
}

@Serializable
enum class InitramfsCompression {
    NONE,
    GZIP,
    BZIP2,
    LZMA,
    XZ,
    LZO,
    LZ4,
    ZSTD
}

@Serializable
enum class EncryptionMethod {
    LUKS,
    PLAIN,
    TRUECRYPT,
    VERACRYPT
}