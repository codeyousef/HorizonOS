package org.horizonos.config.dsl.security

import kotlinx.serialization.Serializable
import kotlin.time.Duration
import kotlin.time.Duration.Companion.days

// ===== GPG Configuration =====

@Serializable
data class GPGConfig(
    val enabled: Boolean = true,
    val defaultKeyType: String = "RSA",
    val defaultKeyLength: Int = 4096,
    val defaultCipher: String = "AES256",
    val defaultDigest: String = "SHA512",
    val defaultCompress: Int = 2,
    val trustModel: GPGTrustModel = GPGTrustModel.PGP,
    val keyservers: List<String> = listOf("hkps://keys.openpgp.org"),
    val keys: List<GPGKey> = emptyList(),
    val agentConfig: GPGAgentConfig = GPGAgentConfig(),
    val autoKeyRetrieve: Boolean = false,
    val autoKeyLocate: List<String> = emptyList()
)

@Serializable
data class GPGKey(
    val keyId: String,
    val fingerprint: String,
    val owner: String,
    val email: String,
    val trustLevel: GPGTrustLevel = GPGTrustLevel.UNKNOWN,
    val imported: Boolean = false,
    val publicKey: String? = null,
    val secretKey: String? = null
)

@Serializable
data class GPGAgentConfig(
    val enabled: Boolean = true,
    val defaultCache: Duration = 600.days,
    val maxCache: Duration = 7200.days,
    val pinentryProgram: String = "/usr/bin/pinentry-gtk2",
    val enableSshSupport: Boolean = false,
    val grabKeyboard: Boolean = true,
    val allowMarkTrusted: Boolean = true,
    val allowPresetPassphrase: Boolean = false,
    val ignoreCache: Boolean = false,
    val noGreeting: Boolean = false,
    val keepDisplay: Boolean = false
)

// ===== Certificate Configuration =====

@Serializable
data class CertificateConfig(
    val enabled: Boolean = true,
    val trustedCerts: List<Certificate> = emptyList(),
    val caCerts: List<CAConfig> = emptyList(),
    val stores: List<CertificateStore> = emptyList(),
    val autoRenewal: AutoRenewalConfig = AutoRenewalConfig()
)

@Serializable
data class CAConfig(
    val name: String,
    val certificate: String,
    val privateKey: String? = null,
    val intermediate: Boolean = false,
    val parentCA: String? = null,
    val keyUsage: List<String> = listOf("digitalSignature", "keyCertSign"),
    val extendedKeyUsage: List<String> = emptyList(),
    val validityPeriod: Duration = 3650.days,
    val subjectKeyIdentifier: Boolean = true,
    val authorityKeyIdentifier: Boolean = true,
    val basicConstraints: String = "CA:TRUE",
    val crlDistributionPoints: List<String> = emptyList(),
    val ocspResponders: List<String> = emptyList()
)

@Serializable
data class Certificate(
    val name: String,
    val path: String,
    val format: CertificateFormat = CertificateFormat.PEM,
    val type: CertificateType = CertificateType.SERVER,
    val subject: String,
    val issuer: String? = null,
    val validFrom: String? = null,
    val validTo: String? = null,
    val keyUsage: List<String> = emptyList(),
    val subjectAltNames: List<String> = emptyList(),
    val autoRenew: Boolean = false,
    val renewThreshold: Duration = 30.days
)

@Serializable
data class CertificateStore(
    val name: String,
    val path: String,
    val type: CertificateStoreType = CertificateStoreType.SYSTEM
)

@Serializable
data class AutoRenewalConfig(
    val enabled: Boolean = false,
    val checkInterval: Duration = 1.days,
    val renewThreshold: Duration = 30.days,
    val retryAttempts: Int = 3,
    val notifyEmail: String? = null
)

// ===== TPM Security Configuration =====

@Serializable
data class TPMSecurityConfig(
    val enabled: Boolean = false,
    val version: String = "2.0",
    val ownership: TPMOwnership = TPMOwnership(),
    val pcr: TPMPCRConfig = TPMPCRConfig(),
    val ima: TPMIMAConfig = TPMIMAConfig(),
    val attestation: TPMAttestationConfig = TPMAttestationConfig()
)

@Serializable
data class TPMOwnership(
    val takeOwnership: Boolean = false,
    val ownerPassword: String? = null,
    val srkPassword: String? = null,
    val clearOwnership: Boolean = false
)

@Serializable
data class TPMPCRConfig(
    val enabled: Boolean = true,
    val extends: List<TPMPCRExtend> = emptyList()
)

@Serializable
data class TPMPCRExtend(
    val pcr: Int,
    val value: String,
    val description: String? = null
)

@Serializable
data class TPMIMAConfig(
    val enabled: Boolean = false,
    val pcrIndex: Int = 10,
    val hashAlgorithm: String = "sha256",
    val template: String = "ima-ng"
)

@Serializable
data class TPMAttestationConfig(
    val enabled: Boolean = false,
    val quoteKey: String? = null,
    val nonce: String? = null
)

// ===== Enums =====

@Serializable
enum class GPGTrustModel {
    PGP,          // Standard PGP trust model
    CLASSIC,      // Classic PGP trust model
    DIRECT,       // Direct trust model
    ALWAYS,       // Always trust
    AUTO          // Automatic trust model
}

@Serializable
enum class GPGTrustLevel {
    UNKNOWN,      // Unknown trust
    NEVER,        // Never trust
    MARGINAL,     // Marginal trust
    FULL,         // Full trust
    ULTIMATE      // Ultimate trust
}

@Serializable
enum class CertificateFormat {
    PEM,          // Privacy-Enhanced Mail
    DER,          // Distinguished Encoding Rules
    PKCS7,        // Public Key Cryptography Standards #7
    PKCS12        // Public Key Cryptography Standards #12
}

@Serializable
enum class CertificateType {
    SERVER,       // Server certificate
    CLIENT,       // Client certificate
    CA,           // Certificate Authority
    INTERMEDIATE, // Intermediate CA certificate
    ROOT          // Root CA certificate
}

@Serializable
enum class CertificateStoreType {
    SYSTEM,       // System certificate store
    USER,         // User certificate store
    CUSTOM        // Custom certificate store
}