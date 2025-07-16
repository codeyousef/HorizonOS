package org.horizonos.config.validation.validators

import org.horizonos.config.dsl.*
import org.horizonos.config.validation.ValidationError

object HardwareValidator {
    
    fun validateHardwareConfig(hardware: HardwareConfig): List<ValidationError> {
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
        }
        
        // Validate OpenGL configuration
        gpu.opengl?.let { opengl ->
            if (opengl.driSupport && opengl.driDrivers.isEmpty()) {
                errors.add(ValidationError.InvalidGPUDriver("DRI support enabled but no drivers specified"))
            }
        }
        
        // Validate Vulkan configuration
        gpu.vulkan?.let { vulkan ->
            if (vulkan.enable && vulkan.drivers.isEmpty()) {
                errors.add(ValidationError.InvalidGPUDriver("Vulkan enabled but no drivers specified"))
            }
        }
        
        return errors
    }
    
    private fun validateDisplayConfig(display: DisplayConfig): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        // Validate monitors
        display.monitors.forEach { monitor ->
            // Validate resolution
            if (!isValidDisplayResolution(monitor.resolution)) {
                errors.add(ValidationError.InvalidDisplayResolution(monitor.resolution))
            }
            
            // Validate refresh rate
            if (!isValidRefreshRate(monitor.refreshRate)) {
                errors.add(ValidationError.InvalidRefreshRate(monitor.refreshRate))
            }
            
            // Validate position coordinates
            if (monitor.position.x < 0 || monitor.position.y < 0) {
                errors.add(ValidationError.InvalidDisplayResolution("Invalid monitor position"))
            }
        }
        
        // Check for duplicate monitor names
        val duplicateMonitors = display.monitors.groupBy { it.name }
            .filter { it.value.size > 1 }
            .keys
        duplicateMonitors.forEach { name ->
            errors.add(ValidationError.ConflictingMonitors(name))
        }
        
        // Validate compositing settings
        display.compositing?.let { compositing ->
            if (compositing.backend !in setOf("xrender", "opengl", "vulkan")) {
                errors.add(ValidationError.InvalidDisplayResolution("Invalid compositing backend"))
            }
        }
        
        return errors
    }
    
    private fun validatePowerConfig(power: PowerConfig): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        // Validate CPU frequency scaling
        power.cpu?.let { cpu ->
            if (cpu.frequencyGovernor !in setOf("performance", "powersave", "ondemand", "conservative", "schedutil")) {
                errors.add(ValidationError.InvalidPowerProfile("Invalid CPU frequency governor: ${cpu.frequencyGovernor}"))
            }
            
            if (cpu.minFrequency != null && cpu.maxFrequency != null && cpu.minFrequency!! > cpu.maxFrequency!!) {
                errors.add(ValidationError.InvalidPowerProfile("Min CPU frequency cannot be greater than max frequency"))
            }
        }
        
        // Validate suspend settings
        power.suspend?.let { suspend ->
            if (suspend.suspendThenHibernate && suspend.hibernateDelaySec <= 0) {
                errors.add(ValidationError.InvalidPowerProfile("Hibernate delay must be positive"))
            }
        }
        
        return errors
    }
    
    private fun validateAudioConfig(audio: AudioConfig): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        // Validate audio devices
        audio.devices.forEach { device ->
            if (!isValidAudioDevice(device.name)) {
                errors.add(ValidationError.InvalidAudioDevice(device.name))
            }
        }
        
        // Validate PulseAudio configuration
        audio.pulseaudio?.let { pulseaudio ->
            if (pulseaudio.sampleRate !in setOf(22050, 44100, 48000, 96000, 192000)) {
                errors.add(ValidationError.InvalidAudioDevice("Invalid sample rate: ${pulseaudio.sampleRate}"))
            }
        }
        
        // Validate ALSA configuration
        audio.alsa?.let { alsa ->
            if (alsa.cardOrder.any { !isValidAudioDevice(it) }) {
                errors.add(ValidationError.InvalidAudioDevice("Invalid ALSA card in order"))
            }
        }
        
        return errors
    }
    
    private fun validateBluetoothConfig(bluetooth: BluetoothConfig): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        // Validate paired devices
        bluetooth.pairedDevices.forEach { device ->
            if (!isValidBluetoothAddress(device.address)) {
                errors.add(ValidationError.InvalidBluetoothAddress(device.address))
            }
        }
        
        // Validate adapter settings
        bluetooth.adapters.forEach { adapter ->
            if (adapter.discoveryTimeout < 0 || adapter.discoveryTimeout > 300) {
                errors.add(ValidationError.InvalidBluetoothAddress("Invalid discovery timeout: ${adapter.discoveryTimeout}"))
            }
        }
        
        return errors
    }
    
    private fun validateUSBConfig(usb: USBConfig): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        // Validate USB devices
        usb.devices.forEach { device ->
            if (!isValidUSBDevice(device.vendorId, device.productId)) {
                errors.add(ValidationError.InvalidUSBDevice("${device.vendorId}:${device.productId}"))
            }
        }
        
        // Validate mount rules
        usb.mountRules.forEach { rule ->
            if (!isValidDevicePath(rule.devicePath)) {
                errors.add(ValidationError.InvalidDevicePath(rule.devicePath))
            }
            
            if (!isValidMountPoint(rule.mountPoint)) {
                errors.add(ValidationError.InvalidMountPoint(rule.mountPoint))
            }
        }
        
        return errors
    }
    
    private fun validateThermalConfig(thermal: ThermalConfig): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        // Validate thermal zones
        thermal.zones.forEach { zone ->
            if (!isValidThermalZone(zone.name)) {
                errors.add(ValidationError.InvalidThermalZone(zone.name))
            }
            
            if (zone.criticalTemp <= zone.warningTemp) {
                errors.add(ValidationError.InvalidThermalZone("Critical temperature must be higher than warning temperature"))
            }
        }
        
        // Validate cooling devices
        thermal.coolingDevices.forEach { device ->
            if (device.maxState < device.minState) {
                errors.add(ValidationError.InvalidThermalZone("Max cooling state must be >= min state"))
            }
        }
        
        return errors
    }
    
    // Helper validation functions
    private fun isValidGPUOption(option: String): Boolean {
        val validOptions = setOf(
            "AccelMethod", "DRI", "TearFree", "SwapbuffersWait", "VariableRefresh",
            "ZaphodHeads", "Option", "Identifier", "Driver", "BusID"
        )
        return validOptions.contains(option) || option.matches(Regex("^[a-zA-Z][a-zA-Z0-9_]*$"))
    }
    
    private fun isValidDisplayResolution(resolution: String): Boolean {
        return resolution.matches(Regex("^\\d{3,5}x\\d{3,5}$"))
    }
    
    private fun isValidRefreshRate(rate: Double): Boolean {
        return rate > 0 && rate <= 240
    }
    
    private fun isValidAudioDevice(device: String): Boolean {
        return device.matches(Regex("^[a-zA-Z0-9_.-]+$"))
    }
    
    private fun isValidBluetoothAddress(address: String): Boolean {
        return address.matches(Regex("^([0-9A-Fa-f]{2}[:-]){5}([0-9A-Fa-f]{2})$"))
    }
    
    private fun isValidUSBDevice(vendorId: String, productId: String): Boolean {
        return vendorId.matches(Regex("^[0-9a-fA-F]{4}$")) && 
               productId.matches(Regex("^[0-9a-fA-F]{4}$"))
    }
    
    private fun isValidDevicePath(path: String): Boolean {
        return path.startsWith("/dev/") && path.matches(Regex("^/dev/[a-zA-Z0-9_/-]+$"))
    }
    
    private fun isValidMountPoint(path: String): Boolean {
        return path.startsWith("/") && !path.contains("..") && 
               path.matches(Regex("^/[a-zA-Z0-9_./\\s-]*$"))
    }
    
    private fun isValidThermalZone(zone: String): Boolean {
        return zone.matches(Regex("^[a-zA-Z0-9_-]+$"))
    }
}