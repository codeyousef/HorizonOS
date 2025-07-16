package org.horizonos.config.dsl.boot.kernel

import kotlinx.serialization.Serializable

/**
 * Kernel Configuration for HorizonOS
 * 
 * This module provides comprehensive kernel configuration including parameters,
 * modules, variants, compression, debugging, and security settings. It supports
 * multiple kernel versions and advanced security mitigations.
 * 
 * ## Key Features:
 * - Kernel parameter management with conditional application
 * - Module loading, blacklisting, and configuration
 * - Multiple kernel variant support
 * - Compression options for kernel and modules
 * - Debugging and profiling capabilities
 * - Advanced security mitigations (KASLR, Spectre, Meltdown, etc.)
 * 
 * ## Usage Example:
 * ```kotlin
 * boot {
 *     kernel {
 *         parameters {
 *             add("quiet")
 *             add("splash")
 *             add("mitigations", "auto")
 *         }
 *         
 *         modules {
 *             blacklist("pcspkr", "snd_pcsp")
 *             load("btrfs", "zstd")
 *         }
 *         
 *         security {
 *             kaslr = true
 *             spectreV2 = SpectreV2Mitigation.AUTO
 *             mds = MDSMitigation.FULL
 *         }
 *     }
 * }
 * ```
 * 
 * @since 1.0
 */

// ===== Kernel Configuration =====

/**
 * Main kernel configuration for HorizonOS
 * 
 * This class defines the complete kernel configuration including boot parameters,
 * module settings, variant management, and security options.
 */
@Serializable
data class KernelConfig(
    /** List of kernel boot parameters */
    val parameters: List<KernelParameter> = emptyList(),
    
    /** Kernel module configuration and management */
    val modules: KernelModuleConfig = KernelModuleConfig(),
    
    /** Preferred kernel version (null for latest) */
    val version: String? = null,
    
    /** Available kernel variants (hardened, realtime, etc.) */
    val variants: List<KernelVariant> = emptyList(),
    
    /** Kernel compression algorithm */
    val compression: KernelCompression = KernelCompression.ZSTD,
    
    /** Kernel debugging and profiling settings */
    val debugging: KernelDebugging = KernelDebugging(),
    
    /** Security mitigations and hardening settings */
    val security: KernelSecurity = KernelSecurity()
)

/**
 * Kernel boot parameter configuration
 * 
 * This class defines a single kernel parameter that can be passed to the kernel
 * at boot time, with optional value and conditional application.
 */
@Serializable
data class KernelParameter(
    /** Parameter name (e.g., "quiet", "splash", "mitigations") */
    val name: String,
    
    /** Parameter value (null for flag parameters) */
    val value: String? = null,
    
    /** Condition for applying this parameter (e.g., "debug", "production") */
    val condition: String? = null
)

/**
 * Kernel module configuration and management
 * 
 * This class defines how kernel modules are loaded, blacklisted, and configured
 * within the system.
 */
@Serializable
data class KernelModuleConfig(
    /** List of modules to blacklist (prevent loading) */
    val blacklist: List<String> = emptyList(),
    
    /** List of modules to force load at boot */
    val load: List<String> = emptyList(),
    
    /** Module-specific configuration options */
    val options: Map<String, String> = emptyMap(),
    
    /** Whether to automatically load modules based on hardware detection */
    val autoLoad: Boolean = true,
    
    /** Compression algorithm for kernel modules */
    val compression: ModuleCompression = ModuleCompression.XZ
)

/**
 * Kernel variant configuration
 * 
 * This class defines a specific kernel variant (e.g., hardened, realtime, zen)
 * with its version, location, and metadata.
 */
@Serializable
data class KernelVariant(
    /** Variant name (e.g., "hardened", "realtime", "zen") */
    val name: String,
    
    /** Kernel version for this variant */
    val version: String,
    
    /** Path to the kernel image */
    val path: String,
    
    /** Whether this variant is enabled for selection */
    val enabled: Boolean = true,
    
    /** Human-readable description of the variant */
    val description: String? = null,
    
    /** Whether this is the default kernel variant */
    val isDefault: Boolean = false
)

/**
 * Kernel debugging and profiling configuration
 * 
 * This class enables various kernel debugging features and profiling tools
 * for system development and troubleshooting.
 */
@Serializable
data class KernelDebugging(
    /** Whether kernel debugging is enabled */
    val enabled: Boolean = false,
    
    /** Enable kernel GDB support for remote debugging */
    val kgdb: Boolean = false,
    
    /** Enable kernel debugger (KDB) */
    val kdb: Boolean = false,
    
    /** Enable early kernel printk for boot debugging */
    val earlyPrintk: Boolean = false,
    
    /** Debug message verbosity level (0-7) */
    val debugLevel: Int = 0,
    
    /** Enable function tracing (ftrace) */
    val ftrace: Boolean = false,
    
    /** Enable kernel probes for dynamic instrumentation */
    val kprobes: Boolean = false,
    
    /** Enable dynamic debug message filtering */
    val dynamicDebug: Boolean = false
)

/**
 * Kernel security mitigations and hardening
 * 
 * This class configures various security features and mitigations for
 * protection against CPU vulnerabilities and attack vectors.
 */
@Serializable
data class KernelSecurity(
    /** Kernel Address Space Layout Randomization */
    val kaslr: Boolean = true,
    
    /** Supervisor Mode Execution Prevention */
    val smep: Boolean = true,
    
    /** Supervisor Mode Access Prevention */
    val smap: Boolean = true,
    
    /** Page Table Isolation (Meltdown mitigation) */
    val pti: Boolean = true,
    
    /** Kernel Page Table Isolation */
    val kpti: Boolean = true,
    
    /** Spectre v2 vulnerability mitigation strategy */
    val spectreV2: SpectreV2Mitigation = SpectreV2Mitigation.AUTO,
    
    /** Spectre v4 (Speculative Store Bypass) mitigation */
    val spectreV4: Boolean = true,
    
    /** L1 Terminal Fault (Foreshadow) mitigation */
    val l1tf: L1TFMitigation = L1TFMitigation.FLUSH,
    
    /** Microarchitectural Data Sampling mitigation */
    val mds: MDSMitigation = MDSMitigation.FULL,
    
    /** Disable Intel TSX (Transactional Synchronization Extensions) */
    val tsx: Boolean = false,
    
    /** Indirect Branch Restricted Speculation */
    val ibrs: Boolean = true,
    
    /** Indirect Branch Prediction Barrier */
    val ibpb: Boolean = true,
    
    /** Single Thread Indirect Branch Predictors */
    val stibp: Boolean = true,
    
    /** Speculative Store Bypass Disable */
    val ssbd: Boolean = true,
    
    /** Retpoline indirect branch speculation mitigation */
    val retpoline: Boolean = true
)

// Kernel Enums

/**
 * Kernel compression algorithms
 * 
 * Different compression algorithms offer trade-offs between compression ratio,
 * decompression speed, and boot time.
 */
@Serializable
enum class KernelCompression {
    /** No compression - fastest boot, largest size */
    NONE,
    
    /** GZIP compression - standard compression with good compatibility */
    GZIP,
    
    /** BZIP2 compression - high compression ratio, slower decompression */
    BZIP2,
    
    /** LZMA compression - excellent compression ratio, moderate speed */
    LZMA,
    
    /** XZ compression - very good compression ratio, fast decompression */
    XZ,
    
    /** LZO compression - very fast decompression, moderate compression */
    LZO,
    
    /** LZ4 compression - extremely fast decompression, lower compression */
    LZ4,
    
    /** ZSTD compression - excellent balance of speed and compression */
    ZSTD
}

/**
 * Module compression algorithms
 * 
 * Compression algorithms for kernel modules to reduce storage space.
 */
@Serializable
enum class ModuleCompression {
    /** No compression - fastest module loading */
    NONE,
    
    /** GZIP compression - standard compression with good compatibility */
    GZIP,
    
    /** XZ compression - excellent compression ratio, good speed */
    XZ,
    
    /** ZSTD compression - optimal balance of speed and compression */
    ZSTD
}

/**
 * Spectre v2 vulnerability mitigation strategies
 * 
 * Different approaches to mitigate Spectre v2 attacks with varying
 * performance and security trade-offs.
 */
@Serializable
enum class SpectreV2Mitigation {
    /** Disable Spectre v2 mitigations - fastest but vulnerable */
    OFF,
    
    /** Automatic selection based on CPU capabilities */
    AUTO,
    
    /** Enable default mitigations for detected CPU */
    ON,
    
    /** Use retpoline technique - good performance, broad compatibility */
    RETPOLINE,
    
    /** Use Indirect Branch Restricted Speculation */
    IBRS,
    
    /** Use enhanced IBRS (if supported by CPU) */
    IBRS_ENHANCED,
    
    /** Use Enhanced Indirect Branch Restricted Speculation */
    EIBRS,
    
    /** Combine enhanced IBRS with retpoline fallback */
    EIBRS_RETPOLINE,
    
    /** Use enhanced IBRS with LFENCE serialization */
    EIBRS_LFENCE
}

/**
 * L1 Terminal Fault (Foreshadow) mitigation strategies
 * 
 * Different approaches to mitigate L1TF attacks on Intel processors.
 */
@Serializable
enum class L1TFMitigation {
    /** Disable L1TF mitigations - fastest but vulnerable */
    OFF,
    
    /** Flush L1 data cache on context switches */
    FLUSH,
    
    /** Flush L1 cache and disable SMT (Simultaneous Multi-Threading) */
    FLUSH_NOSMT,
    
    /** Flush L1 cache without warnings */
    FLUSH_NOWARN,
    
    /** Full mitigations including L1 cache flush */
    FULL,
    
    /** Force full mitigations regardless of CPU support */
    FULL_FORCE
}

/**
 * Microarchitectural Data Sampling (MDS) mitigation strategies
 * 
 * Different approaches to mitigate MDS vulnerabilities (RIDL, Fallout, ZombieLoad).
 */
@Serializable
enum class MDSMitigation {
    /** Disable MDS mitigations - fastest but vulnerable */
    OFF,
    
    /** Full MDS mitigations with buffer clearing */
    FULL,
    
    /** Full mitigations and disable SMT for maximum security */
    FULL_NOSMT
}