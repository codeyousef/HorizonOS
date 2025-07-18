package org.horizonos.config.dsl.ai

import org.horizonos.config.dsl.core.ConfigElement
import kotlin.annotation.AnnotationTarget.*

/**
 * DSL for AI system configuration
 */
@DslMarker
annotation class AISettingsDsl

/**
 * Root AI settings configuration
 */
@AISettingsDsl
class AISettings : ConfigElement {
    var enabled: Boolean = true
    var ollamaEndpoint: String = "http://localhost:11434"
    var defaultModel: String = "llama3.2:latest"
    
    private var _hardware: HardwareSettings? = null
    private var _privacy: PrivacySettings? = null
    private var _learning: LearningSettings? = null
    private var _suggestions: SuggestionSettings? = null
    private var _agents: AgentSettings? = null
    private var _automation: AutomationSettings? = null
    
    /**
     * Hardware optimization settings
     */
    fun hardware(block: HardwareSettings.() -> Unit) {
        _hardware = HardwareSettings().apply(block)
    }
    
    /**
     * Privacy settings
     */
    fun privacy(block: PrivacySettings.() -> Unit) {
        _privacy = PrivacySettings().apply(block)
    }
    
    /**
     * Learning system settings
     */
    fun learning(block: LearningSettings.() -> Unit) {
        _learning = LearningSettings().apply(block)
    }
    
    /**
     * Suggestion system settings
     */
    fun suggestions(block: SuggestionSettings.() -> Unit) {
        _suggestions = SuggestionSettings().apply(block)
    }
    
    /**
     * Agent framework settings
     */
    fun agents(block: AgentSettings.() -> Unit) {
        _agents = AgentSettings().apply(block)
    }
    
    /**
     * Automation settings
     */
    fun automation(block: AutomationSettings.() -> Unit) {
        _automation = AutomationSettings().apply(block)
    }
    
    override fun validate() {
        require(ollamaEndpoint.isNotBlank()) { "Ollama endpoint must not be blank" }
        require(defaultModel.isNotBlank()) { "Default model must not be blank" }
        
        _hardware?.validate()
        _privacy?.validate()
        _learning?.validate()
        _suggestions?.validate()
        _agents?.validate()
        _automation?.validate()
    }
    
    override fun toConfig(): Map<String, Any> {
        return mapOf(
            "enabled" to enabled,
            "ollama_endpoint" to ollamaEndpoint,
            "default_model" to defaultModel,
            "hardware" to (_hardware?.toConfig() ?: emptyMap<String, Any>()),
            "privacy" to (_privacy?.toConfig() ?: emptyMap<String, Any>()),
            "learning" to (_learning?.toConfig() ?: emptyMap<String, Any>()),
            "suggestions" to (_suggestions?.toConfig() ?: emptyMap<String, Any>()),
            "agents" to (_agents?.toConfig() ?: emptyMap<String, Any>()),
            "automation" to (_automation?.toConfig() ?: emptyMap<String, Any>())
        )
    }
}

/**
 * Hardware optimization settings
 */
@AISettingsDsl
class HardwareSettings : ConfigElement {
    var optimization: HardwareOptimization = HardwareOptimization.AUTO
    var gpuMemoryLimit: Long? = null
    var cpuThreads: Int? = null
    var powerMode: PowerMode = PowerMode.BALANCED
    
    override fun validate() {
        gpuMemoryLimit?.let { require(it > 0) { "GPU memory limit must be positive" } }
        cpuThreads?.let { require(it > 0) { "CPU threads must be positive" } }
    }
    
    override fun toConfig(): Map<String, Any> {
        val config = mutableMapOf<String, Any>(
            "optimization" to optimization.name,
            "power_mode" to powerMode.name
        )
        gpuMemoryLimit?.let { config["gpu_memory_limit"] = it }
        cpuThreads?.let { config["cpu_threads"] = it }
        return config
    }
}

/**
 * Privacy settings
 */
@AISettingsDsl
class PrivacySettings : ConfigElement {
    var localOnly: Boolean = true
    var telemetryEnabled: Boolean = false
    var dataRetention: DataRetention = DataRetention.Days(30)
    var encryptStorage: Boolean = true
    var sensitiveDataFilter: Boolean = true
    
    private var _consent: ConsentSettings? = null
    private var _encryption: EncryptionSettings? = null
    private var _anonymization: AnonymizationSettings? = null
    private var _audit: AuditSettings? = null
    
    fun consent(block: ConsentSettings.() -> Unit) {
        _consent = ConsentSettings().apply(block)
    }
    
    fun encryption(block: EncryptionSettings.() -> Unit) {
        _encryption = EncryptionSettings().apply(block)
    }
    
    fun anonymization(block: AnonymizationSettings.() -> Unit) {
        _anonymization = AnonymizationSettings().apply(block)
    }
    
    fun audit(block: AuditSettings.() -> Unit) {
        _audit = AuditSettings().apply(block)
    }
    
    override fun validate() {
        _consent?.validate()
        _encryption?.validate()
        _anonymization?.validate()
        _audit?.validate()
    }
    
    override fun toConfig(): Map<String, Any> {
        return mapOf(
            "local_only" to localOnly,
            "telemetry_enabled" to telemetryEnabled,
            "data_retention" to dataRetention.toConfig(),
            "encrypt_storage" to encryptStorage,
            "sensitive_data_filter" to sensitiveDataFilter,
            "consent" to (_consent?.toConfig() ?: emptyMap<String, Any>()),
            "encryption" to (_encryption?.toConfig() ?: emptyMap<String, Any>()),
            "anonymization" to (_anonymization?.toConfig() ?: emptyMap<String, Any>()),
            "audit" to (_audit?.toConfig() ?: emptyMap<String, Any>())
        )
    }
}

/**
 * Learning system settings
 */
@AISettingsDsl
class LearningSettings : ConfigElement {
    var enabled: Boolean = true
    var applications: Boolean = true
    var documents: Boolean = true
    var websites: Boolean = true
    var workflows: Boolean = true
    var minConfidence: Float = 0.7f
    var minOccurrences: Int = 5
    
    private val excludedApps = mutableListOf<String>()
    private val excludedPaths = mutableListOf<String>()
    private val excludedDomains = mutableListOf<String>()
    
    fun excludeApp(app: String) {
        excludedApps.add(app)
    }
    
    fun excludePath(path: String) {
        excludedPaths.add(path)
    }
    
    fun excludeDomain(domain: String) {
        excludedDomains.add(domain)
    }
    
    override fun validate() {
        require(minConfidence in 0.0f..1.0f) { "Min confidence must be between 0 and 1" }
        require(minOccurrences > 0) { "Min occurrences must be positive" }
    }
    
    override fun toConfig(): Map<String, Any> {
        return mapOf(
            "enabled" to enabled,
            "applications" to applications,
            "documents" to documents,
            "websites" to websites,
            "workflows" to workflows,
            "min_confidence" to minConfidence,
            "min_occurrences" to minOccurrences,
            "excluded_apps" to excludedApps,
            "excluded_paths" to excludedPaths,
            "excluded_domains" to excludedDomains
        )
    }
}

/**
 * Suggestion system settings
 */
@AISettingsDsl
class SuggestionSettings : ConfigElement {
    var enabled: Boolean = true
    var displayMode: DisplayMode = DisplayMode.TOAST
    var maxPerHour: Int = 3
    var quietHours: Pair<String, String>? = "22:00" to "08:00"
    var appLaunch: Boolean = true
    var documentOpen: Boolean = true
    var websiteVisit: Boolean = true
    var workflowAutomation: Boolean = true
    
    override fun validate() {
        require(maxPerHour >= 0) { "Max per hour must be non-negative" }
        quietHours?.let { (start, end) ->
            // Validate time format
            val timeRegex = Regex("^([01]?[0-9]|2[0-3]):[0-5][0-9]$")
            require(start.matches(timeRegex)) { "Invalid start time format: $start" }
            require(end.matches(timeRegex)) { "Invalid end time format: $end" }
        }
    }
    
    override fun toConfig(): Map<String, Any> {
        val config = mutableMapOf<String, Any>(
            "enabled" to enabled,
            "display_mode" to displayMode.name,
            "max_per_hour" to maxPerHour,
            "app_launch" to appLaunch,
            "document_open" to documentOpen,
            "website_visit" to websiteVisit,
            "workflow_automation" to workflowAutomation
        )
        quietHours?.let { (start, end) ->
            config["quiet_hours"] = mapOf("start" to start, "end" to end)
        }
        return config
    }
}

/**
 * Agent framework settings
 */
@AISettingsDsl
class AgentSettings : ConfigElement {
    var enabled: Boolean = true
    var maxConcurrentAgents: Int = 5
    var defaultTimeout: Long = 300000 // 5 minutes
    var retryAttempts: Int = 3
    
    private val agents = mutableListOf<AgentConfig>()
    
    fun agent(name: String, block: AgentConfig.() -> Unit) {
        agents.add(AgentConfig(name).apply(block))
    }
    
    override fun validate() {
        require(maxConcurrentAgents > 0) { "Max concurrent agents must be positive" }
        require(defaultTimeout > 0) { "Default timeout must be positive" }
        require(retryAttempts >= 0) { "Retry attempts must be non-negative" }
        agents.forEach { it.validate() }
    }
    
    override fun toConfig(): Map<String, Any> {
        return mapOf(
            "enabled" to enabled,
            "max_concurrent_agents" to maxConcurrentAgents,
            "default_timeout" to defaultTimeout,
            "retry_attempts" to retryAttempts,
            "agents" to agents.map { it.toConfig() }
        )
    }
}

/**
 * Individual agent configuration
 */
@AISettingsDsl
class AgentConfig(val name: String) : ConfigElement {
    var enabled: Boolean = true
    var model: String? = null
    var temperature: Float = 0.7f
    var maxTokens: Int = 4096
    
    private val capabilities = mutableListOf<String>()
    
    fun capability(name: String) {
        capabilities.add(name)
    }
    
    override fun validate() {
        require(name.isNotBlank()) { "Agent name must not be blank" }
        require(temperature in 0.0f..2.0f) { "Temperature must be between 0 and 2" }
        require(maxTokens > 0) { "Max tokens must be positive" }
    }
    
    override fun toConfig(): Map<String, Any> {
        val config = mutableMapOf<String, Any>(
            "name" to name,
            "enabled" to enabled,
            "temperature" to temperature,
            "max_tokens" to maxTokens,
            "capabilities" to capabilities
        )
        model?.let { config["model"] = it }
        return config
    }
}

/**
 * Automation settings
 */
@AISettingsDsl
class AutomationSettings : ConfigElement {
    var enabled: Boolean = true
    var n8nEndpoint: String = "http://localhost:5678"
    var temporalEndpoint: String = "localhost:7233"
    var maxWorkflows: Int = 100
    var enableBrowserAutomation: Boolean = true
    var enableUIAutomation: Boolean = true
    
    override fun validate() {
        require(n8nEndpoint.isNotBlank()) { "n8n endpoint must not be blank" }
        require(temporalEndpoint.isNotBlank()) { "Temporal endpoint must not be blank" }
        require(maxWorkflows > 0) { "Max workflows must be positive" }
    }
    
    override fun toConfig(): Map<String, Any> {
        return mapOf(
            "enabled" to enabled,
            "n8n_endpoint" to n8nEndpoint,
            "temporal_endpoint" to temporalEndpoint,
            "max_workflows" to maxWorkflows,
            "enable_browser_automation" to enableBrowserAutomation,
            "enable_ui_automation" to enableUIAutomation
        )
    }
}

/**
 * Consent settings
 */
@AISettingsDsl
class ConsentSettings : ConfigElement {
    var requireExplicitConsent: Boolean = true
    var allowWithdrawal: Boolean = true
    var defaultState: ConsentState = ConsentState.NOT_GIVEN
    var renewalPeriod: Int? = 365 // days
    
    override fun validate() {
        renewalPeriod?.let { require(it > 0) { "Renewal period must be positive" } }
    }
    
    override fun toConfig(): Map<String, Any> {
        val config = mutableMapOf<String, Any>(
            "require_explicit_consent" to requireExplicitConsent,
            "allow_withdrawal" to allowWithdrawal,
            "default_state" to defaultState.name
        )
        renewalPeriod?.let { config["renewal_period"] = it }
        return config
    }
}

/**
 * Encryption settings
 */
@AISettingsDsl
class EncryptionSettings : ConfigElement {
    var algorithm: EncryptionAlgorithm = EncryptionAlgorithm.AES256_GCM
    var keyManagement: KeyManagement = KeyManagement.SYSTEM_KEYRING
    var atRest: Boolean = true
    var inTransit: Boolean = true
    var keyRotationDays: Int = 90
    
    override fun validate() {
        require(keyRotationDays > 0) { "Key rotation days must be positive" }
    }
    
    override fun toConfig(): Map<String, Any> {
        return mapOf(
            "algorithm" to algorithm.name,
            "key_management" to keyManagement.name,
            "at_rest" to atRest,
            "in_transit" to inTransit,
            "key_rotation_days" to keyRotationDays
        )
    }
}

/**
 * Anonymization settings
 */
@AISettingsDsl
class AnonymizationSettings : ConfigElement {
    var enabled: Boolean = true
    var technique: AnonymizationTechnique = AnonymizationTechnique.PSEUDONYMIZATION
    var preserveUtility: Boolean = true
    var reversible: Boolean = false
    
    private val customRules = mutableMapOf<String, String>()
    
    fun rule(pattern: String, replacement: String) {
        customRules[pattern] = replacement
    }
    
    override fun validate() {
        // No specific validation needed
    }
    
    override fun toConfig(): Map<String, Any> {
        return mapOf(
            "enabled" to enabled,
            "technique" to technique.name,
            "preserve_utility" to preserveUtility,
            "reversible" to reversible,
            "custom_rules" to customRules
        )
    }
}

/**
 * Audit settings
 */
@AISettingsDsl
class AuditSettings : ConfigElement {
    var enabled: Boolean = true
    var level: AuditLevel = AuditLevel.STANDARD
    var retentionDays: Int = 90
    var logDataAccess: Boolean = true
    var logConsentChanges: Boolean = true
    var tamperProtection: Boolean = true
    
    override fun validate() {
        require(retentionDays > 0) { "Retention days must be positive" }
    }
    
    override fun toConfig(): Map<String, Any> {
        return mapOf(
            "enabled" to enabled,
            "level" to level.name,
            "retention_days" to retentionDays,
            "log_data_access" to logDataAccess,
            "log_consent_changes" to logConsentChanges,
            "tamper_protection" to tamperProtection
        )
    }
}

// Enums and data classes

enum class HardwareOptimization {
    AUTO, PREFER_GPU, CPU_ONLY, POWER_SAVING
}

enum class PowerMode {
    PERFORMANCE, BALANCED, POWER_SAVING
}

sealed class DataRetention {
    object SessionOnly : DataRetention()
    data class Days(val days: Int) : DataRetention()
    object Forever : DataRetention()
    
    fun toConfig(): Map<String, Any> {
        return when (this) {
            is SessionOnly -> mapOf("type" to "session_only")
            is Days -> mapOf("type" to "days", "days" to days)
            is Forever -> mapOf("type" to "forever")
        }
    }
}

enum class DisplayMode {
    TOAST, BUBBLE, SIDEBAR, SYSTRAY, NONE
}

enum class ConsentState {
    GIVEN, NOT_GIVEN, WITHDRAWN, EXPIRED
}

enum class EncryptionAlgorithm {
    AES256_GCM, CHACHA20_POLY1305, XCHACHA20_POLY1305
}

enum class KeyManagement {
    SYSTEM_KEYRING, FILE_STORAGE, MEMORY_ONLY, HSM
}

enum class AnonymizationTechnique {
    PSEUDONYMIZATION, K_ANONYMITY, NOISE_ADDITION, GENERALIZATION
}

enum class AuditLevel {
    MINIMAL, STANDARD, DETAILED, FORENSIC
}

// DSL builder function
fun aiSettings(block: AISettings.() -> Unit): AISettings {
    return AISettings().apply(block).also { it.validate() }
}