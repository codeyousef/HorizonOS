package org.horizonos.config.dsl

import kotlinx.serialization.Serializable

/**
 * Enhanced Package Management DSL for HorizonOS Container Architecture
 * 
 * Supports container-based package management:
 * - System packages via OCI containers (Podman/Docker/Distrobox)
 * - User applications via Flatpak
 * - Legacy compatibility with existing package DSL
 * 
 * Key benefits:
 * - Isolation: System tools run in containers
 * - Flexibility: Use packages from any Linux distribution
 * - Reproducibility: Container image digests guarantee consistency
 * - Immutability: Base system remains unchanged
 * 
 * @see [Containers] for container-based package management
 * @see [ContainersConfig] for container configuration
 * @see [SystemContainer] for individual container settings
 * @see [Layers] for layered architecture configuration
 * @see [SystemLayer] for system container layers
 * @see [UserLayer] for user application layers
 * @see [Reproducible] for reproducible build settings
 * @see [FlatpakApplication] for Flatpak application configuration
 * @see [horizonOS] for main system configuration entry point
 */

// ===== Package Configuration =====

/**
 * Enhanced package management configuration
 * 
 * This class provides modern package management with automatic migration from
 * legacy packages to containers and Flatpaks, supporting multiple package formats
 * and intelligent format selection.
 */
@Serializable
data class PackagesConfig(
    /** System packages configuration (containers, native packages) */
    val system: SystemPackagesConfig = SystemPackagesConfig(),
    
    /** User applications configuration (Flatpak, AppImage, Snap) */
    val applications: ApplicationPackagesConfig = ApplicationPackagesConfig(),
    
    /** Container-based package configuration */
    val containers: ContainersConfig = ContainersConfig(),
    
    /** Legacy package configuration for backward compatibility */
    val legacy: LegacyPackagesConfig = LegacyPackagesConfig(),
    
    /** Whether to automatically migrate legacy packages to modern formats */
    val autoMigrate: Boolean = true,
    
    /** Strategy for migrating packages to modern formats */
    val migrationStrategy: MigrationStrategy = MigrationStrategy.CONTAINER_FIRST
)

@Serializable
data class SystemPackagesConfig(
    val strategy: SystemPackageStrategy = SystemPackageStrategy.CONTAINER,
    val defaultRuntime: ContainerRuntime = ContainerRuntime.DISTROBOX,
    val containers: List<SystemContainer> = emptyList(),
    val globalMounts: List<String> = emptyList(),
    val autoUpdate: Boolean = true
)

@Serializable
data class ApplicationPackagesConfig(
    val strategy: ApplicationPackageStrategy = ApplicationPackageStrategy.FLATPAK,
    val flatpaks: List<FlatpakApplication> = emptyList(),
    val appImages: List<AppImage> = emptyList(),
    val snaps: List<Snap> = emptyList(),
    val autoUpdate: Boolean = true,
    val userInstall: Boolean = true
)

@Serializable
data class LegacyPackagesConfig(
    val packages: List<Package> = emptyList(),
    val migrationEnabled: Boolean = true,
    val migrationTarget: MigrationTarget = MigrationTarget.CONTAINER,
    val warnings: Boolean = true
)

@Serializable
enum class SystemPackageStrategy {
    CONTAINER,      // Use containers for system packages
    LAYERED,        // Use layered images
    MIXED           // Mix of containers and layers
}

@Serializable
enum class ApplicationPackageStrategy {
    FLATPAK,        // Primary: Flatpak applications
    APPIMAGE,       // Secondary: AppImage format
    SNAP,           // Tertiary: Snap packages
    MIXED           // Mix of all formats
}

@Serializable
enum class MigrationStrategy {
    CONTAINER_FIRST,    // Prefer containers for system packages
    FLATPAK_FIRST,      // Prefer Flatpak for applications
    LEGACY_ONLY,        // Keep legacy packages as-is
    AUTOMATIC,          // Automatically choose best format
    MANUAL              // Manual migration control
}

@Serializable
enum class MigrationTarget {
    CONTAINER,          // Migrate to containers
    FLATPAK,           // Migrate to Flatpak
    APPIMAGE,          // Migrate to AppImage
    LEAVE_ALONE        // Don't migrate
}

// ===== Enhanced Package Types =====

@Serializable
data class ContainerPackage(
    val name: String,
    val container: String,
    val version: String? = null,
    val action: PackageAction = PackageAction.INSTALL,
    val export: Boolean = true,
    val binary: String? = null,
    val desktop: Boolean = false
)

@Serializable
data class FlatpakApplication(
    val id: String,
    val action: PackageAction = PackageAction.INSTALL,
    val version: String? = null,
    val branch: String = "stable",
    val runtime: String? = null,
    val runtimeVersion: String? = null,
    val permissions: List<String> = emptyList(),
    val userInstall: Boolean = true,
    val autoUpdate: Boolean = true
)

@Serializable
data class AppImage(
    val name: String,
    val url: String,
    val version: String? = null,
    val checksum: String? = null,
    val desktop: Boolean = true,
    val autoUpdate: Boolean = false
)

@Serializable
data class Snap(
    val name: String,
    val channel: String = "stable",
    val classic: Boolean = false,
    val devmode: Boolean = false,
    val action: PackageAction = PackageAction.INSTALL
)

// ===== DSL Contexts =====

/**
 * DSL context for enhanced package management
 * 
 * This class provides the main interface for modern package management with
 * automatic migration from legacy packages to containers and Flatpaks.
 * It supports multiple package formats and intelligent format selection.
 * 
 * ## Usage Example:
 * ```kotlin
 * enhancedPackages {
 *     autoMigrate = true
 *     migrationStrategy = MigrationStrategy.CONTAINER_FIRST
 *     
 *     system {
 *         development("dev-tools") {
 *             packages("git", "curl", "vim")
 *             export("git", "curl", "vim")
 *         }
 *     }
 *     
 *     applications {
 *         flatpak("org.mozilla.firefox")
 *         flatpak("com.visualstudio.code")
 *     }
 * }
 * ```
 */
@HorizonOSDsl
class PackagesContext {
    var autoMigrate: Boolean = true
    var migrationStrategy: MigrationStrategy = MigrationStrategy.CONTAINER_FIRST
    
    private var systemConfig: SystemPackagesConfig = SystemPackagesConfig()
    private var applicationsConfig: ApplicationPackagesConfig = ApplicationPackagesConfig()
    private var containersConfig: ContainersConfig = ContainersConfig()
    private var legacyConfig: LegacyPackagesConfig = LegacyPackagesConfig()
    
    /**
     * Configure system packages via containers
     */
    fun system(block: SystemPackagesContext.() -> Unit) {
        systemConfig = SystemPackagesContext().apply(block).toConfig()
    }
    
    /**
     * Configure user applications
     */
    fun applications(block: ApplicationPackagesContext.() -> Unit) {
        applicationsConfig = ApplicationPackagesContext().apply(block).toConfig()
    }
    
    /**
     * Configure containers for system packages
     */
    fun containers(block: ContainersContext.() -> Unit) {
        containersConfig = ContainersContext().apply(block).toConfig()
    }
    
    /**
     * Legacy package installation (backward compatibility)
     * Will be migrated to containers automatically
     */
    fun install(vararg names: String) {
        val packages = names.map { Package(it, PackageAction.INSTALL) }
        legacyConfig = legacyConfig.copy(
            packages = legacyConfig.packages + packages
        )
    }
    
    /**
     * Legacy package removal (backward compatibility)
     */
    fun remove(vararg names: String) {
        val packages = names.map { Package(it, PackageAction.REMOVE) }
        legacyConfig = legacyConfig.copy(
            packages = legacyConfig.packages + packages
        )
    }
    
    /**
     * Legacy package group (backward compatibility)
     */
    fun group(name: String, block: GroupContext.() -> Unit) {
        val group = GroupContext(name).apply(block)
        legacyConfig = legacyConfig.copy(
            packages = legacyConfig.packages + group.packages
        )
    }
    
    /**
     * Quick install - automatically chooses best format
     */
    fun quickInstall(vararg names: String) {
        for (name in names) {
            when (getRecommendedFormat(name)) {
                PackageFormat.FLATPAK -> {
                    val flatpakId = getFlatpakId(name)
                    if (flatpakId != null) {
                        applicationsConfig = applicationsConfig.copy(
                            flatpaks = applicationsConfig.flatpaks + FlatpakApplication(flatpakId)
                        )
                    }
                }
                PackageFormat.CONTAINER -> {
                    // Add to legacy packages (would be migrated to container)
                    legacyConfig = legacyConfig.copy(
                        packages = legacyConfig.packages + Package(name, PackageAction.INSTALL)
                    )
                }
                PackageFormat.APPIMAGE -> {
                    // Would need AppImage URL lookup
                }
                PackageFormat.SNAP -> {
                    // Add as snap package
                    applicationsConfig = applicationsConfig.copy(
                        snaps = applicationsConfig.snaps + Snap(name)
                    )
                }
                PackageFormat.NATIVE -> {
                    // Fall back to legacy format
                    legacyConfig = legacyConfig.copy(
                        packages = legacyConfig.packages + Package(name, PackageAction.INSTALL)
                    )
                }
            }
        }
    }
    
    fun toConfig(): PackagesConfig {
        return PackagesConfig(
            system = systemConfig,
            applications = applicationsConfig,
            containers = containersConfig,
            legacy = legacyConfig,
            autoMigrate = autoMigrate,
            migrationStrategy = migrationStrategy
        )
    }
}

/**
 * DSL context for system package management via containers
 * 
 * This class provides configuration for system packages using containers,
 * with support for development, multimedia, and gaming containers.
 * 
 * ## Usage Example:
 * ```kotlin
 * system {
 *     strategy = SystemPackageStrategy.CONTAINER
 *     defaultRuntime = ContainerRuntime.DISTROBOX
 *     
 *     development("dev-tools") {
 *         packages("git", "curl", "vim", "build-essential")
 *         export("git", "curl", "vim", "gcc", "make")
 *     }
 *     
 *     multimedia("media-tools") {
 *         packages("ffmpeg", "imagemagick")
 *         export("ffmpeg", "convert")
 *     }
 * }
 * ```
 */
@HorizonOSDsl
class SystemPackagesContext {
    var strategy: SystemPackageStrategy = SystemPackageStrategy.CONTAINER
    var defaultRuntime: ContainerRuntime = ContainerRuntime.DISTROBOX
    var autoUpdate: Boolean = true
    
    private val containers = mutableListOf<SystemContainer>()
    private val globalMounts = mutableListOf<String>()
    
    /**
     * Add a container for system packages
     */
    fun container(name: String, block: ContainerBuilder.() -> Unit) {
        val builder = ContainerBuilder(name).apply(block)
        containers.add(builder.build())
    }
    
    /**
     * Add development container with common tools
     */
    fun development(name: String = "dev-tools", block: DevContainerBuilder.() -> Unit = {}) {
        val builder = DevContainerBuilder(name).apply {
            packages("git", "curl", "wget", "vim", "build-essential")
            export("git", "curl", "wget", "vim", "gcc", "make")
            block()
        }
        containers.add(builder.build())
    }
    
    /**
     * Add multimedia container
     */
    fun multimedia(name: String = "multimedia", block: ContainerBuilder.() -> Unit = {}) {
        val builder = ContainerBuilder(name).apply {
            purpose = ContainerPurpose.MULTIMEDIA
            packages("ffmpeg", "imagemagick", "sox", "mediainfo")
            export("ffmpeg", "convert", "sox", "mediainfo")
            block()
        }
        containers.add(builder.build())
    }
    
    /**
     * Add gaming container
     */
    fun gaming(name: String = "gaming", block: ContainerBuilder.() -> Unit = {}) {
        val builder = ContainerBuilder(name).apply {
            purpose = ContainerPurpose.GAMING
            packages("steam", "lutris", "wine", "gamemode")
            export("steam", "lutris", "wine")
            block()
        }
        containers.add(builder.build())
    }
    
    /**
     * Add global mount for all containers
     */
    fun globalMount(path: String) {
        globalMounts.add(path)
    }
    
    fun toConfig(): SystemPackagesConfig {
        return SystemPackagesConfig(
            strategy = strategy,
            defaultRuntime = defaultRuntime,
            containers = containers,
            globalMounts = globalMounts,
            autoUpdate = autoUpdate
        )
    }
}

/**
 * DSL context for user application package management
 * 
 * This class provides configuration for user applications using Flatpak,
 * AppImage, and Snap packages with convenient preset methods.
 * 
 * ## Usage Example:
 * ```kotlin
 * applications {
 *     strategy = ApplicationPackageStrategy.FLATPAK
 *     autoUpdate = true
 *     userInstall = true
 *     
 *     flatpak("org.mozilla.firefox") {
 *         branch = "stable"
 *         allowNetwork()
 *         allowFilesystem()
 *     }
 *     
 *     office() // Install LibreOffice and Thunderbird
 *     development() // Install VS Code and Builder
 * }
 * ```
 */
@HorizonOSDsl
class ApplicationPackagesContext {
    var strategy: ApplicationPackageStrategy = ApplicationPackageStrategy.FLATPAK
    var autoUpdate: Boolean = true
    var userInstall: Boolean = true
    
    private val flatpaks = mutableListOf<FlatpakApplication>()
    private val appImages = mutableListOf<AppImage>()
    private val snaps = mutableListOf<Snap>()
    
    /**
     * Install Flatpak applications
     */
    fun flatpak(vararg ids: String) {
        flatpaks.addAll(ids.map { FlatpakApplication(it) })
    }
    
    /**
     * Install Flatpak with configuration
     */
    fun flatpak(id: String, block: FlatpakBuilder.() -> Unit) {
        val builder = FlatpakBuilder(id).apply(block)
        flatpaks.add(builder.build())
    }
    
    /**
     * Install AppImage
     */
    fun appImage(name: String, url: String, checksum: String? = null) {
        appImages.add(AppImage(name, url, checksum = checksum))
    }
    
    /**
     * Install Snap package
     */
    fun snap(name: String, channel: String = "stable", classic: Boolean = false) {
        snaps.add(Snap(name, channel, classic))
    }
    
    /**
     * Quick install popular applications
     */
    fun popular(vararg names: String) {
        for (name in names) {
            val flatpakId = getFlatpakId(name)
            if (flatpakId != null) {
                flatpaks.add(FlatpakApplication(flatpakId))
            }
        }
    }
    
    /**
     * Install office suite
     */
    fun office() {
        flatpaks.add(FlatpakApplication("org.libreoffice.LibreOffice"))
        flatpaks.add(FlatpakApplication("org.mozilla.Thunderbird"))
    }
    
    /**
     * Install development tools
     */
    fun development() {
        flatpaks.addAll(listOf(
            FlatpakApplication("com.visualstudio.code"),
            FlatpakApplication("org.gnome.Builder"),
            FlatpakApplication("com.github.eneshecan.WhatsAppForLinux")
        ))
    }
    
    /**
     * Install multimedia applications
     */
    fun multimedia() {
        flatpaks.addAll(listOf(
            FlatpakApplication("org.gimp.GIMP"),
            FlatpakApplication("org.audacityteam.Audacity"),
            FlatpakApplication("org.blender.Blender"),
            FlatpakApplication("org.inkscape.Inkscape")
        ))
    }
    
    /**
     * Install gaming applications
     */
    fun gaming() {
        flatpaks.addAll(listOf(
            FlatpakApplication("com.valvesoftware.Steam"),
            FlatpakApplication("net.lutris.Lutris"),
            FlatpakApplication("com.discordapp.Discord")
        ))
    }
    
    fun toConfig(): ApplicationPackagesConfig {
        return ApplicationPackagesConfig(
            strategy = strategy,
            flatpaks = flatpaks,
            appImages = appImages,
            snaps = snaps,
            autoUpdate = autoUpdate,
            userInstall = userInstall
        )
    }
}

/**
 * DSL builder for configuring Flatpak applications
 * 
 * This class provides detailed configuration for Flatpak applications including
 * version control, runtime settings, and permission management with convenience methods.
 * 
 * ## Usage Example:
 * ```kotlin
 * flatpak("org.mozilla.firefox") {
 *     version = "latest"
 *     branch = "stable"
 *     allowNetwork()
 *     allowDisplay()
 *     allowFilesystem()
 *     permission("--device=dri")
 * }
 * ```
 */
@HorizonOSDsl
class FlatpakBuilder(private val id: String) {
    var action: PackageAction = PackageAction.INSTALL
    var version: String? = null
    var branch: String = "stable"
    var runtime: String? = null
    var runtimeVersion: String? = null
    var userInstall: Boolean = true
    var autoUpdate: Boolean = true
    
    private val permissions = mutableListOf<String>()
    
    /**
     * Add permission
     */
    fun permission(perm: String) {
        permissions.add(perm)
    }
    
    /**
     * Add multiple permissions
     */
    fun permissions(vararg perms: String) {
        permissions.addAll(perms)
    }
    
    /**
     * Common permission presets
     */
    fun allowNetwork() {
        permissions.add("--share=network")
    }
    
    fun allowFilesystem() {
        permissions.add("--filesystem=home")
    }
    
    fun allowDisplay() {
        permissions.add("--share=ipc")
        permissions.add("--socket=x11")
        permissions.add("--socket=wayland")
    }
    
    fun allowAudio() {
        permissions.add("--socket=pulseaudio")
    }
    
    fun build(): FlatpakApplication {
        return FlatpakApplication(
            id = id,
            action = action,
            version = version,
            branch = branch,
            runtime = runtime,
            runtimeVersion = runtimeVersion,
            permissions = permissions,
            userInstall = userInstall,
            autoUpdate = autoUpdate
        )
    }
}

// ===== Migration and Compatibility =====

@Serializable
enum class PackageFormat {
    FLATPAK,
    CONTAINER,
    APPIMAGE,
    SNAP,
    NATIVE
}

/**
 * Get recommended format for a package
 */
fun getRecommendedFormat(packageName: String): PackageFormat {
    return when (packageName.lowercase()) {
        "firefox", "chromium", "thunderbird", "libreoffice", "gimp", "inkscape", "blender" -> PackageFormat.FLATPAK
        "git", "curl", "wget", "vim", "gcc", "make", "python3", "nodejs", "rust", "go" -> PackageFormat.CONTAINER
        "steam", "discord", "spotify", "telegram" -> PackageFormat.FLATPAK
        else -> PackageFormat.NATIVE
    }
}

/**
 * Get Flatpak ID for common applications
 */
fun getFlatpakId(packageName: String): String? {
    val packageToFlatpak = mapOf(
        "firefox" to "org.mozilla.firefox",
        "chromium" to "org.chromium.Chromium",
        "thunderbird" to "org.mozilla.Thunderbird",
        "libreoffice" to "org.libreoffice.LibreOffice",
        "gimp" to "org.gimp.GIMP",
        "inkscape" to "org.inkscape.Inkscape",
        "blender" to "org.blender.Blender",
        "audacity" to "org.audacityteam.Audacity",
        "vlc" to "org.videolan.VLC",
        "steam" to "com.valvesoftware.Steam",
        "discord" to "com.discordapp.Discord",
        "spotify" to "com.spotify.Client",
        "telegram" to "org.telegram.desktop",
        "vscode" to "com.visualstudio.code",
        "code" to "com.visualstudio.code",
        "obs" to "com.obsproject.Studio",
        "krita" to "org.kde.krita"
    )
    return packageToFlatpak[packageName.lowercase()]
}

/**
 * Get container configuration for system packages
 */
fun getContainerForPackage(packageName: String): ContainerRecommendation? {
    return when (packageName.lowercase()) {
        "git", "curl", "wget", "vim" -> ContainerRecommendation(
            container = "dev-tools",
            image = "archlinux/archlinux",
            packages = listOf(packageName)
        )
        "gcc", "make", "cmake" -> ContainerRecommendation(
            container = "build-tools",
            image = "archlinux/archlinux",
            packages = listOf("base-devel")
        )
        "python3", "pip" -> ContainerRecommendation(
            container = "python-dev",
            image = "python:3.11",
            packages = listOf("python3", "python3-pip")
        )
        "nodejs", "npm" -> ContainerRecommendation(
            container = "node-dev",
            image = "node:18",
            packages = listOf("nodejs", "npm")
        )
        "rust", "cargo" -> ContainerRecommendation(
            container = "rust-dev",
            image = "rust:1.70",
            packages = listOf("rust", "cargo")
        )
        "go" -> ContainerRecommendation(
            container = "go-dev",
            image = "golang:1.20",
            packages = listOf("go")
        )
        else -> null
    }
}

@Serializable
data class ContainerRecommendation(
    val container: String,
    val image: String,
    val packages: List<String>
)

/**
 * Migrate legacy packages to modern formats
 */
fun migratePackages(
    legacyPackages: List<Package>,
    strategy: MigrationStrategy = MigrationStrategy.CONTAINER_FIRST
): MigrationResult {
    val flatpaks = mutableListOf<FlatpakApplication>()
    val containers = mutableMapOf<String, MutableList<String>>()
    val unmigrated = mutableListOf<Package>()
    
    for (pkg in legacyPackages) {
        when (strategy) {
            MigrationStrategy.FLATPAK_FIRST -> {
                val flatpakId = getFlatpakId(pkg.name)
                if (flatpakId != null) {
                    flatpaks.add(FlatpakApplication(flatpakId))
                } else {
                    val containerRec = getContainerForPackage(pkg.name)
                    if (containerRec != null) {
                        containers.getOrPut(containerRec.container) { mutableListOf() }.add(pkg.name)
                    } else {
                        unmigrated.add(pkg)
                    }
                }
            }
            MigrationStrategy.CONTAINER_FIRST -> {
                val containerRec = getContainerForPackage(pkg.name)
                if (containerRec != null) {
                    containers.getOrPut(containerRec.container) { mutableListOf() }.add(pkg.name)
                } else {
                    val flatpakId = getFlatpakId(pkg.name)
                    if (flatpakId != null) {
                        flatpaks.add(FlatpakApplication(flatpakId))
                    } else {
                        unmigrated.add(pkg)
                    }
                }
            }
            MigrationStrategy.LEGACY_ONLY -> {
                unmigrated.add(pkg)
            }
            MigrationStrategy.AUTOMATIC -> {
                when (getRecommendedFormat(pkg.name)) {
                    PackageFormat.FLATPAK -> {
                        val flatpakId = getFlatpakId(pkg.name)
                        if (flatpakId != null) {
                            flatpaks.add(FlatpakApplication(flatpakId))
                        } else {
                            unmigrated.add(pkg)
                        }
                    }
                    PackageFormat.CONTAINER -> {
                        val containerRec = getContainerForPackage(pkg.name)
                        if (containerRec != null) {
                            containers.getOrPut(containerRec.container) { mutableListOf() }.add(pkg.name)
                        } else {
                            unmigrated.add(pkg)
                        }
                    }
                    else -> unmigrated.add(pkg)
                }
            }
            MigrationStrategy.MANUAL -> {
                unmigrated.add(pkg)
            }
        }
    }
    
    return MigrationResult(
        flatpaks = flatpaks,
        containers = containers.mapValues { it.value.toList() },
        unmigrated = unmigrated
    )
}

@Serializable
data class MigrationResult(
    val flatpaks: List<FlatpakApplication>,
    val containers: Map<String, List<String>>,
    val unmigrated: List<Package>
)

/**
 * DSL context for legacy package groups
 * 
 * This class provides backward compatibility for traditional package groups
 * with install and remove operations. Package groups will be automatically
 * migrated to modern container-based formats.
 * 
 * ## Usage Example:
 * ```kotlin
 * group("development") {
 *     install("git", "curl", "vim")
 *     install("gcc", "make", "cmake")
 * }
 * ```
 */
@HorizonOSDsl
class GroupContext(private val groupName: String) {
    internal val packages = mutableListOf<Package>()
    
    fun install(vararg names: String) {
        val pkgs = names.map { Package(it, PackageAction.INSTALL, group = groupName) }
        packages.addAll(pkgs)
    }
    
    fun remove(vararg names: String) {
        val pkgs = names.map { Package(it, PackageAction.REMOVE, group = groupName) }
        packages.addAll(pkgs)
    }
}