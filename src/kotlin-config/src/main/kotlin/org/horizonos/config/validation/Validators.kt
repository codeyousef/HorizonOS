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