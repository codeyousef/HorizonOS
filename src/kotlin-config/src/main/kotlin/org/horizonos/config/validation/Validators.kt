package org.horizonos.config.validation

import org.horizonos.config.dsl.*

sealed class ValidationError(open val message: String) {
    data class InvalidHostname(val hostname: String) : ValidationError("Invalid hostname: $hostname")
    data class InvalidTimezone(val timezone: String) : ValidationError("Invalid timezone: $timezone")
    data class InvalidLocale(val locale: String) : ValidationError("Invalid locale: $locale")
    data class InvalidPackageName(val packageName: String) : ValidationError("Invalid package name: $packageName")
    data class InvalidServiceName(val serviceName: String) : ValidationError("Invalid service name: $serviceName")
    data class InvalidUsername(val username: String) : ValidationError("Invalid username: $username")
    data class InvalidUID(val uid: Int) : ValidationError("Invalid UID: $uid")
    data class InvalidShell(val shell: String) : ValidationError("Invalid shell: $shell")
    data class InvalidGroupName(val groupName: String) : ValidationError("Invalid group name: $groupName")
    data class InvalidRepositoryName(val repoName: String) : ValidationError("Invalid repository name: $repoName")
    data class InvalidUrl(val url: String) : ValidationError("Invalid URL: $url")
    data class InvalidBranch(val branch: String) : ValidationError("Invalid branch name: $branch")
    data class DuplicateUser(val username: String) : ValidationError("Duplicate user: $username")
    data class DuplicateService(val serviceName: String) : ValidationError("Duplicate service: $serviceName")
    data class DuplicateRepository(val repoName: String) : ValidationError("Duplicate repository: $repoName")
    data class ConflictingPackages(val packageName: String) : ValidationError("Package $packageName has conflicting install/remove actions")
    data class MissingAutoLoginUser(val autoLoginUser: String) : ValidationError("Auto-login user '$autoLoginUser' is not defined")
    data class InvalidDesktopConfig(override val message: String) : ValidationError("Invalid desktop configuration: $message")
}

class ValidationResult(val errors: List<ValidationError>) {
    val isValid: Boolean get() = errors.isEmpty()
    val isInvalid: Boolean get() = errors.isNotEmpty()
    
    fun throwIfInvalid() {
        if (isInvalid) {
            throw ValidationException(errors)
        }
    }
}

class ValidationException(val errors: List<ValidationError>) : Exception(
    "Configuration validation failed:\n${errors.joinToString("\n") { "  - ${it.message}" }}"
)

object ConfigurationValidator {
    
    fun validate(config: CompiledConfig): ValidationResult {
        val errors = mutableListOf<ValidationError>()
        
        // Validate system configuration
        errors.addAll(validateSystemConfig(config.system))
        
        // Validate packages
        errors.addAll(validatePackages(config.packages))
        
        // Validate services
        errors.addAll(validateServices(config.services))
        
        // Validate users
        errors.addAll(validateUsers(config.users))
        
        // Validate repositories
        errors.addAll(validateRepositories(config.repositories))
        
        // Validate desktop configuration
        config.desktop?.let { desktop ->
            errors.addAll(validateDesktopConfig(desktop, config.users))
        }
        
        return ValidationResult(errors)
    }
    
    private fun validateSystemConfig(system: SystemConfig): List<ValidationError> {
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
    
    private fun validatePackages(packages: List<Package>): List<ValidationError> {
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
    
    private fun validateServices(services: List<Service>): List<ValidationError> {
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
    
    private fun validateUsers(users: List<User>): List<ValidationError> {
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
            if (!isValidShell(user.shell)) {
                errors.add(ValidationError.InvalidShell(user.shell))
            }
            
            // Validate group names
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
    
    private fun validateRepositories(repositories: List<Repository>): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        repositories.forEach { repo ->
            // Validate repository name
            if (!isValidRepositoryName(repo.name)) {
                errors.add(ValidationError.InvalidRepositoryName(repo.name))
            }
            
            // Validate URL
            if (!isValidUrl(repo.url)) {
                errors.add(ValidationError.InvalidUrl(repo.url))
            }
            
            // Validate OSTree-specific fields
            if (repo is OstreeRepository) {
                repo.branches.forEach { branch ->
                    if (!isValidBranch(branch)) {
                        errors.add(ValidationError.InvalidBranch(branch))
                    }
                }
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
    
    private fun validateDesktopConfig(desktop: DesktopConfig, users: List<User>): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        // Validate auto-login user exists
        if (desktop.autoLogin && desktop.autoLoginUser != null) {
            val userExists = users.any { it.name == desktop.autoLoginUser }
            if (!userExists) {
                errors.add(ValidationError.MissingAutoLoginUser(desktop.autoLoginUser))
            }
        }
        
        // Note: Desktop environment specific configurations are optional
        // Users can configure the environment without providing specific settings
        
        return errors
    }
    
    // Validation helper functions
    private fun isValidHostname(hostname: String): Boolean {
        return hostname.matches(Regex("^[a-zA-Z0-9]([a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?$"))
    }
    
    private fun isValidTimezone(timezone: String): Boolean {
        // Basic timezone validation - should be improved with actual timezone data
        return timezone.matches(Regex("^[A-Za-z_/]+$")) && timezone.length <= 50
    }
    
    private fun isValidLocale(locale: String): Boolean {
        return locale.matches(Regex("^[a-zA-Z]{2,3}(_[A-Z]{2})?(\\..*)?$"))
    }
    
    private fun isValidPackageName(packageName: String): Boolean {
        return packageName.matches(Regex("^[a-zA-Z0-9._+-]+$")) && packageName.isNotBlank()
    }
    
    private fun isValidServiceName(serviceName: String): Boolean {
        return serviceName.matches(Regex("^[a-zA-Z0-9._-]+$")) && serviceName.isNotBlank()
    }
    
    private fun isValidUsername(username: String): Boolean {
        return username.matches(Regex("^[a-z_][a-z0-9_-]*$")) && username.length <= 32
    }
    
    private fun isValidUID(uid: Int): Boolean {
        return uid in 1..65535
    }
    
    private fun isValidShell(shell: String): Boolean {
        return shell.startsWith("/") && shell.length > 1
    }
    
    private fun isValidGroupName(groupName: String): Boolean {
        return groupName.matches(Regex("^[a-z_][a-z0-9_-]*$")) && groupName.length <= 32
    }
    
    private fun isValidRepositoryName(repoName: String): Boolean {
        return repoName.matches(Regex("^[a-zA-Z0-9._-]+$")) && repoName.isNotBlank()
    }
    
    private fun isValidUrl(url: String): Boolean {
        return url.matches(Regex("^https?://.*")) || url.matches(Regex("^file://.*"))
    }
    
    private fun isValidBranch(branch: String): Boolean {
        return branch.matches(Regex("^[a-zA-Z0-9._/-]+$")) && branch.isNotBlank()
    }
}