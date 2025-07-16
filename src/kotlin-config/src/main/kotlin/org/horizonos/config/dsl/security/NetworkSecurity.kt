package org.horizonos.config.dsl.security

import kotlinx.serialization.Serializable
import kotlin.time.Duration
import kotlin.time.Duration.Companion.minutes
import kotlin.time.Duration.Companion.hours

// ===== Fail2Ban Configuration =====

@Serializable
data class Fail2BanConfig(
    val enabled: Boolean = true,
    val logLevel: LogLevel = LogLevel.INFO,
    val logtarget: String = "/var/log/fail2ban.log",
    val syslogTarget: SyslogFacility = SyslogFacility.LOCAL0,
    val syslogSocket: String = "auto",
    val pidfile: String = "/var/run/fail2ban/fail2ban.pid",
    val dbfile: String = "/var/lib/fail2ban/fail2ban.sqlite3",
    val dbpurgeage: Duration = 24.hours,
    val jails: List<Fail2BanJail> = emptyList(),
    val backend: Fail2BanBackend = Fail2BanBackend.AUTO,
    val usedns: UseDNSPolicy = UseDNSPolicy.WARN,
    val banTime: Duration = 10.minutes,
    val findTime: Duration = 10.minutes,
    val maxRetry: Int = 5
)

@Serializable
data class Fail2BanJail(
    val name: String,
    val enabled: Boolean = true,
    val filter: String,
    val logpath: List<String>,
    val backend: Fail2BanBackend = Fail2BanBackend.AUTO,
    val maxRetry: Int = 5,
    val findTime: Duration = 10.minutes,
    val banTime: Duration = 10.minutes,
    val usedns: UseDNSPolicy = UseDNSPolicy.WARN,
    val banAction: String = "iptables-multiport",
    val action: String = "%(banaction)s[name=%(__name__)s, port=\"%(port)s\"]",
    val ignoreIp: List<String> = listOf("127.0.0.1/8"),
    val ignoreCommand: String? = null,
    val ignoreSelf: Boolean = true,
    val port: String = "ssh",
    val protocol: String = "tcp"
)

// ===== Firewall Configuration =====

@Serializable
data class FirewallConfig(
    val enabled: Boolean = true,
    val backend: FirewallBackend = FirewallBackend.NFTABLES,
    val defaultPolicy: Map<String, FirewallPolicy> = mapOf(
        "INPUT" to FirewallPolicy.DROP,
        "FORWARD" to FirewallPolicy.DROP,
        "OUTPUT" to FirewallPolicy.ACCEPT
    ),
    val rules: List<FirewallRule> = emptyList(),
    val chains: List<FirewallChain> = emptyList(),
    val zones: List<FirewallZone> = emptyList(),
    val services: List<FirewallService> = emptyList(),
    val logging: FirewallLogging = FirewallLogging()
)

@Serializable
data class FirewallRule(
    val name: String,
    val chain: String = "INPUT",
    val table: String = "filter",
    val protocol: String? = null,
    val source: String? = null,
    val destination: String? = null,
    val sport: String? = null,
    val dport: String? = null,
    val state: List<ConnectionState> = emptyList(),
    val action: FirewallAction = FirewallAction.ACCEPT,
    val comment: String? = null,
    val priority: Int = 0,
    val enabled: Boolean = true
)

@Serializable
data class FirewallChain(
    val name: String,
    val table: String = "filter",
    val policy: FirewallPolicy = FirewallPolicy.ACCEPT,
    val hook: String? = null,
    val priority: Int = 0
)

@Serializable
data class FirewallZone(
    val name: String,
    val interfaces: List<String> = emptyList(),
    val sources: List<String> = emptyList(),
    val services: List<String> = emptyList(),
    val ports: List<String> = emptyList(),
    val protocols: List<String> = emptyList(),
    val masquerade: Boolean = false,
    val forwardPorts: List<String> = emptyList(),
    val target: FirewallPolicy = FirewallPolicy.DEFAULT
)

@Serializable
data class FirewallService(
    val name: String,
    val ports: List<String> = emptyList(),
    val protocols: List<String> = emptyList(),
    val sourcePort: List<String> = emptyList(),
    val modules: List<String> = emptyList(),
    val destinations: Map<String, String> = emptyMap()
)

@Serializable
data class FirewallLogging(
    val enabled: Boolean = false,
    val logLevel: LogLevel = LogLevel.INFO,
    val logPrefix: String = "FIREWALL: ",
    val logDropped: Boolean = true,
    val logRejected: Boolean = true,
    val logAccepted: Boolean = false,
    val rateLimitBurst: Int = 5,
    val rateLimitInterval: Duration = 1.minutes
)

// ===== Audit Configuration =====

@Serializable
data class AuditConfig(
    val enabled: Boolean = true,
    val rules: List<AuditRule> = emptyList(),
    val bufferSize: Int = 8192,
    val failureMode: Int = 1,
    val maxLogFile: Int = 10,
    val maxLogFileAction: AuditAction = AuditAction.ROTATE,
    val spaceLeft: Int = 75,
    val spaceLeftAction: AuditAction = AuditAction.SYSLOG,
    val adminSpaceLeft: Int = 50,
    val adminSpaceLeftAction: AuditAction = AuditAction.SUSPEND,
    val diskFull: AuditAction = AuditAction.SUSPEND,
    val diskError: AuditAction = AuditAction.SUSPEND,
    val logFormat: AuditLogFormat = AuditLogFormat.RAW,
    val logGroup: String = "root",
    val priority: AuditQOS = AuditQOS.LOSSY,
    val flushMode: String = "INCREMENTAL_ASYNC",
    val freq: Int = 50,
    val numLogs: Int = 5,
    val dispatcher: String = "/sbin/audispd",
    val nameFormat: AuditNameFormat = AuditNameFormat.NONE,
    val name: String? = null,
    val pluginDir: String = "/etc/audisp/plugins.d",
    val tcpListenPort: Int? = null,
    val tcpMaxPerAddr: Int = 1,
    val tcpClientPorts: List<Int> = emptyList(),
    val tcpClientMaxIdle: Duration = 0.minutes,
    val enableKrb5: Boolean = false,
    val krb5Principal: String? = null,
    val krb5KeyTab: String? = null
)

@Serializable
data class AuditRule(
    val rule: String,
    val type: AuditRuleType = AuditRuleType.SYSCALL,
    val enabled: Boolean = true,
    val comment: String? = null
)

// ===== Enums =====

@Serializable
enum class Fail2BanBackend {
    AUTO,         // Automatic backend selection
    PYINOTIFY,    // Python inotify backend
    GAMIN,        // Gamin backend
    POLLING,      // Polling backend
    SYSTEMD       // Systemd journal backend
}

@Serializable
enum class UseDNSPolicy {
    YES,          // Use DNS lookups
    NO,           // Don't use DNS lookups
    WARN,         // Use DNS but warn on slow lookups
    RAW           // Use raw IP addresses only
}

@Serializable
enum class FirewallBackend {
    IPTABLES,     // Legacy iptables
    NFTABLES,     // Modern nftables
    FIREWALLD,    // FirewallD daemon
    UFW           // Uncomplicated Firewall
}

@Serializable
enum class FirewallPolicy {
    ACCEPT,       // Accept packets
    DROP,         // Drop packets silently
    REJECT,       // Reject packets with response
    DEFAULT       // Use default policy
}

@Serializable
enum class FirewallAction {
    ACCEPT,       // Accept the packet
    DROP,         // Drop the packet silently
    REJECT,       // Reject with ICMP response
    LOG,          // Log the packet
    QUEUE,        // Queue for userspace processing
    RETURN,       // Return to calling chain
    SNAT,         // Source NAT
    DNAT,         // Destination NAT
    MASQUERADE,   // Masquerade (dynamic SNAT)
    REDIRECT      // Redirect to local port
}

@Serializable
enum class ConnectionState {
    NEW,          // New connection
    ESTABLISHED,  // Established connection
    RELATED,      // Related to existing connection
    INVALID,      // Invalid connection state
    UNTRACKED     // Untracked connection
}

@Serializable
enum class AuditAction {
    IGNORE,       // Ignore the event
    SYSLOG,       // Send to syslog
    EMAIL,        // Send email notification
    EXEC,         // Execute a program
    SUSPEND,      // Suspend auditing
    SINGLE,       // Single user mode
    HALT,         // Halt the system
    ROTATE,       // Rotate log files
    KEEP_LOGS     // Keep all log files
}

@Serializable
enum class AuditLogFormat {
    RAW,          // Raw format
    NOLOG         // No logging
}

@Serializable
enum class AuditQOS {
    LOSSY,        // Lossy delivery
    LOSSLESS      // Lossless delivery
}

@Serializable
enum class AuditNameFormat {
    NONE,         // No name resolution
    HOSTNAME,     // Use hostname
    FQD,          // Fully qualified domain name
    NUMERIC,      // Numeric format
    USER          // User-defined format
}

@Serializable
enum class AuditRuleType {
    SYSCALL,      // System call rule
    WATCH,        // File/directory watch rule
    CONTROL       // Control rule
}