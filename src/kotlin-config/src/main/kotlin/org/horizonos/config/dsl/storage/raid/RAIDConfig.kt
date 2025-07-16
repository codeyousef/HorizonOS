package org.horizonos.config.dsl.storage.raid

import kotlinx.serialization.Serializable
import kotlin.time.Duration
import kotlin.time.Duration.Companion.minutes

// ===== RAID Configuration =====

@Serializable
data class RAIDStorageConfig(
    val enabled: Boolean = false,
    val arrays: List<RAIDArray> = emptyList(),
    val monitoring: RAIDMonitoring = RAIDMonitoring(),
    val hotspares: List<String> = emptyList(),
    val notifications: RAIDNotifications = RAIDNotifications()
)

@Serializable
data class RAIDArray(
    val name: String,
    val level: RAIDLevel,
    val devices: List<String>,
    val spares: List<String> = emptyList(),
    val chunkSize: String? = null,
    val metadata: RAIDMetadata = RAIDMetadata.DEFAULT,
    val bitmap: RAIDBitmap? = null,
    val layout: RAIDLayout? = null,
    val writePolicy: WritePolicy = WritePolicy.WRITE_BACK,
    val readPolicy: ReadPolicy = ReadPolicy.ADAPTIVE,
    val rebuildPriority: RebuildPriority = RebuildPriority.NORMAL
)

@Serializable
data class RAIDBitmap(
    val enabled: Boolean = true,
    val location: BitmapLocation = BitmapLocation.INTERNAL,
    val chunkSize: String? = null,
    val file: String? = null
)

@Serializable
data class RAIDMonitoring(
    val enabled: Boolean = true,
    val checkInterval: Duration = 30.minutes,
    val emailAlerts: EmailAlerts = EmailAlerts(),
    val scriptActions: List<ScriptAction> = emptyList(),
    val monitorDegradedArrays: Boolean = true,
    val monitorRebuildProgress: Boolean = true,
    val monitorBitmap: Boolean = true
)

@Serializable
data class EmailAlerts(
    val enabled: Boolean = false,
    val recipients: List<String> = emptyList(),
    val mailCommand: String = "/usr/sbin/sendmail -t",
    val events: List<RAIDEvent> = listOf(
        RAIDEvent.FAIL,
        RAIDEvent.DEGRADE,
        RAIDEvent.REBUILD_START,
        RAIDEvent.REBUILD_FINISH
    )
)

@Serializable
data class ScriptAction(
    val event: RAIDEvent,
    val script: String,
    val timeout: Duration = 5.minutes
)

@Serializable
data class RAIDNotifications(
    val desktop: Boolean = true,
    val syslog: Boolean = true,
    val urgency: NotificationUrgency = NotificationUrgency.CRITICAL
)

// RAID Enums
@Serializable
enum class RAIDLevel {
    RAID0,
    RAID1,
    RAID4,
    RAID5,
    RAID6,
    RAID10,
    LINEAR,
    MULTIPATH,
    CONTAINER
}

@Serializable
enum class RAIDMetadata {
    DEFAULT,
    METADATA_0_90,
    METADATA_1_0,
    METADATA_1_1,
    METADATA_1_2,
    DDF,
    IMSM
}

@Serializable
enum class BitmapLocation {
    NONE,
    INTERNAL,
    EXTERNAL
}

@Serializable
enum class RAIDLayout {
    LEFT_SYMMETRIC,
    RIGHT_SYMMETRIC,
    LEFT_ASYMMETRIC,
    RIGHT_ASYMMETRIC,
    LA,
    RA,
    LS,
    RS,
    NEAR,
    FAR,
    OFFSET
}

@Serializable
enum class WritePolicy {
    WRITE_THROUGH,
    WRITE_BACK,
    WRITE_BEHIND
}

@Serializable
enum class ReadPolicy {
    ADAPTIVE,
    ROUND_ROBIN,
    SEQUENTIAL
}

@Serializable
enum class RebuildPriority {
    LOW,
    NORMAL,
    HIGH,
    CRITICAL
}

@Serializable
enum class RAIDEvent {
    FAIL,
    FAIL_SPARE,
    SPARE_ACTIVE,
    NEW_ARRAY,
    REBUILD_START,
    REBUILD_FINISH,
    REBUILD_BY_PERCENT,
    DEGRADE,
    MOVE_SPARE,
    SPARE_MISSING,
    TEST_MESSAGE
}

@Serializable
enum class NotificationUrgency {
    LOW,
    NORMAL,
    CRITICAL
}