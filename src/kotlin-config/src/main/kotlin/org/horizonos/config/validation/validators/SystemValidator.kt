package org.horizonos.config.validation.validators

import org.horizonos.config.dsl.*
import org.horizonos.config.validation.ValidationError

object SystemValidator {
    
    fun validateSystemConfig(system: SystemConfig): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        // Validate hostname
        if (!isValidHostname(system.hostname)) {
            errors.add(ValidationError.InvalidHostname(system.hostname))
        }
        
        // Validate timezone
        if (!isValidTimezone(system.timezone)) {
            errors.add(ValidationError.InvalidTimezone(system.timezone))
        }
        
        // Validate locale
        if (!isValidLocale(system.locale)) {
            errors.add(ValidationError.InvalidLocale(system.locale))
        }
        
        return errors
    }
    
    fun validatePackages(packages: List<Package>): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        // Validate package names
        packages.forEach { pkg ->
            if (!isValidPackageName(pkg.name)) {
                errors.add(ValidationError.InvalidPackageName(pkg.name))
            }
        }
        
        // Check for conflicting actions (install and remove same package)
        val packageActions = packages.groupBy { it.name }
        packageActions.forEach { (name, actions) ->
            val hasInstall = actions.any { it.action == PackageAction.INSTALL }
            val hasRemove = actions.any { it.action == PackageAction.REMOVE }
            if (hasInstall && hasRemove) {
                errors.add(ValidationError.ConflictingPackages(name))
            }
        }
        
        return errors
    }
    
    fun validateServices(services: List<Service>): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        // Validate service names
        services.forEach { service ->
            if (!isValidServiceName(service.name)) {
                errors.add(ValidationError.InvalidServiceName(service.name))
            }
        }
        
        // Check for duplicate services
        val duplicateServices = services.groupBy { it.name }
            .filter { it.value.size > 1 }
            .keys
        duplicateServices.forEach { serviceName ->
            errors.add(ValidationError.DuplicateService(serviceName))
        }
        
        return errors
    }
    
    fun validateUsers(users: List<User>): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        // Validate usernames
        users.forEach { user ->
            if (!isValidUsername(user.name)) {
                errors.add(ValidationError.InvalidUsername(user.name))
            }
            
            // Validate UID
            user.uid?.let { uid ->
                if (!isValidUID(uid)) {
                    errors.add(ValidationError.InvalidUID(uid))
                }
            }
            
            // Validate shell
            user.shell?.let { shell ->
                if (!isValidShell(shell)) {
                    errors.add(ValidationError.InvalidShell(shell))
                }
            }
            
            // Validate groups
            user.groups.forEach { group ->
                if (!isValidGroupName(group)) {
                    errors.add(ValidationError.InvalidGroupName(group))
                }
            }
        }
        
        // Check for duplicate users
        val duplicateUsers = users.groupBy { it.name }
            .filter { it.value.size > 1 }
            .keys
        duplicateUsers.forEach { username ->
            errors.add(ValidationError.DuplicateUser(username))
        }
        
        return errors
    }
    
    fun validateRepositories(repositories: List<Repository>): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        // Validate repository names
        repositories.forEach { repo ->
            if (!isValidRepositoryName(repo.name)) {
                errors.add(ValidationError.InvalidRepositoryName(repo.name))
            }
            
            // Validate URL
            if (!isValidUrl(repo.url)) {
                errors.add(ValidationError.InvalidUrl(repo.url))
            }
        }
        
        // Check for duplicate repositories
        val duplicateRepos = repositories.groupBy { it.name }
            .filter { it.value.size > 1 }
            .keys
        duplicateRepos.forEach { repoName ->
            errors.add(ValidationError.DuplicateRepository(repoName))
        }
        
        return errors
    }
    
    fun validateDesktopConfig(desktop: DesktopConfig, users: List<User>): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        // Validate auto-login user exists
        desktop.autoLoginUser?.let { autoLoginUser ->
            if (users.none { it.name == autoLoginUser }) {
                errors.add(ValidationError.MissingAutoLoginUser(autoLoginUser))
            }
        }
        
        // Validate desktop environment specific settings
        when (desktop.environment) {
            DesktopEnvironment.GNOME -> {
                // Add GNOME-specific validations if needed
            }
            DesktopEnvironment.PLASMA -> {
                // Add KDE Plasma-specific validations if needed  
            }
            DesktopEnvironment.XFCE -> {
                // Add XFCE-specific validations if needed
            }
            DesktopEnvironment.HYPRLAND -> {
                // Add Hyprland-specific validations if needed
            }
            DesktopEnvironment.GRAPH -> {
                // Add Graph desktop-specific validations if needed
            }
        }
        
        return errors
    }
    
    // Validation helper functions
    private fun isValidHostname(hostname: String): Boolean {
        return hostname.matches(Regex("^[a-zA-Z0-9]([a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?$"))
    }
    
    private fun isValidTimezone(timezone: String): Boolean {
        val validTimezones = setOf(
            "UTC", "America/New_York", "America/Los_Angeles", "Europe/London", 
            "Europe/Berlin", "Asia/Tokyo", "Asia/Shanghai", "Australia/Sydney"
        )
        return validTimezones.contains(timezone) || timezone.matches(Regex("^[A-Z][a-z]+/[A-Z][a-z_]+$"))
    }
    
    private fun isValidLocale(locale: String): Boolean {
        return locale.matches(Regex("^[a-z]{2}_[A-Z]{2}(\\.UTF-8)?$"))
    }
    
    private fun isValidPackageName(packageName: String): Boolean {
        return packageName.matches(Regex("^[a-z0-9][a-z0-9+.-]*$"))
    }
    
    private fun isValidServiceName(serviceName: String): Boolean {
        return serviceName.matches(Regex("^[a-z0-9][a-z0-9.-]*$"))
    }
    
    private fun isValidUsername(username: String): Boolean {
        return username.matches(Regex("^[a-z_][a-z0-9_-]*\\$?$")) && username.length <= 32
    }
    
    private fun isValidUID(uid: Int): Boolean {
        return uid in 0..65535
    }
    
    private fun isValidShell(shell: String): Boolean {
        val validShells = setOf(
            "/bin/bash", "/bin/sh", "/bin/zsh", "/bin/fish", 
            "/usr/bin/zsh", "/usr/bin/fish", "/sbin/nologin"
        )
        return validShells.contains(shell)
    }
    
    private fun isValidGroupName(groupName: String): Boolean {
        return groupName.matches(Regex("^[a-z_][a-z0-9_-]*\\$?$")) && groupName.length <= 32
    }
    
    private fun isValidRepositoryName(repoName: String): Boolean {
        return repoName.matches(Regex("^[a-zA-Z0-9][a-zA-Z0-9._-]*$"))
    }
    
    private fun isValidUrl(url: String): Boolean {
        return url.matches(Regex("^https?://[^\\s/$.?#].[^\\s]*$"))
    }
    
    private fun isValidBranch(branch: String): Boolean {
        return branch.matches(Regex("^[a-zA-Z0-9][a-zA-Z0-9._/-]*$"))
    }
    
    private fun isValidGnomeExtension(extension: String): Boolean {
        return extension.matches(Regex("^[a-zA-Z0-9@._-]+$"))
    }
    
    private fun isValidKdeTheme(theme: String): Boolean {
        return theme.matches(Regex("^[a-zA-Z0-9._-]+$"))
    }
    
    private fun isValidXfcePanelItem(item: String): Boolean {
        return item.matches(Regex("^[a-zA-Z0-9._-]+$"))
    }
}