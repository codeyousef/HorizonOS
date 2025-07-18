package org.horizonos.config.dsl

import kotlinx.serialization.Serializable
import org.horizonos.config.validation.ConfigurationValidator
import org.horizonos.config.dsl.graphdesktop.GraphDesktopConfig

/**
 * Core DSL for HorizonOS System Configuration
 * 
 * This is the foundation of the HorizonOS Kotlin DSL, providing a type-safe way to configure
 * an immutable Linux system with container-based package management. The DSL follows a
 * declarative approach where configurations are built using builder patterns and validated
 * at compile time.
 * 
 * ## Key Features:
 * - **Immutable Base System**: OSTree-based immutable root filesystem
 * - **Container-based Packages**: System tools via OCI containers (Podman/Docker/Distrobox)
 * - **Layered Architecture**: Base → System → User application layers
 * - **Reproducible Builds**: Digest-pinned images and OSTree commits
 * - **Type Safety**: Compile-time validation of configuration parameters
 * 
 * ## Basic Usage:
 * ```kotlin
 * horizonOS {
 *     hostname = "my-system"
 *     timezone = "America/New_York"
 *     
 *     containers {
 *         distrobox("dev-tools") {
 *             archlinux()
 *             packages("git", "curl", "vim")
 *             export("git", "curl", "vim")
 *         }
 *     }
 *     
 *     layers {
 *         base {
 *             packages("base", "linux", "systemd")
 *         }
 *         
 *         user {
 *             flatpak("org.mozilla.firefox")
 *             flatpak("com.visualstudio.code")
 *         }
 *     }
 * }
 * ```
 * 
 * ## Architecture:
 * The DSL is organized into specialized configuration modules:
 * - **Core** (this file): System basics, hostname, timezone, locale
 * - **Containers**: OCI container management for system packages
 * - **Layers**: Multi-layer system architecture configuration
 * - **Reproducible**: Reproducible build settings and image pinning
 * - **Packages**: Enhanced package management with migration support
 * - **Boot**: Bootloader, kernel, and initramfs configuration
 * - **Hardware**: Hardware-specific settings and drivers
 * - **Security**: Security policies, firewall, and compliance
 * - **Network**: Network interfaces, services, and connectivity
 * - **Services**: System services and daemon management
 * - **AI**: Local LLM integration and AI services
 * 
 * ## Best Practices:
 * 
 * ### 1. **Immutable Infrastructure**
 * ```kotlin
 * horizonOS {
 *     // Use containers for system packages to maintain immutability
 *     containers {
 *         distrobox("dev-tools") {
 *             archlinux()
 *             packages("git", "curl", "vim")
 *             export("git", "curl", "vim")
 *         }
 *     }
 *     
 *     // Use layers for clear separation of concerns
 *     layers {
 *         base {
 *             minimal = true
 *             packages("base", "linux", "systemd")
 *         }
 *         
 *         user {
 *             flatpak("org.mozilla.firefox")
 *             flatpak("com.visualstudio.code")
 *         }
 *     }
 * }
 * ```
 * 
 * ### 2. **Reproducible Builds**
 * ```kotlin
 * horizonOS {
 *     // Pin container images for reproducibility
 *     containers {
 *         distrobox("dev-tools") {
 *             archlinux()
 *             digest = "sha256:abc123..." // Pin specific digest
 *             packages("git", "curl", "vim")
 *         }
 *     }
 *     
 *     // Enable reproducible build mode
 *     reproducible {
 *         enabled = true
 *         strictMode = true
 *         verifyDigests = true
 *     }
 * }
 * ```
 * 
 * ### 3. **Security Hardening**
 * ```kotlin
 * horizonOS {
 *     security {
 *         enabled = true
 *         
 *         firewall {
 *             enabled = true
 *             defaultPolicy = FirewallPolicy.DROP
 *             
 *             // Only allow necessary ports
 *             rule {
 *                 port = 22
 *                 protocol = "tcp"
 *                 action = FirewallAction.ACCEPT
 *                 comment = "SSH - restrict to specific IPs in production"
 *             }
 *         }
 *         
 *         ssh {
 *             enabled = true
 *             passwordAuth = false  // Use key-based auth only
 *             rootLogin = RootLoginPolicy.PROHIBIT_PASSWORD
 *             maxAuthTries = 3
 *         }
 *     }
 * }
 * ```
 * 
 * ### 4. **Performance Optimization**
 * ```kotlin
 * horizonOS {
 *     // Use appropriate container strategies
 *     containers {
 *         distrobox("dev-tools") {
 *             strategy = ContainerStrategy.PERSISTENT  // For frequent use
 *             autoStart = true
 *         }
 *         
 *         distrobox("build-tools") {
 *             strategy = ContainerStrategy.ON_DEMAND  // For occasional use
 *             autoStart = false
 *         }
 *     }
 *     
 *     // Optimize hardware settings
 *     hardware {
 *         gpu {
 *             vendor = GPUVendor.NVIDIA
 *             driver = "nvidia"  // Use proprietary drivers for performance
 *             powerManagement = true
 *         }
 *         
 *         power {
 *             profile = PowerProfile.PERFORMANCE
 *         }
 *     }
 * }
 * ```
 * 
 * ### 5. **Development Environment Setup**
 * ```kotlin
 * horizonOS {
 *     // Create language-specific containers
 *     containers {
 *         devContainer("rust-dev") {
 *             rust("1.70")
 *             packages("build-essential", "git")
 *             export("rustc", "cargo")
 *             mount("/home/user/projects")
 *         }
 *         
 *         devContainer("node-dev") {
 *             nodejs("18")
 *             packages("build-essential", "git")
 *             export("node", "npm", "yarn")
 *             mount("/home/user/projects")
 *         }
 *     }
 *     
 *     // Use enhanced package management
 *     enhancedPackages {
 *         autoMigrate = true
 *         migrationStrategy = MigrationStrategy.CONTAINER_FIRST
 *         
 *         applications {
 *             development()  // Install common dev tools
 *             office()       // Install office suite
 *         }
 *     }
 * }
 * ```
 * 
 * ### 6. **Network Configuration**
 * ```kotlin
 * horizonOS {
 *     network {
 *         // Configure primary interface
 *         interface("eth0") {
 *             type = InterfaceType.ETHERNET
 *             dhcp = true
 *             
 *             // Fallback to static IP
 *             staticIP {
 *                 address = "192.168.1.100"
 *                 netmask = "255.255.255.0"
 *                 gateway = "192.168.1.1"
 *             }
 *         }
 *         
 *         // DNS configuration
 *         dns {
 *             servers = listOf("1.1.1.1", "8.8.8.8")
 *             dnssec = true
 *             dnsOverHttps = true
 *         }
 *     }
 * }
 * ```
 * 
 * ### 7. **Error Handling and Validation**
 * ```kotlin
 * try {
 *     val config = horizonOS {
 *         hostname = "production-server"
 *         timezone = "UTC"
 *         
 *         // Configuration here...
 *     }
 *     
 *     // Configuration is automatically validated
 *     println("Configuration validated successfully")
 *     
 * } catch (e: ConfigurationException) {
 *     println("Configuration error: ${e.message}")
 *     // Handle configuration errors
 * }
 * ```
 * 
 * @since 1.0
 * @see [Containers] for container-based package management
 * @see [Layers] for layered architecture configuration
 * @see [Reproducible] for reproducible build settings
 * @see [Packages] for enhanced package management
 * @see [Security] for security policies and configurations
 * @see [Network] for network interfaces and services
 * @see [Hardware] for hardware-specific settings
 * @see [Services] for system services and daemon management
 * @see [Boot] for bootloader and kernel configuration
 * @see [Storage] for filesystem and storage configuration
 */

// ===== Core DSL Classes =====

/**
 * Annotation marking the HorizonOS DSL scope
 * 
 * This annotation ensures DSL functions are only available within the appropriate context
 * and prevents accidental mixing of DSL scopes, providing better type safety and IDE support.
 */
@DslMarker
annotation class HorizonOSDsl

/**
 * Main system configuration builder for HorizonOS
 * 
 * This class provides the root DSL context for configuring a HorizonOS system. It manages
 * all system-wide settings including basic system properties, package management, services,
 * and specialized configuration modules.
 * 
 * ## Properties:
 * - **hostname**: System hostname (must be valid DNS name)
 * - **timezone**: System timezone in IANA format (e.g., "America/New_York")
 * - **locale**: System locale in standard format (e.g., "en_US.UTF-8")
 * 
 * ## Configuration Modules:
 * Each specialized area of system configuration is handled by a dedicated module:
 * - Boot configuration (bootloader, kernel, initramfs)
 * - Hardware configuration (drivers, power management)
 * - Security configuration (firewall, policies, compliance)
 * - Network configuration (interfaces, services)
 * - Container management (OCI containers for system packages)
 * - Layer architecture (base/system/user separation)
 * 
 * @see [horizonOS] for the main entry point function
 */
@HorizonOSDsl
class SystemConfiguration {
    /** System hostname - must be a valid DNS name */
    var hostname: String = "horizonos"
    
    /** System timezone in IANA format (e.g., "America/New_York", "UTC") */
    var timezone: String = "UTC"
    
    /** System locale in standard format (e.g., "en_US.UTF-8") */
    var locale: String = "en_US.UTF-8"

    private val packages = mutableListOf<Package>()
    private val services = mutableListOf<Service>()
    private val users = mutableListOf<User>()
    private val repositories = mutableListOf<Repository>()
    private var desktopConfig: DesktopConfig? = null
    private var automationConfig: AutomationConfig? = null
    var aiConfig: AIConfig? = null
    var networkConfig: NetworkConfig? = null
    var bootConfig: BootConfig? = null
    var hardwareConfig: HardwareConfig? = null
    var storageConfig: StorageConfig? = null
    var securityConfig: SecurityConfig? = null
    var enhancedServicesConfig: EnhancedServicesConfig? = null
    var developmentConfig: DevelopmentConfig? = null
    var environmentConfig: EnvironmentConfig? = null
    var enhancedDesktopConfig: EnhancedDesktopConfig? = null
    var graphDesktopConfig: GraphDesktopConfig? = null
    var containersConfig: ContainersConfig? = null
    var layersConfig: LayersConfig? = null
    var reproducibleConfig: ReproducibleConfig? = null
    var packagesConfig: PackagesConfig? = null

    /**
     * Configure system packages using the legacy package management DSL
     * 
     * This function provides backward compatibility with the traditional package
     * management approach. For new configurations, consider using the enhanced
     * package management with containers and layers.
     * 
     * @param block Configuration block for package management
     * @see [enhancedPackages] for modern container-based package management
     * @see [containers] for container-based system packages
     * @see [layers] for layered architecture package management
     */
    fun packages(block: PackagesContext.() -> Unit) {
        val context = PackagesContext().apply(block)
        val config = context.toConfig()
        packages.addAll(config.legacy.packages)
    }

    /**
     * Configure system services and daemons
     * 
     * This function configures traditional system services. For modern service
     * management with containers and advanced features, use enhancedServices.
     * 
     * @param block Configuration block for service management
     * @see [enhancedServices] for modern service management with containers
     */
    fun services(block: ServicesContext.() -> Unit) {
        ServicesContext().apply(block).also {
            services.addAll(it.services)
        }
    }

    /**
     * Configure system users and groups
     * 
     * This function manages system user accounts, groups, and their permissions.
     * Users created here will be available system-wide across all containers and layers.
     * 
     * @param block Configuration block for user management
     */
    fun users(block: UsersContext.() -> Unit) {
        UsersContext().apply(block).also {
            users.addAll(it.users)
        }
    }

    /**
     * Configure package repositories
     * 
     * This function manages package repositories for the system, including
     * repository URLs, GPG keys, and priorities.
     * 
     * @param block Configuration block for repository management
     */
    fun repositories(block: RepositoriesContext.() -> Unit) {
        RepositoriesContext().apply(block).also {
            repositories.addAll(it.repositories)
        }
    }
    
    fun desktop(block: DesktopContext.() -> Unit) {
        desktopConfig = DesktopContext().apply(block).toConfig()
    }
    
    fun automation(block: AutomationContext.() -> Unit) {
        automationConfig = AutomationContext().apply(block).toConfig()
    }
    
    fun ai(block: AIContext.() -> Unit) {
        aiConfig = AIContext().apply(block).toConfig()
    }
    
    fun network(block: NetworkContext.() -> Unit) {
        networkConfig = NetworkContext().apply(block).toConfig()
    }
    
    fun boot(block: BootContext.() -> Unit) {
        bootConfig = BootContext().apply(block).toConfig()
    }
    
    fun hardware(block: HardwareContext.() -> Unit) {
        hardwareConfig = HardwareContext().apply(block).toConfig()
    }
    
    fun storage(block: StorageContext.() -> Unit) {
        storageConfig = StorageContext().apply(block).toConfig()
    }
    
    fun security(block: SecurityContext.() -> Unit) {
        securityConfig = SecurityContext().apply(block).toConfig()
    }
    
    fun enhancedServices(block: EnhancedServicesContext.() -> Unit) {
        enhancedServicesConfig = EnhancedServicesContext().apply(block).toConfig()
    }
    
    fun development(block: DevelopmentContext.() -> Unit) {
        developmentConfig = DevelopmentContext().apply(block).toConfig()
    }
    
    fun environment(block: EnvironmentContext.() -> Unit) {
        environmentConfig = EnvironmentContext().apply(block).toConfig()
    }
    
    fun enhancedDesktop(block: EnhancedDesktopContext.() -> Unit) {
        enhancedDesktopConfig = EnhancedDesktopContext().apply(block).toConfig()
    }
    
    fun graphDesktop(block: GraphDesktopContext.() -> Unit) {
        graphDesktopConfig = GraphDesktopContext().apply(block).toConfig()
    }
    
    /**
     * Configure container-based system packages
     * 
     * This function enables container-based package management using OCI containers
     * (Podman, Docker, Distrobox, Toolbox) for system packages. This is the modern
     * approach for managing system tools while keeping the base system immutable.
     * 
     * @param block Configuration block for container management
     * @see [ContainersConfig] for detailed container configuration options
     * @see [SystemContainer] for individual container settings
     * @see [layers] for layered architecture configuration
     * @see [enhancedPackages] for enhanced package management
     * @see [packages] for legacy package management
     * @see [reproducible] for reproducible container builds
     */
    fun containers(block: ContainersContext.() -> Unit) {
        containersConfig = ContainersContext().apply(block).toConfig()
    }
    
    /**
     * Configure layered system architecture
     * 
     * This function enables the layered architecture approach where the system is
     * divided into Base (OSTree), System (Containers), and User (Flatpak) layers.
     * This provides clear separation of concerns and enables flexible package management.
     * 
     * @param block Configuration block for layer management
     * @see [LayersConfig] for detailed layer configuration options
     * @see [BaseLayer] for base OSTree layer configuration
     * @see [SystemLayer] for system container layers
     * @see [UserLayer] for user application layers
     * @see [containers] for container-based system packages
     * @see [reproducible] for reproducible build configuration
     */
    fun layers(block: LayersContext.() -> Unit) {
        layersConfig = LayersContext().apply(block).toConfig()
    }
    
    /**
     * Configure reproducible build settings
     * 
     * This function enables reproducible builds by pinning container image digests,
     * OSTree commits, and Flatpak commits. This ensures that builds are deterministic
     * and can be reproduced exactly across different environments.
     * 
     * @param block Configuration block for reproducible build settings
     * @see [ReproducibleConfig] for detailed reproducible build options
     * @see [ImageDigest] for container image digest pinning
     * @see [OSTreeCommit] for OSTree commit pinning
     * @see [containers] for container image digest pinning
     * @see [layers] for layer-based reproducible builds
     */
    fun reproducible(block: ReproducibleContext.() -> Unit) {
        reproducibleConfig = ReproducibleContext().apply(block).toConfig()
    }
    
    /**
     * Configure enhanced package management
     * 
     * This function provides the modern package management approach with automatic
     * migration from legacy packages to containers and Flatpaks. It supports
     * container-based system packages and Flatpak user applications with
     * intelligent package format selection.
     * 
     * @param block Configuration block for enhanced package management
     * @see [PackagesConfig] for detailed package management options
     * @see [SystemPackagesConfig] for system package configuration
     * @see [ApplicationPackagesConfig] for user application packages
     * @see [FlatpakApplication] for Flatpak application configuration
     * @see [containers] for container-based system packages
     * @see [layers] for layered package architecture
     * @see [packages] for legacy package management
     */
    fun enhancedPackages(block: PackagesContext.() -> Unit) {
        packagesConfig = PackagesContext().apply(block).toConfig()
    }

    /**
     * Convert the DSL configuration to a compiled configuration object
     * 
     * This method transforms the DSL configuration into a compiled configuration
     * that can be validated, serialized, and used for system deployment.
     * 
     * @return Compiled configuration object ready for validation and deployment
     */
    fun toConfig(): CompiledConfig {
        val config = CompiledConfig(
            system = SystemConfig(hostname, timezone, locale),
            packages = packages,
            services = services,
            users = users,
            repositories = repositories,
            desktop = desktopConfig,
            automation = automationConfig,
            ai = aiConfig,
            network = networkConfig,
            boot = bootConfig,
            hardware = hardwareConfig,
            storage = storageConfig,
            security = securityConfig,
            enhancedServices = enhancedServicesConfig,
            development = developmentConfig,
            environment = environmentConfig,
            enhancedDesktop = enhancedDesktopConfig,
            graphDesktop = graphDesktopConfig,
            containers = containersConfig,
            layers = layersConfig,
            reproducible = reproducibleConfig,
            enhancedPackages = packagesConfig
        )
        
        // Validate configuration before returning
        val validationResult = ConfigurationValidator.validate(config)
        validationResult.throwIfInvalid()
        
        return config
    }
}

// ===== Package Management DSL =====

/**
 * DSL context for configuring a Flatpak application
 * 
 * This class provides a builder pattern for configuring Flatpak applications
 * with version control, runtime settings, and permission management.
 * 
 * ## Usage Example:
 * ```kotlin
 * flatpak("org.mozilla.firefox") {
 *     version = "latest"
 *     branch = "stable"
 *     permissions("--share=network", "--socket=x11")
 *     userInstall = true
 *     autoUpdate = true
 * }
 * ```
 */
@HorizonOSDsl
class FlatpakApplicationContext(private val id: String) {
    var action: PackageAction = PackageAction.INSTALL
    var version: String? = null
    var branch: String = "stable"
    var runtime: String? = null
    var runtimeVersion: String? = null
    var userInstall: Boolean = true
    var autoUpdate: Boolean = true
    
    private val permissions = mutableListOf<String>()
    
    fun permission(perm: String) {
        permissions.add(perm)
    }
    
    fun permissions(vararg perms: String) {
        permissions.addAll(perms)
    }
    
    fun toApplication(): FlatpakApplication {
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

// ===== Service Management DSL =====

/**
 * DSL context for configuring system services
 * 
 * This class provides methods to enable and disable system services
 * with configuration options for service behavior and environment.
 * 
 * ## Usage Example:
 * ```kotlin
 * services {
 *     enable("sshd") {
 *         autoRestart = true
 *         env("SSH_PORT", "22")
 *     }
 *     disable("bluetooth")
 * }
 * ```
 */
@HorizonOSDsl
class ServicesContext {
    internal val services = mutableListOf<Service>()

    fun enable(name: String, block: ServiceConfig.() -> Unit = {}) {
        val config = ServiceConfig().apply(block)
        services.add(Service(name, enabled = true, config = config))
    }

    fun disable(name: String) {
        services.add(Service(name, enabled = false))
    }
}

/**
 * Configuration builder for individual service settings
 * 
 * This class defines service-specific configuration including restart behavior
 * and environment variables for service execution.
 */
@Serializable
@HorizonOSDsl
class ServiceConfig {
    var autoRestart: Boolean = true
    var restartOnFailure: Boolean = true
    val environment = mutableMapOf<String, String>()

    fun env(key: String, value: String) {
        environment[key] = value
    }
}

// ===== User Management DSL =====

/**
 * DSL context for configuring system users
 * 
 * This class provides methods to create and configure user accounts
 * with specific UIDs, shells, groups, and home directories.
 * 
 * ## Usage Example:
 * ```kotlin
 * users {
 *     user("developer") {
 *         uid = 1000
 *         shell = "/usr/bin/fish"
 *         groups("wheel", "docker", "users")
 *         homeDir = "/home/developer"
 *     }
 * }
 * ```
 */
@HorizonOSDsl
class UsersContext {
    internal val users = mutableListOf<User>()

    fun user(name: String, block: UserConfig.() -> Unit) {
        val config = UserConfig().apply(block)
        users.add(User(
            name = name,
            uid = config.uid,
            shell = config.shell,
            groups = config.groups.toList(),
            homeDir = config.homeDir ?: "/home/$name"
        ))
    }
}

/**
 * Configuration builder for individual user settings
 * 
 * This class defines user-specific configuration including UID, shell,
 * home directory, and group membership.
 */
@HorizonOSDsl
class UserConfig {
    var uid: Int? = null
    var shell: String = "/usr/bin/fish"
    var homeDir: String? = null
    internal val groups = mutableSetOf<String>()

    fun groups(vararg names: String) {
        groups.addAll(names)
    }
}

// ===== Repository Management DSL =====

/**
 * DSL context for configuring package repositories
 * 
 * This class provides methods to add package repositories and OSTree repositories
 * with configuration for GPG verification, priority, and branch management.
 * 
 * ## Usage Example:
 * ```kotlin
 * repositories {
 *     add("extra", "https://mirrors.kernel.org/archlinux/extra/os/x86_64") {
 *         enabled = true
 *         gpgCheck = true
 *         priority = 50
 *     }
 *     
 *     ostree("horizonos", "https://repo.horizonos.org/ostree") {
 *         branch("stable/x86_64")
 *         branch("testing/x86_64")
 *     }
 * }
 * ```
 */
@HorizonOSDsl
class RepositoriesContext {
    internal val repositories = mutableListOf<Repository>()

    fun add(name: String, url: String, block: RepoConfig.() -> Unit = {}) {
        val config = RepoConfig().apply(block)
        repositories.add(PackageRepository(
            name = name,
            url = url,
            enabled = config.enabled,
            gpgCheck = config.gpgCheck,
            priority = config.priority
        ))
    }

    fun ostree(name: String, url: String, block: OstreeRepoConfig.() -> Unit = {}) {
        val config = OstreeRepoConfig().apply(block)
        repositories.add(OstreeRepository(
            name = name,
            url = url,
            enabled = config.enabled,
            gpgCheck = config.gpgCheck,
            priority = config.priority,
            branches = config.branches.toList()
        ))
    }
}

/**
 * Base configuration for package repositories
 * 
 * This class defines common repository settings including enabled status,
 * GPG verification, and priority for package resolution.
 */
@HorizonOSDsl
open class RepoConfig {
    var enabled: Boolean = true
    var gpgCheck: Boolean = true
    var priority: Int = 50
}

/**
 * Configuration builder for OSTree repositories
 * 
 * This class extends RepoConfig with OSTree-specific settings including
 * branch management for tracking multiple OSTree branches.
 */
@HorizonOSDsl
class OstreeRepoConfig : RepoConfig() {
    internal val branches = mutableListOf<String>()

    fun branch(name: String) {
        branches.add(name)
    }
}

// ===== Desktop Environment DSL =====

/**
 * DSL context for configuring desktop environments
 * 
 * This class provides configuration for various desktop environments including
 * Hyprland, Plasma, GNOME, and XFCE with auto-login and environment-specific settings.
 * 
 * ## Usage Example:
 * ```kotlin
 * desktop {
 *     environment = DesktopEnvironment.HYPRLAND
 *     autoLogin = true
 *     autoLoginUser = "user"
 *     
 *     hyprland {
 *         theme = "breeze-dark"
 *         animations = true
 *         kdeIntegration = true
 *     }
 * }
 * ```
 */
@HorizonOSDsl
class DesktopContext {
    var environment: DesktopEnvironment = DesktopEnvironment.HYPRLAND
    var autoLogin: Boolean = false
    var autoLoginUser: String? = null
    
    private var hyprlandConfig: HyprlandConfig? = null
    private var plasmaConfig: PlasmaConfig? = null
    
    fun hyprland(block: HyprlandContext.() -> Unit) {
        hyprlandConfig = HyprlandContext().apply(block).toConfig()
    }
    
    fun plasma(block: PlasmaContext.() -> Unit) {
        plasmaConfig = PlasmaContext().apply(block).toConfig()
    }
    
    fun toConfig(): DesktopConfig {
        return DesktopConfig(
            environment = environment,
            autoLogin = autoLogin,
            autoLoginUser = autoLoginUser,
            hyprlandConfig = hyprlandConfig,
            plasmaConfig = plasmaConfig
        )
    }
}

/**
 * DSL context for configuring Hyprland window manager
 * 
 * This class provides configuration for Hyprland-specific settings including
 * theming, animations, window gaps, and KDE integration.
 * 
 * ## Usage Example:
 * ```kotlin
 * hyprland {
 *     theme = "breeze-dark"
 *     animations = true
 *     gaps = 10
 *     borderSize = 2
 *     kdeIntegration = true
 *     personalityMode = PersonalityMode.KDE
 * }
 * ```
 */
@HorizonOSDsl
class HyprlandContext {
    var theme: String = "breeze-dark"
    var animations: Boolean = true
    var gaps: Int = 10
    var borderSize: Int = 2
    var kdeIntegration: Boolean = true
    var personalityMode: PersonalityMode = PersonalityMode.KDE
    
    fun toConfig(): HyprlandConfig {
        return HyprlandConfig(
            theme = theme,
            animations = animations,
            gaps = gaps,
            borderSize = borderSize,
            kdeIntegration = kdeIntegration,
            personalityMode = personalityMode
        )
    }
}

/**
 * DSL context for configuring KDE Plasma desktop
 * 
 * This class provides configuration for Plasma-specific settings including
 * themes, look-and-feel packages, and desktop widgets.
 * 
 * ## Usage Example:
 * ```kotlin
 * plasma {
 *     theme = "breeze-dark"
 *     lookAndFeel = "org.kde.breezedark.desktop"
 *     widgets("org.kde.plasma.systemmonitor", "org.kde.plasma.weather")
 * }
 * ```
 */
@HorizonOSDsl
class PlasmaContext {
    var theme: String = "breeze-dark"
    var lookAndFeel: String = "org.kde.breezedark.desktop"
    var widgets: List<String> = emptyList()
    
    fun widgets(vararg names: String) {
        widgets = names.toList()
    }
    
    fun toConfig(): PlasmaConfig {
        return PlasmaConfig(
            theme = theme,
            lookAndFeel = lookAndFeel,
            widgets = widgets
        )
    }
}

// ===== Data Classes =====

/**
 * Complete compiled configuration for a HorizonOS system
 * 
 * This class represents the final compiled configuration that contains all
 * system settings, packages, services, and specialized configurations.
 * It is the result of processing the DSL configuration through the compiler.
 */
@Serializable
data class CompiledConfig(
    /** Core system configuration (hostname, timezone, locale) */
    val system: SystemConfig,
    
    /** Legacy packages to be installed or removed */
    val packages: List<Package>,
    
    /** System services and daemon configurations */
    val services: List<Service>,
    
    /** User accounts and group configurations */
    val users: List<User>,
    
    /** Package repositories and remote sources */
    val repositories: List<Repository>,
    
    /** Desktop environment configuration */
    val desktop: DesktopConfig? = null,
    
    /** Automation and scripting configuration */
    val automation: AutomationConfig? = null,
    
    /** AI and LLM integration configuration */
    val ai: AIConfig? = null,
    
    /** Network and connectivity configuration */
    val network: NetworkConfig? = null,
    
    /** Boot and kernel configuration */
    val boot: BootConfig? = null,
    
    /** Hardware and driver configuration */
    val hardware: HardwareConfig? = null,
    
    /** Storage and filesystem configuration */
    val storage: StorageConfig? = null,
    
    /** Security and access control configuration */
    val security: SecurityConfig? = null,
    
    /** Enhanced service management configuration */
    val enhancedServices: EnhancedServicesConfig? = null,
    
    /** Development environment configuration */
    val development: DevelopmentConfig? = null,
    
    /** Environment and shell configuration */
    val environment: EnvironmentConfig? = null,
    
    /** Enhanced desktop features configuration */
    val enhancedDesktop: EnhancedDesktopConfig? = null,
    
    /** Graph-based desktop configuration */
    val graphDesktop: GraphDesktopConfig? = null,
    
    /** Container-based package management configuration */
    val containers: ContainersConfig? = null,
    
    /** Layered architecture configuration */
    val layers: LayersConfig? = null,
    
    /** Reproducible build configuration */
    val reproducible: ReproducibleConfig? = null,
    
    /** Enhanced package management configuration */
    val enhancedPackages: PackagesConfig? = null
)

@Serializable
data class SystemConfig(
    val hostname: String,
    val timezone: String,
    val locale: String
)

@Serializable
data class Package(
    val name: String,
    val action: PackageAction,
    val group: String? = null
)

@Serializable
enum class PackageAction {
    INSTALL, REMOVE
}

@Serializable
data class Service(
    val name: String,
    val enabled: Boolean,
    val config: ServiceConfig? = null
)

@Serializable
data class User(
    val name: String,
    val uid: Int? = null,
    val shell: String,
    val groups: List<String>,
    val homeDir: String
)

// ===== Repository Classes =====

@Serializable
sealed class Repository {
    abstract val name: String
    abstract val url: String
    abstract val enabled: Boolean
    abstract val gpgCheck: Boolean
    abstract val priority: Int
}

@Serializable
data class PackageRepository(
    override val name: String,
    override val url: String,
    override val enabled: Boolean = true,
    override val gpgCheck: Boolean = true,
    override val priority: Int = 50
) : Repository()

@Serializable
data class OstreeRepository(
    override val name: String,
    override val url: String,
    override val enabled: Boolean = true,
    override val gpgCheck: Boolean = true,
    override val priority: Int = 50,
    val branches: List<String> = emptyList()
) : Repository()

// ===== Desktop Environment Classes =====

@Serializable
enum class DesktopEnvironment {
    PLASMA,
    HYPRLAND,
    GNOME,
    XFCE,
    GRAPH
}

@Serializable
enum class PersonalityMode {
    KDE,
    GNOME,
    MACOS,
    WINDOWS11,
    I3,
    CUSTOM
}

@Serializable
data class DesktopConfig(
    val environment: DesktopEnvironment,
    val autoLogin: Boolean = false,
    val autoLoginUser: String? = null,
    val hyprlandConfig: HyprlandConfig? = null,
    val plasmaConfig: PlasmaConfig? = null
)

@Serializable
data class HyprlandConfig(
    val theme: String,
    val animations: Boolean,
    val gaps: Int,
    val borderSize: Int,
    val kdeIntegration: Boolean,
    val personalityMode: PersonalityMode
)

@Serializable
data class PlasmaConfig(
    val theme: String,
    val lookAndFeel: String,
    val widgets: List<String>
)

// ===== DSL Entry Point =====

/**
 * Main entry point for HorizonOS system configuration
 * 
 * This function creates a complete HorizonOS system configuration using the type-safe
 * Kotlin DSL. It provides access to all configuration modules including containers,
 * layers, security, networking, and hardware configuration.
 * 
 * ## Example Usage:
 * ```kotlin
 * val config = horizonOS {
 *     hostname = "my-horizonos-system"
 *     timezone = "America/New_York"
 *     
 *     // Modern container-based package management
 *     containers {
 *         distrobox("development") {
 *             archlinux()
 *             packages("git", "curl", "vim")
 *             export("git", "curl", "vim")
 *         }
 *     }
 *     
 *     // Layered architecture configuration
 *     layers {
 *         base {
 *             packages("base", "linux", "systemd")
 *         }
 *         
 *         user {
 *             flatpak("org.mozilla.firefox")
 *             flatpak("com.visualstudio.code")
 *         }
 *     }
 *     
 *     // Security configuration
 *     security {
 *         firewall {
 *             enabled = true
 *             rule {
 *                 port = 22
 *                 protocol = "tcp"
 *                 action = FirewallAction.ACCEPT
 *             }
 *         }
 *     }
 *     
 *     // Hardware configuration
 *     hardware {
 *         gpu {
 *             vendor = GPUVendor.NVIDIA
 *             driver = "nvidia"
 *         }
 *     }
 * }
 * ```
 * 
 * @param block Configuration block that defines the system configuration
 * @return Compiled configuration object ready for validation and deployment
 * @see [SystemConfiguration] for available configuration options
 * @see [ContainersConfig] for container-based package management
 * @see [LayersConfig] for layered architecture configuration
 * @see [SecurityConfig] for security policies and firewall rules
 * @see [NetworkConfig] for network interfaces and connectivity
 * @see [HardwareConfig] for hardware drivers and device settings
 * @see [BootConfig] for bootloader and kernel configuration
 * @see [StorageConfig] for filesystem and storage management
 */
fun horizonOS(block: SystemConfiguration.() -> Unit): CompiledConfig {
    return SystemConfiguration().apply(block).toConfig()
}
