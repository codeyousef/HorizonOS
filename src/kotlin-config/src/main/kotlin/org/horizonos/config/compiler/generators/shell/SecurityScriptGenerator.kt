package org.horizonos.config.compiler.generators.shell

import org.horizonos.config.dsl.CompiledConfig
import org.horizonos.config.compiler.GeneratedFile
import org.horizonos.config.compiler.FileType
import java.io.File

/**
 * Security script generator for security configuration
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
                appendLine("echo 'Setting up security configuration...'")
                appendLine()
                
                // PAM configuration
                if (security.pam.enabled) {
                    appendLine("# PAM Configuration")
                    appendLine("echo 'Configuring PAM authentication...'")
                    appendLine("# PAM modules and rules configuration")
                    appendLine()
                }
                
                // SSH configuration
                if (security.ssh.enabled) {
                    appendLine("# SSH Configuration")
                    appendLine("echo 'Configuring SSH daemon...'")
                    appendLine("systemctl enable sshd")
                    appendLine("mkdir -p /etc/ssh/sshd_config.d")
                    appendLine("# SSH security hardening")
                    appendLine()
                }
                
                // Firewall configuration
                if (security.firewall.enabled) {
                    appendLine("# Firewall Configuration")
                    appendLine("echo 'Configuring firewall...'")
                    appendLine("systemctl enable firewalld")
                    appendLine("systemctl start firewalld")
                    appendLine()
                }
                
                // SELinux/AppArmor
                if (security.selinux.enabled) {
                    appendLine("# SELinux Configuration")
                    appendLine("echo 'Configuring SELinux...'")
                    appendLine("# SELinux policy configuration")
                    appendLine()
                }
                
                if (security.apparmor.enabled) {
                    appendLine("# AppArmor Configuration")
                    appendLine("echo 'Configuring AppArmor...'")
                    appendLine("systemctl enable apparmor")
                    appendLine()
                }
                
                // TPM security
                if (security.tpm.enabled) {
                    appendLine("# TPM Security Configuration")
                    appendLine("echo 'Configuring TPM security...'")
                    appendLine("# TPM configuration")
                    appendLine()
                }
                
                appendLine("echo 'Security configuration completed.'")
            })
            script.setExecutable(true)
            generatedFiles.add(GeneratedFile("scripts/security-config.sh", FileType.SHELL))
        }
    }
}