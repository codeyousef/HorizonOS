package org.horizonos.config.dsl.services

import kotlinx.serialization.Serializable
import org.horizonos.config.dsl.HorizonOSDsl

// ===== Message Queue Configuration =====

@HorizonOSDsl
class MessageQueueContext(private val type: MessageQueueType) {
    var enabled: Boolean = true
    var autoStart: Boolean = true
    var port: Int? = null
    var managementPort: Int? = null
    var configOverrides = mutableMapOf<String, String>()
    
    // Queue-specific configurations
    var rabbitmqConfig: RabbitMQConfig? = null
    var kafkaConfig: KafkaConfig? = null
    var natsConfig: NATSConfig? = null

    fun rabbitmq(block: RabbitMQContext.() -> Unit) {
        rabbitmqConfig = RabbitMQContext().apply(block).toConfig()
    }

    fun kafka(block: KafkaContext.() -> Unit) {
        kafkaConfig = KafkaContext().apply(block).toConfig()
    }

    fun nats(block: NATSContext.() -> Unit) {
        natsConfig = NATSContext().apply(block).toConfig()
    }

    fun config(key: String, value: String) {
        configOverrides[key] = value
    }

    fun toService(): MessageQueueService {
        return MessageQueueService(
            type = type,
            enabled = enabled,
            autoStart = autoStart,
            port = port ?: getDefaultPort(type),
            managementPort = managementPort ?: getDefaultManagementPort(type),
            configOverrides = configOverrides.toMap(),
            rabbitmqConfig = rabbitmqConfig,
            kafkaConfig = kafkaConfig,
            natsConfig = natsConfig
        )
    }

    private fun getDefaultPort(type: MessageQueueType): Int = when (type) {
        MessageQueueType.RABBITMQ -> 5672
        MessageQueueType.KAFKA -> 9092
        MessageQueueType.NATS -> 4222
        MessageQueueType.REDIS_STREAMS -> 6379
    }

    private fun getDefaultManagementPort(type: MessageQueueType): Int? = when (type) {
        MessageQueueType.RABBITMQ -> 15672
        MessageQueueType.KAFKA -> 9093
        MessageQueueType.NATS -> 8222
        MessageQueueType.REDIS_STREAMS -> null
    }
}

// ===== RabbitMQ Configuration =====

@HorizonOSDsl
class RabbitMQContext {
    var clusterEnabled: Boolean = false
    var clusterNodes = mutableListOf<String>()
    var memoryHighWatermark: Double = 0.4
    var diskFreeLimit: String = "2GB"
    var heartbeat: Int = 60
    var plugins = mutableListOf<String>()
    var users = mutableListOf<RabbitMQUser>()
    var vhosts = mutableListOf<String>()

    fun plugin(name: String) {
        plugins.add(name)
    }

    fun user(name: String, block: RabbitMQUserContext.() -> Unit) {
        users.add(RabbitMQUserContext(name).apply(block).toUser())
    }

    fun vhost(name: String) {
        vhosts.add(name)
    }

    fun clusterNode(node: String) {
        clusterNodes.add(node)
    }

    fun toConfig(): RabbitMQConfig {
        return RabbitMQConfig(
            clusterEnabled = clusterEnabled,
            clusterNodes = clusterNodes,
            memoryHighWatermark = memoryHighWatermark,
            diskFreeLimit = diskFreeLimit,
            heartbeat = heartbeat,
            plugins = plugins,
            users = users,
            vhosts = vhosts
        )
    }
}

@HorizonOSDsl
class RabbitMQUserContext(private val name: String) {
    var password: String = ""
    var tags = mutableListOf<String>()
    var permissions = mutableListOf<RabbitMQPermission>()

    fun tag(tag: String) {
        tags.add(tag)
    }

    fun permission(vhost: String, configure: String = ".*", write: String = ".*", read: String = ".*") {
        permissions.add(RabbitMQPermission(vhost, configure, write, read))
    }

    fun toUser(): RabbitMQUser {
        return RabbitMQUser(
            name = name,
            password = password,
            tags = tags,
            permissions = permissions
        )
    }
}

// ===== Kafka Configuration =====

@HorizonOSDsl
class KafkaContext {
    var brokerId: Int = 1
    var logRetentionHours: Int = 168
    var logRetentionBytes: Long = 1073741824
    var logSegmentBytes: Int = 1073741824
    var numPartitions: Int = 1
    var defaultReplicationFactor: Short = 1
    var minInsyncReplicas: Short = 1
    var zookeeperConnect: String = "localhost:2181"
    var listeners = mutableListOf<String>()
    var security: KafkaSecurity? = null

    fun listener(protocol: String, host: String, port: Int) {
        listeners.add("$protocol://$host:$port")
    }

    fun security(block: KafkaSecurityContext.() -> Unit) {
        security = KafkaSecurityContext().apply(block).toSecurity()
    }

    fun toConfig(): KafkaConfig {
        return KafkaConfig(
            brokerId = brokerId,
            logRetentionHours = logRetentionHours,
            logRetentionBytes = logRetentionBytes,
            logSegmentBytes = logSegmentBytes,
            numPartitions = numPartitions,
            defaultReplicationFactor = defaultReplicationFactor,
            minInsyncReplicas = minInsyncReplicas,
            zookeeperConnect = zookeeperConnect,
            listeners = listeners,
            security = security
        )
    }
}

@HorizonOSDsl
class KafkaSecurityContext {
    var protocol: String = "PLAINTEXT"
    var saslMechanism: String? = null
    var keystoreLocation: String? = null
    var keystorePassword: String? = null
    var truststoreLocation: String? = null
    var truststorePassword: String? = null

    fun toSecurity(): KafkaSecurity {
        return KafkaSecurity(
            protocol = protocol,
            saslMechanism = saslMechanism,
            keystoreLocation = keystoreLocation,
            keystorePassword = keystorePassword,
            truststoreLocation = truststoreLocation,
            truststorePassword = truststorePassword
        )
    }
}

// ===== NATS Configuration =====

@HorizonOSDsl
class NATSContext {
    var clusterEnabled: Boolean = false
    var clusterName: String = "my-cluster"
    var clusterRoutes = mutableListOf<String>()
    var jetStreamEnabled: Boolean = false
    var jetStreamConfig: NATSJetStreamConfig? = null
    var auth: NATSAuth? = null

    fun clusterRoute(route: String) {
        clusterRoutes.add(route)
    }

    fun jetstream(block: NATSJetStreamContext.() -> Unit) {
        jetStreamEnabled = true
        jetStreamConfig = NATSJetStreamContext().apply(block).toConfig()
    }

    fun auth(block: NATSAuthContext.() -> Unit) {
        auth = NATSAuthContext().apply(block).toAuth()
    }

    fun toConfig(): NATSConfig {
        return NATSConfig(
            clusterEnabled = clusterEnabled,
            clusterName = clusterName,
            clusterRoutes = clusterRoutes,
            jetStreamEnabled = jetStreamEnabled,
            jetStreamConfig = jetStreamConfig,
            auth = auth
        )
    }
}

@HorizonOSDsl
class NATSJetStreamContext {
    var maxMemory: String = "1GB"
    var maxStorage: String = "10GB"
    var compressOk: Boolean = true

    fun toConfig(): NATSJetStreamConfig {
        return NATSJetStreamConfig(
            maxMemory = maxMemory,
            maxStorage = maxStorage,
            compressOk = compressOk
        )
    }
}

@HorizonOSDsl
class NATSAuthContext {
    var users = mutableListOf<NATSUser>()
    var token: String? = null

    fun user(name: String, block: NATSUserContext.() -> Unit) {
        users.add(NATSUserContext(name).apply(block).toUser())
    }

    fun toAuth(): NATSAuth {
        return NATSAuth(
            users = users,
            token = token
        )
    }
}

@HorizonOSDsl
class NATSUserContext(private val name: String) {
    var password: String? = null
    var permissions: NATSPermissions? = null

    fun permissions(block: NATSPermissionsContext.() -> Unit) {
        permissions = NATSPermissionsContext().apply(block).toPermissions()
    }

    fun toUser(): NATSUser {
        return NATSUser(
            name = name,
            password = password,
            permissions = permissions
        )
    }
}

@HorizonOSDsl
class NATSPermissionsContext {
    var publish = mutableListOf<String>()
    var subscribe = mutableListOf<String>()

    fun publishTo(subject: String) {
        publish.add(subject)
    }

    fun subscribeTo(subject: String) {
        subscribe.add(subject)
    }

    fun toPermissions(): NATSPermissions {
        return NATSPermissions(
            publish = publish,
            subscribe = subscribe
        )
    }
}

// ===== Enums =====

@Serializable
enum class MessageQueueType {
    RABBITMQ, KAFKA, NATS, REDIS_STREAMS
}

// ===== Data Classes =====

@Serializable
data class MessageQueueService(
    val type: MessageQueueType,
    val enabled: Boolean,
    val autoStart: Boolean,
    val port: Int,
    val managementPort: Int?,
    val configOverrides: Map<String, String>,
    val rabbitmqConfig: RabbitMQConfig?,
    val kafkaConfig: KafkaConfig?,
    val natsConfig: NATSConfig?
)

@Serializable
data class RabbitMQConfig(
    val clusterEnabled: Boolean,
    val clusterNodes: List<String>,
    val memoryHighWatermark: Double,
    val diskFreeLimit: String,
    val heartbeat: Int,
    val plugins: List<String>,
    val users: List<RabbitMQUser>,
    val vhosts: List<String>
)

@Serializable
data class RabbitMQUser(
    val name: String,
    val password: String,
    val tags: List<String>,
    val permissions: List<RabbitMQPermission>
)

@Serializable
data class RabbitMQPermission(
    val vhost: String,
    val configure: String,
    val write: String,
    val read: String
)

@Serializable
data class KafkaConfig(
    val brokerId: Int,
    val logRetentionHours: Int,
    val logRetentionBytes: Long,
    val logSegmentBytes: Int,
    val numPartitions: Int,
    val defaultReplicationFactor: Short,
    val minInsyncReplicas: Short,
    val zookeeperConnect: String,
    val listeners: List<String>,
    val security: KafkaSecurity?
)

@Serializable
data class KafkaSecurity(
    val protocol: String,
    val saslMechanism: String?,
    val keystoreLocation: String?,
    val keystorePassword: String?,
    val truststoreLocation: String?,
    val truststorePassword: String?
)

@Serializable
data class NATSConfig(
    val clusterEnabled: Boolean,
    val clusterName: String,
    val clusterRoutes: List<String>,
    val jetStreamEnabled: Boolean,
    val jetStreamConfig: NATSJetStreamConfig?,
    val auth: NATSAuth?
)

@Serializable
data class NATSJetStreamConfig(
    val maxMemory: String,
    val maxStorage: String,
    val compressOk: Boolean
)

@Serializable
data class NATSAuth(
    val users: List<NATSUser>,
    val token: String?
)

@Serializable
data class NATSUser(
    val name: String,
    val password: String?,
    val permissions: NATSPermissions?
)

@Serializable
data class NATSPermissions(
    val publish: List<String>,
    val subscribe: List<String>
)