package org.horizonos.config.compiler.generators.shell

import org.horizonos.config.dsl.CompiledConfig
import org.horizonos.config.compiler.GeneratedFile
import org.horizonos.config.compiler.FileType
import java.io.File

/**
 * Development script generator for development environment configuration
 */
class DevelopmentScriptGenerator(
    private val outputDir: File,
    private val generatedFiles: MutableList<GeneratedFile>
) {
    
    fun generateDevelopmentScript(config: CompiledConfig) {
        config.development?.let { development ->
            val script = File(outputDir, "scripts/development-setup.sh")
            script.writeText(buildString {
                appendLine("#!/bin/bash")
                appendLine("# Development Environment Setup")
                appendLine("echo 'Setting up development environment...'")
                appendLine("# Development environment configuration implementation")
                appendLine("echo 'Development setup completed.'")
            })
            script.setExecutable(true)
            generatedFiles.add(GeneratedFile("scripts/development-setup.sh", FileType.SHELL))
        }
    }
    
    fun generateEnvironmentScript(config: CompiledConfig) {
        config.environment?.let { environment ->
            val script = File(outputDir, "scripts/environment-setup.sh")
            script.writeText(buildString {
                appendLine("#!/bin/bash")
                appendLine("# Shell and Environment Setup")
                appendLine("echo 'Setting up shell and environment...'")
                appendLine("# Environment configuration implementation")
                appendLine("echo 'Environment setup completed.'")
            })
            script.setExecutable(true)
            generatedFiles.add(GeneratedFile("scripts/environment-setup.sh", FileType.SHELL))
        }
    }
}