// src/kotlin-config/src/main/kotlin/org/horizonos/config/dsl/Core.kt
package org.horizonos.config.dsl

import kotlinx.serialization.Serializable

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
    
    fun toConfig(): CompiledConfig {
        return CompiledConfig(
            system = SystemConfig(hostname, timezone, locale),
            packages = packages,
            services = services,
            users = users,
            repositories = repositories
        )
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
        repositories.add(Repository(
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
class RepoConfig {
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

// ===== Data Classes =====

@Serializable
data class CompiledConfig(
    val system: SystemConfig,
    val packages: List<Package>,
    val services: List<Service>,
    val users: List<User>,
    val repositories: List<Repository>
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

@Serializable
open class Repository(
    open val name: String,
    open val url: String,
    open val enabled: Boolean = true,
    open val gpgCheck: Boolean = true,
    open val priority: Int = 50
)

@Serializable
data class OstreeRepository(
    override val name: String,
    override val url: String,
    override val enabled: Boolean = true,
    override val gpgCheck: Boolean = true,
    override val priority: Int = 50,
    val branches: List<String> = emptyList()
) : Repository(name, url, enabled, gpgCheck, priority)

// ===== DSL Entry Point =====

fun horizonOS(block: SystemConfiguration.() -> Unit): CompiledConfig {
    return SystemConfiguration().apply(block).toConfig()
}