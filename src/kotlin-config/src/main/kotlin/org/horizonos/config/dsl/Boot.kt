package org.horizonos.config.dsl

import kotlinx.serialization.Serializable
import org.horizonos.config.dsl.boot.bootloader.*
import org.horizonos.config.dsl.boot.kernel.*
import org.horizonos.config.dsl.boot.initramfs.*
import org.horizonos.config.dsl.boot.plymouth.*
import org.horizonos.config.dsl.boot.secureboot.*
import org.horizonos.config.dsl.boot.recovery.*

/**
 * Boot and Kernel Configuration DSL for HorizonOS
 * 
 * Provides type-safe configuration for bootloader settings, kernel parameters,
 * module management, initramfs configuration, and boot themes.
 */

// ===== Boot Configuration =====

@Serializable
data class BootConfig(
    val bootloader: BootloaderConfig = BootloaderConfig(),
    val kernel: KernelConfig = KernelConfig(),
    val initramfs: InitramfsConfig = InitramfsConfig(),
    val plymouth: PlymouthConfig = PlymouthConfig(),
    val secureBoot: SecureBootConfig = SecureBootConfig(),
    val recovery: RecoveryConfig = RecoveryConfig()
)

// ===== Boot DSL Builder =====

@HorizonOSDsl
class BootContext {
    private var bootloader = BootloaderConfig()
    private var kernel = KernelConfig()
    private var initramfs = InitramfsConfig()
    private var plymouth = PlymouthConfig()
    private var secureBoot = SecureBootConfig()
    private var recovery = RecoveryConfig()
    
    fun bootloader(block: BootloaderContext.() -> Unit) {
        bootloader = BootloaderContext().apply(block).toConfig()
    }
    
    fun kernel(block: KernelContext.() -> Unit) {
        kernel = KernelContext().apply(block).toConfig()
    }
    
    fun initramfs(block: InitramfsContext.() -> Unit) {
        initramfs = InitramfsContext().apply(block).toConfig()
    }
    
    fun plymouth(block: PlymouthContext.() -> Unit) {
        plymouth = PlymouthContext().apply(block).toConfig()
    }
    
    fun secureBoot(block: SecureBootContext.() -> Unit) {
        secureBoot = SecureBootContext().apply(block).toConfig()
    }
    
    fun recovery(block: RecoveryContext.() -> Unit) {
        recovery = RecoveryContext().apply(block).toConfig()
    }
    
    fun toConfig(): BootConfig {
        return BootConfig(
            bootloader = bootloader,
            kernel = kernel,
            initramfs = initramfs,
            plymouth = plymouth,
            secureBoot = secureBoot,
            recovery = recovery
        )
    }
}

// ===== Boot DSL Function =====

@HorizonOSDsl
fun boot(block: BootContext.() -> Unit): BootConfig =
    BootContext().apply(block).toConfig()