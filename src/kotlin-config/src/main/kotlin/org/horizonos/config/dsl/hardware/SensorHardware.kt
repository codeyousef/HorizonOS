package org.horizonos.config.dsl.hardware

import kotlinx.serialization.Serializable
import kotlin.time.Duration
import kotlin.time.Duration.Companion.seconds

// ===== Thermal Configuration =====

@Serializable
data class ThermalConfig(
    val enabled: Boolean = true,
    val zones: List<ThermalZone> = emptyList(),
    val coolingDevices: List<CoolingDevice> = emptyList(),
    val profiles: List<ThermalProfile> = emptyList(),
    val monitoring: ThermalMonitoring = ThermalMonitoring()
)

@Serializable
data class ThermalZone(
    val name: String,
    val type: String,
    val enabled: Boolean = true,
    val warningTemp: Int = 70,
    val criticalTemp: Int = 90,
    val passiveTemp: Int = 80,
    val polling: Duration = 10.seconds,
    val trips: List<ThermalTrip> = emptyList()
)

@Serializable
data class ThermalTrip(
    val name: String,
    val temperature: Int,
    val type: ThermalTripType,
    val action: ThermalAction = ThermalAction.NOTIFY
)

@Serializable
data class CoolingDevice(
    val name: String,
    val type: CoolingDeviceType,
    val enabled: Boolean = true,
    val minState: Int = 0,
    val maxState: Int = 100,
    val curve: List<CoolingPoint> = emptyList()
)

@Serializable
data class CoolingPoint(
    val temperature: Int,
    val state: Int
)

@Serializable
data class ThermalProfile(
    val name: String,
    val active: Boolean = false,
    val passiveMode: Boolean = true,
    val fanSpeed: FanSpeed = FanSpeed.AUTO,
    val temperatureTargets: Map<String, Int> = emptyMap()
)

@Serializable
data class ThermalMonitoring(
    val enabled: Boolean = true,
    val interval: Duration = 30.seconds,
    val logTemperatures: Boolean = false,
    val alertThreshold: Int = 85,
    val emergencyShutdown: Int = 100
)

// ===== Sensor Configuration =====

@Serializable
data class SensorConfig(
    val temperature: List<TemperatureSensor> = emptyList(),
    val voltage: List<VoltageSensor> = emptyList(),
    val fan: List<FanSensor> = emptyList(),
    val power: List<PowerSensor> = emptyList(),
    val accelerometer: AccelerometerConfig = AccelerometerConfig(),
    val gyroscope: GyroscopeConfig = GyroscopeConfig(),
    val magnetometer: MagnetometerConfig = MagnetometerConfig(),
    val ambientLight: AmbientLightConfig = AmbientLightConfig(),
    val proximity: ProximityConfig = ProximityConfig()
)

@Serializable
data class TemperatureSensor(
    val name: String,
    val chip: String,
    val feature: String,
    val enabled: Boolean = true,
    val offset: Double = 0.0,
    val scale: Double = 1.0,
    val alarms: List<TemperatureAlarm> = emptyList()
)

@Serializable
data class TemperatureAlarm(
    val threshold: Int,
    val type: AlarmType = AlarmType.HIGH,
    val action: SensorAction = SensorAction.LOG
)

@Serializable
data class VoltageSensor(
    val name: String,
    val chip: String,
    val feature: String,
    val enabled: Boolean = true,
    val nominal: Double,
    val tolerance: Double = 0.05,
    val alarms: List<VoltageAlarm> = emptyList()
)

@Serializable
data class VoltageAlarm(
    val minVoltage: Double,
    val maxVoltage: Double,
    val action: SensorAction = SensorAction.LOG
)

@Serializable
data class FanSensor(
    val name: String,
    val chip: String,
    val feature: String,
    val enabled: Boolean = true,
    val minRPM: Int = 0,
    val maxRPM: Int = 5000,
    val control: FanControl = FanControl.AUTO
)

@Serializable
data class PowerSensor(
    val name: String,
    val chip: String,
    val feature: String,
    val enabled: Boolean = true,
    val unit: PowerUnit = PowerUnit.WATTS
)

@Serializable
data class AccelerometerConfig(
    val enabled: Boolean = false,
    val device: String? = null,
    val sensitivity: Double = 1.0,
    val autoRotate: Boolean = false
)

@Serializable
data class GyroscopeConfig(
    val enabled: Boolean = false,
    val device: String? = null,
    val sensitivity: Double = 1.0
)

@Serializable
data class MagnetometerConfig(
    val enabled: Boolean = false,
    val device: String? = null,
    val calibration: MagnetometerCalibration? = null
)

@Serializable
data class MagnetometerCalibration(
    val offsetX: Double = 0.0,
    val offsetY: Double = 0.0,
    val offsetZ: Double = 0.0,
    val scaleX: Double = 1.0,
    val scaleY: Double = 1.0,
    val scaleZ: Double = 1.0
)

@Serializable
data class AmbientLightConfig(
    val enabled: Boolean = false,
    val device: String? = null,
    val autoAdjustBrightness: Boolean = false,
    val sensitivity: Double = 1.0
)

@Serializable
data class ProximityConfig(
    val enabled: Boolean = false,
    val device: String? = null,
    val threshold: Double = 5.0,
    val actions: List<ProximityAction> = emptyList()
)

@Serializable
data class ProximityAction(
    val distance: Double,
    val action: SensorAction
)

// ===== Enums =====

@Serializable
enum class ThermalTripType {
    ACTIVE,        // Active cooling trip
    PASSIVE,       // Passive cooling trip
    HOT,           // Hot trip point
    CRITICAL       // Critical trip point
}

@Serializable
enum class ThermalAction {
    NOTIFY,        // Send notification
    THROTTLE,      // Throttle CPU/GPU
    SHUTDOWN,      // Emergency shutdown
    SUSPEND,       // Suspend system
    HIBERNATE      // Hibernate system
}

@Serializable
enum class CoolingDeviceType {
    FAN,           // System fan
    LIQUID,        // Liquid cooling
    PROCESSOR,     // Processor throttling
    MEMORY,        // Memory throttling
    THERMAL_ZONE   // Thermal zone
}

@Serializable
enum class FanSpeed {
    AUTO,          // Automatic fan control
    SILENT,        // Silent mode
    NORMAL,        // Normal mode
    PERFORMANCE,   // Performance mode
    CUSTOM         // Custom fan curve
}

@Serializable
enum class AlarmType {
    HIGH,          // High temperature alarm
    LOW,           // Low temperature alarm
    CRITICAL       // Critical temperature alarm
}

@Serializable
enum class SensorAction {
    LOG,           // Log the event
    NOTIFY,        // Send notification
    EXECUTE,       // Execute command
    SHUTDOWN       // Shutdown system
}

@Serializable
enum class FanControl {
    AUTO,          // Automatic control
    MANUAL,        // Manual control
    PWM,           // PWM control
    DC             // DC control
}

@Serializable
enum class PowerUnit {
    WATTS,         // Watts
    MILLIWATTS,    // Milliwatts
    KILOWATTS      // Kilowatts
}