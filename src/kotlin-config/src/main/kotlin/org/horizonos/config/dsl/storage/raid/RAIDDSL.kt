package org.horizonos.config.dsl.storage.raid

import org.horizonos.config.dsl.HorizonOSDsl
import kotlin.time.Duration
import kotlin.time.Duration.Companion.minutes

// ===== RAID DSL Builders =====

@HorizonOSDsl
class RAIDStorageContext {
    var enabled = false
    private val arrays = mutableListOf<RAIDArray>()
    private var monitoring = RAIDMonitoring()
    private val hotspares = mutableListOf<String>()
    private var notifications = RAIDNotifications()
    
    fun array(name: String, level: RAIDLevel, block: RAIDArrayContext.() -> Unit) {
        val context = RAIDArrayContext(name, level).apply(block)
        arrays.add(context.toConfig())
    }
    
    fun monitoring(block: RAIDMonitoringContext.() -> Unit) {
        monitoring = RAIDMonitoringContext().apply(block).toConfig()
    }
    
    fun hotspare(device: String) {
        hotspares.add(device)
    }
    
    fun notifications(block: RAIDNotificationsContext.() -> Unit) {
        notifications = RAIDNotificationsContext().apply(block).toConfig()
    }
    
    fun toConfig() = RAIDStorageConfig(
        enabled = enabled,
        arrays = arrays,
        monitoring = monitoring,
        hotspares = hotspares,
        notifications = notifications
    )
}

@HorizonOSDsl
class RAIDArrayContext(
    private val name: String,
    private val level: RAIDLevel
) {
    private val devices = mutableListOf<String>()
    private val spares = mutableListOf<String>()
    var chunkSize: String? = null
    var metadata = RAIDMetadata.DEFAULT
    private var bitmap: RAIDBitmap? = null
    var layout: RAIDLayout? = null
    var writePolicy = WritePolicy.WRITE_BACK
    var readPolicy = ReadPolicy.ADAPTIVE
    var rebuildPriority = RebuildPriority.NORMAL
    
    fun device(dev: String) {
        devices.add(dev)
    }
    
    fun spare(dev: String) {
        spares.add(dev)
    }
    
    fun bitmap(block: RAIDBitmapContext.() -> Unit) {
        bitmap = RAIDBitmapContext().apply(block).toConfig()
    }
    
    fun toConfig() = RAIDArray(
        name = name,
        level = level,
        devices = devices,
        spares = spares,
        chunkSize = chunkSize,
        metadata = metadata,
        bitmap = bitmap,
        layout = layout,
        writePolicy = writePolicy,
        readPolicy = readPolicy,
        rebuildPriority = rebuildPriority
    )
}

@HorizonOSDsl
class RAIDBitmapContext {
    var enabled = true
    var location = BitmapLocation.INTERNAL
    var chunkSize: String? = null
    var file: String? = null
    
    fun toConfig() = RAIDBitmap(
        enabled = enabled,
        location = location,
        chunkSize = chunkSize,
        file = file
    )
}

@HorizonOSDsl
class RAIDMonitoringContext {
    var enabled = true
    var checkInterval: Duration = 30.minutes
    private var emailAlerts = EmailAlerts()
    private val scriptActions = mutableListOf<ScriptAction>()
    var monitorDegradedArrays = true
    var monitorRebuildProgress = true
    var monitorBitmap = true
    
    fun emailAlerts(block: EmailAlertsContext.() -> Unit) {
        emailAlerts = EmailAlertsContext().apply(block).toConfig()
    }
    
    fun scriptAction(event: RAIDEvent, script: String, timeout: Duration = 5.minutes) {
        scriptActions.add(ScriptAction(event, script, timeout))
    }
    
    fun toConfig() = RAIDMonitoring(
        enabled = enabled,
        checkInterval = checkInterval,
        emailAlerts = emailAlerts,
        scriptActions = scriptActions,
        monitorDegradedArrays = monitorDegradedArrays,
        monitorRebuildProgress = monitorRebuildProgress,
        monitorBitmap = monitorBitmap
    )
}

@HorizonOSDsl
class EmailAlertsContext {
    var enabled = false
    private val recipients = mutableListOf<String>()
    var mailCommand = "/usr/sbin/sendmail -t"
    private val events = mutableListOf(
        RAIDEvent.FAIL,
        RAIDEvent.DEGRADE,
        RAIDEvent.REBUILD_START,
        RAIDEvent.REBUILD_FINISH
    )
    
    fun recipient(email: String) {
        recipients.add(email)
    }
    
    fun event(evt: RAIDEvent) {
        events.add(evt)
    }
    
    fun toConfig() = EmailAlerts(
        enabled = enabled,
        recipients = recipients,
        mailCommand = mailCommand,
        events = events
    )
}

@HorizonOSDsl
class RAIDNotificationsContext {
    var desktop = true
    var syslog = true
    var urgency = NotificationUrgency.CRITICAL
    
    fun toConfig() = RAIDNotifications(
        desktop = desktop,
        syslog = syslog,
        urgency = urgency
    )
}