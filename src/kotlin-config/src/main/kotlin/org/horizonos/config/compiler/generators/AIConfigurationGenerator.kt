package org.horizonos.config.compiler.generators

import org.horizonos.config.dsl.CompiledConfig
import org.horizonos.config.compiler.FileType
import org.horizonos.config.compiler.GeneratedFile
import org.horizonos.config.dsl.*
// import org.horizonos.config.dsl.llm.*
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import java.io.File

/**
 * Generator for AI and LLM configurations
 */
class AIConfigurationGenerator(
    private val outputDir: File,
    private val generatedFiles: MutableList<GeneratedFile>
) {
    private val json = Json { 
        prettyPrint = true
        encodeDefaults = true
    }
    
    fun generate(config: CompiledConfig) {
        // TODO: LLM integration not yet implemented in DSL
        // Skipping AI configuration generation for now
        return
        
        /* Future implementation when LLM DSL is added:
        config.ai?.llmIntegration?.let { llm ->
            generateOllamaConfiguration(llm)
            generateModelConfigurations(llm)
            generateAPIConfigurations(llm)
            generateSemanticIndexConfiguration(llm)
            generateAIServiceScripts(llm)
            generatePrivacyConfiguration(llm)
        }
    }
    
    private fun generateOllamaConfiguration(llm: LLMIntegrationConfiguration) {
        // Ollama service configuration
        val ollamaConfig = File(outputDir, "configs/ollama/ollama.json")
        ollamaConfig.parentFile.mkdirs()
        
        ollamaConfig.writeText(json.encodeToString(mapOf(
            "bind" to "0.0.0.0:11434",
            "models_path" to "/var/lib/ollama/models",
            "gpu" to llm.hardwareAcceleration.gpu,
            "num_parallel" to llm.resourceLimits.maxConcurrentRequests,
            "num_cpu" to llm.resourceLimits.cpuThreads,
            "memory_limit" to llm.resourceLimits.memoryLimit,
            "cors_allowed_origins" to llm.privacy.allowedDomains
        )))
        generatedFiles.add(GeneratedFile("configs/ollama/ollama.json", FileType.JSON))
        
        // Systemd service override
        val serviceOverride = File(outputDir, "systemd/ollama.service.d/override.conf")
        serviceOverride.parentFile.mkdirs()
        serviceOverride.writeText(buildString {
            appendLine("[Service]")
            appendLine("Environment=\"OLLAMA_HOST=0.0.0.0\"")
            appendLine("Environment=\"OLLAMA_MODELS=/var/lib/ollama/models\"")
            if (llm.hardwareAcceleration.gpu) {
                appendLine("Environment=\"CUDA_VISIBLE_DEVICES=0\"")
            }
            appendLine("LimitNOFILE=1048576")
            appendLine("LimitNPROC=512")
            appendLine("MemoryLimit=${llm.resourceLimits.memoryLimit}")
            if (llm.resourceLimits.cpuThreads != null) {
                appendLine("CPUQuota=${llm.resourceLimits.cpuThreads}00%")
            }
        })
        generatedFiles.add(GeneratedFile("systemd/ollama.service.d/override.conf", FileType.CONFIG))
    }
    
    private fun generateModelConfigurations(llm: LLMIntegrationConfiguration) {
        val modelsDir = File(outputDir, "configs/ollama/models")
        modelsDir.mkdirs()
        
        llm.models.forEach { model ->
            val modelConfig = File(modelsDir, "${model.name}.json")
            modelConfig.writeText(json.encodeToString(mapOf(
                "name" to model.name,
                "parameters" to mapOf(
                    "temperature" to model.temperature,
                    "top_p" to model.topP,
                    "top_k" to model.topK,
                    "num_ctx" to model.contextLength,
                    "num_predict" to model.maxTokens,
                    "stop" to model.stopSequences,
                    "system" to model.systemPrompt,
                    "repeat_penalty" to model.repeatPenalty,
                    "seed" to model.seed
                ),
                "options" to mapOf(
                    "num_gpu" to if (llm.hardwareAcceleration.gpu) model.gpuLayers else 0,
                    "main_gpu" to 0,
                    "low_vram" to llm.hardwareAcceleration.lowVRAM
                )
            )))
            generatedFiles.add(GeneratedFile("configs/ollama/models/${model.name}.json", FileType.JSON))
        }
        
        // Model pull script
        val pullScript = File(outputDir, "scripts/pull-llm-models.sh")
        pullScript.writeText(buildString {
            appendLine("#!/bin/bash")
            appendLine("# Pull configured LLM models")
            appendLine()
            appendLine("set -euo pipefail")
            appendLine()
            appendLine("echo 'Pulling LLM models for HorizonOS...'")
            appendLine()
            
            llm.models.forEach { model ->
                appendLine("echo 'Pulling model: ${model.name}'")
                appendLine("ollama pull ${model.name}")
                
                // Apply model configuration
                val configPath = "/etc/horizonos/ollama/models/${model.name}.json"
                appendLine("if [[ -f \"$configPath\" ]]; then")
                appendLine("    echo 'Applying model configuration...'")
                appendLine("    # TODO: Apply model-specific configuration")
                appendLine("fi")
                appendLine()
            }
            
            appendLine("echo 'All models pulled successfully!'")
        })
        pullScript.setExecutable(true)
        generatedFiles.add(GeneratedFile("scripts/pull-llm-models.sh", FileType.SHELL))
    }
    
    private fun generateAPIConfigurations(llm: LLMIntegrationConfiguration) {
        // API gateway configuration
        val apiConfig = File(outputDir, "configs/llm-api/config.json")
        apiConfig.parentFile.mkdirs()
        
        apiConfig.writeText(json.encodeToString(mapOf(
            "endpoints" to llm.apis.map { api ->
                mapOf(
                    "name" to api.name,
                    "endpoint" to api.endpoint,
                    "enabled" to api.enabled,
                    "rateLimit" to api.rateLimit,
                    "timeout" to api.timeout,
                    "authentication" to api.authentication,
                    "allowedMethods" to api.allowedMethods
                )
            },
            "security" to mapOf(
                "cors" to mapOf(
                    "enabled" to true,
                    "allowedOrigins" to llm.privacy.allowedDomains,
                    "allowedMethods" to listOf("GET", "POST", "OPTIONS"),
                    "allowedHeaders" to listOf("Content-Type", "Authorization")
                ),
                "rateLimit" to mapOf(
                    "enabled" to true,
                    "windowMs" to 60000,
                    "max" to 100
                )
            )
        )))
        generatedFiles.add(GeneratedFile("configs/llm-api/config.json", FileType.JSON))
        
        // NGINX configuration for API proxy
        val nginxConfig = File(outputDir, "configs/nginx/llm-api.conf")
        nginxConfig.parentFile.mkdirs()
        nginxConfig.writeText(buildString {
            appendLine("# LLM API Proxy Configuration")
            appendLine()
            appendLine("upstream ollama_backend {")
            appendLine("    server localhost:11434;")
            appendLine("    keepalive 32;")
            appendLine("}")
            appendLine()
            
            llm.apis.forEach { api ->
                if (api.enabled) {
                    appendLine("location ${api.endpoint} {")
                    appendLine("    proxy_pass http://ollama_backend;")
                    appendLine("    proxy_http_version 1.1;")
                    appendLine("    proxy_set_header Connection \"\";")
                    appendLine("    proxy_set_header Host \$host;")
                    appendLine("    proxy_set_header X-Real-IP \$remote_addr;")
                    appendLine("    proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;")
                    appendLine("    proxy_buffering off;")
                    appendLine("    proxy_read_timeout ${api.timeout}s;")
                    
                    if (api.authentication == "token") {
                        appendLine("    ")
                        appendLine("    # Token authentication")
                        appendLine("    if (\$http_authorization !~ \"Bearer .+\") {")
                        appendLine("        return 401;")
                        appendLine("    }")
                    }
                    
                    if (api.rateLimit != null) {
                        appendLine("    ")
                        appendLine("    # Rate limiting")
                        appendLine("    limit_req zone=llm_api burst=${api.rateLimit} nodelay;")
                    }
                    appendLine("}")
                    appendLine()
                }
            }
        })
        generatedFiles.add(GeneratedFile("configs/nginx/llm-api.conf", FileType.CONFIG))
    }
    
    private fun generateSemanticIndexConfiguration(llm: LLMIntegrationConfiguration) {
        llm.semanticIndex?.let { index ->
            // Vector database configuration
            val vectorDBConfig = File(outputDir, "configs/vector-db/config.json")
            vectorDBConfig.parentFile.mkdirs()
            
            vectorDBConfig.writeText(json.encodeToString(mapOf(
                "enabled" to index.enabled,
                "updateFrequency" to index.updateFrequency,
                "includePaths" to index.includePaths,
                "excludePaths" to index.excludePaths,
                "embeddingModel" to index.embeddingModel,
                "chunkSize" to index.chunkSize,
                "chunkOverlap" to index.chunkOverlap,
                "database" to mapOf(
                    "type" to "chromadb",
                    "path" to "/var/lib/horizonos/semantic-index",
                    "collection" to "system_index"
                )
            )))
            generatedFiles.add(GeneratedFile("configs/vector-db/config.json", FileType.JSON))
            
            // Indexing service script
            val indexScript = File(outputDir, "scripts/semantic-indexer.sh")
            indexScript.writeText(buildString {
                appendLine("#!/bin/bash")
                appendLine("# Semantic Index Builder for HorizonOS")
                appendLine()
                appendLine("set -euo pipefail")
                appendLine()
                appendLine("INDEX_CONFIG=\"/etc/horizonos/vector-db/config.json\"")
                appendLine("INDEX_DB=\"/var/lib/horizonos/semantic-index\"")
                appendLine()
                appendLine("# Create index directory")
                appendLine("mkdir -p \"\$INDEX_DB\"")
                appendLine()
                appendLine("# Function to index a file")
                appendLine("index_file() {")
                appendLine("    local file=\"\$1\"")
                appendLine("    echo \"Indexing: \$file\"")
                appendLine("    # TODO: Implement actual indexing logic")
                appendLine("    # This would involve:")
                appendLine("    # 1. Reading the file")
                appendLine("    # 2. Chunking the content")
                appendLine("    # 3. Generating embeddings using the configured model")
                appendLine("    # 4. Storing in the vector database")
                appendLine("}")
                appendLine()
                appendLine("# Process include paths")
                index.includePaths.forEach { path ->
                    appendLine("find \"$path\" -type f -name \"*.txt\" -o -name \"*.md\" -o -name \"*.conf\" | while read -r file; do")
                    appendLine("    index_file \"\$file\"")
                    appendLine("done")
                }
                appendLine()
                appendLine("echo 'Semantic indexing complete!'")
            })
            indexScript.setExecutable(true)
            generatedFiles.add(GeneratedFile("scripts/semantic-indexer.sh", FileType.SHELL))
        }
    }
    
    private fun generateAIServiceScripts(llm: LLMIntegrationConfiguration) {
        // Main AI service management script
        val serviceScript = File(outputDir, "scripts/horizonos-ai-service.sh")
        serviceScript.writeText(buildString {
            appendLine("#!/bin/bash")
            appendLine("# HorizonOS AI Service Manager")
            appendLine()
            appendLine("set -euo pipefail")
            appendLine()
            appendLine("ACTION=\"\${1:-status}\"")
            appendLine()
            appendLine("start_services() {")
            appendLine("    echo 'Starting HorizonOS AI services...'")
            appendLine("    systemctl start ollama")
            llm.apis.forEach { api ->
                if (api.enabled) {
                    appendLine("    systemctl start horizonos-llm-${api.name}")
                }
            }
            if (llm.semanticIndex?.enabled == true) {
                appendLine("    systemctl start horizonos-semantic-index")
            }
            appendLine("    echo 'AI services started.'")
            appendLine("}")
            appendLine()
            appendLine("stop_services() {")
            appendLine("    echo 'Stopping HorizonOS AI services...'")
            appendLine("    systemctl stop ollama")
            llm.apis.forEach { api ->
                if (api.enabled) {
                    appendLine("    systemctl stop horizonos-llm-${api.name}")
                }
            }
            if (llm.semanticIndex?.enabled == true) {
                appendLine("    systemctl stop horizonos-semantic-index")
            }
            appendLine("    echo 'AI services stopped.'")
            appendLine("}")
            appendLine()
            appendLine("status_services() {")
            appendLine("    echo 'HorizonOS AI Services Status:'")
            appendLine("    echo")
            appendLine("    systemctl status ollama --no-pager || true")
            appendLine("    echo")
            llm.apis.forEach { api ->
                if (api.enabled) {
                    appendLine("    systemctl status horizonos-llm-${api.name} --no-pager || true")
                    appendLine("    echo")
                }
            }
            if (llm.semanticIndex?.enabled == true) {
                appendLine("    systemctl status horizonos-semantic-index --no-pager || true")
            }
            appendLine("}")
            appendLine()
            appendLine("case \"\$ACTION\" in")
            appendLine("    start)")
            appendLine("        start_services")
            appendLine("        ;;")
            appendLine("    stop)")
            appendLine("        stop_services")
            appendLine("        ;;")
            appendLine("    restart)")
            appendLine("        stop_services")
            appendLine("        sleep 2")
            appendLine("        start_services")
            appendLine("        ;;")
            appendLine("    status)")
            appendLine("        status_services")
            appendLine("        ;;")
            appendLine("    *)")
            appendLine("        echo \"Usage: \$0 {start|stop|restart|status}\"")
            appendLine("        exit 1")
            appendLine("        ;;")
            appendLine("esac")
        })
        serviceScript.setExecutable(true)
        generatedFiles.add(GeneratedFile("scripts/horizonos-ai-service.sh", FileType.SHELL))
    }
    
    private fun generatePrivacyConfiguration(llm: LLMIntegrationConfiguration) {
        val privacyConfig = File(outputDir, "configs/llm-privacy/privacy.json")
        privacyConfig.parentFile.mkdirs()
        
        privacyConfig.writeText(json.encodeToString(mapOf(
            "localOnly" to llm.privacy.localOnly,
            "telemetryDisabled" to llm.privacy.telemetryDisabled,
            "dataRetention" to llm.privacy.dataRetention,
            "encryptionRequired" to llm.privacy.encryptionRequired,
            "allowedDomains" to llm.privacy.allowedDomains,
            "sensitiveDataFilter" to mapOf(
                "enabled" to true,
                "patterns" to listOf(
                    "\\b\\d{3}-\\d{2}-\\d{4}\\b",  // SSN pattern
                    "\\b\\d{16}\\b",                // Credit card pattern
                    "\\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\\.[A-Z|a-z]{2,}\\b" // Email pattern
                )
            ),
            "audit" to mapOf(
                "enabled" to true,
                "logPath" to "/var/log/horizonos/llm-audit.log",
                "logLevel" to "INFO",
                "includePrompts" to false,
                "includeResponses" to false
            )
        )))
        generatedFiles.add(GeneratedFile("configs/llm-privacy/privacy.json", FileType.JSON))
        
        // Privacy enforcement script
        val privacyScript = File(outputDir, "scripts/llm-privacy-check.sh")
        privacyScript.writeText(buildString {
            appendLine("#!/bin/bash")
            appendLine("# LLM Privacy Enforcement Check")
            appendLine()
            appendLine("set -euo pipefail")
            appendLine()
            appendLine("PRIVACY_CONFIG=\"/etc/horizonos/llm-privacy/privacy.json\"")
            appendLine()
            appendLine("# Check network isolation")
            appendLine("check_network_isolation() {")
            appendLine("    if grep -q '\"localOnly\": true' \"\$PRIVACY_CONFIG\"; then")
            appendLine("        echo 'Checking network isolation...'")
            appendLine("        # Verify firewall rules")
            appendLine("        if iptables -L | grep -q 'DROP.*11434'; then")
            appendLine("            echo '✓ Network isolation enforced'")
            appendLine("        else")
            appendLine("            echo '✗ WARNING: Network isolation not properly configured'")
            appendLine("            exit 1")
            appendLine("        fi")
            appendLine("    fi")
            appendLine("}")
            appendLine()
            appendLine("# Check data encryption")
            appendLine("check_encryption() {")
            appendLine("    if grep -q '\"encryptionRequired\": true' \"\$PRIVACY_CONFIG\"; then")
            appendLine("        echo 'Checking data encryption...'")
            appendLine("        # Verify encryption is enabled")
            appendLine("        if [[ -d \"/var/lib/ollama/models\" ]]; then")
            appendLine("            # Check if directory is on encrypted filesystem")
            appendLine("            if findmnt -n -o FSTYPE /var/lib/ollama | grep -q 'crypt'; then")
            appendLine("                echo '✓ Model storage is encrypted'")
            appendLine("            else")
            appendLine("                echo '✗ WARNING: Model storage is not encrypted'")
            appendLine("                exit 1")
            appendLine("            fi")
            appendLine("        fi")
            appendLine("    fi")
            appendLine("}")
            appendLine()
            appendLine("# Run all checks")
            appendLine("echo 'Running LLM privacy checks...'")
            appendLine("check_network_isolation")
            appendLine("check_encryption")
            appendLine("echo 'All privacy checks passed!'")
        })
        privacyScript.setExecutable(true)
        generatedFiles.add(GeneratedFile("scripts/llm-privacy-check.sh", FileType.SHELL))
    }
    */
    }
}