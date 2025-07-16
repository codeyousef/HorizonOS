package org.horizonos.config.dsl.hardware

import kotlinx.serialization.Serializable
import kotlin.time.Duration
import kotlin.time.Duration.Companion.minutes

// ===== Storage Hardware Configuration =====

@Serializable
data class StorageHardwareConfig(
    val drives: List<DriveConfig> = emptyList(),
    val controllers: List<StorageController> = emptyList(),
    val optimization: StorageOptimization = StorageOptimization(),
    val monitoring: StorageMonitoring = StorageMonitoring()
)

@Serializable
data class DriveConfig(
    val device: String,
    val type: DriveType,
    val `interface`: StorageInterface,
    val scheduler: IOScheduler = IOScheduler.MQ_DEADLINE,
    val powerManagement: DrivePowerConfig = DrivePowerConfig(),
    val smart: SMARTConfig = SMARTConfig()
)

@Serializable
data class StorageController(
    val name: String,
    val type: ControllerType,
    val driver: String? = null,
    val firmware: String? = null,
    val options: Map<String, String> = emptyMap()
)

@Serializable
data class StorageOptimization(
    val trim: Boolean = true,
    val readahead: Int = 128,
    val nomerges: Int = 2,
    val rotational: Boolean? = null,
    val addRandom: Boolean = true
)

@Serializable
data class StorageMonitoring(
    val enabled: Boolean = true,
    val smartMonitoring: Boolean = true,
    val temperatureThreshold: Int = 60,
    val healthChecks: Boolean = true
)

@Serializable
data class DrivePowerConfig(
    val apm: Int = 254,
    val spindownTime: Duration? = null,
    val powerManagement: Boolean = true
)

@Serializable
data class SMARTConfig(
    val enabled: Boolean = true,
    val shortTest: Boolean = true,
    val longTest: Boolean = false,
    val schedule: String = "weekly"
)

// ===== Network Hardware Configuration =====

@Serializable
data class NetworkHardwareConfig(
    val ethernet: List<EthernetConfig> = emptyList(),
    val wireless: List<WirelessConfig> = emptyList(),
    val bluetooth: BluetoothHardwareConfig = BluetoothHardwareConfig(),
    val cellular: CellularConfig = CellularConfig()
)

@Serializable
data class EthernetConfig(
    val `interface`: String,
    val driver: String? = null,
    val speed: NetworkSpeed = NetworkSpeed.AUTO,
    val duplex: DuplexMode = DuplexMode.AUTO,
    val wakeOnLan: Boolean = false,
    val powerSaving: Boolean = true
)

@Serializable
data class WirelessConfig(
    val `interface`: String,
    val driver: String? = null,
    val powerSaving: Boolean = true,
    val scanRandomization: Boolean = true,
    val country: String = "US",
    val regulatory: RegulatoryMode = RegulatoryMode.WORLD
)

@Serializable
data class BluetoothHardwareConfig(
    val enabled: Boolean = true,
    val adapters: List<BluetoothAdapter> = emptyList(),
    val powerManagement: Boolean = true,
    val fastConnectable: Boolean = true,
    val privacy: BluetoothPrivacy = BluetoothPrivacy.DEVICE
)

@Serializable
data class BluetoothAdapter(
    val name: String,
    val address: String? = null,
    val enabled: Boolean = true,
    val discoverable: Boolean = false,
    val pairable: Boolean = true,
    val timeout: Duration = 3.minutes
)

@Serializable
data class CellularConfig(
    val enabled: Boolean = false,
    val modems: List<ModemConfig> = emptyList(),
    val roaming: Boolean = false,
    val dataLimit: DataLimit? = null
)

@Serializable
data class ModemConfig(
    val name: String,
    val apn: String? = null,
    val username: String? = null,
    val password: String? = null,
    val pinCode: String? = null
)

@Serializable
data class DataLimit(
    val monthly: Long, // MB
    val warning: Int = 80, // percentage
    val action: DataLimitAction = DataLimitAction.WARN
)

// ===== USB Configuration =====

@Serializable
data class USBConfig(
    val controllers: List<USBController> = emptyList(),
    val devices: List<USBDevice> = emptyList(),
    val powerManagement: USBPowerConfig = USBPowerConfig(),
    val mountRules: List<USBMountRule> = emptyList()
)

@Serializable
data class USBController(
    val name: String,
    val type: USBType,
    val enabled: Boolean = true,
    val powerManagement: Boolean = true
)

@Serializable
data class USBDevice(
    val vendorId: String,
    val productId: String,
    val name: String? = null,
    val authorized: Boolean = true,
    val powerManagement: Boolean = true
)

@Serializable
data class USBPowerConfig(
    val autosuspend: Boolean = true,
    val autosuspendDelay: Duration = 5.minutes,
    val wakeup: Boolean = true
)

@Serializable
data class USBMountRule(
    val vendorId: String? = null,
    val productId: String? = null,
    val filesystem: String? = null,
    val mountPoint: String,
    val options: List<String> = emptyList()
)

// ===== Enums =====

@Serializable
enum class DriveType {
    HDD,           // Hard Disk Drive
    SSD,           // Solid State Drive
    NVME,          // NVMe SSD
    OPTICAL,       // Optical drive
    FLOPPY,        // Floppy drive
    USB,           // USB storage
    SD_CARD        // SD card
}

@Serializable
enum class StorageInterface {
    SATA,          // Serial ATA
    PATA,          // Parallel ATA
    SCSI,          // Small Computer System Interface
    SAS,           // Serial Attached SCSI
    NVME,          // NVM Express
    USB,           // Universal Serial Bus
    FIREWIRE,      // FireWire
    THUNDERBOLT    // Thunderbolt
}

@Serializable
enum class IOScheduler {
    NOOP,          // No operation scheduler
    DEADLINE,      // Deadline scheduler
    CFQ,           // Completely Fair Queuing
    BFQ,           // Budget Fair Queuing
    MQ_DEADLINE,   // Multi-queue deadline
    KYBER,         // Kyber scheduler
    NONE           // No scheduler
}

@Serializable
enum class ControllerType {
    AHCI,          // Advanced Host Controller Interface
    IDE,           // Integrated Drive Electronics
    RAID,          // Redundant Array of Independent Disks
    NVME,          // NVM Express controller
    USB,           // USB controller
    SCSI           // SCSI controller
}

@Serializable
enum class NetworkSpeed {
    AUTO,          // Auto-negotiate speed
    SPEED_10,      // 10 Mbps
    SPEED_100,     // 100 Mbps
    SPEED_1000,    // 1 Gbps
    SPEED_2500,    // 2.5 Gbps
    SPEED_5000,    // 5 Gbps
    SPEED_10000    // 10 Gbps
}

@Serializable
enum class DuplexMode {
    AUTO,          // Auto-negotiate duplex
    HALF,          // Half duplex
    FULL           // Full duplex
}

@Serializable
enum class RegulatoryMode {
    WORLD,         // World regulatory domain
    COUNTRY,       // Country-specific domain
    CUSTOM         // Custom regulatory domain
}

@Serializable
enum class BluetoothPrivacy {
    DEVICE,        // Device privacy mode
    NETWORK        // Network privacy mode
}

@Serializable
enum class DataLimitAction {
    WARN,          // Warning only
    BLOCK,         // Block data usage
    THROTTLE       // Throttle data speed
}

@Serializable
enum class USBType {
    USB_1_0,       // USB 1.0
    USB_1_1,       // USB 1.1
    USB_2_0,       // USB 2.0
    USB_3_0,       // USB 3.0
    USB_3_1,       // USB 3.1
    USB_3_2,       // USB 3.2
    USB4,          // USB4
    USB_C          // USB-C
}