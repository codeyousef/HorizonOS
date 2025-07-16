package org.horizonos.config.validation.validators

import org.horizonos.config.dsl.*
import org.horizonos.config.dsl.storage.filesystem.*
import org.horizonos.config.dsl.storage.raid.*
import org.horizonos.config.dsl.storage.encryption.*
import org.horizonos.config.dsl.storage.btrfs.*
import org.horizonos.config.dsl.storage.swap.*
import org.horizonos.config.validation.ValidationError

object StorageValidator {
    
    fun validateStorageConfig(storage: StorageConfig): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        // Validate filesystem configurations
        storage.filesystems.forEach { fs ->
            errors.addAll(validateFilesystemConfig(fs))
        }
        
        // Validate RAID configurations
        storage.raid.arrays.forEach { array ->
            errors.addAll(validateRAIDArray(array))
        }
        
        // Validate Btrfs configurations
        if (storage.btrfs.enabled) {
            storage.btrfs.subvolumes.forEach { subvolume ->
                errors.addAll(validateBtrfsSubvolume(subvolume))
            }
        }
        
        // Validate encryption configurations
        errors.addAll(validateEncryptionConfig(storage.encryption))
        
        // Validate swap configurations
        errors.addAll(validateSwapConfig(storage.swap))
        
        // Check for conflicting mount points
        val mountPoints = storage.filesystems.map { it.mountPoint }
        val duplicateMountPoints = mountPoints.groupBy { it }
            .filter { it.value.size > 1 }
            .keys
        duplicateMountPoints.forEach { mountPoint ->
            errors.add(ValidationError.ConflictingMountPoints(mountPoint))
        }
        
        return errors
    }
    
    private fun validateFilesystemConfig(filesystem: FilesystemConfig): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        // Validate mount point
        if (!isValidMountPoint(filesystem.mountPoint)) {
            errors.add(ValidationError.InvalidMountPoint(filesystem.mountPoint))
        }
        
        // Validate device path
        if (!isValidDevicePath(filesystem.device)) {
            errors.add(ValidationError.InvalidDevicePath(filesystem.device))
        }
        
        // Filesystem type is an enum, so it's always valid
        
        // Validate Btrfs-specific settings
        if (filesystem.type == FilesystemType.BTRFS) {
            // Btrfs-specific validation if needed
        }
        
        return errors
    }
    
    private fun validateRAIDArray(array: RAIDArray): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        // RAID level is an enum, so it's always valid
        
        // Validate devices
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
        
        // Check minimum device count for RAID level
        val minDevices = when (array.level) {
            RAIDLevel.RAID0 -> 2
            RAIDLevel.RAID1 -> 2
            RAIDLevel.RAID4 -> 3
            RAIDLevel.RAID5 -> 3
            RAIDLevel.RAID6 -> 4
            RAIDLevel.RAID10 -> 4
            RAIDLevel.LINEAR -> 1
            RAIDLevel.MULTIPATH -> 2
            RAIDLevel.CONTAINER -> 1
        }
        
        if (array.devices.size < minDevices) {
            errors.add(ValidationError.InvalidRAIDLevel("RAID ${array.level.name} requires at least $minDevices devices"))
        }
        
        return errors
    }
    
    private fun validateBtrfsSubvolume(subvolume: org.horizonos.config.dsl.storage.btrfs.BtrfsSubvolume): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        // Validate subvolume path
        if (!isValidMountPoint(subvolume.path)) {
            errors.add(ValidationError.InvalidMountPoint(subvolume.path))
        }
        
        // Validate subvolume name
        if (subvolume.name.isEmpty()) {
            errors.add(ValidationError.InvalidSubvolumeName("Subvolume name cannot be empty"))
        }
        
        return errors
    }
    
    private fun validateEncryptionConfig(encryption: org.horizonos.config.dsl.storage.encryption.EncryptionConfig): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        if (encryption.enabled) {
            // Validate encrypted devices
            encryption.devices.forEach { device ->
                if (!isValidDevicePath(device.device)) {
                    errors.add(ValidationError.InvalidDevicePath(device.device))
                }
                
                if (!isValidKeySize(device.keySize)) {
                    errors.add(ValidationError.InvalidKeySize(device.keySize))
                }
            }
            
            // Validate key files
            encryption.keyManagement.keyFiles.forEach { keyFile ->
                if (!isValidPath(keyFile.path)) {
                    errors.add(ValidationError.InvalidPath(keyFile.path))
                }
            }
        }
        
        return errors
    }
    
    private fun validateSwapConfig(swap: org.horizonos.config.dsl.storage.swap.SwapConfig): List<ValidationError> {
        val errors = mutableListOf<ValidationError>()
        
        if (swap.enabled) {
            // Validate swap devices
            swap.devices.forEach { device ->
                if (!isValidDevicePath(device.device)) {
                    errors.add(ValidationError.InvalidDevicePath(device.device))
                }
            }
            
            // Validate swap files
            swap.files.forEach { file ->
                if (!isValidPath(file.path)) {
                    errors.add(ValidationError.InvalidPath(file.path))
                }
                
                if (!isValidSwapSize(file.size)) {
                    errors.add(ValidationError.InvalidSwapSize(file.size))
                }
            }
        }
        
        // Validate zswap configuration
        if (swap.zswap.enabled) {
            // ZSwap compressor is an enum, so it's always valid
            
            // ZSwap zpool is an enum, so it's always valid
            // No validation needed for zpool since it's constrained by the enum
            
            if (swap.zswap.maxPoolPercent < 1 || swap.zswap.maxPoolPercent > 50) {
                errors.add(ValidationError.InvalidSwapSize("Invalid max pool percentage: ${swap.zswap.maxPoolPercent}"))
            }
        }
        
        return errors
    }
    
    // Helper validation functions
    private fun isValidDevicePath(path: String): Boolean {
        return path.matches(Regex("^/dev/[a-zA-Z0-9/_-]+$"))
    }
    
    private fun isValidMountPoint(path: String): Boolean {
        return path.matches(Regex("^/[a-zA-Z0-9._/-]*$"))
    }
    
    private fun isValidCompressionAlgorithm(algorithm: String): Boolean {
        val validAlgorithms = setOf("zlib", "lzo", "zstd", "none", "ZSTD", "LZ4", "GZIP", "ZLIB", "XZ", "LZO", "NONE")
        return validAlgorithms.contains(algorithm)
    }
    
    private fun isValidRAIDLevel(level: String): Boolean {
        val validLevels = setOf("0", "1", "4", "5", "6", "10", "linear")
        return validLevels.contains(level)
    }
    
    private fun isValidEncryptionCipher(cipher: String): Boolean {
        val validCiphers = setOf(
            "aes-xts-plain64", "aes-cbc-essiv:sha256", "aes-lrw-benbi", 
            "aes-cbc-plain", "aes-cbc-plain64", "serpent-xts-plain64",
            "AES_XTS_PLAIN64", "AES_CBC_ESSIV", "SERPENT_XTS_PLAIN64", "TWOFISH_XTS_PLAIN64"
        )
        return validCiphers.contains(cipher)
    }
    
    private fun isValidKeySize(size: Int): Boolean {
        val validSizes = setOf(128, 192, 256, 512)
        return validSizes.contains(size)
    }
    
    private fun isValidSwapSize(size: String): Boolean {
        return size.matches(Regex("^\\d+[KMGT]?B?$", RegexOption.IGNORE_CASE))
    }
    
    private fun isValidSizeFormat(size: String): Boolean {
        return size.matches(Regex("^\\d+([KMGT]?B?|%)$", RegexOption.IGNORE_CASE))
    }
    
    private fun isValidPath(path: String): Boolean {
        return path.matches(Regex("^/[a-zA-Z0-9._/-]+$"))
    }
    
    private fun isValidBtrfsProfile(profile: String): Boolean {
        val validProfiles = setOf("single", "dup", "raid0", "raid1", "raid10", "raid5", "raid6")
        return validProfiles.contains(profile)
    }
}