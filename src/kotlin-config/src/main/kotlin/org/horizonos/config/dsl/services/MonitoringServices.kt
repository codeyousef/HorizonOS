package org.horizonos.config.dsl.services

import kotlinx.serialization.Serializable
import org.horizonos.config.dsl.HorizonOSDsl

// ===== Monitoring Configuration =====

@HorizonOSDsl
class MonitoringContext(private val type: MonitoringType) {
    var enabled: Boolean = true
    var autoStart: Boolean = true
    var port: Int? = null
    var adminPort: Int? = null
    var dataRetention: String = "15d"
    var configOverrides = mutableMapOf<String, String>()
    
    // Monitoring-specific configurations
    var prometheusConfig: PrometheusConfig? = null
    var grafanaConfig: GrafanaConfig? = null
    var jaegerConfig: JaegerConfig? = null
    var zipkinConfig: ZipkinConfig? = null

    fun prometheus(block: PrometheusContext.() -> Unit) {
        prometheusConfig = PrometheusContext().apply(block).toConfig()
    }

    fun grafana(block: GrafanaContext.() -> Unit) {
        grafanaConfig = GrafanaContext().apply(block).toConfig()
    }

    fun jaeger(block: JaegerContext.() -> Unit) {
        jaegerConfig = JaegerContext().apply(block).toConfig()
    }

    fun zipkin(block: ZipkinContext.() -> Unit) {
        zipkinConfig = ZipkinContext().apply(block).toConfig()
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
            adminPort = adminPort ?: getDefaultAdminPort(type),
            dataRetention = dataRetention,
            configOverrides = configOverrides.toMap(),
            prometheusConfig = prometheusConfig,
            grafanaConfig = grafanaConfig,
            jaegerConfig = jaegerConfig,
            zipkinConfig = zipkinConfig
        )
    }

    private fun getDefaultPort(type: MonitoringType): Int = when (type) {
        MonitoringType.PROMETHEUS -> 9090
        MonitoringType.GRAFANA -> 3000
        MonitoringType.JAEGER -> 16686
        MonitoringType.ZIPKIN -> 9411
    }

    private fun getDefaultAdminPort(type: MonitoringType): Int? = when (type) {
        MonitoringType.PROMETHEUS -> null
        MonitoringType.GRAFANA -> null
        MonitoringType.JAEGER -> 14268
        MonitoringType.ZIPKIN -> null
    }
}

// ===== Prometheus Configuration =====

@HorizonOSDsl
class PrometheusContext {
    var scrapeInterval: String = "15s"
    var evaluationInterval: String = "15s"
    var retentionTime: String = "15d"
    var retentionSize: String = "10GB"
    var scrapeConfigs = mutableListOf<ScrapeConfig>()
    var rules = mutableListOf<PrometheusRule>()
    var alertmanager: AlertmanagerConfig? = null

    fun scrapeConfig(jobName: String, block: ScrapeConfigContext.() -> Unit) {
        scrapeConfigs.add(ScrapeConfigContext(jobName).apply(block).toConfig())
    }

    fun rule(group: String, block: PrometheusRuleContext.() -> Unit) {
        rules.add(PrometheusRuleContext(group).apply(block).toRule())
    }

    fun alertmanager(block: AlertmanagerContext.() -> Unit) {
        alertmanager = AlertmanagerContext().apply(block).toConfig()
    }

    fun toConfig(): PrometheusConfig {
        return PrometheusConfig(
            scrapeInterval = scrapeInterval,
            evaluationInterval = evaluationInterval,
            retentionTime = retentionTime,
            retentionSize = retentionSize,
            scrapeConfigs = scrapeConfigs,
            rules = rules,
            alertmanager = alertmanager
        )
    }
}

@HorizonOSDsl
class ScrapeConfigContext(private val jobName: String) {
    var scrapeInterval: String = "15s"
    var metricsPath: String = "/metrics"
    var staticConfigs = mutableListOf<StaticConfig>()

    fun staticConfig(block: StaticConfigContext.() -> Unit) {
        staticConfigs.add(StaticConfigContext().apply(block).toConfig())
    }

    fun toConfig(): ScrapeConfig {
        return ScrapeConfig(
            jobName = jobName,
            scrapeInterval = scrapeInterval,
            metricsPath = metricsPath,
            staticConfigs = staticConfigs
        )
    }
}

@HorizonOSDsl
class StaticConfigContext {
    var targets = mutableListOf<String>()
    var labels = mutableMapOf<String, String>()

    fun target(target: String) {
        targets.add(target)
    }

    fun label(key: String, value: String) {
        labels[key] = value
    }

    fun toConfig(): StaticConfig {
        return StaticConfig(
            targets = targets,
            labels = labels.toMap()
        )
    }
}

// ===== Grafana Configuration =====

@HorizonOSDsl
class GrafanaContext {
    var adminUser: String = "admin"
    var adminPassword: String = "admin"
    var domain: String = "localhost"
    var rootUrl: String = "%(protocol)s://%(domain)s:%(http_port)s/"
    var dataSources = mutableListOf<GrafanaDataSource>()
    var dashboards = mutableListOf<GrafanaDashboard>()
    var plugins = mutableListOf<String>()

    fun dataSource(name: String, block: GrafanaDataSourceContext.() -> Unit) {
        dataSources.add(GrafanaDataSourceContext(name).apply(block).toDataSource())
    }

    fun dashboard(name: String, block: GrafanaDashboardContext.() -> Unit) {
        dashboards.add(GrafanaDashboardContext(name).apply(block).toDashboard())
    }

    fun plugin(name: String) {
        plugins.add(name)
    }

    fun toConfig(): GrafanaConfig {
        return GrafanaConfig(
            adminUser = adminUser,
            adminPassword = adminPassword,
            domain = domain,
            rootUrl = rootUrl,
            dataSources = dataSources,
            dashboards = dashboards,
            plugins = plugins
        )
    }
}

@HorizonOSDsl
class GrafanaDataSourceContext(private val name: String) {
    var type: String = "prometheus"
    var url: String = "http://localhost:9090"
    var access: String = "proxy"
    var isDefault: Boolean = false

    fun toDataSource(): GrafanaDataSource {
        return GrafanaDataSource(
            name = name,
            type = type,
            url = url,
            access = access,
            isDefault = isDefault
        )
    }
}

@HorizonOSDsl
class GrafanaDashboardContext(private val name: String) {
    var path: String? = null
    var url: String? = null

    fun toDashboard(): GrafanaDashboard {
        return GrafanaDashboard(
            name = name,
            path = path,
            url = url
        )
    }
}

// ===== Jaeger Configuration =====

@HorizonOSDsl
class JaegerContext {
    var spanStorageType: String = "memory"
    var cassandraConfig: JaegerCassandraConfig? = null
    var elasticsearchConfig: JaegerElasticsearchConfig? = null
    var sampling: JaegerSampling? = null

    fun cassandra(block: JaegerCassandraContext.() -> Unit) {
        spanStorageType = "cassandra"
        cassandraConfig = JaegerCassandraContext().apply(block).toConfig()
    }

    fun elasticsearch(block: JaegerElasticsearchContext.() -> Unit) {
        spanStorageType = "elasticsearch"
        elasticsearchConfig = JaegerElasticsearchContext().apply(block).toConfig()
    }

    fun sampling(block: JaegerSamplingContext.() -> Unit) {
        sampling = JaegerSamplingContext().apply(block).toSampling()
    }

    fun toConfig(): JaegerConfig {
        return JaegerConfig(
            spanStorageType = spanStorageType,
            cassandraConfig = cassandraConfig,
            elasticsearchConfig = elasticsearchConfig,
            sampling = sampling
        )
    }
}

// ===== Zipkin Configuration =====

@HorizonOSDsl
class ZipkinContext {
    var storageType: String = "mem"
    var mysqlConfig: ZipkinMySQLConfig? = null
    var cassandraConfig: ZipkinCassandraConfig? = null

    fun mysql(block: ZipkinMySQLContext.() -> Unit) {
        storageType = "mysql"
        mysqlConfig = ZipkinMySQLContext().apply(block).toConfig()
    }

    fun cassandra(block: ZipkinCassandraContext.() -> Unit) {
        storageType = "cassandra3"
        cassandraConfig = ZipkinCassandraContext().apply(block).toConfig()
    }

    fun toConfig(): ZipkinConfig {
        return ZipkinConfig(
            storageType = storageType,
            mysqlConfig = mysqlConfig,
            cassandraConfig = cassandraConfig
        )
    }
}

// ===== Enums =====

@Serializable
enum class MonitoringType {
    PROMETHEUS, GRAFANA, JAEGER, ZIPKIN
}

// ===== Data Classes =====

@Serializable
data class MonitoringService(
    val type: MonitoringType,
    val enabled: Boolean,
    val autoStart: Boolean,
    val port: Int,
    val adminPort: Int?,
    val dataRetention: String,
    val configOverrides: Map<String, String>,
    val prometheusConfig: PrometheusConfig?,
    val grafanaConfig: GrafanaConfig?,
    val jaegerConfig: JaegerConfig?,
    val zipkinConfig: ZipkinConfig?
)

@Serializable
data class PrometheusConfig(
    val scrapeInterval: String,
    val evaluationInterval: String,
    val retentionTime: String,
    val retentionSize: String,
    val scrapeConfigs: List<ScrapeConfig>,
    val rules: List<PrometheusRule>,
    val alertmanager: AlertmanagerConfig?
)

@Serializable
data class ScrapeConfig(
    val jobName: String,
    val scrapeInterval: String,
    val metricsPath: String,
    val staticConfigs: List<StaticConfig>
)

@Serializable
data class StaticConfig(
    val targets: List<String>,
    val labels: Map<String, String>
)

@Serializable
data class PrometheusRule(
    val group: String,
    val interval: String,
    val rules: List<PrometheusRuleItem>
)

@Serializable
data class PrometheusRuleItem(
    val alert: String?,
    val expr: String,
    val for_: String?,
    val labels: Map<String, String>,
    val annotations: Map<String, String>
)

@Serializable
data class AlertmanagerConfig(
    val staticConfigs: List<StaticConfig>
)

@Serializable
data class GrafanaConfig(
    val adminUser: String,
    val adminPassword: String,
    val domain: String,
    val rootUrl: String,
    val dataSources: List<GrafanaDataSource>,
    val dashboards: List<GrafanaDashboard>,
    val plugins: List<String>
)

@Serializable
data class GrafanaDataSource(
    val name: String,
    val type: String,
    val url: String,
    val access: String,
    val isDefault: Boolean
)

@Serializable
data class GrafanaDashboard(
    val name: String,
    val path: String?,
    val url: String?
)

@Serializable
data class JaegerConfig(
    val spanStorageType: String,
    val cassandraConfig: JaegerCassandraConfig?,
    val elasticsearchConfig: JaegerElasticsearchConfig?,
    val sampling: JaegerSampling?
)

@Serializable
data class JaegerCassandraConfig(
    val servers: List<String>,
    val keyspace: String,
    val localDC: String
)

@Serializable
data class JaegerElasticsearchConfig(
    val serverUrls: List<String>,
    val indexPrefix: String,
    val username: String?,
    val password: String?
)

@Serializable
data class JaegerSampling(
    val defaultStrategy: String,
    val maxTracesPerSecond: Int
)

@Serializable
data class ZipkinConfig(
    val storageType: String,
    val mysqlConfig: ZipkinMySQLConfig?,
    val cassandraConfig: ZipkinCassandraConfig?
)

@Serializable
data class ZipkinMySQLConfig(
    val host: String,
    val port: Int,
    val username: String,
    val password: String,
    val database: String
)

@Serializable
data class ZipkinCassandraConfig(
    val contactPoints: List<String>,
    val localDC: String,
    val keyspace: String
)

// Additional context classes for complex configurations
@HorizonOSDsl
class PrometheusRuleContext(private val group: String) {
    var interval: String = "30s"
    var rules = mutableListOf<PrometheusRuleItem>()

    fun alert(name: String, expr: String, for_: String = "0m", block: (PrometheusRuleItemContext.() -> Unit)? = null) {
        val context = PrometheusRuleItemContext(name, expr, for_)
        block?.invoke(context)
        rules.add(context.toRuleItem())
    }

    fun toRule(): PrometheusRule {
        return PrometheusRule(
            group = group,
            interval = interval,
            rules = rules
        )
    }
}

@HorizonOSDsl
class PrometheusRuleItemContext(
    private val alert: String,
    private val expr: String,
    private val for_: String
) {
    var labels = mutableMapOf<String, String>()
    var annotations = mutableMapOf<String, String>()

    fun label(key: String, value: String) {
        labels[key] = value
    }

    fun annotation(key: String, value: String) {
        annotations[key] = value
    }

    fun toRuleItem(): PrometheusRuleItem {
        return PrometheusRuleItem(
            alert = alert,
            expr = expr,
            for_ = for_,
            labels = labels.toMap(),
            annotations = annotations.toMap()
        )
    }
}

@HorizonOSDsl
class AlertmanagerContext {
    var staticConfigs = mutableListOf<StaticConfig>()

    fun staticConfig(block: StaticConfigContext.() -> Unit) {
        staticConfigs.add(StaticConfigContext().apply(block).toConfig())
    }

    fun toConfig(): AlertmanagerConfig {
        return AlertmanagerConfig(
            staticConfigs = staticConfigs
        )
    }
}

@HorizonOSDsl
class JaegerCassandraContext {
    var servers = mutableListOf<String>()
    var keyspace: String = "jaeger_v1_test"
    var localDC: String = "datacenter1"

    fun server(address: String) {
        servers.add(address)
    }

    fun toConfig(): JaegerCassandraConfig {
        return JaegerCassandraConfig(
            servers = servers,
            keyspace = keyspace,
            localDC = localDC
        )
    }
}

@HorizonOSDsl
class JaegerElasticsearchContext {
    var serverUrls = mutableListOf<String>()
    var indexPrefix: String = "jaeger"
    var username: String? = null
    var password: String? = null

    fun server(url: String) {
        serverUrls.add(url)
    }

    fun toConfig(): JaegerElasticsearchConfig {
        return JaegerElasticsearchConfig(
            serverUrls = serverUrls,
            indexPrefix = indexPrefix,
            username = username,
            password = password
        )
    }
}

@HorizonOSDsl
class JaegerSamplingContext {
    var defaultStrategy: String = "probabilistic"
    var maxTracesPerSecond: Int = 10000

    fun toSampling(): JaegerSampling {
        return JaegerSampling(
            defaultStrategy = defaultStrategy,
            maxTracesPerSecond = maxTracesPerSecond
        )
    }
}

@HorizonOSDsl
class ZipkinMySQLContext {
    var host: String = "localhost"
    var port: Int = 3306
    var username: String = "zipkin"
    var password: String = "zipkin"
    var database: String = "zipkin"

    fun toConfig(): ZipkinMySQLConfig {
        return ZipkinMySQLConfig(
            host = host,
            port = port,
            username = username,
            password = password,
            database = database
        )
    }
}

@HorizonOSDsl
class ZipkinCassandraContext {
    var contactPoints = mutableListOf<String>()
    var localDC: String = "datacenter1"
    var keyspace: String = "zipkin2"

    fun contactPoint(address: String) {
        contactPoints.add(address)
    }

    fun toConfig(): ZipkinCassandraConfig {
        return ZipkinCassandraConfig(
            contactPoints = contactPoints,
            localDC = localDC,
            keyspace = keyspace
        )
    }
}