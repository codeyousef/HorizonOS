package org.horizonos.config.dsl

import kotlinx.serialization.Serializable
import org.horizonos.config.dsl.security.*

/**
 * Security Configuration DSL for HorizonOS
 * 
 * Provides comprehensive security configuration for HorizonOS systems including authentication,
 * authorization, access control, network security, and compliance frameworks. This module
 * implements defense-in-depth security principles with multiple layers of protection.
 * 
 * ## Security Components:
 * - **Authentication**: PAM (Pluggable Authentication Modules) configuration
 * - **Remote Access**: SSH server and client configuration with key management
 * - **Authorization**: sudo privileges and access control policies
 * - **Mandatory Access Control**: SELinux and AppArmor policy configuration
 * - **Network Security**: Firewall rules and intrusion detection (fail2ban)
 * - **Cryptography**: GPG key management and certificate handling
 * - **Audit**: System audit logging and compliance monitoring
 * - **Hardware Security**: TPM (Trusted Platform Module) integration
 * - **Compliance**: NIST, CIS, and other security framework compliance
 * 
 * ## Basic Usage:
 * ```kotlin
 * security {
 *     enabled = true
 *     
 *     firewall {
 *         enabled = true
 *         defaultPolicy = FirewallPolicy.DROP
 *         
 *         rule {
 *             port = 22
 *             protocol = "tcp"
 *             action = FirewallAction.ACCEPT
 *             comment = "SSH access"
 *         }
 *     }
 *     
 *     ssh {
 *         enabled = true
 *         passwordAuthentication = false
 *         keyBasedAuthentication = true
 *         port = 22
 *     }
 *     
 *     sudo {
 *         enabled = true
 *         requirePassword = true
 *         timestampTimeout = 15
 *     }
 * }
 * ```
 * 
 * ## Security Best Practices:
 * - **Principle of Least Privilege**: Users and services run with minimum required permissions
 * - **Defense in Depth**: Multiple security layers protect against different attack vectors
 * - **Secure by Default**: Conservative defaults that can be relaxed as needed
 * - **Audit Trail**: All security-relevant events are logged and monitored
 * - **Regular Updates**: Security policies and rules are kept current
 * 
 * ## Advanced Usage Examples:
 * 
 * ### 1. **Production Server Security**
 * ```kotlin
 * security {
 *     enabled = true
 *     
 *     // Strict firewall configuration
 *     firewall {
 *         enabled = true
 *         defaultPolicy = FirewallPolicy.DROP
 *         
 *         // Only allow specific services
 *         rule {
 *             port = 22
 *             protocol = "tcp"
 *             source = "192.168.1.0/24"  // Restrict SSH to local network
 *             action = FirewallAction.ACCEPT
 *         }
 *         
 *         rule {
 *             port = 443
 *             protocol = "tcp"
 *             action = FirewallAction.ACCEPT
 *             rateLimit = "10/minute"
 *         }
 *     }
 *     
 *     // Hardened SSH configuration
 *     ssh {
 *         enabled = true
 *         port = 2222  // Non-standard port
 *         passwordAuth = false
 *         rootLogin = RootLoginPolicy.NO
 *         maxAuthTries = 2
 *         
 *         encryption {
 *             ciphers = listOf("chacha20-poly1305@openssh.com")
 *             kexAlgorithms = listOf("curve25519-sha256")
 *         }
 *     }
 *     
 *     // Strict sudo configuration
 *     sudo {
 *         enabled = true
 *         requirePassword = true
 *         timestampTimeout = 5  // Short timeout
 *         logCommands = true
 *     }
 * }
 * ```
 * 
 * ### 2. **Development Environment Security**
 * ```kotlin
 * security {
 *     enabled = true
 *     
 *     // More permissive for development
 *     firewall {
 *         enabled = true
 *         defaultPolicy = FirewallPolicy.ACCEPT
 *         
 *         // Block suspicious ports
 *         rule {
 *             port = 23  // Telnet
 *             action = FirewallAction.DROP
 *         }
 *     }
 *     
 *     ssh {
 *         enabled = true
 *         passwordAuth = true  // Allow for development
 *         rootLogin = RootLoginPolicy.PROHIBIT_PASSWORD
 *     }
 *     
 *     // Development-friendly sudo
 *     sudo {
 *         enabled = true
 *         requirePassword = false  // For development convenience
 *         timestampTimeout = 60
 *     }
 * }
 * ```
 * 
 * ### 3. **Multi-User System Security**
 * ```kotlin
 * security {
 *     enabled = true
 *     
 *     // User-specific access controls
 *     pam {
 *         enabled = true
 *         
 *         // Limit login attempts
 *         faillock {
 *             enabled = true
 *             maxTries = 3
 *             unlockTime = 1800  // 30 minutes
 *         }
 *         
 *         // Password requirements
 *         pwquality {
 *             minlen = 12
 *             minclass = 3
 *             maxrepeat = 2
 *         }
 *     }
 *     
 *     // Group-based SSH access
 *     ssh {
 *         enabled = true
 *         
 *         access {
 *             allowGroups = listOf("ssh-users", "developers")
 *             denyUsers = listOf("guest", "ftp")
 *         }
 *     }
 * }
 * ```
 * 
 * @since 1.0
 * @see [PAMConfig] for authentication configuration
 * @see [SSHConfig] for remote access configuration
 * @see [FirewallConfig] for network security configuration
 * @see [Network] for network interface configuration
 * @see [NetworkConfig] for network services and connectivity
 * @see [Services] for system services and daemon management
 * @see [Boot] for secure boot configuration
 * @see [Hardware] for hardware security features
 * @see [Storage] for encrypted storage configuration
 * @see [horizonOS] for main system configuration entry point
 */

// ===== Main Security Configuration =====

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

// ===== DSL Builder =====

@HorizonOSDsl
class SecurityContext {
    private var enabled = true
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
    
    fun ssh(block: SSHContext.() -> Unit) {
        ssh = SSHContext().apply(block).toConfig()
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

@HorizonOSDsl
class SSHContext {
    var enabled: Boolean = true
    var port: Int = 22
    
    fun toConfig() = SSHConfig(enabled = enabled, port = port)
}