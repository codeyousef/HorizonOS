package org.horizonos.config.dsl.security

import kotlinx.serialization.Serializable
import kotlin.time.Duration
import kotlin.time.Duration.Companion.minutes

/**
 * SSH Security Configuration for HorizonOS
 * 
 * This module provides comprehensive SSH server configuration including
 * authentication methods, encryption settings, access controls, and security
 * hardening options for secure remote access.
 * 
 * ## Key Features:
 * - Multiple authentication methods (key-based, password, interactive)
 * - Strong encryption cipher suites and key exchange algorithms
 * - Granular access control with user/group allow/deny lists
 * - Security hardening options and intrusion prevention
 * - Key management and host key configuration
 * - Detailed logging and monitoring options
 * 
 * ## Usage Example:
 * ```kotlin
 * security {
 *     ssh {
 *         enabled = true
 *         port = 22
 *         
 *         auth {
 *             passwordAuth = false
 *             pubkeyAuth = true
 *             rootLogin = RootLoginPolicy.PROHIBIT_PASSWORD
 *         }
 *         
 *         encryption {
 *             ciphers = listOf("chacha20-poly1305@openssh.com")
 *             compression = SSHCompression.DELAYED
 *         }
 *         
 *         access {
 *             allowGroups = listOf("ssh-users")
 *             denyUsers = listOf("guest")
 *         }
 *     }
 * }
 * ```
 * 
 * @since 1.0
 */

// ===== SSH Configuration =====

/**
 * Main SSH server configuration
 * 
 * This class defines the complete SSH server configuration including
 * authentication, encryption, access control, and security settings.
 */
@Serializable
data class SSHConfig(
    /** Whether SSH server is enabled */
    val enabled: Boolean = true,
    
    /** SSH server port number */
    val port: Int = 22,
    
    /** SSH protocol version */
    val protocol: SSHProtocol = SSHProtocol.VERSION_2,
    
    /** Authentication configuration */
    val auth: SSHAuthConfig = SSHAuthConfig(),
    
    /** Encryption and cipher configuration */
    val encryption: SSHEncryptionConfig = SSHEncryptionConfig(),
    
    /** Access control configuration */
    val access: SSHAccessConfig = SSHAccessConfig(),
    
    /** Client keep-alive configuration */
    val clientAlive: ClientAliveConfig = ClientAliveConfig(),
    
    /** Security hardening configuration */
    val security: SSHSecurityConfig = SSHSecurityConfig(),
    
    /** Port forwarding configuration */
    val forwarding: SSHForwardingConfig = SSHForwardingConfig(),
    
    /** SSH key management configuration */
    val keys: SSHKeyConfig = SSHKeyConfig(),
    
    /** Logging configuration */
    val logging: SSHLoggingConfig = SSHLoggingConfig()
)

/**
 * SSH authentication configuration
 * 
 * This class defines authentication methods, policies, and limits for SSH connections.
 */
@Serializable
data class SSHAuthConfig(
    /** Whether password authentication is allowed */
    val passwordAuth: Boolean = true,
    
    /** Whether public key authentication is allowed */
    val pubkeyAuth: Boolean = true,
    
    /** Whether keyboard-interactive authentication is allowed */
    val kbdInteractiveAuth: Boolean = false,
    
    /** Whether challenge-response authentication is allowed */
    val challengeResponseAuth: Boolean = false,
    
    /** Whether GSSAPI authentication is allowed */
    val gssapiAuth: Boolean = false,
    
    /** Whether host-based authentication is allowed */
    val hostbasedAuth: Boolean = false,
    
    /** Root login policy */
    val rootLogin: RootLoginPolicy = RootLoginPolicy.PROHIBIT_PASSWORD,
    
    /** Whether empty passwords are allowed */
    val emptyPasswords: Boolean = false,
    
    /** Maximum number of authentication attempts */
    val maxAuthTries: Int = 3,
    
    /** Maximum number of concurrent sessions */
    val maxSessions: Int = 10,
    
    /** Time allowed for login authentication */
    val loginGraceTime: Duration = 2.minutes
)

/**
 * SSH encryption and cryptographic configuration
 * 
 * This class defines cipher suites, MAC algorithms, key exchange methods,
 * and compression settings for secure SSH connections.
 */
@Serializable
data class SSHEncryptionConfig(
    /** List of allowed symmetric ciphers for encryption */
    val ciphers: List<String> = listOf(
        "chacha20-poly1305@openssh.com",
        "aes256-gcm@openssh.com",
        "aes128-gcm@openssh.com",
        "aes256-ctr",
        "aes192-ctr",
        "aes128-ctr"
    ),
    
    /** List of allowed message authentication codes (MACs) */
    val macs: List<String> = listOf(
        "umac-128-etm@openssh.com",
        "hmac-sha2-256-etm@openssh.com",
        "hmac-sha2-512-etm@openssh.com"
    ),
    
    /** List of allowed key exchange algorithms */
    val kexAlgorithms: List<String> = listOf(
        "curve25519-sha256",
        "curve25519-sha256@libssh.org",
        "ecdh-sha2-nistp256",
        "ecdh-sha2-nistp384",
        "ecdh-sha2-nistp521"
    ),
    
    /** Compression mode for SSH connections */
    val compression: SSHCompression = SSHCompression.DELAYED
)

/**
 * SSH access control configuration
 * 
 * This class defines user and group access controls, network bindings,
 * and connection limits for SSH server security.
 */
@Serializable
data class SSHAccessConfig(
    /** List of users allowed to connect via SSH */
    val allowUsers: List<String> = emptyList(),
    
    /** List of groups allowed to connect via SSH */
    val allowGroups: List<String> = emptyList(),
    
    /** List of users denied SSH access */
    val denyUsers: List<String> = emptyList(),
    
    /** List of groups denied SSH access */
    val denyGroups: List<String> = emptyList(),
    
    /** List of addresses to bind SSH server to */
    val listenAddresses: List<String> = listOf("0.0.0.0"),
    
    /** Whether to permit tunnel device forwarding */
    val permitTunnel: Boolean = false,
    
    /** Whether to allow gateway ports */
    val gatewayPorts: Boolean = false,
    
    /** Maximum concurrent unauthenticated connections (start:rate:full) */
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