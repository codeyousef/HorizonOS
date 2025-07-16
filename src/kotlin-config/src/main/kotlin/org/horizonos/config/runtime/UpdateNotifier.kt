package org.horizonos.config.runtime

import kotlinx.coroutines.*
import org.horizonos.config.dsl.CompiledConfig
import java.time.LocalDateTime
import java.time.format.DateTimeFormatter
import java.util.concurrent.CopyOnWriteArrayList

/**
 * Handles notifications and logging for live updates
 */
class UpdateNotifier(
    private val notificationHandlers: List<NotificationHandler> = listOf(
        SystemdJournalHandler(),
        ConsoleHandler(),
        FileLogHandler()
    )
) {
    
    private val updateLog = CopyOnWriteArrayList<UpdateEvent>()
    private val formatter = DateTimeFormatter.ofPattern("yyyy-MM-dd HH:mm:ss")
    
    /**
     * Notify that an update is starting
     */
    suspend fun notifyUpdateStarting(config: CompiledConfig) {
        val event = UpdateEvent(
            timestamp = LocalDateTime.now(),
            type = EventType.UPDATE_STARTED,
            message = "Starting live update for configuration: ${config.system.hostname}",
            details = mapOf(
                "hostname" to config.system.hostname,
                "packages" to config.packages.size.toString(),
                "services" to config.services.size.toString()
            )
        )
        
        logEvent(event)
        broadcastNotification(
            level = NotificationLevel.INFO,
            title = "System Update Starting",
            message = "HorizonOS is applying configuration updates..."
        )
    }
    
    /**
     * Notify that no changes are required
     */
    suspend fun notifyNoChanges() {
        val event = UpdateEvent(
            timestamp = LocalDateTime.now(),
            type = EventType.NO_CHANGES,
            message = "No configuration changes detected"
        )
        
        logEvent(event)
        broadcastNotification(
            level = NotificationLevel.INFO,
            title = "No Updates Required",
            message = "System configuration is already up to date"
        )
    }
    
    /**
     * Notify that a reboot is required
     */
    suspend fun notifyRebootRequired(changes: List<ConfigChange>) {
        val event = UpdateEvent(
            timestamp = LocalDateTime.now(),
            type = EventType.REBOOT_REQUIRED,
            message = "Reboot required for ${changes.size} changes",
            details = mapOf(
                "changes" to changes.joinToString(", ") { it.description }
            )
        )
        
        logEvent(event)
        broadcastNotification(
            level = NotificationLevel.WARNING,
            title = "Reboot Required",
            message = "Some changes require a system reboot:\n${changes.joinToString("\n") { "• ${it.description}" }}",
            urgent = true
        )
    }
    
    /**
     * Notify that a change was applied successfully
     */
    suspend fun notifyChangeApplied(change: ConfigChange) {
        val event = UpdateEvent(
            timestamp = LocalDateTime.now(),
            type = EventType.CHANGE_APPLIED,
            message = "Applied: ${change.description}",
            details = mapOf(
                "changeType" to change.type.toString(),
                "impact" to change.impact.toString()
            )
        )
        
        logEvent(event)
        
        // Only notify for significant changes
        if (change.impact >= ImpactLevel.MEDIUM) {
            broadcastNotification(
                level = NotificationLevel.INFO,
                title = "Configuration Updated",
                message = change.description
            )
        }
    }
    
    /**
     * Notify that a change failed
     */
    suspend fun notifyChangeFailed(change: ConfigChange, error: Throwable) {
        val event = UpdateEvent(
            timestamp = LocalDateTime.now(),
            type = EventType.CHANGE_FAILED,
            message = "Failed: ${change.description}",
            error = error,
            details = mapOf(
                "changeType" to change.type.toString(),
                "error" to error.message.orEmpty()
            )
        )
        
        logEvent(event)
        broadcastNotification(
            level = NotificationLevel.ERROR,
            title = "Update Failed",
            message = "Failed to apply: ${change.description}\nError: ${error.message}",
            urgent = true
        )
    }
    
    /**
     * Notify that the update completed
     */
    suspend fun notifyUpdateCompleted(
        appliedChanges: List<ConfigChange>,
        failedChanges: List<Pair<ConfigChange, Throwable>>
    ) {
        val event = UpdateEvent(
            timestamp = LocalDateTime.now(),
            type = EventType.UPDATE_COMPLETED,
            message = "Update completed: ${appliedChanges.size} applied, ${failedChanges.size} failed",
            details = mapOf(
                "applied" to appliedChanges.size.toString(),
                "failed" to failedChanges.size.toString()
            )
        )
        
        logEvent(event)
        
        val message = buildString {
            appendLine("Configuration update completed:")
            appendLine("✓ ${appliedChanges.size} changes applied successfully")
            if (failedChanges.isNotEmpty()) {
                appendLine("✗ ${failedChanges.size} changes failed")
            }
        }
        
        broadcastNotification(
            level = if (failedChanges.isEmpty()) NotificationLevel.SUCCESS else NotificationLevel.WARNING,
            title = "Update Completed",
            message = message
        )
    }
    
    /**
     * Notify that the update failed
     */
    suspend fun notifyUpdateFailed(error: Throwable) {
        val event = UpdateEvent(
            timestamp = LocalDateTime.now(),
            type = EventType.UPDATE_FAILED,
            message = "Update failed: ${error.message}",
            error = error
        )
        
        logEvent(event)
        broadcastNotification(
            level = NotificationLevel.ERROR,
            title = "Update Failed",
            message = "System update failed: ${error.message}",
            urgent = true
        )
    }
    
    /**
     * Notify that rollback is starting
     */
    suspend fun notifyRollbackStarting() {
        val event = UpdateEvent(
            timestamp = LocalDateTime.now(),
            type = EventType.ROLLBACK_STARTED,
            message = "Starting rollback due to update failure"
        )
        
        logEvent(event)
        broadcastNotification(
            level = NotificationLevel.WARNING,
            title = "Rolling Back Changes",
            message = "Reverting system to previous state..."
        )
    }
    
    /**
     * Notify that rollback completed
     */
    suspend fun notifyRollbackCompleted() {
        val event = UpdateEvent(
            timestamp = LocalDateTime.now(),
            type = EventType.ROLLBACK_COMPLETED,
            message = "Rollback completed successfully"
        )
        
        logEvent(event)
        broadcastNotification(
            level = NotificationLevel.INFO,
            title = "Rollback Completed",
            message = "System has been restored to previous state"
        )
    }
    
    /**
     * Notify that rollback failed
     */
    suspend fun notifyRollbackFailed(error: Throwable) {
        val event = UpdateEvent(
            timestamp = LocalDateTime.now(),
            type = EventType.ROLLBACK_FAILED,
            message = "Rollback failed: ${error.message}",
            error = error
        )
        
        logEvent(event)
        broadcastNotification(
            level = NotificationLevel.CRITICAL,
            title = "Rollback Failed",
            message = "Failed to restore system state: ${error.message}",
            urgent = true
        )
    }
    
    /**
     * Get update history
     */
    fun getUpdateHistory(limit: Int = 100): List<UpdateEvent> {
        return updateLog.takeLast(limit)
    }
    
    /**
     * Send progress update
     */
    suspend fun notifyProgress(message: String, percentage: Int) {
        coroutineScope {
            notificationHandlers.forEach { handler ->
                launch {
                    handler.sendProgress(message, percentage)
                }
            }
        }
    }
    
    // ===== Private Helper Methods =====
    
    private fun logEvent(event: UpdateEvent) {
        updateLog.add(event)
        
        // Trim log if it gets too large
        if (updateLog.size > 1000) {
            updateLog.removeAt(0)
        }
    }
    
    private suspend fun broadcastNotification(
        level: NotificationLevel,
        title: String,
        message: String,
        urgent: Boolean = false
    ) = coroutineScope {
        notificationHandlers.forEach { handler ->
            launch {
                handler.sendNotification(level, title, message, urgent)
            }
        }
    }
}

// ===== Notification Handlers =====

interface NotificationHandler {
    suspend fun sendNotification(
        level: NotificationLevel,
        title: String,
        message: String,
        urgent: Boolean
    )
    
    suspend fun sendProgress(message: String, percentage: Int)
}

class SystemdJournalHandler : NotificationHandler {
    override suspend fun sendNotification(
        level: NotificationLevel,
        title: String,
        message: String,
        urgent: Boolean
    ) {
        val priority = when (level) {
            NotificationLevel.DEBUG -> "7"
            NotificationLevel.INFO -> "6"
            NotificationLevel.SUCCESS -> "6"
            NotificationLevel.WARNING -> "4"
            NotificationLevel.ERROR -> "3"
            NotificationLevel.CRITICAL -> "2"
        }
        
        withContext(Dispatchers.IO) {
            ProcessBuilder(
                "systemd-cat",
                "-t", "horizonos-update",
                "-p", priority
            ).start().also { process ->
                process.outputStream.bufferedWriter().use { writer ->
                    writer.write("[$title] $message")
                }
                process.waitFor()
            }
        }
    }
    
    override suspend fun sendProgress(message: String, percentage: Int) {
        // Log progress to journal
        sendNotification(
            NotificationLevel.DEBUG,
            "Progress",
            "$message ($percentage%)",
            false
        )
    }
}

class ConsoleHandler : NotificationHandler {
    override suspend fun sendNotification(
        level: NotificationLevel,
        title: String,
        message: String,
        urgent: Boolean
    ) {
        val color = when (level) {
            NotificationLevel.DEBUG -> "\u001B[37m"     // White
            NotificationLevel.INFO -> "\u001B[34m"      // Blue
            NotificationLevel.SUCCESS -> "\u001B[32m"   // Green
            NotificationLevel.WARNING -> "\u001B[33m"   // Yellow
            NotificationLevel.ERROR -> "\u001B[31m"     // Red
            NotificationLevel.CRITICAL -> "\u001B[35m"  // Magenta
        }
        val reset = "\u001B[0m"
        
        val timestamp = LocalDateTime.now().format(DateTimeFormatter.ISO_LOCAL_TIME)
        println("$color[$timestamp] [$title] $message$reset")
    }
    
    override suspend fun sendProgress(message: String, percentage: Int) {
        print("\r\u001B[K$message [${"=".repeat(percentage / 2)}${" ".repeat(50 - percentage / 2)}] $percentage%")
        if (percentage >= 100) println()
    }
}

class FileLogHandler(
    private val logFile: String = "/var/log/horizonos/updates.log"
) : NotificationHandler {
    
    override suspend fun sendNotification(
        level: NotificationLevel,
        title: String,
        message: String,
        urgent: Boolean
    ) = withContext(Dispatchers.IO) {
        val timestamp = LocalDateTime.now().format(DateTimeFormatter.ISO_LOCAL_DATE_TIME)
        val logEntry = "[$timestamp] [$level] [$title] $message\n"
        
        java.io.File(logFile).parentFile.mkdirs()
        java.io.File(logFile).appendText(logEntry)
    }
    
    override suspend fun sendProgress(message: String, percentage: Int) {
        // Don't log progress to file
    }
}

// ===== Data Classes =====

data class UpdateEvent(
    val timestamp: LocalDateTime,
    val type: EventType,
    val message: String,
    val error: Throwable? = null,
    val details: Map<String, String> = emptyMap()
)

enum class EventType {
    UPDATE_STARTED,
    NO_CHANGES,
    REBOOT_REQUIRED,
    CHANGE_APPLIED,
    CHANGE_FAILED,
    UPDATE_COMPLETED,
    UPDATE_FAILED,
    ROLLBACK_STARTED,
    ROLLBACK_COMPLETED,
    ROLLBACK_FAILED
}

enum class NotificationLevel {
    DEBUG,
    INFO,
    SUCCESS,
    WARNING,
    ERROR,
    CRITICAL
}