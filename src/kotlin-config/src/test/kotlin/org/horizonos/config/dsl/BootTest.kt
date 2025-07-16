package org.horizonos.config.dsl

import io.kotest.core.spec.style.StringSpec
import io.kotest.matchers.shouldBe
import io.kotest.matchers.shouldNotBe
import io.kotest.matchers.collections.shouldHaveSize
import io.kotest.matchers.collections.shouldContain
import io.kotest.matchers.booleans.shouldBeTrue
import io.kotest.matchers.booleans.shouldBeFalse
import kotlin.time.Duration.Companion.seconds

class BootTest : StringSpec({
    
    "should create basic boot configuration with systemd-boot" {
        val config = horizonOS {
            hostname = "boot-test"
            timezone = "UTC"
            locale = "en_US.UTF-8"
            
            boot {
                bootloader {
                    type = BootloaderType.SYSTEMD_BOOT
                    timeout = 10.seconds
                    defaultEntry = "horizonos"
                    editor = false
                    
                    entry("HorizonOS") {
                        linux = "/vmlinuz-linux"
                        initrd = "/initramfs-linux.img"
                        options("root=LABEL=root", "rw", "quiet")
                    }
                    
                    entry("HorizonOS Fallback") {
                        linux = "/vmlinuz-linux"
                        initrd = "/initramfs-linux-fallback.img"
                        options("root=LABEL=root", "rw")
                    }
                }
                
                kernel {
                    quiet()
                    splash()
                    rootDevice("LABEL=root")
                    rootfsType("ext4")
                    
                    parameter("cryptdevice", "UUID=1234-5678:root")
                    parameter("resume", "UUID=abcd-efgh")
                }
            }
        }
        
        config.boot shouldNotBe null
        config.hasBoot().shouldBeTrue()
        
        val boot = config.boot!!
        boot.bootloader.type shouldBe BootloaderType.SYSTEMD_BOOT
        boot.bootloader.timeout shouldBe 10.seconds
        boot.bootloader.defaultEntry shouldBe "horizonos"
        boot.bootloader.editor.shouldBeFalse()
        boot.bootloader.entries shouldHaveSize 2
        
        val mainEntry = config.getBootEntry("HorizonOS")
        mainEntry shouldNotBe null
        mainEntry!!.linux shouldBe "/vmlinuz-linux"
        mainEntry.initrd shouldBe "/initramfs-linux.img"
        mainEntry.options shouldContain "quiet"
        mainEntry.options shouldContain "rw"
        
        val fallbackEntry = config.getBootEntry("HorizonOS Fallback")
        fallbackEntry shouldNotBe null
        fallbackEntry!!.initrd shouldBe "/initramfs-linux-fallback.img"
        
        boot.kernel.parameters shouldHaveSize 6
        val rootParam = config.getKernelParameter("root")
        rootParam shouldNotBe null
        rootParam!!.value shouldBe "LABEL=root"
        
        val cryptParam = config.getKernelParameter("cryptdevice")
        cryptParam shouldNotBe null
        cryptParam!!.value shouldBe "UUID=1234-5678:root"
    }
    
    "should configure GRUB bootloader" {
        val config = horizonOS {
            hostname = "grub-test"
            timezone = "UTC"
            locale = "en_US.UTF-8"
            
            boot {
                bootloader {
                    type = BootloaderType.GRUB
                    timeout = 5.seconds
                    
                    grub {
                        distributor = "HorizonOS"
                        defaultTimeout = 5.seconds
                        theme = "/boot/grub/themes/horizonos"
                        background = "/boot/grub/wallpaper.png"
                        gfxMode = "1920x1080"
                        gfxPayload = "keep"
                        disableRecovery = false
                        disableOsProber = true
                        
                        customEntry("menuentry 'Custom Boot Option' {")
                        customEntry("  linux /custom-kernel")
                        customEntry("}")
                    }
                }
            }
        }
        
        val boot = config.boot!!
        boot.bootloader.type shouldBe BootloaderType.GRUB
        boot.bootloader.grubConfig shouldNotBe null
        
        val grub = boot.bootloader.grubConfig!!
        grub.distributor shouldBe "HorizonOS"
        grub.defaultTimeout shouldBe 5.seconds
        grub.theme shouldBe "/boot/grub/themes/horizonos"
        grub.background shouldBe "/boot/grub/wallpaper.png"
        grub.gfxMode shouldBe "1920x1080"
        grub.gfxPayload shouldBe "keep"
        grub.disableRecovery.shouldBeFalse()
        grub.disableOsProber.shouldBeTrue()
        grub.customEntries shouldHaveSize 3
        grub.customEntries shouldContain "menuentry 'Custom Boot Option' {"
    }
    
    "should configure kernel parameters and modules" {
        val config = horizonOS {
            hostname = "kernel-test"
            timezone = "UTC"
            locale = "en_US.UTF-8"
            
            boot {
                kernel {
                    compression = KernelCompression.ZSTD
                    version = "6.1.0-horizonos"
                    
                    parameter("console", "tty0")
                    parameter("console", "ttyS0,115200")
                    parameter("intel_iommu", "on")
                    parameter("iommu", "pt")
                    nvidia()
                    
                    modules {
                        blacklist("nouveau", "radeon")
                        load("nvidia", "nvidia_modeset", "nvidia_uvm")
                        option("nvidia", "modeset=1")
                        option("snd_hda_intel", "enable_msi=1")
                        compression = ModuleCompression.ZSTD
                    }
                    
                    variant("lts", "6.1.0-lts") {
                        description = "Long Term Support kernel"
                        parameter("mitigations", "auto")
                        parameter("pti", "on")
                    }
                    
                    debugging {
                        enabled = true
                        debugLevel = 3
                        earlyPrintk = true
                        crashkernel = "256M"
                    }
                    
                    security {
                        kaslr = true
                        smep = true
                        smap = true
                        pti = true
                        spectre_v2 = SpectreV2Mitigation.RETPOLINE
                        meltdown = true
                        l1tf = L1TFMitigation.FLUSH
                        mds = MDSMitigation.FULL
                        selinux = false
                        apparmor = true
                    }
                }
            }
        }
        
        val kernel = config.boot!!.kernel
        kernel.compression shouldBe KernelCompression.ZSTD
        kernel.version shouldBe "6.1.0-horizonos"
        
        // Check kernel parameters
        kernel.parameters shouldHaveSize 5 // quiet, splash, root*, rootfstype, nvidia-drm.modeset
        val consoleParams = kernel.parameters.filter { it.name == "console" }
        consoleParams shouldHaveSize 2
        
        val nvidiaParam = kernel.parameters.find { it.name == "nvidia-drm.modeset" }
        nvidiaParam shouldNotBe null
        nvidiaParam!!.value shouldBe "1"
        
        // Check module configuration
        kernel.modules.blacklist shouldContain "nouveau"
        kernel.modules.blacklist shouldContain "radeon"
        kernel.modules.load shouldContain "nvidia"
        kernel.modules.load shouldContain "nvidia_modeset"
        kernel.modules.options["nvidia"] shouldBe "modeset=1"
        kernel.modules.options["snd_hda_intel"] shouldBe "enable_msi=1"
        kernel.modules.compression shouldBe ModuleCompression.ZSTD
        
        // Check kernel variant
        kernel.variants shouldHaveSize 1
        val ltsVariant = config.getKernelVariant("lts")
        ltsVariant shouldNotBe null
        ltsVariant!!.version shouldBe "6.1.0-lts"
        ltsVariant.description shouldBe "Long Term Support kernel"
        ltsVariant.parameters shouldHaveSize 2
        
        // Check debugging configuration
        kernel.debugging.enabled.shouldBeTrue()
        kernel.debugging.debugLevel shouldBe 3
        kernel.debugging.earlyPrintk.shouldBeTrue()
        kernel.debugging.crashkernel shouldBe "256M"
        
        // Check security configuration
        kernel.security.kaslr.shouldBeTrue()
        kernel.security.smep.shouldBeTrue()
        kernel.security.smap.shouldBeTrue()
        kernel.security.pti.shouldBeTrue()
        kernel.security.spectre_v2 shouldBe SpectreV2Mitigation.RETPOLINE
        kernel.security.meltdown.shouldBeTrue()
        kernel.security.l1tf shouldBe L1TFMitigation.FLUSH
        kernel.security.mds shouldBe MDSMitigation.FULL
        kernel.security.selinux.shouldBeFalse()
        kernel.security.apparmor.shouldBeTrue()
    }
    
    "should configure initramfs with mkinitcpio" {
        val config = horizonOS {
            hostname = "initramfs-test"
            timezone = "UTC"
            locale = "en_US.UTF-8"
            
            boot {
                initramfs {
                    generator = InitramfsGenerator.MKINITCPIO
                    compression = InitramfsCompression.ZSTD
                    
                    modules("ext4", "btrfs", "dm_crypt", "dm_mod")
                    hooks("base", "udev", "autodetect", "modconf", "block", "encrypt", "filesystems", "keyboard", "fsck")
                    files("/etc/crypttab")
                    
                    microcode {
                        enabled = true
                        intel = true
                        amd = true
                        early = true
                    }
                    
                    encryption {
                        method = EncryptionMethod.LUKS2
                        keyfile = "/etc/luks/keyfile"
                        keyslot = 1
                        tries = 3
                        timeout = 30.seconds
                    }
                    
                    customScript("/usr/local/bin/custom-init.sh")
                }
            }
        }
        
        val initramfs = config.boot!!.initramfs
        initramfs.generator shouldBe InitramfsGenerator.MKINITCPIO
        initramfs.compression shouldBe InitramfsCompression.ZSTD
        
        initramfs.modules shouldContain "ext4"
        initramfs.modules shouldContain "btrfs"
        initramfs.modules shouldContain "dm_crypt"
        initramfs.modules shouldContain "dm_mod"
        
        initramfs.hooks shouldContain "base"
        initramfs.hooks shouldContain "encrypt"
        initramfs.hooks shouldContain "filesystems"
        
        initramfs.files shouldContain "/etc/crypttab"
        initramfs.customScripts shouldContain "/usr/local/bin/custom-init.sh"
        
        // Check microcode configuration
        initramfs.microcode.enabled.shouldBeTrue()
        initramfs.microcode.intel.shouldBeTrue()
        initramfs.microcode.amd.shouldBeTrue()
        initramfs.microcode.early.shouldBeTrue()
        
        // Check encryption configuration
        initramfs.encryption shouldNotBe null
        initramfs.encryption!!.method shouldBe EncryptionMethod.LUKS2
        initramfs.encryption!!.keyfile shouldBe "/etc/luks/keyfile"
        initramfs.encryption!!.keyslot shouldBe 1
        initramfs.encryption!!.tries shouldBe 3
        initramfs.encryption!!.timeout shouldBe 30.seconds
    }
    
    "should configure initramfs with dracut" {
        val config = horizonOS {
            hostname = "dracut-test"
            timezone = "UTC"
            locale = "en_US.UTF-8"
            
            boot {
                initramfs {
                    generator = InitramfsGenerator.DRACUT
                    compression = InitramfsCompression.XZ
                    
                    modules("dm", "crypt", "lvm", "resume")
                    files("/etc/dracut.conf.d/custom.conf")
                }
            }
        }
        
        val initramfs = config.boot!!.initramfs
        initramfs.generator shouldBe InitramfsGenerator.DRACUT
        initramfs.compression shouldBe InitramfsCompression.XZ
        initramfs.modules shouldContain "dm"
        initramfs.modules shouldContain "crypt"
        initramfs.modules shouldContain "lvm"
        initramfs.modules shouldContain "resume"
    }
    
    "should configure Plymouth boot splash" {
        val config = horizonOS {
            hostname = "plymouth-test"
            timezone = "UTC"
            locale = "en_US.UTF-8"
            
            boot {
                plymouth {
                    enabled = true
                    theme = "horizonos-spinner"
                    showDelay = 2.seconds
                    deviceTimeout = 10.seconds
                    modules("drm", "nouveau")
                    plugins("fade-thru", "two-step")
                    quietBoot = true
                    showSplash = true
                }
            }
        }
        
        val plymouth = config.boot!!.plymouth
        plymouth.enabled.shouldBeTrue()
        plymouth.theme shouldBe "horizonos-spinner"
        plymouth.showDelay shouldBe 2.seconds
        plymouth.deviceTimeout shouldBe 10.seconds
        plymouth.modules shouldContain "drm"
        plymouth.modules shouldContain "nouveau"
        plymouth.plugins shouldContain "fade-thru"
        plymouth.plugins shouldContain "two-step"
        plymouth.quietBoot.shouldBeTrue()
        plymouth.showSplash.shouldBeTrue()
    }
    
    "should configure Secure Boot" {
        val config = horizonOS {
            hostname = "secureboot-test"
            timezone = "UTC"
            locale = "en_US.UTF-8"
            
            boot {
                secureBoot {
                    enabled = true
                    mokManager = true
                    signKernel = true
                    signModules = true
                    enrollKeys = true
                    
                    keys {
                        platform = "/etc/secureboot/PK.auth"
                        keyExchange = "/etc/secureboot/KEK.auth"
                        signature = "/etc/secureboot/db.auth"
                        forbidden = "/etc/secureboot/dbx.auth"
                    }
                }
            }
        }
        
        val secureBoot = config.boot!!.secureBoot
        secureBoot.enabled.shouldBeTrue()
        secureBoot.mokManager.shouldBeTrue()
        secureBoot.signKernel.shouldBeTrue()
        secureBoot.signModules.shouldBeTrue()
        secureBoot.enrollKeys.shouldBeTrue()
        
        secureBoot.keys shouldNotBe null
        secureBoot.keys!!.platform shouldBe "/etc/secureboot/PK.auth"
        secureBoot.keys!!.keyExchange shouldBe "/etc/secureboot/KEK.auth"
        secureBoot.keys!!.signature shouldBe "/etc/secureboot/db.auth"
        secureBoot.keys!!.forbidden shouldBe "/etc/secureboot/dbx.auth"
    }
    
    "should configure recovery options" {
        val config = horizonOS {
            hostname = "recovery-test"
            timezone = "UTC"
            locale = "en_US.UTF-8"
            
            boot {
                recovery {
                    enabled = true
                    timeout = 60.seconds
                    autoSelect = false
                    hideFromMenu = false
                    
                    parameter("single")
                    parameter("systemd.unit", "rescue.target")
                    parameter("rd.debug")
                }
            }
        }
        
        val recovery = config.boot!!.recovery
        recovery.enabled.shouldBeTrue()
        recovery.timeout shouldBe 60.seconds
        recovery.autoSelect.shouldBeFalse()
        recovery.hideFromMenu.shouldBeFalse()
        recovery.kernelParameters shouldHaveSize 3
        
        val singleParam = recovery.kernelParameters.find { it.name == "single" }
        singleParam shouldNotBe null
        singleParam!!.value shouldBe null
        
        val unitParam = recovery.kernelParameters.find { it.name == "systemd.unit" }
        unitParam shouldNotBe null
        unitParam!!.value shouldBe "rescue.target"
    }
    
    "should handle complex boot configuration with multiple bootloaders" {
        val config = horizonOS {
            hostname = "complex-boot"
            timezone = "UTC"
            locale = "en_US.UTF-8"
            
            boot {
                bootloader {
                    type = BootloaderType.REFIND
                    timeout = 15.seconds
                    defaultEntry = "horizonos-main"
                    fallbackEntry = "horizonos-fallback"
                    theme = "horizonos-refind"
                    resolution = "1920x1080"
                    consoleMode = ConsoleMode.TEXT
                    editor = true
                    autoEntries = true
                    autoFirmware = false
                    
                    entry("HorizonOS Main") {
                        title = "HorizonOS Main"
                        linux = "/EFI/horizonos/vmlinuz"
                        initrd = "/EFI/horizonos/initramfs.img"
                        options("root=UUID=12345678-1234-1234-1234-123456789abc", "rw", "quiet", "splash")
                        architecture = "x64"
                        version = "6.1.0"
                        sort = 1
                    }
                    
                    entry("HorizonOS Fallback") {
                        title = "HorizonOS Fallback"
                        linux = "/EFI/horizonos/vmlinuz"
                        initrd = "/EFI/horizonos/initramfs-fallback.img"
                        options("root=UUID=12345678-1234-1234-1234-123456789abc", "rw")
                        architecture = "x64"
                        version = "6.1.0"
                        sort = 2
                    }
                }
                
                kernel {
                    compression = KernelCompression.LZ4
                    
                    parameter("console", "tty0")
                    parameter("console", "ttyS0,115200n8")
                    parameter("elevator", "deadline")
                    parameter("audit", "0")
                    parameter("transparent_hugepage", "madvise")
                    
                    modules {
                        blacklist("pcspkr", "snd_pcsp")
                        load("acpi_cpufreq", "cpufreq_ondemand")
                        option("i915", "enable_gvt=1")
                        option("kvm_intel", "nested=1")
                        autoLoad = true
                        compression = ModuleCompression.XZ
                    }
                }
                
                initramfs {
                    generator = InitramfsGenerator.MKINITCPIO
                    compression = InitramfsCompression.LZ4
                    
                    modules("ext4", "xfs", "btrfs", "nvme")
                    hooks("base", "udev", "autodetect", "modconf", "block", "filesystems", "keyboard", "fsck")
                    
                    microcode {
                        enabled = true
                        intel = true
                        amd = false
                        early = true
                    }
                }
                
                plymouth {
                    enabled = true
                    theme = "horizonos-glow"
                    showDelay = 0.seconds
                    deviceTimeout = 8.seconds
                    modules("drm", "i915")
                    quietBoot = true
                    showSplash = true
                }
            }
        }
        
        val boot = config.boot!!
        
        // Verify bootloader configuration
        boot.bootloader.type shouldBe BootloaderType.REFIND
        boot.bootloader.timeout shouldBe 15.seconds
        boot.bootloader.defaultEntry shouldBe "horizonos-main"
        boot.bootloader.fallbackEntry shouldBe "horizonos-fallback"
        boot.bootloader.theme shouldBe "horizonos-refind"
        boot.bootloader.resolution shouldBe "1920x1080"
        boot.bootloader.consoleMode shouldBe ConsoleMode.TEXT
        boot.bootloader.editor.shouldBeTrue()
        boot.bootloader.autoEntries.shouldBeTrue()
        boot.bootloader.autoFirmware.shouldBeFalse()
        
        // Verify boot entries
        boot.bootloader.entries shouldHaveSize 2
        val mainEntry = boot.bootloader.entries.find { it.title == "HorizonOS Main" }
        mainEntry shouldNotBe null
        mainEntry!!.architecture shouldBe "x64"
        mainEntry.version shouldBe "6.1.0"
        mainEntry.sort shouldBe 1
        
        // Verify kernel configuration
        boot.kernel.compression shouldBe KernelCompression.LZ4
        boot.kernel.parameters.size shouldBe 5
        boot.kernel.modules.blacklist shouldContain "pcspkr"
        boot.kernel.modules.load shouldContain "acpi_cpufreq"
        boot.kernel.modules.options["i915"] shouldBe "enable_gvt=1"
        boot.kernel.modules.options["kvm_intel"] shouldBe "nested=1"
        
        // Verify initramfs configuration
        boot.initramfs.generator shouldBe InitramfsGenerator.MKINITCPIO
        boot.initramfs.compression shouldBe InitramfsCompression.LZ4
        boot.initramfs.modules shouldContain "nvme"
        boot.initramfs.hooks shouldContain "autodetect"
        boot.initramfs.microcode.intel.shouldBeTrue()
        boot.initramfs.microcode.amd.shouldBeFalse()
        
        // Verify Plymouth configuration
        boot.plymouth.theme shouldBe "horizonos-glow"
        boot.plymouth.modules shouldContain "i915"
        boot.plymouth.deviceTimeout shouldBe 8.seconds
    }
    
    "should provide helper functions for kernel parameters" {
        val config = horizonOS {
            hostname = "helper-test"
            timezone = "UTC"
            locale = "en_US.UTF-8"
            
            boot {
                kernel {
                    quiet()
                    splash()
                    nomodeset()
                    acpiOff()
                    rootDevice("/dev/sda1")
                    rootfsType("ext4")
                    resume("/dev/sda2")
                    cryptDevice("UUID=abc-123", "root")
                    nvidia("drm.modeset=1")
                }
            }
        }
        
        val parameters = config.boot!!.kernel.parameters
        parameters.find { it.name == "quiet" } shouldNotBe null
        parameters.find { it.name == "splash" } shouldNotBe null
        parameters.find { it.name == "nomodeset" } shouldNotBe null
        parameters.find { it.name == "acpi" }?.value shouldBe "off"
        parameters.find { it.name == "root" }?.value shouldBe "/dev/sda1"
        parameters.find { it.name == "rootfstype" }?.value shouldBe "ext4"
        parameters.find { it.name == "resume" }?.value shouldBe "/dev/sda2"
        parameters.find { it.name == "cryptdevice" }?.value shouldBe "UUID=abc-123:root"
        parameters.find { it.name == "nvidia-drm.modeset" }?.value shouldBe "1"
    }
})