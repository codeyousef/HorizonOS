//! Hardware detection and model selection for optimal AI performance

use crate::{AIError, HardwareOptimization};
use serde::{Deserialize, Serialize};
use sysinfo::System;
use std::process::Command;

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

/// GPU information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GPUInfo {
    /// GPU vendor
    pub vendor: String,
    /// GPU model
    pub model: String,
    /// VRAM in MB
    pub vram_total: u64,
    /// Available VRAM in MB
    pub vram_free: u64,
    /// Whether CUDA is available
    pub cuda_available: bool,
    /// Whether ROCm is available
    pub rocm_available: bool,
    /// Whether Vulkan is available
    pub vulkan_available: bool,
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

/// Storage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageInfo {
    /// Total storage in GB
    pub total: u64,
    /// Free storage in GB
    pub free: u64,
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

    Ok(HardwareProfile {
        cpu,
        gpu,
        memory,
        storage,
        battery,
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
                return Ok(GPUInfo {
                    vendor: "NVIDIA".to_string(),
                    model: parts[0].trim().to_string(),
                    vram_total: parts[1].trim().parse().unwrap_or(0),
                    vram_free: parts[2].trim().parse().unwrap_or(0),
                    cuda_available: true,
                    rocm_available: false,
                    vulkan_available: true,
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
                vram_total: 8192, // Default estimate
                vram_free: 6144,  // Default estimate
                cuda_available: false,
                rocm_available: true,
                vulkan_available: true,
            });
        }
    }

    // Fallback: No dedicated GPU or integrated only
    Ok(GPUInfo {
        vendor: "Integrated".to_string(),
        model: "Integrated Graphics".to_string(),
        vram_total: 0,
        vram_free: 0,
        cuda_available: false,
        rocm_available: false,
        vulkan_available: false,
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
            if profile.gpu.vram_free >= ModelSize::ExtraLarge.vram_requirement() {
                ModelSize::ExtraLarge
            } else if profile.gpu.vram_free >= ModelSize::Large.vram_requirement() {
                ModelSize::Large
            } else if profile.gpu.vram_free >= ModelSize::Medium.vram_requirement() {
                ModelSize::Medium
            } else if profile.gpu.vram_free >= ModelSize::Small.vram_requirement() {
                ModelSize::Small
            } else if profile.memory.available >= ModelSize::Medium.ram_requirement() {
                // Fall back to CPU inference
                ModelSize::Medium
            } else if profile.memory.available >= ModelSize::Small.ram_requirement() {
                ModelSize::Small
            } else {
                ModelSize::Tiny
            }
        }
        HardwareOptimization::PreferGPU => {
            // Prefer GPU but fall back to smaller models if needed
            if profile.gpu.vram_free >= ModelSize::Small.vram_requirement() {
                // Select largest model that fits in VRAM
                if profile.gpu.vram_free >= ModelSize::ExtraLarge.vram_requirement() {
                    ModelSize::ExtraLarge
                } else if profile.gpu.vram_free >= ModelSize::Large.vram_requirement() {
                    ModelSize::Large
                } else if profile.gpu.vram_free >= ModelSize::Medium.vram_requirement() {
                    ModelSize::Medium
                } else {
                    ModelSize::Small
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