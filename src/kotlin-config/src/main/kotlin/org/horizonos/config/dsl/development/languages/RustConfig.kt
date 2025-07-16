package org.horizonos.config.dsl.development.languages

import kotlinx.serialization.Serializable
import org.horizonos.config.dsl.HorizonOSDsl

// ===== Rust Configuration =====

@HorizonOSDsl
class RustContext {
    var toolchain: RustToolchain = RustToolchain.STABLE
    val targets = mutableListOf<String>()
    val components = mutableListOf<String>()
    val cargoConfig = mutableMapOf<String, String>()
    var enableSccache: Boolean = false
    var enableMiri: Boolean = false

    fun stable() {
        toolchain = RustToolchain.STABLE
    }

    fun beta() {
        toolchain = RustToolchain.BETA
    }

    fun nightly() {
        toolchain = RustToolchain.NIGHTLY
    }

    fun target(name: String) {
        targets.add(name)
    }

    fun component(name: String) {
        components.add(name)
    }

    fun cargoConfig(key: String, value: String) {
        cargoConfig[key] = value
    }

    fun toConfig(): RustConfig {
        return RustConfig(
            toolchain = toolchain,
            targets = targets,
            components = components,
            cargoConfig = cargoConfig,
            enableSccache = enableSccache,
            enableMiri = enableMiri
        )
    }
}

@Serializable
data class RustConfig(
    val toolchain: RustToolchain,
    val targets: List<String>,
    val components: List<String>,
    val cargoConfig: Map<String, String>,
    val enableSccache: Boolean,
    val enableMiri: Boolean
)

@Serializable
enum class RustToolchain {
    STABLE, BETA, NIGHTLY
}