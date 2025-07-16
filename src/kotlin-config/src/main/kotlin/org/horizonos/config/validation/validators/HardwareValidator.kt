package org.horizonos.config.validation.validators

import org.horizonos.config.dsl.*
import org.horizonos.config.dsl.hardware.*
import org.horizonos.config.dsl.hardware.CPUGovernor
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
        gpu.opengl.let { opengl ->
            if (opengl.multisampling < 0 || opengl.multisampling > 32) {
                errors.add(ValidationError.InvalidGPUDriver("Invalid multisampling value: ${opengl.multisampling}"))
            }
            if (opengl.anisotropicFiltering < 0 || opengl.anisotropicFiltering > 16) {
                errors.add(ValidationError.InvalidGPUDriver("Invalid anisotropic filtering value: ${opengl.anisotropicFiltering}"))
            }
        }
        
        // Validate Vulkan configuration
        gpu.vulkan.let { vulkan ->
            if (vulkan.enabled && vulkan.validation && vulkan.layers.isEmpty()) {
                errors.add(ValidationError.InvalidGPUDriver("Vulkan validation enabled but no validation layers specified"))
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
                if (resolution.width < 640 || resolution.height < 480) {
                    errors.add(ValidationError.InvalidDisplayResolution("Resolution too small: ${resolution.width}x${resolution.height}"))
                }
                if (resolution.width > 8192 || resolution.height > 8192) {
                    errors.add(ValidationError.InvalidDisplayResolution("Resolution too large: ${resolution.width}x${resolution.height}"))
                }
            }
            
            // Validate refresh rate
            monitor.refreshRate?.let { rate ->
                if (!isValidRefreshRate(rate)) {
                    errors.add(ValidationError.InvalidRefreshRate(rate))
                }
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
        
        // Compositing backend validation is already handled by the enum type
        
        return errors
    }
    
    private fun validatePowerConfig(power: PowerConfig): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        // Validate CPU frequency scaling
        power.cpu.let { cpu ->
            // CPU governor validation is already handled by the enum type
            
            // Validate frequency strings if provided
            val minFreq = cpu.minFreq
            val maxFreq = cpu.maxFreq
            if (minFreq != null && maxFreq != null) {
                try {
                    val min = minFreq.removeSuffix("MHz").toDouble()
                    val max = maxFreq.removeSuffix("MHz").toDouble()
                    if (min > max) {
                        errors.add(ValidationError.InvalidPowerProfile("Min CPU frequency cannot be greater than max frequency"))
                    }
                } catch (e: NumberFormatException) {
                    errors.add(ValidationError.InvalidPowerProfile("Invalid frequency format"))
                }
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
            adapter.address?.let { address ->
                if (!isValidBluetoothAddress(address)) {
                    errors.add(ValidationError.InvalidBluetoothAddress("Invalid adapter address: $address"))
                }
            }
            // timeout is a Duration, so no numeric validation needed
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
            // Validate vendor/product IDs if provided
            rule.vendorId?.let { vendorId ->
                rule.productId?.let { productId ->
                    if (!isValidUSBDevice(vendorId, productId)) {
                        errors.add(ValidationError.InvalidUSBDevice("$vendorId:$productId"))
                    }
                }
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