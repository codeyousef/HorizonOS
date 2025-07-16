import org.horizonos.config.dsl.*

horizonOS {
    // System configuration
    hostname = "boot-showcase"
    timezone = "America/New_York" 
    locale = "en_US.UTF-8"

    // Advanced boot configuration demonstrating all features
    boot {
        // Bootloader configuration - systemd-boot with custom entries
        bootloader {
            type = BootloaderType.SYSTEMD_BOOT
            timeout = 10.seconds
            defaultEntry = "horizonos-main"
            fallbackEntry = "horizonos-fallback"
            editor = false // Disable editor for security
            consoleMode = ConsoleMode.AUTO
            resolution = "1920x1080"
            autoEntries = true
            autoFirmware = true
            
            // Main boot entry
            entry("HorizonOS Main") {
                linux = "/vmlinuz-linux"
                initrd = "/initramfs-linux.img"
                options(
                    "root=UUID=12345678-1234-1234-1234-123456789abc",
                    "rw", "quiet", "splash",
                    "loglevel=3", "rd.udev.log_priority=3"
                )
                version = "6.1.0-horizonos"
                sort = 1
            }
            
            // Fallback boot entry
            entry("HorizonOS Fallback") {
                linux = "/vmlinuz-linux"
                initrd = "/initramfs-linux-fallback.img"
                options(
                    "root=UUID=12345678-1234-1234-1234-123456789abc",
                    "rw"
                )
                version = "6.1.0-horizonos"
                sort = 2
            }
            
            // Recovery boot entry
            entry("HorizonOS Recovery") {
                linux = "/vmlinuz-linux"
                initrd = "/initramfs-linux.img"
                options(
                    "root=UUID=12345678-1234-1234-1234-123456789abc",
                    "rw", "single", "systemd.unit=rescue.target"
                )
                version = "6.1.0-horizonos"
                sort = 10
            }
        }
        
        // Comprehensive kernel configuration
        kernel {
            compression = KernelCompression.ZSTD
            version = "6.1.0-horizonos"
            
            // Essential kernel parameters
            quiet()
            splash()
            rootDevice("UUID=12345678-1234-1234-1234-123456789abc")
            rootfsType("ext4")
            resume("UUID=abcd-efgh-ijkl-mnop-qrstuvwxyz12") // Swap partition for hibernation
            
            // Security parameters
            parameter("lockdown", "confidentiality")
            parameter("init_on_alloc", "1")
            parameter("init_on_free", "1")
            parameter("page_alloc.shuffle", "1")
            parameter("randomize_kstack_offset", "1")
            
            // Performance parameters
            parameter("elevator", "mq-deadline")
            parameter("transparent_hugepage", "madvise")
            parameter("audit", "0") // Disable audit for performance
            
            // Hardware-specific parameters
            parameter("intel_iommu", "on")
            parameter("iommu", "pt")
            parameter("pcie_aspm", "off")
            parameter("acpi_osi", "Linux")
            
            // Graphics parameters (for NVIDIA)
            nvidia() // Enables nvidia-drm.modeset=1
            parameter("nouveau.modeset", "0") // Disable nouveau
            
            // Console configuration
            parameter("console", "tty0")
            parameter("console", "ttyS0,115200n8")
            
            // Module configuration
            modules {
                // Blacklist problematic modules
                blacklist("nouveau", "radeon", "amdgpu", "pcspkr", "snd_pcsp", "iTCO_wdt")
                
                // Force load essential modules
                load("nvidia", "nvidia_modeset", "nvidia_uvm", "nvidia_drm")
                load("acpi_cpufreq", "cpufreq_ondemand")
                load("coretemp", "lm_sensors")
                
                // Module options
                option("nvidia", "modeset=1")
                option("nvidia", "NVreg_UsePageAttributeTable=1")
                option("snd_hda_intel", "enable_msi=1")
                option("i915", "enable_gvt=1") // For Intel graphics virtualization
                option("kvm_intel", "nested=1") // Enable nested virtualization
                option("bluetooth", "disable_ertm=1")
                
                autoLoad = true
                compression = ModuleCompression.ZSTD
            }
            
            // Kernel variant for testing
            variant("testing", "6.2.0-rc1-horizonos") {
                description = "Testing kernel with latest features"
                parameter("experimental", "1")
                parameter("debug", "1")
                initrd = "/initramfs-linux-testing.img"
            }
            
            // LTS kernel variant
            variant("lts", "5.15.0-lts-horizonos") {
                description = "Long Term Support kernel for stability"
                parameter("mitigations", "auto")
                parameter("spectre_v2", "retpoline")
            }
            
            // Debug configuration (disabled by default)
            debugging {
                enabled = false
                debugLevel = 0
                earlyPrintk = false
                ignore_loglevel = false
                printk_time = false
                crashkernel = "256M" // Reserve memory for crash dumps
            }
            
            // Security hardening
            security {
                kaslr = true // Kernel Address Space Layout Randomization
                smep = true  // Supervisor Mode Execution Prevention
                smap = true  // Supervisor Mode Access Prevention
                pti = true   // Page Table Isolation (Meltdown mitigation)
                spectre_v2 = SpectreV2Mitigation.RETPOLINE
                meltdown = true
                l1tf = L1TFMitigation.FLUSH // L1 Terminal Fault mitigation
                mds = MDSMitigation.FULL    // Microarchitectural Data Sampling
                selinux = false // Use AppArmor instead
                apparmor = true
            }
        }
        
        // Advanced initramfs configuration
        initramfs {
            generator = InitramfsGenerator.MKINITCPIO
            compression = InitramfsCompression.ZSTD
            
            // Essential modules for boot
            modules(
                "ext4", "btrfs", "xfs",           // Filesystems
                "dm_mod", "dm_crypt", "dm_integrity", // Device mapper
                "nvme", "ahci", "sd_mod",         // Storage drivers
                "hid_generic", "usbhid",          // Input devices
                "nvidia", "nvidia_modeset"        // Graphics (if needed early)
            )
            
            // Boot hooks in correct order
            hooks(
                "base",
                "systemd",           // Use systemd in initramfs
                "autodetect",        // Auto-detect hardware
                "modconf",           // Module configuration
                "block",             // Block device support
                "encrypt",           // LUKS encryption support
                "lvm2",             // LVM support
                "filesystems",       // Filesystem drivers
                "keyboard",          // Keyboard support
                "fsck"              // Filesystem check
            )
            
            // Additional files to include
            files(
                "/etc/crypttab",
                "/etc/lvm/lvm.conf",
                "/usr/local/bin/unlock-script.sh"
            )
            
            // Microcode configuration
            microcode {
                enabled = true
                intel = true    // Intel CPU microcode
                amd = true      // AMD CPU microcode  
                early = true    // Load microcode early
            }
            
            // LUKS encryption configuration
            encryption {
                method = EncryptionMethod.LUKS2
                keyfile = "/etc/luks/keyfile"
                keyslot = 0
                tries = 3
                timeout = 30.seconds
            }
            
            // Custom initialization script
            customScript("/usr/local/bin/custom-early-init.sh")
        }
        
        // Plymouth boot splash configuration
        plymouth {
            enabled = true
            theme = "horizonos-spinner"
            showDelay = 0.seconds      // Show splash immediately
            deviceTimeout = 5.seconds  // Wait for devices
            modules("drm", "nvidia_drm") // Graphics modules for splash
            plugins("fade-thru", "two-step")
            quietBoot = true
            showSplash = true
        }
        
        // Secure Boot configuration (commented out - requires manual setup)
        secureBoot {
            enabled = false  // Enable only after proper key setup
            mokManager = true
            signKernel = false
            signModules = false
            enrollKeys = false
            
            // Secure Boot keys (paths would need to be created manually)
            keys {
                platform = "/etc/secureboot/PK.esl"
                keyExchange = "/etc/secureboot/KEK.esl"
                signature = "/etc/secureboot/db.esl"
                forbidden = "/etc/secureboot/dbx.esl"
            }
        }
        
        // Recovery boot configuration
        recovery {
            enabled = true
            timeout = 30.seconds
            autoSelect = false
            hideFromMenu = false
            
            // Recovery-specific kernel parameters
            parameter("single")
            parameter("systemd.unit", "rescue.target")
            parameter("rd.debug")
            parameter("rd.shell")
            parameter("systemd.log_level", "debug")
        }
    }
    
    // Install boot-related packages
    packages {
        install(
            // Bootloader packages
            "systemd-boot", "efibootmgr",
            
            // Kernel packages
            "linux", "linux-headers", "linux-firmware",
            
            // Boot utilities
            "mkinitcpio", "plymouth",
            
            // Microcode
            "intel-ucode", "amd-ucode",
            
            // Encryption support
            "cryptsetup", "lvm2",
            
            // Boot themes
            "plymouth-theme-horizonos"
        )
    }
    
    // Enable essential boot services
    services {
        enable("systemd-boot-update") {
            autoRestart = true
        }
        enable("plymouth-start")
        enable("plymouth-quit")
        enable("systemd-cryptsetup-generator")
    }
}