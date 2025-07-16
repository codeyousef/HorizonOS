package org.horizonos.config.compiler.generators.scripts

import org.horizonos.config.dsl.*
import org.horizonos.config.dsl.security.*
import org.horizonos.config.compiler.GeneratedFile
import org.horizonos.config.compiler.FileType
import java.io.File

/**
 * Security Script Generator
 * Generates shell scripts for security configuration including SSH, sudo, PAM, firewall, and compliance
 */
class SecurityScriptGenerator(
    private val outputDir: File,
    private val generatedFiles: MutableList<GeneratedFile>
) {
    
    fun generateSecurityScript(config: CompiledConfig) {
        config.security?.let { security ->
            val script = File(outputDir, "scripts/security-config.sh")
            script.writeText(buildString {
                appendLine("#!/bin/bash")
                appendLine("# Security Configuration")
                appendLine("# Generated from HorizonOS Kotlin DSL")
                appendLine()
                appendLine("set -euo pipefail")
                appendLine()
                appendLine("echo 'Configuring security...'")
                appendLine()
                
                // SSH configuration
                generateSSHConfig(security)
                
                // Sudo configuration
                generateSudoConfig(security)
                
                // PAM configuration
                generatePAMConfig(security)
                
                // Fail2ban configuration
                generateFail2banConfig(security)
                
                // Firewall configuration
                generateFirewallConfig(security)
                
                // SELinux configuration
                generateSELinuxConfig(security)
                
                // AppArmor configuration
                generateAppArmorConfig(security)
                
                // GPG configuration
                generateGPGConfig(security)
                
                // Audit configuration
                generateAuditConfig(security)
                
                // TPM configuration
                generateTPMConfig(security)
                
                // Certificate management
                generateCertificateConfig(security)
                
                // Compliance scanning
                generateComplianceConfig(security)
                
                appendLine("echo 'Security configuration completed.'")
            })
            script.setExecutable(true)
            generatedFiles.add(GeneratedFile("scripts/security-config.sh", FileType.SHELL))
        }
    }
    
    private fun StringBuilder.generateSSHConfig(security: SecurityConfig) {
        if (security.ssh.enabled) {
            appendLine("# SSH Configuration")
            appendLine("cp /etc/ssh/sshd_config /etc/ssh/sshd_config.backup")
            appendLine()
            appendLine("cat > /etc/ssh/sshd_config <<EOF")
            appendLine("# HorizonOS SSH Configuration")
            appendLine("Port ${security.ssh.port}")
            security.ssh.access.listenAddresses.forEach { addr ->
                appendLine("ListenAddress $addr")
            }
            appendLine("Protocol 2")
            appendLine()
            
            // Authentication settings
            appendLine("# Authentication")
            appendLine("PubkeyAuthentication ${if (security.ssh.auth.pubkeyAuth) "yes" else "no"}")
            appendLine("PasswordAuthentication ${if (security.ssh.auth.passwordAuth) "yes" else "no"}")
            appendLine("KerberosAuthentication no")
            appendLine("GSSAPIAuthentication ${if (security.ssh.auth.gssapiAuth) "yes" else "no"}")
            appendLine("HostbasedAuthentication ${if (security.ssh.auth.hostbasedAuth) "yes" else "no"}")
            appendLine("ChallengeResponseAuthentication ${if (security.ssh.auth.challengeResponseAuth) "yes" else "no"}")
            appendLine("MaxAuthTries ${security.ssh.auth.maxAuthTries}")
            appendLine("LoginGraceTime ${security.ssh.auth.loginGraceTime.inWholeSeconds}")
            appendLine("PermitEmptyPasswords ${if (security.ssh.auth.emptyPasswords) "yes" else "no"}")
            appendLine()
            
            // Access control
            if (security.ssh.access.allowUsers.isNotEmpty()) {
                appendLine("AllowUsers ${security.ssh.access.allowUsers.joinToString(" ")}")
            }
            if (security.ssh.access.allowGroups.isNotEmpty()) {
                appendLine("AllowGroups ${security.ssh.access.allowGroups.joinToString(" ")}")
            }
            if (security.ssh.access.denyUsers.isNotEmpty()) {
                appendLine("DenyUsers ${security.ssh.access.denyUsers.joinToString(" ")}")
            }
            if (security.ssh.access.denyGroups.isNotEmpty()) {
                appendLine("DenyGroups ${security.ssh.access.denyGroups.joinToString(" ")}")
            }
            appendLine("PermitRootLogin ${security.ssh.auth.rootLogin.name.lowercase().replace("_", "-")}")
            appendLine("MaxSessions ${security.ssh.auth.maxSessions}")
            appendLine("MaxStartups ${security.ssh.access.maxStartups}")
            appendLine()
            
            // Security settings
            appendLine("# Security")
            appendLine("StrictModes ${if (security.ssh.security.strictModes) "yes" else "no"}")
            appendLine("IgnoreRhosts ${if (security.ssh.security.ignoreRhosts) "yes" else "no"}")
            appendLine("IgnoreUserKnownHosts ${if (security.ssh.security.ignoreuserKnownHosts) "yes" else "no"}")
            appendLine("PrintMotd yes")
            appendLine("PrintLastLog yes")
            appendLine("TCPKeepAlive yes")
            appendLine("Compression ${security.ssh.encryption.compression.name.lowercase()}")
            appendLine("UseDNS no")
            appendLine()
            
            // Encryption settings
            if (security.ssh.encryption.ciphers.isNotEmpty()) {
                appendLine("Ciphers ${security.ssh.encryption.ciphers.joinToString(",")}")
            }
            if (security.ssh.encryption.macs.isNotEmpty()) {
                appendLine("MACs ${security.ssh.encryption.macs.joinToString(",")}")
            }
            if (security.ssh.encryption.kexAlgorithms.isNotEmpty()) {
                appendLine("KexAlgorithms ${security.ssh.encryption.kexAlgorithms.joinToString(",")}")
            }
            appendLine()
            
            // Client alive settings
            appendLine("ClientAliveInterval ${security.ssh.clientAlive.interval.inWholeSeconds}")
            appendLine("ClientAliveCountMax ${security.ssh.clientAlive.maxCount}")
            appendLine()
            
            appendLine("EOF")
            appendLine()
            
            // Generate host keys
            if (security.ssh.keys.keyGeneration.generateHostKeys) {
                appendLine("# Generate SSH host keys")
                security.ssh.keys.hostKeys.forEach { hostKey ->
                    when (hostKey.type) {
                        SSHKeyType.ED25519 -> {
                            appendLine("ssh-keygen -t ed25519 -f ${hostKey.path} -N ''")
                        }
                        SSHKeyType.ECDSA -> {
                            appendLine("ssh-keygen -t ecdsa -b ${hostKey.bits ?: 256} -f ${hostKey.path} -N ''")
                        }
                        SSHKeyType.RSA -> {
                            appendLine("ssh-keygen -t rsa -b ${hostKey.bits ?: 4096} -f ${hostKey.path} -N ''")
                        }
                        SSHKeyType.DSA -> {
                            appendLine("ssh-keygen -t dsa -f ${hostKey.path} -N ''")
                        }
                    }
                }
                appendLine()
            }
            
            // Restart SSH service
            appendLine("systemctl restart sshd.service")
            appendLine("systemctl enable sshd.service")
            appendLine()
        }
    }
    
    private fun StringBuilder.generateSudoConfig(security: SecurityConfig) {
        if (security.sudo.enabled) {
            appendLine("# Sudo Configuration")
            appendLine("cp /etc/sudoers /etc/sudoers.backup")
            appendLine()
            appendLine("cat > /etc/sudoers.d/horizonos <<EOF")
            appendLine("# HorizonOS Sudo Configuration")
            appendLine()
            
            // Defaults
            val defaults = security.sudo.defaults
            appendLine("# Defaults")
            if (!security.sudo.security.requireAuthentication) {
                appendLine("Defaults !authenticate")
            } else {
                appendLine("Defaults authenticate")
            }
            appendLine("Defaults passwd_timeout=${defaults.passwordTimeout.inWholeMinutes}")
            appendLine("Defaults passwd_tries=${defaults.maxTries}")
            security.sudo.logging.logFile?.let { appendLine("Defaults logfile=\"$it\"") }
            defaults.securePathOverride?.let { appendLine("Defaults secure_path=\"$it\"") }
            if (defaults.envReset) {
                appendLine("Defaults env_reset")
            }
            if (defaults.envKeep.isNotEmpty()) {
                appendLine("Defaults env_keep += \"${defaults.envKeep.joinToString(" ")}\"")
            }
            appendLine()
            
            // User aliases
            security.sudo.aliases.userAliases.forEach { (alias, users) ->
                appendLine("User_Alias $alias = ${users.joinToString(", ")}")
            }
            
            // Host aliases
            security.sudo.aliases.hostAliases.forEach { (alias, hosts) ->
                appendLine("Host_Alias $alias = ${hosts.joinToString(", ")}")
            }
            
            // Command aliases
            security.sudo.aliases.cmdAliases.forEach { (alias, commands) ->
                appendLine("Cmnd_Alias $alias = ${commands.joinToString(", ")}")
            }
            
            // RunAs aliases
            security.sudo.aliases.runAsAliases.forEach { (alias, users) ->
                appendLine("Runas_Alias $alias = ${users.joinToString(", ")}")
            }
            
            if (security.sudo.aliases.userAliases.isNotEmpty() || 
                security.sudo.aliases.hostAliases.isNotEmpty() ||
                security.sudo.aliases.cmdAliases.isNotEmpty() ||
                security.sudo.aliases.runAsAliases.isNotEmpty()) {
                appendLine()
            }
            
            // Sudo rules
            appendLine("# Sudo Rules")
            security.sudo.rules.forEach { rule ->
                val tags = if (rule.tags.isNotEmpty()) {
                    rule.tags.joinToString(": ", postfix = ": ")
                } else ""
                
                val options = if (rule.options.isNotEmpty()) {
                    " " + rule.options.joinToString(" ")
                } else ""
                
                val users = (rule.users + rule.groups.map { "%$it" }).joinToString(", ")
                val hosts = rule.hosts.joinToString(", ")
                val runAsUsers = rule.runAsUsers.joinToString(", ")
                val commands = rule.commands.joinToString(", ")
                val ruleStr = "$users $hosts = ($runAsUsers) $tags$commands$options"
                
                appendLine(ruleStr)
            }
            
            appendLine("EOF")
            appendLine()
            appendLine("visudo -c -f /etc/sudoers.d/horizonos")
            appendLine()
        }
    }
    
    private fun StringBuilder.generatePAMConfig(security: SecurityConfig) {
        if (security.pam.enabled) {
            appendLine("# PAM Configuration")
            
            // Password policy
            val passPolicy = security.pam.passwordPolicy
            appendLine("# Configure password policy")
            appendLine("cat > /etc/security/pwquality.conf <<EOF")
            appendLine("minlen = ${passPolicy.minLength}")
            appendLine("maxrepeat = ${passPolicy.maxRepeatingChars}")
            appendLine("maxsequence = ${passPolicy.complexity.maxConsecutiveChars}")
            if (passPolicy.requireUppercase) appendLine("ucredit = -1")
            if (passPolicy.requireLowercase) appendLine("lcredit = -1")
            if (passPolicy.requireNumbers) appendLine("dcredit = -1")
            if (passPolicy.requireSpecialChars) appendLine("ocredit = -1")
            if (passPolicy.dictionary.enabled) {
                appendLine("dictcheck = 1")
                if (passPolicy.dictionary.dictionaryFiles.isNotEmpty()) {
                    appendLine("dictpath = ${passPolicy.dictionary.dictionaryFiles.first()}")
                }
            }
            appendLine("EOF")
            appendLine()
            
            // Account lockout
            if (security.pam.lockout.enabled) {
                appendLine("# Configure account lockout")
                appendLine("cat > /etc/security/faillock.conf <<EOF")
                appendLine("deny = ${security.pam.lockout.maxFailedAttempts}")
                appendLine("fail_interval = ${security.pam.lockout.resetCounterTime.inWholeSeconds}")
                appendLine("unlock_time = ${security.pam.lockout.lockoutDuration.inWholeSeconds}")
                appendLine("EOF")
                appendLine()
            }
            
            // Two-factor authentication
            if (security.pam.twoFactor.enabled) {
                appendLine("# Configure two-factor authentication")
                when (security.pam.twoFactor.method) {
                    TwoFactorMethod.TOTP -> {
                        appendLine("# Install Google Authenticator PAM module")
                        appendLine("pacman -S --noconfirm libpam-google-authenticator")
                        appendLine()
                        appendLine("# Configure TOTP for required users")
                    }
                    else -> {
                        appendLine("# Two-factor method ${security.pam.twoFactor.method} configuration")
                    }
                }
                appendLine()
            }
        }
    }
    
    private fun StringBuilder.generateFail2banConfig(security: SecurityConfig) {
        if (security.fail2ban.enabled) {
            appendLine("# Fail2ban Configuration")
            appendLine("systemctl enable fail2ban.service")
            appendLine()
            appendLine("cat > /etc/fail2ban/jail.local <<EOF")
            appendLine("[DEFAULT]")
            appendLine("bantime = ${security.fail2ban.banTime.inWholeSeconds}")
            appendLine("findtime = ${security.fail2ban.findTime.inWholeSeconds}")
            appendLine("maxretry = ${security.fail2ban.maxRetry}")
            appendLine("backend = ${security.fail2ban.backend.name.lowercase()}")
            appendLine("usedns = ${security.fail2ban.usedns.name.lowercase()}")
            appendLine()
            
            // Jail configurations
            security.fail2ban.jails.forEach { jail ->
                appendLine("[${jail.name}]")
                appendLine("enabled = ${jail.enabled}")
                appendLine("filter = ${jail.filter}")
                appendLine("logpath = ${jail.logpath.joinToString(" ")}")
                appendLine("action = ${jail.action}")
                appendLine("port = ${jail.port}")
                appendLine("protocol = ${jail.protocol}")
                appendLine("bantime = ${jail.banTime.inWholeSeconds}")
                appendLine("findtime = ${jail.findTime.inWholeSeconds}")
                appendLine("maxretry = ${jail.maxRetry}")
                if (jail.ignoreIp.isNotEmpty()) {
                    appendLine("ignoreip = ${jail.ignoreIp.joinToString(" ")}")
                }
                appendLine()
            }
            
            appendLine("EOF")
            appendLine()
            appendLine("systemctl start fail2ban.service")
            appendLine()
        }
    }
    
    private fun StringBuilder.generateFirewallConfig(security: SecurityConfig) {
        if (security.firewall.enabled) {
            appendLine("# Firewall Configuration")
            
            when (security.firewall.backend) {
                FirewallBackend.IPTABLES -> {
                    appendLine("# Configure iptables")
                    appendLine("iptables -F")
                    appendLine("iptables -X")
                    appendLine("iptables -t nat -F")
                    appendLine("iptables -t nat -X")
                    appendLine()
                    
                    appendLine("# Set default policies")
                    security.firewall.defaultPolicy.forEach { (chain, policy) ->
                        appendLine("iptables -P $chain ${policy.name}")
                    }
                    appendLine()
                    
                    appendLine("# Allow loopback")
                    appendLine("iptables -A INPUT -i lo -j ACCEPT")
                    appendLine("iptables -A OUTPUT -o lo -j ACCEPT")
                    appendLine()
                    
                    appendLine("# Allow established connections")
                    appendLine("iptables -A INPUT -m state --state ESTABLISHED,RELATED -j ACCEPT")
                    appendLine()
                    
                    // Custom rules
                    security.firewall.rules.forEach { rule ->
                        val cmd = buildString {
                            append("iptables -A ${rule.chain}")
                            rule.source?.let { append(" -s $it") }
                            rule.destination?.let { append(" -d $it") }
                            rule.protocol?.let { append(" -p $it") }
                            rule.dport?.let { append(" --dport $it") }
                            rule.sport?.let { append(" --sport $it") }
                            if (rule.state.isNotEmpty()) {
                                append(" -m state --state ${rule.state.joinToString(",") { it.name }}")
                            }
                            append(" -j ${rule.action.name}")
                            rule.comment?.let { append(" -m comment --comment \"$it\"") }
                        }
                        appendLine(cmd)
                    }
                    
                    appendLine()
                    appendLine("# Save iptables rules")
                    appendLine("iptables-save > /etc/iptables/iptables.rules")
                    appendLine("systemctl enable iptables.service")
                }
                FirewallBackend.FIREWALLD -> {
                    appendLine("# Configure firewalld")
                    appendLine("systemctl enable firewalld.service")
                    appendLine("systemctl start firewalld.service")
                    appendLine()
                    
                    // Configure zones
                    security.firewall.zones.forEach { zone ->
                        appendLine("# Configure zone: ${zone.name}")
                        zone.interfaces.forEach { iface ->
                            appendLine("firewall-cmd --zone=${zone.name} --add-interface=$iface --permanent")
                        }
                        zone.sources.forEach { source ->
                            appendLine("firewall-cmd --zone=${zone.name} --add-source=$source --permanent")
                        }
                        zone.services.forEach { service ->
                            appendLine("firewall-cmd --zone=${zone.name} --add-service=$service --permanent")
                        }
                        zone.ports.forEach { port ->
                            appendLine("firewall-cmd --zone=${zone.name} --add-port=$port --permanent")
                        }
                        if (zone.masquerade) {
                            appendLine("firewall-cmd --zone=${zone.name} --add-masquerade --permanent")
                        }
                        appendLine()
                    }
                    
                    appendLine("firewall-cmd --reload")
                }
                FirewallBackend.NFTABLES, FirewallBackend.UFW -> {
                    appendLine("# Firewall backend ${security.firewall.backend} configuration")
                }
            }
            appendLine()
        }
    }
    
    private fun StringBuilder.generateSELinuxConfig(security: SecurityConfig) {
        if (security.selinux.enabled) {
            appendLine("# SELinux Configuration")
            appendLine("# Set SELinux mode")
            appendLine("setenforce ${if (security.selinux.mode == SELinuxMode.ENFORCING) "1" else "0"}")
            appendLine("sed -i 's/^SELINUX=.*/SELINUX=${security.selinux.mode.name.lowercase()}/' /etc/selinux/config")
            appendLine()
            
            // SELinux booleans
            security.selinux.booleans.forEach { (boolean, value) ->
                appendLine("setsebool -P $boolean ${if (value) "on" else "off"}")
            }
            
            if (security.selinux.booleans.isNotEmpty()) {
                appendLine()
            }
            
            // Load SELinux modules
            security.selinux.modules.forEach { module ->
                if (module.enabled) {
                    appendLine("# Load SELinux module: ${module.name}")
                    module.path?.let { path ->
                        appendLine("semodule -i $path")
                    } ?: run {
                        appendLine("semodule -e ${module.name}")
                    }
                } else {
                    appendLine("semodule -d ${module.name}")
                }
            }
            
            if (security.selinux.modules.isNotEmpty()) {
                appendLine()
            }
        }
    }
    
    private fun StringBuilder.generateAppArmorConfig(security: SecurityConfig) {
        if (security.apparmor.enabled) {
            appendLine("# AppArmor Configuration")
            appendLine("systemctl enable apparmor.service")
            appendLine("systemctl start apparmor.service")
            appendLine()
            
            // Profile management
            security.apparmor.profiles.forEach { profile ->
                when (profile.mode) {
                    AppArmorMode.ENFORCE -> appendLine("aa-enforce ${profile.path}")
                    AppArmorMode.COMPLAIN -> appendLine("aa-complain ${profile.path}")
                    AppArmorMode.DISABLE -> appendLine("aa-disable ${profile.path}")
                    AppArmorMode.AUDIT -> appendLine("aa-audit ${profile.path}")
                }
            }
            
            if (security.apparmor.profiles.isNotEmpty()) {
                appendLine()
            }
        }
    }
    
    private fun StringBuilder.generateGPGConfig(security: SecurityConfig) {
        if (security.gpg.enabled) {
            appendLine("# GPG Configuration")
            appendLine("mkdir -p /etc/gnupg")
            appendLine()
            appendLine("cat > /etc/gnupg/gpg.conf <<EOF")
            appendLine("# HorizonOS GPG Configuration")
            security.gpg.keyservers.forEach { keyserver ->
                appendLine("keyserver $keyserver")
            }
            appendLine("trust-model ${security.gpg.trustModel.name.lowercase()}")
            appendLine("default-cipher-algo ${security.gpg.defaultCipher}")
            appendLine("default-digest-algo ${security.gpg.defaultDigest}")
            appendLine("compress-level ${security.gpg.defaultCompress}")
            if (security.gpg.autoKeyRetrieve) {
                appendLine("auto-key-retrieve")
            }
            if (security.gpg.autoKeyLocate.isNotEmpty()) {
                appendLine("auto-key-locate ${security.gpg.autoKeyLocate.joinToString(",")}")
            }
            appendLine("EOF")
            appendLine()
            
            // Import GPG keys
            security.gpg.keys.forEach { key ->
                appendLine("# Import GPG key: ${key.keyId}")
                key.publicKey?.let { publicKey ->
                    appendLine("echo '$publicKey' | gpg --import")
                }
                key.secretKey?.let { secretKey ->
                    appendLine("echo '$secretKey' | gpg --import")
                }
                appendLine("# Set trust level to ${key.trustLevel.name}")
                appendLine("echo -e \"5\\ny\\n\" | gpg --command-fd 0 --edit-key ${key.keyId} trust quit")
            }
            
            if (security.gpg.keys.isNotEmpty()) {
                appendLine()
            }
        }
    }
    
    private fun StringBuilder.generateAuditConfig(security: SecurityConfig) {
        if (security.audit.enabled) {
            appendLine("# Audit Configuration")
            appendLine("systemctl enable auditd.service")
            appendLine()
            appendLine("cat > /etc/audit/auditd.conf <<EOF")
            appendLine("# HorizonOS Audit Configuration")
            appendLine("log_file = /var/log/audit/audit.log")
            appendLine("log_format = ${security.audit.logFormat}")
            appendLine("log_group = ${security.audit.logGroup}")
            appendLine("priority_boost = ${security.audit.priority}")
            appendLine("flush = INCREMENTAL_ASYNC")
            appendLine("freq = 50")
            appendLine("max_log_file = ${security.audit.maxLogFile}")
            appendLine("num_logs = 5")
            appendLine("max_log_file_action = ${security.audit.maxLogFileAction}")
            appendLine("space_left = ${security.audit.spaceLeft}")
            appendLine("space_left_action = ${security.audit.spaceLeftAction}")
            appendLine("admin_space_left = ${security.audit.adminSpaceLeft}")
            appendLine("admin_space_left_action = ${security.audit.adminSpaceLeftAction}")
            appendLine("disk_full_action = ${security.audit.diskFull}")
            appendLine("disk_error_action = ${security.audit.diskError}")
            appendLine("use_libwrap = yes")
            security.audit.tcpListenPort?.let { port ->
                appendLine("tcp_listen_port = $port")
                appendLine("tcp_max_per_addr = ${security.audit.tcpMaxPerAddr}")
                appendLine("tcp_client_max_idle = ${security.audit.tcpClientMaxIdle.inWholeSeconds}")
            }
            appendLine("name_format = ${security.audit.nameFormat}")
            security.audit.name?.let { name ->
                appendLine("name = $name")
            }
            appendLine("EOF")
            appendLine()
            
            // Audit rules
            if (security.audit.rules.isNotEmpty()) {
                appendLine("cat > /etc/audit/rules.d/horizonos.rules <<EOF")
                appendLine("# HorizonOS Audit Rules")
                security.audit.rules.forEach { rule ->
                    if (rule.enabled) {
                        rule.comment?.let { comment ->
                            appendLine("# $comment")
                        }
                        appendLine(rule.rule)
                    }
                }
                appendLine("EOF")
                appendLine()
            }
            
            appendLine("systemctl start auditd.service")
            appendLine()
        }
    }
    
    private fun StringBuilder.generateTPMConfig(security: SecurityConfig) {
        if (security.tpm.enabled) {
            appendLine("# TPM Configuration")
            appendLine("modprobe tpm_tis")
            appendLine("modprobe tpm_crb")
            appendLine()
            
            if (security.tpm.ownership.takeOwnership) {
                appendLine("# Take TPM ownership")
                when (security.tpm.version) {
                    "2.0" -> {
                        appendLine("tpm2_startup -c")
                        appendLine("tpm2_clear")
                        security.tpm.ownership.ownerPassword?.let { password ->
                            appendLine("echo '$password' | tpm2_changeauth -c owner")
                        }
                    }
                    "1.2" -> {
                        appendLine("# TPM 1.2 ownership")
                        security.tpm.ownership.ownerPassword?.let { password ->
                            appendLine("echo '$password' | tpm_takeownership")
                        }
                    }
                    else -> {
                        appendLine("# Unknown TPM version: ${security.tpm.version}")
                    }
                }
                appendLine()
            }
            
            if (security.tpm.ima.enabled) {
                appendLine("# Configure IMA/EVM")
                appendLine("echo 'ima_template=${security.tpm.ima.template}' >> /etc/default/grub")
                appendLine("echo 'ima_hash=${security.tpm.ima.hashAlgorithm}' >> /etc/default/grub")
                appendLine("grub-mkconfig -o /boot/grub/grub.cfg")
                appendLine()
            }
        }
    }
    
    private fun StringBuilder.generateCertificateConfig(security: SecurityConfig) {
        if (security.certificates.enabled) {
            appendLine("# Certificate Management")
            
            // Configure CA certificates
            security.certificates.caCerts.forEach { ca ->
                appendLine("# Configure CA: ${ca.name}")
                appendLine("mkdir -p /etc/ssl/certs/ca")
                if (ca.certificate.isNotBlank()) {
                    appendLine("echo '${ca.certificate}' > /etc/ssl/certs/ca/${ca.name}.crt")
                    appendLine("update-ca-certificates")
                }
            }
            
            // Configure trusted certificates
            security.certificates.trustedCerts.forEach { cert ->
                appendLine("# Configure certificate: ${cert.name}")
                appendLine("mkdir -p \$(dirname ${cert.path})")
                appendLine("# Certificate: ${cert.subject}")
                cert.issuer?.let { issuer ->
                    appendLine("# Issuer: $issuer")
                }
                appendLine()
            }
            
            // Update certificate stores
            security.certificates.stores.forEach { store ->
                appendLine("# Update certificate store: ${store.name}")
                appendLine("update-ca-certificates")
            }
            appendLine()
        }
    }
    
    private fun StringBuilder.generateComplianceConfig(security: SecurityConfig) {
        if (security.compliance.enabled) {
            appendLine("# Compliance Configuration")
            
            security.compliance.frameworks.forEach { framework ->
                if (framework.enabled) {
                    appendLine("# Configure ${framework.name} ${framework.version}")
                    appendLine("# Apply profile: ${framework.profile}")
                }
            }
            
            if (security.compliance.scanning.enabled) {
                appendLine("# Setup compliance scanning")
                appendLine("systemctl enable compliance-scan.timer")
            }
            
            if (security.compliance.remediation.enabled && security.compliance.remediation.autoRemediate) {
                appendLine("# Enable auto-remediation")
                appendLine("systemctl enable compliance-remediate.service")
            }
            appendLine()
        }
    }
}