package org.horizonos.config.compiler

/**
 * Common types for configuration generators
 */

data class GeneratedFile(
    val path: String,
    val type: FileType
)

enum class FileType(val displayName: String) {
    JSON("JSON Files"),
    YAML("YAML Files"),
    SHELL("Shell Scripts"),
    SYSTEMD("Systemd Units"),
    ANSIBLE("Ansible Playbooks"),
    DOCKER("Docker Files"),
    DOCUMENTATION("Documentation")
}

// ===== Result Types =====

sealed class GenerationResult {
    data class Success(val files: List<GeneratedFile>) : GenerationResult()
    data class Error(val error: GenerationError) : GenerationResult()
}

sealed class GenerationError(val message: String) {
    data class UnexpectedError(val details: String, val cause: Throwable) : GenerationError("Generation failed: $details")
}