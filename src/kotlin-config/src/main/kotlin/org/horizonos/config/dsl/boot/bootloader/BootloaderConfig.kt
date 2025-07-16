package org.horizonos.config.dsl.boot.bootloader

import kotlinx.serialization.Serializable
import kotlin.time.Duration
import kotlin.time.Duration.Companion.seconds

// ===== Bootloader Configuration =====

@Serializable
data class BootloaderConfig(
    val type: BootloaderType = BootloaderType.SYSTEMD_BOOT,
    val timeout: Duration = 5.seconds,
    val defaultEntry: String? = null,
    val fallbackEntry: String? = null,
    val entries: List<BootEntry> = emptyList(),
    val theme: String? = null,
    val resolution: String? = null,
    val consoleMode: ConsoleMode = ConsoleMode.AUTO,
    val editor: Boolean = true,
    val autoEntries: Boolean = true,
    val autoFirmware: Boolean = true,
    val grubConfig: GrubConfig? = null
)

@Serializable
data class BootEntry(
    val title: String,
    val linux: String,
    val initrd: String? = null,
    val options: List<String> = emptyList(),
    val devicetree: String? = null,
    val architecture: String? = null,
    val version: String? = null,
    val machineId: String? = null,
    val sort: Int? = null
)

@Serializable
data class GrubConfig(
    val distributor: String = "HorizonOS",
    val defaultTimeout: Duration = 5.seconds,
    val theme: String? = null,
    val background: String? = null,
    val gfxMode: String = "auto",
    val gfxPayload: String = "keep",
    val recordFailCount: Boolean = true,
    val submenuShowOnce: Boolean = true,
    val disableRecovery: Boolean = false,
    val disableOsProber: Boolean = false,
    val customEntries: List<String> = emptyList()
)

// Bootloader Enums
@Serializable
enum class BootloaderType {
    SYSTEMD_BOOT,
    GRUB,
    REFIND,
    SYSLINUX,
    LIMINE,
    DIRECT_KERNEL_BOOT,
    UEFI_STUB,
    CUSTOM
}

@Serializable
enum class ConsoleMode {
    AUTO,
    KEEP,
    TEXT,
    MAX
}