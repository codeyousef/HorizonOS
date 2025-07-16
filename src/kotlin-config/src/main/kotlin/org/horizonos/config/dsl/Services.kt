package org.horizonos.config.dsl

import kotlinx.serialization.Serializable

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

    fun systemdUnit(name: String, block: SystemdUnitContext.() -> Unit) {
        systemdUnits.add(SystemdUnitContext(name).apply(block).toUnit())
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

// ===== Database Configuration =====

@HorizonOSDsl
class DatabaseContext(private val type: DatabaseType) {
    var enabled: Boolean = true
    var autoStart: Boolean = true
    var version: String? = null
    var port: Int? = null
    var dataDirectory: String? = null
    var configOverrides = mutableMapOf<String, String>()
    var users = mutableListOf<DatabaseUser>()
    var databases = mutableListOf<DatabaseSchema>()
    
    // PostgreSQL specific
    var postgresConfig: PostgresConfig? = null
    
    // MySQL specific
    var mysqlConfig: MySQLConfig? = null
    
    // Redis specific
    var redisConfig: RedisConfig? = null

    fun postgres(block: PostgresContext.() -> Unit) {
        postgresConfig = PostgresContext().apply(block).toConfig()
    }

    fun mysql(block: MySQLContext.() -> Unit) {
        mysqlConfig = MySQLContext().apply(block).toConfig()
    }

    fun redis(block: RedisContext.() -> Unit) {
        redisConfig = RedisContext().apply(block).toConfig()
    }

    fun user(name: String, block: DatabaseUserContext.() -> Unit) {
        users.add(DatabaseUserContext(name).apply(block).toUser())
    }

    fun database(name: String, block: DatabaseSchemaContext.() -> Unit) {
        databases.add(DatabaseSchemaContext(name).apply(block).toSchema())
    }

    fun config(key: String, value: String) {
        configOverrides[key] = value
    }

    fun toService(): DatabaseService {
        return DatabaseService(
            type = type,
            enabled = enabled,
            autoStart = autoStart,
            version = version,
            port = port ?: getDefaultPort(type),
            dataDirectory = dataDirectory ?: getDefaultDataDirectory(type),
            configOverrides = configOverrides.toMap(),
            users = users,
            databases = databases,
            postgresConfig = postgresConfig,
            mysqlConfig = mysqlConfig,
            redisConfig = redisConfig
        )
    }

    private fun getDefaultPort(type: DatabaseType): Int = when (type) {
        DatabaseType.POSTGRESQL -> 5432
        DatabaseType.MYSQL -> 3306
        DatabaseType.REDIS -> 6379
        DatabaseType.MONGODB -> 27017
        DatabaseType.SQLITE -> 0
    }

    private fun getDefaultDataDirectory(type: DatabaseType): String = when (type) {
        DatabaseType.POSTGRESQL -> "/var/lib/postgresql/data"
        DatabaseType.MYSQL -> "/var/lib/mysql"
        DatabaseType.REDIS -> "/var/lib/redis"
        DatabaseType.MONGODB -> "/var/lib/mongodb"
        DatabaseType.SQLITE -> "/var/lib/sqlite"
    }
}

@HorizonOSDsl
class PostgresContext {
    var maxConnections: Int = 100
    var sharedBuffers: String = "128MB"
    var effectiveCacheSize: String = "4GB"
    var maintenanceWorkMem: String = "64MB"
    var checkpointSegments: Int = 32
    var checkpointCompletionTarget: Double = 0.7
    var walBuffers: String = "16MB"
    var defaultStatisticsTarget: Int = 100
    var logMinDurationStatement: Int = -1
    var logConnections: Boolean = false
    var logDisconnections: Boolean = false
    var logLockWaits: Boolean = false
    var logStatement: String = "none"
    var logLinePrefix: String = "%t "
    var extensions = mutableListOf<String>()

    fun extension(name: String) {
        extensions.add(name)
    }

    fun toConfig(): PostgresConfig {
        return PostgresConfig(
            maxConnections = maxConnections,
            sharedBuffers = sharedBuffers,
            effectiveCacheSize = effectiveCacheSize,
            maintenanceWorkMem = maintenanceWorkMem,
            checkpointSegments = checkpointSegments,
            checkpointCompletionTarget = checkpointCompletionTarget,
            walBuffers = walBuffers,
            defaultStatisticsTarget = defaultStatisticsTarget,
            logMinDurationStatement = logMinDurationStatement,
            logConnections = logConnections,
            logDisconnections = logDisconnections,
            logLockWaits = logLockWaits,
            logStatement = logStatement,
            logLinePrefix = logLinePrefix,
            extensions = extensions
        )
    }
}

@HorizonOSDsl
class MySQLContext {
    var maxConnections: Int = 151
    var innodbBufferPoolSize: String = "128M"
    var innodbLogFileSize: String = "48M"
    var innodbFilePerTable: Boolean = true
    var queryCache: Boolean = false
    var queryCacheSize: String = "0"
    var tmpTableSize: String = "16M"
    var maxHeapTableSize: String = "16M"
    var slowQueryLog: Boolean = false
    var longQueryTime: Int = 10
    var binlogFormat: String = "ROW"
    var serverCharset: String = "utf8mb4"
    var serverCollation: String = "utf8mb4_unicode_ci"

    fun toConfig(): MySQLConfig {
        return MySQLConfig(
            maxConnections = maxConnections,
            innodbBufferPoolSize = innodbBufferPoolSize,
            innodbLogFileSize = innodbLogFileSize,
            innodbFilePerTable = innodbFilePerTable,
            queryCache = queryCache,
            queryCacheSize = queryCacheSize,
            tmpTableSize = tmpTableSize,
            maxHeapTableSize = maxHeapTableSize,
            slowQueryLog = slowQueryLog,
            longQueryTime = longQueryTime,
            binlogFormat = binlogFormat,
            serverCharset = serverCharset,
            serverCollation = serverCollation
        )
    }
}

@HorizonOSDsl
class RedisContext {
    var maxMemory: String = "256mb"
    var maxMemoryPolicy: String = "allkeys-lru"
    var persistenceMode: RedisPersistence = RedisPersistence.RDB
    var rdbSavePolicy = listOf("900 1", "300 10", "60 10000")
    var aofRewritePolicy: Boolean = true
    var tcpKeepalive: Int = 300
    var timeout: Int = 0
    var databases: Int = 16
    var password: String? = null
    var requirepass: Boolean = false
    var masterAuth: String? = null
    var clustering: Boolean = false
    var replication: RedisReplicationConfig? = null

    fun replication(block: RedisReplicationContext.() -> Unit) {
        replication = RedisReplicationContext().apply(block).toConfig()
    }

    fun toConfig(): RedisConfig {
        return RedisConfig(
            maxMemory = maxMemory,
            maxMemoryPolicy = maxMemoryPolicy,
            persistenceMode = persistenceMode,
            rdbSavePolicy = rdbSavePolicy,
            aofRewritePolicy = aofRewritePolicy,
            tcpKeepalive = tcpKeepalive,
            timeout = timeout,
            databases = databases,
            password = password,
            requirepass = requirepass,
            masterAuth = masterAuth,
            clustering = clustering,
            replication = replication
        )
    }
}

@HorizonOSDsl
class RedisReplicationContext {
    var masterHost: String = ""
    var masterPort: Int = 6379
    var masterTimeout: Int = 60
    var replicationBacklogSize: String = "1mb"
    var replicationDisklessSync: Boolean = false

    fun toConfig(): RedisReplicationConfig {
        return RedisReplicationConfig(
            masterHost = masterHost,
            masterPort = masterPort,
            masterTimeout = masterTimeout,
            replicationBacklogSize = replicationBacklogSize,
            replicationDisklessSync = replicationDisklessSync
        )
    }
}

@HorizonOSDsl
class DatabaseUserContext(private val name: String) {
    var password: String? = null
    var privileges = mutableListOf<String>()
    var databases = mutableListOf<String>()
    var host: String = "localhost"

    fun privilege(privilege: String) {
        privileges.add(privilege)
    }

    fun database(database: String) {
        databases.add(database)
    }

    fun toUser(): DatabaseUser {
        return DatabaseUser(
            name = name,
            password = password,
            privileges = privileges,
            databases = databases,
            host = host
        )
    }
}

@HorizonOSDsl
class DatabaseSchemaContext(private val name: String) {
    var charset: String = "utf8mb4"
    var collation: String = "utf8mb4_unicode_ci"
    var owner: String? = null

    fun toSchema(): DatabaseSchema {
        return DatabaseSchema(
            name = name,
            charset = charset,
            collation = collation,
            owner = owner
        )
    }
}

// ===== Web Server Configuration =====

@HorizonOSDsl
class WebServerContext(private val type: WebServerType) {
    var enabled: Boolean = true
    var autoStart: Boolean = true
    var port: Int = 80
    var sslPort: Int = 443
    var serverName: String? = null
    var documentRoot: String? = null
    var logLevel: String = "warn"
    var accessLog: String? = null
    var errorLog: String? = null
    var enableSSL: Boolean = false
    var sslCertificate: String? = null
    var sslCertificateKey: String? = null
    var virtualHosts = mutableListOf<VirtualHost>()
    var modules = mutableListOf<String>()
    var configOverrides = mutableMapOf<String, String>()
    
    // Nginx specific
    var nginxConfig: NginxConfig? = null
    
    // Apache specific
    var apacheConfig: ApacheConfig? = null
    
    // Caddy specific
    var caddyConfig: CaddyConfig? = null

    fun nginx(block: NginxContext.() -> Unit) {
        nginxConfig = NginxContext().apply(block).toConfig()
    }

    fun apache(block: ApacheContext.() -> Unit) {
        apacheConfig = ApacheContext().apply(block).toConfig()
    }

    fun caddy(block: CaddyContext.() -> Unit) {
        caddyConfig = CaddyContext().apply(block).toConfig()
    }

    fun virtualHost(serverName: String, block: VirtualHostContext.() -> Unit) {
        virtualHosts.add(VirtualHostContext(serverName).apply(block).toVirtualHost())
    }

    fun module(name: String) {
        modules.add(name)
    }

    fun config(key: String, value: String) {
        configOverrides[key] = value
    }

    fun toService(): WebServerService {
        return WebServerService(
            type = type,
            enabled = enabled,
            autoStart = autoStart,
            port = port,
            sslPort = sslPort,
            serverName = serverName,
            documentRoot = documentRoot ?: getDefaultDocumentRoot(type),
            logLevel = logLevel,
            accessLog = accessLog,
            errorLog = errorLog,
            enableSSL = enableSSL,
            sslCertificate = sslCertificate,
            sslCertificateKey = sslCertificateKey,
            virtualHosts = virtualHosts,
            modules = modules,
            configOverrides = configOverrides.toMap(),
            nginxConfig = nginxConfig,
            apacheConfig = apacheConfig,
            caddyConfig = caddyConfig
        )
    }

    private fun getDefaultDocumentRoot(type: WebServerType): String = when (type) {
        WebServerType.NGINX -> "/usr/share/nginx/html"
        WebServerType.APACHE -> "/var/www/html"
        WebServerType.CADDY -> "/var/www"
        WebServerType.LIGHTTPD -> "/var/www/html"
    }
}

@HorizonOSDsl
class NginxContext {
    var workerProcesses: String = "auto"
    var workerConnections: Int = 1024
    var keepaliveTimeout: Int = 65
    var clientMaxBodySize: String = "1m"
    var gzipCompression: Boolean = true
    var gzipTypes = listOf("text/plain", "text/css", "application/json", "application/javascript")
    var upstreams = mutableListOf<NginxUpstream>()
    var rateLimiting: NginxRateLimiting? = null

    fun upstream(name: String, block: NginxUpstreamContext.() -> Unit) {
        upstreams.add(NginxUpstreamContext(name).apply(block).toUpstream())
    }

    fun rateLimit(block: NginxRateLimitingContext.() -> Unit) {
        rateLimiting = NginxRateLimitingContext().apply(block).toRateLimit()
    }

    fun toConfig(): NginxConfig {
        return NginxConfig(
            workerProcesses = workerProcesses,
            workerConnections = workerConnections,
            keepaliveTimeout = keepaliveTimeout,
            clientMaxBodySize = clientMaxBodySize,
            gzipCompression = gzipCompression,
            gzipTypes = gzipTypes,
            upstreams = upstreams,
            rateLimiting = rateLimiting
        )
    }
}

@HorizonOSDsl
class NginxUpstreamContext(private val name: String) {
    var servers = mutableListOf<String>()
    var loadBalancing: LoadBalancingMethod = LoadBalancingMethod.ROUND_ROBIN
    var healthCheck: Boolean = false

    fun server(address: String) {
        servers.add(address)
    }

    fun toUpstream(): NginxUpstream {
        return NginxUpstream(
            name = name,
            servers = servers,
            loadBalancing = loadBalancing,
            healthCheck = healthCheck
        )
    }
}

@HorizonOSDsl
class NginxRateLimitingContext {
    var zones = mutableListOf<RateLimitZone>()

    fun zone(name: String, block: RateLimitZoneContext.() -> Unit) {
        zones.add(RateLimitZoneContext(name).apply(block).toZone())
    }

    fun toRateLimit(): NginxRateLimiting {
        return NginxRateLimiting(zones = zones)
    }
}

@HorizonOSDsl
class RateLimitZoneContext(private val name: String) {
    var key: String = "\$binary_remote_addr"
    var size: String = "10m"
    var rate: String = "1r/s"

    fun toZone(): RateLimitZone {
        return RateLimitZone(
            name = name,
            key = key,
            size = size,
            rate = rate
        )
    }
}

@HorizonOSDsl
class ApacheContext {
    var serverTokens: String = "Prod"
    var keepAlive: Boolean = true
    var maxKeepAliveRequests: Int = 100
    var keepAliveTimeout: Int = 5
    var preforkConfig: ApachePreforkConfig? = null
    var workerConfig: ApacheWorkerConfig? = null
    var eventConfig: ApacheEventConfig? = null

    fun prefork(block: ApachePreforkContext.() -> Unit) {
        preforkConfig = ApachePreforkContext().apply(block).toConfig()
    }

    fun worker(block: ApacheWorkerContext.() -> Unit) {
        workerConfig = ApacheWorkerContext().apply(block).toConfig()
    }

    fun event(block: ApacheEventContext.() -> Unit) {
        eventConfig = ApacheEventContext().apply(block).toConfig()
    }

    fun toConfig(): ApacheConfig {
        return ApacheConfig(
            serverTokens = serverTokens,
            keepAlive = keepAlive,
            maxKeepAliveRequests = maxKeepAliveRequests,
            keepAliveTimeout = keepAliveTimeout,
            preforkConfig = preforkConfig,
            workerConfig = workerConfig,
            eventConfig = eventConfig
        )
    }
}

@HorizonOSDsl
class ApachePreforkContext {
    var startServers: Int = 8
    var minSpareServers: Int = 5
    var maxSpareServers: Int = 20
    var serverLimit: Int = 256
    var maxRequestWorkers: Int = 256

    fun toConfig(): ApachePreforkConfig {
        return ApachePreforkConfig(
            startServers = startServers,
            minSpareServers = minSpareServers,
            maxSpareServers = maxSpareServers,
            serverLimit = serverLimit,
            maxRequestWorkers = maxRequestWorkers
        )
    }
}

@HorizonOSDsl
class ApacheWorkerContext {
    var startServers: Int = 3
    var maxRequestWorkers: Int = 400
    var minSpareThreads: Int = 25
    var maxSpareThreads: Int = 75
    var threadLimit: Int = 64
    var threadsPerChild: Int = 25

    fun toConfig(): ApacheWorkerConfig {
        return ApacheWorkerConfig(
            startServers = startServers,
            maxRequestWorkers = maxRequestWorkers,
            minSpareThreads = minSpareThreads,
            maxSpareThreads = maxSpareThreads,
            threadLimit = threadLimit,
            threadsPerChild = threadsPerChild
        )
    }
}

@HorizonOSDsl
class ApacheEventContext {
    var startServers: Int = 3
    var maxRequestWorkers: Int = 400
    var minSpareThreads: Int = 25
    var maxSpareThreads: Int = 75
    var threadLimit: Int = 64
    var threadsPerChild: Int = 25
    var asyncRequestWorkerFactor: Int = 2

    fun toConfig(): ApacheEventConfig {
        return ApacheEventConfig(
            startServers = startServers,
            maxRequestWorkers = maxRequestWorkers,
            minSpareThreads = minSpareThreads,
            maxSpareThreads = maxSpareThreads,
            threadLimit = threadLimit,
            threadsPerChild = threadsPerChild,
            asyncRequestWorkerFactor = asyncRequestWorkerFactor
        )
    }
}

@HorizonOSDsl
class CaddyContext {
    var admin: CaddyAdmin? = null
    var autoHTTPS: Boolean = true
    var httpPort: Int = 80
    var httpsPort: Int = 443
    var grace: String = "5s"
    var logging: CaddyLogging? = null

    fun admin(block: CaddyAdminContext.() -> Unit) {
        admin = CaddyAdminContext().apply(block).toAdmin()
    }

    fun logging(block: CaddyLoggingContext.() -> Unit) {
        logging = CaddyLoggingContext().apply(block).toLogging()
    }

    fun toConfig(): CaddyConfig {
        return CaddyConfig(
            admin = admin,
            autoHTTPS = autoHTTPS,
            httpPort = httpPort,
            httpsPort = httpsPort,
            grace = grace,
            logging = logging
        )
    }
}

@HorizonOSDsl
class CaddyAdminContext {
    var listen: String = "localhost:2019"
    var disabled: Boolean = false

    fun toAdmin(): CaddyAdmin {
        return CaddyAdmin(
            listen = listen,
            disabled = disabled
        )
    }
}

@HorizonOSDsl
class CaddyLoggingContext {
    var logs = mutableListOf<CaddyLog>()

    fun log(name: String, block: CaddyLogContext.() -> Unit) {
        logs.add(CaddyLogContext(name).apply(block).toLog())
    }

    fun toLogging(): CaddyLogging {
        return CaddyLogging(logs = logs)
    }
}

@HorizonOSDsl
class CaddyLogContext(private val name: String) {
    var output: String = "stdout"
    var format: String = "console"
    var level: String = "INFO"

    fun toLog(): CaddyLog {
        return CaddyLog(
            name = name,
            output = output,
            format = format,
            level = level
        )
    }
}

@HorizonOSDsl
class VirtualHostContext(private val serverName: String) {
    var port: Int = 80
    var documentRoot: String? = null
    var directoryIndex: List<String> = listOf("index.html", "index.php")
    var errorDocument = mutableMapOf<Int, String>()
    var redirects = mutableListOf<Redirect>()
    var locations = mutableListOf<Location>()
    var sslEnabled: Boolean = false
    var sslCertificate: String? = null
    var sslCertificateKey: String? = null

    fun redirect(from: String, to: String, permanent: Boolean = false) {
        redirects.add(Redirect(from, to, permanent))
    }

    fun location(path: String, block: LocationContext.() -> Unit) {
        locations.add(LocationContext(path).apply(block).toLocation())
    }

    fun errorPage(code: Int, page: String) {
        errorDocument[code] = page
    }

    fun toVirtualHost(): VirtualHost {
        return VirtualHost(
            serverName = serverName,
            port = port,
            documentRoot = documentRoot,
            directoryIndex = directoryIndex,
            errorDocument = errorDocument.toMap(),
            redirects = redirects,
            locations = locations,
            sslEnabled = sslEnabled,
            sslCertificate = sslCertificate,
            sslCertificateKey = sslCertificateKey
        )
    }
}

@HorizonOSDsl
class LocationContext(private val path: String) {
    var proxyPass: String? = null
    var alias: String? = null
    var tryFiles: List<String> = emptyList()
    var headers = mutableMapOf<String, String>()

    fun header(name: String, value: String) {
        headers[name] = value
    }

    fun toLocation(): Location {
        return Location(
            path = path,
            proxyPass = proxyPass,
            alias = alias,
            tryFiles = tryFiles,
            headers = headers.toMap()
        )
    }
}

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
    var image: String = ""
    var tag: String = "latest"
    var ports = mutableListOf<PortMapping>()
    var volumes = mutableListOf<VolumeMount>()
    var environment = mutableMapOf<String, String>()
    var networks = mutableListOf<String>()
    var restart: RestartPolicy = RestartPolicy.UNLESS_STOPPED
    var healthCheck: HealthCheck? = null
    var resources: ResourceLimits? = null

    fun port(host: Int, container: Int, protocol: String = "tcp") {
        ports.add(PortMapping(host, container, protocol))
    }

    fun volume(source: String, target: String, readOnly: Boolean = false) {
        volumes.add(VolumeMount(source, target, readOnly))
    }

    fun env(key: String, value: String) {
        environment[key] = value
    }

    fun network(name: String) {
        networks.add(name)
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
            ports = ports,
            volumes = volumes,
            environment = environment.toMap(),
            networks = networks,
            restart = restart,
            healthCheck = healthCheck,
            resources = resources
        )
    }
}

@HorizonOSDsl
class HealthCheckContext {
    var test: List<String> = emptyList()
    var interval: String = "30s"
    var timeout: String = "30s"
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
    var memory: String? = null
    var memorySwap: String? = null
    var cpus: String? = null
    var cpuShares: Int? = null

    fun toLimits(): ResourceLimits {
        return ResourceLimits(
            memory = memory,
            memorySwap = memorySwap,
            cpus = cpus,
            cpuShares = cpuShares
        )
    }
}

@HorizonOSDsl
class ContainerBuildContext {
    var dockerfiles = mutableListOf<DockerfileConfig>()
    var buildArgs = mutableMapOf<String, String>()
    var cacheConfig: BuildCacheConfig? = null

    fun dockerfile(path: String, block: DockerfileContext.() -> Unit) {
        dockerfiles.add(DockerfileContext(path).apply(block).toConfig())
    }

    fun arg(key: String, value: String) {
        buildArgs[key] = value
    }

    fun cache(block: BuildCacheContext.() -> Unit) {
        cacheConfig = BuildCacheContext().apply(block).toConfig()
    }

    fun toConfig(): ContainerBuildConfig {
        return ContainerBuildConfig(
            dockerfiles = dockerfiles,
            buildArgs = buildArgs.toMap(),
            cacheConfig = cacheConfig
        )
    }
}

@HorizonOSDsl
class DockerfileContext(private val path: String) {
    var target: String? = null
    var context: String = "."
    var tags = mutableListOf<String>()

    fun tag(name: String) {
        tags.add(name)
    }

    fun toConfig(): DockerfileConfig {
        return DockerfileConfig(
            path = path,
            target = target,
            context = context,
            tags = tags
        )
    }
}

@HorizonOSDsl
class BuildCacheContext {
    var enabled: Boolean = true
    var maxSize: String = "10GB"
    var maxAge: String = "168h"

    fun toConfig(): BuildCacheConfig {
        return BuildCacheConfig(
            enabled = enabled,
            maxSize = maxSize,
            maxAge = maxAge
        )
    }
}

// ===== Message Queue Configuration =====

@HorizonOSDsl
class MessageQueueContext(private val type: MessageQueueType) {
    var enabled: Boolean = true
    var autoStart: Boolean = true
    var port: Int? = null
    var managementPort: Int? = null
    var configOverrides = mutableMapOf<String, String>()
    
    // RabbitMQ specific
    var rabbitmqConfig: RabbitMQConfig? = null
    
    // Kafka specific
    var kafkaConfig: KafkaConfig? = null
    
    // NATS specific
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

    fun user(name: String, password: String, permissions: List<String> = emptyList()) {
        users.add(NATSUser(name, password, permissions))
    }

    fun toAuth(): NATSAuth {
        return NATSAuth(
            users = users,
            token = token
        )
    }
}

// ===== Monitoring Configuration =====

@HorizonOSDsl
class MonitoringContext(private val type: MonitoringType) {
    var enabled: Boolean = true
    var autoStart: Boolean = true
    var port: Int? = null
    var dataRetention: String = "15d"
    var configOverrides = mutableMapOf<String, String>()
    
    // Prometheus specific
    var prometheusConfig: PrometheusConfig? = null
    
    // Grafana specific
    var grafanaConfig: GrafanaConfig? = null

    fun prometheus(block: PrometheusContext.() -> Unit) {
        prometheusConfig = PrometheusContext().apply(block).toConfig()
    }

    fun grafana(block: GrafanaContext.() -> Unit) {
        grafanaConfig = GrafanaContext().apply(block).toConfig()
    }

    fun config(key: String, value: String) {
        configOverrides[key] = value
    }

    fun toService(): MonitoringService {
        return MonitoringService(
            type = type,
            enabled = enabled,
            autoStart = autoStart,
            port = port ?: getDefaultPort(type),
            dataRetention = dataRetention,
            configOverrides = configOverrides.toMap(),
            prometheusConfig = prometheusConfig,
            grafanaConfig = grafanaConfig
        )
    }

    private fun getDefaultPort(type: MonitoringType): Int = when (type) {
        MonitoringType.PROMETHEUS -> 9090
        MonitoringType.GRAFANA -> 3000
        MonitoringType.JAEGER -> 16686
        MonitoringType.ZIPKIN -> 9411
    }
}

@HorizonOSDsl
class PrometheusContext {
    var scrapeInterval: String = "15s"
    var evaluationInterval: String = "15s"
    var scrapeConfigs = mutableListOf<ScrapeConfig>()
    var rules = mutableListOf<PrometheusRule>()
    var alertmanagerConfig: AlertmanagerConfig? = null

    fun scrape(jobName: String, block: ScrapeConfigContext.() -> Unit) {
        scrapeConfigs.add(ScrapeConfigContext(jobName).apply(block).toConfig())
    }

    fun rule(file: String) {
        rules.add(PrometheusRule(file))
    }

    fun alertmanager(block: AlertmanagerContext.() -> Unit) {
        alertmanagerConfig = AlertmanagerContext().apply(block).toConfig()
    }

    fun toConfig(): PrometheusConfig {
        return PrometheusConfig(
            scrapeInterval = scrapeInterval,
            evaluationInterval = evaluationInterval,
            scrapeConfigs = scrapeConfigs,
            rules = rules,
            alertmanagerConfig = alertmanagerConfig
        )
    }
}

@HorizonOSDsl
class ScrapeConfigContext(private val jobName: String) {
    var staticConfigs = mutableListOf<String>()
    var scrapeInterval: String? = null
    var metricsPath: String = "/metrics"

    fun target(address: String) {
        staticConfigs.add(address)
    }

    fun toConfig(): ScrapeConfig {
        return ScrapeConfig(
            jobName = jobName,
            staticConfigs = staticConfigs,
            scrapeInterval = scrapeInterval,
            metricsPath = metricsPath
        )
    }
}

@HorizonOSDsl
class AlertmanagerContext {
    var staticConfigs = mutableListOf<String>()

    fun target(address: String) {
        staticConfigs.add(address)
    }

    fun toConfig(): AlertmanagerConfig {
        return AlertmanagerConfig(staticConfigs = staticConfigs)
    }
}

@HorizonOSDsl
class GrafanaContext {
    var adminUser: String = "admin"
    var adminPassword: String = "admin"
    var allowSignUp: Boolean = false
    var datasources = mutableListOf<GrafanaDatasource>()
    var dashboards = mutableListOf<GrafanaDashboard>()
    var plugins = mutableListOf<String>()

    fun datasource(name: String, block: GrafanaDatasourceContext.() -> Unit) {
        datasources.add(GrafanaDatasourceContext(name).apply(block).toDatasource())
    }

    fun dashboard(name: String, path: String) {
        dashboards.add(GrafanaDashboard(name, path))
    }

    fun plugin(name: String) {
        plugins.add(name)
    }

    fun toConfig(): GrafanaConfig {
        return GrafanaConfig(
            adminUser = adminUser,
            adminPassword = adminPassword,
            allowSignUp = allowSignUp,
            datasources = datasources,
            dashboards = dashboards,
            plugins = plugins
        )
    }
}

@HorizonOSDsl
class GrafanaDatasourceContext(private val name: String) {
    var type: String = "prometheus"
    var url: String = "http://localhost:9090"
    var access: String = "proxy"
    var isDefault: Boolean = false

    fun toDatasource(): GrafanaDatasource {
        return GrafanaDatasource(
            name = name,
            type = type,
            url = url,
            access = access,
            isDefault = isDefault
        )
    }
}

// ===== Systemd Unit Configuration =====

@HorizonOSDsl
class SystemdUnitContext(private val name: String) {
    var unitType: SystemdUnitType = SystemdUnitType.SERVICE
    var description: String = ""
    var after = mutableListOf<String>()
    var requires = mutableListOf<String>()
    var wants = mutableListOf<String>()
    var conflicts = mutableListOf<String>()
    
    // Service section
    var execStart: String? = null
    var execStop: String? = null
    var execReload: String? = null
    var workingDirectory: String? = null
    var user: String? = null
    var group: String? = null
    var environment = mutableMapOf<String, String>()
    var restart: SystemdRestartPolicy = SystemdRestartPolicy.NO
    var restartSec: Int = 10
    var type: SystemdServiceType = SystemdServiceType.SIMPLE
    
    // Install section
    var wantedBy = mutableListOf<String>()
    var requiredBy = mutableListOf<String>()

    fun env(key: String, value: String) {
        environment[key] = value
    }

    fun toUnit(): SystemdUnit {
        return SystemdUnit(
            name = name,
            unitType = unitType,
            description = description,
            after = after,
            requires = requires,
            wants = wants,
            conflicts = conflicts,
            execStart = execStart,
            execStop = execStop,
            execReload = execReload,
            workingDirectory = workingDirectory,
            user = user,
            group = group,
            environment = environment.toMap(),
            restart = restart,
            restartSec = restartSec,
            type = type,
            wantedBy = wantedBy,
            requiredBy = requiredBy
        )
    }
}

// ===== Enums =====

@Serializable
enum class DatabaseType {
    POSTGRESQL, MYSQL, REDIS, MONGODB, SQLITE
}

@Serializable
enum class WebServerType {
    NGINX, APACHE, CADDY, LIGHTTPD
}

@Serializable
enum class ContainerRuntime {
    DOCKER, PODMAN, SYSTEMD_NSPAWN
}

@Serializable
enum class MessageQueueType {
    RABBITMQ, KAFKA, NATS, REDIS_STREAMS
}

@Serializable
enum class MonitoringType {
    PROMETHEUS, GRAFANA, JAEGER, ZIPKIN
}

@Serializable
enum class SystemdUnitType {
    SERVICE, SOCKET, TIMER, MOUNT, TARGET
}

@Serializable
enum class SystemdRestartPolicy {
    NO, ON_SUCCESS, ON_FAILURE, ON_ABNORMAL, ON_ABORT, ON_WATCHDOG, ALWAYS
}

@Serializable
enum class SystemdServiceType {
    SIMPLE, FORKING, ONESHOT, DBUS, NOTIFY, IDLE
}

@Serializable
enum class RedisPersistence {
    RDB, AOF, BOTH, NONE
}

@Serializable
enum class LoadBalancingMethod {
    ROUND_ROBIN, LEAST_CONN, IP_HASH, WEIGHTED
}

@Serializable
enum class RestartPolicy {
    NO, ALWAYS, ON_FAILURE, UNLESS_STOPPED
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

// Database Service Data Classes
@Serializable
data class DatabaseService(
    val type: DatabaseType,
    val enabled: Boolean,
    val autoStart: Boolean,
    val version: String?,
    val port: Int,
    val dataDirectory: String,
    val configOverrides: Map<String, String>,
    val users: List<DatabaseUser>,
    val databases: List<DatabaseSchema>,
    val postgresConfig: PostgresConfig?,
    val mysqlConfig: MySQLConfig?,
    val redisConfig: RedisConfig?
)

@Serializable
data class PostgresConfig(
    val maxConnections: Int,
    val sharedBuffers: String,
    val effectiveCacheSize: String,
    val maintenanceWorkMem: String,
    val checkpointSegments: Int,
    val checkpointCompletionTarget: Double,
    val walBuffers: String,
    val defaultStatisticsTarget: Int,
    val logMinDurationStatement: Int,
    val logConnections: Boolean,
    val logDisconnections: Boolean,
    val logLockWaits: Boolean,
    val logStatement: String,
    val logLinePrefix: String,
    val extensions: List<String>
)

@Serializable
data class MySQLConfig(
    val maxConnections: Int,
    val innodbBufferPoolSize: String,
    val innodbLogFileSize: String,
    val innodbFilePerTable: Boolean,
    val queryCache: Boolean,
    val queryCacheSize: String,
    val tmpTableSize: String,
    val maxHeapTableSize: String,
    val slowQueryLog: Boolean,
    val longQueryTime: Int,
    val binlogFormat: String,
    val serverCharset: String,
    val serverCollation: String
)

@Serializable
data class RedisConfig(
    val maxMemory: String,
    val maxMemoryPolicy: String,
    val persistenceMode: RedisPersistence,
    val rdbSavePolicy: List<String>,
    val aofRewritePolicy: Boolean,
    val tcpKeepalive: Int,
    val timeout: Int,
    val databases: Int,
    val password: String?,
    val requirepass: Boolean,
    val masterAuth: String?,
    val clustering: Boolean,
    val replication: RedisReplicationConfig?
)

@Serializable
data class RedisReplicationConfig(
    val masterHost: String,
    val masterPort: Int,
    val masterTimeout: Int,
    val replicationBacklogSize: String,
    val replicationDisklessSync: Boolean
)

@Serializable
data class DatabaseUser(
    val name: String,
    val password: String?,
    val privileges: List<String>,
    val databases: List<String>,
    val host: String
)

@Serializable
data class DatabaseSchema(
    val name: String,
    val charset: String,
    val collation: String,
    val owner: String?
)

// Web Server Data Classes
@Serializable
data class WebServerService(
    val type: WebServerType,
    val enabled: Boolean,
    val autoStart: Boolean,
    val port: Int,
    val sslPort: Int,
    val serverName: String?,
    val documentRoot: String,
    val logLevel: String,
    val accessLog: String?,
    val errorLog: String?,
    val enableSSL: Boolean,
    val sslCertificate: String?,
    val sslCertificateKey: String?,
    val virtualHosts: List<VirtualHost>,
    val modules: List<String>,
    val configOverrides: Map<String, String>,
    val nginxConfig: NginxConfig?,
    val apacheConfig: ApacheConfig?,
    val caddyConfig: CaddyConfig?
)

@Serializable
data class NginxConfig(
    val workerProcesses: String,
    val workerConnections: Int,
    val keepaliveTimeout: Int,
    val clientMaxBodySize: String,
    val gzipCompression: Boolean,
    val gzipTypes: List<String>,
    val upstreams: List<NginxUpstream>,
    val rateLimiting: NginxRateLimiting?
)

@Serializable
data class NginxUpstream(
    val name: String,
    val servers: List<String>,
    val loadBalancing: LoadBalancingMethod,
    val healthCheck: Boolean
)

@Serializable
data class NginxRateLimiting(
    val zones: List<RateLimitZone>
)

@Serializable
data class RateLimitZone(
    val name: String,
    val key: String,
    val size: String,
    val rate: String
)

@Serializable
data class ApacheConfig(
    val serverTokens: String,
    val keepAlive: Boolean,
    val maxKeepAliveRequests: Int,
    val keepAliveTimeout: Int,
    val preforkConfig: ApachePreforkConfig?,
    val workerConfig: ApacheWorkerConfig?,
    val eventConfig: ApacheEventConfig?
)

@Serializable
data class ApachePreforkConfig(
    val startServers: Int,
    val minSpareServers: Int,
    val maxSpareServers: Int,
    val serverLimit: Int,
    val maxRequestWorkers: Int
)

@Serializable
data class ApacheWorkerConfig(
    val startServers: Int,
    val maxRequestWorkers: Int,
    val minSpareThreads: Int,
    val maxSpareThreads: Int,
    val threadLimit: Int,
    val threadsPerChild: Int
)

@Serializable
data class ApacheEventConfig(
    val startServers: Int,
    val maxRequestWorkers: Int,
    val minSpareThreads: Int,
    val maxSpareThreads: Int,
    val threadLimit: Int,
    val threadsPerChild: Int,
    val asyncRequestWorkerFactor: Int
)

@Serializable
data class CaddyConfig(
    val admin: CaddyAdmin?,
    val autoHTTPS: Boolean,
    val httpPort: Int,
    val httpsPort: Int,
    val grace: String,
    val logging: CaddyLogging?
)

@Serializable
data class CaddyAdmin(
    val listen: String,
    val disabled: Boolean
)

@Serializable
data class CaddyLogging(
    val logs: List<CaddyLog>
)

@Serializable
data class CaddyLog(
    val name: String,
    val output: String,
    val format: String,
    val level: String
)

@Serializable
data class VirtualHost(
    val serverName: String,
    val port: Int,
    val documentRoot: String?,
    val directoryIndex: List<String>,
    val errorDocument: Map<Int, String>,
    val redirects: List<Redirect>,
    val locations: List<Location>,
    val sslEnabled: Boolean,
    val sslCertificate: String?,
    val sslCertificateKey: String?
)

@Serializable
data class Redirect(
    val from: String,
    val to: String,
    val permanent: Boolean
)

@Serializable
data class Location(
    val path: String,
    val proxyPass: String?,
    val alias: String?,
    val tryFiles: List<String>,
    val headers: Map<String, String>
)

// Container Service Data Classes
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
    val image: String,
    val tag: String,
    val ports: List<PortMapping>,
    val volumes: List<VolumeMount>,
    val environment: Map<String, String>,
    val networks: List<String>,
    val restart: RestartPolicy,
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
    val source: String,
    val target: String,
    val readOnly: Boolean
)

@Serializable
data class HealthCheck(
    val test: List<String>,
    val interval: String,
    val timeout: String,
    val retries: Int,
    val startPeriod: String
)

@Serializable
data class ResourceLimits(
    val memory: String?,
    val memorySwap: String?,
    val cpus: String?,
    val cpuShares: Int?
)

@Serializable
data class ContainerBuildConfig(
    val dockerfiles: List<DockerfileConfig>,
    val buildArgs: Map<String, String>,
    val cacheConfig: BuildCacheConfig?
)

@Serializable
data class DockerfileConfig(
    val path: String,
    val target: String?,
    val context: String,
    val tags: List<String>
)

@Serializable
data class BuildCacheConfig(
    val enabled: Boolean,
    val maxSize: String,
    val maxAge: String
)

// Message Queue Data Classes
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
    val password: String,
    val permissions: List<String>
)

// Monitoring Data Classes
@Serializable
data class MonitoringService(
    val type: MonitoringType,
    val enabled: Boolean,
    val autoStart: Boolean,
    val port: Int,
    val dataRetention: String,
    val configOverrides: Map<String, String>,
    val prometheusConfig: PrometheusConfig?,
    val grafanaConfig: GrafanaConfig?
)

@Serializable
data class PrometheusConfig(
    val scrapeInterval: String,
    val evaluationInterval: String,
    val scrapeConfigs: List<ScrapeConfig>,
    val rules: List<PrometheusRule>,
    val alertmanagerConfig: AlertmanagerConfig?
)

@Serializable
data class ScrapeConfig(
    val jobName: String,
    val staticConfigs: List<String>,
    val scrapeInterval: String?,
    val metricsPath: String
)

@Serializable
data class PrometheusRule(
    val file: String
)

@Serializable
data class AlertmanagerConfig(
    val staticConfigs: List<String>
)

@Serializable
data class GrafanaConfig(
    val adminUser: String,
    val adminPassword: String,
    val allowSignUp: Boolean,
    val datasources: List<GrafanaDatasource>,
    val dashboards: List<GrafanaDashboard>,
    val plugins: List<String>
)

@Serializable
data class GrafanaDatasource(
    val name: String,
    val type: String,
    val url: String,
    val access: String,
    val isDefault: Boolean
)

@Serializable
data class GrafanaDashboard(
    val name: String,
    val path: String
)

// Systemd Unit Data Classes
@Serializable
data class SystemdUnit(
    val name: String,
    val unitType: SystemdUnitType,
    val description: String,
    val after: List<String>,
    val requires: List<String>,
    val wants: List<String>,
    val conflicts: List<String>,
    val execStart: String?,
    val execStop: String?,
    val execReload: String?,
    val workingDirectory: String?,
    val user: String?,
    val group: String?,
    val environment: Map<String, String>,
    val restart: SystemdRestartPolicy,
    val restartSec: Int,
    val type: SystemdServiceType,
    val wantedBy: List<String>,
    val requiredBy: List<String>
)