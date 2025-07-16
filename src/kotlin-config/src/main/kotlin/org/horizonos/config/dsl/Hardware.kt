package org.horizonos.config.dsl

import kotlinx.serialization.Serializable
import org.horizonos.config.dsl.hardware.*

/**
 * Hardware Configuration DSL for HorizonOS
 * 
 * Provides comprehensive hardware configuration and driver management for HorizonOS systems.
 * This module handles automatic hardware detection, driver installation, and optimization
 * for various hardware components including graphics cards, input devices, displays,
 * power management, and specialized hardware.
 * 
 * ## Hardware Categories:
 * - **Graphics**: GPU drivers (NVIDIA, AMD, Intel), multi-GPU setups, hardware acceleration
 * - **Display**: Monitor configuration, resolution, refresh rates, color calibration
 * - **Input**: Keyboard, mouse, touchpad, touchscreen, gaming controllers
 * - **Audio**: Sound cards, speakers, microphones, audio routing
 * - **Power**: Battery management, CPU frequency scaling, power profiles
 * - **Thermal**: Temperature monitoring, fan control, thermal throttling
 * - **Storage**: SSD/HDD configuration, SMART monitoring, RAID controllers
 * - **Networking**: WiFi cards, Bluetooth adapters, Ethernet controllers
 * - **USB**: USB device management, power delivery, device-specific drivers
 * - **Sensors**: Temperature, humidity, accelerometer, gyroscope sensors
 * 
 * ## Key Features:
 * - **Automatic Detection**: Hardware is automatically detected and configured
 * - **Driver Management**: Automatic driver installation and updates
 * - **Performance Optimization**: Hardware-specific performance tuning
 * - **Power Efficiency**: Intelligent power management and energy savings
 * - **Hot-plug Support**: Dynamic hardware addition and removal
 * - **Multi-GPU Support**: Advanced GPU configuration for gaming/compute workloads
 * 
 * ## Basic Usage:
 * ```kotlin
 * hardware {
 *     gpu {
 *         vendor = GPUVendor.NVIDIA
 *         driver = "nvidia-dkms"
 *         cuda = true
 *         
 *         performance {
 *             profile = PerformanceProfile.HIGH_PERFORMANCE
 *             overclocking = false
 *         }
 *     }
 *     
 *     display {
 *         monitor("primary") {
 *             resolution = "1920x1080"
 *             refreshRate = 60
 *             position = DisplayPosition.PRIMARY
 *         }
 *         
 *         monitor("secondary") {
 *             resolution = "1920x1080"
 *             refreshRate = 60
 *             position = DisplayPosition.RIGHT
 *         }
 *     }
 *     
 *     power {
 *         profile = PowerProfile.BALANCED
 *         
 *         cpu {
 *             governor = "schedutil"
 *             maxFrequency = "auto"
 *         }
 *         
 *         battery {
 *             chargingThreshold = 80
 *             lowBatteryWarning = 20
 *         }
 *     }
 *     
 *     audio {
 *         backend = AudioBackend.PIPEWIRE
 *         
 *         device("speakers") {
 *             type = AudioDeviceType.OUTPUT
 *             channels = 2
 *             sampleRate = 44100
 *         }
 *     }
 * }
 * ```
 * 
 * ## Hardware Profiles:
 * HorizonOS supports different hardware profiles optimized for specific use cases:
 * - **Desktop**: Optimized for desktop workstations with focus on performance
 * - **Laptop**: Optimized for laptops with focus on battery life and portability
 * - **Gaming**: Optimized for gaming with high-performance graphics and low latency
 * - **Server**: Optimized for server hardware with focus on reliability and efficiency
 * - **Embedded**: Optimized for embedded systems with minimal resource usage
 * 
 * @since 1.0
 * @see [GPUConfig] for graphics card configuration
 * @see [DisplayConfig] for display and monitor configuration
 * @see [PowerConfig] for power management configuration
 * @see [AudioConfig] for audio system configuration
 * @see [Security] for hardware security features
 * @see [SecurityConfig] for security configuration
 * @see [Network] for network hardware configuration
 * @see [NetworkConfig] for network interfaces and connectivity
 * @see [Boot] for hardware boot configuration
 * @see [Storage] for storage hardware configuration
 * @see [horizonOS] for main system configuration entry point
 */

// ===== Main Hardware Configuration =====

@Serializable
data class HardwareConfig(
    val gpu: GPUConfig = GPUConfig(),
    val input: InputConfig = InputConfig(),
    val display: DisplayConfig = DisplayConfig(),
    val power: PowerConfig = PowerConfig(),
    val thermal: ThermalConfig = ThermalConfig(),
    val audio: AudioConfig = AudioConfig(),
    val storage: StorageHardwareConfig = StorageHardwareConfig(),
    val networking: NetworkHardwareConfig = NetworkHardwareConfig(),
    val usb: USBConfig = USBConfig(),
    val bluetooth: BluetoothConfig = BluetoothConfig(),
    val sensors: SensorConfig = SensorConfig()
)

// ===== DSL Builder =====

@HorizonOSDsl
class HardwareContext {
    private var gpu = GPUConfig()
    private var input = InputConfig()
    private var display = DisplayConfig()
    private var power = PowerConfig()
    private var thermal = ThermalConfig()
    private var audio = AudioConfig()
    private var storage = StorageHardwareConfig()
    private var networking = NetworkHardwareConfig()
    private var usb = USBConfig()
    private var bluetooth = BluetoothConfig()
    private var sensors = SensorConfig()
    
    fun gpu(block: GPUContext.() -> Unit) {
        gpu = GPUContext().apply(block).toConfig()
    }
    
    fun toConfig() = HardwareConfig(
        gpu = gpu,
        input = input,
        display = display,
        power = power,
        thermal = thermal,
        audio = audio,
        storage = storage,
        networking = networking,
        usb = usb,
        bluetooth = bluetooth,
        sensors = sensors
    )
}

@HorizonOSDsl
class GPUContext {
    var primary: GPUDriver = GPUDriver.AUTO_DETECT
    
    fun toConfig() = GPUConfig(primary = primary)
}

// Legacy Bluetooth configuration (kept for compatibility)
@Serializable
data class BluetoothConfig(
    val enabled: Boolean = true,
    val adapters: List<BluetoothAdapter> = emptyList(),
    val pairedDevices: List<BluetoothDevice> = emptyList(),
    val powerManagement: Boolean = true,
    val fastConnectable: Boolean = true,
    val privacy: BluetoothPrivacy = BluetoothPrivacy.DEVICE
)

@Serializable
data class BluetoothDevice(
    val name: String,
    val address: String,
    val deviceClass: String? = null,
    val trusted: Boolean = false,
    val autoConnect: Boolean = true
)