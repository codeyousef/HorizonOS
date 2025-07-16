package org.horizonos.config.compiler

import org.horizonos.config.dsl.CompiledConfig
import org.horizonos.config.compiler.generators.*
import java.io.File

/**
 * Refactored enhanced generator for HorizonOS configurations
 * Coordinates specialized generators for different output formats
 */
class RefactoredEnhancedConfigGenerator(private val outputDir: File) {
    
    private val generatedFiles = mutableListOf<GeneratedFile>()
    
    // Specialized generators
    private val jsonGenerator = JsonGenerator(outputDir, generatedFiles)
    private val yamlGenerator = YamlGenerator(outputDir, generatedFiles)
    private val systemdGenerator = SystemdGenerator(outputDir, generatedFiles)
    private val shellScriptGenerator = ShellScriptGenerator(outputDir, generatedFiles)
    private val ansibleGenerator = AnsibleGenerator(outputDir, generatedFiles)
    private val dockerGenerator = DockerGenerator(outputDir, generatedFiles)
    
    /**
     * Generate all output files from configuration
     */
    fun generate(config: CompiledConfig): GenerationResult {
        try {
            // Create directory structure
            createDirectoryStructure()
            
            // Generate various output formats using specialized generators
            jsonGenerator.generate(config)
            yamlGenerator.generate(config)
            systemdGenerator.generate(config)
            shellScriptGenerator.generate(config)
            ansibleGenerator.generate(config)
            dockerGenerator.generate(config)
            
            // Generate additional formats
            generateOSTreeManifest(config)
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
    
    private fun generateOSTreeManifest(config: CompiledConfig) {
        val manifestFile = File(outputDir, "ostree/manifest.json")
        manifestFile.writeText("""
            {
                "ref": "horizonos/stable/x86_64",
                "repos": [
                    {
                        "name": "archlinux",
                        "url": "https://archlinux.org/packages"
                    }
                ],
                "packages": [
                    ${config.packages.filter { it.action == org.horizonos.config.dsl.PackageAction.INSTALL }
                        .joinToString(",\n                    ") { "\"${it.name}\"" }}
                ],
                "hostname": "${config.system.hostname}",
                "timezone": "${config.system.timezone}",
                "locale": "${config.system.locale}"
            }
        """.trimIndent())
        generatedFiles.add(GeneratedFile("ostree/manifest.json", FileType.JSON))
    }
    
    private fun generateDocumentation(config: CompiledConfig) {
        val docFile = File(outputDir, "docs/README.md")
        docFile.writeText(buildString {
            appendLine("# HorizonOS Configuration")
            appendLine()
            appendLine("Generated from Kotlin DSL configuration")
            appendLine()
            appendLine("## System Configuration")
            appendLine("- **Hostname:** ${config.system.hostname}")
            appendLine("- **Timezone:** ${config.system.timezone}")
            appendLine("- **Locale:** ${config.system.locale}")
            appendLine()
            
            if (config.packages.isNotEmpty()) {
                appendLine("## Packages (${config.packages.size})")
                val installs = config.packages.filter { it.action == org.horizonos.config.dsl.PackageAction.INSTALL }
                val removes = config.packages.filter { it.action == org.horizonos.config.dsl.PackageAction.REMOVE }
                if (installs.isNotEmpty()) appendLine("- **To Install:** ${installs.size} packages")
                if (removes.isNotEmpty()) appendLine("- **To Remove:** ${removes.size} packages")
                appendLine()
            }
            
            if (config.services.isNotEmpty()) {
                appendLine("## Services (${config.services.size})")
                val enabled = config.services.filter { it.enabled }
                val disabled = config.services.filter { !it.enabled }
                if (enabled.isNotEmpty()) appendLine("- **Enabled:** ${enabled.size} services")
                if (disabled.isNotEmpty()) appendLine("- **Disabled:** ${disabled.size} services")
                appendLine()
            }
            
            config.network?.let {
                appendLine("## Network Configuration")
                appendLine("- **Manager:** ${it.networkManager}")
                appendLine("- **Interfaces:** ${it.interfaces.size}")
                appendLine("- **WiFi Networks:** ${it.wifiNetworks.size}")
                appendLine("- **VPN Connections:** ${it.vpnConnections.size}")
                appendLine()
            }
            
            config.security?.let {
                appendLine("## Security Configuration")
                appendLine("- **PAM:** ${if (it.pam.enabled) "Enabled" else "Disabled"}")
                appendLine("- **SSH:** ${if (it.ssh.enabled) "Enabled" else "Disabled"}")
                appendLine("- **Firewall:** ${if (it.firewall.enabled) "Enabled" else "Disabled"}")
                appendLine()
            }
            
            appendLine("## Generated Files")
            appendLine()
            generatedFiles.forEach { file ->
                appendLine("- `${file.path}` (${file.type})")
            }
        })
        generatedFiles.add(GeneratedFile("docs/README.md", FileType.MARKDOWN))
    }
}