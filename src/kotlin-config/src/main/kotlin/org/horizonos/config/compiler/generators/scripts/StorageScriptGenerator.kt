package org.horizonos.config.compiler.generators.scripts

import org.horizonos.config.dsl.*
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
                
                storage.raid.monitoring.emailAddress?.let { email ->
                    appendLine("# Configure email notifications")
                    appendLine("echo 'MAILADDR $email' >> /etc/mdadm.conf")
                }
                appendLine()
            }
        }
    }
    
    private fun StringBuilder.generateEncryptionConfig(storage: StorageConfig) {
        if (storage.encryption.enabled && storage.encryption.volumes.isNotEmpty()) {
            appendLine("# LUKS Encryption Configuration")
            storage.encryption.volumes.forEach { volume ->
                appendLine("# Setup encrypted volume: ${volume.name}")
                
                val luksCmd = buildString {
                    append("cryptsetup luksFormat")
                    append(" --type luks2")
                    append(" --cipher ${volume.cipher.name.replace("_", "-").lowercase()}")
                    append(" --key-size ${volume.keySize}")
                    append(" --hash ${volume.hashAlgorithm.name.lowercase()}")
                    append(" --pbkdf ${volume.pbkdf.algorithm.name.lowercase()}")
                    volume.pbkdf.iterations?.let { append(" --pbkdf-force-iterations $it") }
                    volume.pbkdf.memory?.let { append(" --pbkdf-memory $it") }
                    volume.pbkdf.parallelism?.let { append(" --pbkdf-parallel $it") }
                    append(" ${volume.device}")
                }
                appendLine(luksCmd)
                
                // Open encrypted volume
                appendLine("cryptsetup luksOpen ${volume.device} ${volume.name}")
                
                // Add to crypttab if needed
                appendLine("echo '${volume.name} ${volume.device} none luks' >> /etc/crypttab")
                appendLine()
            }
            
            // TPM configuration
            if (storage.encryption.tpm.enabled) {
                appendLine("# Configure TPM-based encryption")
                appendLine("# TPM ${storage.encryption.tpm.version} configuration")
                storage.encryption.tpm.keyHandle?.let { handle ->
                    appendLine("# TPM key handle: $handle")
                }
                appendLine()
            }
        }
    }
    
    private fun StringBuilder.generateBtrfsConfig(storage: StorageConfig) {
        if (storage.btrfs.enabled && storage.btrfs.filesystems.isNotEmpty()) {
            appendLine("# Btrfs Configuration")
            storage.btrfs.filesystems.forEach { btrfs ->
                appendLine("# Create Btrfs filesystem: ${btrfs.label}")
                
                val btrfsCmd = buildString {
                    append("mkfs.btrfs")
                    append(" --label ${btrfs.label}")
                    append(" --data ${btrfs.dataProfile.name.lowercase()}")
                    append(" --metadata ${btrfs.metadataProfile.name.lowercase()}")
                    append(" ${btrfs.devices.joinToString(" ")}")
                }
                appendLine(btrfsCmd)
                
                // Create subvolumes
                if (btrfs.subvolumes.isNotEmpty()) {
                    appendLine("# Create subvolumes")
                    val mountPoint = "/mnt/${btrfs.label}"
                    appendLine("mkdir -p $mountPoint")
                    appendLine("mount ${btrfs.devices.first()} $mountPoint")
                    
                    btrfs.subvolumes.forEach { subvol ->
                        appendLine("btrfs subvolume create $mountPoint/${subvol.name}")
                        
                        if (subvol.defaultSubvolume) {
                            appendLine("btrfs subvolume set-default $mountPoint/${subvol.name}")
                        }
                        
                        subvol.quota?.let { quota ->
                            if (quota.enabled) {
                                appendLine("btrfs quota enable $mountPoint")
                                quota.sizeLimit?.let { limit ->
                                    appendLine("btrfs qgroup limit $limit $mountPoint/${subvol.name}")
                                }
                            }
                        }
                    }
                    
                    appendLine("umount $mountPoint")
                }
                appendLine()
            }
            
            // Btrfs maintenance
            if (storage.btrfs.scrubbing.enabled) {
                appendLine("# Configure Btrfs scrubbing")
                appendLine("systemctl enable btrfs-scrub@-.timer")
                appendLine("systemctl start btrfs-scrub@-.timer")
                appendLine()
            }
        }
    }
    
    private fun StringBuilder.generateSwapConfig(storage: StorageConfig) {
        if (storage.swap.enabled) {
            appendLine("# Swap Configuration")
            
            when (storage.swap.type) {
                SwapType.ZRAM -> {
                    appendLine("# Configure ZRAM swap")
                    appendLine("modprobe zram")
                    appendLine("echo '${storage.swap.zram.algorithm.name.lowercase()}' > /sys/block/zram0/comp_algorithm")
                    appendLine("echo '${storage.swap.zram.size}' > /sys/block/zram0/disksize")
                    appendLine("mkswap /dev/zram0")
                    appendLine("swapon /dev/zram0 -p ${storage.swap.zram.priority}")
                }
                SwapType.FILE -> {
                    storage.swap.files.forEach { swapFile ->
                        appendLine("# Create swap file: ${swapFile.path}")
                        appendLine("fallocate -l ${swapFile.size} ${swapFile.path}")
                        appendLine("chmod ${swapFile.permissions} ${swapFile.path}")
                        appendLine("mkswap ${swapFile.path}")
                        appendLine("swapon ${swapFile.path} -p ${swapFile.priority}")
                        appendLine("echo '${swapFile.path} none swap sw,pri=${swapFile.priority} 0 0' >> /etc/fstab")
                    }
                }
                SwapType.PARTITION -> {
                    storage.swap.partitions.forEach { partition ->
                        appendLine("# Enable swap partition: ${partition.device}")
                        appendLine("mkswap ${partition.device}")
                        appendLine("swapon ${partition.device} -p ${partition.priority}")
                        appendLine("echo '${partition.device} none swap sw,pri=${partition.priority} 0 0' >> /etc/fstab")
                    }
                }
                else -> {
                    appendLine("# Swap type ${storage.swap.type} configuration")
                }
            }
            
            // Configure swappiness
            appendLine("echo 'vm.swappiness=${storage.swap.swappiness}' >> /etc/sysctl.conf")
            appendLine("echo 'vm.vfs_cache_pressure=${storage.swap.vfsCache}' >> /etc/sysctl.conf")
            appendLine()
        }
    }
    
    private fun StringBuilder.generateMaintenanceConfig(storage: StorageConfig) {
        if (storage.maintenance.enabled) {
            appendLine("# Storage Maintenance Configuration")
            
            if (storage.maintenance.fsck.enabled) {
                appendLine("# Configure filesystem check")
                appendLine("systemctl enable fsck@.service")
            }
            
            if (storage.maintenance.trim.enabled) {
                appendLine("# Configure SSD TRIM")
                appendLine("systemctl enable fstrim.timer")
                appendLine("systemctl start fstrim.timer")
            }
            
            if (storage.maintenance.healthChecks.enabled) {
                appendLine("# Configure storage health checks")
                if (storage.maintenance.healthChecks.smart.enabled) {
                    appendLine("systemctl enable smartd.service")
                    appendLine("systemctl start smartd.service")
                }
            }
            appendLine()
        }
    }
    
    private fun StringBuilder.generateAutoMountConfig(storage: StorageConfig) {
        if (storage.autoMount.enabled) {
            appendLine("# Auto-mount Configuration")
            
            if (storage.autoMount.removableMedia.enabled) {
                appendLine("# Configure removable media auto-mount")
                appendLine("systemctl enable udisks2.service")
                appendLine("systemctl start udisks2.service")
            }
            
            if (storage.autoMount.networkShares.enabled) {
                appendLine("# Configure network shares auto-mount")
                appendLine("systemctl enable autofs.service")
                appendLine("systemctl start autofs.service")
            }
            appendLine()
        }
    }
}