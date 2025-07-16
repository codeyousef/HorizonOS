package org.horizonos.config.dsl

import kotlinx.serialization.Serializable
import org.horizonos.config.validation.ConfigurationValidator
import org.horizonos.config.dsl.graphdesktop.GraphDesktopConfig

// ===== Core DSL Classes =====

@DslMarker
annotation class HorizonOSDsl

@HorizonOSDsl
class SystemConfiguration {
    var hostname: String = "horizonos"
    var timezone: String = "UTC"
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

    fun packages(block: PackagesContext.() -> Unit) {
        PackagesContext().apply(block).also {
            packages.addAll(it.packages)
        }
    }

    fun services(block: ServicesContext.() -> Unit) {
        ServicesContext().apply(block).also {
            services.addAll(it.services)
        }
    }

    fun users(block: UsersContext.() -> Unit) {
        UsersContext().apply(block).also {
            users.addAll(it.users)
        }
    }

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
            graphDesktop = graphDesktopConfig
        )
        
        // Validate configuration before returning
        val validationResult = ConfigurationValidator.validate(config)
        validationResult.throwIfInvalid()
        
        return config
    }
}

// ===== Package Management DSL =====

@HorizonOSDsl
class PackagesContext {
    internal val packages = mutableListOf<Package>()

    fun install(vararg names: String) {
        packages.addAll(names.map { Package(it, PackageAction.INSTALL) })
    }

    fun remove(vararg names: String) {
        packages.addAll(names.map { Package(it, PackageAction.REMOVE) })
    }

    fun group(name: String, block: GroupContext.() -> Unit) {
        val group = GroupContext(name).apply(block)
        packages.addAll(group.packages)
    }
}

@HorizonOSDsl
class GroupContext(private val groupName: String) {
    internal val packages = mutableListOf<Package>()

    fun install(vararg names: String) {
        packages.addAll(names.map {
            Package(it, PackageAction.INSTALL, group = groupName)
        })
    }
}

// ===== Service Management DSL =====

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

@HorizonOSDsl
open class RepoConfig {
    var enabled: Boolean = true
    var gpgCheck: Boolean = true
    var priority: Int = 50
}

@HorizonOSDsl
class OstreeRepoConfig : RepoConfig() {
    internal val branches = mutableListOf<String>()

    fun branch(name: String) {
        branches.add(name)
    }
}

// ===== Desktop Environment DSL =====

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

@Serializable
data class CompiledConfig(
    val system: SystemConfig,
    val packages: List<Package>,
    val services: List<Service>,
    val users: List<User>,
    val repositories: List<Repository>,
    val desktop: DesktopConfig? = null,
    val automation: AutomationConfig? = null,
    val ai: AIConfig? = null,
    val network: NetworkConfig? = null,
    val boot: BootConfig? = null,
    val hardware: HardwareConfig? = null,
    val storage: StorageConfig? = null,
    val security: SecurityConfig? = null,
    val enhancedServices: EnhancedServicesConfig? = null,
    val development: DevelopmentConfig? = null,
    val environment: EnvironmentConfig? = null,
    val enhancedDesktop: EnhancedDesktopConfig? = null,
    val graphDesktop: GraphDesktopConfig? = null
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

fun horizonOS(block: SystemConfiguration.() -> Unit): CompiledConfig {
    return SystemConfiguration().apply(block).toConfig()
}
