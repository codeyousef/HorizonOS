package org.horizonos.config.dsl.services

import kotlinx.serialization.Serializable
import org.horizonos.config.dsl.HorizonOSDsl

// ===== Container Configuration =====

@HorizonOSDsl
class ContainerContext(private val runtime: ContainerRuntime) {
    var enabled: Boolean = true
    var autoStart: Boolean = true
    var rootless: Boolean = false
    var storageDriver: String? = null
    var registries = mutableListOf<ContainerRegistry>()
    var networks = mutableListOf<ContainerNetwork>()
    var volumes = mutableListOf<ContainerVolume>()
    var services = mutableListOf<ContainerizedService>()
    var buildConfig: ContainerBuildConfig? = null

    fun registry(url: String, block: ContainerRegistryContext.() -> Unit = {}) {
        registries.add(ContainerRegistryContext(url).apply(block).toRegistry())
    }

    fun network(name: String, block: ContainerNetworkContext.() -> Unit) {
        networks.add(ContainerNetworkContext(name).apply(block).toNetwork())
    }

    fun volume(name: String, block: ContainerVolumeContext.() -> Unit) {
        volumes.add(ContainerVolumeContext(name).apply(block).toVolume())
    }

    fun service(name: String, block: ContainerizedServiceContext.() -> Unit) {
        services.add(ContainerizedServiceContext(name).apply(block).toService())
    }

    fun build(block: ContainerBuildContext.() -> Unit) {
        buildConfig = ContainerBuildContext().apply(block).toConfig()
    }

    fun toService(): ContainerService {
        return ContainerService(
            runtime = runtime,
            enabled = enabled,
            autoStart = autoStart,
            rootless = rootless,
            storageDriver = storageDriver,
            registries = registries,
            networks = networks,
            volumes = volumes,
            services = services,
            buildConfig = buildConfig
        )
    }
}

@HorizonOSDsl
class ContainerRegistryContext(private val url: String) {
    var username: String? = null
    var password: String? = null
    var insecure: Boolean = false

    fun toRegistry(): ContainerRegistry {
        return ContainerRegistry(
            url = url,
            username = username,
            password = password,
            insecure = insecure
        )
    }
}

@HorizonOSDsl
class ContainerNetworkContext(private val name: String) {
    var driver: String = "bridge"
    var subnet: String? = null
    var gateway: String? = null
    var ipRange: String? = null

    fun toNetwork(): ContainerNetwork {
        return ContainerNetwork(
            name = name,
            driver = driver,
            subnet = subnet,
            gateway = gateway,
            ipRange = ipRange
        )
    }
}

@HorizonOSDsl
class ContainerVolumeContext(private val name: String) {
    var driver: String = "local"
    var mountPoint: String? = null
    var options = mutableMapOf<String, String>()

    fun option(key: String, value: String) {
        options[key] = value
    }

    fun toVolume(): ContainerVolume {
        return ContainerVolume(
            name = name,
            driver = driver,
            mountPoint = mountPoint,
            options = options.toMap()
        )
    }
}

@HorizonOSDsl
class ContainerizedServiceContext(private val name: String) {
    var image: String? = null
    var tag: String = "latest"
    var enabled: Boolean = true
    var autoRestart: Boolean = true
    var restartPolicy: RestartPolicy = RestartPolicy.UNLESS_STOPPED
    var ports = mutableListOf<PortMapping>()
    var volumes = mutableListOf<VolumeMount>()
    var environment = mutableMapOf<String, String>()
    var networks = mutableListOf<String>()
    var dependencies = mutableListOf<String>()
    var healthCheck: HealthCheck? = null
    var resources: ResourceLimits? = null

    fun port(host: Int, container: Int, protocol: String = "tcp") {
        ports.add(PortMapping(host, container, protocol))
    }

    fun volume(host: String, container: String, mode: String = "rw") {
        volumes.add(VolumeMount(host, container, mode))
    }

    fun env(key: String, value: String) {
        environment[key] = value
    }

    fun network(name: String) {
        networks.add(name)
    }

    fun dependsOn(service: String) {
        dependencies.add(service)
    }

    fun healthCheck(block: HealthCheckContext.() -> Unit) {
        healthCheck = HealthCheckContext().apply(block).toHealthCheck()
    }

    fun resources(block: ResourceLimitsContext.() -> Unit) {
        resources = ResourceLimitsContext().apply(block).toLimits()
    }

    fun toService(): ContainerizedService {
        return ContainerizedService(
            name = name,
            image = image,
            tag = tag,
            enabled = enabled,
            autoRestart = autoRestart,
            restartPolicy = restartPolicy,
            ports = ports,
            volumes = volumes,
            environment = environment.toMap(),
            networks = networks,
            dependencies = dependencies,
            healthCheck = healthCheck,
            resources = resources
        )
    }
}

@HorizonOSDsl
class HealthCheckContext {
    var test: String? = null
    var interval: String = "30s"
    var timeout: String = "3s"
    var retries: Int = 3
    var startPeriod: String = "0s"

    fun toHealthCheck(): HealthCheck {
        return HealthCheck(
            test = test,
            interval = interval,
            timeout = timeout,
            retries = retries,
            startPeriod = startPeriod
        )
    }
}

@HorizonOSDsl
class ResourceLimitsContext {
    var cpus: String? = null
    var memory: String? = null
    var cpuShares: Int? = null
    var memoryReservation: String? = null

    fun toLimits(): ResourceLimits {
        return ResourceLimits(
            cpus = cpus,
            memory = memory,
            cpuShares = cpuShares,
            memoryReservation = memoryReservation
        )
    }
}

@HorizonOSDsl
class ContainerBuildContext {
    var dockerfile: String? = null
    var context: String? = null
    var buildArgs = mutableMapOf<String, String>()
    var target: String? = null
    var noCache: Boolean = false

    fun arg(key: String, value: String) {
        buildArgs[key] = value
    }

    fun toConfig(): ContainerBuildConfig {
        return ContainerBuildConfig(
            dockerfile = dockerfile,
            context = context,
            buildArgs = buildArgs.toMap(),
            target = target,
            noCache = noCache
        )
    }
}

// ===== Enums =====

@Serializable
enum class ContainerRuntime {
    DOCKER, PODMAN, CONTAINERD
}

@Serializable
enum class RestartPolicy {
    NO, ALWAYS, ON_FAILURE, UNLESS_STOPPED
}

// ===== Data Classes =====

@Serializable
data class ContainerService(
    val runtime: ContainerRuntime,
    val enabled: Boolean,
    val autoStart: Boolean,
    val rootless: Boolean,
    val storageDriver: String?,
    val registries: List<ContainerRegistry>,
    val networks: List<ContainerNetwork>,
    val volumes: List<ContainerVolume>,
    val services: List<ContainerizedService>,
    val buildConfig: ContainerBuildConfig?
)

@Serializable
data class ContainerRegistry(
    val url: String,
    val username: String?,
    val password: String?,
    val insecure: Boolean
)

@Serializable
data class ContainerNetwork(
    val name: String,
    val driver: String,
    val subnet: String?,
    val gateway: String?,
    val ipRange: String?
)

@Serializable
data class ContainerVolume(
    val name: String,
    val driver: String,
    val mountPoint: String?,
    val options: Map<String, String>
)

@Serializable
data class ContainerizedService(
    val name: String,
    val image: String?,
    val tag: String,
    val enabled: Boolean,
    val autoRestart: Boolean,
    val restartPolicy: RestartPolicy,
    val ports: List<PortMapping>,
    val volumes: List<VolumeMount>,
    val environment: Map<String, String>,
    val networks: List<String>,
    val dependencies: List<String>,
    val healthCheck: HealthCheck?,
    val resources: ResourceLimits?
)

@Serializable
data class PortMapping(
    val host: Int,
    val container: Int,
    val protocol: String
)

@Serializable
data class VolumeMount(
    val host: String,
    val container: String,
    val mode: String
)

@Serializable
data class HealthCheck(
    val test: String?,
    val interval: String,
    val timeout: String,
    val retries: Int,
    val startPeriod: String
)

@Serializable
data class ResourceLimits(
    val cpus: String?,
    val memory: String?,
    val cpuShares: Int?,
    val memoryReservation: String?
)

@Serializable
data class ContainerBuildConfig(
    val dockerfile: String?,
    val context: String?,
    val buildArgs: Map<String, String>,
    val target: String?,
    val noCache: Boolean
)