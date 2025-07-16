package org.horizonos.config.dsl.development.languages

import kotlinx.serialization.Serializable
import org.horizonos.config.dsl.HorizonOSDsl

// ===== Node.js Configuration =====

@HorizonOSDsl
class NodeJSContext {
    var packageManager: NodePackageManager = NodePackageManager.NPM
    var yarnVersion: String = "latest"
    var enableCorepack: Boolean = true
    val globalPackages = mutableListOf<String>()
    val registries = mutableMapOf<String, String>()

    fun globalPackage(name: String) {
        globalPackages.add(name)
    }

    fun registry(scope: String, url: String) {
        registries[scope] = url
    }

    fun npm() {
        packageManager = NodePackageManager.NPM
    }

    fun yarn() {
        packageManager = NodePackageManager.YARN
    }

    fun pnpm() {
        packageManager = NodePackageManager.PNPM
    }

    fun bun() {
        packageManager = NodePackageManager.BUN
    }

    fun toConfig(): NodeJSConfig {
        return NodeJSConfig(
            packageManager = packageManager,
            yarnVersion = yarnVersion,
            enableCorepack = enableCorepack,
            globalPackages = globalPackages,
            registries = registries
        )
    }
}

@Serializable
data class NodeJSConfig(
    val packageManager: NodePackageManager,
    val yarnVersion: String,
    val enableCorepack: Boolean,
    val globalPackages: List<String>,
    val registries: Map<String, String>
)

@Serializable
enum class NodePackageManager {
    NPM, YARN, PNPM, BUN
}