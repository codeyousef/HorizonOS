package org.horizonos.config.dsl

import kotlinx.serialization.Serializable
import org.horizonos.config.dsl.services.*

// ===== Enhanced Service Configuration DSL =====

@HorizonOSDsl
class EnhancedServicesContext {
    internal val services = mutableListOf<EnhancedService>()
    internal val databases = mutableListOf<DatabaseService>()
    internal val webServers = mutableListOf<WebServerService>()
    internal val containers = mutableListOf<ContainerService>()
    internal val messageQueues = mutableListOf<MessageQueueService>()
    internal val monitoring = mutableListOf<MonitoringService>()
    internal val systemdUnits = mutableListOf<SystemdUnit>()

    fun database(type: DatabaseType, block: DatabaseContext.() -> Unit) {
        databases.add(DatabaseContext(type).apply(block).toService())
    }

    fun webServer(type: WebServerType, block: WebServerContext.() -> Unit) {
        webServers.add(WebServerContext(type).apply(block).toService())
    }

    fun container(runtime: ContainerRuntime, block: ContainerContext.() -> Unit) {
        containers.add(ContainerContext(runtime).apply(block).toService())
    }

    fun messageQueue(type: MessageQueueType, block: MessageQueueContext.() -> Unit) {
        messageQueues.add(MessageQueueContext(type).apply(block).toService())
    }

    fun monitoring(type: MonitoringType, block: MonitoringContext.() -> Unit) {
        monitoring.add(MonitoringContext(type).apply(block).toService())
    }

    fun systemdUnit(name: String, unitType: SystemdUnitType, block: SystemdUnitContext.() -> Unit) {
        systemdUnits.add(SystemdUnitContext(name, unitType).apply(block).toUnit())
    }

    fun toConfig(): EnhancedServicesConfig {
        return EnhancedServicesConfig(
            databases = databases,
            webServers = webServers,
            containers = containers,
            messageQueues = messageQueues,
            monitoring = monitoring,
            systemdUnits = systemdUnits
        )
    }
}

// ===== Data Classes =====

@Serializable
data class EnhancedServicesConfig(
    val databases: List<DatabaseService> = emptyList(),
    val webServers: List<WebServerService> = emptyList(),
    val containers: List<ContainerService> = emptyList(),
    val messageQueues: List<MessageQueueService> = emptyList(),
    val monitoring: List<MonitoringService> = emptyList(),
    val systemdUnits: List<SystemdUnit> = emptyList()
)

@Serializable
data class EnhancedService(
    val name: String,
    val type: String,
    val enabled: Boolean,
    val autoStart: Boolean,
    val config: Map<String, String> = emptyMap()
)