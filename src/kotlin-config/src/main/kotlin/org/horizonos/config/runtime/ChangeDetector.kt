package org.horizonos.config.runtime

import org.horizonos.config.dsl.*

/**
 * Detects changes between configurations and categorizes them by impact
 */
class ChangeDetector {
    
    /**
     * Detect all changes between two configurations
     */
    fun detectChanges(
        currentConfig: CompiledConfig,
        newConfig: CompiledConfig
    ): List<ConfigChange> {
        val changes = mutableListOf<ConfigChange>()
        
        // Detect system changes
        changes.addAll(detectSystemChanges(currentConfig.system, newConfig.system))
        
        // Detect package changes
        changes.addAll(detectPackageChanges(currentConfig.packages, newConfig.packages))
        
        // Detect service changes
        changes.addAll(detectServiceChanges(currentConfig.services, newConfig.services))
        
        // Detect user changes
        changes.addAll(detectUserChanges(currentConfig.users, newConfig.users))
        
        // Detect repository changes
        changes.addAll(detectRepositoryChanges(currentConfig.repositories, newConfig.repositories))
        
        // Detect desktop changes
        changes.addAll(detectDesktopChanges(currentConfig.desktop, newConfig.desktop))
        
        // Detect automation changes
        changes.addAll(detectAutomationChanges(currentConfig.automation, newConfig.automation))
        
        return changes
    }
    
    /**
     * Detect system configuration changes
     */
    private fun detectSystemChanges(
        current: SystemConfig,
        new: SystemConfig
    ): List<ConfigChange> {
        val changes = mutableListOf<ConfigChange>()
        
        if (current.hostname != new.hostname) {
            changes.add(ConfigChange(
                type = ChangeType.SYSTEM_CONFIG,
                field = "hostname",
                oldValue = current.hostname,
                newValue = new.hostname,
                description = "Hostname change: ${current.hostname} → ${new.hostname}",
                updateStrategy = UpdateStrategy.LIVE,
                impact = ImpactLevel.LOW
            ))
        }
        
        if (current.timezone != new.timezone) {
            changes.add(ConfigChange(
                type = ChangeType.SYSTEM_CONFIG,
                field = "timezone",
                oldValue = current.timezone,
                newValue = new.timezone,
                description = "Timezone change: ${current.timezone} → ${new.timezone}",
                updateStrategy = UpdateStrategy.LIVE,
                impact = ImpactLevel.LOW
            ))
        }
        
        if (current.locale != new.locale) {
            changes.add(ConfigChange(
                type = ChangeType.SYSTEM_CONFIG,
                field = "locale",
                oldValue = current.locale,
                newValue = new.locale,
                description = "Locale change: ${current.locale} → ${new.locale}",
                updateStrategy = UpdateStrategy.LIVE,
                impact = ImpactLevel.MEDIUM
            ))
        }
        
        return changes
    }
    
    /**
     * Detect package changes
     */
    private fun detectPackageChanges(
        current: List<Package>,
        new: List<Package>
    ): List<ConfigChange> {
        val changes = mutableListOf<ConfigChange>()
        
        val currentMap = current.associateBy { it.name }
        val newMap = new.associateBy { it.name }
        
        // Find packages to install
        val toInstall = newMap.filter { (name, pkg) -> 
            !currentMap.containsKey(name) && pkg.action == PackageAction.INSTALL
        }.values.toList()
        
        if (toInstall.isNotEmpty()) {
            changes.add(ConfigChange(
                type = ChangeType.PACKAGE_INSTALL,
                oldValue = emptyList<Package>(),
                newValue = toInstall,
                description = "Install packages: ${toInstall.joinToString { it.name }}",
                updateStrategy = UpdateStrategy.LIVE,
                impact = ImpactLevel.MEDIUM
            ))
        }
        
        // Find packages to remove
        val toRemove = currentMap.filter { (name, _) ->
            val newPkg = newMap[name]
            newPkg == null || newPkg.action == PackageAction.REMOVE
        }.values.toList()
        
        if (toRemove.isNotEmpty()) {
            changes.add(ConfigChange(
                type = ChangeType.PACKAGE_REMOVE,
                oldValue = toRemove,
                newValue = emptyList<Package>(),
                description = "Remove packages: ${toRemove.joinToString { it.name }}",
                updateStrategy = UpdateStrategy.LIVE,
                impact = ImpactLevel.MEDIUM
            ))
        }
        
        return changes
    }
    
    /**
     * Detect service changes
     */
    private fun detectServiceChanges(
        current: List<Service>,
        new: List<Service>
    ): List<ConfigChange> {
        val changes = mutableListOf<ConfigChange>()
        
        val currentMap = current.associateBy { it.name }
        val newMap = new.associateBy { it.name }
        
        // Check each service for changes
        for ((name, newService) in newMap) {
            val currentService = currentMap[name]
            
            if (currentService == null) {
                // New service
                changes.add(ConfigChange(
                    type = ChangeType.SERVICE_ADD,
                    oldValue = null,
                    newValue = newService,
                    affectedService = name,
                    description = "Add service: $name (${if (newService.enabled) "enabled" else "disabled"})",
                    updateStrategy = UpdateStrategy.SERVICE_RELOAD,
                    impact = ImpactLevel.MEDIUM
                ))
            } else if (currentService.enabled != newService.enabled) {
                // Service enable/disable
                changes.add(ConfigChange(
                    type = ChangeType.SERVICE_STATE,
                    oldValue = currentService,
                    newValue = newService,
                    affectedService = name,
                    description = "Service $name: ${if (newService.enabled) "enable" else "disable"}",
                    updateStrategy = UpdateStrategy.SERVICE_RELOAD,
                    impact = ImpactLevel.MEDIUM
                ))
            } else if (currentService.config != newService.config) {
                // Service configuration change
                changes.add(ConfigChange(
                    type = ChangeType.SERVICE_CONFIG,
                    oldValue = currentService.config,
                    newValue = newService.config,
                    affectedService = name,
                    description = "Update configuration for service: $name",
                    updateStrategy = if (canReloadService(name)) 
                        UpdateStrategy.SERVICE_RELOAD 
                    else 
                        UpdateStrategy.REBOOT_REQUIRED,
                    impact = ImpactLevel.HIGH
                ))
            }
        }
        
        // Find removed services
        for ((name, currentService) in currentMap) {
            if (!newMap.containsKey(name)) {
                changes.add(ConfigChange(
                    type = ChangeType.SERVICE_REMOVE,
                    oldValue = currentService,
                    newValue = null,
                    affectedService = name,
                    description = "Remove service: $name",
                    updateStrategy = UpdateStrategy.SERVICE_RELOAD,
                    impact = ImpactLevel.MEDIUM
                ))
            }
        }
        
        return changes
    }
    
    /**
     * Detect user changes
     */
    private fun detectUserChanges(
        current: List<User>,
        new: List<User>
    ): List<ConfigChange> {
        val changes = mutableListOf<ConfigChange>()
        
        val currentMap = current.associateBy { it.name }
        val newMap = new.associateBy { it.name }
        
        // Find new users
        val newUsers = newMap.filter { (name, _) -> !currentMap.containsKey(name) }.values.toList()
        if (newUsers.isNotEmpty()) {
            changes.add(ConfigChange(
                type = ChangeType.USER_ADD,
                oldValue = emptyList<User>(),
                newValue = newUsers,
                description = "Add users: ${newUsers.joinToString { it.name }}",
                updateStrategy = UpdateStrategy.LIVE,
                impact = ImpactLevel.HIGH
            ))
        }
        
        // Find modified users
        for ((name, newUser) in newMap) {
            val currentUser = currentMap[name]
            if (currentUser != null && currentUser != newUser) {
                val modifications = mutableListOf<String>()
                
                if (currentUser.uid != newUser.uid) modifications.add("UID")
                if (currentUser.shell != newUser.shell) modifications.add("shell")
                if (currentUser.groups != newUser.groups) modifications.add("groups")
                if (currentUser.homeDir != newUser.homeDir) modifications.add("home directory")
                
                changes.add(ConfigChange(
                    type = ChangeType.USER_MODIFY,
                    oldValue = currentUser,
                    newValue = newUser,
                    description = "Modify user $name: ${modifications.joinToString(", ")}",
                    updateStrategy = UpdateStrategy.LIVE,
                    impact = ImpactLevel.HIGH
                ))
            }
        }
        
        // Find removed users
        val removedUsers = currentMap.filter { (name, _) -> !newMap.containsKey(name) }.values.toList()
        if (removedUsers.isNotEmpty()) {
            changes.add(ConfigChange(
                type = ChangeType.USER_REMOVE,
                oldValue = removedUsers,
                newValue = emptyList<User>(),
                description = "Remove users: ${removedUsers.joinToString { it.name }}",
                updateStrategy = UpdateStrategy.REBOOT_REQUIRED, // Safer to reboot when removing users
                impact = ImpactLevel.CRITICAL
            ))
        }
        
        return changes
    }
    
    /**
     * Detect repository changes
     */
    private fun detectRepositoryChanges(
        current: List<Repository>,
        new: List<Repository>
    ): List<ConfigChange> {
        val changes = mutableListOf<ConfigChange>()
        
        if (current.toSet() != new.toSet()) {
            changes.add(ConfigChange(
                type = ChangeType.REPOSITORY,
                oldValue = current,
                newValue = new,
                description = "Repository configuration changed",
                updateStrategy = UpdateStrategy.LIVE,
                impact = ImpactLevel.MEDIUM
            ))
        }
        
        return changes
    }
    
    /**
     * Detect desktop environment changes
     */
    private fun detectDesktopChanges(
        current: DesktopConfig?,
        new: DesktopConfig?
    ): List<ConfigChange> {
        val changes = mutableListOf<ConfigChange>()
        
        when {
            current == null && new != null -> {
                changes.add(ConfigChange(
                    type = ChangeType.DESKTOP_CONFIG,
                    oldValue = null,
                    newValue = new,
                    description = "Enable desktop environment: ${new.environment}",
                    updateStrategy = UpdateStrategy.REBOOT_REQUIRED,
                    impact = ImpactLevel.CRITICAL
                ))
            }
            current != null && new == null -> {
                changes.add(ConfigChange(
                    type = ChangeType.DESKTOP_CONFIG,
                    oldValue = current,
                    newValue = null,
                    description = "Disable desktop environment",
                    updateStrategy = UpdateStrategy.REBOOT_REQUIRED,
                    impact = ImpactLevel.CRITICAL
                ))
            }
            current != null && new != null && current != new -> {
                val strategy = if (current.environment == new.environment) {
                    // Same DE, just config changes - might be reloadable
                    UpdateStrategy.SERVICE_RELOAD
                } else {
                    // Different DE - requires reboot
                    UpdateStrategy.REBOOT_REQUIRED
                }
                
                changes.add(ConfigChange(
                    type = ChangeType.DESKTOP_CONFIG,
                    oldValue = current,
                    newValue = new,
                    description = "Update desktop configuration",
                    updateStrategy = strategy,
                    impact = ImpactLevel.HIGH
                ))
            }
        }
        
        return changes
    }
    
    /**
     * Detect automation changes
     */
    private fun detectAutomationChanges(
        current: AutomationConfig?,
        new: AutomationConfig?
    ): List<ConfigChange> {
        val changes = mutableListOf<ConfigChange>()
        
        when {
            current == null && new != null -> {
                new.workflows.forEach { workflow ->
                    changes.add(ConfigChange(
                        type = ChangeType.AUTOMATION_WORKFLOW,
                        oldValue = null,
                        newValue = workflow,
                        description = "Add automation workflow: ${workflow.name}",
                        updateStrategy = UpdateStrategy.LIVE,
                        impact = ImpactLevel.LOW
                    ))
                }
            }
            current != null && new != null -> {
                val currentWorkflows = current.workflows.associateBy { it.name }
                val newWorkflows = new.workflows.associateBy { it.name }
                
                // Find added/modified workflows
                for ((name, workflow) in newWorkflows) {
                    val currentWorkflow = currentWorkflows[name]
                    if (currentWorkflow == null) {
                        changes.add(ConfigChange(
                            type = ChangeType.AUTOMATION_WORKFLOW,
                            oldValue = null,
                            newValue = workflow,
                            description = "Add automation workflow: $name",
                            updateStrategy = UpdateStrategy.LIVE,
                            impact = ImpactLevel.LOW
                        ))
                    } else if (currentWorkflow != workflow) {
                        changes.add(ConfigChange(
                            type = ChangeType.AUTOMATION_WORKFLOW,
                            oldValue = currentWorkflow,
                            newValue = workflow,
                            description = "Update automation workflow: $name",
                            updateStrategy = UpdateStrategy.LIVE,
                            impact = ImpactLevel.LOW
                        ))
                    }
                }
                
                // Find removed workflows
                for ((name, workflow) in currentWorkflows) {
                    if (!newWorkflows.containsKey(name)) {
                        changes.add(ConfigChange(
                            type = ChangeType.AUTOMATION_WORKFLOW,
                            oldValue = workflow,
                            newValue = null,
                            description = "Remove automation workflow: $name",
                            updateStrategy = UpdateStrategy.LIVE,
                            impact = ImpactLevel.LOW
                        ))
                    }
                }
            }
        }
        
        return changes
    }
    
    /**
     * Check if a service supports configuration reload
     */
    private fun canReloadService(serviceName: String): Boolean {
        // Services known to support reload without restart
        val reloadableServices = setOf(
            "nginx",
            "apache2",
            "httpd", 
            "postfix",
            "dovecot",
            "bind9",
            "named",
            "sshd",
            "NetworkManager",
            "systemd-resolved",
            "systemd-timesyncd"
        )
        
        return serviceName in reloadableServices
    }
}

// ===== Data Classes =====

data class ConfigChange(
    val type: ChangeType,
    val field: String? = null,
    val oldValue: Any?,
    val newValue: Any?,
    val affectedService: String? = null,
    val description: String,
    val updateStrategy: UpdateStrategy,
    val impact: ImpactLevel
)

enum class ChangeType {
    SYSTEM_CONFIG,
    PACKAGE_INSTALL,
    PACKAGE_REMOVE,
    SERVICE_ADD,
    SERVICE_REMOVE,
    SERVICE_STATE,
    SERVICE_CONFIG,
    USER_ADD,
    USER_MODIFY,
    USER_REMOVE,
    REPOSITORY,
    DESKTOP_CONFIG,
    AUTOMATION_WORKFLOW
}

enum class UpdateStrategy {
    LIVE,           // Can be applied immediately
    SERVICE_RELOAD, // Requires service reload
    REBOOT_REQUIRED // Requires system reboot
}

enum class ImpactLevel {
    LOW,      // Minimal impact, safe to apply
    MEDIUM,   // Moderate impact, may affect running services
    HIGH,     // High impact, affects system behavior
    CRITICAL  // Critical change, affects system stability
}