package org.horizonos.config.dsl.storage.monitoring

import org.horizonos.config.dsl.HorizonOSDsl
import kotlin.time.Duration
import kotlin.time.Duration.Companion.hours
import kotlin.time.Duration.Companion.minutes

// ===== Storage Monitoring DSL Builders =====

@HorizonOSDsl
class StorageMonitoringContext {
    var enabled = true
    private var smart = SmartMonitoring()
    private var usage = UsageMonitoring()
    private var performance = PerformanceMonitoring()
    private var health = HealthMonitoring()
    private var alerts = AlertConfiguration()
    
    fun smart(block: SmartMonitoringContext.() -> Unit) {
        smart = SmartMonitoringContext().apply(block).toConfig()
    }
    
    fun usage(block: UsageMonitoringContext.() -> Unit) {
        usage = UsageMonitoringContext().apply(block).toConfig()
    }
    
    fun performance(block: PerformanceMonitoringContext.() -> Unit) {
        performance = PerformanceMonitoringContext().apply(block).toConfig()
    }
    
    fun health(block: HealthMonitoringContext.() -> Unit) {
        health = HealthMonitoringContext().apply(block).toConfig()
    }
    
    fun alerts(block: AlertConfigurationContext.() -> Unit) {
        alerts = AlertConfigurationContext().apply(block).toConfig()
    }
    
    fun toConfig() = StorageMonitoringConfig(
        enabled = enabled,
        smart = smart,
        usage = usage,
        performance = performance,
        health = health,
        alerts = alerts
    )
}

@HorizonOSDsl
class SmartMonitoringContext {
    var enabled = true
    var daemon = "smartd"
    private val devices = mutableListOf<SmartDevice>()
    private var schedule = SmartSchedule()
    private var notifications = SmartNotifications()
    
    fun device(device: String, block: SmartDeviceContext.() -> Unit = {}) {
        val context = SmartDeviceContext(device).apply(block)
        devices.add(context.toConfig())
    }
    
    fun schedule(block: SmartScheduleContext.() -> Unit) {
        schedule = SmartScheduleContext().apply(block).toConfig()
    }
    
    fun notifications(block: SmartNotificationsContext.() -> Unit) {
        notifications = SmartNotificationsContext().apply(block).toConfig()
    }
    
    fun toConfig() = SmartMonitoring(
        enabled = enabled,
        daemon = daemon,
        devices = devices,
        schedule = schedule,
        notifications = notifications
    )
}

@HorizonOSDsl
class SmartDeviceContext(private val device: String) {
    var type = DeviceType.AUTO
    private val attributes = mutableListOf<SmartAttribute>()
    private var tests = SmartTests()
    private var temperature = TemperatureMonitoring()
    
    fun attribute(id: Int, name: String, threshold: Int? = null, action: AttributeAction = AttributeAction.LOG) {
        attributes.add(SmartAttribute(id, name, threshold, action))
    }
    
    fun tests(block: SmartTestsContext.() -> Unit) {
        tests = SmartTestsContext().apply(block).toConfig()
    }
    
    fun temperature(block: TemperatureMonitoringContext.() -> Unit) {
        temperature = TemperatureMonitoringContext().apply(block).toConfig()
    }
    
    fun toConfig() = SmartDevice(
        device = device,
        type = type,
        attributes = attributes,
        tests = tests,
        temperature = temperature
    )
}

@HorizonOSDsl
class SmartTestsContext {
    var shortTest: Duration? = 24.hours
    var longTest: Duration? = 168.hours
    var conveyanceTest: Duration? = null
    private var selectiveTest: SelectiveTest? = null
    
    fun selective(schedule: Duration = 168.hours, block: SelectiveTestContext.() -> Unit) {
        selectiveTest = SelectiveTestContext(schedule).apply(block).toConfig()
    }
    
    fun toConfig() = SmartTests(
        shortTest = shortTest,
        longTest = longTest,
        conveyanceTest = conveyanceTest,
        selectiveTest = selectiveTest
    )
}

@HorizonOSDsl
class SelectiveTestContext(private val schedule: Duration) {
    private val spans = mutableListOf<TestSpan>()
    
    fun span(start: Long, end: Long) {
        spans.add(TestSpan(start, end))
    }
    
    fun toConfig() = SelectiveTest(
        spans = spans,
        schedule = schedule
    )
}

@HorizonOSDsl
class TemperatureMonitoringContext {
    var enabled = true
    var warning = 45
    var critical = 55
    var difference = 10
    
    fun toConfig() = TemperatureMonitoring(
        enabled = enabled,
        warning = warning,
        critical = critical,
        difference = difference
    )
}

@HorizonOSDsl
class SmartScheduleContext {
    var attributeCheck: Duration = 30.minutes
    var offlineTest: Duration = 4.hours
    var saveAttributes: Duration = 30.minutes
    
    fun toConfig() = SmartSchedule(
        attributeCheck = attributeCheck,
        offlineTest = offlineTest,
        saveAttributes = saveAttributes
    )
}

@HorizonOSDsl
class SmartNotificationsContext {
    var email = true
    var desktop = true
    var syslog = true
    var script: String? = null
    
    fun toConfig() = SmartNotifications(
        email = email,
        desktop = desktop,
        syslog = syslog,
        script = script
    )
}

@HorizonOSDsl
class UsageMonitoringContext {
    var enabled = true
    private var thresholds = UsageThresholds()
    var quotaCheck: Duration = 1.hours
    private var trends = TrendAnalysis()
    
    fun thresholds(block: UsageThresholdsContext.() -> Unit) {
        thresholds = UsageThresholdsContext().apply(block).toConfig()
    }
    
    fun trends(block: TrendAnalysisContext.() -> Unit) {
        trends = TrendAnalysisContext().apply(block).toConfig()
    }
    
    fun toConfig() = UsageMonitoring(
        enabled = enabled,
        thresholds = thresholds,
        quotaCheck = quotaCheck,
        trends = trends
    )
}

@HorizonOSDsl
class UsageThresholdsContext {
    var warning = 80
    var critical = 90
    private var predictive = PredictiveThreshold()
    
    fun predictive(block: PredictiveThresholdContext.() -> Unit) {
        predictive = PredictiveThresholdContext().apply(block).toConfig()
    }
    
    fun toConfig() = UsageThresholds(
        warning = warning,
        critical = critical,
        predictive = predictive
    )
}

@HorizonOSDsl
class PredictiveThresholdContext {
    var enabled = true
    var daysAhead = 7
    var method = PredictionMethod.LINEAR
    
    fun toConfig() = PredictiveThreshold(
        enabled = enabled,
        daysAhead = daysAhead,
        method = method
    )
}

@HorizonOSDsl
class TrendAnalysisContext {
    var enabled = true
    var retention: Duration = 720.hours
    var granularity: Duration = 1.hours
    
    fun toConfig() = TrendAnalysis(
        enabled = enabled,
        retention = retention,
        granularity = granularity
    )
}

@HorizonOSDsl
class PerformanceMonitoringContext {
    var enabled = true
    val metrics = mutableListOf(
        PerformanceMetric.IOPS,
        PerformanceMetric.THROUGHPUT,
        PerformanceMetric.LATENCY,
        PerformanceMetric.QUEUE_DEPTH
    )
    var interval: Duration = 5.minutes
    private var aggregation = MetricAggregation()
    
    fun metric(metric: PerformanceMetric) {
        metrics.add(metric)
    }
    
    fun aggregation(block: MetricAggregationContext.() -> Unit) {
        aggregation = MetricAggregationContext().apply(block).toConfig()
    }
    
    fun toConfig() = PerformanceMonitoring(
        enabled = enabled,
        metrics = metrics,
        interval = interval,
        aggregation = aggregation
    )
}

@HorizonOSDsl
class MetricAggregationContext {
    var window: Duration = 5.minutes
    var method = AggregationMethod.AVERAGE
    val percentiles = mutableListOf(50, 90, 95, 99)
    
    fun percentile(value: Int) {
        percentiles.add(value)
    }
    
    fun toConfig() = MetricAggregation(
        window = window,
        method = method,
        percentiles = percentiles
    )
}

@HorizonOSDsl
class HealthMonitoringContext {
    var enabled = true
    private val checks = mutableListOf<HealthCheck>()
    var interval: Duration = 30.minutes
    private var history = HealthHistory()
    
    fun check(name: String, type: HealthCheckType, block: HealthCheckContext.() -> Unit = {}) {
        val context = HealthCheckContext(name, type).apply(block)
        checks.add(context.toConfig())
    }
    
    fun history(block: HealthHistoryContext.() -> Unit) {
        history = HealthHistoryContext().apply(block).toConfig()
    }
    
    fun toConfig() = HealthMonitoring(
        enabled = enabled,
        checks = checks,
        interval = interval,
        history = history
    )
}

@HorizonOSDsl
class HealthCheckContext(
    private val name: String,
    private val type: HealthCheckType
) {
    val parameters = mutableMapOf<String, String>()
    var severity = CheckSeverity.WARNING
    
    fun parameter(key: String, value: String) {
        parameters[key] = value
    }
    
    fun toConfig() = HealthCheck(
        name = name,
        type = type,
        parameters = parameters,
        severity = severity
    )
}

@HorizonOSDsl
class HealthHistoryContext {
    var retention: Duration = 168.hours
    var compression = true
    
    fun toConfig() = HealthHistory(
        retention = retention,
        compression = compression
    )
}

@HorizonOSDsl
class AlertConfigurationContext {
    var enabled = true
    private val channels = mutableListOf<AlertChannel>()
    private val rules = mutableListOf<AlertRule>()
    var cooldown: Duration = 5.minutes
    private var aggregation = AlertAggregation()
    
    fun channel(name: String, type: ChannelType, block: AlertChannelContext.() -> Unit = {}) {
        val context = AlertChannelContext(name, type).apply(block)
        channels.add(context.toConfig())
    }
    
    fun rule(name: String, condition: String, block: AlertRuleContext.() -> Unit) {
        val context = AlertRuleContext(name, condition).apply(block)
        rules.add(context.toConfig())
    }
    
    fun aggregation(block: AlertAggregationContext.() -> Unit) {
        aggregation = AlertAggregationContext().apply(block).toConfig()
    }
    
    fun toConfig() = AlertConfiguration(
        enabled = enabled,
        channels = channels,
        rules = rules,
        cooldown = cooldown,
        aggregation = aggregation
    )
}

@HorizonOSDsl
class AlertChannelContext(
    private val name: String,
    private val type: ChannelType
) {
    val configuration = mutableMapOf<String, String>()
    private var filter: AlertFilter? = null
    
    fun config(key: String, value: String) {
        configuration[key] = value
    }
    
    fun filter(block: AlertFilterContext.() -> Unit) {
        filter = AlertFilterContext().apply(block).toConfig()
    }
    
    fun toConfig() = AlertChannel(
        name = name,
        type = type,
        configuration = configuration,
        filter = filter
    )
}

@HorizonOSDsl
class AlertFilterContext {
    val severities = mutableListOf<CheckSeverity>()
    val devices = mutableListOf<String>()
    val types = mutableListOf<String>()
    
    fun severity(sev: CheckSeverity) {
        severities.add(sev)
    }
    
    fun device(dev: String) {
        devices.add(dev)
    }
    
    fun type(typ: String) {
        types.add(typ)
    }
    
    fun toConfig() = AlertFilter(
        severities = severities,
        devices = devices,
        types = types
    )
}

@HorizonOSDsl
class AlertRuleContext(
    private val name: String,
    private val condition: String
) {
    private lateinit var action: AlertAction
    var severity = CheckSeverity.WARNING
    var throttle: Duration? = null
    
    fun action(block: AlertActionContext.() -> Unit) {
        action = AlertActionContext().apply(block).toConfig()
    }
    
    fun toConfig() = AlertRule(
        name = name,
        condition = condition,
        action = action,
        severity = severity,
        throttle = throttle
    )
}

@HorizonOSDsl
class AlertActionContext {
    val channels = mutableListOf<String>()
    var script: String? = null
    private var escalate: EscalationPolicy? = null
    
    fun channel(name: String) {
        channels.add(name)
    }
    
    fun escalate(after: Duration, block: EscalationPolicyContext.() -> Unit) {
        escalate = EscalationPolicyContext(after).apply(block).toConfig()
    }
    
    fun toConfig() = AlertAction(
        channels = channels,
        script = script,
        escalate = escalate
    )
}

@HorizonOSDsl
class EscalationPolicyContext(private val after: Duration) {
    val to = mutableListOf<String>()
    var severity: CheckSeverity? = null
    
    fun escalateTo(channel: String) {
        to.add(channel)
    }
    
    fun toConfig() = EscalationPolicy(
        after = after,
        to = to,
        severity = severity
    )
}

@HorizonOSDsl
class AlertAggregationContext {
    var enabled = true
    var window: Duration = 5.minutes
    var maxAlerts = 10
    
    fun toConfig() = AlertAggregation(
        enabled = enabled,
        window = window,
        maxAlerts = maxAlerts
    )
}