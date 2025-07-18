//! Hardware detection and model selection for optimal AI performance

use crate::{AIError, HardwareOptimization};
use serde::{Deserialize, Serialize};
use sysinfo::System;
use std::process::Command;
use std::fs;
use std::time::{Duration, Instant};
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use std::sync::Arc;

/// Hardware profile of the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareProfile {
    /// CPU information
    pub cpu: CPUInfo,
    /// GPU information
    pub gpu: GPUInfo,
    /// Memory information
    pub memory: MemoryInfo,
    /// Storage information
    pub storage: StorageInfo,
    /// Battery information (if applicable)
    pub battery: Option<BatteryInfo>,
    /// Thermal information
    pub thermal: ThermalInfo,
    /// Network information
    pub network: NetworkInfo,
    /// System uptime
    pub uptime: Duration,
    /// Detection timestamp
    pub detected_at: DateTime<Utc>,
    /// Performance capabilities
    pub capabilities: SystemCapabilities,
}

impl Default for HardwareProfile {
    fn default() -> Self {
        Self {
            cpu: CPUInfo::default(),
            gpu: GPUInfo::default(),
            memory: MemoryInfo::default(),
            storage: StorageInfo::default(),
            battery: None,
            thermal: ThermalInfo::default(),
            network: NetworkInfo::default(),
            uptime: Duration::from_secs(0),
            detected_at: Utc::now(),
            capabilities: SystemCapabilities::default(),
        }
    }
}

/// CPU information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CPUInfo {
    /// CPU brand and model
    pub brand: String,
    /// Number of physical cores
    pub physical_cores: u32,
    /// Number of logical cores (threads)
    pub logical_cores: u32,
    /// CPU frequency in MHz
    pub frequency: u64,
    /// Current CPU usage percentage
    pub usage: f32,
}

impl Default for CPUInfo {
    fn default() -> Self {
        Self {
            brand: "Unknown".to_string(),
            physical_cores: 1,
            logical_cores: 1,
            frequency: 0,
            usage: 0.0,
        }
    }
}

/// GPU information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GPUInfo {
    /// GPU vendor
    pub vendor: String,
    /// GPU model
    pub model: String,
    /// VRAM in MB
    pub vram_total: Option<u64>,
    /// Available VRAM in MB
    pub vram_free: Option<u64>,
    /// GPU utilization percentage
    pub utilization: f32,
    /// GPU temperature in Celsius
    pub temperature: Option<f32>,
    /// Power consumption in watts
    pub power_consumption: Option<f32>,
    /// Whether CUDA is available
    pub cuda_available: bool,
    /// CUDA version if available
    pub cuda_version: Option<String>,
    /// Whether ROCm is available
    pub rocm_available: bool,
    /// ROCm version if available
    pub rocm_version: Option<String>,
    /// Whether Vulkan is available
    pub vulkan_available: bool,
    /// Whether OpenCL is available
    pub opencl_available: bool,
    /// Compute capability (for CUDA)
    pub compute_capability: Option<String>,
    /// Driver version
    pub driver_version: Option<String>,
}

impl Default for GPUInfo {
    fn default() -> Self {
        Self {
            vendor: "Unknown".to_string(),
            model: "Unknown".to_string(),
            vram_total: None,
            vram_free: None,
            utilization: 0.0,
            temperature: None,
            power_consumption: None,
            cuda_available: false,
            cuda_version: None,
            rocm_available: false,
            rocm_version: None,
            vulkan_available: false,
            opencl_available: false,
            compute_capability: None,
            driver_version: None,
        }
    }
}

/// Memory information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryInfo {
    /// Total RAM in MB
    pub total: u64,
    /// Free RAM in MB
    pub free: u64,
    /// Available RAM in MB (free + cache that can be released)
    pub available: u64,
}

impl Default for MemoryInfo {
    fn default() -> Self {
        Self {
            total: 8192,
            free: 4096,
            available: 4096,
        }
    }
}

/// Storage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageInfo {
    /// Total storage in GB
    pub total: u64,
    /// Free storage in GB
    pub free: u64,
}

impl Default for StorageInfo {
    fn default() -> Self {
        Self {
            total: 100,
            free: 50,
        }
    }
}

/// Battery information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryInfo {
    /// Battery level percentage
    pub level: u8,
    /// Whether the device is charging
    pub charging: bool,
    /// Whether running on battery power
    pub on_battery: bool,
    /// Time remaining until empty (in minutes)
    pub time_to_empty: Option<u32>,
    /// Time remaining until full (in minutes)
    pub time_to_full: Option<u32>,
    /// Battery health percentage
    pub health: Option<u8>,
    /// Power consumption in watts
    pub power_consumption: Option<f32>,
}

/// Thermal information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThermalInfo {
    /// CPU temperature in Celsius
    pub cpu_temperature: Option<f32>,
    /// GPU temperature in Celsius
    pub gpu_temperature: Option<f32>,
    /// System temperature in Celsius
    pub system_temperature: Option<f32>,
    /// Whether thermal throttling is active
    pub thermal_throttling: bool,
    /// Fan speeds (RPM)
    pub fan_speeds: Vec<u32>,
}

impl Default for ThermalInfo {
    fn default() -> Self {
        Self {
            cpu_temperature: None,
            gpu_temperature: None,
            system_temperature: None,
            thermal_throttling: false,
            fan_speeds: Vec::new(),
        }
    }
}

/// Network information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfo {
    /// Whether connected to internet
    pub connected: bool,
    /// Connection type (ethernet, wifi, cellular)
    pub connection_type: String,
    /// Network speed in Mbps
    pub speed: Option<u32>,
    /// Network latency in milliseconds
    pub latency: Option<u32>,
}

impl Default for NetworkInfo {
    fn default() -> Self {
        Self {
            connected: false,
            connection_type: "unknown".to_string(),
            speed: None,
            latency: None,
        }
    }
}

/// System capabilities for AI workloads
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemCapabilities {
    /// Recommended concurrent AI sessions
    pub max_concurrent_sessions: u32,
    /// Maximum model size that can be loaded
    pub max_model_size: ModelSize,
    /// Estimated inference speed (tokens/second)
    pub estimated_inference_speed: f32,
    /// Memory bandwidth in GB/s
    pub memory_bandwidth: Option<f32>,
    /// AI acceleration support level
    pub ai_acceleration: AccelerationSupport,
    /// Virtualization support
    pub virtualization_support: bool,
}

impl Default for SystemCapabilities {
    fn default() -> Self {
        Self {
            max_concurrent_sessions: 1,
            max_model_size: ModelSize::Tiny,
            estimated_inference_speed: 2.0,
            memory_bandwidth: None,
            ai_acceleration: AccelerationSupport::None,
            virtualization_support: false,
        }
    }
}

/// AI acceleration support level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccelerationSupport {
    /// No acceleration support
    None,
    /// Basic CPU optimization
    CPUOptimized,
    /// Integrated GPU acceleration
    IntegratedGPU,
    /// Dedicated GPU acceleration
    DedicatedGPU,
    /// High-end GPU with tensor cores
    TensorCores,
}

/// Available AI models
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ModelSize {
    /// Tiny model (1-3B parameters)
    Tiny,
    /// Small model (7B parameters)
    Small,
    /// Medium model (13B parameters)
    Medium,
    /// Large model (30-34B parameters)
    Large,
    /// Extra large model (70B+ parameters)
    ExtraLarge,
}

impl ModelSize {
    /// Get the recommended model name for Ollama
    pub fn model_name(&self) -> &'static str {
        match self {
            ModelSize::Tiny => "tinyllama:latest",
            ModelSize::Small => "llama3.2:latest",
            ModelSize::Medium => "llama3.2:13b",
            ModelSize::Large => "llama3.2:34b",
            ModelSize::ExtraLarge => "llama3.2:70b",
        }
    }

    /// Get approximate VRAM requirement in MB
    pub fn vram_requirement(&self) -> u64 {
        match self {
            ModelSize::Tiny => 2_000,      // 2GB
            ModelSize::Small => 4_000,     // 4GB
            ModelSize::Medium => 8_000,    // 8GB
            ModelSize::Large => 20_000,    // 20GB
            ModelSize::ExtraLarge => 40_000, // 40GB
        }
    }

    /// Get approximate RAM requirement in MB (for CPU inference)
    pub fn ram_requirement(&self) -> u64 {
        match self {
            ModelSize::Tiny => 4_000,      // 4GB
            ModelSize::Small => 8_000,     // 8GB
            ModelSize::Medium => 16_000,   // 16GB
            ModelSize::Large => 32_000,    // 32GB
            ModelSize::ExtraLarge => 64_000, // 64GB
        }
    }
}

/// Detect the current hardware profile
pub fn detect_hardware_profile() -> Result<HardwareProfile, AIError> {
    let mut sys = System::new_all();
    sys.refresh_all();

    // CPU information
    let cpu = {
        let cpus = sys.cpus();
        let first_cpu = cpus.first();
        CPUInfo {
            brand: first_cpu.map(|c| c.brand().to_string()).unwrap_or_else(|| "Unknown".to_string()),
            physical_cores: sys.physical_core_count().unwrap_or(1) as u32,
            logical_cores: cpus.len() as u32,
            frequency: first_cpu.map(|c| c.frequency()).unwrap_or(0),
            usage: first_cpu.map(|c| c.cpu_usage()).unwrap_or(0.0),
        }
    };

    // GPU information
    let gpu = detect_gpu_info()?;

    // Memory information
    let memory = MemoryInfo {
        total: sys.total_memory() / 1024, // Convert to MB
        free: sys.free_memory() / 1024,
        available: sys.available_memory() / 1024,
    };

    // Storage information
    let storage = detect_storage_info()?;

    // Battery information
    let battery = detect_battery_info();

    // Thermal information
    let thermal = detect_thermal_info();
    
    // Network information
    let network = detect_network_info();
    
    // System uptime
    let uptime = System::uptime();
    
    // System capabilities
    let capabilities = analyze_system_capabilities(&cpu, &gpu, &memory);
    
    Ok(HardwareProfile {
        cpu,
        gpu,
        memory,
        storage,
        battery,
        thermal,
        network,
        uptime: Duration::from_secs(uptime),
        detected_at: Utc::now(),
        capabilities,
    })
}

/// Detect GPU information
fn detect_gpu_info() -> Result<GPUInfo, AIError> {
    // Try nvidia-smi first
    if let Ok(output) = Command::new("nvidia-smi")
        .args(&["--query-gpu=name,memory.total,memory.free", "--format=csv,noheader,nounits"])
        .output()
    {
        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            let parts: Vec<&str> = output_str.trim().split(',').collect();
            
            if parts.len() >= 3 {
                let vram_total = parts[1].trim().parse().unwrap_or(0);
                let vram_free = parts[2].trim().parse().unwrap_or(0);
                
                return Ok(GPUInfo {
                    vendor: "NVIDIA".to_string(),
                    model: parts[0].trim().to_string(),
                    vram_total: Some(vram_total),
                    vram_free: Some(vram_free),
                    utilization: 0.0,
                    temperature: None,
                    power_consumption: None,
                    cuda_available: true,
                    cuda_version: None,
                    rocm_available: false,
                    rocm_version: None,
                    vulkan_available: true,
                    opencl_available: false,
                    compute_capability: None,
                    driver_version: None,
                });
            }
        }
    }

    // Try AMD ROCm
    if let Ok(output) = Command::new("rocm-smi")
        .args(&["--showmeminfo", "vram"])
        .output()
    {
        if output.status.success() {
            // Parse AMD GPU info
            // This is simplified - real parsing would be more complex
            return Ok(GPUInfo {
                vendor: "AMD".to_string(),
                model: "AMD GPU".to_string(),
                vram_total: Some(8192), // Default estimate
                vram_free: Some(6144),  // Default estimate
                utilization: 0.0,
                temperature: None,
                power_consumption: None,
                cuda_available: false,
                cuda_version: None,
                rocm_available: true,
                rocm_version: None,
                vulkan_available: true,
                opencl_available: false,
                compute_capability: None,
                driver_version: None,
            });
        }
    }

    // Fallback: No dedicated GPU or integrated only
    Ok(GPUInfo {
        vendor: "Integrated".to_string(),
        model: "Integrated Graphics".to_string(),
        vram_total: None,
        vram_free: None,
        utilization: 0.0,
        temperature: None,
        power_consumption: None,
        cuda_available: false,
        cuda_version: None,
        rocm_available: false,
        rocm_version: None,
        vulkan_available: false,
        opencl_available: false,
        compute_capability: None,
        driver_version: None,
    })
}

/// Detect storage information
fn detect_storage_info() -> Result<StorageInfo, AIError> {
    // Use df command to get disk usage
    if let Ok(output) = Command::new("df")
        .args(&["-BG", "/"])
        .output()
    {
        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            for line in output_str.lines().skip(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    let total = parts[1].trim_end_matches('G').parse().unwrap_or(0);
                    let used = parts[2].trim_end_matches('G').parse().unwrap_or(0);
                    let free = total - used;
                    
                    return Ok(StorageInfo { total, free });
                }
            }
        }
    }

    // Fallback values
    Ok(StorageInfo {
        total: 100,
        free: 20,
    })
}

/// Detect battery information
fn detect_battery_info() -> Option<BatteryInfo> {
    // Check if we're on a laptop by looking for battery info
    if let Ok(output) = Command::new("upower")
        .args(&["-i", "/org/freedesktop/UPower/devices/battery_BAT0"])
        .output()
    {
        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            let mut level = 100;
            let mut charging = false;
            let mut on_battery = true;

            for line in output_str.lines() {
                if line.contains("percentage:") {
                    if let Some(pct) = line.split(':').nth(1) {
                        level = pct.trim().trim_end_matches('%').parse().unwrap_or(100);
                    }
                } else if line.contains("state:") {
                    if line.contains("charging") {
                        charging = true;
                        on_battery = false;
                    } else if line.contains("fully-charged") {
                        charging = false;
                        on_battery = false;
                    }
                }
            }

            return Some(BatteryInfo {
                level,
                charging,
                on_battery,
                time_to_empty: None, // TODO: Parse time remaining
                time_to_full: None,  // TODO: Parse time remaining
                health: None,        // TODO: Parse battery health
                power_consumption: None, // TODO: Parse power consumption
            });
        }
    }

    None
}

/// Select optimal model based on hardware profile
pub fn select_optimal_model(
    profile: &HardwareProfile,
    optimization: HardwareOptimization,
) -> String {
    let model_size = match optimization {
        HardwareOptimization::Auto => {
            // Automatic selection based on available resources
            if let Some(vram_free) = profile.gpu.vram_free {
                if vram_free >= ModelSize::ExtraLarge.vram_requirement() {
                    ModelSize::ExtraLarge
                } else if vram_free >= ModelSize::Large.vram_requirement() {
                    ModelSize::Large
                } else if vram_free >= ModelSize::Medium.vram_requirement() {
                    ModelSize::Medium
                } else if vram_free >= ModelSize::Small.vram_requirement() {
                    ModelSize::Small
                } else if profile.memory.available >= ModelSize::Medium.ram_requirement() {
                    // Fall back to CPU inference
                    ModelSize::Medium
                } else if profile.memory.available >= ModelSize::Small.ram_requirement() {
                    ModelSize::Small
                } else {
                    ModelSize::Tiny
                }
            } else {
                // No dedicated GPU, use CPU inference
                if profile.memory.available >= ModelSize::Medium.ram_requirement() {
                    ModelSize::Medium
                } else if profile.memory.available >= ModelSize::Small.ram_requirement() {
                    ModelSize::Small
                } else {
                    ModelSize::Tiny
                }
            }
        }
        HardwareOptimization::PreferGPU => {
            // Prefer GPU but fall back to smaller models if needed
            if let Some(vram_free) = profile.gpu.vram_free {
                if vram_free >= ModelSize::Small.vram_requirement() {
                    // Select largest model that fits in VRAM
                    if vram_free >= ModelSize::ExtraLarge.vram_requirement() {
                        ModelSize::ExtraLarge
                    } else if vram_free >= ModelSize::Large.vram_requirement() {
                        ModelSize::Large
                    } else if vram_free >= ModelSize::Medium.vram_requirement() {
                        ModelSize::Medium
                    } else {
                        ModelSize::Small
                    }
                } else {
                    ModelSize::Tiny
                }
            } else {
                ModelSize::Tiny
            }
        }
        HardwareOptimization::CPUOnly => {
            // Select based on RAM only
            if profile.memory.available >= ModelSize::Large.ram_requirement() {
                ModelSize::Large
            } else if profile.memory.available >= ModelSize::Medium.ram_requirement() {
                ModelSize::Medium
            } else if profile.memory.available >= ModelSize::Small.ram_requirement() {
                ModelSize::Small
            } else {
                ModelSize::Tiny
            }
        }
        HardwareOptimization::PowerSaving => {
            // Always use small models for power saving
            if let Some(battery) = &profile.battery {
                if battery.on_battery && battery.level < 20 {
                    ModelSize::Tiny
                } else {
                    ModelSize::Small
                }
            } else {
                ModelSize::Small
            }
        }
    };

    model_size.model_name().to_string()
}

/// Check if the system should throttle AI operations
pub fn should_throttle_ai(profile: &HardwareProfile) -> bool {
    // Check CPU usage
    if profile.cpu.usage > 80.0 {
        return true;
    }

    // Check memory pressure
    let memory_usage = 100.0 - (profile.memory.free as f32 / profile.memory.total as f32 * 100.0);
    if memory_usage > 85.0 {
        return true;
    }

    // Check battery
    if let Some(battery) = &profile.battery {
        if battery.on_battery && battery.level < 20 {
            return true;
        }
    }

    false
}

/// Detect thermal information
fn detect_thermal_info() -> ThermalInfo {
    let mut thermal_info = ThermalInfo {
        cpu_temperature: None,
        gpu_temperature: None,
        system_temperature: None,
        thermal_throttling: false,
        fan_speeds: Vec::new(),
    };
    
    // Try to read CPU temperature from thermal zones
    if let Ok(entries) = fs::read_dir("/sys/class/thermal") {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with("thermal_zone") {
                    let temp_path = path.join("temp");
                    if let Ok(temp_str) = fs::read_to_string(temp_path) {
                        if let Ok(temp_millicelsius) = temp_str.trim().parse::<u32>() {
                            let temp_celsius = temp_millicelsius as f32 / 1000.0;
                            if thermal_info.cpu_temperature.is_none() {
                                thermal_info.cpu_temperature = Some(temp_celsius);
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Check for thermal throttling
    if let Some(cpu_temp) = thermal_info.cpu_temperature {
        thermal_info.thermal_throttling = cpu_temp > 85.0;
    }
    
    thermal_info
}

/// Detect network information
fn detect_network_info() -> NetworkInfo {
    let mut network_info = NetworkInfo {
        connected: false,
        connection_type: "unknown".to_string(),
        speed: None,
        latency: None,
    };
    
    // Simple connectivity check
    if let Ok(output) = Command::new("ping")
        .args(&["-c", "1", "-W", "1", "8.8.8.8"])
        .output()
    {
        network_info.connected = output.status.success();
        
        if network_info.connected {
            // Try to determine connection type
            if let Ok(output) = Command::new("nmcli")
                .args(&["-t", "-f", "TYPE,STATE", "connection", "show", "--active"])
                .output()
            {
                let output_str = String::from_utf8_lossy(&output.stdout);
                for line in output_str.lines() {
                    if line.contains("wifi") {
                        network_info.connection_type = "wifi".to_string();
                        break;
                    } else if line.contains("ethernet") {
                        network_info.connection_type = "ethernet".to_string();
                        break;
                    }
                }
            }
        }
    }
    
    network_info
}

/// Analyze system capabilities for AI workloads
fn analyze_system_capabilities(cpu: &CPUInfo, gpu: &GPUInfo, memory: &MemoryInfo) -> SystemCapabilities {
    let ai_acceleration = if gpu.cuda_available || gpu.rocm_available {
        if gpu.model.contains("RTX") || gpu.model.contains("A100") || gpu.model.contains("V100") {
            AccelerationSupport::TensorCores
        } else if gpu.vram_total.unwrap_or(0) > 4000 {
            AccelerationSupport::DedicatedGPU
        } else {
            AccelerationSupport::IntegratedGPU
        }
    } else if cpu.logical_cores >= 8 {
        AccelerationSupport::CPUOptimized
    } else {
        AccelerationSupport::None
    };
    
    let max_model_size = if gpu.vram_total.unwrap_or(0) >= 40000 {
        ModelSize::ExtraLarge
    } else if gpu.vram_total.unwrap_or(0) >= 20000 {
        ModelSize::Large
    } else if gpu.vram_total.unwrap_or(0) >= 8000 {
        ModelSize::Medium
    } else if memory.available >= 16000 {
        ModelSize::Medium
    } else if memory.available >= 8000 {
        ModelSize::Small
    } else {
        ModelSize::Tiny
    };
    
    let estimated_inference_speed = match ai_acceleration {
        AccelerationSupport::TensorCores => 50.0,
        AccelerationSupport::DedicatedGPU => 25.0,
        AccelerationSupport::IntegratedGPU => 10.0,
        AccelerationSupport::CPUOptimized => 5.0,
        AccelerationSupport::None => 2.0,
    };
    
    let max_concurrent_sessions = match ai_acceleration {
        AccelerationSupport::TensorCores => 4,
        AccelerationSupport::DedicatedGPU => 2,
        AccelerationSupport::IntegratedGPU => 1,
        AccelerationSupport::CPUOptimized => 1,
        AccelerationSupport::None => 1,
    };
    
    SystemCapabilities {
        max_concurrent_sessions,
        max_model_size,
        estimated_inference_speed,
        memory_bandwidth: None, // TODO: Implement memory bandwidth detection
        ai_acceleration,
        virtualization_support: detect_virtualization_support(),
    }
}

/// Detect virtualization support
fn detect_virtualization_support() -> bool {
    // Check for virtualization support in CPU flags
    if let Ok(cpuinfo) = fs::read_to_string("/proc/cpuinfo") {
        cpuinfo.contains("vmx") || cpuinfo.contains("svm")
    } else {
        false
    }
}

/// Hardware monitor for continuous monitoring
pub struct HardwareMonitor {
    profile: Arc<RwLock<HardwareProfile>>,
    last_update: Arc<RwLock<Instant>>,
    update_interval: Duration,
}

impl HardwareMonitor {
    /// Create a new hardware monitor with default interval
    pub fn new() -> Self {
        let profile = detect_hardware_profile().unwrap_or_else(|_| HardwareProfile::default());
        
        Self {
            profile: Arc::new(RwLock::new(profile)),
            last_update: Arc::new(RwLock::new(Instant::now())),
            update_interval: Duration::from_secs(30),
        }
    }
    
    /// Create a new hardware monitor with custom interval
    pub fn new_with_interval(update_interval: Duration) -> Result<Self, AIError> {
        let profile = detect_hardware_profile()?;
        
        Ok(Self {
            profile: Arc::new(RwLock::new(profile)),
            last_update: Arc::new(RwLock::new(Instant::now())),
            update_interval,
        })
    }
    
    /// Get the current hardware profile
    pub fn get_profile(&self) -> HardwareProfile {
        self.profile.read().clone()
    }
    
    /// Update the hardware profile if needed
    pub fn update_if_needed(&self) -> Result<bool, AIError> {
        let now = Instant::now();
        let last_update = *self.last_update.read();
        
        if now.duration_since(last_update) >= self.update_interval {
            let new_profile = detect_hardware_profile()?;
            *self.profile.write() = new_profile;
            *self.last_update.write() = now;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    /// Force update the hardware profile
    pub fn force_update(&self) -> Result<(), AIError> {
        let new_profile = detect_hardware_profile()?;
        *self.profile.write() = new_profile;
        *self.last_update.write() = Instant::now();
        Ok(())
    }
}

/// Get performance recommendations based on hardware
pub fn get_performance_recommendations(profile: &HardwareProfile) -> Vec<String> {
    let mut recommendations = Vec::new();
    
    // Memory recommendations
    let memory_usage = 100.0 - (profile.memory.free as f32 / profile.memory.total as f32 * 100.0);
    if memory_usage > 80.0 {
        recommendations.push("Consider closing unnecessary applications to free up memory".to_string());
    }
    
    // GPU recommendations
    if profile.gpu.cuda_available || profile.gpu.rocm_available {
        if profile.gpu.vram_total.unwrap_or(0) < 8000 {
            recommendations.push("Consider using smaller AI models for better performance".to_string());
        }
    } else {
        recommendations.push("Consider using CPU-optimized models or upgrading to a dedicated GPU".to_string());
    }
    
    // Thermal recommendations
    if profile.thermal.thermal_throttling {
        recommendations.push("System is thermal throttling. Consider improving cooling or reducing workload".to_string());
    }
    
    // Battery recommendations
    if let Some(battery) = &profile.battery {
        if battery.on_battery && battery.level < 30 {
            recommendations.push("Low battery detected. Consider connecting to power for intensive AI workloads".to_string());
        }
    }
    
    // Storage recommendations
    if profile.storage.free < 10 {
        recommendations.push("Low disk space. Consider cleaning up storage for model downloads".to_string());
    }
    
    recommendations
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_size_requirements() {
        assert_eq!(ModelSize::Tiny.vram_requirement(), 2_000);
        assert_eq!(ModelSize::Small.vram_requirement(), 4_000);
        assert_eq!(ModelSize::Medium.vram_requirement(), 8_000);
        assert_eq!(ModelSize::Large.vram_requirement(), 20_000);
        assert_eq!(ModelSize::ExtraLarge.vram_requirement(), 40_000);
    }

    #[test]
    fn test_model_names() {
        assert_eq!(ModelSize::Tiny.model_name(), "tinyllama:latest");
        assert_eq!(ModelSize::Small.model_name(), "llama3.2:latest");
        assert_eq!(ModelSize::Medium.model_name(), "llama3.2:13b");
        assert_eq!(ModelSize::Large.model_name(), "llama3.2:34b");
        assert_eq!(ModelSize::ExtraLarge.model_name(), "llama3.2:70b");
    }
}