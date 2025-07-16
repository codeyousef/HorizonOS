package org.horizonos.config.validation

import org.horizonos.config.dsl.*
import org.horizonos.config.validation.validators.*

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
    data class InvalidBootEntryPath(val path: String) : ValidationError("Invalid boot entry path: $path")
    data class InvalidKernelParameter(val parameter: String) : ValidationError("Invalid kernel parameter: $parameter")
    data class InvalidModule(val module: String) : ValidationError("Invalid module name: $module")
    data class InvalidInitramfsHook(val hook: String) : ValidationError("Invalid initramfs hook: $hook")
    data class InvalidPlymouthTheme(val theme: String) : ValidationError("Invalid Plymouth theme: $theme")
    data class ConflictingBootEntries(val title: String) : ValidationError("Duplicate boot entry title: $title")
    data class InvalidSecureBootKey(val keyPath: String) : ValidationError("Invalid Secure Boot key path: $keyPath")
    data class InvalidGPUDriver(val driver: String) : ValidationError("Invalid GPU driver: $driver")
    data class InvalidDisplayResolution(val resolution: String) : ValidationError("Invalid display resolution: $resolution")
    data class InvalidRefreshRate(val rate: Double) : ValidationError("Invalid refresh rate: $rate")
    data class InvalidAudioDevice(val device: String) : ValidationError("Invalid audio device: $device")
    data class InvalidPowerProfile(override val message: String) : ValidationError("Invalid power configuration: $message")
    data class ConflictingMonitors(val name: String) : ValidationError("Duplicate monitor configuration: $name")
    data class InvalidThermalZone(val zone: String) : ValidationError("Invalid thermal zone: $zone")
    data class InvalidBluetoothAddress(val address: String) : ValidationError("Invalid Bluetooth address: $address")
    data class InvalidUSBDevice(val device: String) : ValidationError("Invalid USB device identifier: $device")
    data class InvalidDevicePath(val path: String) : ValidationError("Invalid device path: $path")
    data class InvalidMountPoint(val path: String) : ValidationError("Invalid mount point: $path")
    data class InvalidRAIDLevel(val level: String) : ValidationError("Invalid RAID level: $level")
    data class InvalidEncryptionCipher(val cipher: String) : ValidationError("Invalid encryption cipher: $cipher")
    data class InvalidKeySize(val size: Int) : ValidationError("Invalid key size: $size")
    data class InvalidSwapSize(val size: String) : ValidationError("Invalid swap size: $size")
    data class InvalidFilesystemType(val type: String) : ValidationError("Invalid filesystem type: $type")
    data class ConflictingMountPoints(val mountPoint: String) : ValidationError("Duplicate mount point: $mountPoint")
    data class InvalidBtrfsProfile(val profile: String) : ValidationError("Invalid Btrfs profile: $profile")
    data class InvalidCompressionAlgorithm(val algorithm: String) : ValidationError("Invalid compression algorithm: $algorithm")
    data class InvalidPath(val path: String) : ValidationError("Invalid path: $path")
    data class InvalidSSHPort(val port: Int) : ValidationError("Invalid SSH port: $port")
    data class InvalidSSHCipher(val cipher: String) : ValidationError("Invalid SSH cipher: $cipher")
    data class InvalidSudoRule(val rule: String) : ValidationError("Invalid sudo rule: $rule")
    data class InvalidPAMModule(val module: String) : ValidationError("Invalid PAM module: $module")
    data class InvalidFirewallRule(val rule: String) : ValidationError("Invalid firewall rule: $rule")
    data class InvalidGPGKeyId(val keyId: String) : ValidationError("Invalid GPG key ID: $keyId")
    data class InvalidAuditRule(val rule: String) : ValidationError("Invalid audit rule: $rule")
    data class InvalidCertificatePath(val path: String) : ValidationError("Invalid certificate path: $path")
    data class InvalidPasswordPolicy(override val message: String) : ValidationError("Invalid password policy: $message")
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
        errors.addAll(SystemValidator.validateSystemConfig(config.system))
        
        // Validate packages
        errors.addAll(SystemValidator.validatePackages(config.packages))
        
        // Validate services
        errors.addAll(SystemValidator.validateServices(config.services))
        
        // Validate users
        errors.addAll(SystemValidator.validateUsers(config.users))
        
        // Validate repositories
        errors.addAll(SystemValidator.validateRepositories(config.repositories))
        
        // Validate desktop configuration
        config.desktop?.let { desktop ->
            errors.addAll(SystemValidator.validateDesktopConfig(desktop, config.users))
        }
        
        // Validate boot configuration
        config.boot?.let { boot ->
            errors.addAll(BootValidator.validateBootConfig(boot))
        }
        
        // Validate hardware configuration
        config.hardware?.let { hardware ->
            errors.addAll(HardwareValidator.validateHardwareConfig(hardware))
        }
        
        // Validate storage configuration
        config.storage?.let { storage ->
            errors.addAll(StorageValidator.validateStorageConfig(storage))
        }
        
        // Validate security configuration
        config.security?.let { security ->
            errors.addAll(SecurityValidator.validateSecurityConfig(security))
        }
        
        return ValidationResult(errors)
    }
}