package org.horizonos.config.compiler

import org.horizonos.config.dsl.CompiledConfig
import org.horizonos.config.dsl.horizonOS
import java.io.File
import javax.script.ScriptEngine
import javax.script.ScriptEngineManager
import kotlin.script.experimental.api.*
import kotlin.script.experimental.host.toScriptSource
import kotlin.script.experimental.jvm.*
import kotlin.script.experimental.jvmhost.BasicJvmScriptingHost
import kotlin.script.experimental.jvmhost.createJvmCompilationConfigurationFromTemplate

/**
 * Parser for HorizonOS .kts configuration files
 * Uses Kotlin Script engine to evaluate DSL files
 */
class ConfigParser {
    
    private val scriptingHost = BasicJvmScriptingHost()
    
    /**
     * Parse a .horizonos.kts file and return the compiled configuration
     */
    suspend fun parseFile(file: File): ParseResult {
        if (!file.exists()) {
            return ParseResult.Error(ParseError.FileNotFound(file.absolutePath))
        }
        
        if (!file.name.endsWith(".horizonos.kts")) {
            return ParseResult.Error(ParseError.InvalidFileExtension(file.name))
        }
        
        return try {
            val config = evaluateScript(file)
            ParseResult.Success(config)
        } catch (e: Exception) {
            ParseResult.Error(ParseError.ScriptEvaluationError(e.message ?: "Unknown error", e))
        }
    }
    
    /**
     * Parse configuration from a string
     */
    suspend fun parseString(content: String): ParseResult {
        return try {
            val config = evaluateScriptContent(content)
            ParseResult.Success(config)
        } catch (e: Exception) {
            ParseResult.Error(ParseError.ScriptEvaluationError(e.message ?: "Unknown error", e))
        }
    }
    
    private suspend fun evaluateScript(file: File): CompiledConfig {
        val scriptContent = file.readText()
        return evaluateScriptContent(scriptContent)
    }
    
    private suspend fun evaluateScriptContent(content: String): CompiledConfig {
        // Create compilation configuration first
        val compilationConfiguration = createJvmCompilationConfigurationFromTemplate<Any> {
            jvm {
                // Add current classpath to make DSL classes available
                dependenciesFromCurrentContext(wholeClasspath = true)
            }
            
            // Add default imports
            defaultImports(
                "org.horizonos.config.dsl.*",
                "org.horizonos.config.dsl.security.*",
                "org.horizonos.config.dsl.services.*", 
                "org.horizonos.config.dsl.hardware.*",
                "kotlin.time.Duration",
                "kotlin.time.Duration.Companion.*"
            )
        }
        
        // Evaluate the script
        val evaluationConfiguration = ScriptEvaluationConfiguration {
            // Configure script evaluation
        }
        
        val result = scriptingHost.eval(
            content.toScriptSource(),
            compilationConfiguration,
            evaluationConfiguration
        )
        
        return when (result) {
            is ResultWithDiagnostics.Success -> {
                val returnValue = result.value.returnValue
                when (returnValue) {
                    is ResultValue.Value -> {
                        val value = returnValue.value
                        if (value is CompiledConfig) {
                            value
                        } else {
                            throw IllegalStateException("Script did not return a CompiledConfig, got: ${value?.javaClass?.simpleName}")
                        }
                    }
                    is ResultValue.Unit -> throw IllegalStateException("Script returned Unit instead of CompiledConfig")
                    is ResultValue.Error -> throw returnValue.error
                    else -> throw IllegalStateException("Unexpected return value type")
                }
            }
            is ResultWithDiagnostics.Failure -> {
                val diagnostics = result.reports.joinToString("\n") { report ->
                    "${report.severity}: ${report.message}${report.location?.let { " at $it" } ?: ""}"
                }
                throw IllegalStateException("Script compilation failed:\n$diagnostics")
            }
        }
    }
}

/**
 * Alternative parser using simple pattern matching
 * Fallback option if Kotlin Script engine is not available
 */
class SimpleConfigParser {
    
    fun parseFile(file: File): ParseResult {
        if (!file.exists()) {
            return ParseResult.Error(ParseError.FileNotFound(file.absolutePath))
        }
        
        if (!file.name.endsWith(".horizonos.kts")) {
            return ParseResult.Error(ParseError.InvalidFileExtension(file.name))
        }
        
        return try {
            val content = file.readText()
            val config = parseContent(content)
            ParseResult.Success(config)
        } catch (e: Exception) {
            ParseResult.Error(ParseError.ParsingError(e.message ?: "Unknown error"))
        }
    }
    
    private fun parseContent(content: String): CompiledConfig {
        // This is a simplified parser that extracts basic values
        // In production, use the ScriptEngine-based parser
        
        val hostname = extractStringValue(content, "hostname")
        val timezone = extractStringValue(content, "timezone")
        val locale = extractStringValue(content, "locale")
        
        // Create a basic configuration
        return horizonOS {
            this.hostname = hostname ?: "horizonos"
            this.timezone = timezone ?: "UTC"
            this.locale = locale ?: "en_US.UTF-8"
        }
    }
    
    private fun extractStringValue(content: String, property: String): String? {
        val pattern = Regex("""$property\s*=\s*"([^"]+)"""")
        return pattern.find(content)?.groupValues?.get(1)
    }
}

// ===== Result Types =====

sealed class ParseResult {
    data class Success(val config: CompiledConfig) : ParseResult()
    data class Error(val error: ParseError) : ParseResult()
}

sealed class ParseError(val message: String) {
    data class FileNotFound(val path: String) : ParseError("File not found: $path")
    data class InvalidFileExtension(val filename: String) : ParseError("Invalid file extension. Expected .horizonos.kts, got: $filename")
    data class ScriptEvaluationError(val details: String, val cause: Throwable) : ParseError("Script evaluation failed: $details")
    data class ParsingError(val details: String) : ParseError("Parsing failed: $details")
}