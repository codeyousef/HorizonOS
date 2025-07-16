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
        
        // Validate boot configuration
        config.boot?.let { boot ->
            errors.addAll(validateBootConfig(boot))
        }
        
        // Validate hardware configuration
        config.hardware?.let { hardware ->
            errors.addAll(validateHardwareConfig(hardware))
        }
        
        // Validate storage configuration
        config.storage?.let { storage ->
            errors.addAll(validateStorageConfig(storage))
        }
        
        // Validate security configuration
        config.security?.let { security ->
            errors.addAll(validateSecurityConfig(security))
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
    
    private fun validateBootConfig(boot: BootConfig): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        // Validate bootloader configuration
        errors.addAll(validateBootloaderConfig(boot.bootloader))
        
        // Validate kernel configuration
        errors.addAll(validateKernelConfig(boot.kernel))
        
        // Validate initramfs configuration
        errors.addAll(validateInitramfsConfig(boot.initramfs))
        
        // Validate Plymouth configuration
        errors.addAll(validatePlymouthConfig(boot.plymouth))
        
        // Validate Secure Boot configuration
        errors.addAll(validateSecureBootConfig(boot.secureBoot))
        
        return errors
    }
    
    private fun validateBootloaderConfig(bootloader: BootloaderConfig): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        // Validate boot entries
        bootloader.entries.forEach { entry ->
            // Validate Linux kernel path
            if (!isValidBootEntryPath(entry.linux)) {
                errors.add(ValidationError.InvalidBootEntryPath(entry.linux))
            }
            
            // Validate initrd path if provided
            entry.initrd?.let { initrd ->
                if (!isValidBootEntryPath(initrd)) {
                    errors.add(ValidationError.InvalidBootEntryPath(initrd))
                }
            }
            
            // Validate devicetree path if provided
            entry.devicetree?.let { dt ->
                if (!isValidBootEntryPath(dt)) {
                    errors.add(ValidationError.InvalidBootEntryPath(dt))
                }
            }
        }
        
        // Check for duplicate boot entry titles
        val duplicateTitles = bootloader.entries.groupBy { it.title }
            .filter { it.value.size > 1 }
            .keys
        duplicateTitles.forEach { title ->
            errors.add(ValidationError.ConflictingBootEntries(title))
        }
        
        return errors
    }
    
    private fun validateKernelConfig(kernel: KernelConfig): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        // Validate kernel parameters
        kernel.parameters.forEach { param ->
            if (!isValidKernelParameter(param.name)) {
                errors.add(ValidationError.InvalidKernelParameter(param.name))
            }
        }
        
        // Validate kernel modules
        kernel.modules.blacklist.forEach { module ->
            if (!isValidModuleName(module)) {
                errors.add(ValidationError.InvalidModule(module))
            }
        }
        
        kernel.modules.load.forEach { module ->
            if (!isValidModuleName(module)) {
                errors.add(ValidationError.InvalidModule(module))
            }
        }
        
        kernel.modules.options.keys.forEach { module ->
            if (!isValidModuleName(module)) {
                errors.add(ValidationError.InvalidModule(module))
            }
        }
        
        // Validate kernel variants
        kernel.variants.forEach { variant ->
            variant.parameters.forEach { param ->
                if (!isValidKernelParameter(param.name)) {
                    errors.add(ValidationError.InvalidKernelParameter(param.name))
                }
            }
        }
        
        return errors
    }
    
    private fun validateInitramfsConfig(initramfs: InitramfsConfig): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        // Validate modules
        initramfs.modules.forEach { module ->
            if (!isValidModuleName(module)) {
                errors.add(ValidationError.InvalidModule(module))
            }
        }
        
        // Validate hooks
        initramfs.hooks.forEach { hook ->
            if (!isValidInitramfsHook(hook)) {
                errors.add(ValidationError.InvalidInitramfsHook(hook))
            }
        }
        
        // Validate files (basic path validation)
        initramfs.files.forEach { file ->
            if (!isValidBootEntryPath(file)) {
                errors.add(ValidationError.InvalidBootEntryPath(file))
            }
        }
        
        // Validate custom scripts
        initramfs.customScripts.forEach { script ->
            if (!isValidBootEntryPath(script)) {
                errors.add(ValidationError.InvalidBootEntryPath(script))
            }
        }
        
        return errors
    }
    
    private fun validatePlymouthConfig(plymouth: PlymouthConfig): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        // Validate theme name
        if (!isValidPlymouthTheme(plymouth.theme)) {
            errors.add(ValidationError.InvalidPlymouthTheme(plymouth.theme))
        }
        
        // Validate modules
        plymouth.modules.forEach { module ->
            if (!isValidModuleName(module)) {
                errors.add(ValidationError.InvalidModule(module))
            }
        }
        
        return errors
    }
    
    private fun validateSecureBootConfig(secureBoot: SecureBootConfig): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        // Validate Secure Boot keys if provided
        secureBoot.keys?.let { keys ->
            keys.platform?.let { path ->
                if (!isValidSecureBootKeyPath(path)) {
                    errors.add(ValidationError.InvalidSecureBootKey(path))
                }
            }
            
            keys.keyExchange?.let { path ->
                if (!isValidSecureBootKeyPath(path)) {
                    errors.add(ValidationError.InvalidSecureBootKey(path))
                }
            }
            
            keys.signature?.let { path ->
                if (!isValidSecureBootKeyPath(path)) {
                    errors.add(ValidationError.InvalidSecureBootKey(path))
                }
            }
            
            keys.forbidden?.let { path ->
                if (!isValidSecureBootKeyPath(path)) {
                    errors.add(ValidationError.InvalidSecureBootKey(path))
                }
            }
        }
        
        return errors
    }
    
    private fun validateHardwareConfig(hardware: HardwareConfig): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        // Validate GPU configuration
        errors.addAll(validateGPUConfig(hardware.gpu))
        
        // Validate display configuration
        errors.addAll(validateDisplayConfig(hardware.display))
        
        // Validate power configuration
        errors.addAll(validatePowerConfig(hardware.power))
        
        // Validate audio configuration
        errors.addAll(validateAudioConfig(hardware.audio))
        
        // Validate Bluetooth configuration
        errors.addAll(validateBluetoothConfig(hardware.bluetooth))
        
        // Validate USB configuration
        errors.addAll(validateUSBConfig(hardware.usb))
        
        // Validate thermal configuration
        errors.addAll(validateThermalConfig(hardware.thermal))
        
        return errors
    }
    
    private fun validateGPUConfig(gpu: GPUConfig): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        // Validate GPU drivers
        gpu.drivers.forEach { driver ->
            // Validate driver options
            driver.options.keys.forEach { option ->
                if (!isValidGPUOption(option)) {
                    errors.add(ValidationError.InvalidGPUDriver("Invalid driver option: $option"))
                }
            }
            
            // Validate firmware files
            driver.firmwareFiles.forEach { firmware ->
                if (!isValidBootEntryPath(firmware)) {
                    errors.add(ValidationError.InvalidBootEntryPath(firmware))
                }
            }
            
            // Validate blacklisted drivers
            driver.blacklistedDrivers.forEach { blacklisted ->
                if (!isValidModuleName(blacklisted)) {
                    errors.add(ValidationError.InvalidModule(blacklisted))
                }
            }
        }
        
        // Validate multi-GPU configuration
        gpu.multiGPU?.let { multiGPU ->
            multiGPU.primaryGPU?.let { primary ->
                if (!isValidGPUIdentifier(primary)) {
                    errors.add(ValidationError.InvalidGPUDriver("Invalid primary GPU identifier: $primary"))
                }
            }
            
            multiGPU.discreteGPU?.let { discrete ->
                if (!isValidGPUIdentifier(discrete)) {
                    errors.add(ValidationError.InvalidGPUDriver("Invalid discrete GPU identifier: $discrete"))
                }
            }
        }
        
        return errors
    }
    
    private fun validateDisplayConfig(display: DisplayConfig): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        // Validate monitors
        display.monitors.forEach { monitor ->
            // Validate resolution
            monitor.resolution?.let { resolution ->
                if (!isValidResolution(resolution)) {
                    errors.add(ValidationError.InvalidDisplayResolution("${resolution.width}x${resolution.height}"))
                }
            }
            
            // Validate refresh rate
            monitor.refreshRate?.let { rate ->
                if (!isValidRefreshRate(rate)) {
                    errors.add(ValidationError.InvalidRefreshRate(rate))
                }
            }
            
            // Validate color profile path
            monitor.colorProfile?.let { profile ->
                if (!isValidColorProfilePath(profile)) {
                    errors.add(ValidationError.InvalidBootEntryPath(profile))
                }
            }
            
            // Validate gamma values
            if (!isValidGamma(monitor.gamma.red) || 
                !isValidGamma(monitor.gamma.green) || 
                !isValidGamma(monitor.gamma.blue)) {
                errors.add(ValidationError.InvalidDisplayResolution("Invalid gamma values for monitor ${monitor.name}"))
            }
        }
        
        // Check for duplicate monitor names
        val duplicateMonitors = display.monitors.groupBy { it.name }
            .filter { it.value.size > 1 }
            .keys
        duplicateMonitors.forEach { monitorName ->
            errors.add(ValidationError.ConflictingMonitors(monitorName))
        }
        
        // Validate color management
        display.color.profiles.forEach { profile ->
            if (!isValidColorProfilePath(profile.path)) {
                errors.add(ValidationError.InvalidBootEntryPath(profile.path))
            }
        }
        
        // Validate night light schedule
        if (display.nightLight.schedule == NightLightSchedule.MANUAL) {
            if (!isValidTimeFormat(display.nightLight.manualStart) || 
                !isValidTimeFormat(display.nightLight.manualEnd)) {
                errors.add(ValidationError.InvalidDisplayResolution("Invalid night light schedule times"))
            }
        }
        
        return errors
    }
    
    private fun validatePowerConfig(power: PowerConfig): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        // Validate CPU power configuration
        power.cpu.minFreq?.let { freq ->
            if (!isValidFrequency(freq)) {
                errors.add(ValidationError.InvalidPowerProfile("Invalid minimum CPU frequency: $freq"))
            }
        }
        
        power.cpu.maxFreq?.let { freq ->
            if (!isValidFrequency(freq)) {
                errors.add(ValidationError.InvalidPowerProfile("Invalid maximum CPU frequency: $freq"))
            }
        }
        
        // Validate C-state configuration
        power.cpu.cStates.disabledStates.forEach { state ->
            if (state < 0 || state > 10) {
                errors.add(ValidationError.InvalidPowerProfile("Invalid C-state: $state"))
            }
        }
        
        // Validate P-state configuration
        if (power.cpu.pStates.minPerf < 0 || power.cpu.pStates.minPerf > 100) {
            errors.add(ValidationError.InvalidPowerProfile("Invalid minimum P-state performance: ${power.cpu.pStates.minPerf}"))
        }
        
        if (power.cpu.pStates.maxPerf < 0 || power.cpu.pStates.maxPerf > 100) {
            errors.add(ValidationError.InvalidPowerProfile("Invalid maximum P-state performance: ${power.cpu.pStates.maxPerf}"))
        }
        
        // Validate battery configuration
        if (power.battery.chargingThreshold.startThreshold < 0 || power.battery.chargingThreshold.startThreshold > 100) {
            errors.add(ValidationError.InvalidPowerProfile("Invalid charging start threshold: ${power.battery.chargingThreshold.startThreshold}"))
        }
        
        if (power.battery.chargingThreshold.stopThreshold < 0 || power.battery.chargingThreshold.stopThreshold > 100) {
            errors.add(ValidationError.InvalidPowerProfile("Invalid charging stop threshold: ${power.battery.chargingThreshold.stopThreshold}"))
        }
        
        return errors
    }
    
    private fun validateAudioConfig(audio: AudioConfig): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        // Validate audio devices
        audio.devices.forEach { device ->
            if (!isValidAudioDeviceName(device.name)) {
                errors.add(ValidationError.InvalidAudioDevice(device.name))
            }
            
            // Validate sample rate
            if (!isValidSampleRate(device.sampleRate)) {
                errors.add(ValidationError.InvalidAudioDevice("Invalid sample rate: ${device.sampleRate}"))
            }
            
            // Validate bit depth
            if (!isValidBitDepth(device.bitDepth)) {
                errors.add(ValidationError.InvalidAudioDevice("Invalid bit depth: ${device.bitDepth}"))
            }
            
            // Validate buffer size
            if (!isValidBufferSize(device.bufferSize)) {
                errors.add(ValidationError.InvalidAudioDevice("Invalid buffer size: ${device.bufferSize}"))
            }
        }
        
        // Validate volume configuration
        if (audio.volume.master < 0 || audio.volume.master > audio.volume.maxVolume) {
            errors.add(ValidationError.InvalidAudioDevice("Invalid master volume: ${audio.volume.master}"))
        }
        
        return errors
    }
    
    private fun validateBluetoothConfig(bluetooth: BluetoothConfig): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        // Validate Bluetooth devices
        bluetooth.devices.forEach { device ->
            if (!isValidBluetoothAddress(device.address)) {
                errors.add(ValidationError.InvalidBluetoothAddress(device.address))
            }
        }
        
        return errors
    }
    
    private fun validateUSBConfig(usb: USBConfig): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        // Validate USB devices
        usb.devices.forEach { device ->
            if (!isValidUSBVendorId(device.vendorId) || !isValidUSBProductId(device.productId)) {
                errors.add(ValidationError.InvalidUSBDevice("${device.vendorId}:${device.productId}"))
            }
        }
        
        // Validate autosuspend blacklist
        usb.autosuspend.blacklist.forEach { device ->
            if (!isValidUSBDeviceIdentifier(device)) {
                errors.add(ValidationError.InvalidUSBDevice(device))
            }
        }
        
        return errors
    }
    
    private fun validateThermalConfig(thermal: ThermalConfig): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        // Validate thermal zones
        thermal.zones.forEach { zone ->
            if (!isValidThermalZoneName(zone.name)) {
                errors.add(ValidationError.InvalidThermalZone(zone.name))
            }
            
            // Validate temperature thresholds
            if (zone.criticalTemp <= zone.hotTemp || zone.hotTemp <= zone.passiveTemp) {
                errors.add(ValidationError.InvalidThermalZone("Invalid temperature thresholds for zone ${zone.name}"))
            }
        }
        
        return errors
    }
    
    private fun validateStorageConfig(storage: StorageConfig): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        // Validate filesystems
        errors.addAll(validateFilesystems(storage.filesystems))
        
        // Validate RAID configuration
        errors.addAll(validateRAIDConfig(storage.raid))
        
        // Validate encryption configuration
        errors.addAll(validateEncryptionConfig(storage.encryption))
        
        // Validate Btrfs configuration
        errors.addAll(validateBtrfsConfig(storage.btrfs))
        
        // Validate swap configuration
        errors.addAll(validateSwapConfig(storage.swap))
        
        // Validate auto-mount configuration
        errors.addAll(validateAutoMountConfig(storage.autoMount))
        
        return errors
    }
    
    private fun validateFilesystems(filesystems: List<FilesystemConfig>): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        filesystems.forEach { fs ->
            // Validate device path
            if (!isValidDevicePath(fs.device)) {
                errors.add(ValidationError.InvalidDevicePath(fs.device))
            }
            
            // Validate mount point
            if (!isValidMountPoint(fs.mountPoint)) {
                errors.add(ValidationError.InvalidMountPoint(fs.mountPoint))
            }
            
            // Validate backup frequency
            if (fs.backupFrequency < 0 || fs.backupFrequency > 2) {
                errors.add(ValidationError.InvalidBootEntryPath("Invalid backup frequency: ${fs.backupFrequency}"))
            }
            
            // Validate fsck order
            if (fs.fsckOrder < 0 || fs.fsckOrder > 2) {
                errors.add(ValidationError.InvalidBootEntryPath("Invalid fsck order: ${fs.fsckOrder}"))
            }
        }
        
        // Check for duplicate mount points
        val duplicateMountPoints = filesystems.groupBy { it.mountPoint }
            .filter { it.value.size > 1 }
            .keys
        duplicateMountPoints.forEach { mountPoint ->
            errors.add(ValidationError.ConflictingMountPoints(mountPoint))
        }
        
        return errors
    }
    
    private fun validateRAIDConfig(raid: RAIDStorageConfig): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        if (raid.enabled) {
            raid.arrays.forEach { array ->
                // Validate device paths
                array.devices.forEach { device ->
                    if (!isValidDevicePath(device)) {
                        errors.add(ValidationError.InvalidDevicePath(device))
                    }
                }
                
                // Validate spare devices
                array.spares.forEach { spare ->
                    if (!isValidDevicePath(spare)) {
                        errors.add(ValidationError.InvalidDevicePath(spare))
                    }
                }
                
                // Validate RAID level requirements
                when (array.level) {
                    RAIDLevel.RAID1 -> {
                        if (array.devices.size < 2) {
                            errors.add(ValidationError.InvalidRAIDLevel("RAID1 requires at least 2 devices"))
                        }
                    }
                    RAIDLevel.RAID5 -> {
                        if (array.devices.size < 3) {
                            errors.add(ValidationError.InvalidRAIDLevel("RAID5 requires at least 3 devices"))
                        }
                    }
                    RAIDLevel.RAID6 -> {
                        if (array.devices.size < 4) {
                            errors.add(ValidationError.InvalidRAIDLevel("RAID6 requires at least 4 devices"))
                        }
                    }
                    RAIDLevel.RAID10 -> {
                        if (array.devices.size < 4 || array.devices.size % 2 != 0) {
                            errors.add(ValidationError.InvalidRAIDLevel("RAID10 requires at least 4 devices and even number of devices"))
                        }
                    }
                    else -> {}
                }
                
                // Validate chunk size
                array.chunkSize?.let { chunkSize ->
                    if (!isValidChunkSize(chunkSize)) {
                        errors.add(ValidationError.InvalidRAIDLevel("Invalid chunk size: $chunkSize"))
                    }
                }
            }
            
            // Validate email address for notifications
            raid.monitoring.emailAddress?.let { email ->
                if (!isValidEmailAddress(email)) {
                    errors.add(ValidationError.InvalidUrl("Invalid email address: $email"))
                }
            }
        }
        
        return errors
    }
    
    private fun validateEncryptionConfig(encryption: EncryptionConfig): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        if (encryption.enabled) {
            encryption.volumes.forEach { volume ->
                // Validate device path
                if (!isValidDevicePath(volume.device)) {
                    errors.add(ValidationError.InvalidDevicePath(volume.device))
                }
                
                // Validate key size
                if (!isValidKeySize(volume.keySize)) {
                    errors.add(ValidationError.InvalidKeySize(volume.keySize))
                }
                
                // Validate keyfile paths
                volume.keyFile?.let { keyFile ->
                    if (!isValidBootEntryPath(keyFile)) {
                        errors.add(ValidationError.InvalidBootEntryPath(keyFile))
                    }
                }
                
                // Validate key slots
                volume.keySlots.forEach { keySlot ->
                    if (keySlot.slot < 0 || keySlot.slot > 31) {
                        errors.add(ValidationError.InvalidKeySize(keySlot.slot))
                    }
                }
            }
            
            // Validate keyfiles
            encryption.keyfiles.forEach { keyfile ->
                if (!isValidBootEntryPath(keyfile.path)) {
                    errors.add(ValidationError.InvalidBootEntryPath(keyfile.path))
                }
                
                if (keyfile.size < 1 || keyfile.size > 8192) {
                    errors.add(ValidationError.InvalidKeySize(keyfile.size))
                }
            }
        }
        
        return errors
    }
    
    private fun validateBtrfsConfig(btrfs: BtrfsConfig): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        if (btrfs.enabled) {
            btrfs.filesystems.forEach { fs ->
                // Validate device paths
                fs.devices.forEach { device ->
                    if (!isValidDevicePath(device)) {
                        errors.add(ValidationError.InvalidDevicePath(device))
                    }
                }
                
                // Validate subvolumes
                fs.subvolumes.forEach { subvol ->
                    if (!isValidSubvolumePath(subvol.path)) {
                        errors.add(ValidationError.InvalidBootEntryPath(subvol.path))
                    }
                    
                    subvol.mountPoint?.let { mountPoint ->
                        if (!isValidMountPoint(mountPoint)) {
                            errors.add(ValidationError.InvalidMountPoint(mountPoint))
                        }
                    }
                }
                
                // Check for duplicate subvolume names
                val duplicateNames = fs.subvolumes.groupBy { it.name }
                    .filter { it.value.size > 1 }
                    .keys
                duplicateNames.forEach { name ->
                    errors.add(ValidationError.ConflictingMountPoints("Duplicate subvolume name: $name"))
                }
            }
        }
        
        return errors
    }
    
    private fun validateSwapConfig(swap: SwapConfig): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        if (swap.enabled) {
            // Validate swap size
            if (!isValidSwapSize(swap.size)) {
                errors.add(ValidationError.InvalidSwapSize(swap.size))
            }
            
            // Validate swap files
            swap.files.forEach { swapFile ->
                if (!isValidBootEntryPath(swapFile.path)) {
                    errors.add(ValidationError.InvalidBootEntryPath(swapFile.path))
                }
                
                if (!isValidSwapSize(swapFile.size)) {
                    errors.add(ValidationError.InvalidSwapSize(swapFile.size))
                }
                
                if (swapFile.priority < -1 || swapFile.priority > 32767) {
                    errors.add(ValidationError.InvalidSwapSize("Invalid swap priority: ${swapFile.priority}"))
                }
            }
            
            // Validate swap partitions
            swap.partitions.forEach { partition ->
                if (!isValidDevicePath(partition.device)) {
                    errors.add(ValidationError.InvalidDevicePath(partition.device))
                }
            }
            
            // Validate swappiness
            if (swap.swappiness < 0 || swap.swappiness > 100) {
                errors.add(ValidationError.InvalidSwapSize("Invalid swappiness: ${swap.swappiness}"))
            }
            
            // Validate VFS cache pressure
            if (swap.vfsCache < 0 || swap.vfsCache > 1000) {
                errors.add(ValidationError.InvalidSwapSize("Invalid VFS cache pressure: ${swap.vfsCache}"))
            }
        }
        
        return errors
    }
    
    private fun validateAutoMountConfig(autoMount: AutoMountConfig): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        if (autoMount.enabled) {
            // Validate mount point
            if (!isValidMountPoint(autoMount.removableMedia.mountPoint)) {
                errors.add(ValidationError.InvalidMountPoint(autoMount.removableMedia.mountPoint))
            }
            
            // Validate network shares configuration
            autoMount.networkShares.samba.credentials?.let { credentials ->
                if (!isValidBootEntryPath(credentials)) {
                    errors.add(ValidationError.InvalidBootEntryPath(credentials))
                }
            }
            
            autoMount.networkShares.ssh.identityFile?.let { identityFile ->
                if (!isValidBootEntryPath(identityFile)) {
                    errors.add(ValidationError.InvalidBootEntryPath(identityFile))
                }
            }
        }
        
        return errors
    }
    
    // Storage validation helper functions
    private fun isValidDevicePath(path: String): Boolean {
        return path.isNotBlank() && (
            path.startsWith("/dev/") ||
            path.startsWith("UUID=") ||
            path.startsWith("LABEL=") ||
            path.startsWith("PARTUUID=") ||
            path.startsWith("PARTLABEL=")
        )
    }
    
    private fun isValidMountPoint(path: String): Boolean {
        return path.isNotBlank() && path.startsWith("/") && path != "/"
    }
    
    private fun isValidChunkSize(chunkSize: String): Boolean {
        return chunkSize.matches(Regex("^\\d+[kKmMgG]?$"))
    }
    
    private fun isValidEmailAddress(email: String): Boolean {
        return email.matches(Regex("^[A-Za-z0-9+_.-]+@[A-Za-z0-9.-]+\\.[A-Za-z]{2,}$"))
    }
    
    private fun isValidKeySize(keySize: Int): Boolean {
        return keySize in listOf(128, 256, 512, 1024, 2048, 4096)
    }
    
    private fun isValidSubvolumePath(path: String): Boolean {
        return path.isNotBlank() && !path.contains("..") && !path.startsWith("/")
    }
    
    private fun isValidSwapSize(size: String): Boolean {
        return size == "auto" || size.matches(Regex("^\\d+[kKmMgGtT]?$")) || size.matches(Regex("^\\d+%$"))
    }
    
    private fun validateSecurityConfig(security: SecurityConfig): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        if (security.enabled) {
            // Validate SSH configuration
            errors.addAll(validateSSHConfig(security.ssh))
            
            // Validate sudo configuration
            errors.addAll(validateSudoConfig(security.sudo))
            
            // Validate PAM configuration
            errors.addAll(validatePAMConfig(security.pam))
            
            // Validate firewall configuration
            errors.addAll(validateFirewallConfig(security.firewall))
            
            // Validate GPG configuration
            errors.addAll(validateGPGConfig(security.gpg))
            
            // Validate audit configuration
            errors.addAll(validateAuditConfig(security.audit))
            
            // Validate certificate configuration
            errors.addAll(validateCertificateConfig(security.certificates))
        }
        
        return errors
    }
    
    private fun validateSSHConfig(ssh: SSHConfig): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        if (ssh.enabled) {
            // Validate SSH port
            if (!isValidSSHPort(ssh.port)) {
                errors.add(ValidationError.InvalidSSHPort(ssh.port))
            }
            
            // Validate listen addresses
            ssh.listenAddress.forEach { address ->
                if (!isValidIpAddress(address) && address != "0.0.0.0" && address != "::") {
                    errors.add(ValidationError.InvalidUrl("Invalid SSH listen address: $address"))
                }
            }
            
            // Validate ciphers
            ssh.encryption.ciphers.forEach { cipher ->
                if (!isValidSSHCipher(cipher)) {
                    errors.add(ValidationError.InvalidSSHCipher(cipher))
                }
            }
            
            // Validate authentication settings
            if (ssh.authentication.maxAuthTries < 1 || ssh.authentication.maxAuthTries > 10) {
                errors.add(ValidationError.InvalidSSHPort(ssh.authentication.maxAuthTries))
            }
            
            // Validate max sessions
            if (ssh.access.maxSessions < 1 || ssh.access.maxSessions > 1000) {
                errors.add(ValidationError.InvalidSSHPort(ssh.access.maxSessions))
            }
            
            // Validate host keys
            ssh.keys.hostKeys.forEach { hostKey ->
                if (!isValidBootEntryPath(hostKey.path)) {
                    errors.add(ValidationError.InvalidBootEntryPath(hostKey.path))
                }
            }
            
            // Validate authorized keys
            ssh.keys.authorizedKeys.forEach { authKey ->
                if (!isValidUsername(authKey.user)) {
                    errors.add(ValidationError.InvalidUsername(authKey.user))
                }
            }
        }
        
        return errors
    }
    
    private fun validateSudoConfig(sudo: SudoConfig): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        if (sudo.enabled) {
            // Validate sudo rules
            sudo.rules.forEach { rule ->
                // Validate user
                if (!isValidUsername(rule.user) && !rule.user.startsWith("%") && rule.user != "ALL") {
                    errors.add(ValidationError.InvalidSudoRule("Invalid user in sudo rule: ${rule.user}"))
                }
                
                // Validate commands
                if (rule.commands.isEmpty()) {
                    errors.add(ValidationError.InvalidSudoRule("Sudo rule must have at least one command"))
                }
                
                rule.commands.forEach { command ->
                    if (command.isBlank()) {
                        errors.add(ValidationError.InvalidSudoRule("Empty command in sudo rule"))
                    }
                }
            }
            
            // Validate password timeout
            if (sudo.defaults.passwordRetries < 1 || sudo.defaults.passwordRetries > 10) {
                errors.add(ValidationError.InvalidSudoRule("Invalid password retries: ${sudo.defaults.passwordRetries}"))
            }
            
            // Validate log file path
            if (!isValidBootEntryPath(sudo.defaults.logFile)) {
                errors.add(ValidationError.InvalidBootEntryPath(sudo.defaults.logFile))
            }
        }
        
        return errors
    }
    
    private fun validatePAMConfig(pam: PAMConfig): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        if (pam.enabled) {
            // Validate password policy
            val policy = pam.passwordPolicy
            if (policy.minLength < 1 || policy.minLength > 256) {
                errors.add(ValidationError.InvalidPasswordPolicy("Invalid minimum length: ${policy.minLength}"))
            }
            
            if (policy.maxLength < policy.minLength) {
                errors.add(ValidationError.InvalidPasswordPolicy("Maximum length must be greater than minimum length"))
            }
            
            if (policy.historySize < 0 || policy.historySize > 100) {
                errors.add(ValidationError.InvalidPasswordPolicy("Invalid history size: ${policy.historySize}"))
            }
            
            // Validate dictionary paths
            policy.dictionary.dictionaries.forEach { dict ->
                if (!isValidBootEntryPath(dict)) {
                    errors.add(ValidationError.InvalidBootEntryPath(dict))
                }
            }
            
            // Validate PAM modules
            pam.modules.forEach { module ->
                if (!isValidPAMModule(module.module)) {
                    errors.add(ValidationError.InvalidPAMModule(module.module))
                }
            }
            
            // Validate account lockout settings
            if (pam.lockout.enabled) {
                if (pam.lockout.maxAttempts < 1 || pam.lockout.maxAttempts > 100) {
                    errors.add(ValidationError.InvalidPasswordPolicy("Invalid lockout max attempts: ${pam.lockout.maxAttempts}"))
                }
            }
        }
        
        return errors
    }
    
    private fun validateFirewallConfig(firewall: FirewallConfig): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        if (firewall.enabled) {
            // Validate firewall rules
            firewall.rules.forEach { rule ->
                // Validate source/destination addresses
                rule.source?.let { source ->
                    if (!isValidIpAddressOrRange(source)) {
                        errors.add(ValidationError.InvalidFirewallRule("Invalid source address: $source"))
                    }
                }
                
                rule.destination?.let { dest ->
                    if (!isValidIpAddressOrRange(dest)) {
                        errors.add(ValidationError.InvalidFirewallRule("Invalid destination address: $dest"))
                    }
                }
                
                // Validate port ranges
                rule.port?.let { port ->
                    if (!isValidPortRange(port)) {
                        errors.add(ValidationError.InvalidFirewallRule("Invalid port specification: $port"))
                    }
                }
                
                // Validate priority
                if (rule.priority < 0 || rule.priority > 1000) {
                    errors.add(ValidationError.InvalidFirewallRule("Invalid rule priority: ${rule.priority}"))
                }
            }
            
            // Validate zones
            firewall.zones.forEach { zone ->
                zone.ports.forEach { port ->
                    if (!isValidPortRange(port)) {
                        errors.add(ValidationError.InvalidFirewallRule("Invalid zone port: $port"))
                    }
                }
                
                zone.sources.forEach { source ->
                    if (!isValidIpAddressOrRange(source)) {
                        errors.add(ValidationError.InvalidFirewallRule("Invalid zone source: $source"))
                    }
                }
            }
        }
        
        return errors
    }
    
    private fun validateGPGConfig(gpg: GPGConfig): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        if (gpg.enabled) {
            // Validate keyserver URL
            if (!isValidUrl(gpg.keyserver)) {
                errors.add(ValidationError.InvalidUrl(gpg.keyserver))
            }
            
            // Validate GPG keys
            gpg.keys.forEach { key ->
                if (!isValidGPGKeyId(key.keyId)) {
                    errors.add(ValidationError.InvalidGPGKeyId(key.keyId))
                }
                
                if (!isValidGPGKeyId(key.fingerprint)) {
                    errors.add(ValidationError.InvalidGPGKeyId(key.fingerprint))
                }
                
                key.keyFile?.let { keyFile ->
                    if (!isValidBootEntryPath(keyFile)) {
                        errors.add(ValidationError.InvalidBootEntryPath(keyFile))
                    }
                }
            }
            
            // Validate default key
            gpg.defaultKey?.let { defaultKey ->
                if (!isValidGPGKeyId(defaultKey)) {
                    errors.add(ValidationError.InvalidGPGKeyId(defaultKey))
                }
            }
        }
        
        return errors
    }
    
    private fun validateAuditConfig(audit: AuditConfig): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        if (audit.enabled) {
            // Validate buffer size
            if (audit.bufferSize < 1024 || audit.bufferSize > 1048576) {
                errors.add(ValidationError.InvalidAuditRule("Invalid buffer size: ${audit.bufferSize}"))
            }
            
            // Validate max log file
            if (audit.maxLogFile < 1 || audit.maxLogFile > 1000) {
                errors.add(ValidationError.InvalidAuditRule("Invalid max log file: ${audit.maxLogFile}"))
            }
            
            // Validate space settings
            if (audit.spaceLeft < 1 || audit.spaceLeft > 100) {
                errors.add(ValidationError.InvalidAuditRule("Invalid space left: ${audit.spaceLeft}"))
            }
            
            if (audit.adminSpaceLeft < 1 || audit.adminSpaceLeft > audit.spaceLeft) {
                errors.add(ValidationError.InvalidAuditRule("Invalid admin space left: ${audit.adminSpaceLeft}"))
            }
            
            // Validate TCP settings
            audit.tcpListenPort?.let { port ->
                if (!isValidSSHPort(port)) {
                    errors.add(ValidationError.InvalidSSHPort(port))
                }
            }
            
            // Validate audit rules
            audit.rules.forEach { rule ->
                if (rule.rule.isBlank()) {
                    errors.add(ValidationError.InvalidAuditRule("Empty audit rule: ${rule.name}"))
                }
            }
        }
        
        return errors
    }
    
    private fun validateCertificateConfig(certificates: CertificateConfig): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        if (certificates.enabled) {
            // Validate CA configuration
            if (certificates.ca.enabled) {
                if (!isValidBootEntryPath(certificates.ca.path)) {
                    errors.add(ValidationError.InvalidCertificatePath(certificates.ca.path))
                }
                
                if (!isValidBootEntryPath(certificates.ca.keyPath)) {
                    errors.add(ValidationError.InvalidCertificatePath(certificates.ca.keyPath))
                }
                
                if (certificates.ca.keySize < 1024 || certificates.ca.keySize > 8192) {
                    errors.add(ValidationError.InvalidKeySize(certificates.ca.keySize))
                }
            }
            
            // Validate certificates
            certificates.certificates.forEach { cert ->
                if (!isValidBootEntryPath(cert.keyFile)) {
                    errors.add(ValidationError.InvalidCertificatePath(cert.keyFile))
                }
                
                if (!isValidBootEntryPath(cert.certFile)) {
                    errors.add(ValidationError.InvalidCertificatePath(cert.certFile))
                }
                
                cert.caFile?.let { caFile ->
                    if (!isValidBootEntryPath(caFile)) {
                        errors.add(ValidationError.InvalidCertificatePath(caFile))
                    }
                }
                
                if (cert.keySize < 1024 || cert.keySize > 8192) {
                    errors.add(ValidationError.InvalidKeySize(cert.keySize))
                }
                
                // Validate common name
                if (cert.commonName.isBlank()) {
                    errors.add(ValidationError.InvalidCertificatePath("Empty common name for certificate: ${cert.name}"))
                }
            }
            
            // Validate certificate store
            if (!isValidBootEntryPath(certificates.store.path)) {
                errors.add(ValidationError.InvalidCertificatePath(certificates.store.path))
            }
            
            if (!isValidBootEntryPath(certificates.store.caBundle)) {
                errors.add(ValidationError.InvalidCertificatePath(certificates.store.caBundle))
            }
        }
        
        return errors
    }
    
    // Security validation helper functions
    private fun isValidSSHPort(port: Int): Boolean {
        return port in 1..65535
    }
    
    private fun isValidSSHCipher(cipher: String): Boolean {
        val validCiphers = listOf(
            "aes128-ctr", "aes192-ctr", "aes256-ctr",
            "aes128-gcm@openssh.com", "aes256-gcm@openssh.com",
            "chacha20-poly1305@openssh.com"
        )
        return cipher in validCiphers || cipher.matches(Regex("^[a-zA-Z0-9@.-]+$"))
    }
    
    private fun isValidPAMModule(module: String): Boolean {
        return module.matches(Regex("^[a-zA-Z0-9._-]+$")) && module.isNotBlank()
    }
    
    private fun isValidIpAddress(address: String): Boolean {
        return address.matches(Regex("^\\d{1,3}\\.\\d{1,3}\\.\\d{1,3}\\.\\d{1,3}$")) ||
               address.matches(Regex("^([0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}$"))
    }
    
    private fun isValidIpAddressOrRange(address: String): Boolean {
        return isValidIpAddress(address) ||
               address.matches(Regex("^\\d{1,3}\\.\\d{1,3}\\.\\d{1,3}\\.\\d{1,3}/\\d{1,2}$")) ||
               address.matches(Regex("^([0-9a-fA-F]{1,4}:){1,7}[0-9a-fA-F]{1,4}/\\d{1,3}$"))
    }
    
    private fun isValidPortRange(port: String): Boolean {
        return port.matches(Regex("^\\d{1,5}$")) ||
               port.matches(Regex("^\\d{1,5}-\\d{1,5}$")) ||
               port.matches(Regex("^\\d{1,5}:\\d{1,5}$"))
    }
    
    private fun isValidGPGKeyId(keyId: String): Boolean {
        return keyId.matches(Regex("^[0-9A-Fa-f]{8}$")) ||
               keyId.matches(Regex("^[0-9A-Fa-f]{16}$")) ||
               keyId.matches(Regex("^[0-9A-Fa-f]{40}$"))
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
    
    private fun isValidBootEntryPath(path: String): Boolean {
        // Validate boot entry paths (kernel, initrd, devicetree, files)
        return path.isNotBlank() && 
               (path.startsWith("/") || path.startsWith("\\") || 
                path.matches(Regex("^[a-zA-Z0-9._/-]+$")))
    }
    
    private fun isValidKernelParameter(parameter: String): Boolean {
        // Validate kernel parameter names (basic validation)
        return parameter.matches(Regex("^[a-zA-Z0-9._-]+$")) && parameter.isNotBlank()
    }
    
    private fun isValidModuleName(module: String): Boolean {
        // Validate kernel module names
        return module.matches(Regex("^[a-zA-Z0-9._-]+$")) && module.isNotBlank()
    }
    
    private fun isValidInitramfsHook(hook: String): Boolean {
        // Validate initramfs hook names
        return hook.matches(Regex("^[a-zA-Z0-9._-]+$")) && hook.isNotBlank()
    }
    
    private fun isValidPlymouthTheme(theme: String): Boolean {
        // Validate Plymouth theme names
        return theme.matches(Regex("^[a-zA-Z0-9._-]+$")) && theme.isNotBlank()
    }
    
    private fun isValidSecureBootKeyPath(keyPath: String): Boolean {
        // Validate Secure Boot key file paths
        return keyPath.isNotBlank() && 
               keyPath.startsWith("/") &&
               (keyPath.endsWith(".esl") || keyPath.endsWith(".auth") || 
                keyPath.endsWith(".crt") || keyPath.endsWith(".pem") ||
                keyPath.endsWith(".der") || keyPath.endsWith(".key"))
    }
    
    // Hardware validation helper functions
    private fun isValidGPUOption(option: String): Boolean {
        return option.matches(Regex("^[a-zA-Z0-9._-]+$")) && option.isNotBlank()
    }
    
    private fun isValidGPUIdentifier(identifier: String): Boolean {
        return identifier.matches(Regex("^[a-zA-Z0-9:._-]+$")) && identifier.isNotBlank()
    }
    
    private fun isValidResolution(resolution: Resolution): Boolean {
        return resolution.width > 0 && resolution.height > 0 &&
               resolution.width <= 8192 && resolution.height <= 8192
    }
    
    private fun isValidRefreshRate(rate: Double): Boolean {
        return rate > 0 && rate <= 500.0
    }
    
    private fun isValidColorProfilePath(path: String): Boolean {
        return path.isNotBlank() && path.startsWith("/") &&
               (path.endsWith(".icc") || path.endsWith(".icm"))
    }
    
    private fun isValidGamma(gamma: Double): Boolean {
        return gamma > 0.0 && gamma <= 3.0
    }
    
    private fun isValidTimeFormat(time: String): Boolean {
        return time.matches(Regex("^([01]?[0-9]|2[0-3]):[0-5][0-9]$"))
    }
    
    private fun isValidFrequency(freq: String): Boolean {
        return freq.matches(Regex("^\\d+(\\.\\d+)?[MG]Hz$"))
    }
    
    private fun isValidAudioDeviceName(name: String): Boolean {
        return name.matches(Regex("^[a-zA-Z0-9._: -]+$")) && name.isNotBlank()
    }
    
    private fun isValidSampleRate(rate: Int): Boolean {
        return rate in listOf(8000, 11025, 16000, 22050, 44100, 48000, 88200, 96000, 176400, 192000)
    }
    
    private fun isValidBitDepth(depth: Int): Boolean {
        return depth in listOf(8, 16, 24, 32)
    }
    
    private fun isValidBufferSize(size: Int): Boolean {
        return size > 0 && (size and (size - 1)) == 0 // Power of 2
    }
    
    private fun isValidBluetoothAddress(address: String): Boolean {
        return address.matches(Regex("^[0-9A-Fa-f]{2}:[0-9A-Fa-f]{2}:[0-9A-Fa-f]{2}:[0-9A-Fa-f]{2}:[0-9A-Fa-f]{2}:[0-9A-Fa-f]{2}$"))
    }
    
    private fun isValidUSBVendorId(vendorId: String): Boolean {
        return vendorId.matches(Regex("^[0-9A-Fa-f]{4}$"))
    }
    
    private fun isValidUSBProductId(productId: String): Boolean {
        return productId.matches(Regex("^[0-9A-Fa-f]{4}$"))
    }
    
    private fun isValidUSBDeviceIdentifier(identifier: String): Boolean {
        return identifier.matches(Regex("^[0-9A-Fa-f]{4}:[0-9A-Fa-f]{4}$")) ||
               identifier.matches(Regex("^[a-zA-Z0-9._-]+$"))
    }
    
    private fun isValidThermalZoneName(name: String): Boolean {
        return name.matches(Regex("^[a-zA-Z0-9._-]+$")) && name.isNotBlank()
    }
}