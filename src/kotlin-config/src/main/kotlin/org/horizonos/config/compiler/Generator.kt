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