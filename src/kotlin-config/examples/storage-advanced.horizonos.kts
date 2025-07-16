import org.horizonos.config.dsl.*

horizonOS {
    // System configuration
    hostname = "horizonos-storage"
    timezone = "America/New_York"
    locale = "en_US.UTF-8"
    
    // Advanced storage configuration
    storage {
        // Filesystem configuration
        filesystem("/dev/nvme0n1p1", "/", FilesystemType.BTRFS) {
            enabled = true
            bootMount = true
            label = "root"
            options {
                standard("compress=zstd:3", "noatime")
                security {
                    relatime = true
                    nodev = false
                    noexec = false
                    nosuid = false
                }
                performance {
                    noatime = true
                    commit = 30
                    barrier = true
                }
            }
        }
        
        filesystem("/dev/nvme0n1p2", "/home", FilesystemType.EXT4) {
            enabled = true
            bootMount = true
            label = "home"
            backupFrequency = 1
            fsckOrder = 2
            options {
                standard("defaults", "user_xattr")
                security {
                    relatime = true
                    nodev = true
                }
                performance {
                    barrier = true
                    dataMode = DataMode.ORDERED
                    journalMode = JournalMode.ORDERED
                }
            }
        }
        
        filesystem("/dev/sda1", "/var/log", FilesystemType.XFS) {
            enabled = true
            bootMount = true
            label = "logs"
            options {
                standard("defaults", "logbsize=256k")
                security {
                    noexec = true
                    nosuid = true
                    nodev = true
                }
            }
        }
        
        // RAID configuration
        raid {
            enabled = true
            
            // RAID monitoring
            monitoring {
                enabled = true
                emailNotifications = true
                emailAddress = "admin@horizonos.local"
                checkInterval = 6.hours
                scanSpeed = ScanSpeed.NORMAL
            }
            
            // RAID notifications
            notifications {
                degraded = true
                failed = true
                spareActive = true
                rebuildStarted = true
                rebuildFinished = true
                testFinished = true
            }
        }
        
        // LUKS encryption configuration
        encryption {
            enabled = true
            
            // TPM configuration
            tpm {
                enabled = true
                version = TPMVersion.TPM2
                pcrBanks = listOf(0, 1, 2, 3, 4, 5, 6, 7)
                sealingPolicy = SealingPolicy.SECURE_BOOT
            }
            
            // Key management
            keyManagement {
                autoBackup = true
                backupLocation = "/etc/luks-keys"
                backupEncryption = true
                keyRotation {
                    enabled = true
                    interval = 90.days
                    keepOldKeys = 5
                    automaticRotation = false
                }
            }
        }
        
        // Btrfs configuration
        btrfs {
            enabled = true
            
            // Compression settings
            compression {
                enabled = true
                algorithm = CompressionAlgorithm.ZSTD
                level = 3
                autoCompress = true
                compressibleTypes = listOf("text", "application", "image")
            }
            
            // Snapshot configuration
            snapshots {
                enabled = true
                location = "/.snapshots"
                compression = true
                verification = true
                
                retention {
                    hourly = 24
                    daily = 7
                    weekly = 4
                    monthly = 12
                    yearly = 3
                }
                
                schedule {
                    hourly = true
                    daily = true
                    weekly = true
                    monthly = true
                    yearly = true
                }
            }
            
            // Scrubbing configuration
            scrubbing {
                enabled = true
                schedule = "0 2 * * 0" // Weekly at 2 AM
                priority = ScrubbingPriority.NORMAL
                readOnly = false
                forceCheck = false
            }
            
            // Balancing configuration
            balancing {
                enabled = true
                schedule = "0 3 1 * *" // Monthly at 3 AM
                dataThreshold = 85
                metadataThreshold = 90
                autoBalance = true
            }
        }
        
        // Swap configuration
        swap {
            enabled = true
            type = SwapType.ZRAM
            priority = 10
            swappiness = 10
            vfsCache = 50
            
            // ZRAM configuration
            zram {
                enabled = true
                size = "50%"
                algorithm = CompressionAlgorithm.LZ4
                streams = 0 // Auto-detect
                priority = 100
                disksize = "auto"
            }
            
            // Additional swap file
            files = listOf(
                SwapFile(
                    path = "/swapfile",
                    size = "2G",
                    priority = 5,
                    preallocate = true,
                    permissions = "600"
                )
            )
        }
        
        // Auto-mount configuration
        autoMount {
            enabled = true
            
            // Removable media
            removableMedia {
                enabled = true
                mountPoint = "/media"
                fileManager = true
                desktop = true
                userMount = true
                umask = "022"
                options = listOf("utf8", "shortname=mixed")
            }
            
            // Network shares
            networkShares {
                enabled = true
                
                // Samba configuration
                samba {
                    enabled = true
                    workgroup = "WORKGROUP"
                    version = "3.0"
                    encryption = true
                }
                
                // NFS configuration
                nfs {
                    enabled = true
                    version = "4.0"
                    timeout = 30.minutes
                    retrans = 3
                    rsize = 32768
                    wsize = 32768
                }
                
                // SSH configuration
                ssh {
                    enabled = true
                    compression = true
                    port = 22
                    followSymlinks = true
                }
            }
            
            // Encrypted volume auto-mount
            encryptedVolumes {
                enabled = true
                keyring = true
                passwordPrompt = true
                timeout = 5.minutes
            }
        }
        
        // Storage maintenance
        maintenance {
            enabled = true
            
            // Filesystem check
            fsck {
                enabled = true
                schedule = "0 4 * * 0" // Weekly at 4 AM
                forceCheck = false
                autoFix = false
                skipRoot = false
            }
            
            // Defragmentation
            defragmentation {
                enabled = true
                schedule = "0 5 1 * *" // Monthly at 5 AM
                filesystems = listOf("ext4", "btrfs")
                threshold = 10
                maxFiles = 1000
            }
            
            // SSD TRIM
            trim {
                enabled = true
                schedule = "0 6 * * 0" // Weekly at 6 AM
                continuous = false
                filesystems = listOf("ext4", "xfs", "btrfs")
            }
            
            // Health checks
            healthChecks {
                enabled = true
                schedule = "0 7 * * *" // Daily at 7 AM
                
                // SMART monitoring
                smart {
                    enabled = true
                    testSchedule = "0 8 * * 0" // Weekly at 8 AM
                    testType = SmartTestType.LONG
                    
                    temperature {
                        warning = 60
                        critical = 70
                        monitoring = true
                    }
                    
                    attributes = listOf(
                        SmartAttribute(5, "Reallocated_Sector_Ct", 0, true),
                        SmartAttribute(187, "Reported_Uncorrect", 0, true),
                        SmartAttribute(188, "Command_Timeout", 0, true),
                        SmartAttribute(197, "Current_Pending_Sector", 0, true),
                        SmartAttribute(198, "Offline_Uncorrectable", 0, true)
                    )
                }
                
                // Bad blocks check
                badBlocks {
                    enabled = false
                    schedule = "0 9 1 * *" // Monthly at 9 AM
                    destructive = false
                    pattern = BadBlockPattern.RANDOM
                }
            }
        }
        
        // Storage monitoring
        monitoring {
            enabled = true
            
            // Disk usage monitoring
            diskUsage {
                enabled = true
                interval = 5.minutes
                warningThreshold = 80
                criticalThreshold = 90
                ignoreFilesystems = listOf("tmpfs", "devtmpfs", "proc", "sys")
            }
            
            // Performance monitoring
            performance {
                enabled = true
                interval = 30.minutes
                ioStats = true
                latencyTracking = true
                queueDepth = true
            }
            
            // Notifications
            notifications {
                email = true
                desktop = true
                syslog = true
                webhook = "https://hooks.slack.com/services/YOUR/WEBHOOK/URL"
            }
        }
    }
    
    // Users
    users {
        user("admin") {
            shell = "/usr/bin/zsh"
            groups("wheel", "storage", "disk")
        }
        
        user("storage") {
            shell = "/usr/bin/bash"
            groups("storage")
        }
    }
    
    // Packages for storage management
    packages {
        install(
            // Filesystem tools
            "btrfs-progs",
            "e2fsprogs",
            "xfsprogs",
            "dosfstools",
            "ntfs-3g",
            "exfat-utils",
            
            // RAID tools
            "mdadm",
            "smartmontools",
            
            // Encryption tools
            "cryptsetup",
            "tpm2-tools",
            "clevis",
            
            // Monitoring tools
            "iotop",
            "iostat",
            "ncdu",
            "duf",
            
            // Network filesystem support
            "nfs-utils",
            "cifs-utils",
            "sshfs",
            
            // Auto-mount support
            "udisks2",
            "udiskie",
            "autofs"
        )
    }
    
    // Services for storage
    services {
        enable("mdmonitor") {
            autoRestart = true
            restartOnFailure = true
        }
        
        enable("smartd") {
            autoRestart = true
            restartOnFailure = true
        }
        
        enable("fstrim.timer") {
            autoRestart = true
            restartOnFailure = true
        }
        
        enable("udisks2") {
            autoRestart = true
            restartOnFailure = true
        }
        
        enable("autofs") {
            autoRestart = true
            restartOnFailure = true
        }
    }
}