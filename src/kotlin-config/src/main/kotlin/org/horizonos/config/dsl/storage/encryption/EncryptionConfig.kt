package org.horizonos.config.dsl.storage.encryption

import kotlinx.serialization.Serializable

// ===== Encryption Configuration =====

@Serializable
data class EncryptionConfig(
    val enabled: Boolean = false,
    val devices: List<EncryptedDevice> = emptyList(),
    val keyManagement: KeyManagement = KeyManagement(),
    val performance: EncryptionPerformance = EncryptionPerformance()
)

@Serializable
data class EncryptedDevice(
    val name: String,
    val device: String,
    val cipher: CipherSpec = CipherSpec.AES_XTS_PLAIN64,
    val keySize: Int = 512,
    val hashAlgorithm: HashAlgorithm = HashAlgorithm.SHA256,
    val iterTime: Int = 2000,
    val keyFile: String? = null,
    val header: LUKSHeader = LUKSHeader(),
    val discard: Boolean = false,
    val persistent: Boolean = true,
    val noReadWorkqueue: Boolean = false,
    val noWriteWorkqueue: Boolean = false,
    val integrityProtection: IntegrityProtection? = null
)

@Serializable
data class LUKSHeader(
    val version: LUKSVersion = LUKSVersion.LUKS2,
    val pbkdf: PBKDF = PBKDF.ARGON2ID,
    val memory: Int = 1048576, // 1GB in KB
    val parallelism: Int = 4,
    val detachedHeader: String? = null,
    val headerBackup: String? = null
)

@Serializable
data class IntegrityProtection(
    val algorithm: IntegrityAlgorithm = IntegrityAlgorithm.HMAC_SHA256,
    val journalSize: String = "64M",
    val journalWatermark: Int = 50,
    val journalCommitTime: Int = 10
)

@Serializable
data class KeyManagement(
    val allowDiscards: Boolean = false,
    val keyFiles: List<KeyFile> = emptyList(),
    val tpmIntegration: TPMIntegration = TPMIntegration(),
    val escrowKeys: List<EscrowKey> = emptyList(),
    val passwordQuality: PasswordQuality = PasswordQuality()
)

@Serializable
data class KeyFile(
    val path: String,
    val offset: Long = 0,
    val size: Long? = null,
    val permissions: String = "0400",
    val removeAfterBoot: Boolean = false
)

@Serializable
data class TPMIntegration(
    val enabled: Boolean = false,
    val tpmVersion: TPMVersion = TPMVersion.TPM2,
    val pcrs: List<Int> = listOf(0, 2, 4, 7),
    val sealingPolicy: String? = null,
    val nvramIndex: Int? = null
)

@Serializable
data class EscrowKey(
    val name: String,
    val publicKey: String,
    val description: String? = null
)

@Serializable
data class PasswordQuality(
    val minLength: Int = 12,
    val requireUppercase: Boolean = true,
    val requireLowercase: Boolean = true,
    val requireDigits: Boolean = true,
    val requireSpecial: Boolean = true,
    val prohibitReuse: Int = 5,
    val checkDictionary: Boolean = true
)

@Serializable
data class EncryptionPerformance(
    val queueDepth: Int = 128,
    val workqueueCPUs: List<Int>? = null,
    val sectorSize: Int = 512,
    val noReadWorkqueue: Boolean = false,
    val noWriteWorkqueue: Boolean = false,
    val submitFromCryptCPUs: Boolean = true
)

// Encryption Enums
@Serializable
enum class CipherSpec {
    AES_XTS_PLAIN64,
    AES_XTS_PLAIN,
    AES_CBC_ESSIV_SHA256,
    AES_CBC_PLAIN,
    SERPENT_XTS_PLAIN64,
    TWOFISH_XTS_PLAIN64,
    AES_ADIANTUM_PLAIN64,
    XCHACHA20_POLY1305_PLAIN64
}

@Serializable
enum class HashAlgorithm {
    SHA1,
    SHA256,
    SHA512,
    RIPEMD160,
    WHIRLPOOL
}

@Serializable
enum class LUKSVersion {
    LUKS1,
    LUKS2
}

@Serializable
enum class PBKDF {
    PBKDF2,
    ARGON2I,
    ARGON2ID
}

@Serializable
enum class IntegrityAlgorithm {
    HMAC_SHA256,
    HMAC_SHA512,
    POLY1305,
    BLAKE2B_256,
    BLAKE2S_256,
    CRC32,
    CRC32C,
    XXHASH64
}

@Serializable
enum class TPMVersion {
    TPM1_2,
    TPM2
}