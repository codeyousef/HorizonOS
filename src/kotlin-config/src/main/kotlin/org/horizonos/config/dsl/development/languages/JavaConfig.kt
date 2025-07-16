package org.horizonos.config.dsl.development.languages

import kotlinx.serialization.Serializable
import org.horizonos.config.dsl.HorizonOSDsl

// ===== Java Configuration =====

@HorizonOSDsl
class JavaContext {
    var jvmImplementation: JVMImplementation = JVMImplementation.OPENJDK
    var enableJavaFX: Boolean = false
    val jvmArgs = mutableListOf<String>()
    var mavenConfig: MavenConfig? = null
    var gradleConfig: GradleConfig? = null

    fun jvmArg(arg: String) {
        jvmArgs.add(arg)
    }

    fun openjdk() {
        jvmImplementation = JVMImplementation.OPENJDK
    }

    fun graalvm() {
        jvmImplementation = JVMImplementation.GRAALVM
    }

    fun corretto() {
        jvmImplementation = JVMImplementation.CORRETTO
    }

    fun zulu() {
        jvmImplementation = JVMImplementation.ZULU
    }

    fun maven(block: MavenContext.() -> Unit) {
        mavenConfig = MavenContext().apply(block).toConfig()
    }

    fun gradle(block: GradleContext.() -> Unit) {
        gradleConfig = GradleContext().apply(block).toConfig()
    }

    fun toConfig(): JavaConfig {
        return JavaConfig(
            jvmImplementation = jvmImplementation,
            enableJavaFX = enableJavaFX,
            jvmArgs = jvmArgs,
            mavenConfig = mavenConfig,
            gradleConfig = gradleConfig
        )
    }
}

@HorizonOSDsl
class MavenContext {
    var localRepository: String = "~/.m2/repository"
    val mirrors = mutableListOf<MavenMirror>()
    val profiles = mutableListOf<MavenProfile>()

    fun mirror(id: String, url: String, mirrorOf: String = "*") {
        mirrors.add(MavenMirror(id, url, mirrorOf))
    }

    fun profile(id: String, activeByDefault: Boolean = false, block: MavenProfileContext.() -> Unit) {
        val context = MavenProfileContext(id, activeByDefault)
        context.block()
        profiles.add(context.toProfile())
    }

    fun toConfig(): MavenConfig {
        return MavenConfig(
            localRepository = localRepository,
            mirrors = mirrors,
            profiles = profiles
        )
    }
}

@HorizonOSDsl
class MavenProfileContext(private val id: String, private val activeByDefault: Boolean) {
    val repositories = mutableListOf<MavenRepository>()

    fun repository(id: String, url: String, releases: Boolean = true, snapshots: Boolean = false) {
        repositories.add(MavenRepository(id, url, releases, snapshots))
    }

    fun toProfile(): MavenProfile {
        return MavenProfile(
            id = id,
            activeByDefault = activeByDefault,
            repositories = repositories
        )
    }
}

@HorizonOSDsl
class GradleContext {
    var distributionUrl: String = "https://services.gradle.org/distributions/gradle-8.5-bin.zip"
    var enableDaemon: Boolean = true
    val jvmArgs = mutableListOf<String>()
    val systemProperties = mutableMapOf<String, String>()

    fun jvmArg(arg: String) {
        jvmArgs.add(arg)
    }

    fun systemProperty(key: String, value: String) {
        systemProperties[key] = value
    }

    fun toConfig(): GradleConfig {
        return GradleConfig(
            distributionUrl = distributionUrl,
            enableDaemon = enableDaemon,
            jvmArgs = jvmArgs,
            systemProperties = systemProperties
        )
    }
}

@Serializable
data class JavaConfig(
    val jvmImplementation: JVMImplementation,
    val enableJavaFX: Boolean,
    val jvmArgs: List<String>,
    val mavenConfig: MavenConfig?,
    val gradleConfig: GradleConfig?
)

@Serializable
data class MavenConfig(
    val localRepository: String,
    val mirrors: List<MavenMirror>,
    val profiles: List<MavenProfile>
)

@Serializable
data class MavenMirror(
    val id: String,
    val url: String,
    val mirrorOf: String
)

@Serializable
data class MavenProfile(
    val id: String,
    val activeByDefault: Boolean,
    val repositories: List<MavenRepository>
)

@Serializable
data class MavenRepository(
    val id: String,
    val url: String,
    val releases: Boolean,
    val snapshots: Boolean
)

@Serializable
data class GradleConfig(
    val distributionUrl: String,
    val enableDaemon: Boolean,
    val jvmArgs: List<String>,
    val systemProperties: Map<String, String>
)

@Serializable
enum class JVMImplementation {
    OPENJDK, ORACLE_JDK, GRAALVM, CORRETTO, ZULU, TEMURIN, LIBERICA, SAPMACHINE
}