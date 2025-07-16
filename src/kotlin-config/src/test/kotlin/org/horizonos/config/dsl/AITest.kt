package org.horizonos.config.dsl

import io.kotest.core.spec.style.StringSpec
import io.kotest.matchers.shouldBe
import io.kotest.matchers.shouldNotBe
import io.kotest.matchers.collections.shouldHaveSize
import io.kotest.matchers.collections.shouldContain
import io.kotest.matchers.booleans.shouldBeTrue
import io.kotest.matchers.booleans.shouldBeFalse

class AITest : StringSpec({
    
    "should create basic AI configuration" {
        val config = horizonOS {
            hostname = "ai-test"
            timezone = "UTC"
            locale = "en_US.UTF-8"
            
            ai {
                enabled = true
                
                model("llama3") {
                    provider = "ollama"
                    size = ModelSize.MEDIUM
                    quantization = ModelQuantization.Q4_0
                    capabilities(ModelCapability.TEXT_GENERATION, ModelCapability.CODE_GENERATION)
                }
                
                provider("ollama", ProviderType.OLLAMA) {
                    endpoint = "localhost"
                    port = 11434
                    models("llama3", "codellama")
                }
            }
        }
        
        config.ai shouldNotBe null
        config.ai!!.enabled.shouldBeTrue()
        config.ai!!.models shouldHaveSize 1
        config.ai!!.providers shouldHaveSize 1
        
        val model = config.ai!!.models[0]
        model.name shouldBe "llama3"
        model.provider shouldBe "ollama"
        model.size shouldBe ModelSize.MEDIUM
        model.capabilities shouldContain ModelCapability.TEXT_GENERATION
        model.capabilities shouldContain ModelCapability.CODE_GENERATION
        
        val provider = config.ai!!.providers[0]
        provider.name shouldBe "ollama"
        provider.type shouldBe ProviderType.OLLAMA
        provider.port shouldBe 11434
        provider.models shouldContain "llama3"
        provider.models shouldContain "codellama"
    }
    
    "should create AI services" {
        val config = horizonOS {
            hostname = "ai-services"
            timezone = "UTC"
            locale = "en_US.UTF-8"
            
            ai {
                enabled = true
                
                service("code-assistant", "codellama") {
                    description = "AI-powered code completion"
                    prompt = "You are a helpful coding assistant"
                    temperature = 0.3
                    maxTokens = 4096
                }
                
                service("chat-bot", "llama3") {
                    description = "General purpose chat"
                    temperature = 0.7
                    maxTokens = 2048
                }
            }
        }
        
        config.ai!!.services shouldHaveSize 2
        
        val codeAssistant = config.ai!!.services.find { it.name == "code-assistant" }
        codeAssistant shouldNotBe null
        codeAssistant!!.model shouldBe "codellama"
        codeAssistant.temperature shouldBe 0.3
        codeAssistant.maxTokens shouldBe 4096
        
        val chatBot = config.ai!!.services.find { it.name == "chat-bot" }
        chatBot shouldNotBe null
        chatBot!!.model shouldBe "llama3"
        chatBot.temperature shouldBe 0.7
    }
    
    "should configure hardware optimization" {
        val config = horizonOS {
            hostname = "gpu-system"
            timezone = "UTC"
            locale = "en_US.UTF-8"
            
            ai {
                enabled = true
                
                hardware {
                    gpuAcceleration = true
                    cpuThreads = 8
                    memoryLimit = "16GB"
                    optimization = HardwareOptimization.GPU_NVIDIA
                    backends(AIBackend.OLLAMA, AIBackend.LLAMACPP)
                }
            }
        }
        
        val hardware = config.ai!!.hardware
        hardware.gpuAcceleration.shouldBeTrue()
        hardware.cpuThreads shouldBe 8
        hardware.memoryLimit shouldBe "16GB"
        hardware.optimization shouldBe HardwareOptimization.GPU_NVIDIA
        hardware.backends shouldHaveSize 2
        hardware.backends shouldContain AIBackend.OLLAMA
        hardware.backends shouldContain AIBackend.LLAMACPP
    }
    
    "should configure privacy settings" {
        val config = horizonOS {
            hostname = "privacy-focused"
            timezone = "UTC"
            locale = "en_US.UTF-8"
            
            ai {
                enabled = true
                
                privacy {
                    localOnly = true
                    telemetryEnabled = false
                    dataRetention = DataRetention.SESSION_ONLY
                    encryptStorage = true
                    allowNetworkAccess("localhost", "127.0.0.1")
                }
            }
        }
        
        val privacy = config.ai!!.privacy
        privacy.localOnly.shouldBeTrue()
        privacy.telemetryEnabled.shouldBeFalse()
        privacy.dataRetention shouldBe DataRetention.SESSION_ONLY
        privacy.encryptStorage.shouldBeTrue()
        privacy.allowedNetworkAccess shouldHaveSize 2
        privacy.allowedNetworkAccess shouldContain "localhost"
        privacy.allowedNetworkAccess shouldContain "127.0.0.1"
    }
    
    "should handle multiple models with different configurations" {
        val config = horizonOS {
            hostname = "multi-model"
            timezone = "UTC"
            locale = "en_US.UTF-8"
            
            ai {
                enabled = true
                
                model("llama3-8b") {
                    provider = "ollama"
                    size = ModelSize.MEDIUM
                    quantization = ModelQuantization.Q4_0
                    capabilities(ModelCapability.TEXT_GENERATION, ModelCapability.REASONING)
                    preload = true
                }
                
                model("codellama-13b") {
                    provider = "ollama"
                    size = ModelSize.LARGE
                    quantization = ModelQuantization.Q8_0
                    capabilities(ModelCapability.CODE_GENERATION, ModelCapability.FUNCTION_CALLING)
                    preload = false
                }
                
                model("whisper") {
                    provider = "ollama"
                    size = ModelSize.SMALL
                    capabilities(ModelCapability.AUDIO_TRANSCRIPTION)
                    parameter("language", "auto")
                }
            }
        }
        
        config.ai!!.models shouldHaveSize 3
        
        val llama = config.ai!!.models.find { it.name == "llama3-8b" }
        llama shouldNotBe null
        llama!!.size shouldBe ModelSize.MEDIUM
        llama.preload.shouldBeTrue()
        llama.capabilities shouldContain ModelCapability.REASONING
        
        val codellama = config.ai!!.models.find { it.name == "codellama-13b" }
        codellama shouldNotBe null
        codellama!!.size shouldBe ModelSize.LARGE
        codellama.quantization shouldBe ModelQuantization.Q8_0
        codellama.preload.shouldBeFalse()
        
        val whisper = config.ai!!.models.find { it.name == "whisper" }
        whisper shouldNotBe null
        whisper!!.capabilities shouldContain ModelCapability.AUDIO_TRANSCRIPTION
        whisper.parameters["language"] shouldBe "auto"
    }
    
    "should handle multiple AI providers" {
        val config = horizonOS {
            hostname = "multi-provider"
            timezone = "UTC"
            locale = "en_US.UTF-8"
            
            ai {
                enabled = true
                
                provider("local-ollama", ProviderType.OLLAMA) {
                    endpoint = "localhost"
                    port = 11434
                    models("llama3", "codellama")
                }
                
                provider("remote-api", ProviderType.OPENAI_API) {
                    endpoint = "api.openai.com"
                    port = 443
                    apiKey = "sk-..."
                    models("gpt-4", "gpt-3.5-turbo")
                }
                
                provider("huggingface", ProviderType.HUGGINGFACE) {
                    endpoint = "api-inference.huggingface.co"
                    apiKey = "hf_..."
                    models("microsoft/DialoGPT-medium")
                }
            }
        }
        
        config.ai!!.providers shouldHaveSize 3
        
        val ollama = config.ai!!.providers.find { it.name == "local-ollama" }
        ollama shouldNotBe null
        ollama!!.type shouldBe ProviderType.OLLAMA
        ollama.port shouldBe 11434
        
        val openai = config.ai!!.providers.find { it.name == "remote-api" }
        openai shouldNotBe null
        openai!!.type shouldBe ProviderType.OPENAI_API
        openai.apiKey shouldBe "sk-..."
        
        val hf = config.ai!!.providers.find { it.name == "huggingface" }
        hf shouldNotBe null
        hf!!.type shouldBe ProviderType.HUGGINGFACE
    }
    
    "should use AI presets" {
        val config = horizonOS {
            hostname = "preset-test"
            timezone = "UTC"
            locale = "en_US.UTF-8"
            
            ai {
                enabled = true
                
                // Use preset configuration
                model("code-helper") {
                    val preset = AIPresets.codeAssistant()
                    size = preset.size
                    provider = "ollama"
                    capabilities(ModelCapability.CODE_GENERATION, ModelCapability.TEXT_GENERATION)
                    parameter("system_prompt", "You are a helpful coding assistant.")
                }
            }
        }
        
        val model = config.ai!!.models[0]
        model.name shouldBe "code-helper"
        model.size shouldBe ModelSize.MEDIUM
        model.capabilities shouldContain ModelCapability.CODE_GENERATION
        model.capabilities shouldContain ModelCapability.TEXT_GENERATION
        model.parameters["system_prompt"] shouldBe "You are a helpful coding assistant."
    }
    
    "should validate AI configuration completeness" {
        val config = horizonOS {
            hostname = "validation-test"
            timezone = "UTC"
            locale = "en_US.UTF-8"
            
            ai {
                enabled = true
                
                model("test-model") {
                    provider = "test-provider"
                    size = ModelSize.SMALL
                }
                
                provider("test-provider", ProviderType.OLLAMA) {
                    endpoint = "localhost"
                    port = 11434
                }
                
                service("test-service", "test-model") {
                    description = "Test AI service"
                }
            }
        }
        
        // Test extension functions
        config.hasAI().shouldBeTrue()
        config.getAIModel("test-model") shouldNotBe null
        config.getAIModel("nonexistent") shouldBe null
        config.getAIService("test-service") shouldNotBe null
        config.getAIService("nonexistent") shouldBe null
    }
    
    "should handle disabled AI configuration" {
        val config = horizonOS {
            hostname = "no-ai"
            timezone = "UTC"
            locale = "en_US.UTF-8"
            
            ai {
                enabled = false
            }
        }
        
        config.ai shouldNotBe null
        config.ai!!.enabled.shouldBeFalse()
        config.hasAI().shouldBeFalse()
    }
})