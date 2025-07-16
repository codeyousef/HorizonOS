package org.horizonos.config.validation.validators

import org.horizonos.config.dsl.*
import org.horizonos.config.validation.ValidationError

object BootValidator {
    
    fun validateBootConfig(boot: BootConfig): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        // Validate bootloader configuration
        errors.addAll(validateBootloaderConfig(boot.bootloader))
        
        // Validate kernel configuration
        errors.addAll(validateKernelConfig(boot.kernel))
        
        // Validate initramfs configuration
        errors.addAll(validateInitramfsConfig(boot.initramfs))
        
        // Validate Plymouth configuration
        boot.plymouth?.let { plymouth ->
            errors.addAll(validatePlymouthConfig(plymouth))
        }
        
        // Validate Secure Boot configuration
        boot.secureBoot?.let { secureBoot ->
            errors.addAll(validateSecureBootConfig(secureBoot))
        }
        
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
    
    // Helper validation functions
    private fun isValidBootEntryPath(path: String): Boolean {
        return path.startsWith("/") && !path.contains("..") && 
               path.matches(Regex("^/[a-zA-Z0-9/_.-]*$"))
    }
    
    private fun isValidKernelParameter(parameter: String): Boolean {
        return parameter.matches(Regex("^[a-zA-Z0-9_.=-]+$"))
    }
    
    private fun isValidModuleName(module: String): Boolean {
        return module.matches(Regex("^[a-zA-Z0-9_-]+$"))
    }
    
    private fun isValidInitramfsHook(hook: String): Boolean {
        val validHooks = setOf(
            "base", "udev", "autodetect", "modconf", "block", "filesystems", 
            "keyboard", "fsck", "encrypt", "lvm2", "resume", "systemd"
        )
        return validHooks.contains(hook) || hook.matches(Regex("^[a-zA-Z0-9_-]+$"))
    }
    
    private fun isValidPlymouthTheme(theme: String): Boolean {
        val validThemes = setOf(
            "bgrt", "details", "fade-in", "glow", "script", "solar", 
            "spinner", "spinfinity", "text", "tribar"
        )
        return validThemes.contains(theme) || theme.matches(Regex("^[a-zA-Z0-9_-]+$"))
    }
    
    private fun isValidSecureBootKeyPath(path: String): Boolean {
        return path.startsWith("/") && (path.endsWith(".key") || path.endsWith(".crt") || 
               path.endsWith(".pem")) && !path.contains("..")
    }
}