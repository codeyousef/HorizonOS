package org.horizonos.config.compiler.generators

import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import org.horizonos.config.dsl.CompiledConfig
import org.horizonos.config.compiler.GeneratedFile
import org.horizonos.config.compiler.FileType
import java.io.File

/**
 * JSON output generator for HorizonOS configurations
 * Generates comprehensive JSON representation of the compiled configuration
 */
class JsonGenerator(
    private val outputDir: File,
    private val generatedFiles: MutableList<GeneratedFile>
) {
    
    private val json = Json { 
        prettyPrint = true
        encodeDefaults = true
    }
    
    /**
     * Generate JSON output files from compiled configuration
     */
    fun generate(config: CompiledConfig) {
        // Generate main configuration JSON
        val jsonFile = File(outputDir, "json/config.json")
        jsonFile.writeText(json.encodeToString(config))
        generatedFiles.add(GeneratedFile("json/config.json", FileType.JSON))
        
        // Generate separate JSON files for each component
        generateComponentJsonFiles(config)
    }
    
    private fun generateComponentJsonFiles(config: CompiledConfig) {
        // Core system components (always present)
        File(outputDir, "json/system.json").writeText(json.encodeToString(config.system))
        File(outputDir, "json/packages.json").writeText(json.encodeToString(config.packages))
        File(outputDir, "json/services.json").writeText(json.encodeToString(config.services))
        File(outputDir, "json/users.json").writeText(json.encodeToString(config.users))
        File(outputDir, "json/repositories.json").writeText(json.encodeToString(config.repositories))
        
        // Optional configuration modules
        config.desktop?.let {
            File(outputDir, "json/desktop.json").writeText(json.encodeToString(it))
        }
        
        config.automation?.let {
            File(outputDir, "json/automation.json").writeText(json.encodeToString(it))
        }
        
        config.ai?.let {
            File(outputDir, "json/ai.json").writeText(json.encodeToString(it))
        }
        
        config.network?.let {
            File(outputDir, "json/network.json").writeText(json.encodeToString(it))
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
        
        config.enhancedDesktop?.let {
            File(outputDir, "json/enhanced-desktop.json").writeText(json.encodeToString(it))
        }
        
        config.graphDesktop?.let {
            File(outputDir, "json/graph-desktop.json").writeText(json.encodeToString(it))
        }
    }
}