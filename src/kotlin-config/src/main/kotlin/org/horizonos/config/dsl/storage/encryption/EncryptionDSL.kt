package org.horizonos.config.dsl.storage.encryption

import org.horizonos.config.dsl.HorizonOSDsl

// ===== Encryption DSL Builders =====

@HorizonOSDsl
class EncryptionContext {
    var enabled = false
    private val devices = mutableListOf<EncryptedDevice>()
    private var keyManagement = KeyManagement()
    private var performance = EncryptionPerformance()
    
    fun device(name: String, device: String, block: EncryptedDeviceContext.() -> Unit = {}) {
        val context = EncryptedDeviceContext(name, device).apply(block)
        devices.add(context.toConfig())
    }
    
    fun keyManagement(block: KeyManagementContext.() -> Unit) {
        keyManagement = KeyManagementContext().apply(block).toConfig()
    }
    
    fun performance(block: EncryptionPerformanceContext.() -> Unit) {
        performance = EncryptionPerformanceContext().apply(block).toConfig()
    }
    
    fun toConfig() = EncryptionConfig(
        enabled = enabled,
        devices = devices,
        keyManagement = keyManagement,
        performance = performance
    )
}

@HorizonOSDsl
class EncryptedDeviceContext(
    private val name: String,
    private val device: String
) {
    var cipher = CipherSpec.AES_XTS_PLAIN64
    var keySize = 512
    var hashAlgorithm = HashAlgorithm.SHA256
    var iterTime = 2000
    var keyFile: String? = null
    private var header = LUKSHeader()
    var discard = false
    var persistent = true
    var noReadWorkqueue = false
    var noWriteWorkqueue = false
    private var integrityProtection: IntegrityProtection? = null
    
    fun header(block: LUKSHeaderContext.() -> Unit) {
        header = LUKSHeaderContext().apply(block).toConfig()
    }
    
    fun integrity(block: IntegrityProtectionContext.() -> Unit) {
        integrityProtection = IntegrityProtectionContext().apply(block).toConfig()
    }
    
    fun toConfig() = EncryptedDevice(
        name = name,
        device = device,
        cipher = cipher,
        keySize = keySize,
        hashAlgorithm = hashAlgorithm,
        iterTime = iterTime,
        keyFile = keyFile,
        header = header,
        discard = discard,
        persistent = persistent,
        noReadWorkqueue = noReadWorkqueue,
        noWriteWorkqueue = noWriteWorkqueue,
        integrityProtection = integrityProtection
    )
}

@HorizonOSDsl
class LUKSHeaderContext {
    var version = LUKSVersion.LUKS2
    var pbkdf = PBKDF.ARGON2ID
    var memory = 1048576 // 1GB in KB
    var parallelism = 4
    var detachedHeader: String? = null
    var headerBackup: String? = null
    
    fun toConfig() = LUKSHeader(
        version = version,
        pbkdf = pbkdf,
        memory = memory,
        parallelism = parallelism,
        detachedHeader = detachedHeader,
        headerBackup = headerBackup
    )
}

@HorizonOSDsl
class IntegrityProtectionContext {
    var algorithm = IntegrityAlgorithm.HMAC_SHA256
    var journalSize = "64M"
    var journalWatermark = 50
    var journalCommitTime = 10
    
    fun toConfig() = IntegrityProtection(
        algorithm = algorithm,
        journalSize = journalSize,
        journalWatermark = journalWatermark,
        journalCommitTime = journalCommitTime
    )
}

@HorizonOSDsl
class KeyManagementContext {
    var allowDiscards = false
    private val keyFiles = mutableListOf<KeyFile>()
    private var tpmIntegration = TPMIntegration()
    private val escrowKeys = mutableListOf<EscrowKey>()
    private var passwordQuality = PasswordQuality()
    
    fun keyFile(path: String, block: KeyFileContext.() -> Unit = {}) {
        val context = KeyFileContext(path).apply(block)
        keyFiles.add(context.toConfig())
    }
    
    fun tpm(block: TPMIntegrationContext.() -> Unit) {
        tpmIntegration = TPMIntegrationContext().apply(block).toConfig()
    }
    
    fun escrowKey(name: String, publicKey: String, description: String? = null) {
        escrowKeys.add(EscrowKey(name, publicKey, description))
    }
    
    fun passwordQuality(block: PasswordQualityContext.() -> Unit) {
        passwordQuality = PasswordQualityContext().apply(block).toConfig()
    }
    
    fun toConfig() = KeyManagement(
        allowDiscards = allowDiscards,
        keyFiles = keyFiles,
        tpmIntegration = tpmIntegration,
        escrowKeys = escrowKeys,
        passwordQuality = passwordQuality
    )
}

@HorizonOSDsl
class KeyFileContext(private val path: String) {
    var offset = 0L
    var size: Long? = null
    var permissions = "0400"
    var removeAfterBoot = false
    
    fun toConfig() = KeyFile(
        path = path,
        offset = offset,
        size = size,
        permissions = permissions,
        removeAfterBoot = removeAfterBoot
    )
}

@HorizonOSDsl
class TPMIntegrationContext {
    var enabled = false
    var tpmVersion = TPMVersion.TPM2
    val pcrs = mutableListOf(0, 2, 4, 7)
    var sealingPolicy: String? = null
    var nvramIndex: Int? = null
    
    fun pcr(index: Int) {
        pcrs.add(index)
    }
    
    fun toConfig() = TPMIntegration(
        enabled = enabled,
        tpmVersion = tpmVersion,
        pcrs = pcrs,
        sealingPolicy = sealingPolicy,
        nvramIndex = nvramIndex
    )
}

@HorizonOSDsl
class PasswordQualityContext {
    var minLength = 12
    var requireUppercase = true
    var requireLowercase = true
    var requireDigits = true
    var requireSpecial = true
    var prohibitReuse = 5
    var checkDictionary = true
    
    fun toConfig() = PasswordQuality(
        minLength = minLength,
        requireUppercase = requireUppercase,
        requireLowercase = requireLowercase,
        requireDigits = requireDigits,
        requireSpecial = requireSpecial,
        prohibitReuse = prohibitReuse,
        checkDictionary = checkDictionary
    )
}

@HorizonOSDsl
class EncryptionPerformanceContext {
    var queueDepth = 128
    val workqueueCPUs = mutableListOf<Int>()
    var sectorSize = 512
    var noReadWorkqueue = false
    var noWriteWorkqueue = false
    var submitFromCryptCPUs = true
    
    fun workqueueCPU(cpu: Int) {
        workqueueCPUs.add(cpu)
    }
    
    fun toConfig() = EncryptionPerformance(
        queueDepth = queueDepth,
        workqueueCPUs = if (workqueueCPUs.isEmpty()) null else workqueueCPUs,
        sectorSize = sectorSize,
        noReadWorkqueue = noReadWorkqueue,
        noWriteWorkqueue = noWriteWorkqueue,
        submitFromCryptCPUs = submitFromCryptCPUs
    )
}