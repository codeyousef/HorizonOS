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