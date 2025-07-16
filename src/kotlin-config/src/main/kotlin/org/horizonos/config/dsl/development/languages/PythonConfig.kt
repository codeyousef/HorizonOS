package org.horizonos.config.dsl.development.languages

import kotlinx.serialization.Serializable
import org.horizonos.config.dsl.HorizonOSDsl

// ===== Python Configuration =====

@HorizonOSDsl
class PythonContext {
    var packageManager: PythonPackageManager = PythonPackageManager.PIP
    var virtualenvTool: PythonVirtualenvTool = PythonVirtualenvTool.VENV
    val globalPackages = mutableListOf<String>()
    val pipConfig = mutableMapOf<String, String>()
    var enableUvLoop: Boolean = false

    fun globalPackage(name: String) {
        globalPackages.add(name)
    }

    fun pipConfig(key: String, value: String) {
        pipConfig[key] = value
    }

    fun pip() {
        packageManager = PythonPackageManager.PIP
    }

    fun pipenv() {
        packageManager = PythonPackageManager.PIPENV
    }

    fun poetry() {
        packageManager = PythonPackageManager.POETRY
    }

    fun pdm() {
        packageManager = PythonPackageManager.PDM
    }

    fun conda() {
        packageManager = PythonPackageManager.CONDA
    }

    fun uv() {
        packageManager = PythonPackageManager.UV
    }

    fun venv() {
        virtualenvTool = PythonVirtualenvTool.VENV
    }

    fun virtualenv() {
        virtualenvTool = PythonVirtualenvTool.VIRTUALENV
    }

    fun pyenv() {
        virtualenvTool = PythonVirtualenvTool.PYENV
    }

    fun toConfig(): PythonConfig {
        return PythonConfig(
            packageManager = packageManager,
            virtualenvTool = virtualenvTool,
            globalPackages = globalPackages,
            pipConfig = pipConfig,
            enableUvLoop = enableUvLoop
        )
    }
}

@Serializable
data class PythonConfig(
    val packageManager: PythonPackageManager,
    val virtualenvTool: PythonVirtualenvTool,
    val globalPackages: List<String>,
    val pipConfig: Map<String, String>,
    val enableUvLoop: Boolean
)

@Serializable
enum class PythonPackageManager {
    PIP, PIPENV, POETRY, PDM, CONDA, UV
}

@Serializable
enum class PythonVirtualenvTool {
    VENV, VIRTUALENV, PYENV, CONDA, POETRY, PIPENV
}