package org.horizonos.config.dsl.development.containers

import kotlinx.serialization.Serializable
import org.horizonos.config.dsl.HorizonOSDsl
import org.horizonos.config.dsl.services.PortMapping
import org.horizonos.config.dsl.services.VolumeMount

// ===== Container Development Environment =====

@HorizonOSDsl
class ContainerDevContext(private val name: String) {
    var enabled: Boolean = true
    var image: String = ""
    var tag: String = "latest"
    val ports = mutableListOf<PortMapping>()
    val volumes = mutableListOf<VolumeMount>()
    val environment = mutableMapOf<String, String>()
    val labels = mutableMapOf<String, String>()
    var workdir: String? = null
    var user: String? = null
    val capabilities = mutableListOf<String>()
    val devices = mutableListOf<String>()

    fun port(host: Int, container: Int, protocol: String = "tcp") {
        ports.add(PortMapping(host, container, protocol))
    }

    fun volume(host: String, container: String, readOnly: Boolean = false) {
        volumes.add(VolumeMount(host, container, if (readOnly) "ro" else "rw"))
    }

    fun env(key: String, value: String) {
        environment[key] = value
    }

    fun label(key: String, value: String) {
        labels[key] = value
    }

    fun capability(cap: String) {
        capabilities.add(cap)
    }

    fun device(dev: String) {
        devices.add(dev)
    }

    fun toEnvironment(): ContainerDevEnvironment {
        return ContainerDevEnvironment(
            name = name,
            enabled = enabled,
            image = image,
            tag = tag,
            ports = ports,
            volumes = volumes,
            environment = environment,
            labels = labels,
            workdir = workdir,
            user = user,
            capabilities = capabilities,
            devices = devices
        )
    }
}

@Serializable
data class ContainerDevEnvironment(
    val name: String,
    val enabled: Boolean,
    val image: String,
    val tag: String,
    val ports: List<PortMapping>,
    val volumes: List<VolumeMount>,
    val environment: Map<String, String>,
    val labels: Map<String, String>,
    val workdir: String?,
    val user: String?,
    val capabilities: List<String>,
    val devices: List<String>
)