package org.horizonos.config.dsl

import kotlinx.serialization.Serializable
import org.horizonos.config.dsl.security.*

/**
 * Security Configuration DSL for HorizonOS
 * 
 * Provides type-safe configuration for security components including
 * PAM, SSH, sudo, SELinux/AppArmor, GPG, and intrusion detection.
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