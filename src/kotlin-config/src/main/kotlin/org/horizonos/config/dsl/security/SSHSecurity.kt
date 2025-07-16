package org.horizonos.config.dsl.security

import kotlinx.serialization.Serializable
import kotlin.time.Duration
import kotlin.time.Duration.Companion.minutes

// ===== SSH Configuration =====

@Serializable
data class SSHConfig(
    val enabled: Boolean = true,
    val port: Int = 22,
    val protocol: SSHProtocol = SSHProtocol.VERSION_2,
    val auth: SSHAuthConfig = SSHAuthConfig(),
    val encryption: SSHEncryptionConfig = SSHEncryptionConfig(),
    val access: SSHAccessConfig = SSHAccessConfig(),
    val clientAlive: ClientAliveConfig = ClientAliveConfig(),
    val security: SSHSecurityConfig = SSHSecurityConfig(),
    val forwarding: SSHForwardingConfig = SSHForwardingConfig(),
    val keys: SSHKeyConfig = SSHKeyConfig(),
    val logging: SSHLoggingConfig = SSHLoggingConfig()
)

@Serializable
data class SSHAuthConfig(
    val passwordAuth: Boolean = true,
    val pubkeyAuth: Boolean = true,
    val kbdInteractiveAuth: Boolean = false,
    val challengeResponseAuth: Boolean = false,
    val gssapiAuth: Boolean = false,
    val hostbasedAuth: Boolean = false,
    val rootLogin: RootLoginPolicy = RootLoginPolicy.PROHIBIT_PASSWORD,
    val emptyPasswords: Boolean = false,
    val maxAuthTries: Int = 3,
    val maxSessions: Int = 10,
    val loginGraceTime: Duration = 2.minutes
)

@Serializable
data class SSHEncryptionConfig(
    val ciphers: List<String> = listOf(
        "chacha20-poly1305@openssh.com",
        "aes256-gcm@openssh.com",
        "aes128-gcm@openssh.com",
        "aes256-ctr",
        "aes192-ctr",
        "aes128-ctr"
    ),
    val macs: List<String> = listOf(
        "umac-128-etm@openssh.com",
        "hmac-sha2-256-etm@openssh.com",
        "hmac-sha2-512-etm@openssh.com"
    ),
    val kexAlgorithms: List<String> = listOf(
        "curve25519-sha256",
        "curve25519-sha256@libssh.org",
        "ecdh-sha2-nistp256",
        "ecdh-sha2-nistp384",
        "ecdh-sha2-nistp521"
    ),
    val compression: SSHCompression = SSHCompression.DELAYED
)

@Serializable
data class SSHAccessConfig(
    val allowUsers: List<String> = emptyList(),
    val allowGroups: List<String> = emptyList(),
    val denyUsers: List<String> = emptyList(),
    val denyGroups: List<String> = emptyList(),
    val listenAddresses: List<String> = listOf("0.0.0.0"),
    val permitTunnel: Boolean = false,
    val gatewayPorts: Boolean = false,
    val maxStartups: String = "10:30:100"
)

@Serializable
data class ClientAliveConfig(
    val enabled: Boolean = true,
    val interval: Duration = 15.minutes,
    val maxCount: Int = 3
)

@Serializable
data class SSHSecurityConfig(
    val strictModes: Boolean = true,
    val ignoreuserKnownHosts: Boolean = false,
    val ignoreRhosts: Boolean = true,
    val hostbasedUsesNameFromPacketOnly: Boolean = false,
    val permitUserEnvironment: Boolean = false,
    val acceptEnv: List<String> = listOf("LANG", "LC_*"),
    val setEnv: Map<String, String> = emptyMap(),
    val chrootDirectory: String? = null,
    val forceCommand: String? = null
)

@Serializable
data class SSHForwardingConfig(
    val x11Forwarding: Boolean = false,
    val x11DisplayOffset: Int = 10,
    val x11UseLocalhost: Boolean = true,
    val tcpForwarding: Boolean = true,
    val agentForwarding: Boolean = false,
    val streamLocalBindMask: String = "0177",
    val streamLocalBindUnlink: Boolean = false
)

@Serializable
data class SSHKeyConfig(
    val hostKeys: List<SSHHostKey> = emptyList(),
    val authorizedKeys: List<SSHAuthorizedKey> = emptyList(),
    val knownHosts: List<SSHKnownHost> = emptyList(),
    val keyGeneration: SSHKeyGenConfig = SSHKeyGenConfig()
)

@Serializable
data class SSHHostKey(
    val type: SSHKeyType,
    val path: String,
    val bits: Int? = null
)

@Serializable
data class SSHAuthorizedKey(
    val user: String,
    val key: String,
    val keyType: SSHKeyType,
    val comment: String? = null,
    val options: List<String> = emptyList()
)

@Serializable
data class SSHKnownHost(
    val host: String,
    val keyType: SSHKeyType,
    val key: String,
    val port: Int = 22
)

@Serializable
data class SSHKeyGenConfig(
    val generateHostKeys: Boolean = true,
    val defaultKeyType: SSHKeyType = SSHKeyType.ED25519,
    val keyStrength: Int = 256,
    val autoRegenerateWeakKeys: Boolean = true
)

@Serializable
data class SSHLoggingConfig(
    val logLevel: LogLevel = LogLevel.INFO,
    val syslogFacility: SyslogFacility = SyslogFacility.AUTH,
    val logFailures: Boolean = true,
    val logSuccesses: Boolean = false,
    val verboseLogging: Boolean = false
)

// ===== Enums =====

@Serializable
enum class SSHProtocol {
    VERSION_1,
    VERSION_2
}

@Serializable
enum class RootLoginPolicy {
    YES,                    // Allow root login
    NO,                     // Deny root login
    PROHIBIT_PASSWORD,      // Allow only key-based root login
    FORCED_COMMANDS_ONLY    // Only allow root with forced commands
}

@Serializable
enum class SSHCompression {
    YES,     // Enable compression
    NO,      // Disable compression  
    DELAYED  // Enable after authentication
}

@Serializable
enum class SSHKeyType {
    RSA,
    DSA,
    ECDSA,
    ED25519
}

@Serializable
enum class LogLevel {
    QUIET,
    FATAL, 
    ERROR,
    INFO,
    VERBOSE,
    DEBUG,
    DEBUG1,
    DEBUG2,
    DEBUG3
}

@Serializable
enum class SyslogFacility {
    DAEMON,
    USER,
    AUTH,
    AUTHPRIV,
    LOCAL0,
    LOCAL1,
    LOCAL2,
    LOCAL3,
    LOCAL4,
    LOCAL5,
    LOCAL6,
    LOCAL7
}