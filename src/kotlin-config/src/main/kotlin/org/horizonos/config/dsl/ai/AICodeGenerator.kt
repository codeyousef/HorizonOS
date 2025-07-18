package org.horizonos.config.dsl.ai

import org.horizonos.config.dsl.core.CodeGenerator
import java.io.File

/**
 * Code generator for AI settings
 * 
 * Generates Rust configuration code from the AI settings DSL
 */
class AICodeGenerator : CodeGenerator {
    
    /**
     * Generate code from configuration
     */
    override fun generate(config: Map<String, Any>): String {
        return generateRustCode(config)
    }
    
    /**
     * Generate Rust configuration file from AI settings
     */
    fun generateRustConfig(settings: AISettings, outputPath: String): File {
        val config = settings.toConfig()
        val rustCode = generateRustCode(config)
        
        val file = File(outputPath)
        file.parentFile.mkdirs()
        file.writeText(rustCode)
        
        return file
    }
    
    /**
     * Generate Rust code from configuration map
     */
    private fun generateRustCode(config: Map<String, Any>): String {
        return buildString {
            appendLine("//! Auto-generated AI configuration")
            appendLine("//! Generated from HorizonOS Kotlin DSL")
            appendLine()
            appendLine("use crate::ai::{AIConfig, HardwareOptimization, PrivacyConfig, LearningConfig, SuggestionConfig};")
            appendLine("use crate::ai::{ConsentConfig, EncryptionConfig, AnonymizationConfig, AuditConfig};")
            appendLine("use crate::ai::{AgentConfig, AutomationConfig, DisplayMode, DataRetention};")
            appendLine("use std::collections::HashMap;")
            appendLine()
            appendLine("/// Create AI configuration")
            appendLine("pub fn create_ai_config() -> AIConfig {")
            appendLine("    AIConfig {")
            
            // Basic settings
            appendLine("        enabled: ${config["enabled"]},")
            appendLine("        ollama_endpoint: \"${config["ollama_endpoint"]}\".to_string(),")
            appendLine("        default_model: \"${config["default_model"]}\".to_string(),")
            
            // Hardware optimization
            val hardware = config["hardware"] as? Map<*, *>
            if (hardware != null) {
                appendLine("        hardware_optimization: ${generateHardwareOptimization(hardware)},")
            }
            
            // Privacy configuration
            val privacy = config["privacy"] as? Map<*, *>
            if (privacy != null) {
                appendLine("        privacy: ${generatePrivacyConfig(privacy)},")
            }
            
            // Learning configuration
            val learning = config["learning"] as? Map<*, *>
            if (learning != null) {
                appendLine("        learning: ${generateLearningConfig(learning)},")
            }
            
            // Suggestions configuration
            val suggestions = config["suggestions"] as? Map<*, *>
            if (suggestions != null) {
                appendLine("        suggestions: ${generateSuggestionConfig(suggestions)},")
            }
            
            appendLine("    }")
            appendLine("}")
            
            // Generate agent configurations
            val agents = config["agents"] as? Map<*, *>
            if (agents != null) {
                appendLine()
                generateAgentConfigs(agents)
            }
            
            // Generate automation configuration
            val automation = config["automation"] as? Map<*, *>
            if (automation != null) {
                appendLine()
                generateAutomationConfig(automation)
            }
        }
    }
    
    /**
     * Generate hardware optimization enum
     */
    private fun generateHardwareOptimization(hardware: Map<*, *>): String {
        return when (hardware["optimization"]) {
            "AUTO" -> "HardwareOptimization::Auto"
            "PREFER_GPU" -> "HardwareOptimization::PreferGPU"
            "CPU_ONLY" -> "HardwareOptimization::CPUOnly"
            "POWER_SAVING" -> "HardwareOptimization::PowerSaving"
            else -> "HardwareOptimization::Auto"
        }
    }
    
    /**
     * Generate privacy configuration
     */
    private fun generatePrivacyConfig(privacy: Map<*, *>): String {
        return buildString {
            appendLine("PrivacyConfig {")
            appendLine("            local_only: ${privacy["local_only"]},")
            appendLine("            telemetry_enabled: ${privacy["telemetry_enabled"]},")
            appendLine("            data_retention: ${generateDataRetention(privacy["data_retention"] as? Map<*, *>)},")
            appendLine("            encrypt_storage: ${privacy["encrypt_storage"]},")
            appendLine("            sensitive_data_filter: ${privacy["sensitive_data_filter"]},")
            
            // Nested privacy configurations
            val consent = privacy["consent"] as? Map<*, *>
            if (consent != null) {
                appendLine("            consent: Some(${generateConsentConfig(consent)}),")
            }
            
            val encryption = privacy["encryption"] as? Map<*, *>
            if (encryption != null) {
                appendLine("            encryption: Some(${generateEncryptionConfig(encryption)}),")
            }
            
            val anonymization = privacy["anonymization"] as? Map<*, *>
            if (anonymization != null) {
                appendLine("            anonymization: Some(${generateAnonymizationConfig(anonymization)}),")
            }
            
            val audit = privacy["audit"] as? Map<*, *>
            if (audit != null) {
                appendLine("            audit: Some(${generateAuditConfig(audit)}),")
            }
            
            append("        }")
        }
    }
    
    /**
     * Generate data retention enum
     */
    private fun generateDataRetention(retention: Map<*, *>?): String {
        return when (retention?.get("type")) {
            "session_only" -> "DataRetention::SessionOnly"
            "days" -> "DataRetention::Days(${retention["days"]})"
            "forever" -> "DataRetention::Forever"
            else -> "DataRetention::Days(30)"
        }
    }
    
    /**
     * Generate consent configuration
     */
    private fun generateConsentConfig(consent: Map<*, *>): String {
        return buildString {
            appendLine("ConsentConfig {")
            appendLine("                require_explicit_consent: ${consent["require_explicit_consent"]},")
            appendLine("                allow_withdrawal: ${consent["allow_withdrawal"]},")
            appendLine("                default_state: ConsentState::${(consent["default_state"] as? String)?.replace("_", "")},")
            consent["renewal_period"]?.let { 
                appendLine("                renewal_period: Some($it),")
            }
            append("            }")
        }
    }
    
    /**
     * Generate encryption configuration
     */
    private fun generateEncryptionConfig(encryption: Map<*, *>): String {
        return buildString {
            appendLine("EncryptionConfig {")
            appendLine("                algorithm: EncryptionAlgorithm::${(encryption["algorithm"] as? String)?.replace("_", "")},")
            appendLine("                key_management: KeyManagement::${(encryption["key_management"] as? String)?.replace("_", "")},")
            appendLine("                at_rest: ${encryption["at_rest"]},")
            appendLine("                in_transit: ${encryption["in_transit"]},")
            appendLine("                key_rotation_days: ${encryption["key_rotation_days"]},")
            append("            }")
        }
    }
    
    /**
     * Generate anonymization configuration
     */
    private fun generateAnonymizationConfig(anonymization: Map<*, *>): String {
        return buildString {
            appendLine("AnonymizationConfig {")
            appendLine("                enabled: ${anonymization["enabled"]},")
            appendLine("                technique: AnonymizationTechnique::${anonymization["technique"]},")
            appendLine("                preserve_utility: ${anonymization["preserve_utility"]},")
            appendLine("                reversible: ${anonymization["reversible"]},")
            
            val customRules = anonymization["custom_rules"] as? Map<*, *>
            if (!customRules.isNullOrEmpty()) {
                appendLine("                custom_rules: {")
                appendLine("                    let mut rules = HashMap::new();")
                customRules.forEach { (pattern, replacement) ->
                    appendLine("                    rules.insert(\"$pattern\".to_string(), \"$replacement\".to_string());")
                }
                appendLine("                    rules")
                appendLine("                },")
            }
            
            append("            }")
        }
    }
    
    /**
     * Generate audit configuration
     */
    private fun generateAuditConfig(audit: Map<*, *>): String {
        return buildString {
            appendLine("AuditConfig {")
            appendLine("                enabled: ${audit["enabled"]},")
            appendLine("                level: AuditLevel::${audit["level"]},")
            appendLine("                retention_days: ${audit["retention_days"]},")
            appendLine("                log_data_access: ${audit["log_data_access"]},")
            appendLine("                log_consent_changes: ${audit["log_consent_changes"]},")
            appendLine("                tamper_protection: ${audit["tamper_protection"]},")
            append("            }")
        }
    }
    
    /**
     * Generate learning configuration
     */
    private fun generateLearningConfig(learning: Map<*, *>): String {
        return buildString {
            appendLine("LearningConfig {")
            appendLine("            enabled: ${learning["enabled"]},")
            appendLine("            applications: ${learning["applications"]},")
            appendLine("            documents: ${learning["documents"]},")
            appendLine("            websites: ${learning["websites"]},")
            appendLine("            workflows: ${learning["workflows"]},")
            appendLine("            min_confidence: ${learning["min_confidence"]}f32,")
            appendLine("            min_occurrences: ${learning["min_occurrences"]},")
            
            val excludedApps = learning["excluded_apps"] as? List<*>
            if (!excludedApps.isNullOrEmpty()) {
                appendLine("            excluded_apps: vec![${excludedApps.joinToString(", ") { "\"$it\".to_string()" }}],")
            }
            
            val excludedPaths = learning["excluded_paths"] as? List<*>
            if (!excludedPaths.isNullOrEmpty()) {
                appendLine("            excluded_paths: vec![${excludedPaths.joinToString(", ") { "\"$it\".to_string()" }}],")
            }
            
            val excludedDomains = learning["excluded_domains"] as? List<*>
            if (!excludedDomains.isNullOrEmpty()) {
                appendLine("            excluded_domains: vec![${excludedDomains.joinToString(", ") { "\"$it\".to_string()" }}],")
            }
            
            append("        }")
        }
    }
    
    /**
     * Generate suggestion configuration
     */
    private fun generateSuggestionConfig(suggestions: Map<*, *>): String {
        return buildString {
            appendLine("SuggestionConfig {")
            appendLine("            enabled: ${suggestions["enabled"]},")
            appendLine("            display_mode: DisplayMode::${suggestions["display_mode"]},")
            appendLine("            max_per_hour: ${suggestions["max_per_hour"]},")
            
            val quietHours = suggestions["quiet_hours"] as? Map<*, *>
            if (quietHours != null) {
                appendLine("            quiet_hours: Some((\"${quietHours["start"]}\".to_string(), \"${quietHours["end"]}\".to_string())),")
            }
            
            appendLine("            app_launch: ${suggestions["app_launch"]},")
            appendLine("            document_open: ${suggestions["document_open"]},")
            appendLine("            website_visit: ${suggestions["website_visit"]},")
            appendLine("            workflow_automation: ${suggestions["workflow_automation"]},")
            append("        }")
        }
    }
    
    /**
     * Generate agent configurations
     */
    private fun generateAgentConfigs(agents: Map<*, *>): String {
        return buildString {
            appendLine("/// Create agent configurations")
            appendLine("pub fn create_agent_configs() -> Vec<AgentConfig> {")
            appendLine("    vec![")
            
            val agentList = agents["agents"] as? List<*>
            agentList?.forEach { agent ->
                if (agent is Map<*, *>) {
                    appendLine("        AgentConfig {")
                    appendLine("            name: \"${agent["name"]}\".to_string(),")
                    appendLine("            enabled: ${agent["enabled"]},")
                    agent["model"]?.let {
                        appendLine("            model: Some(\"$it\".to_string()),")
                    }
                    appendLine("            temperature: ${agent["temperature"]}f32,")
                    appendLine("            max_tokens: ${agent["max_tokens"]},")
                    
                    val capabilities = agent["capabilities"] as? List<*>
                    if (!capabilities.isNullOrEmpty()) {
                        appendLine("            capabilities: vec![${capabilities.joinToString(", ") { "\"$it\".to_string()" }}],")
                    }
                    
                    appendLine("        },")
                }
            }
            
            appendLine("    ]")
            append("}")
        }
    }
    
    /**
     * Generate automation configuration
     */
    private fun generateAutomationConfig(automation: Map<*, *>): String {
        return buildString {
            appendLine("/// Create automation configuration")
            appendLine("pub fn create_automation_config() -> AutomationConfig {")
            appendLine("    AutomationConfig {")
            appendLine("        enabled: ${automation["enabled"]},")
            appendLine("        n8n_endpoint: \"${automation["n8n_endpoint"]}\".to_string(),")
            appendLine("        temporal_endpoint: \"${automation["temporal_endpoint"]}\".to_string(),")
            appendLine("        max_workflows: ${automation["max_workflows"]},")
            appendLine("        enable_browser_automation: ${automation["enable_browser_automation"]},")
            appendLine("        enable_ui_automation: ${automation["enable_ui_automation"]},")
            appendLine("    }")
            append("}")
        }
    }
    
    /**
     * Generate TOML configuration file
     */
    fun generateTomlConfig(settings: AISettings, outputPath: String): File {
        val config = settings.toConfig()
        val tomlContent = generateToml(config)
        
        val file = File(outputPath)
        file.parentFile.mkdirs()
        file.writeText(tomlContent)
        
        return file
    }
    
    /**
     * Generate TOML content from configuration
     */
    private fun generateToml(config: Map<String, Any>): String {
        return buildString {
            appendLine("# HorizonOS AI Configuration")
            appendLine("# Auto-generated from Kotlin DSL")
            appendLine()
            appendLine("[ai]")
            appendLine("enabled = ${config["enabled"]}")
            appendLine("ollama_endpoint = \"${config["ollama_endpoint"]}\"")
            appendLine("default_model = \"${config["default_model"]}\"")
            appendLine()
            
            // Hardware section
            val hardware = config["hardware"] as? Map<*, *>
            if (hardware != null) {
                appendLine("[ai.hardware]")
                appendLine("optimization = \"${hardware["optimization"]}\"")
                appendLine("power_mode = \"${hardware["power_mode"]}\"")
                hardware["gpu_memory_limit"]?.let {
                    appendLine("gpu_memory_limit = $it")
                }
                hardware["cpu_threads"]?.let {
                    appendLine("cpu_threads = $it")
                }
                appendLine()
            }
            
            // Privacy section
            val privacy = config["privacy"] as? Map<*, *>
            if (privacy != null) {
                appendLine("[ai.privacy]")
                appendLine("local_only = ${privacy["local_only"]}")
                appendLine("telemetry_enabled = ${privacy["telemetry_enabled"]}")
                appendLine("encrypt_storage = ${privacy["encrypt_storage"]}")
                appendLine("sensitive_data_filter = ${privacy["sensitive_data_filter"]}")
                
                val dataRetention = privacy["data_retention"] as? Map<*, *>
                if (dataRetention != null) {
                    appendLine()
                    appendLine("[ai.privacy.data_retention]")
                    appendLine("type = \"${dataRetention["type"]}\"")
                    dataRetention["days"]?.let {
                        appendLine("days = $it")
                    }
                }
                appendLine()
            }
            
            // Learning section
            val learning = config["learning"] as? Map<*, *>
            if (learning != null) {
                appendLine("[ai.learning]")
                appendLine("enabled = ${learning["enabled"]}")
                appendLine("applications = ${learning["applications"]}")
                appendLine("documents = ${learning["documents"]}")
                appendLine("websites = ${learning["websites"]}")
                appendLine("workflows = ${learning["workflows"]}")
                appendLine("min_confidence = ${learning["min_confidence"]}")
                appendLine("min_occurrences = ${learning["min_occurrences"]}")
                
                val excludedApps = learning["excluded_apps"] as? List<*>
                if (!excludedApps.isNullOrEmpty()) {
                    appendLine("excluded_apps = [${excludedApps.joinToString(", ") { "\"$it\"" }}]")
                }
                
                val excludedPaths = learning["excluded_paths"] as? List<*>
                if (!excludedPaths.isNullOrEmpty()) {
                    appendLine("excluded_paths = [${excludedPaths.joinToString(", ") { "\"$it\"" }}]")
                }
                
                val excludedDomains = learning["excluded_domains"] as? List<*>
                if (!excludedDomains.isNullOrEmpty()) {
                    appendLine("excluded_domains = [${excludedDomains.joinToString(", ") { "\"$it\"" }}]")
                }
                appendLine()
            }
            
            // Agents section
            val agents = config["agents"] as? Map<*, *>
            if (agents != null) {
                val agentList = agents["agents"] as? List<*>
                agentList?.forEach { agent ->
                    if (agent is Map<*, *>) {
                        appendLine("[[ai.agents]]")
                        appendLine("name = \"${agent["name"]}\"")
                        appendLine("enabled = ${agent["enabled"]}")
                        agent["model"]?.let {
                            appendLine("model = \"$it\"")
                        }
                        appendLine("temperature = ${agent["temperature"]}")
                        appendLine("max_tokens = ${agent["max_tokens"]}")
                        
                        val capabilities = agent["capabilities"] as? List<*>
                        if (!capabilities.isNullOrEmpty()) {
                            appendLine("capabilities = [${capabilities.joinToString(", ") { "\"$it\"" }}]")
                        }
                        appendLine()
                    }
                }
            }
        }
    }
}