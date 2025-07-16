package org.horizonos.config.dsl

import org.horizonos.config.dsl.boot.bootloader.BootEntry
import org.horizonos.config.dsl.boot.kernel.KernelParameter
import org.horizonos.config.dsl.boot.kernel.KernelVariant

/**
 * Extension functions for configuration testing and utility
 */

// Boot configuration extensions
fun CompiledConfig.hasBoot(): Boolean = this.boot != null

fun CompiledConfig.getBootEntry(title: String): BootEntry? {
    return this.boot?.bootloader?.entries?.find { it.title == title }
}

fun CompiledConfig.getKernelParameter(name: String): KernelParameter? {
    return this.boot?.kernel?.parameters?.find { it.name == name }
}

fun CompiledConfig.getKernelVariant(name: String): KernelVariant? {
    return this.boot?.kernel?.variants?.find { it.name == name }
}

// AI configuration extensions are already defined in AI.kt

// Network configuration extensions
fun CompiledConfig.hasNetwork(): Boolean = this.network != null

// Security configuration extensions
fun CompiledConfig.hasSecurity(): Boolean = this.security != null

// Hardware configuration extensions
fun CompiledConfig.hasHardware(): Boolean = this.hardware != null

// Storage configuration extensions
fun CompiledConfig.hasStorage(): Boolean = this.storage != null

// Desktop configuration extensions
fun CompiledConfig.hasDesktop(): Boolean = this.desktop != null

// Development configuration extensions
fun CompiledConfig.hasDevelopment(): Boolean = this.development != null

// Automation configuration extensions
fun CompiledConfig.hasAutomation(): Boolean = this.automation != null