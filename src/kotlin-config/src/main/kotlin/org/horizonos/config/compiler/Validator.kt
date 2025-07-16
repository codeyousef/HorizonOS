package org.horizonos.config.compiler

import org.horizonos.config.dsl.*
import org.horizonos.config.validation.ConfigurationValidator
import org.horizonos.config.validation.ValidationError
import org.horizonos.config.validation.ValidationResult

/**
 * Enhanced validator for HorizonOS configurations
 * Performs semantic validation beyond basic syntax checking
 */
class EnhancedConfigValidator {
    
    private val basicValidator = ConfigurationValidator
    
    /**
     * Validate a compiled configuration with enhanced checks
     */
    fun validate(config: CompiledConfig): EnhancedValidationResult {
        val errors = mutableListOf<ConfigValidationError>()
        val warnings = mutableListOf<ConfigValidationWarning>()
        
        // Run basic validation first
        val basicResult = basicValidator.validate(config)
        if (basicResult.isInvalid) {
            errors.addAll(basicResult.errors.map { ConfigValidationError.BasicValidationError(it) })
        }
        
        // Enhanced validation checks
        performEnhancedValidation(config, errors, warnings)
        
        return EnhancedValidationResult(
            errors = errors,
            warnings = warnings,
            isValid = errors.isEmpty()
        )
    }
    
    private fun performEnhancedValidation(
        config: CompiledConfig,
        errors: MutableList<ConfigValidationError>,
        warnings: MutableList<ConfigValidationWarning>
    ) {
        // Check for security issues
        validateSecurity(config, errors, warnings)
        
        // Check for performance issues
        validatePerformance(config, errors, warnings)
        
        // Check for compatibility issues
        validateCompatibility(config, errors, warnings)
        
        // Check for best practices
        validateBestPractices(config, errors, warnings)
        
        // Check automation workflows
        config.automation?.let { validateAutomation(it, errors, warnings) }
    }
    
    private fun validateSecurity(
        config: CompiledConfig,
        errors: MutableList<ConfigValidationError>,
        warnings: MutableList<ConfigValidationWarning>
    ) {
        // Check for weak passwords (if any)
        config.users.forEach { user ->
            if (user.name == "root" && user.uid == 0) {
                warnings.add(ConfigValidationWarning.SecurityWarning(
                    "Creating root user directly is not recommended. Use sudo instead."
                ))
            }
        }
        
        // Check for insecure services
        config.services.forEach { service ->
            if (service.name == "telnet" && service.enabled) {
                errors.add(ConfigValidationError.SecurityError(
                    "Telnet service is insecure and should not be enabled"
                ))
            }
        }
        
        // Check for GPG verification disabled
        val insecureRepos = config.repositories.filter { !it.gpgCheck }
        if (insecureRepos.isNotEmpty()) {
            warnings.add(ConfigValidationWarning.SecurityWarning(
                "GPG verification is disabled for repositories: ${insecureRepos.map { it.name }.joinToString(", ")}"
            ))
        }
    }
    
    private fun validatePerformance(
        config: CompiledConfig,
        errors: MutableList<ConfigValidationError>,
        warnings: MutableList<ConfigValidationWarning>
    ) {
        // Check for too many packages
        if (config.packages.size > 1000) {
            warnings.add(ConfigValidationWarning.PerformanceWarning(
                "Large number of packages (${config.packages.size}) may slow down system updates"
            ))
        }
        
        // Check for conflicting services
        val networkManagers = listOf("NetworkManager", "systemd-networkd", "wicd")
        val enabledNetworkManagers = config.services
            .filter { it.enabled && it.name in networkManagers }
            .map { it.name }
        
        if (enabledNetworkManagers.size > 1) {
            errors.add(ConfigValidationError.ConflictError(
                "Multiple network managers enabled: ${enabledNetworkManagers.joinToString(", ")}"
            ))
        }
    }
    
    private fun validateCompatibility(
        config: CompiledConfig,
        errors: MutableList<ConfigValidationError>,
        warnings: MutableList<ConfigValidationWarning>
    ) {
        // Check desktop environment compatibility
        config.desktop?.let { desktop ->
            when (desktop.environment) {
                DesktopEnvironment.HYPRLAND -> {
                    // Check if Wayland-compatible packages are installed
                    val requiredPackages = listOf("wayland", "xorg-xwayland")
                    val installedPackages = config.packages
                        .filter { it.action == PackageAction.INSTALL }
                        .map { it.name }
                    
                    requiredPackages.forEach { pkg ->
                        if (pkg !in installedPackages) {
                            warnings.add(ConfigValidationWarning.CompatibilityWarning(
                                "Hyprland requires package '$pkg' which is not explicitly installed"
                            ))
                        }
                    }
                }
                DesktopEnvironment.PLASMA -> {
                    // Check for Plasma dependencies
                    if ("plasma-meta" !in config.packages.map { it.name }) {
                        warnings.add(ConfigValidationWarning.CompatibilityWarning(
                            "Plasma desktop selected but 'plasma-meta' package not installed"
                        ))
                    }
                }
                else -> { /* Other desktop environments */ }
            }
        }
    }
    
    private fun validateBestPractices(
        config: CompiledConfig,
        errors: MutableList<ConfigValidationError>,
        warnings: MutableList<ConfigValidationWarning>
    ) {
        // Check if essential services are enabled
        val essentialServices = listOf("NetworkManager", "systemd-networkd")
        val hasNetworking = config.services.any { it.enabled && it.name in essentialServices }
        
        if (!hasNetworking) {
            warnings.add(ConfigValidationWarning.BestPracticeWarning(
                "No networking service enabled. System may not have network connectivity."
            ))
        }
        
        // Check if users have appropriate groups
        config.users.forEach { user ->
            if (user.name != "root" && "wheel" !in user.groups && "sudo" !in user.groups) {
                warnings.add(ConfigValidationWarning.BestPracticeWarning(
                    "User '${user.name}' is not in wheel or sudo group, may not have admin privileges"
                ))
            }
        }
        
        // Check for locale generation
        if (config.packages.none { it.name == "glibc-locales" && it.action == PackageAction.INSTALL }) {
            warnings.add(ConfigValidationWarning.BestPracticeWarning(
                "Consider installing 'glibc-locales' for proper locale support"
            ))
        }
    }
    
    private fun validateAutomation(
        automation: AutomationConfig,
        errors: MutableList<ConfigValidationError>,
        warnings: MutableList<ConfigValidationWarning>
    ) {
        // Check workflows
        automation.workflows.forEach { workflow ->
            // Check for infinite loops
            workflow.actions.forEach { action ->
                if (action is Action.Loop && action.times > 1000) {
                    warnings.add(ConfigValidationWarning.AutomationWarning(
                        "Workflow '${workflow.name}' has a loop with ${action.times} iterations, may cause performance issues"
                    ))
                }
            }
            
            // Check for missing triggers
            if (workflow.trigger == null && workflow.enabled) {
                warnings.add(ConfigValidationWarning.AutomationWarning(
                    "Workflow '${workflow.name}' is enabled but has no trigger defined"
                ))
            }
            
            // Check for security in automation
            workflow.actions.forEach { action ->
                when (action) {
                    is Action.RunCommand -> {
                        if (action.command.contains("sudo") || action.command.contains("su")) {
                            warnings.add(ConfigValidationWarning.SecurityWarning(
                                "Workflow '${workflow.name}' uses privilege escalation in command: ${action.command}"
                            ))
                        }
                    }
                    is Action.FileOperation -> {
                        when (val op = action.operation) {
                            is FileOperation.Delete -> {
                                if (op.path.startsWith("/etc") || op.path.startsWith("/usr")) {
                                    errors.add(ConfigValidationError.AutomationError(
                                        "Workflow attempts to delete system file: ${op.path}"
                                    ))
                                }
                            }
                            is FileOperation.Write -> {
                                if (op.path.startsWith("/etc") && !op.path.contains("horizonos")) {
                                    warnings.add(ConfigValidationWarning.AutomationWarning(
                                        "Workflow writes to system configuration: ${op.path}"
                                    ))
                                }
                            }
                            else -> { /* Other operations are less risky */ }
                        }
                    }
                    else -> { /* Other actions */ }
                }
            }
        }
        
        // Check teaching modes
        automation.teachingModes.forEach { teaching ->
            if (teaching.watchedPath == "/" || teaching.watchedPath == "/etc") {
                errors.add(ConfigValidationError.AutomationError(
                    "Teaching mode '${teaching.name}' watches system-critical path: ${teaching.watchedPath}"
                ))
            }
        }
    }
}

// ===== Result Types =====

data class EnhancedValidationResult(
    val errors: List<ConfigValidationError>,
    val warnings: List<ConfigValidationWarning>,
    val isValid: Boolean
) {
    val hasWarnings: Boolean get() = warnings.isNotEmpty()
    val hasErrors: Boolean get() = errors.isNotEmpty()
    
    fun getAllMessages(): List<String> {
        return errors.map { "ERROR: ${it.message}" } +
               warnings.map { "WARNING: ${it.message}" }
    }
}

sealed class ConfigValidationError(val message: String) {
    data class BasicValidationError(val error: ValidationError) : ConfigValidationError(error.message)
    data class SecurityError(val details: String) : ConfigValidationError("Security: $details")
    data class ConflictError(val details: String) : ConfigValidationError("Conflict: $details")
    data class AutomationError(val details: String) : ConfigValidationError("Automation: $details")
}

sealed class ConfigValidationWarning(val message: String) {
    data class SecurityWarning(val details: String) : ConfigValidationWarning("Security: $details")
    data class PerformanceWarning(val details: String) : ConfigValidationWarning("Performance: $details")
    data class CompatibilityWarning(val details: String) : ConfigValidationWarning("Compatibility: $details")
    data class BestPracticeWarning(val details: String) : ConfigValidationWarning("Best Practice: $details")
    data class AutomationWarning(val details: String) : ConfigValidationWarning("Automation: $details")
}