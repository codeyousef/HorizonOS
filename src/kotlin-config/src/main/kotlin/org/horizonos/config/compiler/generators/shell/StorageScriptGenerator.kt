package org.horizonos.config.compiler.generators.shell

import org.horizonos.config.dsl.CompiledConfig
import org.horizonos.config.compiler.GeneratedFile
import org.horizonos.config.compiler.FileType
import java.io.File

/**
 * Storage script generator for storage configuration
 */
class StorageScriptGenerator(
    private val outputDir: File,
    private val generatedFiles: MutableList<GeneratedFile>
) {
    
    fun generateStorageScript(config: CompiledConfig) {
        config.storage?.let { storage ->
            val script = File(outputDir, "scripts/storage-config.sh")
            script.writeText(buildString {
                appendLine("#!/bin/bash")
                appendLine("# Storage Configuration")
                appendLine("echo 'Setting up storage configuration...'")
                appendLine("# Storage configuration implementation")
                appendLine("echo 'Storage configuration completed.'")
            })
            script.setExecutable(true)
            generatedFiles.add(GeneratedFile("scripts/storage-config.sh", FileType.SHELL))
        }
    }
}