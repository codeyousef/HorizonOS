package org.horizonos.config.compiler

import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import org.horizonos.config.dsl.*
import org.horizonos.config.dsl.hardware.*
import org.horizonos.config.dsl.security.*
import org.horizonos.config.dsl.services.*
import org.horizonos.config.compiler.generators.*
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
    
    // Specialized generators
    private val jsonGenerator = JsonGenerator(outputDir, generatedFiles)
    private val yamlGenerator = YamlGenerator(outputDir, generatedFiles)
    private val systemdGenerator = SystemdGenerator(outputDir, generatedFiles)
    private val shellScriptGenerator = ShellScriptGenerator(outputDir, generatedFiles)
    private val ansibleGenerator = AnsibleGenerator(outputDir, generatedFiles)
    private val dockerGenerator = DockerGenerator(outputDir, generatedFiles)
    private val osTreeManifestGenerator = OSTreeManifestGenerator(outputDir, generatedFiles)
    private val automationGenerator = AutomationGenerator(outputDir, generatedFiles)
    private val aiConfigurationGenerator = AIConfigurationGenerator(outputDir, generatedFiles)
    private val documentationGenerator = DocumentationGenerator(outputDir, generatedFiles)
    private val containerGenerator = ContainerGenerator()
    private val layerGenerator = LayerGenerator()
    
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
            osTreeManifestGenerator.generate(config)
            automationGenerator.generate(config)
            aiConfigurationGenerator.generate(config)
            documentationGenerator.generate(config)
            
            // Generate container and layer deployment scripts
            containerGenerator.generateContainerDeployment(config, outputDir)
            layerGenerator.generateLayerDeployment(config, outputDir)
            
            return GenerationResult.Success(generatedFiles.toList())
        } catch (e: Exception) {
            return GenerationResult.Error(GenerationError.UnexpectedError(e.message ?: "Unknown error", e))
        }
    }
    
    private fun createDirectoryStructure() {
        val dirs = listOf(
            "json", "yaml", "scripts", "systemd", "ansible", 
            "docker", "ostree", "automation", "docs", "configs",
            "configs/ollama", "configs/llm-api", "configs/nginx",
            "configs/vector-db", "configs/llm-privacy", "automation/tasks",
            "automation/workflows", "automation/systemd", "automation/cron.d",
            "docs/services", "containers", "layers"
        )
        dirs.forEach { 
            File(outputDir, it).mkdirs() 
        }
    }
}