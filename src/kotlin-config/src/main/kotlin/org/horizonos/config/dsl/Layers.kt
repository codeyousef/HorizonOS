package org.horizonos.config.dsl

import kotlinx.serialization.Serializable

/**
 * Layer Architecture DSL for HorizonOS
 * 
 * Implements a layered system architecture:
 * - Base Layer: Minimal OSTree image with essential packages
 * - System Layers: Purpose-specific containers for system tools
 * - User Layer: Flatpak applications for user space
 * 
 * This approach provides:
 * - Clear separation of concerns
 * - Reproducible builds with image digests
 * - Flexible package management
 * - Immutable base with mutable layers
 * 
 * ## Advanced Usage Example:
 * 
 * ### **Complete Development Workstation**
 * ```kotlin
 * layers {
 *     // Minimal, reproducible base layer
 *     base {
 *         minimal = true
 *         packages("base", "linux", "systemd", "ostree")
 *         services("systemd-networkd", "systemd-resolved")
 *         
 *         // Pin to specific commit for reproducibility
 *         commit("abc123def456...")
 *     }
 *     
 *     // Development tools layer
 *     systemLayer("development", LayerPurpose.DEVELOPMENT) {
 *         strategy = LayerStrategy.PERSISTENT
 *         priority = 10
 *         enabled = true
 *         
 *         development {
 *             packages("git", "curl", "vim", "tmux")
 *             export("git", "curl", "vim", "tmux")
 *         }
 *         
 *         healthCheck {
 *             command = "git --version"
 *             interval = "30s"
 *         }
 *     }
 *     
 *     // Language-specific layers with dependencies
 *     systemLayer("rust-tools", LayerPurpose.DEVELOPMENT) {
 *         dependsOn("development")
 *         strategy = LayerStrategy.ON_DEMAND
 *         
 *         container {
 *             image = "docker.io/rust"
 *             tag = "1.70"
 *             packages("build-essential")
 *             export("rustc", "cargo")
 *         }
 *     }
 *     
 *     systemLayer("node-tools", LayerPurpose.DEVELOPMENT) {
 *         dependsOn("development")
 *         strategy = LayerStrategy.ON_DEMAND
 *         
 *         container {
 *             image = "docker.io/node"
 *             tag = "18"
 *             packages("build-essential")
 *             export("node", "npm", "yarn")
 *         }
 *     }
 *     
 *     // Multimedia layer
 *     systemLayer("multimedia", LayerPurpose.MULTIMEDIA) {
 *         strategy = LayerStrategy.ON_DEMAND
 *         
 *         multimedia {
 *             packages("ffmpeg", "imagemagick", "sox")
 *             export("ffmpeg", "convert", "sox")
 *         }
 *     }
 *     
 *     // User applications layer
 *     user {
 *         autoUpdates = true
 *         userScope = true
 *         
 *         // Development applications
 *         flatpak("com.visualstudio.code") {
 *             branch = "stable"
 *             autoUpdate = true
 *         }
 *         
 *         flatpak("org.gnome.Builder") {
 *             branch = "stable"
 *         }
 *         
 *         // Productivity applications
 *         flatpak("org.mozilla.firefox")
 *         flatpak("org.libreoffice.LibreOffice")
 *         
 *         // AppImage for specialized tools
 *         appImage("Obsidian", "https://github.com/obsidianmd/obsidian-releases/releases/download/v1.0.0/Obsidian-1.0.0.AppImage")
 *     }
 *     
 *     // Define layer startup order
 *     order("development", "rust-tools", "node-tools", "multimedia")
 *     
 *     // Global mounts for all layers
 *     globalMount("/home/user/projects")
 *     globalMount("/home/user/Documents")
 *     
 *     // Shared volumes
 *     sharedVolume("build-cache")
 *     sharedVolume("npm-cache")
 * }
 * ```
 * 
 * @see [Containers] for container-based package management
 * @see [ContainersConfig] for container configuration
 * @see [SystemContainer] for individual container settings
 * @see [Reproducible] for reproducible build settings
 * @see [ReproducibleConfig] for image digest pinning
 * @see [Packages] for enhanced package management
 * @see [FlatpakApplication] for user application packages
 * @see [horizonOS] for main system configuration entry point
 */

// ===== Layer Types =====

/**
 * Layer types in the HorizonOS architecture
 * 
 * Defines the three-tier architecture of HorizonOS system layers.
 */
@Serializable
enum class LayerType {
    /** Base layer - Immutable OSTree system foundation */
    BASE,
    
    /** System layer - Containerized system packages and tools */
    SYSTEM,
    
    /** User layer - Flatpak applications and user software */
    USER
}

/**
 * Layer purpose categories for functional organization
 * 
 * Each purpose provides optimized defaults and grouping for related functionality.
 */
@Serializable
enum class LayerPurpose {
    /** Development tools, compilers, and programming environments */
    DEVELOPMENT,
    
    /** Gaming engines, emulators, and gaming utilities */
    GAMING,
    
    /** Media creation, editing, and multimedia processing tools */
    MULTIMEDIA,
    
    /** Office productivity, document processing, and business tools */
    OFFICE,
    
    /** Security tools, penetration testing, and security analysis */
    SECURITY,
    
    /** Network tools, monitoring, and network services */
    NETWORKING,
    
    /** Custom purpose with user-defined functionality */
    CUSTOM
}

/**
 * Layer activation strategies
 * 
 * Determines when and how layers are activated and managed.
 */
@Serializable
enum class LayerStrategy {
    /** Always running - Layer is permanently active */
    ALWAYS_ON,
    
    /** Start when needed - Layer activates on demand */
    ON_DEMAND,
    
    /** Recreate on each use - Layer is rebuilt for each activation */
    EPHEMERAL
}

// ===== Layer Configuration =====

/**
 * Configuration for the layered system architecture
 * 
 * This class defines the complete three-tier architecture of HorizonOS with
 * Base, System, and User layers, along with their interaction and ordering.
 */
@Serializable
data class LayersConfig(
    /** Base layer configuration - immutable OSTree foundation */
    val base: BaseLayer,
    
    /** System layers - containerized system packages and tools */
    val system: List<SystemLayer> = emptyList(),
    
    /** User layer - Flatpak applications and user software */
    val user: UserLayer = UserLayer(),
    
    /** Explicit layer activation order (overrides default dependency resolution) */
    val layerOrder: List<String> = emptyList(),
    
    /** Mount points available to all layers */
    val globalMounts: List<String> = emptyList(),
    
    /** Shared volumes accessible across layers */
    val sharedVolumes: List<String> = emptyList()
)

@Serializable
data class BaseLayer(
    val image: String = "horizonos/base",
    val tag: String = "stable",
    val digest: String? = null,
    val minimal: Boolean = true,
    val packages: List<String> = listOf(
        "base", "linux", "systemd", "ostree", 
        "podman", "flatpak", "fish", "neovim"
    ),
    val services: List<String> = listOf(
        "systemd-networkd", "systemd-resolved",
        "podman.socket", "flatpak-system-helper"
    ),
    val ostreeRef: String = "horizonos/stable/x86_64",
    val ostreeCommit: String? = null,
    val version: String = "1.0"
)

@Serializable
data class SystemLayer(
    val name: String,
    val purpose: LayerPurpose,
    val container: SystemContainer,
    val dependencies: List<String> = emptyList(),
    val strategy: LayerStrategy = LayerStrategy.ON_DEMAND,
    val priority: Int = 50,
    val enabled: Boolean = true,
    val autoStart: Boolean = false,
    val healthCheck: HealthCheck? = null
)

@Serializable
data class UserLayer(
    val flatpaks: List<FlatpakApplication> = emptyList(),
    val appImages: List<AppImage> = emptyList(),
    val snaps: List<Snap> = emptyList(),
    val autoUpdates: Boolean = true,
    val userScope: Boolean = true
)

@Serializable
data class HealthCheck(
    val command: String,
    val interval: String = "30s",
    val timeout: String = "10s",
    val retries: Int = 3,
    val startPeriod: String = "60s"
)


// ===== DSL Contexts =====

/**
 * DSL context for configuring layered system architecture
 * 
 * This class provides configuration for HorizonOS's three-tier architecture:
 * Base (OSTree), System (Containers), and User (Flatpak) layers with
 * dependency management and shared resources.
 * 
 * ## Usage Example:
 * ```kotlin
 * layers {
 *     base {
 *         packages("base", "linux", "systemd")
 *         services("systemd-networkd", "podman.socket")
 *     }
 *     
 *     systemLayer("development", LayerPurpose.DEVELOPMENT) {
 *         development {
 *             packages("git", "curl", "vim")
 *             export("git", "curl", "vim")
 *         }
 *     }
 *     
 *     user {
 *         flatpak("org.mozilla.firefox")
 *         flatpak("com.visualstudio.code")
 *     }
 * }
 * ```
 */
@HorizonOSDsl
class LayersContext {
    private var base = BaseLayer()
    private val systemLayers = mutableListOf<SystemLayer>()
    private var userLayer = UserLayer()
    private val layerOrder = mutableListOf<String>()
    private val globalMounts = mutableListOf<String>()
    private val sharedVolumes = mutableListOf<String>()
    
    /**
     * Configure the base OSTree layer
     */
    fun base(block: BaseLayerBuilder.() -> Unit) {
        base = BaseLayerBuilder().apply(block).build()
    }
    
    /**
     * Add a system layer with a container
     */
    fun systemLayer(name: String, purpose: LayerPurpose, block: SystemLayerBuilder.() -> Unit) {
        val builder = SystemLayerBuilder(name, purpose).apply(block)
        systemLayers.add(builder.build())
    }
    
    /**
     * Configure user layer applications
     */
    fun user(block: UserLayerBuilder.() -> Unit) {
        userLayer = UserLayerBuilder().apply(block).build()
    }
    
    /**
     * Set the order of layer startup
     */
    fun order(vararg layers: String) {
        layerOrder.clear()
        layerOrder.addAll(layers)
    }
    
    /**
     * Add global mount available to all layers
     */
    fun globalMount(path: String) {
        globalMounts.add(path)
    }
    
    /**
     * Add shared volume between layers
     */
    fun sharedVolume(name: String) {
        sharedVolumes.add(name)
    }
    
    fun toConfig(): LayersConfig {
        return LayersConfig(
            base = base,
            system = systemLayers,
            user = userLayer,
            layerOrder = layerOrder,
            globalMounts = globalMounts,
            sharedVolumes = sharedVolumes
        )
    }
}

/**
 * DSL builder for configuring the base OSTree layer
 * 
 * This class provides configuration for the immutable base layer including
 * essential packages, services, and OSTree commit pinning for reproducible builds.
 * 
 * ## Usage Example:
 * ```kotlin
 * base {
 *     image = "horizonos/base"
 *     tag = "stable"
 *     minimal = true
 *     
 *     packages("base", "linux", "systemd", "ostree")
 *     services("systemd-networkd", "systemd-resolved")
 *     
 *     commit("abc123...") // Pin to specific commit
 *     digest("sha256:def456...") // Pin to specific digest
 * }
 * ```
 */
@HorizonOSDsl
class BaseLayerBuilder {
    var image: String = "horizonos/base"
    var tag: String = "stable"
    var digest: String? = null
    var minimal: Boolean = true
    var ostreeRef: String = "horizonos/stable/x86_64"
    var ostreeCommit: String? = null
    var version: String = "1.0"
    
    private val packages = mutableListOf<String>()
    private val services = mutableListOf<String>()
    
    init {
        // Default essential packages
        packages.addAll(listOf(
            "base", "linux", "systemd", "ostree", 
            "podman", "flatpak", "fish", "neovim"
        ))
        
        // Default essential services
        services.addAll(listOf(
            "systemd-networkd", "systemd-resolved",
            "podman.socket", "flatpak-system-helper"
        ))
    }
    
    /**
     * Add base packages
     */
    fun packages(vararg pkgs: String) {
        packages.addAll(pkgs)
    }
    
    /**
     * Add base services
     */
    fun services(vararg svcs: String) {
        services.addAll(svcs)
    }
    
    /**
     * Pin to specific OSTree commit
     */
    fun commit(hash: String) {
        ostreeCommit = hash
    }
    
    /**
     * Use specific image digest
     */
    fun digest(sha256: String) {
        digest = sha256
    }
    
    fun build(): BaseLayer {
        return BaseLayer(
            image = image,
            tag = tag,
            digest = digest,
            minimal = minimal,
            packages = packages.toList(),
            services = services.toList(),
            ostreeRef = ostreeRef,
            ostreeCommit = ostreeCommit,
            version = version
        )
    }
}

/**
 * DSL builder for configuring system layers
 * 
 * This class provides configuration for system layers that contain containerized
 * system packages and tools. Each layer can have dependencies, health checks,
 * and specific activation strategies.
 * 
 * ## Usage Example:
 * ```kotlin
 * systemLayer("development", LayerPurpose.DEVELOPMENT) {
 *     strategy = LayerStrategy.ON_DEMAND
 *     priority = 50
 *     enabled = true
 *     
 *     dependsOn("base-tools")
 *     
 *     development {
 *         packages("git", "curl", "vim")
 *         export("git", "curl", "vim")
 *     }
 *     
 *     healthCheck {
 *         command = "git --version"
 *         interval = "30s"
 *     }
 * }
 * ```
 */
@HorizonOSDsl
class SystemLayerBuilder(
    private val name: String,
    private val purpose: LayerPurpose
) {
    var strategy: LayerStrategy = LayerStrategy.ON_DEMAND
    var priority: Int = 50
    var enabled: Boolean = true
    var autoStart: Boolean = false
    
    private val dependencies = mutableListOf<String>()
    private var container: SystemContainer? = null
    private var healthCheck: HealthCheck? = null
    
    /**
     * Configure the container for this layer
     */
    fun container(block: ContainerBuilder.() -> Unit) {
        val builder = ContainerBuilder(name).apply {
            this.purpose = this@SystemLayerBuilder.purpose.toContainerPurpose()
            block()
        }
        container = builder.build()
    }
    
    /**
     * Add layer dependencies
     */
    fun dependsOn(vararg layers: String) {
        dependencies.addAll(layers)
    }
    
    /**
     * Configure health check
     */
    fun healthCheck(block: HealthCheckBuilder.() -> Unit) {
        healthCheck = HealthCheckBuilder().apply(block).build()
    }
    
    /**
     * Quick setup for development layer
     */
    fun development(block: DevContainerBuilder.() -> Unit) {
        val builder = DevContainerBuilder(name).apply(block)
        container = builder.build()
    }
    
    /**
     * Quick setup for gaming layer
     */
    fun gaming(block: ContainerBuilder.() -> Unit) {
        val builder = ContainerBuilder(name).apply {
            purpose = ContainerPurpose.GAMING
            packages("steam", "lutris", "wine", "gamemode", "mangohud")
            export("steam", "lutris", "wine")
            block()
        }
        container = builder.build()
    }
    
    /**
     * Quick setup for multimedia layer
     */
    fun multimedia(block: ContainerBuilder.() -> Unit) {
        val builder = ContainerBuilder(name).apply {
            purpose = ContainerPurpose.MULTIMEDIA
            packages("ffmpeg", "imagemagick", "gimp", "audacity", "blender")
            export("ffmpeg", "convert", "gimp", "audacity", "blender")
            block()
        }
        container = builder.build()
    }
    
    fun build(): SystemLayer {
        if (container == null) {
            throw IllegalStateException("System layer '$name' must have a container configuration")
        }
        
        return SystemLayer(
            name = name,
            purpose = purpose,
            container = container!!,
            dependencies = dependencies,
            strategy = strategy,
            priority = priority,
            enabled = enabled,
            autoStart = autoStart,
            healthCheck = healthCheck
        )
    }
}

/**
 * DSL builder for configuring the user layer
 * 
 * This class provides configuration for user applications using Flatpak,
 * AppImage, and Snap packages with automatic updates and user-scope installation.
 * 
 * ## Usage Example:
 * ```kotlin
 * user {
 *     autoUpdates = true
 *     userScope = true
 *     
 *     flatpak("org.mozilla.firefox") {
 *         branch = "stable"
 *         userInstall = true
 *         autoUpdate = true
 *     }
 *     
 *     appImage("Obsidian", "https://github.com/obsidianmd/obsidian-releases/releases/download/v1.0.0/Obsidian-1.0.0.AppImage")
 *     
 *     snap("discord", "stable")
 * }
 * ```
 */
@HorizonOSDsl
class UserLayerBuilder {
    var autoUpdates: Boolean = true
    var userScope: Boolean = true
    
    private val flatpaks = mutableListOf<FlatpakApplication>()
    private val appImages = mutableListOf<AppImage>()
    private val snaps = mutableListOf<Snap>()
    
    /**
     * Add Flatpak application
     */
    fun flatpak(id: String, block: FlatpakApplicationContext.() -> Unit = {}) {
        val context = FlatpakApplicationContext(id).apply(block)
        flatpaks.add(context.toApplication())
    }
    
    /**
     * Add multiple Flatpak applications
     */
    fun flatpaks(vararg ids: String) {
        ids.forEach { flatpak(it) }
    }
    
    /**
     * Add AppImage
     */
    fun appImage(name: String, url: String, checksum: String? = null, version: String? = null) {
        appImages.add(AppImage(name, url, version, checksum))
    }
    
    /**
     * Add Snap package
     */
    fun snap(name: String, channel: String = "stable", classic: Boolean = false, devmode: Boolean = false) {
        snaps.add(Snap(name, channel, classic, devmode))
    }
    
    fun build(): UserLayer {
        return UserLayer(
            flatpaks = flatpaks.toList(),
            appImages = appImages.toList(),
            snaps = snaps.toList(),
            autoUpdates = autoUpdates,
            userScope = userScope
        )
    }
}

/**
 * DSL builder for configuring health checks
 * 
 * This class provides configuration for health checks that monitor the
 * status of system layers and their containers.
 * 
 * ## Usage Example:
 * ```kotlin
 * healthCheck {
 *     command = "curl -f http://localhost:8080/health"
 *     interval = "30s"
 *     timeout = "10s"
 *     retries = 3
 *     startPeriod = "60s"
 * }
 * ```
 */
@HorizonOSDsl
class HealthCheckBuilder {
    var command: String = ""
    var interval: String = "30s"
    var timeout: String = "10s"
    var retries: Int = 3
    var startPeriod: String = "60s"
    
    fun build(): HealthCheck {
        if (command.isBlank()) {
            throw IllegalStateException("Health check command cannot be empty")
        }
        
        return HealthCheck(
            command = command,
            interval = interval,
            timeout = timeout,
            retries = retries,
            startPeriod = startPeriod
        )
    }
}

// ===== Utility Functions =====

/**
 * Get default packages for layer purpose
 */
fun getDefaultLayerPackages(purpose: LayerPurpose): List<String> {
    return when (purpose) {
        LayerPurpose.DEVELOPMENT -> listOf(
            "git", "curl", "wget", "vim", "build-essential",
            "gcc", "make", "cmake", "python3", "nodejs", "rust", "go"
        )
        LayerPurpose.GAMING -> listOf(
            "steam", "lutris", "wine", "gamemode", "mangohud",
            "vulkan-tools", "mesa-utils", "nvidia-utils"
        )
        LayerPurpose.MULTIMEDIA -> listOf(
            "ffmpeg", "imagemagick", "gimp", "audacity", "blender",
            "inkscape", "krita", "obs-studio", "vlc"
        )
        LayerPurpose.OFFICE -> listOf(
            "libreoffice", "thunderbird", "firefox", "chromium",
            "texlive", "pandoc", "zathura"
        )
        LayerPurpose.SECURITY -> listOf(
            "nmap", "wireshark", "metasploit", "burpsuite",
            "john", "hashcat", "aircrack-ng", "sqlmap"
        )
        LayerPurpose.NETWORKING -> listOf(
            "iperf3", "traceroute", "tcpdump", "netstat",
            "ss", "dig", "nslookup", "curl", "wget"
        )
        LayerPurpose.CUSTOM -> emptyList()
    }
}

/**
 * Get default Flatpaks for layer purpose
 */
fun getDefaultFlatpaks(purpose: LayerPurpose): List<String> {
    return when (purpose) {
        LayerPurpose.DEVELOPMENT -> listOf(
            "com.visualstudio.code",
            "org.gnome.Builder",
            "io.github.shiftey.Desktop"
        )
        LayerPurpose.GAMING -> listOf(
            "com.valvesoftware.Steam",
            "net.lutris.Lutris",
            "com.discordapp.Discord"
        )
        LayerPurpose.MULTIMEDIA -> listOf(
            "org.gimp.GIMP",
            "org.audacityteam.Audacity",
            "org.blender.Blender",
            "org.inkscape.Inkscape"
        )
        LayerPurpose.OFFICE -> listOf(
            "org.libreoffice.LibreOffice",
            "org.mozilla.firefox",
            "org.mozilla.Thunderbird"
        )
        LayerPurpose.SECURITY -> listOf(
            "org.wireshark.Wireshark",
            "com.github.jeromerobert.pdfarranger"
        )
        LayerPurpose.NETWORKING -> listOf(
            "org.wireshark.Wireshark",
            "com.github.phase1geo.minder"
        )
        LayerPurpose.CUSTOM -> emptyList()
    }
}

/**
 * Validate layer dependencies
 */
fun validateLayerDependencies(layers: List<SystemLayer>): List<String> {
    val errors = mutableListOf<String>()
    val layerNames = layers.map { it.name }.toSet()
    
    for (layer in layers) {
        for (dependency in layer.dependencies) {
            if (dependency !in layerNames) {
                errors.add("Layer '${layer.name}' depends on non-existent layer '$dependency'")
            }
        }
    }
    
    return errors
}

/**
 * Sort layers by priority and dependencies
 */
fun sortLayers(layers: List<SystemLayer>): List<SystemLayer> {
    val sorted = mutableListOf<SystemLayer>()
    val remaining = layers.toMutableList()
    val processed = mutableSetOf<String>()
    
    while (remaining.isNotEmpty()) {
        val readyLayers = remaining.filter { layer ->
            layer.dependencies.all { it in processed }
        }
        
        if (readyLayers.isEmpty()) {
            // Circular dependency detected
            throw IllegalStateException("Circular dependency detected in layers: ${remaining.map { it.name }}")
        }
        
        val nextLayer = readyLayers.minByOrNull { it.priority }!!
        sorted.add(nextLayer)
        processed.add(nextLayer.name)
        remaining.remove(nextLayer)
    }
    
    return sorted
}

/**
 * Convert LayerPurpose to ContainerPurpose
 */
fun LayerPurpose.toContainerPurpose(): ContainerPurpose {
    return when (this) {
        LayerPurpose.DEVELOPMENT -> ContainerPurpose.DEVELOPMENT
        LayerPurpose.GAMING -> ContainerPurpose.GAMING
        LayerPurpose.MULTIMEDIA -> ContainerPurpose.MULTIMEDIA
        LayerPurpose.OFFICE -> ContainerPurpose.OFFICE
        LayerPurpose.SECURITY -> ContainerPurpose.SECURITY
        LayerPurpose.NETWORKING -> ContainerPurpose.CUSTOM
        LayerPurpose.CUSTOM -> ContainerPurpose.CUSTOM
    }
}