package org.horizonos.config.dsl.development.languages

import kotlinx.serialization.Serializable
import org.horizonos.config.dsl.HorizonOSDsl

// ===== Language Runtime Configuration =====

@HorizonOSDsl
class LanguageRuntimeContext(private val type: LanguageType) {
    var enabled: Boolean = true
    var version: String? = null
    var defaultVersion: String? = null
    var globalPackages = mutableListOf<String>()
    var environmentVariables = mutableMapOf<String, String>()
    var configOverrides = mutableMapOf<String, String>()
    
    // Language-specific configurations
    var nodeConfig: NodeJSConfig? = null
    var pythonConfig: PythonConfig? = null
    var javaConfig: JavaConfig? = null
    var rustConfig: RustConfig? = null
    var goConfig: GoConfig? = null
    var rubyConfig: RubyConfig? = null

    fun nodejs(block: NodeJSContext.() -> Unit) {
        nodeConfig = NodeJSContext().apply(block).toConfig()
    }

    fun python(block: PythonContext.() -> Unit) {
        pythonConfig = PythonContext().apply(block).toConfig()
    }

    fun java(block: JavaContext.() -> Unit) {
        javaConfig = JavaContext().apply(block).toConfig()
    }

    fun rust(block: RustContext.() -> Unit) {
        rustConfig = RustContext().apply(block).toConfig()
    }

    fun go(block: GoContext.() -> Unit) {
        goConfig = GoContext().apply(block).toConfig()
    }

    fun ruby(block: RubyContext.() -> Unit) {
        rubyConfig = RubyContext().apply(block).toConfig()
    }

    fun globalPackage(name: String) {
        globalPackages.add(name)
    }

    fun env(key: String, value: String) {
        environmentVariables[key] = value
    }

    fun config(key: String, value: String) {
        configOverrides[key] = value
    }

    fun toRuntime(): LanguageRuntime {
        return LanguageRuntime(
            type = type,
            enabled = enabled,
            version = version,
            defaultVersion = defaultVersion,
            globalPackages = globalPackages,
            environmentVariables = environmentVariables,
            configOverrides = configOverrides,
            nodeConfig = nodeConfig,
            pythonConfig = pythonConfig,
            javaConfig = javaConfig,
            rustConfig = rustConfig,
            goConfig = goConfig,
            rubyConfig = rubyConfig
        )
    }
}

// Language Runtime Data Classes
@Serializable
data class LanguageRuntime(
    val type: LanguageType,
    val enabled: Boolean,
    val version: String?,
    val defaultVersion: String?,
    val globalPackages: List<String>,
    val environmentVariables: Map<String, String>,
    val configOverrides: Map<String, String>,
    val nodeConfig: NodeJSConfig?,
    val pythonConfig: PythonConfig?,
    val javaConfig: JavaConfig?,
    val rustConfig: RustConfig?,
    val goConfig: GoConfig?,
    val rubyConfig: RubyConfig?
)

@Serializable
enum class LanguageType {
    NODEJS, PYTHON, JAVA, RUST, GO, RUBY, CPP, C, CSHARP, 
    PHP, SWIFT, KOTLIN, SCALA, ELIXIR, ERLANG, HASKELL, 
    LUA, PERL, R, JULIA, NIM, CRYSTAL, ZIG, V
}