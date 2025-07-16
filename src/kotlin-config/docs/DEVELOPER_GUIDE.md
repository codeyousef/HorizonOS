# HorizonOS Kotlin DSL Developer Guide

This guide covers the internal architecture and development of the HorizonOS Kotlin Configuration DSL.

## Architecture Overview

The DSL is organized into several key components:

```
kotlin-config/
├── src/main/kotlin/org/horizonos/config/
│   ├── dsl/              # DSL definitions and builders
│   │   ├── Core.kt       # Main DSL entry point
│   │   └── Automation.kt # Automation DSL
│   ├── validation/       # Validation logic
│   │   └── Validators.kt # Built-in validators
│   ├── compiler/         # Compilation pipeline
│   │   ├── Parser.kt     # Kotlin script parser
│   │   ├── Validator.kt  # Enhanced validation
│   │   └── Generator.kt  # Multi-format generator
│   └── runtime/          # Runtime execution
│       ├── ExecutionEngine.kt    # Main execution engine
│       ├── LiveUpdateManager.kt  # Live updates
│       ├── ChangeDetector.kt     # Configuration diffing
│       └── StateSyncManager.kt   # State management
```

## Key Components

### DSL Core (`dsl/Core.kt`)

The DSL provides a type-safe builder pattern:

```kotlin
@DslMarker
annotation class HorizonOSDsl

@HorizonOSDsl
class HorizonOSContext {
    var hostname: String = ""
    var timezone: String = ""
    var locale: String = ""
    
    fun packages(block: PackageContext.() -> Unit) {
        // Package configuration
    }
}
```

### Validation System

Validators ensure configuration correctness:

```kotlin
interface Validator<T> {
    fun validate(value: T): ValidationResult
}

class HostnameValidator : Validator<String> {
    override fun validate(value: String): ValidationResult {
        // RFC 1123 compliant validation
    }
}
```

### Compilation Pipeline

1. **Parser**: Reads `.horizonos.kts` files
2. **Validator**: Checks configuration validity
3. **Generator**: Produces output formats

### Runtime Execution

The execution engine applies configurations:

```kotlin
class ExecutionEngine {
    suspend fun applyConfiguration(config: CompiledConfig): ExecutionResult
    suspend fun applyLiveUpdates(current: CompiledConfig, new: CompiledConfig): LiveUpdateResult
}
```

## Adding New Features

### Adding a New DSL Element

1. Define the data model in `Core.kt`:

```kotlin
@Serializable
data class NewFeature(
    val name: String,
    val enabled: Boolean = true,
    val config: Map<String, String> = emptyMap()
)
```

2. Add builder context:

```kotlin
@HorizonOSDsl
class NewFeatureContext {
    var name: String = ""
    var enabled: Boolean = true
    private val config = mutableMapOf<String, String>()
    
    fun configure(key: String, value: String) {
        config[key] = value
    }
    
    fun toFeature() = NewFeature(name, enabled, config)
}
```

3. Add to main context:

```kotlin
class HorizonOSContext {
    private val newFeatures = mutableListOf<NewFeature>()
    
    fun newFeature(name: String, block: NewFeatureContext.() -> Unit) {
        val context = NewFeatureContext().apply {
            this.name = name
            block()
        }
        newFeatures.add(context.toFeature())
    }
}
```

### Adding a New Validator

1. Create validator class:

```kotlin
class NewFeatureValidator : Validator<String> {
    override fun validate(value: String): ValidationResult {
        return when {
            value.isBlank() -> ValidationResult.Invalid("Value cannot be blank")
            value.length > 100 -> ValidationResult.Invalid("Value too long")
            else -> ValidationResult.Valid
        }
    }
}
```

2. Register in `Validators.kt`:

```kotlin
object Validators {
    val newFeature = NewFeatureValidator()
}
```

### Adding a New Output Format

1. Add format type in `Generator.kt`:

```kotlin
enum class FileType(val displayName: String) {
    NEWFORMAT("New Format Files")
}
```

2. Implement generator method:

```kotlin
private fun generateNewFormat(config: CompiledConfig) {
    val content = buildString {
        // Generate format-specific content
    }
    
    val file = File(outputDir, "newformat/config.ext")
    file.writeText(content)
    generatedFiles.add(GeneratedFile("newformat/config.ext", FileType.NEWFORMAT))
}
```

## Testing

### Unit Testing

Test DSL builders:

```kotlin
class NewFeatureTest : StringSpec({
    "should create new feature with configuration" {
        val config = horizonOS {
            newFeature("test") {
                enabled = true
                configure("key", "value")
            }
        }
        
        config.newFeatures shouldHaveSize 1
        config.newFeatures[0].name shouldBe "test"
    }
})
```

### Integration Testing

Test compilation pipeline:

```kotlin
"should compile configuration with new feature" {
    val configFile = createTempFile(suffix = ".horizonos.kts")
    configFile.writeText("""
        horizonOS {
            newFeature("test") {
                enabled = true
            }
        }
    """)
    
    val result = Compiler.compile(configFile)
    result.shouldBeInstanceOf<CompilationResult.Success>()
}
```

## Best Practices

### DSL Design

1. **Consistency**: Follow existing patterns
2. **Type Safety**: Leverage Kotlin's type system
3. **Validation**: Validate early and clearly
4. **Documentation**: Document all public APIs

### Error Handling

```kotlin
sealed class ConfigError(val message: String) {
    data class ValidationError(val field: String, val reason: String) : 
        ConfigError("Invalid $field: $reason")
    data class ParseError(val line: Int, val cause: String) : 
        ConfigError("Parse error at line $line: $cause")
}
```

### Performance

1. Use coroutines for I/O operations
2. Cache validation results when possible
3. Generate files in parallel
4. Minimize memory allocations

## Debugging

### Enable Debug Output

```kotlin
class ExecutionEngine(
    private val debug: Boolean = System.getenv("HORIZONOS_DEBUG") == "true"
) {
    private fun debug(message: String) {
        if (debug) println("[DEBUG] $message")
    }
}
```

### Common Issues

1. **Serialization Errors**: Ensure all data classes are `@Serializable`
2. **Script Parsing**: Check for missing imports in `.kts` files
3. **Validation Failures**: Review validator logic and error messages

## Contributing

### Code Style

- Use Kotlin coding conventions
- Keep functions small and focused
- Prefer immutability
- Write comprehensive tests

### Pull Request Process

1. Create feature branch
2. Write tests first (TDD)
3. Implement feature
4. Update documentation
5. Submit PR with clear description

## Advanced Topics

### Custom Script Host

For advanced DSL features:

```kotlin
class CustomScriptHost : ScriptHost {
    override fun eval(script: String, context: ScriptContext): Any? {
        // Custom evaluation logic
    }
}
```

### Plugin System

Extend DSL via plugins:

```kotlin
interface HorizonOSPlugin {
    fun configure(context: HorizonOSContext)
    fun validate(config: CompiledConfig): List<ValidationError>
    fun generate(config: CompiledConfig, outputDir: File)
}
```

### Live Update Strategies

Implement custom update strategies:

```kotlin
class CustomUpdateStrategy : UpdateStrategy {
    override fun canUpdate(change: ConfigChange): Boolean {
        // Determine if change can be applied live
    }
    
    override fun apply(change: ConfigChange, system: SystemManager) {
        // Apply the change
    }
}
```

## Resources

- [Kotlin DSL Documentation](https://kotlinlang.org/docs/type-safe-builders.html)
- [Kotlin Serialization](https://github.com/Kotlin/kotlinx.serialization)
- [Kotlin Script](https://github.com/Kotlin/kotlin-script-examples)
- [OSTree Documentation](https://ostreedev.github.io/ostree/)

## FAQ

**Q: How do I add a new desktop environment?**
A: Add the enum value in `Core.kt`, implement configuration in `DesktopConfig`, and add generation logic in `Generator.kt`.

**Q: Can I extend the DSL without modifying core?**
A: Yes, use extension functions on existing contexts or implement a plugin system.

**Q: How do I debug script parsing errors?**
A: Enable debug logging and check the script host's error output. Common issues are missing imports or syntax errors.

**Q: What's the difference between validation and compilation errors?**
A: Validation errors are semantic (invalid values), while compilation errors are syntactic (parsing failures).