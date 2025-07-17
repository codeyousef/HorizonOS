package org.horizonos.config.dsl

import kotlinx.serialization.Serializable
import java.time.Instant
import java.time.LocalDateTime
import java.time.format.DateTimeFormatter

/**
 * Reproducible Build Support for HorizonOS Container Architecture
 * 
 * This module provides reproducible build capabilities through:
 * - OSTree commit pinning for immutable base system
 * - Container image digest verification
 * - Flatpak commit tracking
 * - System state lockfiles with image signatures
 * - Deterministic layer ordering
 * 
 * @see [Containers] for container image digest pinning
 * @see [ContainersConfig] for container configuration
 * @see [SystemContainer] for individual container settings
 * @see [Layers] for layered architecture configuration
 * @see [BaseLayer] for base OSTree layer configuration
 * @see [Packages] for package reproducibility
 * @see [Storage] for storage configuration
 * @see [horizonOS] for main system configuration entry point
 */

// ===== Core Data Structures =====

@Serializable
data class ReproducibleConfig(
    val enabled: Boolean = true,
    val strictMode: Boolean = false,
    val verifyDigests: Boolean = true,
    val lockfile: String = "/etc/horizonos/system.lock",
    val pinnedBase: String? = null,
    val systemImage: SystemImage? = null,
    val validationMode: ValidationMode = ValidationMode.WARN,
    val signatureValidation: Boolean = true,
    val allowUnsigned: Boolean = false
)

@Serializable
data class SystemImage(
    val version: String = "1.0",
    val timestamp: String = Instant.now().toString(),
    val base: OstreeImage,
    val containers: List<ContainerImage> = emptyList(),
    val flatpaks: List<FlatpakImage> = emptyList(),
    val layers: List<LayerImage> = emptyList(),
    val signature: String? = null,
    val metadata: Map<String, String> = emptyMap()
)

@Serializable
data class OstreeImage(
    val ref: String,
    val commit: String,
    val version: String,
    val digest: String,
    val url: String? = null,
    val signature: String? = null,
    val size: Long = 0,
    val timestamp: String = Instant.now().toString()
)

@Serializable
data class ContainerImage(
    val name: String,
    val image: String,
    val tag: String,
    val digest: String,
    val runtime: ContainerRuntime,
    val purpose: ContainerPurpose,
    val packages: List<PackageInfo> = emptyList(),
    val size: Long = 0,
    val layers: List<String> = emptyList(),
    val signature: String? = null,
    val buildTime: String = Instant.now().toString()
)

@Serializable
data class FlatpakImage(
    val id: String,
    val version: String,
    val branch: String,
    val commit: String,
    val runtime: String,
    val runtimeVersion: String,
    val downloadSize: Long = 0,
    val installedSize: Long = 0,
    val signature: String? = null,
    val buildTime: String = Instant.now().toString()
)

@Serializable
data class LayerImage(
    val name: String,
    val purpose: LayerPurpose,
    val containerImage: ContainerImage,
    val dependencies: List<String> = emptyList(),
    val priority: Int = 50,
    val checksum: String,
    val buildTime: String = Instant.now().toString()
)

@Serializable
data class PackageInfo(
    val name: String,
    val version: String,
    val architecture: String = "x86_64",
    val size: Long = 0,
    val checksum: String,
    val dependencies: List<String> = emptyList(),
    val origin: String = "unknown"
)

@Serializable
enum class ValidationMode {
    STRICT,  // Fail on any validation error
    WARN,    // Warn but continue on validation errors
    DISABLED // Skip validation entirely
}

@Serializable
enum class ImageSource {
    OSTREE,
    CONTAINER_REGISTRY,
    FLATPAK_REPO,
    LOCAL_BUILD
}

// ===== DSL Contexts =====

@HorizonOSDsl
class ReproducibleContext {
    var enabled: Boolean = true
    var strictMode: Boolean = false
    var verifyDigests: Boolean = true
    var lockfile: String = "/etc/horizonos/system.lock"
    var validationMode: ValidationMode = ValidationMode.WARN
    var signatureValidation: Boolean = true
    var allowUnsigned: Boolean = false
    
    private var pinnedBase: String? = null
    private var systemImage: SystemImage? = null
    
    /**
     * Pin system to specific OSTree commit
     */
    fun base(commit: String) {
        pinnedBase = commit
    }
    
    /**
     * Load system state from lockfile
     */
    fun fromLockfile(path: String) {
        lockfile = path
        // TODO: Implement lockfile loading
    }
    
    /**
     * Pin to specific system image
     */
    fun systemImage(block: SystemImageBuilder.() -> Unit) {
        systemImage = SystemImageBuilder().apply(block).build()
    }
    
    /**
     * Configure OSTree base image
     */
    fun ostree(ref: String, commit: String, block: OstreeImageBuilder.() -> Unit = {}) {
        val builder = OstreeImageBuilder(ref, commit).apply(block)
        val ostreeImage = builder.build()
        
        if (systemImage == null) {
            systemImage = SystemImage(
                base = ostreeImage,
                timestamp = Instant.now().toString()
            )
        } else {
            systemImage = systemImage!!.copy(base = ostreeImage)
        }
    }
    
    fun toConfig(): ReproducibleConfig {
        return ReproducibleConfig(
            enabled = enabled,
            strictMode = strictMode,
            verifyDigests = verifyDigests,
            lockfile = lockfile,
            pinnedBase = pinnedBase,
            systemImage = systemImage,
            validationMode = validationMode,
            signatureValidation = signatureValidation,
            allowUnsigned = allowUnsigned
        )
    }
}

@HorizonOSDsl
class SystemImageBuilder {
    var version: String = "1.0"
    var timestamp: String = Instant.now().toString()
    var signature: String? = null
    
    private var base: OstreeImage? = null
    private val containers = mutableListOf<ContainerImage>()
    private val flatpaks = mutableListOf<FlatpakImage>()
    private val layers = mutableListOf<LayerImage>()
    private val metadata = mutableMapOf<String, String>()
    
    /**
     * Configure base OSTree image
     */
    fun base(ref: String, commit: String, block: OstreeImageBuilder.() -> Unit = {}) {
        base = OstreeImageBuilder(ref, commit).apply(block).build()
    }
    
    /**
     * Add container image
     */
    fun container(name: String, image: String, digest: String, block: ContainerImageBuilder.() -> Unit = {}) {
        val builder = ContainerImageBuilder(name, image, digest).apply(block)
        containers.add(builder.build())
    }
    
    /**
     * Add Flatpak image
     */
    fun flatpak(id: String, commit: String, block: FlatpakImageBuilder.() -> Unit = {}) {
        val builder = FlatpakImageBuilder(id, commit).apply(block)
        flatpaks.add(builder.build())
    }
    
    /**
     * Add layer image
     */
    fun layer(name: String, purpose: LayerPurpose, block: LayerImageBuilder.() -> Unit) {
        val builder = LayerImageBuilder(name, purpose).apply(block)
        layers.add(builder.build())
    }
    
    /**
     * Add metadata
     */
    fun metadata(key: String, value: String) {
        metadata[key] = value
    }
    
    /**
     * Sign the system image
     */
    fun sign(signature: String) {
        this.signature = signature
    }
    
    fun build(): SystemImage {
        if (base == null) {
            throw IllegalStateException("System image must have a base OSTree image")
        }
        
        return SystemImage(
            version = version,
            timestamp = timestamp,
            base = base!!,
            containers = containers,
            flatpaks = flatpaks,
            layers = layers,
            signature = signature,
            metadata = metadata
        )
    }
}

@HorizonOSDsl
class OstreeImageBuilder(
    private val ref: String,
    private val commit: String
) {
    var version: String = "1.0"
    var digest: String = ""
    var url: String? = null
    var signature: String? = null
    var size: Long = 0
    var timestamp: String = Instant.now().toString()
    
    fun build(): OstreeImage {
        return OstreeImage(
            ref = ref,
            commit = commit,
            version = version,
            digest = digest,
            url = url,
            signature = signature,
            size = size,
            timestamp = timestamp
        )
    }
}

@HorizonOSDsl
class ContainerImageBuilder(
    private val name: String,
    private val image: String,
    private val digest: String
) {
    var tag: String = "latest"
    var runtime: ContainerRuntime = ContainerRuntime.DISTROBOX
    var purpose: ContainerPurpose = ContainerPurpose.CUSTOM
    var size: Long = 0
    var signature: String? = null
    var buildTime: String = Instant.now().toString()
    
    private val packages = mutableListOf<PackageInfo>()
    private val layers = mutableListOf<String>()
    
    /**
     * Add package information
     */
    fun pkg(name: String, version: String, block: PackageInfoBuilder.() -> Unit = {}) {
        val builder = PackageInfoBuilder(name, version).apply(block)
        packages.add(builder.build())
    }
    
    /**
     * Add layer information
     */
    fun layer(digest: String) {
        layers.add(digest)
    }
    
    fun build(): ContainerImage {
        return ContainerImage(
            name = name,
            image = image,
            tag = tag,
            digest = digest,
            runtime = runtime,
            purpose = purpose,
            packages = packages,
            size = size,
            layers = layers,
            signature = signature,
            buildTime = buildTime
        )
    }
}

@HorizonOSDsl
class FlatpakImageBuilder(
    private val id: String,
    private val commit: String
) {
    var version: String = ""
    var branch: String = "stable"
    var runtime: String = ""
    var runtimeVersion: String = ""
    var downloadSize: Long = 0
    var installedSize: Long = 0
    var signature: String? = null
    var buildTime: String = Instant.now().toString()
    
    fun build(): FlatpakImage {
        return FlatpakImage(
            id = id,
            version = version,
            branch = branch,
            commit = commit,
            runtime = runtime,
            runtimeVersion = runtimeVersion,
            downloadSize = downloadSize,
            installedSize = installedSize,
            signature = signature,
            buildTime = buildTime
        )
    }
}

@HorizonOSDsl
class LayerImageBuilder(
    private val name: String,
    private val purpose: LayerPurpose
) {
    var priority: Int = 50
    var checksum: String = ""
    var buildTime: String = Instant.now().toString()
    
    private val dependencies = mutableListOf<String>()
    private var containerImage: ContainerImage? = null
    
    /**
     * Add dependency
     */
    fun dependsOn(layer: String) {
        dependencies.add(layer)
    }
    
    /**
     * Configure container image
     */
    fun container(name: String, image: String, digest: String, block: ContainerImageBuilder.() -> Unit = {}) {
        val builder = ContainerImageBuilder(name, image, digest).apply(block)
        containerImage = builder.build()
    }
    
    fun build(): LayerImage {
        if (containerImage == null) {
            throw IllegalStateException("Layer image must have a container image")
        }
        
        return LayerImage(
            name = name,
            purpose = purpose,
            containerImage = containerImage!!,
            dependencies = dependencies,
            priority = priority,
            checksum = checksum,
            buildTime = buildTime
        )
    }
}

@HorizonOSDsl
class PackageInfoBuilder(
    private val name: String,
    private val version: String
) {
    var architecture: String = "x86_64"
    var size: Long = 0
    var checksum: String = ""
    var origin: String = "unknown"
    
    private val dependencies = mutableListOf<String>()
    
    /**
     * Add dependency
     */
    fun dependency(name: String) {
        dependencies.add(name)
    }
    
    /**
     * Add multiple dependencies
     */
    fun dependencies(vararg names: String) {
        dependencies.addAll(names)
    }
    
    fun build(): PackageInfo {
        return PackageInfo(
            name = name,
            version = version,
            architecture = architecture,
            size = size,
            checksum = checksum,
            dependencies = dependencies,
            origin = origin
        )
    }
}

// ===== Utility Functions =====

/**
 * Generate system image checksum
 */
fun generateSystemImageChecksum(image: SystemImage): String {
    // TODO: Implement checksum generation based on all image components
    return "sha256:placeholder"
}

/**
 * Validate system image integrity
 */
fun validateSystemImage(image: SystemImage): List<String> {
    val errors = mutableListOf<String>()
    
    // Validate base image
    if (image.base.ref.isBlank()) {
        errors.add("Base image ref cannot be empty")
    }
    
    if (image.base.commit.isBlank()) {
        errors.add("Base image commit cannot be empty")
    }
    
    // Validate container images
    for (container in image.containers) {
        if (container.digest.isBlank()) {
            errors.add("Container '${container.name}' must have a digest")
        }
        
        if (!container.digest.startsWith("sha256:")) {
            errors.add("Container '${container.name}' digest must be SHA256")
        }
        
        // Validate SHA256 format
        val sha256Regex = Regex("^sha256:[a-fA-F0-9]{64}$")
        if (!sha256Regex.matches(container.digest)) {
            errors.add("Container '${container.name}' has invalid SHA256 digest format")
        }
    }
    
    // Validate Flatpak images
    for (flatpak in image.flatpaks) {
        if (flatpak.commit.isBlank()) {
            errors.add("Flatpak '${flatpak.id}' must have a commit")
        }
    }
    
    return errors
}

/**
 * Compare two system images for differences
 */
fun compareSystemImages(old: SystemImage, new: SystemImage): SystemImageDiff {
    val baseChanged = old.base.commit != new.base.commit
    val containerChanges = compareContainerImages(old.containers, new.containers)
    val flatpakChanges = compareFlatpakImages(old.flatpaks, new.flatpaks)
    
    return SystemImageDiff(
        baseChanged = baseChanged,
        containerChanges = containerChanges,
        flatpakChanges = flatpakChanges
    )
}

private fun compareContainerImages(old: List<ContainerImage>, new: List<ContainerImage>): List<ContainerChange> {
    val changes = mutableListOf<ContainerChange>()
    val oldMap = old.associateBy { it.name }
    val newMap = new.associateBy { it.name }
    
    // Find added containers
    for (name in newMap.keys - oldMap.keys) {
        changes.add(ContainerChange(name, null, newMap[name]?.digest, ChangeType.ADDED))
    }
    
    // Find removed containers
    for (name in oldMap.keys - newMap.keys) {
        changes.add(ContainerChange(name, oldMap[name]?.digest, null, ChangeType.REMOVED))
    }
    
    // Find changed containers
    for (name in oldMap.keys intersect newMap.keys) {
        val oldDigest = oldMap[name]?.digest
        val newDigest = newMap[name]?.digest
        if (oldDigest != newDigest) {
            changes.add(ContainerChange(name, oldDigest, newDigest, ChangeType.UPDATED))
        }
    }
    
    return changes
}

private fun compareFlatpakImages(old: List<FlatpakImage>, new: List<FlatpakImage>): List<FlatpakChange> {
    val changes = mutableListOf<FlatpakChange>()
    val oldMap = old.associateBy { it.id }
    val newMap = new.associateBy { it.id }
    
    // Find added Flatpaks
    for (id in newMap.keys - oldMap.keys) {
        changes.add(FlatpakChange(id, null, newMap[id]?.commit, ChangeType.ADDED))
    }
    
    // Find removed Flatpaks
    for (id in oldMap.keys - newMap.keys) {
        changes.add(FlatpakChange(id, oldMap[id]?.commit, null, ChangeType.REMOVED))
    }
    
    // Find changed Flatpaks
    for (id in oldMap.keys intersect newMap.keys) {
        val oldCommit = oldMap[id]?.commit
        val newCommit = newMap[id]?.commit
        if (oldCommit != newCommit) {
            changes.add(FlatpakChange(id, oldCommit, newCommit, ChangeType.UPDATED))
        }
    }
    
    return changes
}

// ===== Change Tracking =====

@Serializable
data class SystemImageDiff(
    val baseChanged: Boolean,
    val containerChanges: List<ContainerChange>,
    val flatpakChanges: List<FlatpakChange>
) {
    fun hasChanges(): Boolean = baseChanged || containerChanges.isNotEmpty() || flatpakChanges.isNotEmpty()
}

@Serializable
data class ContainerChange(
    val name: String,
    val oldDigest: String?,
    val newDigest: String?,
    val type: ChangeType
)

@Serializable
data class FlatpakChange(
    val id: String,
    val oldCommit: String?,
    val newCommit: String?,
    val type: ChangeType
)

@Serializable
enum class ChangeType {
    ADDED, REMOVED, UPDATED
}