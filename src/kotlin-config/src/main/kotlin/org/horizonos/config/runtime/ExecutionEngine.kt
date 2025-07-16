package org.horizonos.config.runtime

import org.horizonos.config.dsl.CompiledConfig
import org.horizonos.config.dsl.Package
import org.horizonos.config.dsl.PackageAction
import org.horizonos.config.dsl.Service
import org.horizonos.config.dsl.User
import org.horizonos.config.dsl.Repository
import org.horizonos.config.dsl.PackageRepository
import org.horizonos.config.dsl.OstreeRepository
import org.horizonos.config.dsl.DesktopConfig
import org.horizonos.config.dsl.AutomationConfig
import java.io.File
import java.nio.file.Path
import java.nio.file.Paths
import kotlin.io.path.exists
import kotlin.io.path.writeText

/**
 * Runtime execution engine for HorizonOS configuration management
 * Integrates with OSTree for atomic updates and system state management
 */
class ExecutionEngine(
    private val ostreeRepo: Path = Paths.get("/ostree/repo"),
    private val systemRoot: Path = Paths.get("/"),
    private val configRoot: Path = Paths.get("/etc/horizonos"),
    private val dryRun: Boolean = false
) {
    
    private val commandExecutor = CommandExecutor(dryRun)
    private val ostreeManager = OstreeManager(ostreeRepo, commandExecutor)
    private val systemManager = SystemManager(systemRoot, commandExecutor)
    private val configManager = ConfigManager(configRoot, commandExecutor)
    
    // Live update components
    private val changeDetector = ChangeDetector()
    private val stateSync = StateSyncManager()
    private val serviceReloader = ServiceReloader()
    private val notifier = UpdateNotifier()
    private val liveUpdateManager = LiveUpdateManager(
        systemManager, changeDetector, stateSync, serviceReloader, notifier
    )
    
    /**
     * Apply a compiled configuration to the system
     */
    suspend fun applyConfiguration(config: CompiledConfig): ExecutionResult {
        val operations = mutableListOf<ExecutionOperation>()
        val errors = mutableListOf<ExecutionError>()
        
        try {
            // 1. Create OSTree commit with configuration
            operations.add(ExecutionOperation.OSTreeCommit("Creating OSTree commit"))
            val commitId = ostreeManager.createCommit(config)
            
            // 2. Apply system configuration
            operations.add(ExecutionOperation.SystemConfig("Applying system configuration"))
            systemManager.applySystemConfig(config.system)
            
            // 3. Configure package repositories
            operations.add(ExecutionOperation.RepositoryConfig("Configuring repositories"))
            configManager.configureRepositories(config.repositories)
            
            // 4. Install/remove packages
            if (config.packages.isNotEmpty()) {
                operations.add(ExecutionOperation.PackageManagement("Managing packages"))
                systemManager.managePackages(config.packages)
            }
            
            // 5. Configure services
            if (config.services.isNotEmpty()) {
                operations.add(ExecutionOperation.ServiceConfig("Configuring services"))
                systemManager.configureServices(config.services)
            }
            
            // 6. Create/update users
            if (config.users.isNotEmpty()) {
                operations.add(ExecutionOperation.UserManagement("Managing users"))
                systemManager.manageUsers(config.users)
            }
            
            // 7. Configure desktop environment
            config.desktop?.let { desktop ->
                operations.add(ExecutionOperation.DesktopConfig("Configuring desktop environment"))
                systemManager.configureDesktop(desktop)
            }
            
            // 8. Configure automation workflows
            config.automation?.let { automation ->
                operations.add(ExecutionOperation.AutomationConfig("Configuring automation"))
                systemManager.configureAutomation(automation)
            }
            
            // 9. Deploy OSTree commit
            operations.add(ExecutionOperation.OSTreeDeploy("Deploying OSTree commit"))
            ostreeManager.deployCommit(commitId)
            
            return ExecutionResult.Success(operations, commitId)
            
        } catch (e: Exception) {
            errors.add(ExecutionError.UnexpectedError(e.message ?: "Unknown error"))
            return ExecutionResult.Failure(operations, errors)
        }
    }
    
    /**
     * Rollback to a previous OSTree commit
     */
    suspend fun rollback(commitId: String): ExecutionResult {
        val operations = mutableListOf<ExecutionOperation>()
        val errors = mutableListOf<ExecutionError>()
        
        try {
            operations.add(ExecutionOperation.OSTreeRollback("Rolling back to commit $commitId"))
            ostreeManager.rollback(commitId)
            
            return ExecutionResult.Success(operations, commitId)
            
        } catch (e: Exception) {
            errors.add(ExecutionError.RollbackFailed(e.message ?: "Rollback failed"))
            return ExecutionResult.Failure(operations, errors)
        }
    }
    
    /**
     * Apply configuration updates to a live system
     */
    suspend fun applyLiveUpdates(
        currentConfig: CompiledConfig,
        newConfig: CompiledConfig,
        options: LiveUpdateOptions = LiveUpdateOptions()
    ): LiveUpdateResult {
        return liveUpdateManager.applyLiveUpdates(currentConfig, newConfig, options)
    }
    
    /**
     * Check if live updates are possible
     */
    suspend fun canApplyLiveUpdates(
        currentConfig: CompiledConfig,
        newConfig: CompiledConfig
    ): LiveUpdateCapability {
        return liveUpdateManager.canApplyLiveUpdates(currentConfig, newConfig)
    }
    
    /**
     * Get current system status
     */
    suspend fun getSystemStatus(): SystemStatus {
        val currentCommit = ostreeManager.getCurrentCommit()
        val availableCommits = ostreeManager.getAvailableCommits()
        val systemInfo = systemManager.getSystemInfo()
        val syncStatus = stateSync.checkSync(getLastAppliedConfig())
        
        return SystemStatus(
            currentCommit = currentCommit,
            availableCommits = availableCommits,
            systemInfo = systemInfo,
            syncStatus = syncStatus
        )
    }
    
    /**
     * Get the last applied configuration
     */
    private suspend fun getLastAppliedConfig(): CompiledConfig {
        val configFile = configRoot.resolve("current-config.json")
        return if (configFile.exists()) {
            kotlinx.serialization.json.Json.decodeFromString(
                CompiledConfig.serializer(),
                configFile.toFile().readText()
            )
        } else {
            // Return a minimal default config
            CompiledConfig(
                system = org.horizonos.config.dsl.SystemConfig("horizonos", "UTC", "en_US.UTF-8"),
                packages = emptyList(),
                services = emptyList(),
                users = emptyList(),
                repositories = emptyList()
            )
        }
    }
    
    /**
     * Validate configuration before applying
     */
    suspend fun validateConfiguration(config: CompiledConfig): ValidationResult {
        val errors = mutableListOf<ExecutionError>()
        
        // Check OSTree repository
        if (!ostreeRepo.exists()) {
            errors.add(ExecutionError.OSTreeError("OSTree repository not found"))
        }
        
        // For dry run mode, skip system checks
        if (!dryRun) {
            // Check system permissions
            if (!systemManager.hasRequiredPermissions()) {
                errors.add(ExecutionError.PermissionError("Insufficient permissions"))
            }
            
            // Validate package availability
            config.packages.forEach { pkg ->
                if (!systemManager.isPackageAvailable(pkg.name)) {
                    errors.add(ExecutionError.PackageNotFound(pkg.name))
                }
            }
        }
        
        return if (errors.isEmpty()) {
            ValidationResult.Valid
        } else {
            ValidationResult.Invalid(errors)
        }
    }
}

/**
 * Manages OSTree operations
 */
class OstreeManager(
    private val repoPath: Path,
    private val commandExecutor: CommandExecutor
) {
    
    suspend fun createCommit(config: CompiledConfig): String {
        // Create temporary directory for commit content
        val tempDir = kotlin.io.path.createTempDirectory("horizonos-commit")
        
        try {
            // Write configuration to temporary directory
            val configFile = tempDir.resolve("config.json")
            configFile.writeText(kotlinx.serialization.json.Json.encodeToString(CompiledConfig.serializer(), config))
            
            // Create OSTree commit
            val commitId = commandExecutor.execute(
                "ostree", "commit",
                "--repo=${repoPath}",
                "--tree=dir=${tempDir}",
                "--subject=HorizonOS configuration update",
                "--branch=horizonos/stable/x86_64"
            )
            
            return commitId.trim()
            
        } finally {
            // Clean up temporary directory
            tempDir.toFile().deleteRecursively()
        }
    }
    
    suspend fun deployCommit(commitId: String) {
        commandExecutor.execute(
            "ostree", "admin", "deploy",
            "--os=horizonos",
            "horizonos/stable/x86_64"
        )
    }
    
    suspend fun rollback(commitId: String) {
        commandExecutor.execute(
            "ostree", "admin", "deploy",
            "--os=horizonos",
            commitId
        )
    }
    
    suspend fun getCurrentCommit(): String {
        return commandExecutor.execute(
            "ostree", "admin", "status",
            "--os=horizonos"
        ).lines().first { it.contains("*") }.split(" ")[1]
    }
    
    suspend fun getAvailableCommits(): List<String> {
        return commandExecutor.execute(
            "ostree", "log",
            "--repo=${repoPath}",
            "horizonos/stable/x86_64"
        ).lines()
            .filter { it.contains("commit") }
            .map { it.split(" ")[1] }
    }
}

/**
 * Manages system-level operations
 */
class SystemManager(
    private val systemRoot: Path,
    private val commandExecutor: CommandExecutor
) {
    
    suspend fun applySystemConfig(system: org.horizonos.config.dsl.SystemConfig) {
        // Set hostname
        commandExecutor.execute("hostnamectl", "set-hostname", system.hostname)
        
        // Set timezone
        commandExecutor.execute("timedatectl", "set-timezone", system.timezone)
        
        // Set locale
        val localeFile = systemRoot.resolve("etc/locale.conf")
        localeFile.writeText("LANG=${system.locale}\n")
        commandExecutor.execute("locale-gen")
    }
    
    suspend fun managePackages(packages: List<Package>) {
        val toInstall = packages.filter { it.action == PackageAction.INSTALL }
        val toRemove = packages.filter { it.action == PackageAction.REMOVE }
        
        if (toInstall.isNotEmpty()) {
            val packageNames = toInstall.map { it.name }
            commandExecutor.execute("pacman", "-S", "--needed", "--noconfirm", *packageNames.toTypedArray())
        }
        
        if (toRemove.isNotEmpty()) {
            val packageNames = toRemove.map { it.name }
            commandExecutor.execute("pacman", "-R", "--noconfirm", *packageNames.toTypedArray())
        }
    }
    
    suspend fun configureServices(services: List<Service>) {
        services.forEach { service ->
            if (service.enabled) {
                commandExecutor.execute("systemctl", "enable", service.name)
                commandExecutor.execute("systemctl", "start", service.name)
            } else {
                commandExecutor.execute("systemctl", "disable", service.name)
                commandExecutor.execute("systemctl", "stop", service.name)
            }
        }
    }
    
    suspend fun manageUsers(users: List<User>) {
        users.forEach { user ->
            val args = mutableListOf("useradd", "-m")
            
            user.uid?.let { args.addAll(listOf("-u", it.toString())) }
            args.addAll(listOf("-s", user.shell))
            
            if (user.groups.isNotEmpty()) {
                args.addAll(listOf("-G", user.groups.joinToString(",")))
            }
            
            args.add(user.name)
            
            try {
                commandExecutor.execute(*args.toTypedArray())
            } catch (e: Exception) {
                // User might already exist, try to modify instead
                commandExecutor.execute("usermod", "-s", user.shell, user.name)
                if (user.groups.isNotEmpty()) {
                    commandExecutor.execute("usermod", "-G", user.groups.joinToString(","), user.name)
                }
            }
        }
    }
    
    suspend fun configureDesktop(desktop: DesktopConfig) {
        // Configure desktop environment
        when (desktop.environment) {
            org.horizonos.config.dsl.DesktopEnvironment.HYPRLAND -> {
                commandExecutor.execute("systemctl", "enable", "hyprland")
                desktop.hyprlandConfig?.let { configureHyprland(it) }
            }
            org.horizonos.config.dsl.DesktopEnvironment.PLASMA -> {
                commandExecutor.execute("systemctl", "enable", "plasma")
                desktop.plasmaConfig?.let { configurePlasma(it) }
            }
            else -> {
                // Handle other desktop environments
            }
        }
        
        // Configure auto-login
        if (desktop.autoLogin && desktop.autoLoginUser != null) {
            configureAutoLogin(desktop.autoLoginUser)
        }
    }
    
    private suspend fun configureHyprland(config: org.horizonos.config.dsl.HyprlandConfig) {
        // Configure Hyprland settings
        val hyprlandConfig = """
            theme = ${config.theme}
            animations = ${config.animations}
            gaps = ${config.gaps}
            border_size = ${config.borderSize}
            kde_integration = ${config.kdeIntegration}
            personality_mode = ${config.personalityMode}
        """.trimIndent()
        
        systemRoot.resolve("etc/hypr/hyprland.conf").writeText(hyprlandConfig)
    }
    
    private suspend fun configurePlasma(config: org.horizonos.config.dsl.PlasmaConfig) {
        // Configure Plasma settings
        val plasmaConfig = """
            theme = ${config.theme}
            look_and_feel = ${config.lookAndFeel}
            widgets = ${config.widgets.joinToString(",")}
        """.trimIndent()
        
        systemRoot.resolve("etc/plasma/plasmarc").writeText(plasmaConfig)
    }
    
    private suspend fun configureAutoLogin(username: String) {
        val displayManagerConfig = """
            [Seat:*]
            autologin-user=$username
            autologin-user-timeout=0
        """.trimIndent()
        
        systemRoot.resolve("etc/lightdm/lightdm.conf").writeText(displayManagerConfig)
    }
    
    suspend fun configureAutomation(automation: AutomationConfig) {
        // Write automation configuration
        val automationConfigFile = systemRoot.resolve("etc/horizonos/automation.json")
        automationConfigFile.writeText(
            kotlinx.serialization.json.Json.encodeToString(AutomationConfig.serializer(), automation)
        )
        
        // Enable automation service
        commandExecutor.execute("systemctl", "enable", "horizonos-automation")
        commandExecutor.execute("systemctl", "start", "horizonos-automation")
    }
    
    suspend fun hasRequiredPermissions(): Boolean {
        return try {
            commandExecutor.execute("id", "-u").trim() == "0"
        } catch (e: Exception) {
            false
        }
    }
    
    suspend fun isPackageAvailable(packageName: String): Boolean {
        return try {
            commandExecutor.execute("pacman", "-Si", packageName)
            true
        } catch (e: Exception) {
            false
        }
    }
    
    suspend fun getSystemInfo(): SystemInfo {
        val uptime = commandExecutor.execute("uptime").trim()
        val kernelVersion = commandExecutor.execute("uname", "-r").trim()
        val memoryInfo = commandExecutor.execute("free", "-h")
        
        return SystemInfo(
            uptime = uptime,
            kernelVersion = kernelVersion,
            memoryInfo = memoryInfo
        )
    }
    
    // Methods for live updates
    
    suspend fun installPackages(packages: List<Package>, dryRun: Boolean) {
        if (dryRun) {
            println("DRY RUN: Would install packages: ${packages.joinToString { it.name }}")
            return
        }
        val packageNames = packages.map { it.name }
        commandExecutor.execute("pacman", "-S", "--needed", "--noconfirm", *packageNames.toTypedArray())
    }
    
    suspend fun removePackages(packages: List<Package>, dryRun: Boolean) {
        if (dryRun) {
            println("DRY RUN: Would remove packages: ${packages.joinToString { it.name }}")
            return
        }
        val packageNames = packages.map { it.name }
        commandExecutor.execute("pacman", "-R", "--noconfirm", *packageNames.toTypedArray())
    }
    
    suspend fun createUsers(users: List<User>, dryRun: Boolean) {
        if (dryRun) {
            println("DRY RUN: Would create users: ${users.joinToString { it.name }}")
            return
        }
        users.forEach { user ->
            val args = mutableListOf("useradd", "-m")
            user.uid?.let { args.addAll(listOf("-u", it.toString())) }
            args.addAll(listOf("-s", user.shell))
            if (user.groups.isNotEmpty()) {
                args.addAll(listOf("-G", user.groups.joinToString(",")))
            }
            args.add(user.name)
            commandExecutor.execute(*args.toTypedArray())
        }
    }
    
    suspend fun modifyUser(oldUser: User, newUser: User, dryRun: Boolean) {
        if (dryRun) {
            println("DRY RUN: Would modify user ${oldUser.name}")
            return
        }
        commandExecutor.execute("usermod", "-s", newUser.shell, newUser.name)
        if (newUser.groups.isNotEmpty()) {
            commandExecutor.execute("usermod", "-G", newUser.groups.joinToString(","), newUser.name)
        }
    }
    
    suspend fun updateServiceConfig(serviceName: String, config: org.horizonos.config.dsl.ServiceConfig?, dryRun: Boolean) {
        if (dryRun) {
            println("DRY RUN: Would update config for service $serviceName")
            return
        }
        // Update service configuration
        config?.let { cfg ->
            if (cfg.environment.isNotEmpty()) {
                val envDir = systemRoot.resolve("etc/systemd/system/$serviceName.service.d")
                envDir.toFile().mkdirs()
                val envFile = envDir.resolve("environment.conf")
                val content = buildString {
                    appendLine("[Service]")
                    cfg.environment.forEach { (key, value) ->
                        appendLine("Environment=\"$key=$value\"")
                    }
                }
                envFile.writeText(content)
                commandExecutor.execute("systemctl", "daemon-reload")
            }
        }
    }
    
    suspend fun setTimezone(timezone: String, dryRun: Boolean) {
        if (dryRun) {
            println("DRY RUN: Would set timezone to $timezone")
            return
        }
        commandExecutor.execute("timedatectl", "set-timezone", timezone)
    }
    
    suspend fun setLocale(locale: String, dryRun: Boolean) {
        if (dryRun) {
            println("DRY RUN: Would set locale to $locale")
            return
        }
        val localeFile = systemRoot.resolve("etc/locale.conf")
        localeFile.writeText("LANG=$locale\n")
        commandExecutor.execute("locale-gen")
    }
    
    suspend fun setHostname(hostname: String, dryRun: Boolean) {
        if (dryRun) {
            println("DRY RUN: Would set hostname to $hostname")
            return
        }
        commandExecutor.execute("hostnamectl", "set-hostname", hostname)
    }
    
    suspend fun updateAutomationWorkflow(workflow: org.horizonos.config.dsl.Workflow, dryRun: Boolean) {
        if (dryRun) {
            println("DRY RUN: Would update automation workflow ${workflow.name}")
            return
        }
        // Update automation workflow configuration
        val workflowFile = systemRoot.resolve("etc/horizonos/workflows/${workflow.name}.json")
        workflowFile.parent.toFile().mkdirs()
        workflowFile.writeText(
            kotlinx.serialization.json.Json.encodeToString(org.horizonos.config.dsl.Workflow.serializer(), workflow)
        )
        // Notify automation service
        commandExecutor.execute("systemctl", "reload", "horizonos-automation")
    }
}

/**
 * Manages configuration file operations
 */
class ConfigManager(
    private val configRoot: Path,
    private val commandExecutor: CommandExecutor
) {
    
    suspend fun configureRepositories(repositories: List<Repository>) {
        val pacmanRepos = repositories.filterIsInstance<PackageRepository>()
        val ostreeRepos = repositories.filterIsInstance<OstreeRepository>()
        
        // Configure pacman repositories
        if (pacmanRepos.isNotEmpty()) {
            val pacmanConfig = buildString {
                pacmanRepos.forEach { repo ->
                    appendLine("[${repo.name}]")
                    appendLine("Server = ${repo.url}")
                    if (!repo.gpgCheck) {
                        appendLine("SigLevel = Never")
                    }
                    appendLine()
                }
            }
            
            configRoot.resolve("pacman-repos.conf").writeText(pacmanConfig)
        }
        
        // Configure OSTree repositories
        if (ostreeRepos.isNotEmpty()) {
            val ostreeConfig = kotlinx.serialization.json.Json.encodeToString(kotlinx.serialization.builtins.ListSerializer(OstreeRepository.serializer()), ostreeRepos)
            configRoot.resolve("ostree-repos.json").writeText(ostreeConfig)
        }
    }
}

/**
 * Executes system commands
 */
class CommandExecutor(private val dryRun: Boolean = false) {
    
    suspend fun execute(vararg command: String): String {
        if (dryRun) {
            println("DRY RUN: ${command.joinToString(" ")}")
            return ""
        }
        
        val process = ProcessBuilder(*command).start()
        val result = process.inputStream.bufferedReader().readText()
        val exitCode = process.waitFor()
        
        if (exitCode != 0) {
            val error = process.errorStream.bufferedReader().readText()
            throw RuntimeException("Command failed with exit code $exitCode: $error")
        }
        
        return result
    }
}

// ===== Data Classes =====

data class SystemStatus(
    val currentCommit: String,
    val availableCommits: List<String>,
    val systemInfo: SystemInfo,
    val syncStatus: SyncStatus? = null
)

data class SystemInfo(
    val uptime: String,
    val kernelVersion: String,
    val memoryInfo: String
)

sealed class ExecutionResult {
    data class Success(val operations: List<ExecutionOperation>, val commitId: String) : ExecutionResult()
    data class Failure(val operations: List<ExecutionOperation>, val errors: List<ExecutionError>) : ExecutionResult()
}

sealed class ExecutionOperation(val description: String) {
    data class OSTreeCommit(val desc: String) : ExecutionOperation(desc)
    data class OSTreeDeploy(val desc: String) : ExecutionOperation(desc)
    data class OSTreeRollback(val desc: String) : ExecutionOperation(desc)
    data class SystemConfig(val desc: String) : ExecutionOperation(desc)
    data class PackageManagement(val desc: String) : ExecutionOperation(desc)
    data class ServiceConfig(val desc: String) : ExecutionOperation(desc)
    data class UserManagement(val desc: String) : ExecutionOperation(desc)
    data class RepositoryConfig(val desc: String) : ExecutionOperation(desc)
    data class DesktopConfig(val desc: String) : ExecutionOperation(desc)
    data class AutomationConfig(val desc: String) : ExecutionOperation(desc)
}

sealed class ExecutionError(val message: String) {
    data class OSTreeError(val error: String) : ExecutionError("OSTree error: $error")
    data class PackageNotFound(val packageName: String) : ExecutionError("Package not found: $packageName")
    data class PermissionError(val error: String) : ExecutionError("Permission error: $error")
    data class RollbackFailed(val error: String) : ExecutionError("Rollback failed: $error")
    data class UnexpectedError(val error: String) : ExecutionError("Unexpected error: $error")
}

sealed class ValidationResult {
    object Valid : ValidationResult()
    data class Invalid(val errors: List<ExecutionError>) : ValidationResult()
}