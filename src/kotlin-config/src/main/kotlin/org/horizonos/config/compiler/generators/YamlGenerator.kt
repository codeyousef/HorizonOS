package org.horizonos.config.compiler.generators

import org.horizonos.config.dsl.CompiledConfig
import org.horizonos.config.compiler.GeneratedFile
import org.horizonos.config.compiler.FileType
import java.io.File

/**
 * YAML output generator for HorizonOS configurations
 * Generates human-readable YAML representation of the compiled configuration
 */
class YamlGenerator(
    private val outputDir: File,
    private val generatedFiles: MutableList<GeneratedFile>
) {
    
    /**
     * Generate YAML output file from compiled configuration
     */
    fun generate(config: CompiledConfig) {
        val yamlFile = File(outputDir, "yaml/config.yaml")
        val yaml = buildYamlContent(config)
        yamlFile.writeText(yaml)
        generatedFiles.add(GeneratedFile("yaml/config.yaml", FileType.YAML))
    }
    
    private fun buildYamlContent(config: CompiledConfig): String = buildString {
        appendLine("# HorizonOS Configuration")
        appendLine("# Generated from Kotlin DSL")
        appendLine()
        
        // System configuration
        appendLine("system:")
        appendLine("  hostname: ${config.system.hostname}")
        appendLine("  timezone: ${config.system.timezone}")
        appendLine("  locale: ${config.system.locale}")
        appendLine()
        
        // Packages configuration
        if (config.packages.isNotEmpty()) {
            appendLine("packages:")
            config.packages.forEach { pkg ->
                appendLine("  - name: ${pkg.name}")
                appendLine("    action: ${pkg.action.name.lowercase()}")
                pkg.group?.let { appendLine("    group: $it") }
            }
            appendLine()
        }
        
        // Services configuration
        if (config.services.isNotEmpty()) {
            appendLine("services:")
            config.services.forEach { service ->
                appendLine("  - name: ${service.name}")
                appendLine("    enabled: ${service.enabled}")
                service.config?.let { serviceConfig ->
                    appendLine("    auto_restart: ${serviceConfig.autoRestart}")
                    appendLine("    restart_on_failure: ${serviceConfig.restartOnFailure}")
                    if (serviceConfig.environment.isNotEmpty()) {
                        appendLine("    environment:")
                        serviceConfig.environment.forEach { (key, value) ->
                            appendLine("      $key: $value")
                        }
                    }
                }
            }
            appendLine()
        }
        
        // Users configuration
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
            appendLine()
        }
        
        // Repositories configuration
        if (config.repositories.isNotEmpty()) {
            appendLine("repositories:")
            config.repositories.forEach { repo ->
                appendLine("  - name: ${repo.name}")
                appendLine("    url: ${repo.url}")
                appendLine("    enabled: ${repo.enabled}")
                appendLine("    gpg_check: ${repo.gpgCheck}")
                appendLine("    priority: ${repo.priority}")
            }
            appendLine()
        }
        
        // Optional module configurations
        config.desktop?.let {
            appendLine("desktop:")
            appendLine("  environment: ${it.environment}")
            appendLine("  auto_login: ${it.autoLogin}")
            it.autoLoginUser?.let { user -> appendLine("  auto_login_user: $user") }
        }
        
        config.network?.let {
            appendLine("network:")
            appendLine("  manager: ${it.networkManager}")
            if (it.hostname.isNotEmpty()) appendLine("  hostname: ${it.hostname}")
            if (it.domainName.isNotEmpty()) appendLine("  domain: ${it.domainName}")
        }
        
        config.boot?.let {
            appendLine("boot:")
            appendLine("  bootloader: ${it.bootloader.type}")
            appendLine("  kernel: ${it.kernel.version}")
        }
        
        config.hardware?.let {
            appendLine("hardware:")
            appendLine("  gpu_driver: ${it.gpu.primary}")
            appendLine("  power_management: enabled")
        }
        
        config.storage?.let {
            appendLine("storage:")
            appendLine("  filesystems: ${it.filesystems.size} configured")
            appendLine("  encryption: ${it.encryption.enabled}")
        }
        
        config.security?.let {
            appendLine("security:")
            appendLine("  enabled: ${it.enabled}")
            appendLine("  pam: ${it.pam.enabled}")
            appendLine("  ssh: ${it.ssh.enabled}")
            appendLine("  firewall: ${it.firewall.enabled}")
        }
        
        config.enhancedServices?.let {
            appendLine("enhanced_services:")
            appendLine("  databases: ${it.databases.size}")
            appendLine("  web_servers: ${it.webServers.size}")
            appendLine("  containers: ${it.containers.size}")
        }
        
        config.development?.let {
            appendLine("development:")
            appendLine("  languages: ${it.languages.size}")
            appendLine("  ides: ${it.ides.size}")
            appendLine("  tools: ${it.tools.size}")
        }
        
        config.environment?.let {
            appendLine("environment:")
            appendLine("  shells: ${it.shells.size}")
            appendLine("  terminals: ${it.terminals.size}")
        }
        
        config.enhancedDesktop?.let {
            appendLine("enhanced_desktop:")
            appendLine("  window_managers: ${it.windowManagers.size}")
            appendLine("  themes: ${it.themes.size}")
        }
        
        config.graphDesktop?.let { graphDesktop ->
            appendLine("graph_desktop:")
            appendLine("  enabled: ${graphDesktop.enabled}")
            appendLine("  rendering_engine: ${graphDesktop.renderingEngine}")
            appendLine("  node_types: ${graphDesktop.nodeTypes.size}")
        }
    }
}