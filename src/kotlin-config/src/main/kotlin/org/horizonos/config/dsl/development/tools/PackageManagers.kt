package org.horizonos.config.dsl.development.tools

import kotlinx.serialization.Serializable
import org.horizonos.config.dsl.HorizonOSDsl

// ===== Package Manager Configuration =====

@HorizonOSDsl
class PackageManagerContext(private val type: PackageManagerType) {
    var enabled: Boolean = true
    val repositories = mutableListOf<String>()
    val globalPackages = mutableListOf<String>()
    val configurations = mutableMapOf<String, String>()
    val environmentVariables = mutableMapOf<String, String>()

    fun repository(url: String) {
        repositories.add(url)
    }

    fun globalPackage(name: String) {
        globalPackages.add(name)
    }

    fun config(key: String, value: String) {
        configurations[key] = value
    }

    fun env(key: String, value: String) {
        environmentVariables[key] = value
    }

    fun toConfig(): PackageManagerConfig {
        return PackageManagerConfig(
            type = type,
            enabled = enabled,
            repositories = repositories,
            globalPackages = globalPackages,
            configurations = configurations,
            environmentVariables = environmentVariables
        )
    }
}

@Serializable
data class PackageManagerConfig(
    val type: PackageManagerType,
    val enabled: Boolean,
    val repositories: List<String>,
    val globalPackages: List<String>,
    val configurations: Map<String, String>,
    val environmentVariables: Map<String, String>
)

@Serializable
enum class PackageManagerType {
    APT, YUM, DNF, PACMAN, YAY, PARU, ZYPPER, APK, BREW, 
    MACPORTS, CHOCOLATEY, SCOOP, WINGET, NIX, GUIX, SNAP, 
    FLATPAK, APPIMAGE, CARGO, CONAN, VCPKG
}