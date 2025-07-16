package org.horizonos.config.compiler

import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import org.horizonos.config.dsl.*
import java.io.File
import java.nio.file.Path
import java.nio.file.Paths

/**
 * Enhanced generator for HorizonOS configurations
 * Generates multiple output formats from compiled configuration
 */
class EnhancedConfigGenerator(private val outputDir: File) {
    
    private val json = Json { 
        prettyPrint = true
        encodeDefaults = true
    }
    
    private val generatedFiles = mutableListOf<GeneratedFile>()
    
    /**
     * Generate all output files from configuration
     */
    fun generate(config: CompiledConfig): GenerationResult {
        try {
            // Create directory structure
            createDirectoryStructure()
            
            // Generate various output formats
            generateJsonOutput(config)
            generateYamlOutput(config)
            generateSystemdUnits(config)
            generateShellScripts(config)
            generateAnsiblePlaybook(config)
            generateDockerfile(config)
            generateOSTreeManifest(config)
            generateAutomationScripts(config)
            generateAIConfiguration(config)
            generateDocumentation(config)
            
            return GenerationResult.Success(generatedFiles.toList())
        } catch (e: Exception) {
            return GenerationResult.Error(GenerationError.UnexpectedError(e.message ?: "Unknown error", e))
        }
    }
    
    private fun createDirectoryStructure() {
        val dirs = listOf(
            "json", "yaml", "scripts", "systemd", "ansible", 
            "docker", "ostree", "automation", "docs", "configs"
        )
        dirs.forEach { 
            File(outputDir, it).mkdirs() 
        }
    }
    
    private fun generateJsonOutput(config: CompiledConfig) {
        val jsonFile = File(outputDir, "json/config.json")
        jsonFile.writeText(json.encodeToString(config))
        generatedFiles.add(GeneratedFile("json/config.json", FileType.JSON))
        
        // Generate separate JSON files for each component
        File(outputDir, "json/system.json").writeText(json.encodeToString(config.system))
        File(outputDir, "json/packages.json").writeText(json.encodeToString(config.packages))
        File(outputDir, "json/services.json").writeText(json.encodeToString(config.services))
        File(outputDir, "json/users.json").writeText(json.encodeToString(config.users))
        File(outputDir, "json/repositories.json").writeText(json.encodeToString(config.repositories))
        
        config.desktop?.let {
            File(outputDir, "json/desktop.json").writeText(json.encodeToString(it))
        }
        
        config.automation?.let {
            File(outputDir, "json/automation.json").writeText(json.encodeToString(it))
        }
        
        config.ai?.let {
            File(outputDir, "json/ai.json").writeText(json.encodeToString(it))
        }
        
        config.boot?.let {
            File(outputDir, "json/boot.json").writeText(json.encodeToString(it))
        }
        
        config.hardware?.let {
            File(outputDir, "json/hardware.json").writeText(json.encodeToString(it))
        }
        
        config.storage?.let {
            File(outputDir, "json/storage.json").writeText(json.encodeToString(it))
        }
        
        config.security?.let {
            File(outputDir, "json/security.json").writeText(json.encodeToString(it))
        }
        
        config.enhancedServices?.let {
            File(outputDir, "json/enhanced-services.json").writeText(json.encodeToString(it))
        }
        
        config.development?.let {
            File(outputDir, "json/development.json").writeText(json.encodeToString(it))
        }
        
        config.environment?.let {
            File(outputDir, "json/environment.json").writeText(json.encodeToString(it))
        }
    }
    
    private fun generateYamlOutput(config: CompiledConfig) {
        // Generate YAML representation (simplified - in production use a YAML library)
        val yamlFile = File(outputDir, "yaml/config.yaml")
        val yaml = buildString {
            appendLine("# HorizonOS Configuration")
            appendLine("# Generated from Kotlin DSL")
            appendLine()
            appendLine("system:")
            appendLine("  hostname: ${config.system.hostname}")
            appendLine("  timezone: ${config.system.timezone}")
            appendLine("  locale: ${config.system.locale}")
            appendLine()
            
            if (config.packages.isNotEmpty()) {
                appendLine("packages:")
                config.packages.forEach { pkg ->
                    appendLine("  - name: ${pkg.name}")
                    appendLine("    action: ${pkg.action.name.lowercase()}")
                    pkg.group?.let { appendLine("    group: $it") }
                }
                appendLine()
            }
            
            if (config.services.isNotEmpty()) {
                appendLine("services:")
                config.services.forEach { service ->
                    appendLine("  - name: ${service.name}")
                    appendLine("    enabled: ${service.enabled}")
                }
                appendLine()
            }
            
            if (config.users.isNotEmpty()) {
                appendLine("users:")
                config.users.forEach { user ->
                    appendLine("  - name: ${user.name}")
                    user.uid?.let { appendLine("    uid: $it") }
                    appendLine("    shell: ${user.shell}")
                    appendLine("    home: ${user.homeDir}")
                    if (user.groups.isNotEmpty()) {
                        appendLine("    groups: [${user.groups.joinToString(", ")}]")
                    }
                }
            }
        }
        
        yamlFile.writeText(yaml)
        generatedFiles.add(GeneratedFile("yaml/config.yaml", FileType.YAML))
    }
    
    private fun generateSystemdUnits(config: CompiledConfig) {
        // Generate systemd service for HorizonOS configuration
        val serviceFile = File(outputDir, "systemd/horizonos-config.service")
        serviceFile.writeText("""
            [Unit]
            Description=HorizonOS Configuration Service
            After=multi-user.target
            
            [Service]
            Type=oneshot
            ExecStart=/usr/bin/horizonos-apply /etc/horizonos/config.json
            RemainAfterExit=yes
            StandardOutput=journal
            StandardError=journal
            
            [Install]
            WantedBy=multi-user.target
        """.trimIndent())
        generatedFiles.add(GeneratedFile("systemd/horizonos-config.service", FileType.SYSTEMD))
        
        // Generate timer for periodic updates
        val timerFile = File(outputDir, "systemd/horizonos-update.timer")
        timerFile.writeText("""
            [Unit]
            Description=HorizonOS Configuration Update Timer
            
            [Timer]
            OnBootSec=5min
            OnUnitActiveSec=1h
            
            [Install]
            WantedBy=timers.target
        """.trimIndent())
        generatedFiles.add(GeneratedFile("systemd/horizonos-update.timer", FileType.SYSTEMD))
        
        // Generate automation service if needed
        config.automation?.let {
            val automationService = File(outputDir, "systemd/horizonos-automation.service")
            automationService.writeText("""
                [Unit]
                Description=HorizonOS Automation Service
                After=network.target
                
                [Service]
                Type=simple
                ExecStart=/usr/bin/horizonos-automation-engine
                Restart=always
                RestartSec=30
                User=horizonos-automation
                
                [Install]
                WantedBy=default.target
            """.trimIndent())
            generatedFiles.add(GeneratedFile("systemd/horizonos-automation.service", FileType.SYSTEMD))
        }
    }
    
    private fun generateShellScripts(config: CompiledConfig) {
        // Main deployment script
        val deployScript = File(outputDir, "scripts/deploy.sh")
        deployScript.writeText(buildString {
            appendLine("#!/bin/bash")
            appendLine("# HorizonOS Deployment Script")
            appendLine("# Generated from Kotlin DSL configuration")
            appendLine()
            appendLine("set -euo pipefail")
            appendLine()
            appendLine("# Color output")
            appendLine("RED='\\033[0;31m'")
            appendLine("GREEN='\\033[0;32m'")
            appendLine("YELLOW='\\033[0;33m'")
            appendLine("NC='\\033[0m' # No Color")
            appendLine()
            appendLine("echo -e \"\${GREEN}Starting HorizonOS deployment...\${NC}\"")
            appendLine()
            appendLine("# Run all configuration scripts")
            appendLine("./system-config.sh")
            appendLine("./package-manager.sh")
            appendLine("./service-manager.sh")
            appendLine("./user-manager.sh")
            appendLine("./repository-config.sh")
            config.boot?.let { appendLine("./boot-config.sh") }
            config.hardware?.let { appendLine("./hardware-config.sh") }
            config.storage?.let { appendLine("./storage-config.sh") }
            config.security?.let { appendLine("./security-config.sh") }
            config.enhancedServices?.let { appendLine("./enhanced-services-config.sh") }
            config.development?.let { appendLine("./development-setup.sh") }
            config.environment?.let { appendLine("./environment-setup.sh") }
            config.desktop?.let { appendLine("./desktop-setup.sh") }
            config.automation?.let { appendLine("./automation-setup.sh") }
            appendLine()
            appendLine("echo -e \"\${GREEN}Deployment completed successfully!\${NC}\"")
        })
        deployScript.setExecutable(true)
        generatedFiles.add(GeneratedFile("scripts/deploy.sh", FileType.SHELL))
        
        // System configuration script
        generateSystemConfigScript(config)
        
        // Package management script
        generatePackageScript(config)
        
        // Service management script
        generateServiceScript(config)
        
        // User management script
        generateUserScript(config)
        
        // Repository configuration script
        generateRepoScript(config)
        
        // Boot configuration script
        config.boot?.let { generateBootScript(config) }
        
        // Hardware configuration script
        config.hardware?.let { generateHardwareScript(config) }
        
        // Storage configuration script
        config.storage?.let { generateStorageScript(config) }
        
        // Security configuration script
        config.security?.let { generateSecurityScript(config) }
        
        // Enhanced services configuration script
        config.enhancedServices?.let { generateEnhancedServicesScript(config) }
        
        // Development environment setup script
        config.development?.let { generateDevelopmentScript(config) }
        
        // Environment setup script
        config.environment?.let { generateEnvironmentScript(config) }
        
        // Desktop setup script
        config.desktop?.let { generateDesktopScript(config) }
        
        // Automation setup script
        config.automation?.let { generateAutomationScript(config) }
    }
    
    private fun generateSystemConfigScript(config: CompiledConfig) {
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
    
    private fun generatePackageScript(config: CompiledConfig) {
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
    
    private fun generateServiceScript(config: CompiledConfig) {
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
    
    private fun generateUserScript(config: CompiledConfig) {
        val script = File(outputDir, "scripts/user-manager.sh")
        script.writeText(buildString {
            appendLine("#!/bin/bash")
            appendLine("# User Management")
            appendLine()
            appendLine("echo 'Managing users...'")
            appendLine()
            
            config.users.forEach { user ->
                appendLine("# Create/update user: ${user.name}")
                append("if ! id '${user.name}' &>/dev/null; then\n")
                append("    useradd -m")
                user.uid?.let { append(" -u $it") }
                append(" -s ${user.shell}")
                if (user.groups.isNotEmpty()) {
                    append(" -G ${user.groups.joinToString(",")}")
                }
                appendLine(" ${user.name}")
                appendLine("else")
                appendLine("    usermod -s ${user.shell} ${user.name}")
                if (user.groups.isNotEmpty()) {
                    appendLine("    usermod -G ${user.groups.joinToString(",")} ${user.name}")
                }
                appendLine("fi")
                appendLine()
            }
            
            appendLine("echo 'User management completed.'")
        })
        script.setExecutable(true)
        generatedFiles.add(GeneratedFile("scripts/user-manager.sh", FileType.SHELL))
    }
    
    private fun generateRepoScript(config: CompiledConfig) {
        val script = File(outputDir, "scripts/repository-config.sh")
        script.writeText(buildString {
            appendLine("#!/bin/bash")
            appendLine("# Repository Configuration")
            appendLine()
            appendLine("echo 'Configuring repositories...'")
            appendLine()
            
            val pacmanRepos = config.repositories.filterIsInstance<PackageRepository>()
            val ostreeRepos = config.repositories.filterIsInstance<OstreeRepository>()
            
            if (pacmanRepos.isNotEmpty()) {
                appendLine("# Configure pacman repositories")
                appendLine("cat >> /etc/pacman.conf <<EOF")
                appendLine()
                pacmanRepos.forEach { repo ->
                    appendLine("[${repo.name}]")
                    appendLine("Server = ${repo.url}")
                    if (!repo.gpgCheck) {
                        appendLine("SigLevel = Never")
                    }
                    appendLine()
                }
                appendLine("EOF")
                appendLine()
                appendLine("pacman -Sy")
            }
            
            if (ostreeRepos.isNotEmpty()) {
                appendLine("# Configure OSTree repositories")
                ostreeRepos.forEach { repo ->
                    appendLine("ostree remote add --if-not-exists '${repo.name}' '${repo.url}'")
                    repo.branches.forEach { branch ->
                        appendLine("# Branch: $branch")
                    }
                }
            }
            
            appendLine()
            appendLine("echo 'Repository configuration completed.'")
        })
        script.setExecutable(true)
        generatedFiles.add(GeneratedFile("scripts/repository-config.sh", FileType.SHELL))
    }
    
    private fun generateBootScript(config: CompiledConfig) {
        config.boot?.let { boot ->
            val script = File(outputDir, "scripts/boot-config.sh")
            script.writeText(buildString {
                appendLine("#!/bin/bash")
                appendLine("# Boot Configuration")
                appendLine()
                appendLine("echo 'Configuring boot system...'")
                appendLine()
                
                // Bootloader configuration
                when (boot.bootloader.type) {
                    BootloaderType.SYSTEMD_BOOT -> {
                        appendLine("# Configure systemd-boot")
                        appendLine("bootctl install || true")
                        appendLine("mkdir -p /boot/loader/entries")
                        appendLine()
                        
                        // Main loader configuration
                        appendLine("cat > /boot/loader/loader.conf <<EOF")
                        appendLine("default      ${boot.bootloader.defaultEntry ?: "horizonos"}")
                        appendLine("timeout      ${boot.bootloader.timeout.inWholeSeconds}")
                        appendLine("console-mode ${boot.bootloader.consoleMode.name.lowercase()}")
                        appendLine("editor       ${if (boot.bootloader.editor) "yes" else "no"}")
                        boot.bootloader.resolution?.let { appendLine("resolution   $it") }
                        appendLine("EOF")
                        appendLine()
                        
                        // Generate boot entries
                        boot.bootloader.entries.forEach { entry ->
                            appendLine("# Boot entry: ${entry.title}")
                            appendLine("cat > /boot/loader/entries/${entry.title.lowercase().replace(" ", "-")}.conf <<EOF")
                            appendLine("title      ${entry.title}")
                            appendLine("linux      ${entry.linux}")
                            entry.initrd?.let { appendLine("initrd     $it") }
                            if (entry.options.isNotEmpty()) {
                                appendLine("options    ${entry.options.joinToString(" ")}")
                            }
                            entry.devicetree?.let { appendLine("devicetree $it") }
                            entry.architecture?.let { appendLine("architecture $it") }
                            entry.version?.let { appendLine("version    $it") }
                            entry.machineId?.let { appendLine("machine-id $it") }
                            appendLine("EOF")
                            appendLine()
                        }
                    }
                    BootloaderType.GRUB -> {
                        appendLine("# Configure GRUB")
                        appendLine("grub-install --target=x86_64-efi --efi-directory=/boot --bootloader-id=HorizonOS")
                        appendLine()
                        
                        boot.bootloader.grubConfig?.let { grub ->
                            appendLine("# Update GRUB configuration")
                            appendLine("sed -i 's/GRUB_DISTRIBUTOR=.*/GRUB_DISTRIBUTOR=\"${grub.distributor}\"/' /etc/default/grub")
                            appendLine("sed -i 's/GRUB_TIMEOUT=.*/GRUB_TIMEOUT=${grub.defaultTimeout.inWholeSeconds}/' /etc/default/grub")
                            grub.theme?.let { appendLine("echo 'GRUB_THEME=\"$it\"' >> /etc/default/grub") }
                            grub.background?.let { appendLine("echo 'GRUB_BACKGROUND=\"$it\"' >> /etc/default/grub") }
                            appendLine("echo 'GRUB_GFXMODE=${grub.gfxMode}' >> /etc/default/grub")
                            appendLine("echo 'GRUB_GFXPAYLOAD=${grub.gfxPayload}' >> /etc/default/grub")
                            if (!grub.recordFailCount) {
                                appendLine("echo 'GRUB_RECORDFAIL_TIMEOUT=0' >> /etc/default/grub")
                            }
                            if (grub.disableRecovery) {
                                appendLine("echo 'GRUB_DISABLE_RECOVERY=true' >> /etc/default/grub")
                            }
                            if (grub.disableOsProber) {
                                appendLine("echo 'GRUB_DISABLE_OS_PROBER=true' >> /etc/default/grub")
                            }
                        }
                        appendLine("grub-mkconfig -o /boot/grub/grub.cfg")
                    }
                    else -> {
                        appendLine("# ${boot.bootloader.type} configuration not yet implemented")
                    }
                }
                appendLine()
                
                // Kernel configuration
                if (boot.kernel.parameters.isNotEmpty()) {
                    appendLine("# Kernel parameters")
                    val kernelCmdline = boot.kernel.parameters.joinToString(" ") { param ->
                        if (param.value != null) "${param.name}=${param.value}" else param.name
                    }
                    appendLine("echo 'Kernel command line: $kernelCmdline'")
                    
                    when (boot.bootloader.type) {
                        BootloaderType.SYSTEMD_BOOT -> {
                            appendLine("# Update systemd-boot entries with kernel parameters")
                            appendLine("find /boot/loader/entries -name '*.conf' -exec sed -i 's/^options.*/options $kernelCmdline/' {} \\;")
                        }
                        BootloaderType.GRUB -> {
                            appendLine("# Update GRUB with kernel parameters")
                            appendLine("sed -i 's/GRUB_CMDLINE_LINUX_DEFAULT=.*/GRUB_CMDLINE_LINUX_DEFAULT=\"$kernelCmdline\"/' /etc/default/grub")
                            appendLine("grub-mkconfig -o /boot/grub/grub.cfg")
                        }
                        else -> {}
                    }
                    appendLine()
                }
                
                // Kernel module configuration
                if (boot.kernel.modules.blacklist.isNotEmpty()) {
                    appendLine("# Blacklist kernel modules")
                    appendLine("cat > /etc/modprobe.d/horizonos-blacklist.conf <<EOF")
                    boot.kernel.modules.blacklist.forEach { module ->
                        appendLine("blacklist $module")
                    }
                    appendLine("EOF")
                    appendLine()
                }
                
                if (boot.kernel.modules.load.isNotEmpty()) {
                    appendLine("# Load kernel modules")
                    appendLine("cat > /etc/modules-load.d/horizonos.conf <<EOF")
                    boot.kernel.modules.load.forEach { module ->
                        appendLine(module)
                    }
                    appendLine("EOF")
                    appendLine()
                }
                
                if (boot.kernel.modules.options.isNotEmpty()) {
                    appendLine("# Module options")
                    appendLine("cat > /etc/modprobe.d/horizonos-options.conf <<EOF")
                    boot.kernel.modules.options.forEach { (module, options) ->
                        appendLine("options $module $options")
                    }
                    appendLine("EOF")
                    appendLine()
                }
                
                // Initramfs configuration
                when (boot.initramfs.generator) {
                    InitramfsGenerator.MKINITCPIO -> {
                        appendLine("# Configure mkinitcpio")
                        if (boot.initramfs.modules.isNotEmpty() || boot.initramfs.hooks.isNotEmpty()) {
                            appendLine("cp /etc/mkinitcpio.conf /etc/mkinitcpio.conf.backup")
                            
                            if (boot.initramfs.modules.isNotEmpty()) {
                                val modules = boot.initramfs.modules.joinToString(" ")
                                appendLine("sed -i 's/MODULES=(.*)/MODULES=($modules)/' /etc/mkinitcpio.conf")
                            }
                            
                            if (boot.initramfs.hooks.isNotEmpty()) {
                                val hooks = boot.initramfs.hooks.joinToString(" ")
                                appendLine("sed -i 's/HOOKS=(.*)/HOOKS=($hooks)/' /etc/mkinitcpio.conf")
                            }
                            
                            appendLine("mkinitcpio -P")
                        }
                    }
                    InitramfsGenerator.DRACUT -> {
                        appendLine("# Configure dracut")
                        appendLine("mkdir -p /etc/dracut.conf.d")
                        appendLine("cat > /etc/dracut.conf.d/horizonos.conf <<EOF")
                        appendLine("compress=\"${boot.initramfs.compression.name.lowercase()}\"")
                        if (boot.initramfs.modules.isNotEmpty()) {
                            appendLine("add_dracutmodules+=\" ${boot.initramfs.modules.joinToString(" ")} \"")
                        }
                        appendLine("EOF")
                        appendLine("dracut --force")
                    }
                    else -> {
                        appendLine("# ${boot.initramfs.generator} configuration not yet implemented")
                    }
                }
                appendLine()
                
                // Plymouth configuration
                if (boot.plymouth.enabled) {
                    appendLine("# Configure Plymouth")
                    appendLine("plymouth-set-default-theme ${boot.plymouth.theme}")
                    if (boot.plymouth.modules.isNotEmpty()) {
                        appendLine("echo 'plymouth.modules=${boot.plymouth.modules.joinToString(",")}' >> /etc/kernel/cmdline")
                    }
                    appendLine("mkinitcpio -P")
                    appendLine()
                }
                
                // Secure Boot configuration
                if (boot.secureBoot.enabled) {
                    appendLine("# Configure Secure Boot")
                    appendLine("echo 'Secure Boot configuration requires manual key enrollment'")
                    appendLine("echo 'Please refer to the documentation for detailed instructions'")
                    boot.secureBoot.keys?.let { keys ->
                        keys.platform?.let { appendLine("# Platform key: $it") }
                        keys.keyExchange?.let { appendLine("# Key exchange key: $it") }
                        keys.signature?.let { appendLine("# Signature database: $it") }
                    }
                    appendLine()
                }
                
                appendLine("echo 'Boot configuration completed.'")
            })
            script.setExecutable(true)
            generatedFiles.add(GeneratedFile("scripts/boot-config.sh", FileType.SHELL))
        }
    }
    
    private fun generateHardwareScript(config: CompiledConfig) {
        config.hardware?.let { hardware ->
            val script = File(outputDir, "scripts/hardware-config.sh")
            script.writeText(buildString {
                appendLine("#!/bin/bash")
                appendLine("# Hardware Configuration")
                appendLine()
                appendLine("echo 'Configuring hardware...'")
                appendLine()
                
                // GPU configuration
                if (hardware.gpu.drivers.isNotEmpty()) {
                    appendLine("# GPU Configuration")
                    hardware.gpu.drivers.forEach { driver ->
                        when (driver.type) {
                            GPUDriver.NVIDIA_PROPRIETARY -> {
                                appendLine("# Configure NVIDIA proprietary driver")
                                appendLine("modprobe nvidia")
                                appendLine("modprobe nvidia_modeset")
                                appendLine("modprobe nvidia_drm")
                                
                                if (driver.options.isNotEmpty()) {
                                    appendLine("# NVIDIA driver options")
                                    appendLine("cat > /etc/modprobe.d/nvidia.conf <<EOF")
                                    driver.options.forEach { (key, value) ->
                                        appendLine("options nvidia $key=$value")
                                    }
                                    appendLine("EOF")
                                }
                                
                                driver.blacklistedDrivers.forEach { blacklisted ->
                                    appendLine("echo 'blacklist $blacklisted' >> /etc/modprobe.d/nvidia-blacklist.conf")
                                }
                            }
                            GPUDriver.AMD_AMDGPU -> {
                                appendLine("# Configure AMD AMDGPU driver")
                                appendLine("modprobe amdgpu")
                                
                                if (driver.options.isNotEmpty()) {
                                    appendLine("# AMDGPU driver options")
                                    appendLine("cat > /etc/modprobe.d/amdgpu.conf <<EOF")
                                    driver.options.forEach { (key, value) ->
                                        appendLine("options amdgpu $key=$value")
                                    }
                                    appendLine("EOF")
                                }
                            }
                            GPUDriver.INTEL -> {
                                appendLine("# Configure Intel graphics")
                                appendLine("modprobe i915")
                                
                                if (driver.options.isNotEmpty()) {
                                    appendLine("# Intel graphics options")
                                    appendLine("cat > /etc/modprobe.d/i915.conf <<EOF")
                                    driver.options.forEach { (key, value) ->
                                        appendLine("options i915 $key=$value")
                                    }
                                    appendLine("EOF")
                                }
                            }
                            else -> {
                                appendLine("# ${driver.type} driver configuration")
                            }
                        }
                    }
                    appendLine()
                }
                
                // Multi-GPU configuration
                hardware.gpu.multiGPU?.let { multiGPU ->
                    appendLine("# Multi-GPU Configuration")
                    when (multiGPU.mode) {
                        MultiGPUMode.OPTIMUS -> {
                            appendLine("# Configure NVIDIA Optimus")
                            appendLine("# Install nvidia-prime for GPU switching")
                            if (multiGPU.switching.runtime) {
                                appendLine("# Enable runtime GPU switching")
                                appendLine("echo 'auto' > /sys/bus/pci/devices/0000:01:00.0/power/control")
                            }
                        }
                        MultiGPUMode.PRIME -> {
                            appendLine("# Configure PRIME offloading")
                            multiGPU.offloading.environmentVariables.forEach { (key, value) ->
                                appendLine("echo 'export $key=$value' >> /etc/environment")
                            }
                        }
                        else -> {
                            appendLine("# ${multiGPU.mode} configuration")
                        }
                    }
                    appendLine()
                }
                
                // Power management configuration
                appendLine("# Power Management Configuration")
                when (hardware.power.cpu.governor) {
                    CPUGovernor.PERFORMANCE -> {
                        appendLine("# Set CPU governor to performance")
                        appendLine("cpupower frequency-set -g performance")
                    }
                    CPUGovernor.POWERSAVE -> {
                        appendLine("# Set CPU governor to powersave")
                        appendLine("cpupower frequency-set -g powersave")
                    }
                    CPUGovernor.SCHEDUTIL -> {
                        appendLine("# Set CPU governor to schedutil")
                        appendLine("cpupower frequency-set -g schedutil")
                    }
                    else -> {
                        appendLine("# Set CPU governor to ${hardware.power.cpu.governor.name.lowercase()}")
                        appendLine("cpupower frequency-set -g ${hardware.power.cpu.governor.name.lowercase()}")
                    }
                }
                
                hardware.power.cpu.minFreq?.let { minFreq ->
                    appendLine("cpupower frequency-set -d $minFreq")
                }
                
                hardware.power.cpu.maxFreq?.let { maxFreq ->
                    appendLine("cpupower frequency-set -u $maxFreq")
                }
                
                if (!hardware.power.cpu.boostEnabled) {
                    appendLine("echo '0' > /sys/devices/system/cpu/cpufreq/boost")
                }
                appendLine()
                
                // Display configuration
                if (hardware.display.monitors.isNotEmpty()) {
                    appendLine("# Display Configuration")
                    hardware.display.monitors.forEach { monitor ->
                        if (monitor.enabled) {
                            appendLine("# Configure monitor: ${monitor.name}")
                            val xrandrCmd = buildString {
                                append("xrandr --output ${monitor.name}")
                                if (monitor.primary) append(" --primary")
                                monitor.resolution?.let { append(" --mode ${it.width}x${it.height}") }
                                monitor.refreshRate?.let { append(" --rate $it") }
                                append(" --pos ${monitor.position.x}x${monitor.position.y}")
                                if (monitor.rotation != ScreenRotation.NORMAL) {
                                    append(" --rotate ${monitor.rotation.name.lowercase()}")
                                }
                                if (monitor.scale != 1.0) {
                                    append(" --scale ${monitor.scale}")
                                }
                            }
                            appendLine(xrandrCmd)
                        } else {
                            appendLine("xrandr --output ${monitor.name} --off")
                        }
                    }
                    appendLine()
                }
                
                // Audio configuration
                when (hardware.audio.system) {
                    AudioSystem.PIPEWIRE -> {
                        appendLine("# Configure PipeWire")
                        appendLine("systemctl --user enable pipewire pipewire-pulse")
                        appendLine("systemctl --user start pipewire pipewire-pulse")
                    }
                    AudioSystem.PULSEAUDIO -> {
                        appendLine("# Configure PulseAudio")
                        appendLine("systemctl --user enable pulseaudio")
                        appendLine("systemctl --user start pulseaudio")
                    }
                    AudioSystem.ALSA -> {
                        appendLine("# Configure ALSA")
                        appendLine("# ALSA configuration")
                    }
                    AudioSystem.JACK -> {
                        appendLine("# Configure JACK")
                        appendLine("systemctl --user enable jack")
                    }
                }
                
                hardware.audio.defaultSink?.let { sink ->
                    appendLine("# Set default audio sink")
                    when (hardware.audio.system) {
                        AudioSystem.PIPEWIRE, AudioSystem.PULSEAUDIO -> {
                            appendLine("pactl set-default-sink $sink")
                        }
                        else -> {
                            appendLine("# Set default sink: $sink")
                        }
                    }
                }
                appendLine()
                
                // Input device configuration
                appendLine("# Input Device Configuration")
                
                // Keyboard configuration
                val keyboard = hardware.input.keyboard
                if (keyboard.layout != "us" || keyboard.variant != null) {
                    appendLine("# Configure keyboard layout")
                    val layoutCmd = buildString {
                        append("localectl set-keymap ${keyboard.layout}")
                        keyboard.variant?.let { append(" $it") }
                    }
                    appendLine(layoutCmd)
                }
                
                // Touchpad configuration
                val touchpad = hardware.input.touchpad
                if (touchpad.enabled) {
                    appendLine("# Configure touchpad")
                    appendLine("xinput set-prop 'SynPS/2 Synaptics TouchPad' 'libinput Tapping Enabled' ${if (touchpad.tapToClick) 1 else 0}")
                    appendLine("xinput set-prop 'SynPS/2 Synaptics TouchPad' 'libinput Natural Scrolling Enabled' ${if (touchpad.naturalScrolling) 1 else 0}")
                    appendLine("xinput set-prop 'SynPS/2 Synaptics TouchPad' 'libinput Disable While Typing Enabled' ${if (touchpad.disableWhileTyping) 1 else 0}")
                }
                appendLine()
                
                // Bluetooth configuration
                if (hardware.bluetooth.enabled) {
                    appendLine("# Bluetooth Configuration")
                    appendLine("systemctl enable bluetooth")
                    appendLine("systemctl start bluetooth")
                    
                    if (hardware.bluetooth.experimental) {
                        appendLine("# Enable experimental Bluetooth features")
                        appendLine("sed -i 's/#Experimental = false/Experimental = true/' /etc/bluetooth/main.conf")
                    }
                    
                    if (hardware.bluetooth.fastConnectable) {
                        appendLine("# Enable fast connectable mode")
                        appendLine("sed -i 's/#FastConnectable = false/FastConnectable = true/' /etc/bluetooth/main.conf")
                    }
                }
                appendLine()
                
                // USB configuration
                if (hardware.usb.autosuspend.enabled) {
                    appendLine("# USB Autosuspend Configuration")
                    appendLine("echo '${hardware.usb.autosuspend.delay.inWholeSeconds}' > /sys/module/usbcore/parameters/autosuspend")
                    
                    hardware.usb.autosuspend.blacklist.forEach { device ->
                        appendLine("echo '$device' >> /sys/bus/usb/drivers/usb/blacklist")
                    }
                }
                appendLine()
                
                // Thermal configuration
                if (hardware.thermal.enabled) {
                    appendLine("# Thermal Management")
                    appendLine("systemctl enable thermald")
                    appendLine("systemctl start thermald")
                    
                    hardware.thermal.zones.forEach { zone ->
                        appendLine("# Configure thermal zone: ${zone.name}")
                        appendLine("echo '${zone.criticalTemp}000' > /sys/class/thermal/${zone.name}/trip_point_0_temp")
                    }
                }
                
                appendLine()
                appendLine("echo 'Hardware configuration completed.'")
            })
            script.setExecutable(true)
            generatedFiles.add(GeneratedFile("scripts/hardware-config.sh", FileType.SHELL))
        }
    }
    
    private fun generateDesktopScript(config: CompiledConfig) {
        config.desktop?.let { desktop ->
            val script = File(outputDir, "scripts/desktop-setup.sh")
            script.writeText(buildString {
                appendLine("#!/bin/bash")
                appendLine("# Desktop Environment Setup")
                appendLine()
                appendLine("echo 'Setting up desktop environment...'")
                appendLine()
                
                when (desktop.environment) {
                    DesktopEnvironment.HYPRLAND -> {
                        appendLine("# Configure Hyprland")
                        desktop.hyprlandConfig?.let { hypr ->
                            appendLine("mkdir -p /etc/hypr")
                            appendLine("cat > /etc/hypr/hyprland.conf <<EOF")
                            appendLine("# HorizonOS Hyprland Configuration")
                            appendLine("general {")
                            appendLine("    gaps_in = ${hypr.gaps}")
                            appendLine("    gaps_out = ${hypr.gaps * 2}")
                            appendLine("    border_size = ${hypr.borderSize}")
                            appendLine("}")
                            appendLine()
                            appendLine("animations {")
                            appendLine("    enabled = ${if (hypr.animations) "yes" else "no"}")
                            appendLine("}")
                            appendLine("EOF")
                        }
                    }
                    DesktopEnvironment.PLASMA -> {
                        appendLine("# Configure Plasma")
                        desktop.plasmaConfig?.let { plasma ->
                            appendLine("# Set Plasma theme")
                            appendLine("kwriteconfig5 --file kdeglobals --group General --key ColorScheme ${plasma.theme}")
                            appendLine("kwriteconfig5 --file kdeglobals --group KDE --key LookAndFeelPackage ${plasma.lookAndFeel}")
                        }
                    }
                    else -> {
                        appendLine("# Desktop environment: ${desktop.environment}")
                    }
                }
                
                if (desktop.autoLogin && desktop.autoLoginUser != null) {
                    appendLine()
                    appendLine("# Configure auto-login")
                    appendLine("mkdir -p /etc/lightdm/lightdm.conf.d")
                    appendLine("cat > /etc/lightdm/lightdm.conf.d/autologin.conf <<EOF")
                    appendLine("[Seat:*]")
                    appendLine("autologin-user=${desktop.autoLoginUser}")
                    appendLine("autologin-user-timeout=0")
                    appendLine("EOF")
                }
                
                appendLine()
                appendLine("echo 'Desktop setup completed.'")
            })
            script.setExecutable(true)
            generatedFiles.add(GeneratedFile("scripts/desktop-setup.sh", FileType.SHELL))
        }
    }
    
    private fun generateAutomationScript(config: CompiledConfig) {
        config.automation?.let { automation ->
            val script = File(outputDir, "scripts/automation-setup.sh")
            script.writeText(buildString {
                appendLine("#!/bin/bash")
                appendLine("# Automation Setup")
                appendLine()
                appendLine("echo 'Setting up automation...'")
                appendLine()
                appendLine("# Create automation directory")
                appendLine("mkdir -p /etc/horizonos/automation")
                appendLine()
                appendLine("# Copy automation configuration")
                appendLine("cp ../json/automation.json /etc/horizonos/automation/")
                appendLine()
                appendLine("# Enable automation service")
                appendLine("systemctl enable horizonos-automation.service")
                appendLine()
                appendLine("echo 'Automation setup completed.'")
            })
            script.setExecutable(true)
            generatedFiles.add(GeneratedFile("scripts/automation-setup.sh", FileType.SHELL))
        }
    }
    
    private fun generateAnsiblePlaybook(config: CompiledConfig) {
        val playbook = File(outputDir, "ansible/horizonos-playbook.yml")
        playbook.writeText(buildString {
            appendLine("---")
            appendLine("# HorizonOS Ansible Playbook")
            appendLine("# Generated from Kotlin DSL configuration")
            appendLine()
            appendLine("- name: Configure HorizonOS System")
            appendLine("  hosts: all")
            appendLine("  become: yes")
            appendLine("  tasks:")
            appendLine()
            appendLine("    - name: Set hostname")
            appendLine("      hostname:")
            appendLine("        name: ${config.system.hostname}")
            appendLine()
            appendLine("    - name: Set timezone")
            appendLine("      timezone:")
            appendLine("        name: ${config.system.timezone}")
            appendLine()
            
            if (config.packages.isNotEmpty()) {
                val toInstall = config.packages.filter { it.action == PackageAction.INSTALL }
                if (toInstall.isNotEmpty()) {
                    appendLine("    - name: Install packages")
                    appendLine("      pacman:")
                    appendLine("        name:")
                    toInstall.forEach { pkg ->
                        appendLine("          - ${pkg.name}")
                    }
                    appendLine("        state: present")
                    appendLine()
                }
            }
            
            config.services.forEach { service ->
                appendLine("    - name: Configure service ${service.name}")
                appendLine("      systemd:")
                appendLine("        name: ${service.name}")
                appendLine("        enabled: ${if (service.enabled) "yes" else "no"}")
                appendLine("        state: ${if (service.enabled) "started" else "stopped"}")
                appendLine()
            }
            
            config.users.forEach { user ->
                appendLine("    - name: Create user ${user.name}")
                appendLine("      user:")
                appendLine("        name: ${user.name}")
                user.uid?.let { appendLine("        uid: $it") }
                appendLine("        shell: ${user.shell}")
                appendLine("        home: ${user.homeDir}")
                if (user.groups.isNotEmpty()) {
                    appendLine("        groups: ${user.groups.joinToString(",")}")
                }
                appendLine()
            }
        })
        generatedFiles.add(GeneratedFile("ansible/horizonos-playbook.yml", FileType.ANSIBLE))
    }
    
    private fun generateDockerfile(config: CompiledConfig) {
        val dockerfile = File(outputDir, "docker/Dockerfile")
        dockerfile.writeText(buildString {
            appendLine("# HorizonOS Container Image")
            appendLine("# Generated from Kotlin DSL configuration")
            appendLine()
            appendLine("FROM archlinux:latest")
            appendLine()
            appendLine("# Set system configuration")
            appendLine("ENV TZ=${config.system.timezone}")
            appendLine("ENV LANG=${config.system.locale}")
            appendLine()
            
            val toInstall = config.packages.filter { it.action == PackageAction.INSTALL }
            if (toInstall.isNotEmpty()) {
                appendLine("# Install packages")
                appendLine("RUN pacman -Syu --noconfirm && \\")
                appendLine("    pacman -S --noconfirm \\")
                toInstall.forEach { pkg ->
                    append("        ${pkg.name}")
                    if (pkg != toInstall.last()) append(" \\")
                    appendLine()
                }
                appendLine()
            }
            
            appendLine("# Copy configuration")
            appendLine("COPY json/config.json /etc/horizonos/config.json")
            appendLine()
            appendLine("# Entry point")
            appendLine("CMD [\"/bin/bash\"]")
        })
        generatedFiles.add(GeneratedFile("docker/Dockerfile", FileType.DOCKER))
        
        // Generate docker-compose.yml
        val compose = File(outputDir, "docker/docker-compose.yml")
        compose.writeText("""
            version: '3.8'
            services:
              horizonos:
                build: .
                hostname: ${config.system.hostname}
                environment:
                  - TZ=${config.system.timezone}
                  - LANG=${config.system.locale}
                volumes:
                  - horizonos_data:/data
            
            volumes:
              horizonos_data:
        """.trimIndent())
        generatedFiles.add(GeneratedFile("docker/docker-compose.yml", FileType.DOCKER))
    }
    
    private fun generateOSTreeManifest(config: CompiledConfig) {
        val manifest = File(outputDir, "ostree/manifest.json")
        manifest.writeText(json.encodeToString(OSTreeManifest(
            ref = "horizonos/stable/x86_64",
            metadata = mapOf(
                "version" to "1.0.0",
                "variant" to "desktop"
            ),
            packages = config.packages.filter { it.action == PackageAction.INSTALL }.map { it.name },
            repos = config.repositories.map { it.name }
        )))
        generatedFiles.add(GeneratedFile("ostree/manifest.json", FileType.JSON))
        
        // Generate OSTree build script
        val buildScript = File(outputDir, "ostree/build-ostree.sh")
        buildScript.writeText("""
            #!/bin/bash
            # Build OSTree commit from configuration
            
            set -euo pipefail
            
            REPO_PATH="${'$'}{REPO_PATH:-/ostree/repo}"
            BRANCH="horizonos/stable/x86_64"
            
            # Create temporary root
            TMPDIR=$(mktemp -d)
            trap "rm -rf ${'$'}TMPDIR" EXIT
            
            # Apply configuration
            ../scripts/deploy.sh
            
            # Create OSTree commit
            ostree commit \
                --repo=${'$'}REPO_PATH \
                --branch=${'$'}BRANCH \
                --subject="HorizonOS configuration update" \
                --tree=dir=${'$'}TMPDIR
            
            echo "OSTree commit created successfully"
        """.trimIndent())
        buildScript.setExecutable(true)
        generatedFiles.add(GeneratedFile("ostree/build-ostree.sh", FileType.SHELL))
    }
    
    private fun generateAutomationScripts(config: CompiledConfig) {
        config.automation?.let { automation ->
            // Generate individual workflow scripts
            automation.workflows.forEach { workflow ->
                val workflowScript = File(outputDir, "automation/workflow-${workflow.name}.sh")
                workflowScript.writeText(buildString {
                    appendLine("#!/bin/bash")
                    appendLine("# Workflow: ${workflow.name}")
                    workflow.description.let { appendLine("# $it") }
                    appendLine()
                    appendLine("# This is a placeholder for workflow execution")
                    appendLine("# Actual execution would be handled by the automation engine")
                    appendLine()
                    appendLine("echo 'Executing workflow: ${workflow.name}'")
                })
                workflowScript.setExecutable(true)
                generatedFiles.add(GeneratedFile("automation/workflow-${workflow.name}.sh", FileType.SHELL))
            }
        }
    }
    
    private fun generateAIConfiguration(config: CompiledConfig) {
        config.ai?.let { aiConfig ->
            if (!aiConfig.enabled) return@let
            
            // Generate Ollama configuration
            val ollamaConfig = File(outputDir, "ai/ollama-config.json")
            ollamaConfig.parentFile.mkdirs()
            
            val ollamaProviders = aiConfig.providers.filter { it.type == ProviderType.OLLAMA }
            if (ollamaProviders.isNotEmpty()) {
                val ollamaSettings = mapOf(
                    "models" to aiConfig.models.filter { model ->
                        ollamaProviders.any { it.name == model.provider }
                    }.map { model ->
                        mapOf(
                            "name" to model.name,
                            "size" to model.size.name.lowercase(),
                            "quantization" to model.quantization.name.lowercase(),
                            "preload" to model.preload,
                            "parameters" to model.parameters
                        )
                    },
                    "hardware" to mapOf(
                        "gpu_acceleration" to aiConfig.hardware.gpuAcceleration,
                        "cpu_threads" to aiConfig.hardware.cpuThreads,
                        "memory_limit" to aiConfig.hardware.memoryLimit
                    ),
                    "privacy" to mapOf(
                        "local_only" to aiConfig.privacy.localOnly,
                        "telemetry_enabled" to aiConfig.privacy.telemetryEnabled,
                        "data_retention" to aiConfig.privacy.dataRetention.name.lowercase()
                    )
                )
                ollamaConfig.writeText(json.encodeToString(ollamaSettings))
                generatedFiles.add(GeneratedFile("ai/ollama-config.json", FileType.JSON))
            }
            
            // Generate AI services configuration
            if (aiConfig.services.isNotEmpty()) {
                val servicesConfig = File(outputDir, "ai/services.json")
                servicesConfig.writeText(json.encodeToString(aiConfig.services))
                generatedFiles.add(GeneratedFile("ai/services.json", FileType.JSON))
            }
            
            // Generate AI setup script
            val setupScript = File(outputDir, "scripts/ai-setup.sh")
            setupScript.writeText(buildString {
                appendLine("#!/bin/bash")
                appendLine("# AI/LLM Setup Script")
                appendLine("# Generated from HorizonOS Kotlin DSL")
                appendLine()
                appendLine("set -euo pipefail")
                appendLine()
                appendLine("echo 'Setting up AI/LLM services...'")
                appendLine()
                
                // Install Ollama if needed
                if (ollamaProviders.isNotEmpty()) {
                    appendLine("# Install Ollama")
                    appendLine("if ! command -v ollama &> /dev/null; then")
                    appendLine("    curl -fsSL https://ollama.ai/install.sh | sh")
                    appendLine("fi")
                    appendLine()
                    
                    appendLine("# Start Ollama service")
                    appendLine("systemctl --user enable ollama")
                    appendLine("systemctl --user start ollama")
                    appendLine()
                    
                    // Pull models
                    aiConfig.models.filter { model ->
                        ollamaProviders.any { it.name == model.provider }
                    }.forEach { model ->
                        appendLine("# Pull model: ${model.name}")
                        appendLine("ollama pull ${model.name}")
                    }
                    appendLine()
                }
                
                // Configure hardware optimization
                when (aiConfig.hardware.optimization) {
                    HardwareOptimization.GPU_NVIDIA -> {
                        appendLine("# NVIDIA GPU optimization")
                        appendLine("export CUDA_VISIBLE_DEVICES=0")
                        appendLine("export OLLAMA_GPU=1")
                    }
                    HardwareOptimization.GPU_AMD -> {
                        appendLine("# AMD GPU optimization") 
                        appendLine("export HSA_OVERRIDE_GFX_VERSION=10.3.0")
                        appendLine("export OLLAMA_GPU=1")
                    }
                    HardwareOptimization.CPU_ONLY -> {
                        appendLine("# CPU-only execution")
                        appendLine("export OLLAMA_GPU=0")
                        appendLine("export OMP_NUM_THREADS=${aiConfig.hardware.cpuThreads}")
                    }
                    else -> {
                        appendLine("# Auto hardware detection")
                    }
                }
                appendLine()
                
                // Create AI configuration directory
                appendLine("# Create AI configuration directory")
                appendLine("mkdir -p /etc/horizonos/ai")
                appendLine("cp ../ai/*.json /etc/horizonos/ai/ 2>/dev/null || true")
                appendLine()
                
                appendLine("echo 'AI/LLM setup completed.'")
            })
            setupScript.setExecutable(true)
            generatedFiles.add(GeneratedFile("scripts/ai-setup.sh", FileType.SHELL))
            
            // Generate systemd service for AI services
            val aiService = File(outputDir, "systemd/horizonos-ai.service")
            aiService.writeText("""
                [Unit]
                Description=HorizonOS AI Services
                After=network.target ollama.service
                Wants=ollama.service
                
                [Service]
                Type=simple
                ExecStart=/usr/bin/horizonos-ai-manager
                Restart=always
                RestartSec=30
                User=ai
                Group=ai
                Environment=HORIZONOS_AI_CONFIG=/etc/horizonos/ai
                
                [Install]
                WantedBy=multi-user.target
            """.trimIndent())
            generatedFiles.add(GeneratedFile("systemd/horizonos-ai.service", FileType.SYSTEMD))
        }
    }
    
    private fun generateDocumentation(config: CompiledConfig) {
        val readme = File(outputDir, "docs/README.md")
        readme.writeText(buildString {
            appendLine("# HorizonOS Configuration")
            appendLine()
            appendLine("This configuration was generated from a HorizonOS Kotlin DSL file.")
            appendLine()
            appendLine("## System Information")
            appendLine()
            appendLine("- **Hostname**: ${config.system.hostname}")
            appendLine("- **Timezone**: ${config.system.timezone}")
            appendLine("- **Locale**: ${config.system.locale}")
            appendLine()
            
            if (config.packages.isNotEmpty()) {
                appendLine("## Packages")
                appendLine()
                val byAction = config.packages.groupBy { it.action }
                byAction[PackageAction.INSTALL]?.let { packages ->
                    appendLine("### To Install (${packages.size})")
                    packages.forEach { appendLine("- ${it.name}") }
                    appendLine()
                }
                byAction[PackageAction.REMOVE]?.let { packages ->
                    appendLine("### To Remove (${packages.size})")
                    packages.forEach { appendLine("- ${it.name}") }
                    appendLine()
                }
            }
            
            if (config.services.isNotEmpty()) {
                appendLine("## Services")
                appendLine()
                val byStatus = config.services.groupBy { it.enabled }
                byStatus[true]?.let { services ->
                    appendLine("### Enabled (${services.size})")
                    services.forEach { appendLine("- ${it.name}") }
                    appendLine()
                }
                byStatus[false]?.let { services ->
                    appendLine("### Disabled (${services.size})")
                    services.forEach { appendLine("- ${it.name}") }
                    appendLine()
                }
            }
            
            if (config.users.isNotEmpty()) {
                appendLine("## Users")
                appendLine()
                config.users.forEach { user ->
                    appendLine("### ${user.name}")
                    user.uid?.let { appendLine("- UID: $it") }
                    appendLine("- Shell: ${user.shell}")
                    appendLine("- Home: ${user.homeDir}")
                    if (user.groups.isNotEmpty()) {
                        appendLine("- Groups: ${user.groups.joinToString(", ")}")
                    }
                    appendLine()
                }
            }
            
            config.desktop?.let { desktop ->
                appendLine("## Desktop Environment")
                appendLine()
                appendLine("- **Environment**: ${desktop.environment}")
                appendLine("- **Auto-login**: ${if (desktop.autoLogin) "Yes" else "No"}")
                desktop.autoLoginUser?.let { appendLine("- **Auto-login User**: $it") }
                appendLine()
            }
            
            config.automation?.let { automation ->
                appendLine("## Automation")
                appendLine()
                if (automation.workflows.isNotEmpty()) {
                    appendLine("### Workflows (${automation.workflows.size})")
                    automation.workflows.forEach { workflow ->
                        appendLine("- **${workflow.name}**${if (!workflow.enabled) " (disabled)" else ""}")
                        workflow.description.takeIf { it.isNotEmpty() }?.let { appendLine("  - $it") }
                    }
                    appendLine()
                }
                if (automation.teachingModes.isNotEmpty()) {
                    appendLine("### Teaching Modes (${automation.teachingModes.size})")
                    automation.teachingModes.forEach { teaching ->
                        appendLine("- **${teaching.name}**${if (!teaching.enabled) " (disabled)" else ""}")
                        teaching.description.takeIf { it.isNotEmpty() }?.let { appendLine("  - $it") }
                    }
                }
            }
            
            config.ai?.let { ai ->
                if (ai.enabled) {
                    appendLine()
                    appendLine("## AI/LLM Integration")
                    appendLine()
                    appendLine("- **Status**: Enabled")
                    
                    if (ai.models.isNotEmpty()) {
                        appendLine("### Models (${ai.models.size})")
                        ai.models.forEach { model ->
                            appendLine("- **${model.name}**${if (!model.enabled) " (disabled)" else ""}")
                            appendLine("  - Provider: ${model.provider}")
                            appendLine("  - Size: ${model.size}")
                            appendLine("  - Quantization: ${model.quantization}")
                            if (model.capabilities.isNotEmpty()) {
                                appendLine("  - Capabilities: ${model.capabilities.joinToString(", ")}")
                            }
                            if (model.preload) {
                                appendLine("  - Preloaded at startup")
                            }
                        }
                        appendLine()
                    }
                    
                    if (ai.providers.isNotEmpty()) {
                        appendLine("### Providers (${ai.providers.size})")
                        ai.providers.forEach { provider ->
                            appendLine("- **${provider.name}** (${provider.type})")
                            appendLine("  - Endpoint: ${provider.endpoint}:${provider.port}")
                            if (provider.models.isNotEmpty()) {
                                appendLine("  - Available Models: ${provider.models.joinToString(", ")}")
                            }
                        }
                        appendLine()
                    }
                    
                    if (ai.services.isNotEmpty()) {
                        appendLine("### Services (${ai.services.size})")
                        ai.services.forEach { service ->
                            appendLine("- **${service.name}**")
                            if (service.description.isNotEmpty()) {
                                appendLine("  - ${service.description}")
                            }
                            appendLine("  - Model: ${service.model}")
                            appendLine("  - Temperature: ${service.temperature}")
                            appendLine("  - Max Tokens: ${service.maxTokens}")
                        }
                        appendLine()
                    }
                    
                    appendLine("### Hardware Configuration")
                    appendLine("- **GPU Acceleration**: ${if (ai.hardware.gpuAcceleration) "Enabled" else "Disabled"}")
                    appendLine("- **CPU Threads**: ${if (ai.hardware.cpuThreads == 0) "Auto-detect" else ai.hardware.cpuThreads}")
                    appendLine("- **Memory Limit**: ${ai.hardware.memoryLimit}")
                    appendLine("- **Optimization**: ${ai.hardware.optimization}")
                    appendLine("- **Backends**: ${ai.hardware.backends.joinToString(", ")}")
                    appendLine()
                    
                    appendLine("### Privacy Settings")
                    appendLine("- **Local Only**: ${if (ai.privacy.localOnly) "Yes" else "No"}")
                    appendLine("- **Telemetry**: ${if (ai.privacy.telemetryEnabled) "Enabled" else "Disabled"}")
                    appendLine("- **Data Retention**: ${ai.privacy.dataRetention}")
                    appendLine("- **Storage Encryption**: ${if (ai.privacy.encryptStorage) "Enabled" else "Disabled"}")
                    if (ai.privacy.allowedNetworkAccess.isNotEmpty()) {
                        appendLine("- **Allowed Network Access**: ${ai.privacy.allowedNetworkAccess.joinToString(", ")}")
                    }
                }
            }
            
            appendLine()
            appendLine("## Deployment")
            appendLine()
            appendLine("To deploy this configuration:")
            appendLine()
            appendLine("```bash")
            appendLine("cd scripts")
            appendLine("sudo ./deploy.sh")
            appendLine("```")
            appendLine()
            appendLine("## Files Generated")
            appendLine()
            generatedFiles.groupBy { it.type }.forEach { (type, files) ->
                appendLine("### ${type.displayName}")
                files.forEach { appendLine("- `${it.path}`") }
                appendLine()
            }
        })
        generatedFiles.add(GeneratedFile("docs/README.md", FileType.DOCUMENTATION))
    }
    
    private fun generateStorageScript(config: CompiledConfig) {
        config.storage?.let { storage ->
            val script = File(outputDir, "scripts/storage-config.sh")
            script.writeText(buildString {
                appendLine("#!/bin/bash")
                appendLine("# Storage Configuration")
                appendLine("# Generated from HorizonOS Kotlin DSL")
                appendLine()
                appendLine("set -euo pipefail")
                appendLine()
                appendLine("echo 'Configuring storage...'")
                appendLine()
                
                // Filesystem configuration
                if (storage.filesystems.isNotEmpty()) {
                    appendLine("# Filesystem Configuration")
                    storage.filesystems.forEach { fs ->
                        if (fs.enabled) {
                            appendLine("# Configure filesystem: ${fs.mountPoint}")
                            appendLine("mkdir -p ${fs.mountPoint}")
                            
                            // Build mount options
                            val mountOptions = mutableListOf<String>()
                            mountOptions.addAll(fs.options.standard)
                            
                            // Add security options
                            if (fs.options.security.noexec) mountOptions.add("noexec")
                            if (fs.options.security.nosuid) mountOptions.add("nosuid")
                            if (fs.options.security.nodev) mountOptions.add("nodev")
                            if (fs.options.security.relatime) mountOptions.add("relatime")
                            if (fs.options.security.ro) mountOptions.add("ro")
                            
                            // Add performance options
                            if (fs.options.performance.noatime) mountOptions.add("noatime")
                            if (fs.options.performance.nodiratime) mountOptions.add("nodiratime")
                            fs.options.performance.commit?.let { mountOptions.add("commit=$it") }
                            
                            val optionsStr = if (mountOptions.isNotEmpty()) "-o ${mountOptions.joinToString(",")}" else ""
                            
                            if (fs.bootMount) {
                                appendLine("# Add to fstab for boot mount")
                                val fstabEntry = "${fs.device} ${fs.mountPoint} ${fs.type.name.lowercase()} ${mountOptions.joinToString(",").ifEmpty { "defaults" }} ${fs.backupFrequency} ${fs.fsckOrder}"
                                appendLine("echo '$fstabEntry' >> /etc/fstab")
                            } else {
                                appendLine("mount -t ${fs.type.name.lowercase()} $optionsStr ${fs.device} ${fs.mountPoint}")
                            }
                            
                            fs.label?.let { label ->
                                appendLine("# Set filesystem label")
                                when (fs.type) {
                                    FilesystemType.EXT4, FilesystemType.EXT3, FilesystemType.EXT2 -> {
                                        appendLine("e2label ${fs.device} $label")
                                    }
                                    FilesystemType.XFS -> {
                                        appendLine("xfs_admin -L $label ${fs.device}")
                                    }
                                    FilesystemType.BTRFS -> {
                                        appendLine("btrfs filesystem label ${fs.device} $label")
                                    }
                                    else -> {
                                        appendLine("# Label setting not supported for ${fs.type}")
                                    }
                                }
                            }
                            appendLine()
                        }
                    }
                }
                
                // RAID configuration
                if (storage.raid.enabled && storage.raid.arrays.isNotEmpty()) {
                    appendLine("# RAID Configuration")
                    storage.raid.arrays.forEach { raid ->
                        appendLine("# Create RAID array: ${raid.name}")
                        val raidCmd = buildString {
                            append("mdadm --create /dev/md/${raid.name}")
                            append(" --level=${raid.level.name.replace("RAID", "")}")
                            append(" --raid-devices=${raid.devices.size}")
                            if (raid.spares.isNotEmpty()) {
                                append(" --spare-devices=${raid.spares.size}")
                            }
                            raid.chunkSize?.let { append(" --chunk=$it") }
                            append(" --metadata=${raid.metadata.name.replace("_", ".")}")
                            append(" ${raid.devices.joinToString(" ")}")
                            if (raid.spares.isNotEmpty()) {
                                append(" ${raid.spares.joinToString(" ")}")
                            }
                        }
                        appendLine(raidCmd)
                        
                        // Configure RAID bitmap
                        raid.bitmap?.let { bitmap ->
                            if (bitmap.enabled) {
                                appendLine("mdadm --grow --bitmap=${bitmap.location} /dev/md/${raid.name}")
                            }
                        }
                        appendLine()
                    }
                    
                    // RAID monitoring
                    if (storage.raid.monitoring.enabled) {
                        appendLine("# Configure RAID monitoring")
                        appendLine("systemctl enable mdmonitor.service")
                        appendLine("systemctl start mdmonitor.service")
                        
                        storage.raid.monitoring.emailAddress?.let { email ->
                            appendLine("# Configure email notifications")
                            appendLine("echo 'MAILADDR $email' >> /etc/mdadm.conf")
                        }
                        appendLine()
                    }
                }
                
                // LUKS encryption configuration
                if (storage.encryption.enabled && storage.encryption.volumes.isNotEmpty()) {
                    appendLine("# LUKS Encryption Configuration")
                    storage.encryption.volumes.forEach { volume ->
                        appendLine("# Setup encrypted volume: ${volume.name}")
                        
                        val luksCmd = buildString {
                            append("cryptsetup luksFormat")
                            append(" --type luks2")
                            append(" --cipher ${volume.cipher.name.replace("_", "-").lowercase()}")
                            append(" --key-size ${volume.keySize}")
                            append(" --hash ${volume.hashAlgorithm.name.lowercase()}")
                            append(" --pbkdf ${volume.pbkdf.algorithm.name.lowercase()}")
                            volume.pbkdf.iterations?.let { append(" --pbkdf-force-iterations $it") }
                            volume.pbkdf.memory?.let { append(" --pbkdf-memory $it") }
                            volume.pbkdf.parallelism?.let { append(" --pbkdf-parallel $it") }
                            append(" ${volume.device}")
                        }
                        appendLine(luksCmd)
                        
                        // Open encrypted volume
                        appendLine("cryptsetup luksOpen ${volume.device} ${volume.name}")
                        
                        // Add to crypttab if needed
                        appendLine("echo '${volume.name} ${volume.device} none luks' >> /etc/crypttab")
                        appendLine()
                    }
                    
                    // TPM configuration
                    if (storage.encryption.tpm.enabled) {
                        appendLine("# Configure TPM-based encryption")
                        appendLine("# TPM ${storage.encryption.tpm.version} configuration")
                        storage.encryption.tpm.keyHandle?.let { handle ->
                            appendLine("# TPM key handle: $handle")
                        }
                        appendLine()
                    }
                }
                
                // Btrfs configuration
                if (storage.btrfs.enabled && storage.btrfs.filesystems.isNotEmpty()) {
                    appendLine("# Btrfs Configuration")
                    storage.btrfs.filesystems.forEach { btrfs ->
                        appendLine("# Create Btrfs filesystem: ${btrfs.label}")
                        
                        val btrfsCmd = buildString {
                            append("mkfs.btrfs")
                            append(" --label ${btrfs.label}")
                            append(" --data ${btrfs.dataProfile.name.lowercase()}")
                            append(" --metadata ${btrfs.metadataProfile.name.lowercase()}")
                            append(" ${btrfs.devices.joinToString(" ")}")
                        }
                        appendLine(btrfsCmd)
                        
                        // Create subvolumes
                        if (btrfs.subvolumes.isNotEmpty()) {
                            appendLine("# Create subvolumes")
                            val mountPoint = "/mnt/${btrfs.label}"
                            appendLine("mkdir -p $mountPoint")
                            appendLine("mount ${btrfs.devices.first()} $mountPoint")
                            
                            btrfs.subvolumes.forEach { subvol ->
                                appendLine("btrfs subvolume create $mountPoint/${subvol.name}")
                                
                                if (subvol.defaultSubvolume) {
                                    appendLine("btrfs subvolume set-default $mountPoint/${subvol.name}")
                                }
                                
                                subvol.quota?.let { quota ->
                                    if (quota.enabled) {
                                        appendLine("btrfs quota enable $mountPoint")
                                        quota.sizeLimit?.let { limit ->
                                            appendLine("btrfs qgroup limit $limit $mountPoint/${subvol.name}")
                                        }
                                    }
                                }
                            }
                            
                            appendLine("umount $mountPoint")
                        }
                        appendLine()
                    }
                    
                    // Btrfs maintenance
                    if (storage.btrfs.scrubbing.enabled) {
                        appendLine("# Configure Btrfs scrubbing")
                        appendLine("systemctl enable btrfs-scrub@-.timer")
                        appendLine("systemctl start btrfs-scrub@-.timer")
                        appendLine()
                    }
                }
                
                // Swap configuration
                if (storage.swap.enabled) {
                    appendLine("# Swap Configuration")
                    
                    when (storage.swap.type) {
                        SwapType.ZRAM -> {
                            appendLine("# Configure ZRAM swap")
                            appendLine("modprobe zram")
                            appendLine("echo '${storage.swap.zram.algorithm.name.lowercase()}' > /sys/block/zram0/comp_algorithm")
                            appendLine("echo '${storage.swap.zram.size}' > /sys/block/zram0/disksize")
                            appendLine("mkswap /dev/zram0")
                            appendLine("swapon /dev/zram0 -p ${storage.swap.zram.priority}")
                        }
                        SwapType.FILE -> {
                            storage.swap.files.forEach { swapFile ->
                                appendLine("# Create swap file: ${swapFile.path}")
                                appendLine("fallocate -l ${swapFile.size} ${swapFile.path}")
                                appendLine("chmod ${swapFile.permissions} ${swapFile.path}")
                                appendLine("mkswap ${swapFile.path}")
                                appendLine("swapon ${swapFile.path} -p ${swapFile.priority}")
                                appendLine("echo '${swapFile.path} none swap sw,pri=${swapFile.priority} 0 0' >> /etc/fstab")
                            }
                        }
                        SwapType.PARTITION -> {
                            storage.swap.partitions.forEach { partition ->
                                appendLine("# Enable swap partition: ${partition.device}")
                                appendLine("mkswap ${partition.device}")
                                appendLine("swapon ${partition.device} -p ${partition.priority}")
                                appendLine("echo '${partition.device} none swap sw,pri=${partition.priority} 0 0' >> /etc/fstab")
                            }
                        }
                        else -> {
                            appendLine("# Swap type ${storage.swap.type} configuration")
                        }
                    }
                    
                    // Configure swappiness
                    appendLine("echo 'vm.swappiness=${storage.swap.swappiness}' >> /etc/sysctl.conf")
                    appendLine("echo 'vm.vfs_cache_pressure=${storage.swap.vfsCache}' >> /etc/sysctl.conf")
                    appendLine()
                }
                
                // Storage maintenance
                if (storage.maintenance.enabled) {
                    appendLine("# Storage Maintenance Configuration")
                    
                    if (storage.maintenance.fsck.enabled) {
                        appendLine("# Configure filesystem check")
                        appendLine("systemctl enable fsck@.service")
                    }
                    
                    if (storage.maintenance.trim.enabled) {
                        appendLine("# Configure SSD TRIM")
                        appendLine("systemctl enable fstrim.timer")
                        appendLine("systemctl start fstrim.timer")
                    }
                    
                    if (storage.maintenance.healthChecks.enabled) {
                        appendLine("# Configure storage health checks")
                        if (storage.maintenance.healthChecks.smart.enabled) {
                            appendLine("systemctl enable smartd.service")
                            appendLine("systemctl start smartd.service")
                        }
                    }
                    appendLine()
                }
                
                // Auto-mount configuration
                if (storage.autoMount.enabled) {
                    appendLine("# Auto-mount Configuration")
                    
                    if (storage.autoMount.removableMedia.enabled) {
                        appendLine("# Configure removable media auto-mount")
                        appendLine("systemctl enable udisks2.service")
                        appendLine("systemctl start udisks2.service")
                    }
                    
                    if (storage.autoMount.networkShares.enabled) {
                        appendLine("# Configure network shares auto-mount")
                        appendLine("systemctl enable autofs.service")
                        appendLine("systemctl start autofs.service")
                    }
                    appendLine()
                }
                
                appendLine("echo 'Storage configuration completed.'")
            })
            script.setExecutable(true)
            generatedFiles.add(GeneratedFile("scripts/storage-config.sh", FileType.SHELL))
        }
    }
    
    private fun generateSecurityScript(config: CompiledConfig) {
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
                if (security.ssh.enabled) {
                    appendLine("# SSH Configuration")
                    appendLine("cp /etc/ssh/sshd_config /etc/ssh/sshd_config.backup")
                    appendLine()
                    appendLine("cat > /etc/ssh/sshd_config <<EOF")
                    appendLine("# HorizonOS SSH Configuration")
                    appendLine("Port ${security.ssh.port}")
                    security.ssh.listenAddress.forEach { addr ->
                        appendLine("ListenAddress $addr")
                    }
                    appendLine("Protocol ${security.ssh.protocol.name.lowercase()}")
                    appendLine()
                    
                    // Authentication settings
                    appendLine("# Authentication")
                    appendLine("PubkeyAuthentication ${if (security.ssh.authentication.publicKey) "yes" else "no"}")
                    appendLine("PasswordAuthentication ${if (security.ssh.authentication.password) "yes" else "no"}")
                    appendLine("KerberosAuthentication ${if (security.ssh.authentication.kerberos) "yes" else "no"}")
                    appendLine("GSSAPIAuthentication ${if (security.ssh.authentication.gssapi) "yes" else "no"}")
                    appendLine("HostbasedAuthentication ${if (security.ssh.authentication.hostbased) "yes" else "no"}")
                    appendLine("ChallengeResponseAuthentication ${if (security.ssh.authentication.challenge) "yes" else "no"}")
                    appendLine("MaxAuthTries ${security.ssh.authentication.maxAuthTries}")
                    appendLine("LoginGraceTime ${security.ssh.authentication.loginGraceTime.inWholeSeconds}")
                    appendLine("PermitEmptyPasswords ${if (security.ssh.authentication.permitEmptyPasswords) "yes" else "no"}")
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
                    appendLine("PermitRootLogin ${security.ssh.access.permitRoot.name.lowercase().replace("_", "-")}")
                    appendLine("MaxSessions ${security.ssh.access.maxSessions}")
                    appendLine("MaxStartups ${security.ssh.access.maxStartups}")
                    appendLine()
                    
                    // Security settings
                    appendLine("# Security")
                    appendLine("StrictModes ${if (security.ssh.security.strictModes) "yes" else "no"}")
                    appendLine("IgnoreRhosts ${if (security.ssh.security.ignorerhosts) "yes" else "no"}")
                    appendLine("IgnoreUserKnownHosts ${if (security.ssh.security.ignoreUserKnownHosts) "yes" else "no"}")
                    appendLine("PrintMotd ${if (security.ssh.security.printMotd) "yes" else "no"}")
                    appendLine("PrintLastLog ${if (security.ssh.security.printLastLog) "yes" else "no"}")
                    appendLine("TCPKeepAlive ${if (security.ssh.security.tcpKeepAlive) "yes" else "no"}")
                    appendLine("Compression ${security.ssh.security.compression.name.lowercase()}")
                    appendLine("UseDNS ${if (security.ssh.security.useDNS) "yes" else "no"}")
                    appendLine()
                    
                    // Encryption settings
                    if (security.ssh.encryption.ciphers.isNotEmpty()) {
                        appendLine("Ciphers ${security.ssh.encryption.ciphers.joinToString(",")}")
                    }
                    if (security.ssh.encryption.macs.isNotEmpty()) {
                        appendLine("MACs ${security.ssh.encryption.macs.joinToString(",")}")
                    }
                    if (security.ssh.encryption.kex.isNotEmpty()) {
                        appendLine("KexAlgorithms ${security.ssh.encryption.kex.joinToString(",")}")
                    }
                    if (security.ssh.encryption.hostKeyAlgorithms.isNotEmpty()) {
                        appendLine("HostKeyAlgorithms ${security.ssh.encryption.hostKeyAlgorithms.joinToString(",")}")
                    }
                    if (security.ssh.encryption.pubkeyAcceptedAlgorithms.isNotEmpty()) {
                        appendLine("PubkeyAcceptedAlgorithms ${security.ssh.encryption.pubkeyAcceptedAlgorithms.joinToString(",")}")
                    }
                    appendLine()
                    
                    // Client alive settings
                    appendLine("ClientAliveInterval ${security.ssh.access.clientAlive.interval.inWholeSeconds}")
                    appendLine("ClientAliveCountMax ${security.ssh.access.clientAlive.maxCount}")
                    appendLine()
                    
                    // Banner
                    security.ssh.banner?.let { banner ->
                        appendLine("Banner /etc/ssh/banner")
                        appendLine("EOF")
                        appendLine()
                        appendLine("echo '$banner' > /etc/ssh/banner")
                        appendLine()
                    } ?: run {
                        appendLine("EOF")
                        appendLine()
                    }
                    
                    // Generate host keys
                    if (security.ssh.keys.keyGeneration.autoGenerate) {
                        appendLine("# Generate SSH host keys")
                        security.ssh.keys.keyGeneration.keyTypes.forEach { keyType ->
                            when (keyType) {
                                SSHKeyType.ED25519 -> {
                                    appendLine("ssh-keygen -t ed25519 -f /etc/ssh/ssh_host_ed25519_key -N ''")
                                }
                                SSHKeyType.ECDSA -> {
                                    val bits = security.ssh.keys.keyGeneration.bits[keyType] ?: 256
                                    appendLine("ssh-keygen -t ecdsa -b $bits -f /etc/ssh/ssh_host_ecdsa_key -N ''")
                                }
                                SSHKeyType.RSA -> {
                                    val bits = security.ssh.keys.keyGeneration.bits[keyType] ?: 4096
                                    appendLine("ssh-keygen -t rsa -b $bits -f /etc/ssh/ssh_host_rsa_key -N ''")
                                }
                                SSHKeyType.DSA -> {
                                    appendLine("ssh-keygen -t dsa -f /etc/ssh/ssh_host_dsa_key -N ''")
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
                
                // Sudo configuration
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
                    if (defaults.requirePassword) {
                        appendLine("Defaults !authenticate")
                    } else {
                        appendLine("Defaults authenticate")
                    }
                    appendLine("Defaults passwd_timeout=${defaults.passwordTimeout.inWholeMinutes}")
                    appendLine("Defaults passwd_tries=${defaults.passwordRetries}")
                    appendLine("Defaults logfile=${defaults.logFile}")
                    appendLine("Defaults secure_path=\"${defaults.secure_path}\"")
                    if (defaults.env_reset) {
                        appendLine("Defaults env_reset")
                    }
                    if (defaults.env_keep.isNotEmpty()) {
                        appendLine("Defaults env_keep += \"${defaults.env_keep.joinToString(" ")}\"")
                    }
                    if (defaults.env_delete.isNotEmpty()) {
                        appendLine("Defaults env_delete += \"${defaults.env_delete.joinToString(" ")}\"")
                    }
                    appendLine()
                    
                    // User aliases
                    security.sudo.aliases.users.forEach { (alias, users) ->
                        appendLine("User_Alias $alias = ${users.joinToString(", ")}")
                    }
                    
                    // Host aliases
                    security.sudo.aliases.hosts.forEach { (alias, hosts) ->
                        appendLine("Host_Alias $alias = ${hosts.joinToString(", ")}")
                    }
                    
                    // Command aliases
                    security.sudo.aliases.commands.forEach { (alias, commands) ->
                        appendLine("Cmnd_Alias $alias = ${commands.joinToString(", ")}")
                    }
                    
                    // RunAs aliases
                    security.sudo.aliases.runAs.forEach { (alias, users) ->
                        appendLine("Runas_Alias $alias = ${users.joinToString(", ")}")
                    }
                    
                    if (security.sudo.aliases.users.isNotEmpty() || 
                        security.sudo.aliases.hosts.isNotEmpty() ||
                        security.sudo.aliases.commands.isNotEmpty() ||
                        security.sudo.aliases.runAs.isNotEmpty()) {
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
                        
                        val ruleStr = "${rule.user} ${rule.host} = (${rule.runAs}) $tags${rule.commands.joinToString(", ")}$options"
                        
                        rule.comment?.let { comment ->
                            appendLine("# $comment")
                        }
                        appendLine(ruleStr)
                    }
                    
                    appendLine("EOF")
                    appendLine()
                    appendLine("visudo -c -f /etc/sudoers.d/horizonos")
                    appendLine()
                }
                
                // PAM configuration
                if (security.pam.enabled) {
                    appendLine("# PAM Configuration")
                    
                    // Password policy
                    val passPolicy = security.pam.passwordPolicy
                    appendLine("# Configure password policy")
                    appendLine("cat > /etc/security/pwquality.conf <<EOF")
                    appendLine("minlen = ${passPolicy.minLength}")
                    appendLine("maxrepeat = ${passPolicy.maxRepeats}")
                    appendLine("maxsequence = ${passPolicy.maxSequential}")
                    if (passPolicy.requireUppercase) appendLine("ucredit = -1")
                    if (passPolicy.requireLowercase) appendLine("lcredit = -1")
                    if (passPolicy.requireNumbers) appendLine("dcredit = -1")
                    if (passPolicy.requireSpecial) appendLine("ocredit = -1")
                    appendLine("remember = ${passPolicy.historySize}")
                    if (passPolicy.dictionary.enabled) {
                        appendLine("dictcheck = 1")
                        if (passPolicy.dictionary.dictionaries.isNotEmpty()) {
                            appendLine("dictpath = ${passPolicy.dictionary.dictionaries.first()}")
                        }
                    }
                    appendLine("EOF")
                    appendLine()
                    
                    // Account lockout
                    if (security.pam.lockout.enabled) {
                        appendLine("# Configure account lockout")
                        appendLine("cat > /etc/security/faillock.conf <<EOF")
                        appendLine("deny = ${security.pam.lockout.maxAttempts}")
                        appendLine("fail_interval = ${security.pam.lockout.resetAfter.inWholeSeconds}")
                        appendLine("unlock_time = ${security.pam.lockout.lockoutDuration.inWholeSeconds}")
                        if (security.pam.lockout.rootExempt) {
                            appendLine("even_deny_root")
                        }
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
                                security.pam.twoFactor.required.forEach { user ->
                                    appendLine("sudo -u $user google-authenticator -t -d -f -r 3 -R 30 -W")
                                }
                            }
                            else -> {
                                appendLine("# Two-factor method ${security.pam.twoFactor.method} configuration")
                            }
                        }
                        appendLine()
                    }
                }
                
                // Fail2ban configuration
                if (security.fail2ban.enabled) {
                    appendLine("# Fail2ban Configuration")
                    appendLine("systemctl enable fail2ban.service")
                    appendLine()
                    appendLine("cat > /etc/fail2ban/jail.local <<EOF")
                    appendLine("[DEFAULT]")
                    appendLine("bantime = ${security.fail2ban.bantime.inWholeSeconds}")
                    appendLine("findtime = ${security.fail2ban.findtime.inWholeSeconds}")
                    appendLine("maxretry = ${security.fail2ban.maxretry}")
                    appendLine("backend = ${security.fail2ban.backend.name.lowercase()}")
                    appendLine("usedns = ${security.fail2ban.usedns.name.lowercase()}")
                    appendLine("ignoreip = ${security.fail2ban.ignoreip.joinToString(" ")}")
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
                        jail.bantime?.let { appendLine("bantime = ${it.inWholeSeconds}") }
                        jail.findtime?.let { appendLine("findtime = ${it.inWholeSeconds}") }
                        jail.maxretry?.let { appendLine("maxretry = $it") }
                        if (jail.ignoreip.isNotEmpty()) {
                            appendLine("ignoreip = ${jail.ignoreip.joinToString(" ")}")
                        }
                        appendLine()
                    }
                    
                    appendLine("EOF")
                    appendLine()
                    appendLine("systemctl start fail2ban.service")
                    appendLine()
                }
                
                // Firewall configuration
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
                            appendLine("iptables -P INPUT ${security.firewall.defaultPolicy}")
                            appendLine("iptables -P FORWARD ${security.firewall.defaultPolicy}")
                            appendLine("iptables -P OUTPUT ACCEPT")
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
                                    rule.protocol?.let { append(" -p ${it.name.lowercase()}") }
                                    rule.port?.let { append(" --dport $it") }
                                    rule.interface?.let { append(" -i $it") }
                                    if (rule.state.isNotEmpty()) {
                                        append(" -m state --state ${rule.state.joinToString(",")}")
                                    }
                                    append(" -j ${rule.action}")
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
                        else -> {
                            appendLine("# Firewall backend ${security.firewall.backend} configuration")
                        }
                    }
                    appendLine()
                }
                
                // SELinux configuration
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
                            module.source?.let { source ->
                                appendLine("semodule -i $source")
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
                
                // AppArmor configuration
                if (security.apparmor.enabled) {
                    appendLine("# AppArmor Configuration")
                    appendLine("systemctl enable apparmor.service")
                    appendLine("systemctl start apparmor.service")
                    appendLine()
                    
                    // Profile management
                    security.apparmor.enforce.forEach { profile ->
                        appendLine("aa-enforce $profile")
                    }
                    
                    security.apparmor.complain.forEach { profile ->
                        appendLine("aa-complain $profile")
                    }
                    
                    security.apparmor.disable.forEach { profile ->
                        appendLine("aa-disable $profile")
                    }
                    
                    if (security.apparmor.enforce.isNotEmpty() || 
                        security.apparmor.complain.isNotEmpty() || 
                        security.apparmor.disable.isNotEmpty()) {
                        appendLine()
                    }
                }
                
                // GPG configuration
                if (security.gpg.enabled) {
                    appendLine("# GPG Configuration")
                    appendLine("mkdir -p /etc/gnupg")
                    appendLine()
                    appendLine("cat > /etc/gnupg/gpg.conf <<EOF")
                    appendLine("# HorizonOS GPG Configuration")
                    appendLine("keyserver ${security.gpg.keyserver}")
                    security.gpg.keyserverOptions.forEach { option ->
                        appendLine("keyserver-options $option")
                    }
                    security.gpg.defaultKey?.let { key ->
                        appendLine("default-key $key")
                    }
                    security.gpg.defaultRecipient?.let { recipient ->
                        appendLine("default-recipient $recipient")
                    }
                    appendLine("trust-model ${security.gpg.trustModel.name.lowercase()}")
                    if (security.gpg.cipherPrefs.isNotEmpty()) {
                        appendLine("personal-cipher-preferences ${security.gpg.cipherPrefs.joinToString(" ")}")
                    }
                    if (security.gpg.digestPrefs.isNotEmpty()) {
                        appendLine("personal-digest-preferences ${security.gpg.digestPrefs.joinToString(" ")}")
                    }
                    if (security.gpg.compressPrefs.isNotEmpty()) {
                        appendLine("personal-compress-preferences ${security.gpg.compressPrefs.joinToString(" ")}")
                    }
                    appendLine("EOF")
                    appendLine()
                    
                    // Import GPG keys
                    security.gpg.keys.forEach { key ->
                        appendLine("# Import GPG key: ${key.keyId}")
                        key.keyFile?.let { keyFile ->
                            appendLine("gpg --import $keyFile")
                        }
                        appendLine("gpg --edit-key ${key.keyId} trust quit")
                    }
                    
                    if (security.gpg.keys.isNotEmpty()) {
                        appendLine()
                    }
                }
                
                // Audit configuration
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
                        appendLine("tcp_listen_queue = ${security.audit.tcpListenQueue}")
                        appendLine("tcp_max_per_addr = ${security.audit.tcpMaxPerAddr}")
                        appendLine("tcp_client_max_idle = ${security.audit.tcpClientMaxIdle}")
                    }
                    appendLine("distribute_network = ${if (security.audit.distributeNetwork) "yes" else "no"}")
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
                                appendLine("# ${rule.name}")
                                appendLine(rule.rule)
                            }
                        }
                        appendLine("EOF")
                        appendLine()
                    }
                    
                    appendLine("systemctl start auditd.service")
                    appendLine()
                }
                
                // TPM configuration
                if (security.tpm.enabled) {
                    appendLine("# TPM Configuration")
                    appendLine("modprobe tpm_tis")
                    appendLine("modprobe tpm_crb")
                    appendLine()
                    
                    if (security.tpm.ownership.takeOwnership) {
                        appendLine("# Take TPM ownership")
                        when (security.tpm.version) {
                            TPMVersion.TPM2 -> {
                                appendLine("tpm2_startup -c")
                                appendLine("tpm2_clear")
                                security.tpm.ownership.ownerPassword?.let { password ->
                                    appendLine("echo '$password' | tpm2_changeauth -c owner")
                                }
                            }
                            TPMVersion.TPM1 -> {
                                appendLine("# TPM 1.2 ownership")
                                security.tpm.ownership.ownerPassword?.let { password ->
                                    appendLine("echo '$password' | tpm_takeownership")
                                }
                            }
                        }
                        appendLine()
                    }
                    
                    if (security.tpm.ima.enabled) {
                        appendLine("# Configure IMA/EVM")
                        appendLine("echo 'ima_policy=${security.tpm.ima.policy}' >> /etc/default/grub")
                        appendLine("echo 'ima_template=${security.tpm.ima.template}' >> /etc/default/grub")
                        appendLine("echo 'ima_hash=${security.tpm.ima.hash}' >> /etc/default/grub")
                        appendLine("grub-mkconfig -o /boot/grub/grub.cfg")
                        appendLine()
                    }
                }
                
                // Certificate management
                if (security.certificates.enabled) {
                    appendLine("# Certificate Management")
                    
                    if (security.certificates.ca.enabled) {
                        appendLine("# Setup Certificate Authority")
                        appendLine("mkdir -p ${security.certificates.ca.path}")
                        appendLine("mkdir -p ${security.certificates.ca.keyPath}")
                        appendLine("chmod 700 ${security.certificates.ca.keyPath}")
                        appendLine()
                    }
                    
                    // Configure certificates
                    security.certificates.certificates.forEach { cert ->
                        appendLine("# Configure certificate: ${cert.name}")
                        appendLine("mkdir -p \$(dirname ${cert.certFile})")
                        appendLine("mkdir -p \$(dirname ${cert.keyFile})")
                        
                        if (cert.autoRenew) {
                            appendLine("# Setup auto-renewal for ${cert.name}")
                            appendLine("systemctl enable cert-renew@${cert.name}.timer")
                        }
                        appendLine()
                    }
                    
                    // Update certificate store
                    appendLine("${security.certificates.store.updateCommand}")
                    appendLine("${security.certificates.store.rehashCommand} ${security.certificates.store.path}")
                    appendLine()
                }
                
                // Compliance scanning
                if (security.compliance.enabled) {
                    appendLine("# Compliance Configuration")
                    
                    security.compliance.frameworks.forEach { framework ->
                        if (framework.enabled) {
                            appendLine("# Configure ${framework.name} ${framework.version}")
                            framework.profiles.forEach { profile ->
                                appendLine("# Apply profile: $profile")
                            }
                        }
                    }
                    
                    if (security.compliance.scanning.enabled) {
                        appendLine("# Setup compliance scanning")
                        appendLine("systemctl enable compliance-scan.timer")
                        if (security.compliance.scanning.remediate) {
                            appendLine("systemctl enable compliance-remediate.service")
                        }
                    }
                    appendLine()
                }
                
                appendLine("echo 'Security configuration completed.'")
            })
            script.setExecutable(true)
            generatedFiles.add(GeneratedFile("scripts/security-config.sh", FileType.SHELL))
        }
    }
    
    private fun generateEnhancedServicesScript(config: CompiledConfig) {
        config.enhancedServices?.let { services ->
            val script = File(outputDir, "scripts/enhanced-services-config.sh")
            script.writeText(buildString {
                appendLine("#!/bin/bash")
                appendLine("# Enhanced Services Configuration")
                appendLine("# Generated from HorizonOS Kotlin DSL")
                appendLine()
                appendLine("set -euo pipefail")
                appendLine()
                appendLine("echo 'Configuring enhanced services...'")
                appendLine()
                
                // Database services
                if (services.databases.isNotEmpty()) {
                    appendLine("# === Database Services ===")
                    services.databases.forEach { db ->
                        appendLine("echo 'Configuring ${db.type.name.lowercase()} database...'")
                        
                        when (db.type) {
                            DatabaseType.POSTGRESQL -> generatePostgreSQLConfig(db)
                            DatabaseType.MYSQL -> generateMySQLConfig(db)
                            DatabaseType.REDIS -> generateRedisConfig(db)
                            DatabaseType.MONGODB -> generateMongoDBConfig(db)
                            DatabaseType.SQLITE -> generateSQLiteConfig(db)
                        }
                        
                        if (db.enabled) {
                            appendLine("systemctl enable ${getDatabaseServiceName(db.type)}")
                            if (db.autoStart) {
                                appendLine("systemctl start ${getDatabaseServiceName(db.type)}")
                            }
                        }
                        appendLine()
                    }
                }
                
                // Web server services
                if (services.webServers.isNotEmpty()) {
                    appendLine("# === Web Server Services ===")
                    services.webServers.forEach { webServer ->
                        appendLine("echo 'Configuring ${webServer.type.name.lowercase()} web server...'")
                        
                        when (webServer.type) {
                            WebServerType.NGINX -> generateNginxConfig(webServer)
                            WebServerType.APACHE -> generateApacheConfig(webServer)
                            WebServerType.CADDY -> generateCaddyConfig(webServer)
                            WebServerType.LIGHTTPD -> generateLighttpdConfig(webServer)
                        }
                        
                        if (webServer.enabled) {
                            appendLine("systemctl enable ${getWebServerServiceName(webServer.type)}")
                            if (webServer.autoStart) {
                                appendLine("systemctl start ${getWebServerServiceName(webServer.type)}")
                            }
                        }
                        appendLine()
                    }
                }
                
                // Container services
                if (services.containers.isNotEmpty()) {
                    appendLine("# === Container Services ===")
                    services.containers.forEach { container ->
                        appendLine("echo 'Configuring ${container.runtime.name.lowercase()} container runtime...'")
                        
                        when (container.runtime) {
                            ContainerRuntime.DOCKER -> generateDockerConfig(container)
                            ContainerRuntime.PODMAN -> generatePodmanConfig(container)
                            ContainerRuntime.SYSTEMD_NSPAWN -> generateSystemdNspawnConfig(container)
                        }
                        
                        if (container.enabled) {
                            appendLine("systemctl enable ${getContainerServiceName(container.runtime)}")
                            if (container.autoStart) {
                                appendLine("systemctl start ${getContainerServiceName(container.runtime)}")
                            }
                        }
                        appendLine()
                    }
                }
                
                // Message queue services
                if (services.messageQueues.isNotEmpty()) {
                    appendLine("# === Message Queue Services ===")
                    services.messageQueues.forEach { mq ->
                        appendLine("echo 'Configuring ${mq.type.name.lowercase()} message queue...'")
                        
                        when (mq.type) {
                            MessageQueueType.RABBITMQ -> generateRabbitMQConfig(mq)
                            MessageQueueType.KAFKA -> generateKafkaConfig(mq)
                            MessageQueueType.NATS -> generateNATSConfig(mq)
                            MessageQueueType.REDIS_STREAMS -> generateRedisStreamsConfig(mq)
                        }
                        
                        if (mq.enabled) {
                            appendLine("systemctl enable ${getMessageQueueServiceName(mq.type)}")
                            if (mq.autoStart) {
                                appendLine("systemctl start ${getMessageQueueServiceName(mq.type)}")
                            }
                        }
                        appendLine()
                    }
                }
                
                // Monitoring services
                if (services.monitoring.isNotEmpty()) {
                    appendLine("# === Monitoring Services ===")
                    services.monitoring.forEach { monitoring ->
                        appendLine("echo 'Configuring ${monitoring.type.name.lowercase()} monitoring...'")
                        
                        when (monitoring.type) {
                            MonitoringType.PROMETHEUS -> generatePrometheusConfig(monitoring)
                            MonitoringType.GRAFANA -> generateGrafanaConfig(monitoring)
                            MonitoringType.JAEGER -> generateJaegerConfig(monitoring)
                            MonitoringType.ZIPKIN -> generateZipkinConfig(monitoring)
                        }
                        
                        if (monitoring.enabled) {
                            appendLine("systemctl enable ${getMonitoringServiceName(monitoring.type)}")
                            if (monitoring.autoStart) {
                                appendLine("systemctl start ${getMonitoringServiceName(monitoring.type)}")
                            }
                        }
                        appendLine()
                    }
                }
                
                // Custom systemd units
                if (services.systemdUnits.isNotEmpty()) {
                    appendLine("# === Custom Systemd Units ===")
                    services.systemdUnits.forEach { unit ->
                        appendLine("echo 'Creating systemd unit: ${unit.name}...'")
                        generateSystemdUnitFile(unit)
                        appendLine("systemctl daemon-reload")
                        
                        if (unit.wantedBy.isNotEmpty()) {
                            appendLine("systemctl enable ${unit.name}.${unit.unitType.name.lowercase()}")
                        }
                        appendLine()
                    }
                }
                
                appendLine("echo 'Enhanced services configuration completed.'")
            })
            script.setExecutable(true)
            generatedFiles.add(GeneratedFile("scripts/enhanced-services-config.sh", FileType.SHELL))
        }
    }
    
    private fun StringBuilder.generatePostgreSQLConfig(db: DatabaseService) {
        appendLine("# PostgreSQL Configuration")
        appendLine("mkdir -p ${db.dataDirectory}")
        appendLine("chown postgres:postgres ${db.dataDirectory}")
        
        db.postgresConfig?.let { postgres ->
            appendLine("cat > /etc/postgresql/postgresql.conf <<EOF")
            appendLine("# PostgreSQL Configuration - Generated by HorizonOS")
            appendLine("max_connections = ${postgres.maxConnections}")
            appendLine("shared_buffers = ${postgres.sharedBuffers}")
            appendLine("effective_cache_size = ${postgres.effectiveCacheSize}")
            appendLine("maintenance_work_mem = ${postgres.maintenanceWorkMem}")
            appendLine("checkpoint_completion_target = ${postgres.checkpointCompletionTarget}")
            appendLine("wal_buffers = ${postgres.walBuffers}")
            appendLine("default_statistics_target = ${postgres.defaultStatisticsTarget}")
            appendLine("log_min_duration_statement = ${postgres.logMinDurationStatement}")
            appendLine("log_connections = ${if (postgres.logConnections) "on" else "off"}")
            appendLine("log_disconnections = ${if (postgres.logDisconnections) "on" else "off"}")
            appendLine("log_lock_waits = ${if (postgres.logLockWaits) "on" else "off"}")
            appendLine("log_statement = '${postgres.logStatement}'")
            appendLine("log_line_prefix = '${postgres.logLinePrefix}'")
            appendLine("EOF")
            appendLine()
            
            if (postgres.extensions.isNotEmpty()) {
                appendLine("# Enable PostgreSQL extensions")
                postgres.extensions.forEach { ext ->
                    appendLine("sudo -u postgres psql -c \"CREATE EXTENSION IF NOT EXISTS $ext;\"")
                }
            }
        }
        
        // Create databases and users
        if (db.databases.isNotEmpty()) {
            appendLine("# Create databases")
            db.databases.forEach { database ->
                appendLine("sudo -u postgres createdb ${database.name} --encoding=${database.charset} --locale=${database.collation}")
                database.owner?.let { owner ->
                    appendLine("sudo -u postgres psql -c \"ALTER DATABASE ${database.name} OWNER TO $owner;\"")
                }
            }
        }
        
        if (db.users.isNotEmpty()) {
            appendLine("# Create users")
            db.users.forEach { user ->
                appendLine("sudo -u postgres psql -c \"CREATE USER ${user.name} WITH PASSWORD '${user.password}';\"")
                user.privileges.forEach { privilege ->
                    user.databases.forEach { database ->
                        appendLine("sudo -u postgres psql -c \"GRANT $privilege ON DATABASE $database TO ${user.name};\"")
                    }
                }
            }
        }
    }
    
    private fun StringBuilder.generateMySQLConfig(db: DatabaseService) {
        appendLine("# MySQL Configuration")
        appendLine("mkdir -p ${db.dataDirectory}")
        appendLine("chown mysql:mysql ${db.dataDirectory}")
        
        db.mysqlConfig?.let { mysql ->
            appendLine("cat > /etc/mysql/mysql.conf.d/horizonos.cnf <<EOF")
            appendLine("# MySQL Configuration - Generated by HorizonOS")
            appendLine("[mysqld]")
            appendLine("max_connections = ${mysql.maxConnections}")
            appendLine("innodb_buffer_pool_size = ${mysql.innodbBufferPoolSize}")
            appendLine("innodb_log_file_size = ${mysql.innodbLogFileSize}")
            appendLine("innodb_file_per_table = ${if (mysql.innodbFilePerTable) 1 else 0}")
            appendLine("query_cache_type = ${if (mysql.queryCache) 1 else 0}")
            appendLine("query_cache_size = ${mysql.queryCacheSize}")
            appendLine("tmp_table_size = ${mysql.tmpTableSize}")
            appendLine("max_heap_table_size = ${mysql.maxHeapTableSize}")
            appendLine("slow_query_log = ${if (mysql.slowQueryLog) 1 else 0}")
            appendLine("long_query_time = ${mysql.longQueryTime}")
            appendLine("binlog_format = ${mysql.binlogFormat}")
            appendLine("character-set-server = ${mysql.serverCharset}")
            appendLine("collation-server = ${mysql.serverCollation}")
            appendLine("EOF")
        }
    }
    
    private fun StringBuilder.generateRedisConfig(db: DatabaseService) {
        appendLine("# Redis Configuration")
        appendLine("mkdir -p ${db.dataDirectory}")
        appendLine("chown redis:redis ${db.dataDirectory}")
        
        db.redisConfig?.let { redis ->
            appendLine("cat > /etc/redis/redis.conf <<EOF")
            appendLine("# Redis Configuration - Generated by HorizonOS")
            appendLine("port ${db.port}")
            appendLine("bind 127.0.0.1")
            appendLine("maxmemory ${redis.maxMemory}")
            appendLine("maxmemory-policy ${redis.maxMemoryPolicy}")
            appendLine("databases ${redis.databases}")
            appendLine("timeout ${redis.timeout}")
            appendLine("tcp-keepalive ${redis.tcpKeepalive}")
            
            when (redis.persistenceMode) {
                RedisPersistence.RDB -> {
                    redis.rdbSavePolicy.forEach { policy ->
                        appendLine("save $policy")
                    }
                }
                RedisPersistence.AOF -> {
                    appendLine("appendonly yes")
                    appendLine("auto-aof-rewrite-percentage 100")
                    appendLine("auto-aof-rewrite-min-size 64mb")
                }
                RedisPersistence.BOTH -> {
                    redis.rdbSavePolicy.forEach { policy ->
                        appendLine("save $policy")
                    }
                    appendLine("appendonly yes")
                }
                RedisPersistence.NONE -> {
                    appendLine("save \"\"")
                    appendLine("appendonly no")
                }
            }
            
            if (redis.requirepass && redis.password != null) {
                appendLine("requirepass ${redis.password}")
            }
            
            redis.replication?.let { repl ->
                appendLine("slaveof ${repl.masterHost} ${repl.masterPort}")
                appendLine("slave-read-only yes")
                appendLine("repl-diskless-sync ${if (repl.replicationDisklessSync) "yes" else "no"}")
            }
            
            appendLine("EOF")
        }
    }
    
    private fun StringBuilder.generateNginxConfig(webServer: WebServerService) {
        appendLine("# Nginx Configuration")
        appendLine("mkdir -p /etc/nginx/sites-available /etc/nginx/sites-enabled")
        
        webServer.nginxConfig?.let { nginx ->
            appendLine("cat > /etc/nginx/nginx.conf <<EOF")
            appendLine("# Nginx Configuration - Generated by HorizonOS")
            appendLine("user nginx;")
            appendLine("worker_processes ${nginx.workerProcesses};")
            appendLine("error_log /var/log/nginx/error.log;")
            appendLine("pid /run/nginx.pid;")
            appendLine()
            appendLine("events {")
            appendLine("    worker_connections ${nginx.workerConnections};")
            appendLine("}")
            appendLine()
            appendLine("http {")
            appendLine("    include /etc/nginx/mime.types;")
            appendLine("    default_type application/octet-stream;")
            appendLine("    keepalive_timeout ${nginx.keepaliveTimeout};")
            appendLine("    client_max_body_size ${nginx.clientMaxBodySize};")
            
            if (nginx.gzipCompression) {
                appendLine("    gzip on;")
                appendLine("    gzip_types ${nginx.gzipTypes.joinToString(" ")};")
            }
            
            // Upstreams
            nginx.upstreams.forEach { upstream ->
                appendLine()
                appendLine("    upstream ${upstream.name} {")
                when (upstream.loadBalancing) {
                    LoadBalancingMethod.LEAST_CONN -> appendLine("        least_conn;")
                    LoadBalancingMethod.IP_HASH -> appendLine("        ip_hash;")
                    LoadBalancingMethod.WEIGHTED -> appendLine("        # Weighted round robin (default)")
                    LoadBalancingMethod.ROUND_ROBIN -> {} // Default, no directive needed
                }
                upstream.servers.forEach { server ->
                    appendLine("        server $server;")
                }
                appendLine("    }")
            }
            
            appendLine("    include /etc/nginx/sites-enabled/*;")
            appendLine("}")
            appendLine("EOF")
        }
        
        // Generate virtual host files
        webServer.virtualHosts.forEach { vhost ->
            appendLine("cat > /etc/nginx/sites-available/${vhost.serverName} <<EOF")
            appendLine("server {")
            appendLine("    listen ${vhost.port};")
            appendLine("    server_name ${vhost.serverName};")
            
            vhost.documentRoot?.let { docRoot ->
                appendLine("    root $docRoot;")
            }
            
            appendLine("    index ${vhost.directoryIndex.joinToString(" ")};")
            
            // Locations
            vhost.locations.forEach { location ->
                appendLine()
                appendLine("    location ${location.path} {")
                location.proxyPass?.let { proxy ->
                    appendLine("        proxy_pass $proxy;")
                }
                location.alias?.let { alias ->
                    appendLine("        alias $alias;")
                }
                location.headers.forEach { (name, value) ->
                    appendLine("        proxy_set_header $name $value;")
                }
                appendLine("    }")
            }
            
            appendLine("}")
            appendLine("EOF")
            appendLine("ln -sf /etc/nginx/sites-available/${vhost.serverName} /etc/nginx/sites-enabled/")
        }
    }
    
    private fun StringBuilder.generateSystemdUnitFile(unit: SystemdUnit) {
        val unitFileName = "${unit.name}.${unit.unitType.name.lowercase()}"
        appendLine("cat > /etc/systemd/system/$unitFileName <<EOF")
        appendLine("# ${unit.description}")
        appendLine("# Generated by HorizonOS Kotlin DSL")
        appendLine()
        
        // [Unit] section
        appendLine("[Unit]")
        appendLine("Description=${unit.description}")
        if (unit.after.isNotEmpty()) {
            appendLine("After=${unit.after.joinToString(" ")}")
        }
        if (unit.requires.isNotEmpty()) {
            appendLine("Requires=${unit.requires.joinToString(" ")}")
        }
        if (unit.wants.isNotEmpty()) {
            appendLine("Wants=${unit.wants.joinToString(" ")}")
        }
        if (unit.conflicts.isNotEmpty()) {
            appendLine("Conflicts=${unit.conflicts.joinToString(" ")}")
        }
        appendLine()
        
        // [Service] section (for service units)
        if (unit.unitType == SystemdUnitType.SERVICE) {
            appendLine("[Service]")
            appendLine("Type=${unit.type.name.lowercase()}")
            unit.execStart?.let { appendLine("ExecStart=$it") }
            unit.execStop?.let { appendLine("ExecStop=$it") }
            unit.execReload?.let { appendLine("ExecReload=$it") }
            unit.workingDirectory?.let { appendLine("WorkingDirectory=$it") }
            unit.user?.let { appendLine("User=$it") }
            unit.group?.let { appendLine("Group=$it") }
            appendLine("Restart=${unit.restart.name.lowercase().replace("_", "-")}")
            appendLine("RestartSec=${unit.restartSec}")
            
            if (unit.environment.isNotEmpty()) {
                unit.environment.forEach { (key, value) ->
                    appendLine("Environment=\"$key=$value\"")
                }
            }
            appendLine()
        }
        
        // [Install] section
        if (unit.wantedBy.isNotEmpty() || unit.requiredBy.isNotEmpty()) {
            appendLine("[Install]")
            if (unit.wantedBy.isNotEmpty()) {
                appendLine("WantedBy=${unit.wantedBy.joinToString(" ")}")
            }
            if (unit.requiredBy.isNotEmpty()) {
                appendLine("RequiredBy=${unit.requiredBy.joinToString(" ")}")
            }
        }
        
        appendLine("EOF")
    }
    
    // Helper functions for service names
    private fun getDatabaseServiceName(type: DatabaseType): String = when (type) {
        DatabaseType.POSTGRESQL -> "postgresql"
        DatabaseType.MYSQL -> "mysql"
        DatabaseType.REDIS -> "redis"
        DatabaseType.MONGODB -> "mongodb"
        DatabaseType.SQLITE -> "" // SQLite doesn't have a service
    }
    
    private fun getWebServerServiceName(type: WebServerType): String = when (type) {
        WebServerType.NGINX -> "nginx"
        WebServerType.APACHE -> "apache2"
        WebServerType.CADDY -> "caddy"
        WebServerType.LIGHTTPD -> "lighttpd"
    }
    
    private fun getContainerServiceName(runtime: ContainerRuntime): String = when (runtime) {
        ContainerRuntime.DOCKER -> "docker"
        ContainerRuntime.PODMAN -> "podman"
        ContainerRuntime.SYSTEMD_NSPAWN -> "systemd-nspawn@"
    }
    
    private fun getMessageQueueServiceName(type: MessageQueueType): String = when (type) {
        MessageQueueType.RABBITMQ -> "rabbitmq-server"
        MessageQueueType.KAFKA -> "kafka"
        MessageQueueType.NATS -> "nats-server"
        MessageQueueType.REDIS_STREAMS -> "redis"
    }
    
    private fun getMonitoringServiceName(type: MonitoringType): String = when (type) {
        MonitoringType.PROMETHEUS -> "prometheus"
        MonitoringType.GRAFANA -> "grafana-server"
        MonitoringType.JAEGER -> "jaeger"
        MonitoringType.ZIPKIN -> "zipkin"
    }
    
    // Placeholder functions for other service types (can be expanded)
    private fun StringBuilder.generateMongoDBConfig(db: DatabaseService) {
        appendLine("# MongoDB configuration placeholder")
    }
    
    private fun StringBuilder.generateSQLiteConfig(db: DatabaseService) {
        appendLine("# SQLite configuration placeholder")
    }
    
    private fun StringBuilder.generateApacheConfig(webServer: WebServerService) {
        appendLine("# Apache configuration placeholder")
    }
    
    private fun StringBuilder.generateCaddyConfig(webServer: WebServerService) {
        appendLine("# Caddy configuration placeholder")
    }
    
    private fun StringBuilder.generateLighttpdConfig(webServer: WebServerService) {
        appendLine("# Lighttpd configuration placeholder")
    }
    
    private fun StringBuilder.generateDockerConfig(container: ContainerService) {
        appendLine("# Docker configuration placeholder")
    }
    
    private fun StringBuilder.generatePodmanConfig(container: ContainerService) {
        appendLine("# Podman configuration placeholder")
    }
    
    private fun StringBuilder.generateSystemdNspawnConfig(container: ContainerService) {
        appendLine("# systemd-nspawn configuration placeholder")
    }
    
    private fun StringBuilder.generateRabbitMQConfig(mq: MessageQueueService) {
        appendLine("# RabbitMQ configuration placeholder")
    }
    
    private fun StringBuilder.generateKafkaConfig(mq: MessageQueueService) {
        appendLine("# Kafka configuration placeholder")
    }
    
    private fun StringBuilder.generateNATSConfig(mq: MessageQueueService) {
        appendLine("# NATS configuration placeholder")
    }
    
    private fun StringBuilder.generateRedisStreamsConfig(mq: MessageQueueService) {
        appendLine("# Redis Streams configuration placeholder")
    }
    
    private fun StringBuilder.generatePrometheusConfig(monitoring: MonitoringService) {
        appendLine("# Prometheus configuration placeholder")
    }
    
    private fun StringBuilder.generateGrafanaConfig(monitoring: MonitoringService) {
        appendLine("# Grafana configuration placeholder")
    }
    
    private fun StringBuilder.generateJaegerConfig(monitoring: MonitoringService) {
        appendLine("# Jaeger configuration placeholder")
    }
    
    private fun StringBuilder.generateZipkinConfig(monitoring: MonitoringService) {
        appendLine("# Zipkin configuration placeholder")
    }
    
    private fun generateDevelopmentScript(config: CompiledConfig) {
        config.development?.let { dev ->
            val script = File(outputDir, "scripts/development-setup.sh")
            script.writeText(buildString {
                appendLine("#!/bin/bash")
                appendLine("# Development Environment Setup")
                appendLine("# Generated from HorizonOS Kotlin DSL")
                appendLine()
                appendLine("set -euo pipefail")
                appendLine()
                appendLine("echo 'Setting up development environment...'")
                appendLine()
                
                // Language runtimes
                if (dev.languages.isNotEmpty()) {
                    appendLine("# === Language Runtimes ===")
                    dev.languages.forEach { lang ->
                        if (lang.enabled) {
                            appendLine("echo 'Installing ${lang.type.name.lowercase()} runtime...'")
                            generateLanguageRuntimeSetup(lang)
                            appendLine()
                        }
                    }
                }
                
                // IDEs
                if (dev.ides.isNotEmpty()) {
                    appendLine("# === IDE Installation and Configuration ===")
                    dev.ides.forEach { ide ->
                        if (ide.enabled) {
                            appendLine("echo 'Setting up ${ide.type.name.lowercase().replace("_", " ")}...'")
                            generateIDESetup(ide)
                            appendLine()
                        }
                    }
                }
                
                // Editors
                if (dev.editors.isNotEmpty()) {
                    appendLine("# === Editor Configuration ===")
                    dev.editors.forEach { editor ->
                        if (editor.enabled) {
                            appendLine("echo 'Configuring ${editor.type.name.lowercase()}...'")
                            generateEditorSetup(editor)
                            appendLine()
                        }
                    }
                }
                
                // Development tools
                if (dev.tools.isNotEmpty()) {
                    appendLine("# === Development Tools ===")
                    dev.tools.forEach { tool ->
                        if (tool.enabled && tool.autoInstall) {
                            appendLine("echo 'Installing development tool: ${tool.name}...'")
                            generateToolSetup(tool)
                            appendLine()
                        }
                    }
                }
                
                // Package managers
                if (dev.packageManagers.isNotEmpty()) {
                    appendLine("# === Package Manager Configuration ===")
                    dev.packageManagers.forEach { pm ->
                        if (pm.enabled) {
                            appendLine("echo 'Configuring ${pm.type.name.lowercase()} package manager...'")
                            generatePackageManagerSetup(pm)
                            appendLine()
                        }
                    }
                }
                
                // Container development environments
                if (dev.containerDev.isNotEmpty()) {
                    appendLine("# === Container Development Environments ===")
                    dev.containerDev.forEach { containerEnv ->
                        appendLine("echo 'Setting up container dev environment: ${containerEnv.name}...'")
                        generateContainerDevSetup(containerEnv)
                        appendLine()
                    }
                }
                
                // Version control
                if (dev.versionControl.isNotEmpty()) {
                    appendLine("# === Version Control Configuration ===")
                    dev.versionControl.forEach { vcs ->
                        if (vcs.enabled) {
                            appendLine("echo 'Configuring ${vcs.type.name.lowercase()} version control...'")
                            generateVCSSetup(vcs)
                            appendLine()
                        }
                    }
                }
                
                appendLine("echo 'Development environment setup completed.'")
            })
            script.setExecutable(true)
            generatedFiles.add(GeneratedFile("scripts/development-setup.sh", FileType.SHELL))
        }
    }
    
    private fun generateEnvironmentScript(config: CompiledConfig) {
        config.environment?.let { env ->
            val script = File(outputDir, "scripts/environment-setup.sh")
            script.writeText(buildString {
                appendLine("#!/bin/bash")
                appendLine("# Shell and Environment Setup")
                appendLine("# Generated from HorizonOS Kotlin DSL")
                appendLine()
                appendLine("set -euo pipefail")
                appendLine()
                appendLine("echo 'Setting up shell and environment...'")
                appendLine()
                
                // Shell configurations
                if (env.shells.isNotEmpty()) {
                    appendLine("# === Shell Configuration ===")
                    env.shells.forEach { shell ->
                        if (shell.enabled) {
                            appendLine("echo 'Configuring ${shell.type.name.lowercase()} shell...'")
                            generateShellSetup(shell)
                            
                            if (shell.defaultShell) {
                                appendLine("chsh -s /bin/${shell.type.name.lowercase()} \$USER")
                            }
                            appendLine()
                        }
                    }
                }
                
                // Environment variables
                if (env.environmentVariables.isNotEmpty()) {
                    appendLine("# === Environment Variables ===")
                    appendLine("echo 'Setting up environment variables...'")
                    env.environmentVariables.values.forEach { envVar ->
                        generateEnvironmentVariableSetup(envVar)
                    }
                    appendLine()
                }
                
                // PATH entries
                if (env.pathEntries.isNotEmpty()) {
                    appendLine("# === PATH Configuration ===")
                    appendLine("echo 'Configuring PATH entries...'")
                    generatePathSetup(env.pathEntries)
                    appendLine()
                }
                
                // Dotfiles
                if (env.dotfiles.isNotEmpty()) {
                    appendLine("# === Dotfiles Management ===")
                    env.dotfiles.forEach { dotfile ->
                        appendLine("echo 'Managing dotfile: ${dotfile.name}...'")
                        generateDotfileSetup(dotfile)
                        appendLine()
                    }
                }
                
                // Terminal configurations
                if (env.terminals.isNotEmpty()) {
                    appendLine("# === Terminal Configuration ===")
                    env.terminals.forEach { terminal ->
                        if (terminal.enabled) {
                            appendLine("echo 'Configuring ${terminal.type.name.lowercase().replace("_", "-")} terminal...'")
                            generateTerminalSetup(terminal)
                            appendLine()
                        }
                    }
                }
                
                // Prompt configurations
                if (env.prompts.isNotEmpty()) {
                    appendLine("# === Prompt Configuration ===")
                    env.prompts.forEach { prompt ->
                        if (prompt.enabled) {
                            appendLine("echo 'Setting up ${prompt.type.name.lowercase()} prompt...'")
                            generatePromptSetup(prompt)
                            appendLine()
                        }
                    }
                }
                
                appendLine("echo 'Shell and environment setup completed.'")
            })
            script.setExecutable(true)
            generatedFiles.add(GeneratedFile("scripts/environment-setup.sh", FileType.SHELL))
        }
    }
    
    // Development setup helper functions
    private fun StringBuilder.generateLanguageRuntimeSetup(lang: LanguageRuntime) {
        when (lang.type) {
            LanguageType.NODEJS -> {
                appendLine("# Install Node.js")
                appendLine("curl -fsSL https://deb.nodesource.com/setup_lts.x | sudo -E bash -")
                appendLine("sudo apt-get install -y nodejs")
                
                lang.nodeConfig?.let { node ->
                    when (node.packageManager) {
                        NodePackageManager.YARN -> {
                            appendLine("npm install -g yarn@${node.yarnVersion}")
                        }
                        NodePackageManager.PNPM -> {
                            appendLine("npm install -g pnpm")
                        }
                        NodePackageManager.NPM -> {} // Already installed with Node.js
                    }
                    
                    if (node.enableCorepack) {
                        appendLine("corepack enable")
                    }
                    
                    node.globalPackages.forEach { pkg ->
                        appendLine("npm install -g $pkg")
                    }
                }
            }
            LanguageType.PYTHON -> {
                appendLine("# Install Python")
                appendLine("sudo apt-get install -y python3 python3-pip python3-venv")
                
                lang.pythonConfig?.let { python ->
                    python.globalPackages.forEach { pkg ->
                        appendLine("pip3 install $pkg")
                    }
                }
            }
            LanguageType.JAVA -> {
                lang.javaConfig?.let { java ->
                    when (java.jvmImplementation) {
                        JVMImplementation.OPENJDK -> appendLine("sudo apt-get install -y openjdk-17-jdk")
                        JVMImplementation.ORACLE_JDK -> appendLine("# Oracle JDK installation")
                        JVMImplementation.GRAALVM -> appendLine("# GraalVM installation")
                        JVMImplementation.AZUL_ZULU -> appendLine("# Azul Zulu installation")
                        JVMImplementation.ADOPTIUM -> appendLine("# Adoptium installation")
                    }
                }
            }
            LanguageType.RUST -> {
                appendLine("# Install Rust")
                appendLine("curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y")
                appendLine("source ~/.cargo/env")
                
                lang.rustConfig?.let { rust ->
                    rust.targets.forEach { target ->
                        appendLine("rustup target add $target")
                    }
                    rust.components.forEach { component ->
                        appendLine("rustup component add $component")
                    }
                }
            }
            LanguageType.GO -> {
                appendLine("# Install Go")
                appendLine("wget -O go.tar.gz https://golang.org/dl/go1.21.0.linux-amd64.tar.gz")
                appendLine("sudo tar -C /usr/local -xzf go.tar.gz")
                appendLine("echo 'export PATH=\$PATH:/usr/local/go/bin' >> ~/.bashrc")
            }
            else -> {
                appendLine("# ${lang.type.name} installation placeholder")
            }
        }
        
        // Set environment variables
        lang.environmentVariables.forEach { (key, value) ->
            appendLine("echo 'export $key=\"$value\"' >> ~/.bashrc")
        }
    }
    
    private fun StringBuilder.generateIDESetup(ide: IDEConfiguration) {
        when (ide.type) {
            IDEType.VSCODE -> {
                appendLine("# Install Visual Studio Code")
                appendLine("wget -qO- https://packages.microsoft.com/keys/microsoft.asc | gpg --dearmor > packages.microsoft.gpg")
                appendLine("sudo install -o root -g root -m 644 packages.microsoft.gpg /etc/apt/trusted.gpg.d/")
                appendLine("echo 'deb [arch=amd64,arm64,armhf signed-by=/etc/apt/trusted.gpg.d/packages.microsoft.gpg] https://packages.microsoft.com/repos/code stable main' | sudo tee /etc/apt/sources.list.d/vscode.list")
                appendLine("sudo apt update && sudo apt install -y code")
                
                ide.extensions.forEach { ext ->
                    appendLine("code --install-extension $ext")
                }
            }
            IDEType.VIM -> {
                appendLine("sudo apt-get install -y vim")
                ide.vimConfig?.let { vim ->
                    appendLine("cat > ~/.vimrc <<EOF")
                    appendLine("\" Vim configuration")
                    if (vim.enableSyntaxHighlighting) appendLine("syntax on")
                    if (vim.enableLineNumbers) appendLine("set number")
                    appendLine("set tabstop=${vim.tabWidth}")
                    appendLine("set expandtab=${if (vim.expandTabs) "true" else "false"}")
                    appendLine("colorscheme ${vim.colorScheme}")
                    appendLine("EOF")
                }
            }
            else -> {
                appendLine("# ${ide.type.name} installation placeholder")
            }
        }
    }
    
    private fun StringBuilder.generateShellSetup(shell: ShellConfiguration) {
        when (shell.type) {
            ShellType.BASH -> {
                shell.bashConfig?.let { bash ->
                    appendLine("cat >> ~/.bashrc <<EOF")
                    appendLine("# Bash configuration")
                    appendLine("HISTSIZE=${bash.historySize}")
                    appendLine("HISTFILESIZE=${bash.historyFileSize}")
                    appendLine("HISTCONTROL=${bash.historyControl}")
                    if (bash.enableColorPrompt) appendLine("force_color_prompt=yes")
                    if (bash.enableCompletion) appendLine("source /etc/bash_completion")
                    if (bash.enableGlobstar) appendLine("shopt -s globstar")
                    if (bash.checkWinSize) appendLine("shopt -s checkwinsize")
                    appendLine("EOF")
                }
            }
            ShellType.ZSH -> {
                appendLine("sudo apt-get install -y zsh")
                shell.zshConfig?.let { zsh ->
                    if (zsh.enableOhMyZsh) {
                        appendLine("sh -c \"\$(curl -fsSL https://raw.githubusercontent.com/ohmyzsh/ohmyzsh/master/tools/install.sh)\"")
                        appendLine("sed -i 's/ZSH_THEME=\"robbyrussell\"/ZSH_THEME=\"${zsh.theme}\"/' ~/.zshrc")
                        appendLine("sed -i 's/plugins=(git)/plugins=(${zsh.ohMyZshPlugins.joinToString(" ")})/' ~/.zshrc")
                    }
                }
            }
            ShellType.FISH -> {
                appendLine("sudo apt-get install -y fish")
                shell.fishConfig?.let { fish ->
                    appendLine("fish -c 'set -U fish_greeting \"${if (fish.enableGreeting) "Welcome to fish" else ""}\"\\'")
                    fish.abbreviations.forEach { (abbr, expansion) ->
                        appendLine("fish -c 'abbr $abbr \"$expansion\"'")
                    }
                }
            }
            else -> {
                appendLine("# ${shell.type.name} setup placeholder")
            }
        }
        
        // Add aliases
        shell.aliases.forEach { (alias, command) ->
            appendLine("echo 'alias $alias=\"$command\"' >> ${shell.configFile}")
        }
        
        // Add functions
        shell.functions.forEach { (name, body) ->
            appendLine("cat >> ${shell.configFile} <<EOF")
            appendLine("$name() {")
            appendLine("    $body")
            appendLine("}")
            appendLine("EOF")
        }
    }
    
    private fun StringBuilder.generateEnvironmentVariableSetup(envVar: EnvironmentVariable) {
        val exportLine = "export ${envVar.name}=\"${envVar.value}\""
        
        when (envVar.scope) {
            VariableScope.SYSTEM -> {
                appendLine("echo '$exportLine' | sudo tee -a /etc/environment")
            }
            VariableScope.USER -> {
                appendLine("echo '$exportLine' >> ~/.bashrc")
                appendLine("echo '$exportLine' >> ~/.zshrc")
            }
            VariableScope.SESSION -> {
                appendLine(exportLine)
            }
        }
    }
    
    private fun StringBuilder.generatePathSetup(pathEntries: List<PathEntry>) {
        val sortedEntries = pathEntries.sortedBy { it.priority }
        
        appendLine("# Setup PATH entries")
        appendLine("cat >> ~/.bashrc <<EOF")
        appendLine("# PATH configuration")
        sortedEntries.forEach { entry ->
            if (entry.createIfMissing) {
                appendLine("mkdir -p ${entry.path}")
            }
            if (entry.condition != null) {
                appendLine("if ${entry.condition}; then")
                appendLine("    export PATH=\"${entry.path}:\$PATH\"")
                appendLine("fi")
            } else {
                appendLine("export PATH=\"${entry.path}:\$PATH\"")
            }
        }
        appendLine("EOF")
    }
    
    private fun StringBuilder.generateDotfileSetup(dotfile: DotfileConfiguration) {
        if (dotfile.backup && !dotfile.overwrite) {
            appendLine("if [ -f ${dotfile.targetPath} ]; then")
            appendLine("    cp ${dotfile.targetPath} ${dotfile.targetPath}.backup")
            appendLine("fi")
        }
        
        if (dotfile.sourcePath != null) {
            if (dotfile.template) {
                appendLine("# Process template variables for ${dotfile.name}")
                dotfile.variables.forEach { (key, value) ->
                    appendLine("export ${key}=\"${value}\"")
                }
                appendLine("envsubst < ${dotfile.sourcePath} > ${dotfile.targetPath}")
            } else {
                appendLine("cp ${dotfile.sourcePath} ${dotfile.targetPath}")
            }
        }
        
        if (dotfile.executable) {
            appendLine("chmod +x ${dotfile.targetPath}")
        }
        
        dotfile.owner?.let { owner ->
            appendLine("chown ${owner}:${dotfile.group ?: owner} ${dotfile.targetPath}")
        }
        
        dotfile.permissions?.let { perms ->
            appendLine("chmod $perms ${dotfile.targetPath}")
        }
    }
    
    // Placeholder functions for other components
    private fun StringBuilder.generateEditorSetup(editor: EditorConfiguration) {
        appendLine("# ${editor.type.name} editor setup placeholder")
    }
    
    private fun StringBuilder.generateToolSetup(tool: DevelopmentTool) {
        appendLine("# Install ${tool.name}")
        appendLine("# Category: ${tool.category}")
        tool.dependencies.forEach { dep ->
            appendLine("# Dependency: $dep")
        }
    }
    
    private fun StringBuilder.generatePackageManagerSetup(pm: PackageManagerConfig) {
        appendLine("# ${pm.type.name} package manager setup placeholder")
    }
    
    private fun StringBuilder.generateContainerDevSetup(containerEnv: ContainerDevEnvironment) {
        appendLine("# Container dev environment: ${containerEnv.name}")
        appendLine("# Image: ${containerEnv.image}:${containerEnv.tag}")
    }
    
    private fun StringBuilder.generateVCSSetup(vcs: VCSConfiguration) {
        appendLine("# ${vcs.type.name} version control setup placeholder")
    }
    
    private fun StringBuilder.generateTerminalSetup(terminal: TerminalConfiguration) {
        appendLine("# ${terminal.type.name} terminal setup placeholder")
    }
    
    private fun StringBuilder.generatePromptSetup(prompt: PromptConfiguration) {
        appendLine("# ${prompt.type.name} prompt setup placeholder")
    }
}

// ===== Data Classes =====

@kotlinx.serialization.Serializable
data class OSTreeManifest(
    val ref: String,
    val metadata: Map<String, String>,
    val packages: List<String>,
    val repos: List<String>
)

data class GeneratedFile(
    val path: String,
    val type: FileType
)

enum class FileType(val displayName: String) {
    JSON("JSON Files"),
    YAML("YAML Files"),
    SHELL("Shell Scripts"),
    SYSTEMD("Systemd Units"),
    ANSIBLE("Ansible Playbooks"),
    DOCKER("Docker Files"),
    DOCUMENTATION("Documentation")
}

// ===== Result Types =====

sealed class GenerationResult {
    data class Success(val files: List<GeneratedFile>) : GenerationResult()
    data class Error(val error: GenerationError) : GenerationResult()
}

sealed class GenerationError(val message: String) {
    data class UnexpectedError(val details: String, val cause: Throwable) : GenerationError("Generation failed: $details")
}