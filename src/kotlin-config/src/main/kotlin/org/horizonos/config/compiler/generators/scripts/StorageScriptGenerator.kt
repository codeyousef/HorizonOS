package org.horizonos.config.compiler.generators.scripts

import org.horizonos.config.dsl.*
import org.horizonos.config.dsl.storage.filesystem.*
import org.horizonos.config.dsl.storage.filesystem.FilesystemType
import org.horizonos.config.dsl.storage.raid.*
import org.horizonos.config.dsl.storage.encryption.*
import org.horizonos.config.dsl.storage.btrfs.*
import org.horizonos.config.dsl.storage.swap.*
import org.horizonos.config.dsl.storage.maintenance.*
import org.horizonos.config.compiler.GeneratedFile
import org.horizonos.config.compiler.FileType
import java.io.File

/**
 * Storage Script Generator
 * Generates shell scripts for storage configuration including filesystems, RAID, encryption, and Btrfs
 */
class StorageScriptGenerator(
    private val outputDir: File,
    private val generatedFiles: MutableList<GeneratedFile>
) {
    
    fun generateStorageScript(config: CompiledConfig) {
        config.storage?.let { storage ->
            val script = File(outputDir, "scripts/storage-config.sh")
            script.writeText(buildString {
                appendLine("#!/bin/bash")
                appendLine("# Storage Configuration")
                appendLine("# Generated from HorizonOS Kotlin DSL")
                appendLine()
                appendLine("set -euo pipefail")
                appendLine()
                appendLine("echo 'Configuring storage...'")
                appendLine()
                
                // Filesystem configuration
                generateFilesystemConfig(storage)
                
                // RAID configuration
                generateRAIDConfig(storage)
                
                // LUKS encryption configuration
                generateEncryptionConfig(storage)
                
                // Btrfs configuration
                generateBtrfsConfig(storage)
                
                // Swap configuration
                generateSwapConfig(storage)
                
                // Storage maintenance
                generateMaintenanceConfig(storage)
                
                // Auto-mount configuration
                generateAutoMountConfig(storage)
                
                appendLine("echo 'Storage configuration completed.'")
            })
            script.setExecutable(true)
            generatedFiles.add(GeneratedFile("scripts/storage-config.sh", FileType.SHELL))
        }
    }
    
    private fun StringBuilder.generateFilesystemConfig(storage: StorageConfig) {
        if (storage.filesystems.isNotEmpty()) {
            appendLine("# Filesystem Configuration")
            storage.filesystems.forEach { fs ->
                if (fs.enabled) {
                    appendLine("# Configure filesystem: ${fs.mountPoint}")
                    appendLine("mkdir -p ${fs.mountPoint}")
                    
                    // Build mount options
                    val mountOptions = mutableListOf<String>()
                    mountOptions.addAll(fs.options.standard)
                    
                    // Add security options
                    if (fs.options.security.noexec) mountOptions.add("noexec")
                    if (fs.options.security.nosuid) mountOptions.add("nosuid")
                    if (fs.options.security.nodev) mountOptions.add("nodev")
                    if (fs.options.security.relatime) mountOptions.add("relatime")
                    if (fs.options.security.ro) mountOptions.add("ro")
                    
                    // Add performance options
                    if (fs.options.performance.noatime) mountOptions.add("noatime")
                    if (fs.options.performance.nodiratime) mountOptions.add("nodiratime")
                    fs.options.performance.commit?.let { mountOptions.add("commit=$it") }
                    
                    val optionsStr = if (mountOptions.isNotEmpty()) "-o ${mountOptions.joinToString(",")}" else ""
                    
                    if (fs.bootMount) {
                        appendLine("# Add to fstab for boot mount")
                        val fstabEntry = "${fs.device} ${fs.mountPoint} ${fs.type.name.lowercase()} ${mountOptions.joinToString(",").ifEmpty { "defaults" }} ${fs.backupFrequency} ${fs.fsckOrder}"
                        appendLine("echo '$fstabEntry' >> /etc/fstab")
                    } else {
                        appendLine("mount -t ${fs.type.name.lowercase()} $optionsStr ${fs.device} ${fs.mountPoint}")
                    }
                    
                    fs.label?.let { label ->
                        appendLine("# Set filesystem label")
                        when (fs.type) {
                            FilesystemType.EXT4, FilesystemType.EXT3, FilesystemType.EXT2 -> {
                                appendLine("e2label ${fs.device} $label")
                            }
                            FilesystemType.XFS -> {
                                appendLine("xfs_admin -L $label ${fs.device}")
                            }
                            FilesystemType.BTRFS -> {
                                appendLine("btrfs filesystem label ${fs.device} $label")
                            }
                            else -> {
                                appendLine("# Label setting not supported for ${fs.type}")
                            }
                        }
                    }
                    appendLine()
                }
            }
        }
    }
    
    private fun StringBuilder.generateRAIDConfig(storage: StorageConfig) {
        if (storage.raid.enabled && storage.raid.arrays.isNotEmpty()) {
            appendLine("# RAID Configuration")
            storage.raid.arrays.forEach { raid ->
                appendLine("# Create RAID array: ${raid.name}")
                val raidCmd = buildString {
                    append("mdadm --create /dev/md/${raid.name}")
                    append(" --level=${raid.level.name.replace("RAID", "")}")
                    append(" --raid-devices=${raid.devices.size}")
                    if (raid.spares.isNotEmpty()) {
                        append(" --spare-devices=${raid.spares.size}")
                    }
                    raid.chunkSize?.let { append(" --chunk=$it") }
                    append(" --metadata=${raid.metadata.name.replace("_", ".")}")
                    append(" ${raid.devices.joinToString(" ")}")
                    if (raid.spares.isNotEmpty()) {
                        append(" ${raid.spares.joinToString(" ")}")
                    }
                }
                appendLine(raidCmd)
                
                // Configure RAID bitmap
                raid.bitmap?.let { bitmap ->
                    if (bitmap.enabled) {
                        appendLine("mdadm --grow --bitmap=${bitmap.location} /dev/md/${raid.name}")
                    }
                }
                appendLine()
            }
            
            // RAID monitoring
            if (storage.raid.monitoring.enabled) {
                appendLine("# Configure RAID monitoring")
                appendLine("systemctl enable mdmonitor.service")
                appendLine("systemctl start mdmonitor.service")
                
                storage.raid.monitoring.emailAlerts.recipients.forEach { email ->
                    appendLine("# Configure email notifications")
                    appendLine("echo 'MAILADDR $email' >> /etc/mdadm.conf")
                }
                appendLine()
            }
        }
    }
    
    private fun StringBuilder.generateEncryptionConfig(storage: StorageConfig) {
        if (storage.encryption.enabled && storage.encryption.devices.isNotEmpty()) {
            appendLine("# LUKS Encryption Configuration")
            storage.encryption.devices.forEach { device ->
                appendLine("# Setup encrypted device: ${device.name}")
                
                val luksCmd = buildString {
                    append("cryptsetup luksFormat")
                    append(" --type luks${device.header.version.name.removePrefix("LUKS")}")
                    append(" --cipher ${device.cipher.name.replace("_", "-").lowercase()}")
                    append(" --key-size ${device.keySize}")
                    append(" --hash ${device.hashAlgorithm.name.lowercase()}")
                    append(" --pbkdf ${device.header.pbkdf.name.lowercase()}")
                    append(" --iter-time ${device.iterTime}")
                    device.header.memory?.let { append(" --pbkdf-memory $it") }
                    device.header.parallelism?.let { append(" --pbkdf-parallel $it") }
                    append(" ${device.device}")
                }
                appendLine(luksCmd)
                
                // Open encrypted device
                appendLine("cryptsetup luksOpen ${device.device} ${device.name}")
                
                // Add to crypttab if needed
                appendLine("echo '${device.name} ${device.device} none luks' >> /etc/crypttab")
                appendLine()
            }
            
            // TPM configuration
            if (storage.encryption.keyManagement.tpmIntegration.enabled) {
                appendLine("# Configure TPM-based encryption")
                appendLine("# TPM ${storage.encryption.keyManagement.tpmIntegration.tpmVersion} configuration")
                storage.encryption.keyManagement.tpmIntegration.nvramIndex?.let { index ->
                    appendLine("# TPM NVRAM index: $index")
                }
                storage.encryption.keyManagement.tpmIntegration.pcrs.forEach { pcr ->
                    appendLine("# Using PCR: $pcr")
                }
                appendLine()
            }
        }
    }
    
    private fun StringBuilder.generateBtrfsConfig(storage: StorageConfig) {
        if (storage.btrfs.enabled && storage.btrfs.subvolumes.isNotEmpty()) {
            appendLine("# Btrfs Configuration")
            
            // Create subvolumes
            storage.btrfs.subvolumes.forEach { subvolume ->
                appendLine("# Create Btrfs subvolume: ${subvolume.name}")
                appendLine("btrfs subvolume create ${subvolume.path}")
                
                // Set default subvolume if needed
                if (subvolume.defaultSubvolume) {
                    appendLine("# Set as default subvolume")
                    appendLine("btrfs subvolume set-default ${subvolume.path}")
                }
                
                // Set compression if specified
                subvolume.compression?.let { compression ->
                    appendLine("# Set compression: ${compression.name.lowercase()}")
                    appendLine("btrfs property set ${subvolume.path} compression ${compression.name.lowercase()}")
                }
                
                // Disable copy-on-write if needed
                if (!subvolume.copyOnWrite) {
                    appendLine("# Disable copy-on-write")
                    appendLine("chattr +C ${subvolume.path}")
                }
                
                // Set quota if specified
                subvolume.quota?.let { quota ->
                    appendLine("# Set quota")
                    appendLine("btrfs quota enable ${subvolume.path}")
                    quota.sizeLimit?.let { limit ->
                        appendLine("btrfs qgroup limit $limit ${subvolume.path}")
                    }
                }
                appendLine()
            }
            
            // Btrfs scrub configuration
            if (storage.btrfs.scrub.enabled) {
                appendLine("# Configure Btrfs scrubbing")
                appendLine("# Schedule: ${storage.btrfs.scrub.schedule}")
                appendLine("systemctl enable btrfs-scrub@-.timer")
                appendLine("systemctl start btrfs-scrub@-.timer")
                appendLine()
            }
            
            // Btrfs balance configuration
            if (storage.btrfs.balance.enabled) {
                appendLine("# Configure Btrfs balance")
                appendLine("# Schedule: ${storage.btrfs.balance.schedule}")
                appendLine()
            }
        }
    }
    
    private fun StringBuilder.generateSwapConfig(storage: StorageConfig) {
        if (storage.swap.enabled) {
            appendLine("# Swap Configuration")
            
            // Configure swap devices
            storage.swap.devices.forEach { device ->
                appendLine("# Enable swap device: ${device.device}")
                appendLine("mkswap ${device.device}")
                val priority = if (device.priority >= 0) "-p ${device.priority}" else ""
                appendLine("swapon ${device.device} $priority")
                appendLine("echo '${device.device} none swap sw,pri=${device.priority} 0 0' >> /etc/fstab")
                appendLine()
            }
            
            // Configure swap files
            storage.swap.files.forEach { swapFile ->
                appendLine("# Create swap file: ${swapFile.path}")
                when (swapFile.allocateMode) {
                    AllocateMode.FALLOCATE -> appendLine("fallocate -l ${swapFile.size} ${swapFile.path}")
                    AllocateMode.DD -> appendLine("dd if=/dev/zero of=${swapFile.path} bs=1M count=\$(numfmt --from=iec ${swapFile.size} | awk '{print \$1/1048576}') status=progress")
                    AllocateMode.TRUNCATE -> appendLine("truncate -s ${swapFile.size} ${swapFile.path}")
                }
                appendLine("chmod ${swapFile.permissions} ${swapFile.path}")
                appendLine("mkswap ${swapFile.path}")
                val priority = if (swapFile.priority >= 0) "-p ${swapFile.priority}" else ""
                appendLine("swapon ${swapFile.path} $priority")
                appendLine("echo '${swapFile.path} none swap sw,pri=${swapFile.priority} 0 0' >> /etc/fstab")
                appendLine()
            }
            
            // Configure ZRAM
            if (storage.swap.zram.enabled) {
                appendLine("# Configure ZRAM swap")
                storage.swap.zram.devices.forEach { zramDevice ->
                    appendLine("# Configure ${zramDevice.name}")
                    appendLine("modprobe zram")
                    appendLine("echo '${storage.swap.zram.algorithm.name.lowercase()}' > /sys/block/${zramDevice.name}/comp_algorithm")
                    appendLine("echo '${zramDevice.size}' > /sys/block/${zramDevice.name}/disksize")
                    appendLine("mkswap /dev/${zramDevice.name}")
                    appendLine("swapon /dev/${zramDevice.name} -p ${zramDevice.priority}")
                    appendLine()
                }
            }
            
            // Configure swappiness and other vm settings
            appendLine("# Configure VM settings")
            appendLine("echo 'vm.swappiness=${storage.swap.swappiness}' >> /etc/sysctl.conf")
            appendLine("echo 'vm.vfs_cache_pressure=${storage.swap.vfsCachePressure}' >> /etc/sysctl.conf")
            storage.swap.minFreeKbytes?.let {
                appendLine("echo 'vm.min_free_kbytes=$it' >> /etc/sysctl.conf")
            }
            appendLine("echo 'vm.watermark_scale_factor=${storage.swap.watermarkScaleFactor}' >> /etc/sysctl.conf")
            appendLine("sysctl -p")
            appendLine()
        }
    }
    
    private fun StringBuilder.generateMaintenanceConfig(storage: StorageConfig) {
        if (storage.maintenance.enabled) {
            appendLine("# Storage Maintenance Configuration")
            
            if (storage.maintenance.trim.enabled) {
                appendLine("# Configure SSD TRIM")
                appendLine("systemctl enable fstrim.timer")
                appendLine("systemctl start fstrim.timer")
                appendLine("# Schedule: ${storage.maintenance.trim.schedule}")
            }
            
            if (storage.maintenance.verification.filesystem.enabled) {
                appendLine("# Configure filesystem verification")
                appendLine("# Schedule: ${storage.maintenance.verification.filesystem.schedule}")
            }
            
            // Configure optimization tasks
            if (storage.maintenance.optimization.enabled) {
                appendLine("# Configure storage optimization")
                if (storage.maintenance.optimization.database.enabled) {
                    appendLine("# Database optimization enabled")
                }
                if (storage.maintenance.optimization.index.enabled) {
                    appendLine("# Index optimization enabled")
                    appendLine("systemctl enable updatedb.timer")
                }
            }
            appendLine()
        }
    }
    
    private fun StringBuilder.generateAutoMountConfig(storage: StorageConfig) {
        if (storage.autoMount.enabled) {
            appendLine("# Auto-mount Configuration")
            appendLine("# Timeout: ${storage.autoMount.timeout} seconds")
            
            if (storage.autoMount.showInFileManager) {
                appendLine("# Show auto-mounted devices in file manager")
            }
            
            if (storage.autoMount.allowPolkitActions) {
                appendLine("# Allow polkit actions for mounting")
                appendLine("systemctl enable udisks2.service")
                appendLine("systemctl start udisks2.service")
            }
            appendLine()
        }
    }
}