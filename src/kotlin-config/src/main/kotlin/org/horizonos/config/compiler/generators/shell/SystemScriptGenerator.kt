package org.horizonos.config.compiler.generators.shell

import org.horizonos.config.dsl.*
import org.horizonos.config.dsl.boot.bootloader.BootloaderType
import org.horizonos.config.compiler.GeneratedFile
import org.horizonos.config.compiler.FileType
import java.io.File

/**
 * System script generator for core system configuration
 * Handles system settings, packages, services, users, repositories, boot, and automation
 */
class SystemScriptGenerator(
    private val outputDir: File,
    private val generatedFiles: MutableList<GeneratedFile>
) {
    
    fun generateSystemConfigScript(config: CompiledConfig) {
        val script = File(outputDir, "scripts/system-config.sh")
        script.writeText(buildString {
            appendLine("#!/bin/bash")
            appendLine("# System Configuration")
            appendLine()
            appendLine("echo 'Configuring system...'")
            appendLine()
            appendLine("# Set hostname")
            appendLine("hostnamectl set-hostname '${config.system.hostname}'")
            appendLine()
            appendLine("# Set timezone")
            appendLine("timedatectl set-timezone '${config.system.timezone}'")
            appendLine()
            appendLine("# Set locale")
            appendLine("echo 'LANG=${config.system.locale}' > /etc/locale.conf")
            appendLine("locale-gen")
            appendLine()
            appendLine("echo 'System configuration completed.'")
        })
        script.setExecutable(true)
        generatedFiles.add(GeneratedFile("scripts/system-config.sh", FileType.SHELL))
    }
    
    fun generatePackageScript(config: CompiledConfig) {
        val script = File(outputDir, "scripts/package-manager.sh")
        script.writeText(buildString {
            appendLine("#!/bin/bash")
            appendLine("# Package Management")
            appendLine()
            
            val toInstall = config.packages.filter { it.action == PackageAction.INSTALL }
            val toRemove = config.packages.filter { it.action == PackageAction.REMOVE }
            
            if (toInstall.isNotEmpty()) {
                appendLine("echo 'Installing packages...'")
                appendLine("pacman -S --needed --noconfirm \\")
                toInstall.forEach { pkg ->
                    append("    ${pkg.name}")
                    if (pkg != toInstall.last()) append(" \\")
                    appendLine()
                }
                appendLine()
            }
            
            if (toRemove.isNotEmpty()) {
                appendLine("echo 'Removing packages...'")
                appendLine("pacman -R --noconfirm \\")
                toRemove.forEach { pkg ->
                    append("    ${pkg.name}")
                    if (pkg != toRemove.last()) append(" \\")
                    appendLine()
                }
                appendLine()
            }
            
            appendLine("echo 'Package management completed.'")
        })
        script.setExecutable(true)
        generatedFiles.add(GeneratedFile("scripts/package-manager.sh", FileType.SHELL))
    }
    
    fun generateServiceScript(config: CompiledConfig) {
        val script = File(outputDir, "scripts/service-manager.sh")
        script.writeText(buildString {
            appendLine("#!/bin/bash")
            appendLine("# Service Management")
            appendLine()
            appendLine("echo 'Configuring services...'")
            appendLine()
            
            config.services.forEach { service ->
                if (service.enabled) {
                    appendLine("systemctl enable ${service.name}")
                    appendLine("systemctl start ${service.name} || true")
                } else {
                    appendLine("systemctl disable ${service.name} || true")
                    appendLine("systemctl stop ${service.name} || true")
                }
                
                service.config?.let { cfg ->
                    if (cfg.environment.isNotEmpty()) {
                        appendLine()
                        appendLine("# Configure ${service.name} environment")
                        appendLine("mkdir -p /etc/systemd/system/${service.name}.service.d")
                        appendLine("cat > /etc/systemd/system/${service.name}.service.d/environment.conf <<EOF")
                        appendLine("[Service]")
                        cfg.environment.forEach { (key, value) ->
                            appendLine("Environment=\"$key=$value\"")
                        }
                        appendLine("EOF")
                    }
                }
            }
            
            appendLine()
            appendLine("systemctl daemon-reload")
            appendLine("echo 'Service configuration completed.'")
        })
        script.setExecutable(true)
        generatedFiles.add(GeneratedFile("scripts/service-manager.sh", FileType.SHELL))
    }
    
    fun generateUserScript(config: CompiledConfig) {
        val script = File(outputDir, "scripts/user-manager.sh")
        script.writeText(buildString {
            appendLine("#!/bin/bash")
            appendLine("# User Management")
            appendLine()
            appendLine("echo 'Creating users...'")
            appendLine()
            
            config.users.forEach { user ->
                appendLine("# Create user: ${user.name}")
                append("useradd -m")
                user.uid?.let { append(" -u $it") }
                append(" -s ${user.shell}")
                if (user.groups.isNotEmpty()) {
                    append(" -G ${user.groups.joinToString(",")}")
                }
                appendLine(" ${user.name}")
                appendLine()
            }
            
            appendLine("echo 'User management completed.'")
        })
        script.setExecutable(true)
        generatedFiles.add(GeneratedFile("scripts/user-manager.sh", FileType.SHELL))
    }
    
    fun generateRepoScript(config: CompiledConfig) {
        val script = File(outputDir, "scripts/repository-config.sh")
        script.writeText(buildString {
            appendLine("#!/bin/bash")
            appendLine("# Repository Configuration")
            appendLine()
            appendLine("echo 'Configuring repositories...'")
            appendLine()
            
            config.repositories.forEach { repo ->
                when (repo) {
                    is PackageRepository -> {
                        appendLine("# Add package repository: ${repo.name}")
                        appendLine("echo '[${repo.name}]' >> /etc/pacman.conf")
                        appendLine("echo 'Server = ${repo.url}' >> /etc/pacman.conf")
                        if (!repo.gpgCheck) {
                            appendLine("echo 'SigLevel = Never' >> /etc/pacman.conf")
                        }
                        if (!repo.enabled) {
                            appendLine("# Repository ${repo.name} is disabled")
                        }
                    }
                    is OstreeRepository -> {
                        appendLine("# Add OSTree repository: ${repo.name}")
                        appendLine("ostree remote add ${repo.name} ${repo.url}")
                        repo.branches.forEach { branch ->
                            appendLine("ostree pull ${repo.name}:$branch || true")
                        }
                    }
                }
                appendLine()
            }
            
            appendLine("echo 'Repository configuration completed.'")
        })
        script.setExecutable(true)
        generatedFiles.add(GeneratedFile("scripts/repository-config.sh", FileType.SHELL))
    }
    
    fun generateBootScript(config: CompiledConfig) {
        config.boot?.let { boot ->
            val script = File(outputDir, "scripts/boot-config.sh")
            script.writeText(buildString {
                appendLine("#!/bin/bash")
                appendLine("# Boot Configuration")
                appendLine()
                appendLine("echo 'Configuring boot system...'")
                appendLine()
                
                appendLine("# Bootloader: ${boot.bootloader.type}")
                when (boot.bootloader.type) {
                    BootloaderType.SYSTEMD_BOOT -> {
                        appendLine("bootctl install")
                        appendLine("mkdir -p /boot/loader/entries")
                        appendLine("cat > /boot/loader/loader.conf <<EOF")
                        appendLine("timeout ${boot.bootloader.timeout}")
                        appendLine("default ${boot.bootloader.defaultEntry}")
                        appendLine("EOF")
                    }
                    BootloaderType.GRUB -> {
                        appendLine("grub-install --target=x86_64-efi --efi-directory=/boot")
                        appendLine("grub-mkconfig -o /boot/grub/grub.cfg")
                    }
                    else -> {
                        appendLine("# Bootloader ${boot.bootloader.type} configuration")
                    }
                }
                
                if (boot.kernel.parameters.isNotEmpty()) {
                    appendLine()
                    appendLine("# Kernel parameters")
                    boot.kernel.parameters.forEach { param ->
                        appendLine("echo '${param.name}=${param.value}' >> /etc/kernel/cmdline")
                    }
                }
                
                appendLine()
                appendLine("echo 'Boot configuration completed.'")
            })
            script.setExecutable(true)
            generatedFiles.add(GeneratedFile("scripts/boot-config.sh", FileType.SHELL))
        }
    }
    
    fun generateAutomationScript(config: CompiledConfig) {
        config.automation?.let { automation ->
            val script = File(outputDir, "scripts/automation-setup.sh")
            script.writeText(buildString {
                appendLine("#!/bin/bash")
                appendLine("# Automation Setup")
                appendLine()
                appendLine("echo 'Setting up automation...'")
                appendLine()
                
                appendLine("# Install automation engine")
                appendLine("pacman -S --needed --noconfirm python python-pip")
                appendLine("pip install automation-framework")
                
                if (automation.workflows.isNotEmpty()) {
                    appendLine()
                    appendLine("# Configure workflows")
                    automation.workflows.forEach { workflow ->
                        appendLine("echo 'Setting up workflow: ${workflow.name}'")
                        appendLine("mkdir -p /etc/horizonos/automation/workflows")
                        appendLine("# Workflow ${workflow.name} configuration")
                    }
                }
                
                appendLine()
                appendLine("echo 'Automation setup completed.'")
            })
            script.setExecutable(true)
            generatedFiles.add(GeneratedFile("scripts/automation-setup.sh", FileType.SHELL))
        }
    }
}