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