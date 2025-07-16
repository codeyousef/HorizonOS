package org.horizonos.config.dsl.hardware

import kotlinx.serialization.Serializable

// ===== GPU Configuration =====

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

// ===== Enums =====

@Serializable
enum class GPUDriver {
    AUTO_DETECT,   // Automatic driver detection
    NVIDIA,        // NVIDIA proprietary driver
    NOUVEAU,       // NVIDIA open source driver  
    AMD,           // AMD open source driver
    AMDGPU,        // AMD GPU driver
    RADEON,        // Legacy AMD Radeon driver
    INTEL,         // Intel graphics driver
    FBDEV,         // Framebuffer device driver
    VESA          // VESA driver
}

@Serializable
enum class MultiGPUMode {
    OPTIMUS,       // NVIDIA Optimus technology
    PRIME,         // PRIME render offloading
    CROSSFIRE,     // AMD CrossFire
    SLI,           // NVIDIA SLI
    MUXLESS,       // Muxless switching
    DISCRETE_ONLY  // Discrete GPU only
}

@Serializable
enum class SwitchingMethod {
    OPTIMUS,       // NVIDIA Optimus
    PRIME,         // PRIME switching
    BUMBLEBEE,     // Bumblebee project
    SYSTEM76_POWER,// System76 power management
    ENVYCONTROL,   // EnvyControl
    MANUAL         // Manual switching
}

@Serializable
enum class PowerProfile {
    PERFORMANCE,   // Maximum performance
    BALANCED,      // Balanced performance/power
    POWER_SAVER,   // Power saving mode
    ADAPTIVE,      // Adaptive based on load
    CUSTOM         // Custom profile
}

@Serializable
enum class OpenGLProfile {
    CORE,          // Core profile
    COMPATIBILITY, // Compatibility profile
    ES             // OpenGL ES profile
}