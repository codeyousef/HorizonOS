package org.horizonos.config.dsl.hardware

import kotlinx.serialization.Serializable
import kotlin.time.Duration
import kotlin.time.Duration.Companion.minutes

// ===== Power Configuration =====

@Serializable
data class PowerConfig(
    val profiles: List<PowerProfileConfig> = emptyList(),
    val cpu: CPUPowerConfig = CPUPowerConfig(),
    val gpu: GPUPowerConfig = GPUPowerConfig(),
    val battery: BatteryConfig = BatteryConfig(),
    val charging: ChargingConfig = ChargingConfig(),
    val suspend: SuspendConfig = SuspendConfig(),
    val hibernate: HibernateConfig = HibernateConfig()
)

@Serializable
data class PowerProfileConfig(
    val name: String,
    val active: Boolean = false,
    val cpuGovernor: CPUGovernor = CPUGovernor.POWERSAVE,
    val gpuProfile: PowerProfile = PowerProfile.BALANCED,
    val displayBrightness: Double = 0.8,
    val keyboardBacklight: Double = 0.5
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
    val optimizeCharging: Boolean = true,
    val chargeThreshold: Int = 80,
    val lowBatteryWarning: Int = 20,
    val criticalBatteryAction: BatteryAction = BatteryAction.SUSPEND,
    val powerButton: PowerButtonAction = PowerButtonAction.SUSPEND,
    val lidClose: LidCloseAction = LidCloseAction.SUSPEND
)

@Serializable
data class ChargingConfig(
    val fastCharging: Boolean = true,
    val smartCharging: Boolean = true,
    val thermalThrottling: Boolean = true,
    val maxChargingCurrent: Int? = null
)

@Serializable
data class SuspendConfig(
    val enabled: Boolean = true,
    val mode: SuspendMode = SuspendMode.SUSPEND_TO_RAM,
    val timeout: Duration = 30.minutes,
    val wakeOnLAN: Boolean = false,
    val wakeOnUSB: Boolean = true,
    val rtcWake: Boolean = true
)

@Serializable
data class HibernateConfig(
    val enabled: Boolean = true,
    val swapFile: String? = null,
    val compression: Boolean = true,
    val timeout: Duration = 60.minutes
)

// ===== Enums =====

@Serializable
enum class CPUGovernor {
    PERFORMANCE,   // Maximum performance
    POWERSAVE,     // Power saving
    USERSPACE,     // User-controlled frequency
    ONDEMAND,      // On-demand frequency scaling
    CONSERVATIVE,  // Conservative frequency scaling
    SCHEDUTIL      // Scheduler-guided scaling
}

@Serializable
enum class PStateDriver {
    INTEL_PSTATE,  // Intel P-State driver
    INTEL_CPUFREQ, // Intel CPU frequency driver
    ACPI_CPUFREQ,  // ACPI CPU frequency driver
    AMD_PSTATE     // AMD P-State driver
}

@Serializable
enum class BatteryAction {
    NOTHING,       // No action
    SUSPEND,       // Suspend to RAM
    HIBERNATE,     // Hibernate to disk
    SHUTDOWN,      // Shutdown system
    HYBRID_SLEEP   // Hybrid sleep mode
}

@Serializable
enum class PowerButtonAction {
    NOTHING,       // No action
    SUSPEND,       // Suspend system
    HIBERNATE,     // Hibernate system
    SHUTDOWN,      // Shutdown system
    ASK            // Ask user for action
}

@Serializable
enum class LidCloseAction {
    NOTHING,       // No action
    SUSPEND,       // Suspend system
    HIBERNATE,     // Hibernate system
    SHUTDOWN,      // Shutdown system
    LOCK           // Lock screen only
}

@Serializable
enum class SuspendMode {
    SUSPEND_TO_RAM,    // Suspend to RAM (S3)
    SUSPEND_TO_DISK,   // Suspend to disk (S4)
    HYBRID_SLEEP,      // Hybrid sleep (S3+S4)
    SUSPEND_TO_IDLE    // Suspend to idle (S0ix)
}