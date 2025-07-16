package org.horizonos.config

import com.github.ajalt.clikt.core.CliktCommand
import com.github.ajalt.clikt.parameters.arguments.argument
import com.github.ajalt.clikt.parameters.options.default
import com.github.ajalt.clikt.parameters.options.option
import com.github.ajalt.clikt.parameters.types.file
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import org.horizonos.config.dsl.*
import org.horizonos.config.compiler.*
import java.io.File
import kotlinx.coroutines.runBlocking

class CompileCommand : CliktCommand(
    name = "compile",
    help = "Compile a HorizonOS configuration file"
) {
    private val configFile by argument(help = "Configuration file to compile")
        .file(mustExist = true, canBeDir = false, mustBeReadable = true)

    private val outputDir by option("-o", "--output", help = "Output directory")
        .file(canBeFile = false)
        .default(File("./output"))

    override fun run() = runBlocking {
        echo("Compiling HorizonOS configuration: ${configFile.absolutePath}")

        // Create output directory
        outputDir.mkdirs()

        try {
            // Parse the configuration file
            echo("  Parsing configuration...")
            val parser = ConfigParser()
            val parseResult = parser.parseFile(configFile)
            
            val config = when (parseResult) {
                is ParseResult.Success -> {
                    echo("  ✓ Parsing successful")
                    parseResult.config
                }
                is ParseResult.Error -> {
                    echo("  ✗ Parsing failed: ${parseResult.error.message}", err = true)
                    throw RuntimeException(parseResult.error.message)
                }
            }
            
            // Validate the configuration
            echo("  Validating configuration...")
            val validator = EnhancedConfigValidator()
            val validationResult = validator.validate(config)
            
            if (validationResult.hasErrors) {
                echo("  ✗ Validation failed with ${validationResult.errors.size} errors:", err = true)
                validationResult.errors.forEach { error ->
                    echo("    - ${error.message}", err = true)
                }
                throw RuntimeException("Configuration validation failed")
            }
            
            if (validationResult.hasWarnings) {
                echo("  ⚠ Validation completed with ${validationResult.warnings.size} warnings:")
                validationResult.warnings.forEach { warning ->
                    echo("    - ${warning.message}")
                }
            } else {
                echo("  ✓ Validation successful")
            }
            
            // Generate output files
            echo("  Generating output files...")
            val generator = EnhancedConfigGenerator(outputDir)
            val generationResult = generator.generate(config)
            
            when (generationResult) {
                is GenerationResult.Success -> {
                    echo("  ✓ Generation successful")
                    echo()
                    echo("✓ Compilation completed successfully!")
                    echo("  Output directory: ${outputDir.absolutePath}")
                    echo("  Generated ${generationResult.files.size} files:")
                    
                    generationResult.files.groupBy { it.type }.forEach { (type, files) ->
                        echo()
                        echo("  ${type.displayName}:")
                        files.forEach { file ->
                            echo("    - ${file.path}")
                        }
                    }
                }
                is GenerationResult.Error -> {
                    echo("  ✗ Generation failed: ${generationResult.error.message}", err = true)
                    throw RuntimeException(generationResult.error.message)
                }
            }
        } catch (e: Exception) {
            echo()
            echo("✗ Compilation failed: ${e.message}", err = true)
            throw e
        }
    }
}

fun main(args: Array<String>) = CompileCommand().main(args)
