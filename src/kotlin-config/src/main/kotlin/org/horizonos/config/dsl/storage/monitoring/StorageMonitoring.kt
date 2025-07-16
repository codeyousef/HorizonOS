package org.horizonos.config.dsl.storage.monitoring

import kotlinx.serialization.Serializable
import kotlin.time.Duration
import kotlin.time.Duration.Companion.hours
import kotlin.time.Duration.Companion.minutes

// ===== Storage Monitoring Configuration =====

@Serializable
data class StorageMonitoringConfig(
    val enabled: Boolean = true,
    val smart: SmartMonitoring = SmartMonitoring(),
    val usage: UsageMonitoring = UsageMonitoring(),
    val performance: PerformanceMonitoring = PerformanceMonitoring(),
    val health: HealthMonitoring = HealthMonitoring(),
    val alerts: AlertConfiguration = AlertConfiguration()
)

@Serializable
data class SmartMonitoring(
    val enabled: Boolean = true,
    val daemon: String = "smartd",
    val devices: List<SmartDevice> = emptyList(),
    val schedule: SmartSchedule = SmartSchedule(),
    val notifications: SmartNotifications = SmartNotifications()
)

@Serializable
data class SmartDevice(
    val device: String,
    val type: DeviceType = DeviceType.AUTO,
    val attributes: List<SmartAttribute> = emptyList(),
    val tests: SmartTests = SmartTests(),
    val temperature: TemperatureMonitoring = TemperatureMonitoring()
)

@Serializable
data class SmartAttribute(
    val id: Int,
    val name: String,
    val threshold: Int? = null,
    val action: AttributeAction = AttributeAction.LOG
)

@Serializable
data class SmartTests(
    val shortTest: Duration? = 24.hours,
    val longTest: Duration? = 168.hours, // weekly
    val conveyanceTest: Duration? = null,
    val selectiveTest: SelectiveTest? = null
)

@Serializable
data class SelectiveTest(
    val spans: List<TestSpan> = emptyList(),
    val schedule: Duration = 168.hours
)

@Serializable
data class TestSpan(
    val start: Long,
    val end: Long
)

@Serializable
data class TemperatureMonitoring(
    val enabled: Boolean = true,
    val warning: Int = 45,
    val critical: Int = 55,
    val difference: Int = 10
)

@Serializable
data class SmartSchedule(
    val attributeCheck: Duration = 30.minutes,
    val offlineTest: Duration = 4.hours,
    val saveAttributes: Duration = 30.minutes
)

@Serializable
data class SmartNotifications(
    val email: Boolean = true,
    val desktop: Boolean = true,
    val syslog: Boolean = true,
    val script: String? = null
)

@Serializable
data class UsageMonitoring(
    val enabled: Boolean = true,
    val thresholds: UsageThresholds = UsageThresholds(),
    val quotaCheck: Duration = 1.hours,
    val trends: TrendAnalysis = TrendAnalysis()
)

@Serializable
data class UsageThresholds(
    val warning: Int = 80,
    val critical: Int = 90,
    val predictive: PredictiveThreshold = PredictiveThreshold()
)

@Serializable
data class PredictiveThreshold(
    val enabled: Boolean = true,
    val daysAhead: Int = 7,
    val method: PredictionMethod = PredictionMethod.LINEAR
)

@Serializable
data class TrendAnalysis(
    val enabled: Boolean = true,
    val retention: Duration = 720.hours, // 30 days
    val granularity: Duration = 1.hours
)

@Serializable
data class PerformanceMonitoring(
    val enabled: Boolean = true,
    val metrics: List<PerformanceMetric> = listOf(
        PerformanceMetric.IOPS,
        PerformanceMetric.THROUGHPUT,
        PerformanceMetric.LATENCY,
        PerformanceMetric.QUEUE_DEPTH
    ),
    val interval: Duration = 5.minutes,
    val aggregation: MetricAggregation = MetricAggregation()
)

@Serializable
data class MetricAggregation(
    val window: Duration = 5.minutes,
    val method: AggregationMethod = AggregationMethod.AVERAGE,
    val percentiles: List<Int> = listOf(50, 90, 95, 99)
)

@Serializable
data class HealthMonitoring(
    val enabled: Boolean = true,
    val checks: List<HealthCheck> = emptyList(),
    val interval: Duration = 30.minutes,
    val history: HealthHistory = HealthHistory()
)

@Serializable
data class HealthCheck(
    val name: String,
    val type: HealthCheckType,
    val parameters: Map<String, String> = emptyMap(),
    val severity: CheckSeverity = CheckSeverity.WARNING
)

@Serializable
data class HealthHistory(
    val retention: Duration = 168.hours, // 7 days
    val compression: Boolean = true
)

@Serializable
data class AlertConfiguration(
    val enabled: Boolean = true,
    val channels: List<AlertChannel> = emptyList(),
    val rules: List<AlertRule> = emptyList(),
    val cooldown: Duration = 5.minutes,
    val aggregation: AlertAggregation = AlertAggregation()
)

@Serializable
data class AlertChannel(
    val name: String,
    val type: ChannelType,
    val configuration: Map<String, String> = emptyMap(),
    val filter: AlertFilter? = null
)

@Serializable
data class AlertFilter(
    val severities: List<CheckSeverity> = emptyList(),
    val devices: List<String> = emptyList(),
    val types: List<String> = emptyList()
)

@Serializable
data class AlertRule(
    val name: String,
    val condition: String,
    val action: AlertAction,
    val severity: CheckSeverity = CheckSeverity.WARNING,
    val throttle: Duration? = null
)

@Serializable
data class AlertAction(
    val channels: List<String>,
    val script: String? = null,
    val escalate: EscalationPolicy? = null
)

@Serializable
data class EscalationPolicy(
    val after: Duration,
    val to: List<String>,
    val severity: CheckSeverity? = null
)

@Serializable
data class AlertAggregation(
    val enabled: Boolean = true,
    val window: Duration = 5.minutes,
    val maxAlerts: Int = 10
)

// Monitoring Enums
@Serializable
enum class DeviceType {
    AUTO,
    ATA,
    SCSI,
    SAT,
    NVME
}

@Serializable
enum class AttributeAction {
    LOG,
    EMAIL,
    SCRIPT,
    SHUTDOWN
}

@Serializable
enum class PredictionMethod {
    LINEAR,
    EXPONENTIAL,
    POLYNOMIAL,
    ARIMA
}

@Serializable
enum class PerformanceMetric {
    IOPS,
    THROUGHPUT,
    LATENCY,
    QUEUE_DEPTH,
    UTILIZATION,
    AWAIT,
    SERVICE_TIME
}

@Serializable
enum class AggregationMethod {
    AVERAGE,
    MAX,
    MIN,
    SUM,
    PERCENTILE
}

@Serializable
enum class HealthCheckType {
    FILESYSTEM,
    RAID,
    SMART,
    CAPACITY,
    PERFORMANCE,
    CUSTOM
}

@Serializable
enum class CheckSeverity {
    INFO,
    WARNING,
    ERROR,
    CRITICAL
}

@Serializable
enum class ChannelType {
    EMAIL,
    WEBHOOK,
    SYSLOG,
    DESKTOP,
    SCRIPT,
    SNMP
}