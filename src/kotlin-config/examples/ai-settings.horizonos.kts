#!/usr/bin/env kscript

/**
 * Example AI Settings Configuration for HorizonOS
 * 
 * This demonstrates the full capabilities of the AI configuration DSL
 */

import org.horizonos.config.dsl.ai.*

val config = aiSettings {
    // Basic AI configuration
    enabled = true
    ollamaEndpoint = "http://localhost:11434"
    defaultModel = "llama3.2:latest"
    
    // Hardware optimization settings
    hardware {
        optimization = HardwareOptimization.AUTO
        powerMode = PowerMode.BALANCED
        // Optionally limit GPU memory usage
        // gpuMemoryLimit = 8_000_000_000 // 8GB
        // cpuThreads = 8
    }
    
    // Privacy settings
    privacy {
        localOnly = true
        telemetryEnabled = false
        dataRetention = DataRetention.Days(30)
        encryptStorage = true
        sensitiveDataFilter = true
        
        // Consent management
        consent {
            requireExplicitConsent = true
            allowWithdrawal = true
            defaultState = ConsentState.NOT_GIVEN
            renewalPeriod = 365 // days
        }
        
        // Encryption settings
        encryption {
            algorithm = EncryptionAlgorithm.AES256_GCM
            keyManagement = KeyManagement.SYSTEM_KEYRING
            atRest = true
            inTransit = true
            keyRotationDays = 90
        }
        
        // Anonymization settings
        anonymization {
            enabled = true
            technique = AnonymizationTechnique.PSEUDONYMIZATION
            preserveUtility = true
            reversible = false
            
            // Custom anonymization rules
            rule("\\b\\d{3}-\\d{2}-\\d{4}\\b", "XXX-XX-XXXX") // SSN
            rule("\\b\\d{16}\\b", "XXXX-XXXX-XXXX-XXXX") // Credit card
        }
        
        // Audit settings
        audit {
            enabled = true
            level = AuditLevel.STANDARD
            retentionDays = 90
            logDataAccess = true
            logConsentChanges = true
            tamperProtection = true
        }
    }
    
    // Learning system settings
    learning {
        enabled = true
        applications = true
        documents = true
        websites = true
        workflows = true
        minConfidence = 0.7f
        minOccurrences = 5
        
        // Exclude sensitive applications
        excludeApp("1password")
        excludeApp("keepassxc")
        excludeApp("bitwarden")
        
        // Exclude private paths
        excludePath("~/private")
        excludePath("~/secure")
        excludePath("~/Documents/confidential")
        
        // Exclude sensitive domains
        excludeDomain("*.bank.com")
        excludeDomain("*.health.gov")
        excludeDomain("*.medical.org")
    }
    
    // Suggestion system settings
    suggestions {
        enabled = true
        displayMode = DisplayMode.TOAST
        maxPerHour = 3
        quietHours = "22:00" to "08:00"
        appLaunch = true
        documentOpen = true
        websiteVisit = true
        workflowAutomation = true
    }
    
    // Agent framework settings
    agents {
        enabled = true
        maxConcurrentAgents = 5
        defaultTimeout = 300_000 // 5 minutes
        retryAttempts = 3
        
        // Configure individual agents
        agent("code-assistant") {
            enabled = true
            model = "codellama:latest"
            temperature = 0.3f
            maxTokens = 8192
            capability("code-generation")
            capability("code-review")
            capability("refactoring")
        }
        
        agent("research-assistant") {
            enabled = true
            temperature = 0.7f
            maxTokens = 4096
            capability("web-search")
            capability("summarization")
            capability("fact-checking")
        }
        
        agent("task-planner") {
            enabled = true
            temperature = 0.5f
            maxTokens = 4096
            capability("task-decomposition")
            capability("scheduling")
            capability("priority-management")
        }
        
        agent("automation-assistant") {
            enabled = true
            temperature = 0.4f
            maxTokens = 4096
            capability("workflow-creation")
            capability("process-optimization")
            capability("ui-automation")
        }
    }
    
    // Automation settings
    automation {
        enabled = true
        n8nEndpoint = "http://localhost:5678"
        temporalEndpoint = "localhost:7233"
        maxWorkflows = 100
        enableBrowserAutomation = true
        enableUIAutomation = true
    }
}

// Print the configuration for verification
println("AI Settings Configuration:")
println("=========================")
println(config.toConfig())

// Example: Check specific settings
println("\nPrivacy local-only mode: ${config.toConfig()["privacy"] as Map<*, *>)["local_only"]}")
println("Learning confidence threshold: ${(config.toConfig()["learning"] as Map<*, *>)["min_confidence"]}")