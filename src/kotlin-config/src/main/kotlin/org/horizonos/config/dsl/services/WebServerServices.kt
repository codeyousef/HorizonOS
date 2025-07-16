package org.horizonos.config.dsl.services

import kotlinx.serialization.Serializable
import org.horizonos.config.dsl.HorizonOSDsl

// ===== Web Server Configuration =====

@HorizonOSDsl
class WebServerContext(private val type: WebServerType) {
    var enabled: Boolean = true
    var autoStart: Boolean = true
    var version: String? = null
    var port: Int? = null
    var sslPort: Int? = null
    var virtualHosts = mutableListOf<VirtualHost>()
    var sslEnabled: Boolean = false
    var sslCertificate: String? = null
    var sslCertificateKey: String? = null
    
    // Server-specific configurations
    var nginxConfig: NginxConfig? = null
    var apacheConfig: ApacheConfig? = null
    var caddyConfig: CaddyConfig? = null
    var lighttpdConfig: LighttpdConfig? = null

    fun nginx(block: NginxContext.() -> Unit) {
        nginxConfig = NginxContext().apply(block).toConfig()
    }

    fun apache(block: ApacheContext.() -> Unit) {
        apacheConfig = ApacheContext().apply(block).toConfig()
    }

    fun caddy(block: CaddyContext.() -> Unit) {
        caddyConfig = CaddyContext().apply(block).toConfig()
    }

    fun lighttpd(block: LighttpdContext.() -> Unit) {
        lighttpdConfig = LighttpdContext().apply(block).toConfig()
    }

    fun virtualHost(serverName: String, block: VirtualHostContext.() -> Unit) {
        virtualHosts.add(VirtualHostContext(serverName).apply(block).toVirtualHost())
    }

    fun toService(): WebServerService {
        return WebServerService(
            type = type,
            enabled = enabled,
            autoStart = autoStart,
            version = version,
            port = port ?: getDefaultPort(type),
            sslPort = sslPort ?: getDefaultSslPort(type),
            virtualHosts = virtualHosts,
            sslEnabled = sslEnabled,
            sslCertificate = sslCertificate,
            sslCertificateKey = sslCertificateKey,
            nginxConfig = nginxConfig,
            apacheConfig = apacheConfig,
            caddyConfig = caddyConfig,
            lighttpdConfig = lighttpdConfig
        )
    }

    private fun getDefaultPort(type: WebServerType): Int = when (type) {
        WebServerType.NGINX -> 80
        WebServerType.APACHE -> 80
        WebServerType.CADDY -> 80
        WebServerType.LIGHTTPD -> 80
    }

    private fun getDefaultSslPort(type: WebServerType): Int = when (type) {
        WebServerType.NGINX -> 443
        WebServerType.APACHE -> 443
        WebServerType.CADDY -> 443
        WebServerType.LIGHTTPD -> 443
    }
}

// ===== Nginx Configuration =====

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

// ===== Apache Configuration =====

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

// ===== Caddy Configuration =====

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

// ===== Lighttpd Configuration =====

@HorizonOSDsl
class LighttpdContext {
    var serverMaxFds: Int = 2048
    var serverMaxConnections: Int = 1024
    var serverNetworkBackend: String = "sendfile"
    var serverUploadDirs = listOf("/var/tmp")
    var indexFileNames = listOf("index.php", "index.html", "index.htm", "default.htm")
    var urlRewriteOnce: Boolean = true
    var dirListing: Boolean = false
    var modules = mutableListOf<String>()

    fun module(name: String) {
        modules.add(name)
    }

    fun toConfig(): LighttpdConfig {
        return LighttpdConfig(
            serverMaxFds = serverMaxFds,
            serverMaxConnections = serverMaxConnections,
            serverNetworkBackend = serverNetworkBackend,
            serverUploadDirs = serverUploadDirs,
            indexFileNames = indexFileNames,
            urlRewriteOnce = urlRewriteOnce,
            dirListing = dirListing,
            modules = modules
        )
    }
}

// ===== Virtual Host Configuration =====

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

// ===== Enums =====

@Serializable
enum class WebServerType {
    NGINX, APACHE, CADDY, LIGHTTPD
}

@Serializable
enum class LoadBalancingMethod {
    ROUND_ROBIN, LEAST_CONN, IP_HASH, LEAST_TIME
}

// ===== Data Classes =====

@Serializable
data class WebServerService(
    val type: WebServerType,
    val enabled: Boolean,
    val autoStart: Boolean,
    val version: String?,
    val port: Int,
    val sslPort: Int,
    val virtualHosts: List<VirtualHost>,
    val sslEnabled: Boolean,
    val sslCertificate: String?,
    val sslCertificateKey: String?,
    val nginxConfig: NginxConfig?,
    val apacheConfig: ApacheConfig?,
    val caddyConfig: CaddyConfig?,
    val lighttpdConfig: LighttpdConfig?
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
data class LighttpdConfig(
    val serverMaxFds: Int,
    val serverMaxConnections: Int,
    val serverNetworkBackend: String,
    val serverUploadDirs: List<String>,
    val indexFileNames: List<String>,
    val urlRewriteOnce: Boolean,
    val dirListing: Boolean,
    val modules: List<String>
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