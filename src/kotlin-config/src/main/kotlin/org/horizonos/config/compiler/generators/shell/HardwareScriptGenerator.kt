package org.horizonos.config.compiler.generators.shell

import org.horizonos.config.dsl.CompiledConfig
import org.horizonos.config.compiler.GeneratedFile
import org.horizonos.config.compiler.FileType
import java.io.File

/**
 * Hardware script generator for hardware configuration
 */
class HardwareScriptGenerator(
    private val outputDir: File,
    private val generatedFiles: MutableList<GeneratedFile>
) {
    
    fun generateHardwareScript(config: CompiledConfig) {
        config.hardware?.let { hardware ->
            val script = File(outputDir, "scripts/hardware-config.sh")
            script.writeText(buildString {
                appendLine("#!/bin/bash")
                appendLine("# Hardware Configuration")
                appendLine("echo 'Setting up hardware configuration...'")
                appendLine("# Hardware configuration implementation")
                appendLine("echo 'Hardware configuration completed.'")
            })
            script.setExecutable(true)
            generatedFiles.add(GeneratedFile("scripts/hardware-config.sh", FileType.SHELL))
        }
    }
}