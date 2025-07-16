package org.horizonos.config.dsl.boot.secureboot

import org.horizonos.config.dsl.HorizonOSDsl

// ===== Secure Boot DSL Builders =====

@HorizonOSDsl
class SecureBootContext {
    var enabled: Boolean = false
    var enrollKeys: Boolean = false
    var setupMode: Boolean = false
    var allowUnsignedDrivers: Boolean = false
    var mokManager: Boolean = false
    var signKernel: Boolean = false
    var signModules: Boolean = false
    private var keys = SecureBootKeys()
    
    fun keys(block: SecureBootKeysContext.() -> Unit) {
        keys = SecureBootKeysContext().apply(block).toKeys()
    }
    
    fun toConfig(): SecureBootConfig {
        return SecureBootConfig(
            enabled = enabled,
            enrollKeys = enrollKeys,
            setupMode = setupMode,
            allowUnsignedDrivers = allowUnsignedDrivers,
            mokManager = mokManager,
            signKernel = signKernel,
            signModules = signModules,
            keys = keys
        )
    }
}

@HorizonOSDsl
class SecureBootKeysContext {
    var pk: String? = null
    private val kek = mutableListOf<String>()
    private val db = mutableListOf<String>()
    private val dbx = mutableListOf<String>()
    private val mok = mutableListOf<String>()
    private val mokListRT = mutableListOf<String>()
    var platform: String? = null
    var keyExchange: String? = null
    var signature: String? = null
    private val forbidden = mutableListOf<String>()
    
    fun kek(key: String) {
        kek.add(key)
    }
    
    fun db(key: String) {
        db.add(key)
    }
    
    fun dbx(key: String) {
        dbx.add(key)
    }
    
    fun mok(key: String) {
        mok.add(key)
    }
    
    fun mokListRT(key: String) {
        mokListRT.add(key)
    }
    
    fun forbidden(key: String) {
        forbidden.add(key)
    }
    
    fun toKeys(): SecureBootKeys {
        return SecureBootKeys(
            pk = pk,
            kek = kek,
            db = db,
            dbx = dbx,
            mok = mok,
            mokListRT = mokListRT,
            platform = platform,
            keyExchange = keyExchange,
            signature = signature,
            forbidden = forbidden
        )
    }
}