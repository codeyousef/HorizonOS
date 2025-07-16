package org.horizonos.config.dsl

import kotlinx.serialization.Serializable
import kotlin.time.Duration
import kotlin.time.Duration.Companion.seconds
import kotlin.time.Duration.Companion.minutes

/**
 * Hardware Configuration DSL for HorizonOS
 * 
 * Provides type-safe configuration for hardware components including
 * GPU drivers, input devices, displays, power management, and thermal control.
 */

// ===== Hardware Configuration =====

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

@Serializable
data class GPUConfig(
    val primary: GPUDriver = GPUDriver.AUTO_DETECT,
    val drivers: List<GPUDriverConfig> = emptyList(),
    val multiGPU: MultiGPUConfig? = null,
    val acceleration: HardwareAcceleration = HardwareAcceleration(),
    val vulkan: VulkanConfig = VulkanConfig(),
    val opengl: OpenGLConfig = OpenGLConfig(),
    val compute: ComputeConfig = ComputeConfig()
)

@Serializable
data class GPUDriverConfig(
    val type: GPUDriver,
    val enabled: Boolean = true,
    val options: Map<String, String> = emptyMap(),
    val firmwareFiles: List<String> = emptyList(),
    val blacklistedDrivers: List<String> = emptyList(),
    val powerManagement: GPUPowerManagement = GPUPowerManagement()
)

@Serializable
data class MultiGPUConfig(
    val mode: MultiGPUMode = MultiGPUMode.OPTIMUS,
    val primaryGPU: String? = null,
    val discreteGPU: String? = null,
    val switching: GPUSwitching = GPUSwitching(),
    val offloading: GPUOffloading = GPUOffloading()
)

@Serializable
data class GPUSwitching(
    val enabled: Boolean = true,
    val method: SwitchingMethod = SwitchingMethod.OPTIMUS,
    val runtime: Boolean = true,
    val logoutRequired: Boolean = false
)

@Serializable
data class GPUOffloading(
    val enabled: Boolean = true,
    val environmentVariables: Map<String, String> = emptyMap(),
    val applications: List<String> = emptyList()
)

@Serializable
data class GPUPowerManagement(
    val enabled: Boolean = true,
    val profile: PowerProfile = PowerProfile.ADAPTIVE,
    val maxPerformanceLevel: Int = 0, // 0 = auto
    val persistence: Boolean = true,
    val clockBoost: Boolean = true
)

@Serializable
data class HardwareAcceleration(
    val vaapi: Boolean = true,
    val vdpau: Boolean = true,
    val nvenc: Boolean = true,
    val nvdec: Boolean = true,
    val quickSync: Boolean = true,
    val openCL: Boolean = true,
    val cuda: Boolean = true
)

@Serializable
data class VulkanConfig(
    val enabled: Boolean = true,
    val layers: List<String> = emptyList(),
    val extensions: List<String> = emptyList(),
    val validation: Boolean = false,
    val debugUtils: Boolean = false
)

@Serializable
data class OpenGLConfig(
    val version: String = "4.6",
    val profile: OpenGLProfile = OpenGLProfile.CORE,
    val vsync: Boolean = true,
    val multisampling: Int = 4,
    val anisotropicFiltering: Int = 16
)

@Serializable
data class ComputeConfig(
    val cuda: CUDAConfig = CUDAConfig(),
    val openCL: OpenCLConfig = OpenCLConfig(),
    val rocm: ROCmConfig = ROCmConfig()
)

@Serializable
data class CUDAConfig(
    val enabled: Boolean = false,
    val version: String? = null,
    val toolkit: Boolean = false,
    val samples: Boolean = false,
    val persistentMode: Boolean = true
)

@Serializable
data class OpenCLConfig(
    val enabled: Boolean = true,
    val platforms: List<String> = emptyList(),
    val icdLoader: Boolean = true
)

@Serializable
data class ROCmConfig(
    val enabled: Boolean = false,
    val version: String? = null,
    val openCL: Boolean = true,
    val hip: Boolean = true
)

@Serializable
data class InputConfig(
    val keyboard: KeyboardConfig = KeyboardConfig(),
    val mouse: MouseConfig = MouseConfig(),
    val touchpad: TouchpadConfig = TouchpadConfig(),
    val touchscreen: TouchscreenConfig = TouchscreenConfig(),
    val gameController: GameControllerConfig = GameControllerConfig(),
    val accessibility: AccessibilityConfig = AccessibilityConfig()
)

@Serializable
data class KeyboardConfig(
    val layout: String = "us",
    val variant: String? = null,
    val model: String? = null,
    val options: List<String> = emptyList(),
    val repeatDelay: Duration = 500.seconds,
    val repeatRate: Int = 25,
    val numLock: Boolean = true,
    val capsLock: CapsLockBehavior = CapsLockBehavior.CAPS_LOCK
)

@Serializable
data class MouseConfig(
    val acceleration: Double = 1.0,
    val threshold: Double = 4.0,
    val leftHanded: Boolean = false,
    val middleButtonEmulation: Boolean = false,
    val scrollMethod: ScrollMethod = ScrollMethod.TWO_FINGER,
    val naturalScrolling: Boolean = false,
    val clickMethod: ClickMethod = ClickMethod.BUTTON_AREAS
)

@Serializable
data class TouchpadConfig(
    val enabled: Boolean = true,
    val tapToClick: Boolean = true,
    val twoFingerScroll: Boolean = true,
    val edgeScrolling: Boolean = false,
    val naturalScrolling: Boolean = true,
    val palmDetection: Boolean = true,
    val disableWhileTyping: Boolean = true,
    val acceleration: Double = 1.0,
    val sensitivity: Double = 1.0,
    val gestures: GestureConfig = GestureConfig()
)

@Serializable
data class GestureConfig(
    val enabled: Boolean = true,
    val threeFingerSwipe: Boolean = true,
    val fourFingerSwipe: Boolean = true,
    val pinchToZoom: Boolean = true,
    val rotateGesture: Boolean = false,
    val customGestures: List<CustomGesture> = emptyList()
)

@Serializable
data class CustomGesture(
    val name: String,
    val fingers: Int,
    val direction: GestureDirection,
    val action: String
)

@Serializable
data class TouchscreenConfig(
    val enabled: Boolean = true,
    val calibration: TouchCalibration? = null,
    val rotation: ScreenRotation = ScreenRotation.NORMAL,
    val multitouch: Boolean = true,
    val gestures: Boolean = true
)

@Serializable
data class TouchCalibration(
    val matrix: List<Double> = emptyList(),
    val tool: CalibrationTool = CalibrationTool.XINPUT_CALIBRATOR
)

@Serializable
data class GameControllerConfig(
    val enabled: Boolean = true,
    val xboxSupport: Boolean = true,
    val playstationSupport: Boolean = true,
    val steamController: Boolean = true,
    val customMappings: List<ControllerMapping> = emptyList(),
    val rumble: Boolean = true
)

@Serializable
data class ControllerMapping(
    val name: String,
    val vendorId: String,
    val productId: String,
    val mapping: String
)

@Serializable
data class AccessibilityConfig(
    val enabled: Boolean = false,
    val highContrast: Boolean = false,
    val largeText: Boolean = false,
    val stickyKeys: Boolean = false,
    val slowKeys: Boolean = false,
    val bounceKeys: Boolean = false,
    val mouseKeys: Boolean = false,
    val onScreenKeyboard: Boolean = false,
    val screenReader: Boolean = false
)

@Serializable
data class DisplayConfig(
    val monitors: List<MonitorConfig> = emptyList(),
    val layout: DisplayLayout = DisplayLayout.SINGLE,
    val scaling: DisplayScaling = DisplayScaling(),
    val color: ColorManagement = ColorManagement(),
    val nightLight: NightLightConfig = NightLightConfig(),
    val dpms: DPMSConfig = DPMSConfig()
)

@Serializable
data class MonitorConfig(
    val name: String,
    val enabled: Boolean = true,
    val primary: Boolean = false,
    val resolution: Resolution? = null,
    val refreshRate: Double? = null,
    val position: Position = Position(0, 0),
    val rotation: ScreenRotation = ScreenRotation.NORMAL,
    val scale: Double = 1.0,
    val colorProfile: String? = null,
    val brightness: Double = 1.0,
    val gamma: GammaConfig = GammaConfig()
)

@Serializable
data class Resolution(
    val width: Int,
    val height: Int
)

@Serializable
data class Position(
    val x: Int,
    val y: Int
)

@Serializable
data class GammaConfig(
    val red: Double = 1.0,
    val green: Double = 1.0,
    val blue: Double = 1.0
)

@Serializable
data class DisplayScaling(
    val mode: ScalingMode = ScalingMode.AUTO,
    val factor: Double = 1.0,
    val perMonitor: Boolean = true,
    val fractionalScaling: Boolean = true
)

@Serializable
data class ColorManagement(
    val enabled: Boolean = true,
    val defaultProfile: String? = null,
    val profiles: List<ColorProfile> = emptyList(),
    val adaptation: ColorAdaptation = ColorAdaptation.BRADFORD
)

@Serializable
data class ColorProfile(
    val name: String,
    val path: String,
    val description: String? = null
)

@Serializable
data class NightLightConfig(
    val enabled: Boolean = true,
    val temperature: Int = 4000, // Kelvin
    val schedule: NightLightSchedule = NightLightSchedule.AUTOMATIC,
    val manualStart: String = "20:00",
    val manualEnd: String = "06:00",
    val transition: Duration = 30.minutes
)

@Serializable
data class DPMSConfig(
    val enabled: Boolean = true,
    val standbyTime: Duration = 10.minutes,
    val suspendTime: Duration = 15.minutes,
    val offTime: Duration = 20.minutes
)

@Serializable
data class PowerConfig(
    val profiles: List<PowerProfile> = emptyList(),
    val cpu: CPUPowerConfig = CPUPowerConfig(),
    val gpu: GPUPowerConfig = GPUPowerConfig(),
    val battery: BatteryConfig = BatteryConfig(),
    val charging: ChargingConfig = ChargingConfig(),
    val suspend: SuspendConfig = SuspendConfig(),
    val hibernate: HibernateConfig = HibernateConfig()
)

@Serializable
data class CPUPowerConfig(
    val governor: CPUGovernor = CPUGovernor.POWERSAVE,
    val scalingDriver: String? = null,
    val minFreq: String? = null,
    val maxFreq: String? = null,
    val boostEnabled: Boolean = true,
    val turboEnabled: Boolean = true,
    val cStates: CStateConfig = CStateConfig(),
    val pStates: PStateConfig = PStateConfig()
)

@Serializable
data class CStateConfig(
    val enabled: Boolean = true,
    val maxLatency: Duration? = null,
    val disabledStates: List<Int> = emptyList()
)

@Serializable
data class PStateConfig(
    val enabled: Boolean = true,
    val driver: PStateDriver = PStateDriver.INTEL_PSTATE,
    val minPerf: Int = 0,
    val maxPerf: Int = 100,
    val noTurbo: Boolean = false
)

@Serializable
data class GPUPowerConfig(
    val profile: PowerProfile = PowerProfile.ADAPTIVE,
    val dynamicSwitching: Boolean = true,
    val powerLimit: Int? = null,
    val fanCurve: FanCurve? = null
)

@Serializable
data class FanCurve(
    val points: List<FanPoint> = emptyList(),
    val hysteresis: Int = 5
)

@Serializable
data class FanPoint(
    val temperature: Int,
    val fanSpeed: Int
)

@Serializable
data class BatteryConfig(
    val chargingThreshold: ChargingThreshold = ChargingThreshold(),
    val calibration: Boolean = false,
    val conservation: Boolean = false,
    val notifications: BatteryNotifications = BatteryNotifications()
)

@Serializable
data class ChargingThreshold(
    val enabled: Boolean = false,
    val startThreshold: Int = 20,
    val stopThreshold: Int = 80
)

@Serializable
data class BatteryNotifications(
    val lowBattery: Int = 20,
    val criticalBattery: Int = 10,
    val fullCharge: Boolean = true,
    val unplugged: Boolean = false
)

@Serializable
data class ChargingConfig(
    val fastCharging: Boolean = true,
    val adaptiveCharging: Boolean = true,
    val chargingLimit: Int = 100,
    val schedule: ChargingSchedule? = null
)

@Serializable
data class ChargingSchedule(
    val enabled: Boolean = false,
    val startTime: String = "22:00",
    val endTime: String = "06:00",
    val weekdaysOnly: Boolean = true
)

@Serializable
data class SuspendConfig(
    val enabled: Boolean = true,
    val method: SuspendMethod = SuspendMethod.SUSPEND_TO_RAM,
    val timeout: Duration = 30.minutes,
    val wakeOnLAN: Boolean = false,
    val wakeOnUSB: Boolean = true,
    val rtcWake: Boolean = true
)

@Serializable
data class HibernateConfig(
    val enabled: Boolean = true,
    val swapFile: String? = null,
    val swapPartition: String? = null,
    val compression: Boolean = true,
    val timeout: Duration = 2.minutes
)

@Serializable
data class ThermalConfig(
    val enabled: Boolean = true,
    val zones: List<ThermalZone> = emptyList(),
    val policies: List<ThermalPolicy> = emptyList(),
    val monitoring: ThermalMonitoring = ThermalMonitoring()
)

@Serializable
data class ThermalZone(
    val name: String,
    val type: String,
    val criticalTemp: Int = 100,
    val hotTemp: Int = 85,
    val passiveTemp: Int = 70,
    val coolingDevices: List<String> = emptyList()
)

@Serializable
data class ThermalPolicy(
    val name: String,
    val governor: ThermalGovernor = ThermalGovernor.STEP_WISE,
    val activeThreshold: Int = 60,
    val passiveThreshold: Int = 80
)

@Serializable
data class ThermalMonitoring(
    val enabled: Boolean = true,
    val interval: Duration = 5.seconds,
    val logTemperatures: Boolean = false,
    val alertThreshold: Int = 85
)

@Serializable
data class AudioConfig(
    val enabled: Boolean = true,
    val system: AudioSystem = AudioSystem.PIPEWIRE,
    val devices: List<AudioDevice> = emptyList(),
    val defaultSink: String? = null,
    val defaultSource: String? = null,
    val volume: VolumeConfig = VolumeConfig(),
    val bluetooth: BluetoothAudioConfig = BluetoothAudioConfig()
)

@Serializable
data class AudioDevice(
    val name: String,
    val type: AudioDeviceType,
    val enabled: Boolean = true,
    val channels: Int = 2,
    val sampleRate: Int = 48000,
    val bitDepth: Int = 16,
    val bufferSize: Int = 1024
)

@Serializable
data class VolumeConfig(
    val master: Int = 70,
    val overAmplification: Boolean = false,
    val maxVolume: Int = 100,
    val stepSize: Int = 5
)

@Serializable
data class BluetoothAudioConfig(
    val enabled: Boolean = true,
    val codec: BluetoothCodec = BluetoothCodec.SBC,
    val highQuality: Boolean = true,
    val lowLatency: Boolean = false
)

@Serializable
data class StorageHardwareConfig(
    val nvme: NVMeConfig = NVMeConfig(),
    val sata: SATAConfig = SATAConfig(),
    val usb: USBStorageConfig = USBStorageConfig(),
    val raid: RAIDConfig = RAIDConfig()
)

@Serializable
data class NVMeConfig(
    val enabled: Boolean = true,
    val powerManagement: Boolean = true,
    val temperatureMonitoring: Boolean = true,
    val namespace: NVMeNamespace = NVMeNamespace()
)

@Serializable
data class NVMeNamespace(
    val multipath: Boolean = false,
    val ioScheduler: IOScheduler = IOScheduler.NONE,
    val queueDepth: Int = 32
)

@Serializable
data class SATAConfig(
    val enabled: Boolean = true,
    val linkPowerManagement: SATALinkPM = SATALinkPM.MEDIUM_POWER,
    val hotplug: Boolean = true,
    val ncq: Boolean = true
)

@Serializable
data class USBStorageConfig(
    val enabled: Boolean = true,
    val autoMount: Boolean = true,
    val quirks: List<String> = emptyList()
)

@Serializable
data class RAIDConfig(
    val enabled: Boolean = false,
    val monitoring: Boolean = true,
    val notifications: Boolean = true,
    val checkInterval: Duration = 24.minutes
)

@Serializable
data class NetworkHardwareConfig(
    val ethernet: EthernetConfig = EthernetConfig(),
    val wireless: WirelessHardwareConfig = WirelessHardwareConfig(),
    val powerManagement: NetworkPowerConfig = NetworkPowerConfig()
)

@Serializable
data class EthernetConfig(
    val enabled: Boolean = true,
    val wakeOnLAN: Boolean = false,
    val linkSpeed: LinkSpeed = LinkSpeed.AUTO,
    val flowControl: Boolean = true
)

@Serializable
data class WirelessHardwareConfig(
    val enabled: Boolean = true,
    val powerSave: Boolean = true,
    val regulatory: String = "00", // World regulatory domain
    val scanning: WirelessScanning = WirelessScanning()
)

@Serializable
data class WirelessScanning(
    val backgroundScan: Boolean = true,
    val intervalActive: Duration = 10.seconds,
    val intervalSuspend: Duration = 60.seconds
)

@Serializable
data class NetworkPowerConfig(
    val enabled: Boolean = true,
    val wakeOnWLAN: Boolean = false,
    val runtimePM: Boolean = true
)

@Serializable
data class USBConfig(
    val enabled: Boolean = true,
    val autosuspend: USBAutosuspend = USBAutosuspend(),
    val devices: List<USBDeviceConfig> = emptyList(),
    val authorization: USBAuthorization = USBAuthorization()
)

@Serializable
data class USBAutosuspend(
    val enabled: Boolean = true,
    val delay: Duration = 2.seconds,
    val blacklist: List<String> = emptyList()
)

@Serializable
data class USBDeviceConfig(
    val vendorId: String,
    val productId: String,
    val enabled: Boolean = true,
    val autosuspend: Boolean = true,
    val persistentNaming: Boolean = false
)

@Serializable
data class USBAuthorization(
    val mode: USBAuthMode = USBAuthMode.NONE,
    val whitelist: List<String> = emptyList()
)

@Serializable
data class BluetoothConfig(
    val enabled: Boolean = true,
    val powerManagement: Boolean = true,
    val fastConnectable: Boolean = true,
    val privacy: BluetoothPrivacy = BluetoothPrivacy.DEVICE,
    val experimental: Boolean = false,
    val devices: List<BluetoothDeviceConfig> = emptyList()
)

@Serializable
data class BluetoothDeviceConfig(
    val address: String,
    val name: String? = null,
    val trusted: Boolean = false,
    val autoConnect: Boolean = true
)

@Serializable
data class SensorConfig(
    val enabled: Boolean = true,
    val temperature: TemperatureSensors = TemperatureSensors(),
    val accelerometer: AccelerometerConfig = AccelerometerConfig(),
    val gyroscope: GyroscopeConfig = GyroscopeConfig(),
    val magnetometer: MagnetometerConfig = MagnetometerConfig(),
    val ambientLight: AmbientLightConfig = AmbientLightConfig()
)

@Serializable
data class TemperatureSensors(
    val enabled: Boolean = true,
    val cpu: Boolean = true,
    val gpu: Boolean = true,
    val motherboard: Boolean = true,
    val drives: Boolean = true
)

@Serializable
data class AccelerometerConfig(
    val enabled: Boolean = true,
    val autoRotate: Boolean = true,
    val threshold: Double = 0.5
)

@Serializable
data class GyroscopeConfig(
    val enabled: Boolean = true,
    val calibration: Boolean = true
)

@Serializable
data class MagnetometerConfig(
    val enabled: Boolean = true,
    val calibration: Boolean = true
)

@Serializable
data class AmbientLightConfig(
    val enabled: Boolean = true,
    val autoBrightness: Boolean = true,
    val threshold: Int = 100
)

// ===== Enums =====

@Serializable
enum class GPUDriver {
    AUTO_DETECT,
    NVIDIA_PROPRIETARY,
    NVIDIA_OPEN,
    NOUVEAU,
    AMD_AMDGPU,
    AMD_RADEON,
    INTEL,
    MESA
}

@Serializable
enum class MultiGPUMode {
    DISABLED,
    OPTIMUS,
    PRIME,
    CROSSFIRE,
    SLI,
    HYBRID
}

@Serializable
enum class SwitchingMethod {
    OPTIMUS,
    PRIME,
    BBSWITCH,
    ACPI_CALL,
    MANUAL
}

@Serializable
enum class PowerProfile {
    POWER_SAVE,
    BALANCED,
    PERFORMANCE,
    ADAPTIVE,
    CUSTOM
}

@Serializable
enum class OpenGLProfile {
    CORE,
    COMPATIBILITY,
    ES
}

@Serializable
enum class CapsLockBehavior {
    CAPS_LOCK,
    CTRL,
    ESC,
    DISABLED
}

@Serializable
enum class ScrollMethod {
    TWO_FINGER,
    EDGE,
    BUTTON,
    DISABLED
}

@Serializable
enum class ClickMethod {
    BUTTON_AREAS,
    CLICK_FINGER,
    DISABLED
}

@Serializable
enum class GestureDirection {
    UP,
    DOWN,
    LEFT,
    RIGHT,
    PINCH_IN,
    PINCH_OUT,
    ROTATE_CW,
    ROTATE_CCW
}

@Serializable
enum class CalibrationTool {
    XINPUT_CALIBRATOR,
    EVDEV_CALIBRATOR,
    LIBINPUT_CALIBRATOR
}

@Serializable
enum class ScreenRotation {
    NORMAL,
    LEFT,
    RIGHT,
    INVERTED
}

@Serializable
enum class DisplayLayout {
    SINGLE,
    EXTENDED,
    MIRRORED,
    CUSTOM
}

@Serializable
enum class ScalingMode {
    AUTO,
    MANUAL,
    INTEGER,
    FRACTIONAL
}

@Serializable
enum class ColorAdaptation {
    BRADFORD,
    VON_KRIES,
    XYZ_SCALING
}

@Serializable
enum class NightLightSchedule {
    AUTOMATIC,
    MANUAL,
    DISABLED
}

@Serializable
enum class CPUGovernor {
    PERFORMANCE,
    POWERSAVE,
    USERSPACE,
    ONDEMAND,
    CONSERVATIVE,
    SCHEDUTIL
}

@Serializable
enum class PStateDriver {
    INTEL_PSTATE,
    INTEL_CPUFREQ,
    ACPI_CPUFREQ,
    AMD_PSTATE
}

@Serializable
enum class SuspendMethod {
    SUSPEND_TO_RAM,
    SUSPEND_TO_DISK,
    HYBRID_SUSPEND
}

@Serializable
enum class ThermalGovernor {
    STEP_WISE,
    FAIR_SHARE,
    BANG_BANG,
    PID,
    USER_SPACE
}

@Serializable
enum class AudioSystem {
    PIPEWIRE,
    PULSEAUDIO,
    ALSA,
    JACK
}

@Serializable
enum class AudioDeviceType {
    PLAYBACK,
    CAPTURE,
    DUPLEX
}

@Serializable
enum class BluetoothCodec {
    SBC,
    AAC,
    APTX,
    APTX_HD,
    LDAC,
    LC3
}

@Serializable
enum class IOScheduler {
    NONE,
    MQ_DEADLINE,
    KYBER,
    BFQ
}

@Serializable
enum class SATALinkPM {
    MAX_PERFORMANCE,
    MEDIUM_POWER,
    MEDIUM_POWER_WITH_DIPM,
    MIN_POWER
}

@Serializable
enum class LinkSpeed {
    AUTO,
    SPEED_10M,
    SPEED_100M,
    SPEED_1G,
    SPEED_2_5G,
    SPEED_5G,
    SPEED_10G
}

@Serializable
enum class USBAuthMode {
    NONE,
    ALL,
    INTERNAL,
    WHITELIST
}

@Serializable
enum class BluetoothPrivacy {
    DISABLED,
    DEVICE,
    NETWORK
}

// ===== DSL Builders =====

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
    
    fun input(block: InputContext.() -> Unit) {
        input = InputContext().apply(block).toConfig()
    }
    
    fun display(block: DisplayContext.() -> Unit) {
        display = DisplayContext().apply(block).toConfig()
    }
    
    fun power(block: PowerContext.() -> Unit) {
        power = PowerContext().apply(block).toConfig()
    }
    
    fun thermal(block: ThermalContext.() -> Unit) {
        thermal = ThermalContext().apply(block).toConfig()
    }
    
    fun audio(block: AudioContext.() -> Unit) {
        audio = AudioContext().apply(block).toConfig()
    }
    
    fun storage(block: StorageHardwareContext.() -> Unit) {
        storage = StorageHardwareContext().apply(block).toConfig()
    }
    
    fun networking(block: NetworkHardwareContext.() -> Unit) {
        networking = NetworkHardwareContext().apply(block).toConfig()
    }
    
    fun usb(block: USBContext.() -> Unit) {
        usb = USBContext().apply(block).toConfig()
    }
    
    fun bluetooth(block: BluetoothContext.() -> Unit) {
        bluetooth = BluetoothContext().apply(block).toConfig()
    }
    
    fun sensors(block: SensorContext.() -> Unit) {
        sensors = SensorContext().apply(block).toConfig()
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
    private val drivers = mutableListOf<GPUDriverConfig>()
    private var multiGPU: MultiGPUConfig? = null
    private var acceleration = HardwareAcceleration()
    private var vulkan = VulkanConfig()
    private var opengl = OpenGLConfig()
    private var compute = ComputeConfig()
    
    fun driver(type: GPUDriver, block: GPUDriverContext.() -> Unit = {}) {
        val context = GPUDriverContext().apply {
            this.type = type
            block()
        }
        drivers.add(context.toConfig())
    }
    
    fun nvidia(block: GPUDriverContext.() -> Unit = {}) {
        driver(GPUDriver.NVIDIA_PROPRIETARY, block)
    }
    
    fun amd(block: GPUDriverContext.() -> Unit = {}) {
        driver(GPUDriver.AMD_AMDGPU, block)
    }
    
    fun intel(block: GPUDriverContext.() -> Unit = {}) {
        driver(GPUDriver.INTEL, block)
    }
    
    fun multiGPU(block: MultiGPUContext.() -> Unit) {
        multiGPU = MultiGPUContext().apply(block).toConfig()
    }
    
    fun acceleration(block: HardwareAccelerationContext.() -> Unit) {
        acceleration = HardwareAccelerationContext().apply(block).toConfig()
    }
    
    fun vulkan(block: VulkanContext.() -> Unit) {
        vulkan = VulkanContext().apply(block).toConfig()
    }
    
    fun opengl(block: OpenGLContext.() -> Unit) {
        opengl = OpenGLContext().apply(block).toConfig()
    }
    
    fun compute(block: ComputeContext.() -> Unit) {
        compute = ComputeContext().apply(block).toConfig()
    }
    
    fun toConfig() = GPUConfig(
        primary = primary,
        drivers = drivers,
        multiGPU = multiGPU,
        acceleration = acceleration,
        vulkan = vulkan,
        opengl = opengl,
        compute = compute
    )
}

@HorizonOSDsl
class GPUDriverContext {
    var type: GPUDriver = GPUDriver.AUTO_DETECT
    var enabled: Boolean = true
    private val options = mutableMapOf<String, String>()
    private val firmwareFiles = mutableListOf<String>()
    private val blacklistedDrivers = mutableListOf<String>()
    private var powerManagement = GPUPowerManagement()
    
    fun option(key: String, value: String) {
        options[key] = value
    }
    
    fun firmware(vararg files: String) {
        firmwareFiles.addAll(files)
    }
    
    fun blacklist(vararg drivers: String) {
        blacklistedDrivers.addAll(drivers)
    }
    
    fun powerManagement(block: GPUPowerManagementContext.() -> Unit) {
        powerManagement = GPUPowerManagementContext().apply(block).toConfig()
    }
    
    fun toConfig() = GPUDriverConfig(
        type = type,
        enabled = enabled,
        options = options,
        firmwareFiles = firmwareFiles,
        blacklistedDrivers = blacklistedDrivers,
        powerManagement = powerManagement
    )
}

// Additional context classes would continue here...
// For brevity, I'll implement the key ones and provide the structure

@HorizonOSDsl
class MultiGPUContext {
    var mode: MultiGPUMode = MultiGPUMode.OPTIMUS
    var primaryGPU: String? = null
    var discreteGPU: String? = null
    private var switching = GPUSwitching()
    private var offloading = GPUOffloading()
    
    fun switching(block: GPUSwitchingContext.() -> Unit) {
        switching = GPUSwitchingContext().apply(block).toConfig()
    }
    
    fun offloading(block: GPUOffloadingContext.() -> Unit) {
        offloading = GPUOffloadingContext().apply(block).toConfig()
    }
    
    fun toConfig() = MultiGPUConfig(
        mode = mode,
        primaryGPU = primaryGPU,
        discreteGPU = discreteGPU,
        switching = switching,
        offloading = offloading
    )
}

// Placeholder context classes for structure
@HorizonOSDsl class GPUPowerManagementContext { fun toConfig() = GPUPowerManagement() }
@HorizonOSDsl class GPUSwitchingContext { fun toConfig() = GPUSwitching() }
@HorizonOSDsl class GPUOffloadingContext { fun toConfig() = GPUOffloading() }
@HorizonOSDsl class HardwareAccelerationContext { fun toConfig() = HardwareAcceleration() }
@HorizonOSDsl class VulkanContext { fun toConfig() = VulkanConfig() }
@HorizonOSDsl class OpenGLContext { fun toConfig() = OpenGLConfig() }
@HorizonOSDsl class ComputeContext { fun toConfig() = ComputeConfig() }
@HorizonOSDsl class InputContext { fun toConfig() = InputConfig() }
@HorizonOSDsl class DisplayContext { fun toConfig() = DisplayConfig() }
@HorizonOSDsl class PowerContext { fun toConfig() = PowerConfig() }
@HorizonOSDsl class ThermalContext { fun toConfig() = ThermalConfig() }
@HorizonOSDsl class AudioContext { fun toConfig() = AudioConfig() }
@HorizonOSDsl class StorageHardwareContext { fun toConfig() = StorageHardwareConfig() }
@HorizonOSDsl class NetworkHardwareContext { fun toConfig() = NetworkHardwareConfig() }
@HorizonOSDsl class USBContext { fun toConfig() = USBConfig() }
@HorizonOSDsl class BluetoothContext { fun toConfig() = BluetoothConfig() }
@HorizonOSDsl class SensorContext { fun toConfig() = SensorConfig() }

// ===== Extension Functions =====

fun CompiledConfig.hasHardware(): Boolean = hardware != null

fun CompiledConfig.getGPUDriver(type: GPUDriver): GPUDriverConfig? = 
    hardware?.gpu?.drivers?.find { it.type == type }

fun CompiledConfig.getMonitor(name: String): MonitorConfig? = 
    hardware?.display?.monitors?.find { it.name == name }

fun CompiledConfig.hasMultiGPU(): Boolean = 
    hardware?.gpu?.multiGPU != null

fun CompiledConfig.getPowerProfile(): PowerProfile? = 
    hardware?.power?.cpu?.governor?.let { PowerProfile.valueOf(it.name) }