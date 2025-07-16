package org.horizonos.config.validation.validators

import org.horizonos.config.dsl.*
import org.horizonos.config.dsl.security.*
import org.horizonos.config.validation.ValidationError

object SecurityValidator {
    
    fun validateSecurityConfig(security: SecurityConfig): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        // Validate SSH configuration
        errors.addAll(validateSSHConfig(security.ssh))
        
        // Validate sudo configuration
        errors.addAll(validateSudoConfig(security.sudo))
        
        // Validate PAM configuration
        errors.addAll(validatePAMConfig(security.pam))
        
        // Validate firewall configuration
        errors.addAll(validateFirewallConfig(security.firewall))
        
        // Validate GPG configuration
        errors.addAll(validateGPGConfig(security.gpg))
        
        // Validate audit configuration
        errors.addAll(validateAuditConfig(security.audit))
        
        // Validate AppArmor configuration
        errors.addAll(validateAppArmorConfig(security.apparmor))
        
        // Validate SELinux configuration
        errors.addAll(validateSELinuxConfig(security.selinux))
        
        // Validate certificates
        errors.addAll(validateCertificateConfig(security.certificates))
        
        // Validate compliance configuration
        errors.addAll(validateComplianceConfig(security.compliance))
        
        return errors
    }
    
    private fun validateSSHConfig(ssh: SSHConfig): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        // Validate SSH port
        if (ssh.port < 1 || ssh.port > 65535) {
            errors.add(ValidationError.InvalidSSHPort(ssh.port))
        }
        
        // Validate ciphers
        ssh.encryption.ciphers.forEach { cipher ->
            if (!isValidSSHCipher(cipher)) {
                errors.add(ValidationError.InvalidSSHCipher(cipher))
            }
        }
        
        // Validate key exchange algorithms
        ssh.encryption.kexAlgorithms.forEach { kex ->
            if (!isValidSSHKex(kex)) {
                errors.add(ValidationError.InvalidSSHCipher("Invalid key exchange algorithm: $kex"))
            }
        }
        
        // Validate MAC algorithms
        ssh.encryption.macs.forEach { mac ->
            if (!isValidSSHMac(mac)) {
                errors.add(ValidationError.InvalidSSHCipher("Invalid MAC algorithm: $mac"))
            }
        }
        
        // Validate allowed users/groups
        ssh.access.allowUsers.forEach { user ->
            if (!isValidUsername(user)) {
                errors.add(ValidationError.InvalidUsername(user))
            }
        }
        
        ssh.access.allowGroups.forEach { group ->
            if (!isValidGroupName(group)) {
                errors.add(ValidationError.InvalidGroupName(group))
            }
        }
        
        return errors
    }
    
    private fun validateSudoConfig(sudo: SudoConfig): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        // Validate sudo rules
        sudo.rules.forEach { rule ->
            // Validate users
            rule.users.forEach { user ->
                if (!isValidUsername(user)) {
                    errors.add(ValidationError.InvalidUsername(user))
                }
            }
            
            // Validate groups
            rule.groups.forEach { group ->
                if (!isValidGroupName(group)) {
                    errors.add(ValidationError.InvalidGroupName(group))
                }
            }
            
            // Validate commands
            rule.commands.forEach { command ->
                if (!isValidCommand(command)) {
                    errors.add(ValidationError.InvalidSudoRule("Invalid command: $command"))
                }
            }
        }
        
        return errors
    }
    
    private fun validatePAMConfig(pam: PAMConfig): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        // Validate PAM modules
        pam.modules.forEach { module ->
            if (!isValidPAMModule(module.module)) {
                errors.add(ValidationError.InvalidPAMModule(module.module))
            }
        }
        
        // Validate PAM rules
        pam.rules.forEach { rule ->
            if (!isValidPAMModule(rule.module)) {
                errors.add(ValidationError.InvalidPAMModule(rule.module))
            }
        }
        
        // Validate account lockout
        if (pam.lockout.maxFailedAttempts <= 0) {
            errors.add(ValidationError.InvalidPAMModule("Max failed attempts must be positive"))
        }
        
        return errors
    }
    
    private fun validateFirewallConfig(firewall: FirewallConfig): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        // Validate firewall rules
        firewall.rules.forEach { rule ->
            // Validate port numbers
            rule.dport?.let { port ->
                if (!isValidPortRange(port)) {
                    errors.add(ValidationError.InvalidFirewallRule("Invalid destination port: $port"))
                }
            }
            
            rule.sport?.let { port ->
                if (!isValidPortRange(port)) {
                    errors.add(ValidationError.InvalidFirewallRule("Invalid source port: $port"))
                }
            }
            
            // Validate IP addresses
            rule.source?.let { source ->
                if (!isValidIPAddress(source)) {
                    errors.add(ValidationError.InvalidFirewallRule("Invalid source IP: $source"))
                }
            }
            
            rule.destination?.let { dest ->
                if (!isValidIPAddress(dest)) {
                    errors.add(ValidationError.InvalidFirewallRule("Invalid destination IP: $dest"))
                }
            }
        }
        
        return errors
    }
    
    private fun validateGPGConfig(gpg: GPGConfig): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        // Validate key IDs
        gpg.keys.forEach { key ->
            if (!isValidGPGKeyId(key.keyId)) {
                errors.add(ValidationError.InvalidGPGKeyId(key.keyId))
            }
        }
        
        // Trust levels are already validated by the enum type, no additional validation needed
        
        return errors
    }
    
    private fun validateAuditConfig(audit: AuditConfig): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        // Validate audit rules
        audit.rules.forEach { rule ->
            if (!isValidAuditRule(rule.rule)) {
                errors.add(ValidationError.InvalidAuditRule(rule.rule))
            }
        }
        
        // Validate buffer size
        if (audit.bufferSize < 64) {
            errors.add(ValidationError.InvalidAuditRule("Buffer size too small"))
        }
        
        return errors
    }
    
    private fun validateAppArmorConfig(apparmor: AppArmorConfig): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        // AppArmor validation
        apparmor.profiles.forEach { profile ->
            if (!isValidPath(profile.path)) {
                errors.add(ValidationError.InvalidPath(profile.path))
            }
        }
        
        return errors
    }
    
    private fun validateSELinuxConfig(selinux: SELinuxConfig): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        // SELinux validation if needed
        
        return errors
    }
    
    private fun validateCertificateConfig(certs: CertificateConfig): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        // Validate trusted certificates
        certs.trustedCerts.forEach { cert ->
            if (!isValidPath(cert.path)) {
                errors.add(ValidationError.InvalidCertificatePath(cert.path))
            }
        }
        
        // Validate CA certificates
        certs.caCerts.forEach { ca ->
            if (ca.certificate.isBlank()) {
                errors.add(ValidationError.InvalidCertificatePath("CA certificate cannot be empty"))
            }
        }
        
        return errors
    }
    
    private fun validateComplianceConfig(compliance: ComplianceConfig): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        // Compliance validation if needed
        
        return errors
    }
    
    // Helper validation functions
    private fun isValidSSHCipher(cipher: String): Boolean {
        val validCiphers = setOf(
            "aes128-ctr", "aes192-ctr", "aes256-ctr",
            "aes128-gcm@openssh.com", "aes256-gcm@openssh.com",
            "chacha20-poly1305@openssh.com"
        )
        return validCiphers.contains(cipher)
    }
    
    private fun isValidSSHKex(kex: String): Boolean {
        val validKex = setOf(
            "curve25519-sha256", "curve25519-sha256@libssh.org",
            "ecdh-sha2-nistp256", "ecdh-sha2-nistp384", "ecdh-sha2-nistp521",
            "diffie-hellman-group-exchange-sha256"
        )
        return validKex.contains(kex)
    }
    
    private fun isValidSSHMac(mac: String): Boolean {
        val validMacs = setOf(
            "umac-128-etm@openssh.com", "hmac-sha2-256-etm@openssh.com",
            "hmac-sha2-512-etm@openssh.com", "hmac-sha2-256", "hmac-sha2-512"
        )
        return validMacs.contains(mac)
    }
    
    private fun isValidUsername(username: String): Boolean {
        return username.matches(Regex("^[a-z_][a-z0-9_-]*\\$?$")) && username.length <= 32
    }
    
    private fun isValidGroupName(groupName: String): Boolean {
        return groupName.matches(Regex("^[a-z_][a-z0-9_-]*\\$?$")) && groupName.length <= 32
    }
    
    private fun isValidCommand(command: String): Boolean {
        return command.isNotEmpty() && !command.contains('\n')
    }
    
    private fun isValidPAMModule(module: String): Boolean {
        val validModules = setOf(
            "pam_unix.so", "pam_deny.so", "pam_permit.so", "pam_env.so",
            "pam_faillock.so", "pam_limits.so", "pam_systemd.so", "pam_google_authenticator.so"
        )
        return validModules.contains(module) || module.startsWith("pam_")
    }
    
    private fun isValidFirewallRule(rule: String): Boolean {
        return rule.isNotEmpty()
    }
    
    private fun isValidPortRange(port: String): Boolean {
        return when {
            port.contains(':') -> {
                val parts = port.split(':')
                parts.size == 2 && parts.all { isValidPort(it) }
            }
            else -> isValidPort(port)
        }
    }
    
    private fun isValidPort(port: String): Boolean {
        return port.toIntOrNull()?.let { it in 1..65535 } ?: false
    }
    
    private fun isValidIPAddress(address: String): Boolean {
        return address.matches(Regex("^(\\d{1,3}\\.){3}\\d{1,3}(/\\d{1,2})?$")) ||
               address.matches(Regex("^[0-9a-fA-F:]+(/\\d{1,3})?$"))
    }
    
    private fun isValidGPGKeyId(keyId: String): Boolean {
        return keyId.matches(Regex("^[0-9A-F]{8,40}$"))
    }
    
    private fun isValidTrustLevel(level: String): Boolean {
        val validLevels = setOf("unknown", "undefined", "never", "marginal", "full", "ultimate")
        return validLevels.contains(level.lowercase())
    }
    
    private fun isValidAuditRule(rule: String): Boolean {
        return rule.isNotEmpty() && rule.startsWith("-")
    }
    
    private fun isValidPath(path: String): Boolean {
        return path.matches(Regex("^/[a-zA-Z0-9._/-]+$"))
    }
}