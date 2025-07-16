package org.horizonos.config.compiler.generators

import org.horizonos.config.dsl.CompiledConfig
import org.horizonos.config.compiler.GeneratedFile
import org.horizonos.config.compiler.FileType
import org.horizonos.config.compiler.generators.shell.*
import java.io.File

/**
 * Shell script generator coordinator for HorizonOS configurations
 * Orchestrates generation of all shell scripts using specialized generators
 */
class ShellScriptGenerator(
    private val outputDir: File,
    private val generatedFiles: MutableList<GeneratedFile>
) {
    
    // Specialized shell script generators
    private val systemScriptGenerator = SystemScriptGenerator(outputDir, generatedFiles)
    private val networkScriptGenerator = NetworkScriptGenerator(outputDir, generatedFiles)
    private val securityScriptGenerator = SecurityScriptGenerator(outputDir, generatedFiles)
    private val storageScriptGenerator = StorageScriptGenerator(outputDir, generatedFiles)
    private val hardwareScriptGenerator = HardwareScriptGenerator(outputDir, generatedFiles)
    private val servicesScriptGenerator = ServicesScriptGenerator(outputDir, generatedFiles)
    private val desktopScriptGenerator = DesktopScriptGenerator(outputDir, generatedFiles)
    private val developmentScriptGenerator = DevelopmentScriptGenerator(outputDir, generatedFiles)
    
    /**
     * Generate all shell scripts from compiled configuration
     */
    fun generate(config: CompiledConfig) {
        generateMainDeploymentScript(config)
        generateIndividualScripts(config)
    }
    
    private fun generateMainDeploymentScript(config: CompiledConfig) {
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
            config.network?.let { appendLine("./network-config.sh") }
            config.boot?.let { appendLine("./boot-config.sh") }
            config.hardware?.let { appendLine("./hardware-config.sh") }
            config.storage?.let { appendLine("./storage-config.sh") }
            config.security?.let { appendLine("./security-config.sh") }
            config.enhancedServices?.let { appendLine("./enhanced-services-config.sh") }
            config.development?.let { appendLine("./development-setup.sh") }
            config.environment?.let { appendLine("./environment-setup.sh") }
            config.enhancedDesktop?.let { appendLine("./enhanced-desktop-setup.sh") }
            config.graphDesktop?.let { appendLine("./graph-desktop-setup.sh") }
            config.desktop?.let { appendLine("./desktop-setup.sh") }
            config.automation?.let { appendLine("./automation-setup.sh") }
            appendLine()
            appendLine("echo -e \"\${GREEN}Deployment completed successfully!\${NC}\"")
        })
        deployScript.setExecutable(true)
        generatedFiles.add(GeneratedFile("scripts/deploy.sh", FileType.SHELL))
    }
    
    private fun generateIndividualScripts(config: CompiledConfig) {
        // Core system scripts (always generated)
        systemScriptGenerator.generateSystemConfigScript(config)
        systemScriptGenerator.generatePackageScript(config)
        systemScriptGenerator.generateServiceScript(config)
        systemScriptGenerator.generateUserScript(config)
        systemScriptGenerator.generateRepoScript(config)
        
        // Optional module scripts
        config.network?.let { networkScriptGenerator.generateNetworkScript(config) }
        config.boot?.let { systemScriptGenerator.generateBootScript(config) }
        config.hardware?.let { hardwareScriptGenerator.generateHardwareScript(config) }
        config.storage?.let { storageScriptGenerator.generateStorageScript(config) }
        config.security?.let { securityScriptGenerator.generateSecurityScript(config) }
        config.enhancedServices?.let { servicesScriptGenerator.generateEnhancedServicesScript(config) }
        config.development?.let { developmentScriptGenerator.generateDevelopmentScript(config) }
        config.environment?.let { developmentScriptGenerator.generateEnvironmentScript(config) }
        config.enhancedDesktop?.let { desktopScriptGenerator.generateEnhancedDesktopScript(config) }
        config.graphDesktop?.let { desktopScriptGenerator.generateGraphDesktopScript(config) }
        config.desktop?.let { desktopScriptGenerator.generateDesktopScript(config) }
        config.automation?.let { systemScriptGenerator.generateAutomationScript(config) }
    }
}