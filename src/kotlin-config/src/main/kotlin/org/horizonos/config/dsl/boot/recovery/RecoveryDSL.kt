package org.horizonos.config.dsl.boot.recovery

import org.horizonos.config.dsl.HorizonOSDsl

// ===== Recovery DSL Builders =====

@HorizonOSDsl
class RecoveryContext {
    var enabled: Boolean = true
    var autoboot: Boolean = false
    var timeout: Int = 0
    var kernel: String? = null
    var initrd: String? = null
    private val options = mutableListOf<String>()
    private val services = mutableListOf<String>()
    private val environment = mutableMapOf<String, String>()
    
    fun option(opt: String) {
        options.add(opt)
    }
    
    fun service(svc: String) {
        services.add(svc)
    }
    
    fun env(key: String, value: String) {
        environment[key] = value
    }
    
    fun toConfig(): RecoveryConfig {
        return RecoveryConfig(
            enabled = enabled,
            autoboot = autoboot,
            timeout = timeout,
            kernel = kernel,
            initrd = initrd,
            options = options,
            services = services,
            environment = environment
        )
    }
}