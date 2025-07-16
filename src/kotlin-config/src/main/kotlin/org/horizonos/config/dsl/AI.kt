package org.horizonos.config.dsl

import kotlinx.serialization.Serializable
import kotlin.time.Duration
import kotlin.time.Duration.Companion.seconds

/**
 * AI/LLM Integration DSL for HorizonOS
 * 
 * Provides type-safe configuration for local LLM integration with automatic
 * hardware optimization and privacy-focused local execution.
 */

// ===== AI Configuration =====

@Serializable
data class AIConfig(
    val enabled: Boolean = false,
    val models: List<AIModel> = emptyList(),
    val providers: List<AIProvider> = emptyList(),
    val services: List<AIService> = emptyList(),
    val hardware: AIHardware = AIHardware(),
    val privacy: AIPrivacy = AIPrivacy()
)

@Serializable
data class AIModel(
    val name: String,
    val provider: String,
    val modelPath: String = "",
    val size: ModelSize = ModelSize.MEDIUM,
    val quantization: ModelQuantization = ModelQuantization.Q4_0,
    val capabilities: List<ModelCapability> = emptyList(),
    val parameters: Map<String, String> = emptyMap(),
    val enabled: Boolean = true,
    val preload: Boolean = false
)

@Serializable
data class AIProvider(
    val name: String,
    val type: ProviderType,
    val endpoint: String = "localhost",
    val port: Int = 11434,
    val apiKey: String = "",
    val models: List<String> = emptyList(),
    val enabled: Boolean = true
)

@Serializable
data class AIService(
    val name: String,
    val description: String = "",
    val model: String,
    val prompt: String = "",
    val temperature: Double = 0.7,
    val maxTokens: Int = 2048,
    val timeout: Long = 30,
    val enabled: Boolean = true
)

@Serializable
data class AIHardware(
    val gpuAcceleration: Boolean = true,
    val cpuThreads: Int = 0, // 0 = auto-detect
    val memoryLimit: String = "auto", // e.g. "8GB", "auto"
    val optimization: HardwareOptimization = HardwareOptimization.AUTO,
    val backends: List<AIBackend> = listOf(AIBackend.OLLAMA)
)

@Serializable
data class AIPrivacy(
    val localOnly: Boolean = true,
    val telemetryEnabled: Boolean = false,
    val dataRetention: DataRetention = DataRetention.SESSION_ONLY,
    val encryptStorage: Boolean = true,
    val allowedNetworkAccess: List<String> = emptyList()
)

// ===== Enums =====

@Serializable
enum class ModelSize {
    TINY,    // < 1B parameters
    SMALL,   // 1-3B parameters  
    MEDIUM,  // 3-7B parameters
    LARGE,   // 7-13B parameters
    XLARGE,  // 13-30B parameters
    HUGE     // > 30B parameters
}

@Serializable
enum class ModelQuantization {
    F16,    // Full precision
    Q8_0,   // 8-bit quantization
    Q4_0,   // 4-bit quantization (balanced)
    Q4_1,   // 4-bit quantization (legacy)
    Q2_K,   // 2-bit quantization (very small)
    AUTO    // Auto-select based on hardware
}

@Serializable
enum class ModelCapability {
    TEXT_GENERATION,
    CODE_GENERATION,
    TRANSLATION,
    SUMMARIZATION,
    QUESTION_ANSWERING,
    CLASSIFICATION,
    EMBEDDING,
    VISION,
    AUDIO_TRANSCRIPTION,
    FUNCTION_CALLING,
    REASONING,
    MATH
}

@Serializable
enum class ProviderType {
    OLLAMA,     // Local Ollama instance
    LLAMACPP,   // Direct llama.cpp integration
    OPENAI_API, // OpenAI-compatible API
    HUGGINGFACE,// HuggingFace transformers
    CUSTOM      // Custom provider
}

@Serializable
enum class HardwareOptimization {
    AUTO,       // Automatic optimization
    CPU_ONLY,   // CPU-only execution
    GPU_NVIDIA, // NVIDIA GPU optimization
    GPU_AMD,    // AMD GPU optimization
    GPU_INTEL,  // Intel GPU optimization
    METAL,      // Apple Metal
    CUSTOM      // Custom optimization
}

@Serializable
enum class AIBackend {
    OLLAMA,     // Ollama backend
    LLAMACPP,   // llama.cpp backend
    PYTORCH,    // PyTorch backend
    ONNX,       // ONNX Runtime
    TENSORRT,   // NVIDIA TensorRT
    OPENVINO    // Intel OpenVINO
}

@Serializable
enum class DataRetention {
    SESSION_ONLY,  // Data cleared after session
    DAILY,         // Data kept for 24 hours
    WEEKLY,        // Data kept for 7 days
    NEVER_DELETE,  // Data kept indefinitely
    CUSTOM         // Custom retention policy
}

// ===== DSL Builders =====

@HorizonOSDsl
class AIContext {
    var enabled: Boolean = false
    private val models = mutableListOf<AIModel>()
    private val providers = mutableListOf<AIProvider>()
    private val services = mutableListOf<AIService>()
    private var hardware = AIHardware()
    private var privacy = AIPrivacy()
    
    fun model(name: String, block: AIModelContext.() -> Unit) {
        val context = AIModelContext().apply {
            this.name = name
            block()
        }
        models.add(context.toModel())
    }
    
    fun provider(name: String, type: ProviderType, block: AIProviderContext.() -> Unit = {}) {
        val context = AIProviderContext().apply {
            this.name = name
            this.type = type
            block()
        }
        providers.add(context.toProvider())
    }
    
    fun service(name: String, model: String, block: AIServiceContext.() -> Unit = {}) {
        val context = AIServiceContext().apply {
            this.name = name
            this.model = model
            block()
        }
        services.add(context.toService())
    }
    
    fun hardware(block: AIHardwareContext.() -> Unit) {
        hardware = AIHardwareContext().apply(block).toHardware()
    }
    
    fun privacy(block: AIPrivacyContext.() -> Unit) {
        privacy = AIPrivacyContext().apply(block).toPrivacy()
    }
    
    fun toConfig() = AIConfig(enabled, models, providers, services, hardware, privacy)
}

@HorizonOSDsl
class AIModelContext {
    var name: String = ""
    var provider: String = "ollama"
    var modelPath: String = ""
    var size: ModelSize = ModelSize.MEDIUM
    var quantization: ModelQuantization = ModelQuantization.Q4_0
    var enabled: Boolean = true
    var preload: Boolean = false
    
    private val capabilities = mutableListOf<ModelCapability>()
    private val parameters = mutableMapOf<String, String>()
    
    fun capabilities(vararg caps: ModelCapability) {
        capabilities.addAll(caps)
    }
    
    fun parameter(key: String, value: String) {
        parameters[key] = value
    }
    
    fun toModel() = AIModel(
        name, provider, modelPath, size, quantization,
        capabilities.toList(), parameters.toMap(), enabled, preload
    )
}

@HorizonOSDsl
class AIProviderContext {
    var name: String = ""
    var type: ProviderType = ProviderType.OLLAMA
    var endpoint: String = "localhost"
    var port: Int = 11434
    var apiKey: String = ""
    var enabled: Boolean = true
    
    private val models = mutableListOf<String>()
    
    fun models(vararg modelNames: String) {
        models.addAll(modelNames)
    }
    
    fun toProvider() = AIProvider(name, type, endpoint, port, apiKey, models.toList(), enabled)
}

@HorizonOSDsl
class AIServiceContext {
    var name: String = ""
    var description: String = ""
    var model: String = ""
    var prompt: String = ""
    var temperature: Double = 0.7
    var maxTokens: Int = 2048
    var timeout: Long = 30
    var enabled: Boolean = true
    
    fun toService() = AIService(name, description, model, prompt, temperature, maxTokens, timeout, enabled)
}

@HorizonOSDsl
class AIHardwareContext {
    var gpuAcceleration: Boolean = true
    var cpuThreads: Int = 0
    var memoryLimit: String = "auto"
    var optimization: HardwareOptimization = HardwareOptimization.AUTO
    
    private val backends = mutableListOf<AIBackend>()
    
    fun backends(vararg backendList: AIBackend) {
        backends.clear()
        backends.addAll(backendList)
    }
    
    fun toHardware() = AIHardware(
        gpuAcceleration, cpuThreads, memoryLimit, optimization,
        if (backends.isEmpty()) listOf(AIBackend.OLLAMA) else backends.toList()
    )
}

@HorizonOSDsl
class AIPrivacyContext {
    var localOnly: Boolean = true
    var telemetryEnabled: Boolean = false
    var dataRetention: DataRetention = DataRetention.SESSION_ONLY
    var encryptStorage: Boolean = true
    
    private val allowedNetworkAccess = mutableListOf<String>()
    
    fun allowNetworkAccess(vararg hosts: String) {
        allowedNetworkAccess.addAll(hosts)
    }
    
    fun toPrivacy() = AIPrivacy(
        localOnly, telemetryEnabled, dataRetention, encryptStorage, allowedNetworkAccess.toList()
    )
}

// ===== Common Model Presets =====

object AIPresets {
    fun codeAssistant() = AIModelContext().apply {
        size = ModelSize.MEDIUM
        capabilities(
            ModelCapability.CODE_GENERATION,
            ModelCapability.TEXT_GENERATION,
            ModelCapability.QUESTION_ANSWERING
        )
        parameter("system_prompt", "You are a helpful coding assistant.")
    }
    
    fun chatBot() = AIModelContext().apply {
        size = ModelSize.SMALL
        capabilities(
            ModelCapability.TEXT_GENERATION,
            ModelCapability.QUESTION_ANSWERING
        )
        parameter("system_prompt", "You are a friendly and helpful assistant.")
    }
    
    fun translator() = AIModelContext().apply {
        size = ModelSize.SMALL
        capabilities(ModelCapability.TRANSLATION)
        parameter("system_prompt", "You are a professional translator.")
    }
    
    fun summarizer() = AIModelContext().apply {
        size = ModelSize.SMALL
        capabilities(ModelCapability.SUMMARIZATION)
        parameter("system_prompt", "You create concise, accurate summaries.")
    }
}

// ===== Extension Functions =====

fun CompiledConfig.hasAI(): Boolean = ai?.enabled == true

fun CompiledConfig.getAIModel(name: String): AIModel? = 
    ai?.models?.find { it.name == name }

fun CompiledConfig.getAIService(name: String): AIService? = 
    ai?.services?.find { it.name == name }