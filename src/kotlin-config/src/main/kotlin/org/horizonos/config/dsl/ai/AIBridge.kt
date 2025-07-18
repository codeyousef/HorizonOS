package org.horizonos.config.dsl.ai

import org.horizonos.config.dsl.*

/**
 * Bridge between the enhanced AI settings DSL and the existing AI configuration
 * 
 * This provides compatibility between the new comprehensive AI settings
 * and the existing AI configuration structure.
 */

/**
 * Extension function to convert AISettings to AIConfig
 */
fun AISettings.toAIConfig(): AIConfig {
    val providers = mutableListOf<AIProvider>()
    val models = mutableListOf<AIModel>()
    val services = mutableListOf<AIService>()
    
    // Create Ollama provider
    providers.add(AIProvider(
        name = "ollama",
        type = ProviderType.OLLAMA,
        endpoint = ollamaEndpoint.substringBefore(":").substringAfter("//"),
        port = ollamaEndpoint.substringAfterLast(":").toIntOrNull() ?: 11434,
        enabled = enabled
    ))
    
    // Create default model
    models.add(AIModel(
        name = "default",
        provider = "ollama",
        modelPath = defaultModel,
        size = ModelSize.MEDIUM,
        quantization = ModelQuantization.AUTO,
        capabilities = listOf(
            ModelCapability.TEXT_GENERATION,
            ModelCapability.CODE_GENERATION,
            ModelCapability.QUESTION_ANSWERING
        ),
        enabled = enabled
    ))
    
    // Add configured agents as models
    val agentConfig = this.toConfig()["agents"] as? Map<*, *>
    val agents = (agentConfig?.get("agents") as? List<*>)?.filterIsInstance<Map<String, Any>>() ?: emptyList()
    
    agents.forEach { agent ->
        val name = agent["name"] as? String ?: return@forEach
        val model = agent["model"] as? String
        val capabilities = (agent["capabilities"] as? List<*>)?.filterIsInstance<String>() ?: emptyList()
        
        models.add(AIModel(
            name = name,
            provider = "ollama",
            modelPath = model ?: defaultModel,
            size = ModelSize.MEDIUM,
            quantization = ModelQuantization.AUTO,
            capabilities = mapCapabilities(capabilities),
            parameters = mapOf(
                "temperature" to (agent["temperature"]?.toString() ?: "0.7"),
                "max_tokens" to (agent["max_tokens"]?.toString() ?: "4096")
            ),
            enabled = agent["enabled"] as? Boolean ?: true
        ))
    }
    
    // Create AI hardware config
    val hardwareConfig = this.toConfig()["hardware"] as? Map<*, *>
    val hardware = AIHardware(
        gpuAcceleration = hardwareConfig?.get("optimization") != "CPU_ONLY",
        cpuThreads = (hardwareConfig?.get("cpu_threads") as? Int) ?: 0,
        memoryLimit = (hardwareConfig?.get("gpu_memory_limit") as? Long)?.let { "${it / 1_000_000_000}GB" } ?: "auto",
        optimization = mapHardwareOptimization(hardwareConfig?.get("optimization") as? String),
        backends = listOf(AIBackend.OLLAMA)
    )
    
    // Create AI privacy config
    val privacyConfig = this.toConfig()["privacy"] as? Map<*, *>
    val privacy = AIPrivacy(
        localOnly = privacyConfig?.get("local_only") as? Boolean ?: true,
        telemetryEnabled = privacyConfig?.get("telemetry_enabled") as? Boolean ?: false,
        dataRetention = mapDataRetention(privacyConfig?.get("data_retention") as? Map<*, *>),
        encryptStorage = privacyConfig?.get("encrypt_storage") as? Boolean ?: true,
        allowedNetworkAccess = emptyList()
    )
    
    return AIConfig(
        enabled = enabled,
        models = models,
        providers = providers,
        services = services,
        hardware = hardware,
        privacy = privacy
    )
}

/**
 * Map capability strings to ModelCapability enum
 */
private fun mapCapabilities(capabilities: List<String>): List<ModelCapability> {
    return capabilities.mapNotNull { cap ->
        when (cap) {
            "code-generation" -> ModelCapability.CODE_GENERATION
            "code-review" -> ModelCapability.CODE_GENERATION
            "refactoring" -> ModelCapability.CODE_GENERATION
            "web-search" -> ModelCapability.QUESTION_ANSWERING
            "summarization" -> ModelCapability.SUMMARIZATION
            "fact-checking" -> ModelCapability.QUESTION_ANSWERING
            "task-decomposition" -> ModelCapability.REASONING
            "scheduling" -> ModelCapability.REASONING
            "priority-management" -> ModelCapability.REASONING
            "workflow-creation" -> ModelCapability.FUNCTION_CALLING
            "process-optimization" -> ModelCapability.REASONING
            "ui-automation" -> ModelCapability.FUNCTION_CALLING
            else -> null
        }
    }
}

/**
 * Map hardware optimization string to enum
 */
private fun mapHardwareOptimization(optimization: String?): org.horizonos.config.dsl.HardwareOptimization {
    return when (optimization) {
        "AUTO" -> org.horizonos.config.dsl.HardwareOptimization.AUTO
        "PREFER_GPU" -> org.horizonos.config.dsl.HardwareOptimization.AUTO
        "CPU_ONLY" -> org.horizonos.config.dsl.HardwareOptimization.CPU_ONLY
        "POWER_SAVING" -> org.horizonos.config.dsl.HardwareOptimization.CPU_ONLY
        else -> org.horizonos.config.dsl.HardwareOptimization.AUTO
    }
}

/**
 * Map data retention config to enum
 */
private fun mapDataRetention(retention: Map<*, *>?): org.horizonos.config.dsl.DataRetention {
    return when (retention?.get("type")) {
        "session_only" -> org.horizonos.config.dsl.DataRetention.SESSION_ONLY
        "days" -> {
            val days = retention["days"] as? Int ?: 30
            when {
                days <= 1 -> org.horizonos.config.dsl.DataRetention.DAILY
                days <= 7 -> org.horizonos.config.dsl.DataRetention.WEEKLY
                else -> org.horizonos.config.dsl.DataRetention.CUSTOM
            }
        }
        "forever" -> org.horizonos.config.dsl.DataRetention.NEVER_DELETE
        else -> org.horizonos.config.dsl.DataRetention.SESSION_ONLY
    }
}

/**
 * Extension function to create AIContext from AISettings
 */
fun AIContext.fromSettings(settings: AISettings) {
    enabled = settings.enabled
    
    // Add default provider
    provider("ollama", ProviderType.OLLAMA) {
        endpoint = settings.ollamaEndpoint.substringBefore(":").substringAfter("//")
        port = settings.ollamaEndpoint.substringAfterLast(":").toIntOrNull() ?: 11434
    }
    
    // Add default model
    model("default") {
        provider = "ollama"
        modelPath = settings.defaultModel
        size = ModelSize.MEDIUM
        quantization = ModelQuantization.AUTO
        capabilities(
            ModelCapability.TEXT_GENERATION,
            ModelCapability.CODE_GENERATION,
            ModelCapability.QUESTION_ANSWERING
        )
    }
    
    // Configure hardware
    hardware {
        val hardwareConfig = settings.toConfig()["hardware"] as? Map<*, *>
        gpuAcceleration = hardwareConfig?.get("optimization") != "CPU_ONLY"
        cpuThreads = (hardwareConfig?.get("cpu_threads") as? Int) ?: 0
        memoryLimit = (hardwareConfig?.get("gpu_memory_limit") as? Long)?.let { "${it / 1_000_000_000}GB" } ?: "auto"
        optimization = mapHardwareOptimization(hardwareConfig?.get("optimization") as? String)
        backends(AIBackend.OLLAMA)
    }
    
    // Configure privacy
    privacy {
        val privacyConfig = settings.toConfig()["privacy"] as? Map<*, *>
        localOnly = privacyConfig?.get("local_only") as? Boolean ?: true
        telemetryEnabled = privacyConfig?.get("telemetry_enabled") as? Boolean ?: false
        dataRetention = mapDataRetention(privacyConfig?.get("data_retention") as? Map<*, *>)
        encryptStorage = privacyConfig?.get("encrypt_storage") as? Boolean ?: true
    }
}