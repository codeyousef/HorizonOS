package org.horizonos.config.dsl

import kotlinx.serialization.Serializable

/**
 * Container Management DSL for HorizonOS
 * 
 * Provides container-based system package management using:
 * - Podman (default, rootless)
 * - Docker (optional)
 * - Toolbox (Fedora toolbox)
 * - Distrobox (better host integration)
 * 
 * Features:
 * - Container image pinning with SHA256 digests
 * - Package installation within containers
 * - Binary export to host system
 * - Persistent volume mounting
 * - Auto-start configuration
 * 
 * ## Best Practices:
 * 
 * ### 1. **Container Strategy Selection**
 * ```kotlin
 * containers {
 *     // Use PERSISTENT for frequently used development tools
 *     distrobox("dev-tools") {
 *         strategy = ContainerStrategy.PERSISTENT
 *         autoStart = true
 *         packages("git", "curl", "vim")
 *     }
 *     
 *     // Use ON_DEMAND for occasional build tools
 *     distrobox("build-tools") {
 *         strategy = ContainerStrategy.ON_DEMAND
 *         autoStart = false
 *         packages("gcc", "make", "cmake")
 *     }
 *     
 *     // Use EPHEMERAL for testing or temporary tasks
 *     distrobox("test-env") {
 *         strategy = ContainerStrategy.EPHEMERAL
 *         packages("pytest", "nodejs", "npm")
 *     }
 * }
 * ```
 * 
 * ### 2. **Image Pinning for Reproducibility**
 * ```kotlin
 * containers {
 *     distrobox("production-tools") {
 *         archlinux()
 *         // Pin to specific digest for reproducible builds
 *         digest = "sha256:abc123def456..."
 *         packages("git", "curl", "vim")
 *     }
 * }
 * ```
 * 
 * ### 3. **Purpose-Based Organization**
 * ```kotlin
 * containers {
 *     // Development containers
 *     devContainer("rust-dev") {
 *         rust("1.70")
 *         purpose = ContainerPurpose.DEVELOPMENT
 *     }
 *     
 *     // Multimedia containers
 *     distrobox("media-tools") {
 *         purpose = ContainerPurpose.MULTIMEDIA
 *         packages("ffmpeg", "imagemagick", "gimp")
 *     }
 *     
 *     // Gaming containers
 *     distrobox("gaming") {
 *         purpose = ContainerPurpose.GAMING
 *         packages("steam", "lutris", "wine")
 *     }
 * }
 * ```
 * 
 * @see [Layers] for layered architecture configuration
 * @see [SystemLayer] for system container layers
 * @see [Reproducible] for reproducible build settings
 * @see [ReproducibleConfig] for image digest pinning
 * @see [Packages] for enhanced package management
 * @see [SystemPackagesConfig] for system package management
 * @see [horizonOS] for main system configuration entry point
 */

// ===== Container Runtime Types =====

/**
 * Container runtime options for system containers
 * 
 * Each runtime has different characteristics and use cases for container management.
 */
@Serializable
enum class ContainerRuntime {
    /** Podman - Default rootless container runtime with strong security */
    PODMAN,
    
    /** Docker - Popular container runtime with broad ecosystem support */
    DOCKER,
    
    /** Toolbox - Fedora's development-focused container solution */
    TOOLBOX,
    
    /** Distrobox - Enhanced container integration with host system */
    DISTROBOX
}

/**
 * Container purpose categories for optimization and default settings
 * 
 * Each purpose provides optimized defaults for packages, performance, and configuration.
 */
@Serializable
enum class ContainerPurpose {
    /** Development tools, compilers, and programming languages */
    DEVELOPMENT,
    
    /** Gaming tools, emulators, and entertainment software */
    GAMING,
    
    /** Media creation, editing, and multimedia tools */
    MULTIMEDIA,
    
    /** Office productivity and business applications */
    OFFICE,
    
    /** Security tools, penetration testing, and forensics */
    SECURITY,
    
    /** Custom purpose with user-defined configuration */
    CUSTOM
}

/**
 * Container lifecycle management strategies
 * 
 * Determines how and when containers are started, stopped, and managed.
 */
@Serializable
enum class ContainerStrategy {
    /** Recreate container on each use - ensures clean state */
    EPHEMERAL,
    
    /** Keep container running persistently - faster access */
    PERSISTENT,
    
    /** Start container when needed, stop when idle - balanced approach */
    ON_DEMAND
}

// ===== Container Configuration =====

/**
 * Configuration for container-based system package management
 * 
 * This class defines the overall container management configuration including
 * runtime preferences, container definitions, and global settings that apply
 * to all containers in the system.
 */
@Serializable
data class ContainersConfig(
    /** Default container runtime to use when none is specified */
    val defaultRuntime: ContainerRuntime = ContainerRuntime.DISTROBOX,
    
    /** List of configured system containers */
    val containers: List<SystemContainer> = emptyList(),
    
    /** Global mount points that will be available in all containers */
    val globalMounts: List<String> = emptyList(),
    
    /** Whether containers should automatically start at system boot */
    val autoStart: Boolean = false,
    
    /** Whether to automatically clean up containers on system shutdown */
    val cleanupOnExit: Boolean = true
)

/**
 * Configuration for a system container
 * 
 * This class defines a single container that provides system packages and tools
 * while keeping the base system immutable. Containers can be configured with
 * specific runtimes, purposes, and lifecycle management strategies.
 */
@Serializable
data class SystemContainer(
    /** Unique container name within the system */
    val name: String,
    
    /** Container image name (e.g., "archlinux/archlinux", "ubuntu", "fedora") */
    val image: String,
    
    /** Container image tag (e.g., "latest", "stable", "22.04") */
    val tag: String = "latest",
    
    /** SHA256 digest for reproducible builds (e.g., "sha256:abc123...") */
    val digest: String? = null,
    
    /** Container runtime to use (Podman, Docker, Distrobox, Toolbox) */
    val runtime: ContainerRuntime = ContainerRuntime.DISTROBOX,
    
    /** Container purpose for optimization and default settings */
    val purpose: ContainerPurpose = ContainerPurpose.CUSTOM,
    
    /** Container lifecycle management strategy */
    val strategy: ContainerStrategy = ContainerStrategy.ON_DEMAND,
    
    /** List of packages to install within the container */
    val packages: List<String> = emptyList(),
    
    /** Commands to run before package installation */
    val preCommands: List<String> = emptyList(),
    
    /** Commands to run after package installation */
    val postCommands: List<String> = emptyList(),
    
    /** Whether to automatically start this container at system boot */
    val autoStart: Boolean = false,
    
    /** Binary commands to export to the host system PATH */
    val binaries: List<String> = emptyList(),
    
    /** Persistent mount points within the container */
    val persistent: List<String> = emptyList(),
    
    /** Environment variables to set in the container */
    val environment: Map<String, String> = emptyMap(),
    
    /** Port mappings in format "host:container" (e.g., "8080:80") */
    val ports: List<String> = emptyList(),
    
    /** Whether to run the container with privileged access */
    val privileged: Boolean = false,
    
    /** Container network mode (bridge, host, none) */
    val networkMode: String = "bridge",
    
    /** Custom hostname for the container */
    val hostname: String? = null,
    
    /** User to run as within the container */
    val user: String? = null,
    
    /** Working directory within the container */
    val workingDir: String? = null,
    
    /** Custom labels to apply to the container */
    val labels: Map<String, String> = emptyMap()
)

// ===== DSL Contexts =====

@HorizonOSDsl
class ContainersContext {
    var defaultRuntime: ContainerRuntime = ContainerRuntime.DISTROBOX
    var autoStart: Boolean = false
    var cleanupOnExit: Boolean = true
    internal val containers = mutableListOf<SystemContainer>()
    internal val globalMounts = mutableListOf<String>()
    
    /**
     * Create a general-purpose container
     * 
     * @param name Container name (must be unique within the configuration)
     * @param block Configuration block for the container
     */
    fun container(name: String, block: ContainerBuilder.() -> Unit) {
        val builder = ContainerBuilder(name).apply(block)
        containers.add(builder.build())
    }
    
    /**
     * Create a Toolbox container (Fedora-style)
     * 
     * @param name Container name (must be unique within the configuration)
     * @param block Configuration block for the Toolbox container
     */
    fun toolbox(name: String, block: ToolboxBuilder.() -> Unit) {
        val builder = ToolboxBuilder(name).apply(block)
        containers.add(builder.build())
    }
    
    /**
     * Create a Distrobox container (better host integration)
     * 
     * @param name Container name (must be unique within the configuration)
     * @param block Configuration block for the Distrobox container
     */
    fun distrobox(name: String, block: DistroboxBuilder.() -> Unit) {
        val builder = DistroboxBuilder(name).apply(block)
        containers.add(builder.build())
    }
    
    /**
     * Create a development container
     * 
     * @param name Container name (must be unique within the configuration)
     * @param block Configuration block for the development container
     */
    fun devContainer(name: String, block: DevContainerBuilder.() -> Unit) {
        val builder = DevContainerBuilder(name).apply(block)
        containers.add(builder.build())
    }
    
    /**
     * Add global mount that applies to all containers
     * 
     * @param path File system path to mount in all containers
     */
    fun globalMount(path: String) {
        globalMounts.add(path)
    }
    
    /**
     * Add multiple global mounts
     * 
     * @param paths File system paths to mount in all containers
     */
    fun globalMounts(vararg paths: String) {
        globalMounts.addAll(paths)
    }
    
    /**
     * Convert the container context to a configuration object
     * 
     * @return ContainersConfig object with all configured containers and settings
     */
    fun toConfig(): ContainersConfig {
        return ContainersConfig(
            defaultRuntime = defaultRuntime,
            containers = containers,
            globalMounts = globalMounts,
            autoStart = autoStart,
            cleanupOnExit = cleanupOnExit
        )
    }
}

/**
 * DSL builder for configuring containers
 * 
 * This class provides comprehensive configuration for system containers including
 * image settings, runtime options, package management, and environment setup.
 * 
 * ## Usage Example:
 * ```kotlin
 * container("dev-tools") {
 *     image = "docker.io/archlinux/archlinux"
 *     tag = "latest"
 *     runtime = ContainerRuntime.DISTROBOX
 *     purpose = ContainerPurpose.DEVELOPMENT
 *     
 *     packages("git", "curl", "vim")
 *     export("git", "curl", "vim")
 *     
 *     env("EDITOR", "vim")
 *     mount("/home/user/projects")
 * }
 * ```
 */
@HorizonOSDsl
open class ContainerBuilder(private val name: String) {
    var image: String = "docker.io/archlinux/archlinux"
    var tag: String = "latest"
    var digest: String? = null
    var runtime: ContainerRuntime = ContainerRuntime.DISTROBOX
    var purpose: ContainerPurpose = ContainerPurpose.CUSTOM
    var strategy: ContainerStrategy = ContainerStrategy.ON_DEMAND
    var autoStart: Boolean = false
    var privileged: Boolean = false
    var networkMode: String = "bridge"
    var hostname: String? = null
    var user: String? = null
    var workingDir: String? = null
    
    internal val packages = mutableListOf<String>()
    internal val preCommands = mutableListOf<String>()
    internal val postCommands = mutableListOf<String>()
    internal val exports = mutableListOf<String>()
    internal val mounts = mutableListOf<String>()
    internal val environment = mutableMapOf<String, String>()
    internal val ports = mutableListOf<String>()
    internal val labels = mutableMapOf<String, String>()
    
    /**
     * Add packages to install in the container
     * 
     * @param pkgs Package names to install
     */
    fun packages(vararg pkgs: String) {
        packages.addAll(pkgs)
    }
    
    /**
     * Add package with specific version
     */
    fun pkg(name: String, version: String? = null) {
        val packageName = if (version != null) "$name=$version" else name
        packages.add(packageName)
    }
    
    /**
     * Add command to run before package installation
     */
    fun preCommand(command: String) {
        preCommands.add(command)
    }
    
    /**
     * Add command to run after package installation
     */
    fun postCommand(command: String) {
        postCommands.add(command)
    }
    
    /**
     * Export binary to host system
     */
    fun export(vararg binaries: String) {
        exports.addAll(binaries)
    }
    
    /**
     * Add persistent mount
     */
    fun mount(path: String) {
        mounts.add(path)
    }
    
    /**
     * Add persistent mounts
     */
    fun mounts(vararg paths: String) {
        mounts.addAll(paths)
    }
    
    /**
     * Set environment variable
     */
    fun env(key: String, value: String) {
        environment[key] = value
    }
    
    /**
     * Set environment variables
     */
    fun env(vars: Map<String, String>) {
        environment.putAll(vars)
    }
    
    /**
     * Expose port
     */
    fun port(port: String) {
        ports.add(port)
    }
    
    /**
     * Expose multiple ports
     */
    fun ports(vararg portList: String) {
        ports.addAll(portList)
    }
    
    /**
     * Add label
     */
    fun label(key: String, value: String) {
        labels[key] = value
    }
    
    /**
     * Add multiple labels
     */
    fun labels(labelMap: Map<String, String>) {
        labels.putAll(labelMap)
    }
    
    fun build(): SystemContainer {
        return SystemContainer(
            name = name,
            image = image,
            tag = tag,
            digest = digest,
            runtime = runtime,
            purpose = purpose,
            strategy = strategy,
            packages = packages.toList(),
            preCommands = preCommands.toList(),
            postCommands = postCommands.toList(),
            autoStart = autoStart,
            binaries = exports.toList(),
            persistent = mounts.toList(),
            environment = environment.toMap(),
            ports = ports.toList(),
            privileged = privileged,
            networkMode = networkMode,
            hostname = hostname,
            user = user,
            workingDir = workingDir,
            labels = labels.toMap()
        )
    }
}

/**
 * DSL builder for configuring Toolbox containers
 * 
 * This class provides configuration for Fedora Toolbox containers with
 * predefined defaults for development environments.
 * 
 * ## Usage Example:
 * ```kotlin
 * toolbox("dev-env") {
 *     fedora("38")
 *     packages("git", "curl", "vim")
 *     export("git", "curl", "vim")
 * }
 * ```
 */
@HorizonOSDsl
class ToolboxBuilder(private val name: String) : ContainerBuilder(name) {
    init {
        runtime = ContainerRuntime.TOOLBOX
        image = "registry.fedoraproject.org/fedora-toolbox"
        tag = "latest"
        purpose = ContainerPurpose.DEVELOPMENT
    }
    
    /**
     * Use Fedora version
     */
    fun fedora(version: String) {
        image = "registry.fedoraproject.org/fedora-toolbox"
        tag = version
    }
    
    /**
     * Use RHEL UBI
     */
    fun rhel(version: String) {
        image = "registry.redhat.io/ubi8/ubi"
        tag = version
    }
}

/**
 * DSL builder for configuring Distrobox containers
 * 
 * This class provides configuration for Distrobox containers with better
 * host integration and support for multiple Linux distributions.
 * 
 * ## Usage Example:
 * ```kotlin
 * distrobox("arch-dev") {
 *     archlinux("latest")
 *     packages("git", "curl", "vim")
 *     export("git", "curl", "vim")
 * }
 * ```
 */
@HorizonOSDsl
class DistroboxBuilder(private val name: String) : ContainerBuilder(name) {
    init {
        runtime = ContainerRuntime.DISTROBOX
        image = "docker.io/archlinux/archlinux"
        tag = "latest"
        purpose = ContainerPurpose.DEVELOPMENT
    }
    
    /**
     * Use Arch Linux
     */
    fun archlinux(tag: String = "latest") {
        image = "docker.io/archlinux/archlinux"
        this.tag = tag
    }
    
    /**
     * Use Ubuntu
     */
    fun ubuntu(version: String = "latest") {
        image = "docker.io/ubuntu"
        tag = version
    }
    
    /**
     * Use Fedora
     */
    fun fedora(version: String = "latest") {
        image = "docker.io/fedora"
        tag = version
    }
    
    /**
     * Use Alpine
     */
    fun alpine(version: String = "latest") {
        image = "docker.io/alpine"
        tag = version
    }
    
    /**
     * Use OpenSUSE
     */
    fun opensuse(version: String = "latest") {
        image = "docker.io/opensuse/leap"
        tag = version
    }
}

/**
 * DSL builder for configuring development containers
 * 
 * This class provides specialized configuration for development containers
 * with language-specific presets and optimized settings for coding environments.
 * 
 * ## Usage Example:
 * ```kotlin
 * devContainer("rust-dev") {
 *     rust("1.70")
 *     packages("build-essential", "git")
 *     export("rustc", "cargo")
 * }
 * ```
 */
@HorizonOSDsl
class DevContainerBuilder(private val name: String) : ContainerBuilder(name) {
    init {
        runtime = ContainerRuntime.DISTROBOX
        purpose = ContainerPurpose.DEVELOPMENT
        strategy = ContainerStrategy.PERSISTENT
    }
    
    /**
     * Configure for Rust development
     */
    fun rust(version: String = "latest") {
        image = "docker.io/rust"
        tag = version
        packages("build-essential", "git", "curl")
        export("rustc", "cargo", "rustfmt", "clippy")
    }
    
    /**
     * Configure for Go development
     */
    fun golang(version: String = "latest") {
        image = "docker.io/golang"
        tag = version
        packages("build-essential", "git", "curl")
        export("go", "gofmt", "godoc")
    }
    
    /**
     * Configure for Node.js development
     */
    fun nodejs(version: String = "latest") {
        image = "docker.io/node"
        tag = version
        packages("build-essential", "git", "python3")
        export("node", "npm", "npx", "yarn")
    }
    
    /**
     * Configure for Python development
     */
    fun python(version: String = "latest") {
        image = "docker.io/python"
        tag = version
        packages("build-essential", "git", "curl")
        export("python", "pip", "python3")
    }
    
    /**
     * Configure for Java development
     */
    fun java(version: String = "17") {
        image = "docker.io/openjdk"
        tag = version
        packages("build-essential", "git", "curl")
        export("java", "javac", "jar")
    }
}

// ===== Utility Functions =====

/**
 * Generate container name with prefix
 */
fun generateContainerName(prefix: String, name: String): String {
    return "${prefix}-${name}"
}

/**
 * Validate container image reference
 */
fun validateImageReference(image: String, tag: String, digest: String?): Boolean {
    // Basic validation for image format
    if (image.isBlank() || tag.isBlank()) return false
    
    // If digest is provided, validate SHA256 format
    if (digest != null) {
        val sha256Regex = Regex("^sha256:[a-fA-F0-9]{64}$")
        return sha256Regex.matches(digest)
    }
    
    return true
}

/**
 * Get default packages for container purpose
 */
fun getDefaultPackages(purpose: ContainerPurpose): List<String> {
    return when (purpose) {
        ContainerPurpose.DEVELOPMENT -> listOf("git", "curl", "wget", "vim", "build-essential")
        ContainerPurpose.GAMING -> listOf("steam", "lutris", "wine", "gamemode")
        ContainerPurpose.MULTIMEDIA -> listOf("ffmpeg", "imagemagick", "gimp", "audacity")
        ContainerPurpose.OFFICE -> listOf("libreoffice", "thunderbird", "firefox")
        ContainerPurpose.SECURITY -> listOf("nmap", "wireshark", "metasploit", "burpsuite")
        ContainerPurpose.CUSTOM -> emptyList()
    }
}

/**
 * Get recommended runtime for container purpose
 */
fun getRecommendedRuntime(purpose: ContainerPurpose): ContainerRuntime {
    return when (purpose) {
        ContainerPurpose.DEVELOPMENT -> ContainerRuntime.DISTROBOX
        ContainerPurpose.GAMING -> ContainerRuntime.PODMAN
        ContainerPurpose.MULTIMEDIA -> ContainerRuntime.DISTROBOX
        ContainerPurpose.OFFICE -> ContainerRuntime.DISTROBOX
        ContainerPurpose.SECURITY -> ContainerRuntime.PODMAN
        ContainerPurpose.CUSTOM -> ContainerRuntime.DISTROBOX
    }
}