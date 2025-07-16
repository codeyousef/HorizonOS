package org.horizonos.config.dsl.security

import kotlinx.serialization.Serializable

// ===== SELinux Configuration =====

@Serializable
data class SELinuxConfig(
    val enabled: Boolean = false,
    val mode: SELinuxMode = SELinuxMode.PERMISSIVE,
    val policy: String = "targeted",
    val enforcing: Boolean = false,
    val modules: List<SELinuxModule> = emptyList(),
    val contexts: List<SELinuxContext> = emptyList(),
    val users: List<SELinuxUser> = emptyList(),
    val ports: List<SELinuxPort> = emptyList(),
    val files: List<SELinuxFile> = emptyList(),
    val booleans: Map<String, Boolean> = emptyMap(),
    val auditAllow: Boolean = false,
    val auditDeny: Boolean = true,
    val checkreqprot: Boolean = false
)

@Serializable
data class SELinuxModule(
    val name: String,
    val version: String,
    val enabled: Boolean = true,
    val priority: Int = 400,
    val path: String? = null
)

@Serializable
data class SELinuxContext(
    val path: String,
    val context: String,
    val recursive: Boolean = false
)

@Serializable
data class SELinuxUser(
    val name: String,
    val selinuxUser: String,
    val mlsRange: String? = null,
    val roles: List<String> = emptyList()
)

@Serializable
data class SELinuxPort(
    val port: Int,
    val protocol: NetworkProtocol,
    val context: String
)

@Serializable
data class SELinuxFile(
    val path: String,
    val type: String,
    val recursive: Boolean = false,
    val restorecon: Boolean = true
)

// ===== AppArmor Configuration =====

@Serializable
data class AppArmorConfig(
    val enabled: Boolean = false,
    val mode: AppArmorMode = AppArmorMode.ENFORCE,
    val profiles: List<AppArmorProfile> = emptyList(),
    val includes: List<String> = emptyList(),
    val abstractions: List<AppArmorAbstraction> = emptyList(),
    val complain: Boolean = false,
    val auditMode: Boolean = false,
    val cacheDir: String = "/var/cache/apparmor"
)

@Serializable
data class AppArmorProfile(
    val name: String,
    val path: String,
    val mode: AppArmorMode = AppArmorMode.ENFORCE,
    val rules: List<AppArmorRule> = emptyList(),
    val includes: List<String> = emptyList(),
    val flags: List<String> = emptyList()
)

@Serializable
data class AppArmorRule(
    val type: AppArmorRuleType,
    val path: String,
    val permissions: List<String> = emptyList(),
    val owner: Boolean = false,
    val prefix: String? = null
)

@Serializable
data class AppArmorAbstraction(
    val name: String,
    val path: String
)

// ===== Enums =====

@Serializable
enum class SELinuxMode {
    ENFORCING,    // SELinux enforces policy
    PERMISSIVE,   // SELinux logs but doesn't enforce
    DISABLED      // SELinux is disabled
}

@Serializable
enum class NetworkProtocol {
    TCP,
    UDP,
    ICMP,
    SCTP,
    DCCP
}

@Serializable
enum class AppArmorMode {
    ENFORCE,      // Enforce profile rules
    COMPLAIN,     // Log violations but don't enforce
    AUDIT,        // Log all accesses
    DISABLE       // Disable profile
}

@Serializable
enum class AppArmorRuleType {
    FILE,         // File access rule
    NETWORK,      // Network access rule
    CAPABILITY,   // Capability rule
    DBUS,         // D-Bus rule
    MOUNT,        // Mount rule
    PIVOT_ROOT,   // Pivot root rule
    PTRACE,       // Process tracing rule
    SIGNAL,       // Signal rule
    UNIX,         // Unix domain socket rule
    RLIMIT        // Resource limit rule
}