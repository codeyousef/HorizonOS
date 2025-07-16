package org.horizonos.config.dsl.boot.bootloader

import kotlinx.serialization.Serializable
import kotlin.time.Duration
import kotlin.time.Duration.Companion.seconds

/**
 * Bootloader Configuration for HorizonOS
 * 
 * This module provides configuration for system bootloaders including systemd-boot
 * and GRUB. It handles boot entries, theming, timeout settings, and bootloader-specific
 * options for the HorizonOS boot process.
 * 
 * ## Supported Bootloaders:
 * - **systemd-boot**: Modern UEFI bootloader with simple configuration
 * - **GRUB**: Traditional bootloader with extensive customization options
 * 
 * ## Key Features:
 * - Boot entry management with kernel options
 * - Theme and visual customization
 * - Fallback and recovery boot options
 * - Automatic boot entry generation
 * - UEFI and legacy BIOS support
 * 
 * @since 1.0
 */

// ===== Bootloader Configuration =====

/**
 * Main bootloader configuration for HorizonOS
 * 
 * This class defines the complete bootloader setup including type, timeout settings,
 * boot entries, theming, and bootloader-specific configurations.
 */
@Serializable
data class BootloaderConfig(
    /** Type of bootloader to use (systemd-boot, GRUB, etc.) */
    val type: BootloaderType = BootloaderType.SYSTEMD_BOOT,
    
    /** Boot timeout duration before selecting default entry */
    val timeout: Duration = 5.seconds,
    
    /** Default boot entry to select if no user input */
    val defaultEntry: String? = null,
    
    /** Fallback entry to use if default entry fails */
    val fallbackEntry: String? = null,
    
    /** List of custom boot entries to create */
    val entries: List<BootEntry> = emptyList(),
    
    /** Theme name for bootloader appearance */
    val theme: String? = null,
    
    /** Display resolution for boot screen (e.g., "1920x1080") */
    val resolution: String? = null,
    
    /** Console mode for boot screen display */
    val consoleMode: ConsoleMode = ConsoleMode.AUTO,
    
    /** Whether to enable boot entry editor */
    val editor: Boolean = true,
    
    /** Whether to automatically generate boot entries */
    val autoEntries: Boolean = true,
    
    /** Whether to automatically detect firmware settings */
    val autoFirmware: Boolean = true,
    
    /** GRUB-specific configuration (only used when type is GRUB) */
    val grubConfig: GrubConfig? = null
)

/**
 * Boot entry configuration for custom boot options
 * 
 * This class defines a single boot entry with kernel, initrd, and boot options.
 * Boot entries can be customized for specific hardware or boot scenarios.
 */
@Serializable
data class BootEntry(
    /** Display title for the boot entry */
    val title: String,
    
    /** Path to the Linux kernel image */
    val linux: String,
    
    /** Path to the initial RAM disk (optional) */
    val initrd: String? = null,
    
    /** List of kernel boot options/parameters */
    val options: List<String> = emptyList(),
    
    /** Path to device tree blob for ARM systems (optional) */
    val devicetree: String? = null,
    
    /** Target architecture for this boot entry (optional) */
    val architecture: String? = null,
    
    /** Version string for this boot entry (optional) */
    val version: String? = null,
    
    /** Machine ID for this boot entry (optional) */
    val machineId: String? = null,
    
    /** Sort order for displaying boot entries (optional) */
    val sort: Int? = null
)

/**
 * GRUB-specific bootloader configuration
 * 
 * This class contains configuration options specific to GRUB bootloader,
 * including display settings, timeout, and GRUB-specific features.
 */
@Serializable
data class GrubConfig(
    /** Distributor name shown in GRUB menu */
    val distributor: String = "HorizonOS",
    
    /** Default timeout for GRUB menu */
    val defaultTimeout: Duration = 5.seconds,
    
    /** GRUB theme name for menu appearance */
    val theme: String? = null,
    
    /** Background image path for GRUB menu */
    val background: String? = null,
    
    /** Graphics mode for GRUB display (e.g., "auto", "1920x1080") */
    val gfxMode: String = "auto",
    
    /** Graphics payload mode for kernel handoff */
    val gfxPayload: String = "keep",
    
    /** Whether to record boot failure count */
    val recordFailCount: Boolean = true,
    
    /** Whether to show submenus only once */
    val submenuShowOnce: Boolean = true,
    
    /** Whether to disable recovery mode entries */
    val disableRecovery: Boolean = false,
    
    /** Whether to disable OS detection (os-prober) */
    val disableOsProber: Boolean = false,
    
    /** List of custom GRUB configuration entries */
    val customEntries: List<String> = emptyList()
)

// Bootloader Enums

/**
 * Supported bootloader types in HorizonOS
 * 
 * Each bootloader type has different capabilities and target use cases.
 */
@Serializable
enum class BootloaderType {
    /** systemd-boot - Modern UEFI-only bootloader with simple configuration */
    SYSTEMD_BOOT,
    
    /** GRUB - Traditional bootloader with extensive features and legacy support */
    GRUB,
    
    /** rEFInd - Graphical UEFI boot manager with automatic OS detection */
    REFIND,
    
    /** SYSLINUX - Lightweight bootloader for legacy systems */
    SYSLINUX,
    
    /** Limine - Modern bootloader with advanced features */
    LIMINE,
    
    /** Direct kernel boot - Boot kernel directly without bootloader */
    DIRECT_KERNEL_BOOT,
    
    /** UEFI stub - Boot using kernel's built-in UEFI stub loader */
    UEFI_STUB,
    
    /** Custom bootloader - User-defined bootloader configuration */
    CUSTOM
}

/**
 * Console mode options for bootloader display
 * 
 * Controls how the bootloader displays the boot menu and console output.
 */
@Serializable
enum class ConsoleMode {
    /** Automatically detect best console mode */
    AUTO,
    
    /** Keep current console mode unchanged */
    KEEP,
    
    /** Force text mode console */
    TEXT,
    
    /** Use maximum available console resolution */
    MAX
}