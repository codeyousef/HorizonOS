package org.horizonos.config.dsl

import kotlinx.serialization.Serializable
import org.horizonos.config.dsl.services.*

/**
 * Enhanced Service Configuration DSL for HorizonOS
 * 
 * Provides comprehensive service management capabilities for HorizonOS systems, including
 * database services, web servers, container services, message queues, monitoring systems,
 * and systemd unit management. This module enables declarative service configuration with
 * automatic dependency management and health monitoring.
 * 
 * ## Service Categories:
 * - **Database Services**: PostgreSQL, MySQL, Redis, MongoDB, ClickHouse
 * - **Web Servers**: Apache, Nginx, Caddy with SSL/TLS configuration
 * - **Container Services**: Docker, Podman, LXC container management
 * - **Message Queues**: RabbitMQ, Apache Kafka, Redis Pub/Sub
 * - **Monitoring**: Prometheus, Grafana, monitoring agent configuration
 * - **Systemd Units**: Custom systemd service unit management
 * 
 * ## Key Features:
 * - **Automatic Dependencies**: Service dependencies are resolved automatically
 * - **Health Monitoring**: Built-in health checks and monitoring
 * - **SSL/TLS Support**: Automatic certificate management and renewal
 * - **Resource Management**: CPU, memory, and storage resource limits
 * - **Backup Integration**: Automated backup configuration
 * - **Load Balancing**: Multi-instance service load balancing
 * 
 * ## Basic Usage:
 * ```kotlin
 * enhancedServices {
 *     database(DatabaseType.POSTGRESQL) {
 *         name = "main-db"
 *         version = "15"
 *         port = 5432
 *         
 *         authentication {
 *             method = AuthMethod.PASSWORD
 *             users {
 *                 user("app_user") {
 *                     password = "secure_password"
 *                     databases = listOf("app_db")
 *                 }
 *             }
 *         }
 *         
 *         backup {
 *             enabled = true
 *             schedule = "0 2 * * *"
 *             retention = 7
 *         }
 *     }
 *     
 *     webServer(WebServerType.NGINX) {
 *         name = "web-frontend"
 *         port = 80
 *         
 *         ssl {
 *             enabled = true
 *             certificate = "/etc/ssl/certs/server.crt"
 *             privateKey = "/etc/ssl/private/server.key"
 *         }
 *         
 *         virtualHost("example.com") {
 *             documentRoot = "/var/www/html"
 *             index = listOf("index.html", "index.php")
 *         }
 *     }
 *     
 *     monitoring(MonitoringType.PROMETHEUS) {
 *         name = "metrics-server"
 *         port = 9090
 *         scrapeInterval = "15s"
 *         
 *         targets {
 *             target("localhost:8080") {
 *                 job = "web-app"
 *                 metrics_path = "/metrics"
 *             }
 *         }
 *     }
 * }
 * ```
 * 
 * ## Service Lifecycle:
 * Services are managed through their complete lifecycle:
 * 1. **Configuration**: Declarative service configuration
 * 2. **Deployment**: Automatic service deployment and startup
 * 3. **Monitoring**: Continuous health and performance monitoring
 * 4. **Scaling**: Automatic or manual service scaling
 * 5. **Backup**: Automated data backup and recovery
 * 6. **Updates**: Safe service updates with rollback capability
 * 
 * @since 1.0
 * @see [DatabaseService] for database service configuration
 * @see [WebServerService] for web server configuration
 * @see [MonitoringService] for monitoring system configuration
 * @see [Containers] for containerized service deployment
 * @see [SystemContainer] for container-based services
 * @see [Security] for service security configuration
 * @see [Network] for network service configuration
 * @see [Storage] for storage service configuration
 * @see [horizonOS] for main system configuration entry point
 */

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

    fun container(runtime: org.horizonos.config.dsl.services.ContainerRuntime, block: ContainerContext.() -> Unit) {
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