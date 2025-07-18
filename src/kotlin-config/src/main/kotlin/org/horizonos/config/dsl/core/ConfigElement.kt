package org.horizonos.config.dsl.core

/**
 * Base interface for all configuration elements in the DSL
 */
interface ConfigElement {
    /**
     * Validate the configuration element
     * @throws IllegalArgumentException if validation fails
     */
    fun validate()
    
    /**
     * Convert the configuration element to a map representation
     */
    fun toConfig(): Map<String, Any>
}

/**
 * DSL marker annotation for configuration DSL scope
 */
@DslMarker
annotation class ConfigDsl

/**
 * Base interface for code generation
 */
interface CodeGenerator {
    /**
     * Generate code from configuration
     */
    fun generate(config: Map<String, Any>): String
}

/**
 * Configuration validation result
 */
data class ValidationResult(
    val isValid: Boolean,
    val errors: List<ValidationError> = emptyList()
) {
    fun throwIfInvalid() {
        if (!isValid) {
            throw IllegalArgumentException(
                "Configuration validation failed:\n" + 
                errors.joinToString("\n") { "- ${it.path}: ${it.message}" }
            )
        }
    }
}

/**
 * Validation error detail
 */
data class ValidationError(
    val path: String,
    val message: String,
    val severity: ValidationSeverity = ValidationSeverity.ERROR
)

/**
 * Validation severity levels
 */
enum class ValidationSeverity {
    WARNING,
    ERROR,
    CRITICAL
}