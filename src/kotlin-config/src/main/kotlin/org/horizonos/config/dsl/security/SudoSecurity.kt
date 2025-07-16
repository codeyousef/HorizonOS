package org.horizonos.config.dsl.security

import kotlinx.serialization.Serializable
import kotlin.time.Duration
import kotlin.time.Duration.Companion.minutes

// ===== Sudo Configuration =====

@Serializable
data class SudoConfig(
    val enabled: Boolean = true,
    val rules: List<SudoRule> = emptyList(),
    val defaults: SudoDefaults = SudoDefaults(),
    val aliases: SudoAliases = SudoAliases(),
    val security: SudoSecurity = SudoSecurity(),
    val logging: SudoLogging = SudoLogging()
)

@Serializable
data class SudoRule(
    val users: List<String> = emptyList(),
    val groups: List<String> = emptyList(),
    val hosts: List<String> = listOf("ALL"),
    val runAsUsers: List<String> = listOf("ALL"),
    val runAsGroups: List<String> = listOf("ALL"),
    val commands: List<String> = listOf("ALL"),
    val tags: List<SudoTag> = emptyList(),
    val options: List<String> = emptyList()
)

@Serializable
data class SudoDefaults(
    val requireTty: Boolean = false,
    val visiblepw: Boolean = true,
    val pwfeedback: Boolean = false,
    val rootpw: Boolean = false,
    val runaspw: Boolean = false,
    val targetpw: Boolean = false,
    val alwaysSetHome: Boolean = false,
    val setHome: Boolean = false,
    val envReset: Boolean = true,
    val envKeep: List<String> = listOf(
        "COLORS", "DISPLAY", "HOSTNAME", "HISTSIZE", "KDEDIR", "LS_COLORS",
        "MAIL", "PS1", "PS2", "QTDIR", "USERNAME", "LANG", "LC_ADDRESS",
        "LC_CTYPE", "LC_COLLATE", "LC_IDENTIFICATION", "LC_MEASUREMENT",
        "LC_MESSAGES", "LC_MONETARY", "LC_NAME", "LC_NUMERIC", "LC_PAPER",
        "LC_TELEPHONE", "LC_TIME", "LC_ALL", "LANGUAGE", "LINGUAS", "_XKB_CHARSET"
    ),
    val securePathOverride: String? = null,
    val timestampTimeout: Duration = 15.minutes,
    val passwordTimeout: Duration = 5.minutes,
    val maxTries: Int = 3
)

@Serializable
data class SudoAliases(
    val userAliases: Map<String, List<String>> = emptyMap(),
    val runAsAliases: Map<String, List<String>> = emptyMap(), 
    val hostAliases: Map<String, List<String>> = emptyMap(),
    val cmdAliases: Map<String, List<String>> = emptyMap()
)

@Serializable
data class SudoSecurity(
    val requireAuthentication: Boolean = true,
    val rootSudo: Boolean = true,
    val logHost: Boolean = true,
    val logYear: Boolean = true,
    val shellEscape: Boolean = false,
    val setLogfile: Boolean = true,
    val insults: Boolean = false,
    val badpassMessage: String = "Sorry, try again.",
    val lectureFile: String? = null,
    val lecture: SudoLectureMode = SudoLectureMode.ONCE,
    val listpw: Boolean = true,
    val verifypw: Boolean = true,
    val closeSessions: Boolean = true,
    val use_pty: Boolean = true
)

@Serializable
data class SudoLogging(
    val syslog: Boolean = true,
    val syslogFacility: SyslogFacility = SyslogFacility.AUTHPRIV,
    val syslogPriority: String = "notice",
    val logFile: String? = null,
    val logInput: Boolean = false,
    val logOutput: Boolean = false,
    val compress: Boolean = true,
    val useIOPlugin: Boolean = false,
    val maxLogSize: String = "10M",
    val logDir: String = "/var/log/sudo",
    val iologDir: String = "/var/log/sudo-io",
    val iologFile: String = "%{seq}",
    val iologFlush: Boolean = true
)

// ===== Enums =====

@Serializable
enum class SudoTag {
    NOPASSWD,     // No password required
    PASSWD,       // Password required (default)
    NOEXEC,       // Disable exec() family functions
    EXEC,         // Allow exec() family functions (default)
    SETENV,       // Allow setting environment variables
    NOSETENV,     // Disallow setting environment variables (default)
    LOG_INPUT,    // Log command input
    NOLOG_INPUT,  // Don't log command input (default)
    LOG_OUTPUT,   // Log command output
    NOLOG_OUTPUT, // Don't log command output (default)
    MAIL,         // Send mail on security events
    NOMAIL,       // Don't send mail
    FOLLOW,       // Follow symbolic links
    NOFOLLOW      // Don't follow symbolic links
}

@Serializable
enum class SudoLectureMode {
    NEVER,        // Never show lecture
    ONCE,         // Show lecture once per user
    ALWAYS        // Always show lecture
}