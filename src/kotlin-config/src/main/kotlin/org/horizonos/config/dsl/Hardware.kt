package org.horizonos.config.dsl

import kotlinx.serialization.Serializable
import org.horizonos.config.dsl.hardware.*

/**
 * Hardware Configuration DSL for HorizonOS
 * 
 * Provides type-safe configuration for hardware components including
 * GPU drivers, input devices, displays, power management, and thermal control.
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