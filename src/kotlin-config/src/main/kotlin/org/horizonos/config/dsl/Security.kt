package org.horizonos.config.dsl

import kotlinx.serialization.Serializable
import kotlin.time.Duration
import kotlin.time.Duration.Companion.minutes
import kotlin.time.Duration.Companion.hours
import kotlin.time.Duration.Companion.days

/**
 * Security Configuration DSL for HorizonOS
 * 
 * Provides type-safe configuration for security components including
 * PAM, SSH, sudo, SELinux/AppArmor, GPG, and intrusion detection.
 */

// ===== Security Configuration =====

@Serializable
data class SecurityConfig(
    val enabled: Boolean = true,
    val pam: PAMConfig = PAMConfig(),
    val ssh: SSHConfig = SSHConfig(),
    val sudo: SudoConfig = SudoConfig(),
    val selinux: SELinuxConfig = SELinuxConfig(),
    val apparmor: AppArmorConfig = AppArmorConfig(),
    val gpg: GPGConfig = GPGConfig(),
    val fail2ban: Fail2BanConfig = Fail2BanConfig(),
    val firewall: FirewallConfig = FirewallConfig(),
    val audit: AuditConfig = AuditConfig(),
    val tpm: TPMSecurityConfig = TPMSecurityConfig(),
    val certificates: CertificateConfig = CertificateConfig(),
    val compliance: ComplianceConfig = ComplianceConfig()
)

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
    val message: String = "Welcome to HorizonOS",
    val showSystemInfo: Boolean = true,
    val showLastLogin: Boolean = true,
    val customScript: String? = null
)

@Serializable
data class PasswordPolicyConfig(
    val minLength: Int = 12,
    val maxLength: Int = 128,
    val requireUppercase: Boolean = true,
    val requireLowercase: Boolean = true,
    val requireNumbers: Boolean = true,
    val requireSpecial: Boolean = true,
    val maxRepeats: Int = 3,
    val maxSequential: Int = 3,
    val historySize: Int = 12,
    val dictionary: DictionaryConfig = DictionaryConfig(),
    val complexity: ComplexityConfig = ComplexityConfig()
)

@Serializable
data class DictionaryConfig(
    val enabled: Boolean = true,
    val dictionaries: List<String> = listOf("/usr/share/dict/words"),
    val customWords: List<String> = emptyList(),
    val strictMode: Boolean = false
)

@Serializable
data class ComplexityConfig(
    val enabled: Boolean = true,
    val minClasses: Int = 3,
    val maxSimilarChars: Int = 3,
    val palindromeCheck: Boolean = true,
    val usernameCheck: Boolean = true
)

@Serializable
data class TwoFactorConfig(
    val enabled: Boolean = false,
    val method: TwoFactorMethod = TwoFactorMethod.TOTP,
    val backup: BackupCodeConfig = BackupCodeConfig(),
    val grace: Duration = 30.minutes,
    val required: List<String> = emptyList(),
    val exempt: List<String> = emptyList()
)

@Serializable
data class BackupCodeConfig(
    val enabled: Boolean = true,
    val count: Int = 10,
    val length: Int = 8,
    val regenerateAfter: Int = 5
)

@Serializable
data class AccountLockoutConfig(
    val enabled: Boolean = true,
    val maxAttempts: Int = 5,
    val lockoutDuration: Duration = 15.minutes,
    val resetAfter: Duration = 24.hours,
    val adminOverride: Boolean = true,
    val rootExempt: Boolean = true
)

@Serializable
data class SSHConfig(
    val enabled: Boolean = true,
    val port: Int = 22,
    val listenAddress: List<String> = listOf("0.0.0.0"),
    val protocol: SSHProtocol = SSHProtocol.SSH2,
    val authentication: SSHAuthConfig = SSHAuthConfig(),
    val encryption: SSHEncryptionConfig = SSHEncryptionConfig(),
    val access: SSHAccessConfig = SSHAccessConfig(),
    val security: SSHSecurityConfig = SSHSecurityConfig(),
    val forwarding: SSHForwardingConfig = SSHForwardingConfig(),
    val keys: SSHKeyConfig = SSHKeyConfig(),
    val banner: String? = null,
    val logging: SSHLoggingConfig = SSHLoggingConfig()
)

@Serializable
data class SSHAuthConfig(
    val publicKey: Boolean = true,
    val password: Boolean = false,
    val kerberos: Boolean = false,
    val gssapi: Boolean = false,
    val hostbased: Boolean = false,
    val keyboard: Boolean = false,
    val challenge: Boolean = false,
    val maxAuthTries: Int = 3,
    val loginGraceTime: Duration = 2.minutes,
    val permitEmptyPasswords: Boolean = false
)

@Serializable
data class SSHEncryptionConfig(
    val ciphers: List<String> = listOf("aes256-gcm@openssh.com", "aes128-gcm@openssh.com"),
    val macs: List<String> = listOf("hmac-sha2-256-etm@openssh.com", "hmac-sha2-512-etm@openssh.com"),
    val kex: List<String> = listOf("curve25519-sha256@libssh.org", "ecdh-sha2-nistp256"),
    val hostKeyAlgorithms: List<String> = listOf("ssh-ed25519", "ecdsa-sha2-nistp256"),
    val pubkeyAcceptedAlgorithms: List<String> = listOf("ssh-ed25519", "ecdsa-sha2-nistp256")
)

@Serializable
data class SSHAccessConfig(
    val allowUsers: List<String> = emptyList(),
    val allowGroups: List<String> = emptyList(),
    val denyUsers: List<String> = emptyList(),
    val denyGroups: List<String> = emptyList(),
    val permitRoot: RootLoginPolicy = RootLoginPolicy.PROHIBIT_PASSWORD,
    val maxSessions: Int = 10,
    val maxStartups: String = "10:30:100",
    val clientAlive: ClientAliveConfig = ClientAliveConfig()
)

@Serializable
data class ClientAliveConfig(
    val interval: Duration = 5.minutes,
    val maxCount: Int = 3,
    val sendKeepalive: Boolean = true
)

@Serializable
data class SSHSecurityConfig(
    val strictModes: Boolean = true,
    val ignorerhosts: Boolean = true,
    val ignoreUserKnownHosts: Boolean = false,
    val printMotd: Boolean = true,
    val printLastLog: Boolean = true,
    val tcpKeepAlive: Boolean = true,
    val compression: SSHCompression = SSHCompression.DELAYED,
    val useDNS: Boolean = false,
    val verifyReverseMapping: Boolean = false
)

@Serializable
data class SSHForwardingConfig(
    val x11: Boolean = false,
    val agent: Boolean = false,
    val tcp: Boolean = false,
    val streamLocal: Boolean = false,
    val gateway: Boolean = false,
    val permitOpen: List<String> = emptyList(),
    val permitListen: List<String> = emptyList()
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
    val bits: Int? = null,
    val comment: String? = null
)

@Serializable
data class SSHAuthorizedKey(
    val user: String,
    val key: String,
    val keyType: SSHKeyType,
    val comment: String? = null,
    val restrictions: List<String> = emptyList(),
    val from: List<String> = emptyList()
)

@Serializable
data class SSHKnownHost(
    val hostname: String,
    val key: String,
    val keyType: SSHKeyType,
    val port: Int? = null
)

@Serializable
data class SSHKeyGenConfig(
    val autoGenerate: Boolean = true,
    val keyTypes: List<SSHKeyType> = listOf(SSHKeyType.ED25519, SSHKeyType.ECDSA),
    val bits: Map<SSHKeyType, Int> = emptyMap(),
    val regenerateOnBoot: Boolean = false
)

@Serializable
data class SSHLoggingConfig(
    val level: LogLevel = LogLevel.INFO,
    val facility: SyslogFacility = SyslogFacility.AUTH,
    val verboseLogging: Boolean = false,
    val logFile: String? = null
)

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
    val user: String,
    val host: String = "ALL",
    val runAs: String = "ALL",
    val commands: List<String>,
    val options: List<String> = emptyList(),
    val tags: List<SudoTag> = emptyList(),
    val comment: String? = null
)

@Serializable
data class SudoDefaults(
    val requirePassword: Boolean = true,
    val passwordTimeout: Duration = 5.minutes,
    val passwordRetries: Int = 3,
    val logHost: Boolean = true,
    val logYear: Boolean = true,
    val logFile: String = "/var/log/sudo.log",
    val mailBadpass: Boolean = false,
    val mailNoHost: Boolean = false,
    val mailNoUser: Boolean = false,
    val mailNoperms: Boolean = false,
    val mailto: String = "root",
    val secure_path: String = "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
    val env_reset: Boolean = true,
    val env_keep: List<String> = listOf("COLORS", "DISPLAY", "HOSTNAME", "HISTSIZE", "INPUTRC", "KDEDIR", "LS_COLORS", "MAIL", "PS1", "PS2", "QTDIR", "USERNAME", "LANG", "LC_ADDRESS", "LC_CTYPE", "LC_COLLATE", "LC_IDENTIFICATION", "LC_MEASUREMENT", "LC_MESSAGES", "LC_MONETARY", "LC_NAME", "LC_NUMERIC", "LC_PAPER", "LC_TELEPHONE", "LC_TIME", "LC_ALL", "LANGUAGE", "LINGUAS", "_XKB_CHARSET", "XAUTHORITY"),
    val env_delete: List<String> = listOf("IFS", "CDPATH", "LOCALDOMAIN", "RES_OPTIONS", "HOSTALIASES", "NLSPATH", "PATH_LOCALE", "TERMINFO", "TERMINFO_DIRS", "TERMPATH")
)

@Serializable
data class SudoAliases(
    val users: Map<String, List<String>> = emptyMap(),
    val hosts: Map<String, List<String>> = emptyMap(),
    val commands: Map<String, List<String>> = emptyMap(),
    val runAs: Map<String, List<String>> = emptyMap()
)

@Serializable
data class SudoSecurity(
    val requireTty: Boolean = false,
    val visiblepw: Boolean = false,
    val rootpw: Boolean = false,
    val runaspw: Boolean = false,
    val targetpw: Boolean = false,
    val set_home: Boolean = false,
    val always_set_home: Boolean = false,
    val set_logname: Boolean = true,
    val stay_setuid: Boolean = false,
    val preserve_groups: Boolean = false,
    val use_loginclass: Boolean = false
)

@Serializable
data class SudoLogging(
    val enabled: Boolean = true,
    val logHost: Boolean = true,
    val logYear: Boolean = true,
    val logFile: String = "/var/log/sudo.log",
    val maxLogSize: String = "10M",
    val ioLog: Boolean = false,
    val ioLogDir: String = "/var/log/sudo-io",
    val ioLogFile: String = "%{seq}",
    val ioLogCompress: Boolean = false,
    val ioLogFlush: Boolean = false,
    val syslog: Boolean = true,
    val syslogFacility: SyslogFacility = SyslogFacility.AUTHPRIV,
    val syslogGoodbad: Boolean = true
)

@Serializable
data class SELinuxConfig(
    val enabled: Boolean = false,
    val mode: SELinuxMode = SELinuxMode.ENFORCING,
    val policy: String = "targeted",
    val booleans: Map<String, Boolean> = emptyMap(),
    val modules: List<SELinuxModule> = emptyList(),
    val contexts: List<SELinuxContext> = emptyList(),
    val users: List<SELinuxUser> = emptyList(),
    val ports: List<SELinuxPort> = emptyList(),
    val files: List<SELinuxFile> = emptyList(),
    val audit: Boolean = true,
    val autorelabel: Boolean = false
)

@Serializable
data class SELinuxModule(
    val name: String,
    val enabled: Boolean = true,
    val priority: Int = 400,
    val source: String? = null,
    val package: String? = null
)

@Serializable
data class SELinuxContext(
    val name: String,
    val user: String,
    val role: String,
    val type: String,
    val range: String? = null
)

@Serializable
data class SELinuxUser(
    val name: String,
    val roles: List<String>,
    val range: String? = null,
    val default: Boolean = false
)

@Serializable
data class SELinuxPort(
    val port: Int,
    val protocol: NetworkProtocol,
    val type: String,
    val range: String? = null
)

@Serializable
data class SELinuxFile(
    val path: String,
    val type: String,
    val recursive: Boolean = false,
    val user: String? = null,
    val role: String? = null,
    val range: String? = null
)

@Serializable
data class AppArmorConfig(
    val enabled: Boolean = false,
    val profiles: List<AppArmorProfile> = emptyList(),
    val complain: List<String> = emptyList(),
    val enforce: List<String> = emptyList(),
    val disable: List<String> = emptyList(),
    val abstractions: List<AppArmorAbstraction> = emptyList(),
    val tunables: Map<String, String> = emptyMap(),
    val audit: Boolean = true,
    val cacheDir: String = "/var/cache/apparmor"
)

@Serializable
data class AppArmorProfile(
    val name: String,
    val path: String,
    val mode: AppArmorMode = AppArmorMode.ENFORCE,
    val rules: List<AppArmorRule> = emptyList(),
    val includes: List<String> = emptyList(),
    val attachments: List<String> = emptyList()
)

@Serializable
data class AppArmorRule(
    val type: AppArmorRuleType,
    val path: String,
    val permissions: List<String> = emptyList(),
    val qualifiers: List<String> = emptyList(),
    val comment: String? = null
)

@Serializable
data class AppArmorAbstraction(
    val name: String,
    val path: String,
    val enabled: Boolean = true
)

@Serializable
data class GPGConfig(
    val enabled: Boolean = true,
    val keyserver: String = "hkps://keys.openpgp.org",
    val keyserverOptions: List<String> = listOf("honor-keyserver-url", "include-revoked"),
    val defaultKey: String? = null,
    val defaultRecipient: String? = null,
    val trustModel: GPGTrustModel = GPGTrustModel.PGP,
    val keys: List<GPGKey> = emptyList(),
    val keyrings: List<String> = emptyList(),
    val cipherPrefs: List<String> = listOf("AES256", "AES192", "AES", "CAST5"),
    val digestPrefs: List<String> = listOf("SHA512", "SHA384", "SHA256", "SHA224"),
    val compressPrefs: List<String> = listOf("ZLIB", "BZIP2", "ZIP", "Uncompressed"),
    val agent: GPGAgentConfig = GPGAgentConfig()
)

@Serializable
data class GPGKey(
    val keyId: String,
    val fingerprint: String,
    val userId: String,
    val trustLevel: GPGTrustLevel,
    val keyFile: String? = null,
    val revoked: Boolean = false,
    val expired: Boolean = false
)

@Serializable
data class GPGAgentConfig(
    val enabled: Boolean = true,
    val maxCacheTtl: Duration = 2.hours,
    val defaultCacheTtl: Duration = 10.minutes,
    val maxCacheTtlSsh: Duration = 2.hours,
    val defaultCacheTtlSsh: Duration = 10.minutes,
    val pinentryProgram: String = "/usr/bin/pinentry-gtk-2",
    val allowPresetPassphrase: Boolean = true,
    val allowLoopbackPinentry: Boolean = true,
    val grabKeyboard: Boolean = true,
    val ssh: Boolean = false
)

@Serializable
data class Fail2BanConfig(
    val enabled: Boolean = true,
    val jails: List<Fail2BanJail> = emptyList(),
    val ignoreip: List<String> = listOf("127.0.0.1/8", "::1"),
    val bantime: Duration = 10.minutes,
    val findtime: Duration = 10.minutes,
    val maxretry: Int = 5,
    val backend: Fail2BanBackend = Fail2BanBackend.SYSTEMD,
    val usedns: UseDNSPolicy = UseDNSPolicy.WARN,
    val logLevel: LogLevel = LogLevel.INFO,
    val logTarget: String = "/var/log/fail2ban.log",
    val socket: String = "/var/run/fail2ban/fail2ban.sock",
    val pidfile: String = "/var/run/fail2ban/fail2ban.pid"
)

@Serializable
data class Fail2BanJail(
    val name: String,
    val enabled: Boolean = true,
    val filter: String,
    val logpath: List<String>,
    val action: String = "iptables-multiport",
    val port: String = "ssh",
    val protocol: String = "tcp",
    val bantime: Duration? = null,
    val findtime: Duration? = null,
    val maxretry: Int? = null,
    val ignoreip: List<String> = emptyList(),
    val banaction: String? = null,
    val mta: String = "sendmail",
    val destemail: String? = null,
    val sender: String? = null,
    val usedns: UseDNSPolicy? = null
)

@Serializable
data class FirewallConfig(
    val enabled: Boolean = true,
    val backend: FirewallBackend = FirewallBackend.IPTABLES,
    val defaultPolicy: FirewallPolicy = FirewallPolicy.DROP,
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
    val source: String? = null,
    val destination: String? = null,
    val port: String? = null,
    val protocol: NetworkProtocol? = null,
    val action: FirewallAction,
    val interface: String? = null,
    val state: List<ConnectionState> = emptyList(),
    val comment: String? = null,
    val priority: Int = 0
)

@Serializable
data class FirewallChain(
    val name: String,
    val table: String = "filter",
    val policy: FirewallPolicy = FirewallPolicy.DROP,
    val rules: List<String> = emptyList()
)

@Serializable
data class FirewallZone(
    val name: String,
    val interfaces: List<String> = emptyList(),
    val sources: List<String> = emptyList(),
    val services: List<String> = emptyList(),
    val ports: List<String> = emptyList(),
    val masquerade: Boolean = false,
    val forward: Boolean = false,
    val target: FirewallPolicy = FirewallPolicy.DROP
)

@Serializable
data class FirewallService(
    val name: String,
    val ports: List<String>,
    val protocols: List<NetworkProtocol> = emptyList(),
    val modules: List<String> = emptyList(),
    val destinations: List<String> = emptyList()
)

@Serializable
data class FirewallLogging(
    val enabled: Boolean = true,
    val level: LogLevel = LogLevel.INFO,
    val prefix: String = "FIREWALL: ",
    val rateLimit: String = "10/min",
    val burst: Int = 5
)

@Serializable
data class AuditConfig(
    val enabled: Boolean = true,
    val rules: List<AuditRule> = emptyList(),
    val bufferSize: Int = 8192,
    val maxLogFile: Int = 8,
    val maxLogFileAction: AuditAction = AuditAction.ROTATE,
    val spaceLeft: Int = 75,
    val spaceLeftAction: AuditAction = AuditAction.SYSLOG,
    val adminSpaceLeft: Int = 50,
    val adminSpaceLeftAction: AuditAction = AuditAction.SUSPEND,
    val diskFull: AuditAction = AuditAction.SUSPEND,
    val diskError: AuditAction = AuditAction.SUSPEND,
    val tcpListenPort: Int? = null,
    val tcpListenQueue: Int = 5,
    val tcpMaxPerAddr: Int = 1,
    val tcpClientPorts: String? = null,
    val tcpClientMaxIdle: Int = 0,
    val enableKrb5: Boolean = false,
    val krb5Principal: String? = null,
    val krb5KeyTab: String? = null,
    val distributeNetwork: Boolean = false,
    val logFormat: AuditLogFormat = AuditLogFormat.RAW,
    val logGroup: String = "root",
    val priority: Int = 4,
    val disp_qos: AuditQOS = AuditQOS.LOSSY,
    val dispatcher: String = "/sbin/audispd",
    val nameFormat: AuditNameFormat = AuditNameFormat.NONE,
    val name: String? = null
)

@Serializable
data class AuditRule(
    val name: String,
    val rule: String,
    val syscalls: List<String> = emptyList(),
    val fields: List<String> = emptyList(),
    val keys: List<String> = emptyList(),
    val enabled: Boolean = true
)

@Serializable
data class TPMSecurityConfig(
    val enabled: Boolean = false,
    val version: TPMVersion = TPMVersion.TPM2,
    val ownership: TPMOwnership = TPMOwnership(),
    val pcr: TPMPCRConfig = TPMPCRConfig(),
    val ima: TPMIMAConfig = TPMIMAConfig(),
    val attestation: TPMAttestationConfig = TPMAttestationConfig()
)

@Serializable
data class TPMOwnership(
    val takeOwnership: Boolean = true,
    val ownerPassword: String? = null,
    val srkPassword: String? = null,
    val clearLockout: Boolean = true
)

@Serializable
data class TPMPCRConfig(
    val extend: List<TPMPCRExtend> = emptyList(),
    val quote: List<Int> = emptyList(),
    val seal: List<Int> = emptyList()
)

@Serializable
data class TPMPCRExtend(
    val pcr: Int,
    val digest: String,
    val event: String? = null
)

@Serializable
data class TPMIMAConfig(
    val enabled: Boolean = false,
    val policy: String = "tcb",
    val template: String = "ima-ng",
    val hash: String = "sha256"
)

@Serializable
data class TPMAttestationConfig(
    val enabled: Boolean = false,
    val keyType: String = "rsa2048",
    val nameAlg: String = "sha256",
    val signAlg: String = "rsassa"
)

@Serializable
data class CertificateConfig(
    val enabled: Boolean = true,
    val ca: CAConfig = CAConfig(),
    val certificates: List<Certificate> = emptyList(),
    val store: CertificateStore = CertificateStore(),
    val autoRenewal: AutoRenewalConfig = AutoRenewalConfig()
)

@Serializable
data class CAConfig(
    val enabled: Boolean = false,
    val path: String = "/etc/ssl/certs",
    val keyPath: String = "/etc/ssl/private",
    val country: String = "US",
    val state: String = "California",
    val locality: String = "San Francisco",
    val organization: String = "HorizonOS",
    val organizationUnit: String = "IT Department",
    val commonName: String = "HorizonOS CA",
    val validity: Duration = 3650.days,
    val keySize: Int = 4096,
    val digest: String = "sha256"
)

@Serializable
data class Certificate(
    val name: String,
    val commonName: String,
    val subjectAltNames: List<String> = emptyList(),
    val keyFile: String,
    val certFile: String,
    val caFile: String? = null,
    val keySize: Int = 2048,
    val validity: Duration = 365.days,
    val autoRenew: Boolean = true,
    val ocspStapling: Boolean = false,
    val hsts: Boolean = false
)

@Serializable
data class CertificateStore(
    val path: String = "/etc/ssl/certs",
    val caBundle: String = "/etc/ssl/certs/ca-certificates.crt",
    val updateCommand: String = "update-ca-certificates",
    val rehashCommand: String = "c_rehash"
)

@Serializable
data class AutoRenewalConfig(
    val enabled: Boolean = true,
    val renewBefore: Duration = 30.days,
    val checkInterval: Duration = 24.hours,
    val retryInterval: Duration = 1.hours,
    val maxRetries: Int = 3
)

@Serializable
data class ComplianceConfig(
    val enabled: Boolean = false,
    val frameworks: List<ComplianceFramework> = emptyList(),
    val scanning: ComplianceScanConfig = ComplianceScanConfig(),
    val reporting: ComplianceReportConfig = ComplianceReportConfig(),
    val remediation: ComplianceRemediationConfig = ComplianceRemediationConfig()
)

@Serializable
data class ComplianceFramework(
    val name: String,
    val version: String,
    val profiles: List<String> = emptyList(),
    val enabled: Boolean = true
)

@Serializable
data class ComplianceScanConfig(
    val enabled: Boolean = true,
    val schedule: String = "0 2 * * 0", // Weekly
    val profiles: List<String> = emptyList(),
    val remediate: Boolean = false
)

@Serializable
data class ComplianceReportConfig(
    val enabled: Boolean = true,
    val format: List<ReportFormat> = listOf(ReportFormat.HTML),
    val outputDir: String = "/var/log/compliance",
    val retention: Duration = 90.days
)

@Serializable
data class ComplianceRemediationConfig(
    val enabled: Boolean = false,
    val autoRemediate: Boolean = false,
    val confirmBeforeRemediation: Boolean = true,
    val backupBeforeRemediation: Boolean = true
)

// ===== Enums =====

@Serializable
enum class PAMType {
    ACCOUNT,
    AUTH,
    PASSWORD,
    SESSION
}

@Serializable
enum class PAMControl {
    REQUIRED,
    REQUISITE,
    SUFFICIENT,
    OPTIONAL,
    INCLUDE,
    SUBSTACK
}

@Serializable
enum class PAMConditionType {
    USER,
    GROUP,
    HOST,
    TTY,
    RHOST,
    SERVICE,
    TIME,
    AUDIT
}

@Serializable
enum class ComparisonOperator {
    EQUALS,
    NOT_EQUALS,
    CONTAINS,
    NOT_CONTAINS,
    REGEX,
    NOT_REGEX
}

@Serializable
enum class PAMLimitType {
    SOFT,
    HARD
}

@Serializable
enum class TwoFactorMethod {
    TOTP,
    SMS,
    EMAIL,
    HARDWARE,
    PUSH
}

@Serializable
enum class SSHProtocol {
    SSH1,
    SSH2
}

@Serializable
enum class RootLoginPolicy {
    YES,
    NO,
    PROHIBIT_PASSWORD,
    FORCED_COMMANDS_ONLY
}

@Serializable
enum class SSHCompression {
    YES,
    NO,
    DELAYED
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
    DEBUG,
    INFO,
    NOTICE,
    WARNING,
    ERROR,
    CRITICAL
}

@Serializable
enum class SyslogFacility {
    AUTH,
    AUTHPRIV,
    DAEMON,
    KERN,
    LPR,
    MAIL,
    NEWS,
    SYSLOG,
    USER,
    UUCP,
    LOCAL0,
    LOCAL1,
    LOCAL2,
    LOCAL3,
    LOCAL4,
    LOCAL5,
    LOCAL6,
    LOCAL7
}

@Serializable
enum class SudoTag {
    NOPASSWD,
    PASSWD,
    NOEXEC,
    EXEC,
    SETENV,
    NOSETENV,
    LOG_INPUT,
    NOLOG_INPUT,
    LOG_OUTPUT,
    NOLOG_OUTPUT
}

@Serializable
enum class SELinuxMode {
    DISABLED,
    PERMISSIVE,
    ENFORCING
}

@Serializable
enum class NetworkProtocol {
    TCP,
    UDP,
    ICMP,
    ALL
}

@Serializable
enum class AppArmorMode {
    ENFORCE,
    COMPLAIN,
    DISABLE
}

@Serializable
enum class AppArmorRuleType {
    FILE,
    CAPABILITY,
    NETWORK,
    MOUNT,
    SIGNAL,
    PTRACE,
    UNIX,
    DBUS
}

@Serializable
enum class GPGTrustModel {
    PGP,
    CLASSIC,
    DIRECT,
    ALWAYS,
    AUTO
}

@Serializable
enum class GPGTrustLevel {
    UNKNOWN,
    EXPIRED,
    UNDEFINED,
    NEVER,
    MARGINAL,
    FULL,
    ULTIMATE
}

@Serializable
enum class Fail2BanBackend {
    AUTO,
    PYINOTIFY,
    GAMIN,
    POLLING,
    SYSTEMD
}

@Serializable
enum class UseDNSPolicy {
    YES,
    NO,
    WARN,
    RAW
}

@Serializable
enum class FirewallBackend {
    IPTABLES,
    NFTABLES,
    FIREWALLD,
    UFW
}

@Serializable
enum class FirewallPolicy {
    ACCEPT,
    DROP,
    REJECT
}

@Serializable
enum class FirewallAction {
    ACCEPT,
    DROP,
    REJECT,
    LOG,
    RETURN
}

@Serializable
enum class ConnectionState {
    NEW,
    ESTABLISHED,
    RELATED,
    INVALID,
    UNTRACKED
}

@Serializable
enum class AuditAction {
    IGNORE,
    SYSLOG,
    SUSPEND,
    ROTATE,
    KEEP_LOGS,
    HALT,
    SINGLE
}

@Serializable
enum class AuditLogFormat {
    RAW,
    NOLOG
}

@Serializable
enum class AuditQOS {
    LOSSY,
    LOSSLESS
}

@Serializable
enum class AuditNameFormat {
    NONE,
    HOSTNAME,
    FQD,
    NUMERIC,
    USER
}

@Serializable
enum class ReportFormat {
    HTML,
    XML,
    JSON,
    CSV,
    PDF
}

// ===== DSL Builders =====

@HorizonOSDsl
class SecurityContext {
    var enabled: Boolean = true
    private var pam = PAMConfig()
    private var ssh = SSHConfig()
    private var sudo = SudoConfig()
    private var selinux = SELinuxConfig()
    private var apparmor = AppArmorConfig()
    private var gpg = GPGConfig()
    private var fail2ban = Fail2BanConfig()
    private var firewall = FirewallConfig()
    private var audit = AuditConfig()
    private var tpm = TPMSecurityConfig()
    private var certificates = CertificateConfig()
    private var compliance = ComplianceConfig()
    
    fun pam(block: PAMContext.() -> Unit) {
        pam = PAMContext().apply(block).toConfig()
    }
    
    fun ssh(block: SSHContext.() -> Unit) {
        ssh = SSHContext().apply(block).toConfig()
    }
    
    fun sudo(block: SudoContext.() -> Unit) {
        sudo = SudoContext().apply(block).toConfig()
    }
    
    fun selinux(block: SELinuxContext.() -> Unit) {
        selinux = SELinuxContext().apply(block).toConfig()
    }
    
    fun apparmor(block: AppArmorContext.() -> Unit) {
        apparmor = AppArmorContext().apply(block).toConfig()
    }
    
    fun gpg(block: GPGContext.() -> Unit) {
        gpg = GPGContext().apply(block).toConfig()
    }
    
    fun fail2ban(block: Fail2BanContext.() -> Unit) {
        fail2ban = Fail2BanContext().apply(block).toConfig()
    }
    
    fun firewall(block: FirewallContext.() -> Unit) {
        firewall = FirewallContext().apply(block).toConfig()
    }
    
    fun audit(block: AuditContext.() -> Unit) {
        audit = AuditContext().apply(block).toConfig()
    }
    
    fun tpm(block: TPMSecurityContext.() -> Unit) {
        tpm = TPMSecurityContext().apply(block).toConfig()
    }
    
    fun certificates(block: CertificateContext.() -> Unit) {
        certificates = CertificateContext().apply(block).toConfig()
    }
    
    fun compliance(block: ComplianceContext.() -> Unit) {
        compliance = ComplianceContext().apply(block).toConfig()
    }
    
    fun toConfig() = SecurityConfig(
        enabled = enabled,
        pam = pam,
        ssh = ssh,
        sudo = sudo,
        selinux = selinux,
        apparmor = apparmor,
        gpg = gpg,
        fail2ban = fail2ban,
        firewall = firewall,
        audit = audit,
        tpm = tpm,
        certificates = certificates,
        compliance = compliance
    )
}

// Placeholder context classes for comprehensive DSL structure
@HorizonOSDsl class PAMContext { fun toConfig() = PAMConfig() }
@HorizonOSDsl class SSHContext { fun toConfig() = SSHConfig() }
@HorizonOSDsl class SudoContext { fun toConfig() = SudoConfig() }
@HorizonOSDsl class SELinuxContext { fun toConfig() = SELinuxConfig() }
@HorizonOSDsl class AppArmorContext { fun toConfig() = AppArmorConfig() }
@HorizonOSDsl class GPGContext { fun toConfig() = GPGConfig() }
@HorizonOSDsl class Fail2BanContext { fun toConfig() = Fail2BanConfig() }
@HorizonOSDsl class FirewallContext { fun toConfig() = FirewallConfig() }
@HorizonOSDsl class AuditContext { fun toConfig() = AuditConfig() }
@HorizonOSDsl class TPMSecurityContext { fun toConfig() = TPMSecurityConfig() }
@HorizonOSDsl class CertificateContext { fun toConfig() = CertificateConfig() }
@HorizonOSDsl class ComplianceContext { fun toConfig() = ComplianceConfig() }

// ===== Extension Functions =====

fun CompiledConfig.hasSecurity(): Boolean = security != null

fun CompiledConfig.getSSHConfig(): SSHConfig? = security?.ssh

fun CompiledConfig.getSudoRules(): List<SudoRule> = security?.sudo?.rules ?: emptyList()

fun CompiledConfig.getFirewallRules(): List<FirewallRule> = security?.firewall?.rules ?: emptyList()

fun CompiledConfig.isSelinuxEnabled(): Boolean = security?.selinux?.enabled == true

fun CompiledConfig.isAppArmorEnabled(): Boolean = security?.apparmor?.enabled == true

fun CompiledConfig.getFail2BanJails(): List<Fail2BanJail> = security?.fail2ban?.jails ?: emptyList()

fun CompiledConfig.getGPGKeys(): List<GPGKey> = security?.gpg?.keys ?: emptyList()

fun CompiledConfig.getAuditRules(): List<AuditRule> = security?.audit?.rules ?: emptyList()

fun CompiledConfig.isTpmEnabled(): Boolean = security?.tpm?.enabled == true

fun CompiledConfig.getCertificates(): List<Certificate> = security?.certificates?.certificates ?: emptyList()

fun CompiledConfig.getComplianceFrameworks(): List<ComplianceFramework> = security?.compliance?.frameworks ?: emptyList()