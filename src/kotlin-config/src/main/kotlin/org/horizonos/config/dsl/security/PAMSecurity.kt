package org.horizonos.config.dsl.security

import kotlinx.serialization.Serializable
import kotlin.time.Duration
import kotlin.time.Duration.Companion.minutes

// ===== PAM Configuration =====

@Serializable
data class PAMConfig(
    val enabled: Boolean = true,
    val modules: List<PAMModule> = emptyList(),
    val rules: List<PAMRule> = emptyList(),
    val sessionConfig: PAMSessionConfig = PAMSessionConfig(),
    val passwordPolicy: PasswordPolicyConfig = PasswordPolicyConfig(),
    val twoFactor: TwoFactorConfig = TwoFactorConfig(),
    val lockout: AccountLockoutConfig = AccountLockoutConfig()
)

@Serializable
data class PAMModule(
    val name: String,
    val type: PAMType,
    val control: PAMControl,
    val module: String,
    val arguments: List<String> = emptyList(),
    val enabled: Boolean = true,
    val priority: Int = 0
)

@Serializable
data class PAMRule(
    val service: String,
    val type: PAMType,
    val control: PAMControl,
    val module: String,
    val arguments: List<String> = emptyList(),
    val conditions: List<PAMCondition> = emptyList()
)

@Serializable
data class PAMCondition(
    val type: PAMConditionType,
    val value: String,
    val operator: ComparisonOperator = ComparisonOperator.EQUALS
)

@Serializable
data class PAMSessionConfig(
    val loginDefs: LoginDefsConfig = LoginDefsConfig(),
    val limits: List<PAMLimit> = emptyList(),
    val environment: Map<String, String> = emptyMap(),
    val motd: MOTDConfig = MOTDConfig()
)

@Serializable
data class LoginDefsConfig(
    val passMaxDays: Int = 90,
    val passMinDays: Int = 1,
    val passWarnAge: Int = 7,
    val loginRetries: Int = 3,
    val loginTimeout: Duration = 60.minutes,
    val umask: String = "022",
    val homeCreateMode: String = "0755"
)

@Serializable
data class PAMLimit(
    val domain: String,
    val type: PAMLimitType,
    val item: String,
    val value: String
)

@Serializable
data class MOTDConfig(
    val enabled: Boolean = true,
    val dynamicMotd: Boolean = true,
    val showLastLogin: Boolean = true,
    val customMessage: String? = null,
    val includeSystemInfo: Boolean = true
)

@Serializable
data class PasswordPolicyConfig(
    val enabled: Boolean = true,
    val minLength: Int = 8,
    val maxLength: Int = 128,
    val requireLowercase: Boolean = true,
    val requireUppercase: Boolean = true,
    val requireNumbers: Boolean = true,
    val requireSpecialChars: Boolean = true,
    val maxRepeatingChars: Int = 3,
    val dictionary: DictionaryConfig = DictionaryConfig(),
    val complexity: ComplexityConfig = ComplexityConfig()
)

@Serializable
data class DictionaryConfig(
    val enabled: Boolean = true,
    val dictionaryFiles: List<String> = listOf("/usr/share/dict/words"),
    val customWords: List<String> = emptyList(),
    val checkReverse: Boolean = true
)

@Serializable
data class ComplexityConfig(
    val minCharClasses: Int = 3,
    val maxConsecutiveChars: Int = 3,
    val rejectUserInfo: Boolean = true,
    val rejectCommonPasswords: Boolean = true
)

@Serializable
data class TwoFactorConfig(
    val enabled: Boolean = false,
    val method: TwoFactorMethod = TwoFactorMethod.TOTP,
    val issuer: String = "HorizonOS",
    val windowSize: Int = 3,
    val rateLimit: Int = 3,
    val backupCodes: BackupCodeConfig = BackupCodeConfig()
)

@Serializable
data class BackupCodeConfig(
    val enabled: Boolean = true,
    val codeLength: Int = 8,
    val codeCount: Int = 10
)

@Serializable
data class AccountLockoutConfig(
    val enabled: Boolean = true,
    val maxFailedAttempts: Int = 5,
    val lockoutDuration: Duration = 15.minutes,
    val resetCounterTime: Duration = 60.minutes
)

// ===== Enums =====

@Serializable
enum class PAMType {
    AUTH,        // Authentication
    ACCOUNT,     // Account management 
    PASSWORD,    // Password management
    SESSION      // Session management
}

@Serializable
enum class PAMControl {
    REQUIRED,     // Must succeed
    REQUISITE,    // Must succeed, stop on failure
    SUFFICIENT,   // Success satisfies module type
    OPTIONAL,     // Failure is ignored
    INCLUDE,      // Include another PAM file
    SUBSTACK      // Include with isolated stack
}

@Serializable
enum class PAMConditionType {
    USER,         // Username condition
    GROUP,        // Group membership condition  
    TIME,         // Time-based condition
    TTY,          // Terminal condition
    RHOST,        // Remote host condition
    SHELL,        // User shell condition
    SERVICE       // PAM service condition
}

@Serializable
enum class ComparisonOperator {
    EQUALS,       // Exact match
    NOT_EQUALS,   // Does not match
    CONTAINS,     // Contains substring
    STARTS_WITH,  // Starts with pattern
    ENDS_WITH,    // Ends with pattern
    REGEX         // Regular expression match
}

@Serializable
enum class PAMLimitType {
    HARD,         // Hard limit (enforced)
    SOFT          // Soft limit (warning)
}

@Serializable
enum class TwoFactorMethod {
    TOTP,         // Time-based One-Time Password
    HOTP,         // HMAC-based One-Time Password
    SMS,          // SMS verification
    EMAIL,        // Email verification
    HARDWARE_KEY, // Hardware security key
    BACKUP_CODES  // Backup recovery codes
}