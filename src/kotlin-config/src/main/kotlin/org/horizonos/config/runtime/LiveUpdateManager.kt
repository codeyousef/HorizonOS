package org.horizonos.config.runtime

import kotlinx.coroutines.*
import org.horizonos.config.dsl.*
import java.time.LocalDateTime
import kotlin.time.Duration
import kotlin.time.Duration.Companion.seconds

/**
 * Manages live updates to the system without requiring reboots
 * Coordinates the application of configuration changes to a running system
 */
class LiveUpdateManager(
    private val systemManager: SystemManager,
    private val changeDetector: ChangeDetector,
    private val stateSync: StateSyncManager,
    private val serviceReloader: ServiceReloader,
    private val notifier: UpdateNotifier
) {
    
    /**
     * Apply configuration updates to a live system
     */
    suspend fun applyLiveUpdates(
        currentConfig: CompiledConfig,
        newConfig: CompiledConfig,
        options: LiveUpdateOptions = LiveUpdateOptions()
    ): LiveUpdateResult = coroutineScope {
        
        notifier.notifyUpdateStarting(newConfig)
        
        try {
            // Detect changes between configurations
            val changes = changeDetector.detectChanges(currentConfig, newConfig)
            
            if (changes.isEmpty()) {
                notifier.notifyNoChanges()
                return@coroutineScope LiveUpdateResult.NoChangesRequired
            }
            
            // Categorize changes by update strategy
            val categorizedChanges = categorizeChanges(changes)
            
            // Check if reboot is required
            if (categorizedChanges.rebootRequired.isNotEmpty() && !options.allowPartialUpdate) {
                notifier.notifyRebootRequired(categorizedChanges.rebootRequired)
                return@coroutineScope LiveUpdateResult.RebootRequired(categorizedChanges.rebootRequired)
            }
            
            // Create state snapshot for recovery
            val stateSnapshot = stateSync.createSnapshot()
            
            val appliedChanges = mutableListOf<ConfigChange>()
            val failedChanges = mutableListOf<Pair<ConfigChange, Throwable>>()
            
            try {
                // Apply live-updatable changes
                for (change in categorizedChanges.liveUpdatable) {
                    try {
                        applyLiveChange(change, options)
                        appliedChanges.add(change)
                        notifier.notifyChangeApplied(change)
                    } catch (e: Exception) {
                        failedChanges.add(change to e)
                        notifier.notifyChangeFailed(change, e)
                        
                        if (!options.continueOnError) {
                            throw LiveUpdateException("Failed to apply change: ${change.description}", e)
                        }
                    }
                }
                
                // Apply service reloads
                for (change in categorizedChanges.serviceReloads) {
                    try {
                        applyServiceReload(change)
                        appliedChanges.add(change)
                        notifier.notifyChangeApplied(change)
                    } catch (e: Exception) {
                        failedChanges.add(change to e)
                        notifier.notifyChangeFailed(change, e)
                        
                        if (!options.continueOnError) {
                            throw LiveUpdateException("Failed to reload service: ${change.description}", e)
                        }
                    }
                }
                
                // Sync state after all changes
                stateSync.syncState(newConfig)
                
                notifier.notifyUpdateCompleted(appliedChanges, failedChanges)
                
                return@coroutineScope if (failedChanges.isEmpty()) {
                    LiveUpdateResult.Success(
                        appliedChanges = appliedChanges,
                        pendingRebootChanges = categorizedChanges.rebootRequired
                    )
                } else {
                    LiveUpdateResult.PartialSuccess(
                        appliedChanges = appliedChanges,
                        failedChanges = failedChanges,
                        pendingRebootChanges = categorizedChanges.rebootRequired
                    )
                }
                
            } catch (e: Exception) {
                // Rollback on failure if requested
                if (options.rollbackOnFailure) {
                    notifier.notifyRollbackStarting()
                    try {
                        stateSync.restoreSnapshot(stateSnapshot)
                        notifier.notifyRollbackCompleted()
                    } catch (rollbackError: Exception) {
                        notifier.notifyRollbackFailed(rollbackError)
                        throw LiveUpdateException(
                            "Update failed and rollback also failed", 
                            rollbackError
                        )
                    }
                }
                throw e
            }
            
        } catch (e: Exception) {
            notifier.notifyUpdateFailed(e)
            return@coroutineScope LiveUpdateResult.Failed(e)
        }
    }
    
    /**
     * Categorize changes by update strategy
     */
    private fun categorizeChanges(changes: List<ConfigChange>): CategorizedChanges {
        val liveUpdatable = mutableListOf<ConfigChange>()
        val serviceReloads = mutableListOf<ConfigChange>()
        val rebootRequired = mutableListOf<ConfigChange>()
        
        for (change in changes) {
            when (change.updateStrategy) {
                UpdateStrategy.LIVE -> liveUpdatable.add(change)
                UpdateStrategy.SERVICE_RELOAD -> serviceReloads.add(change)
                UpdateStrategy.REBOOT_REQUIRED -> rebootRequired.add(change)
            }
        }
        
        return CategorizedChanges(liveUpdatable, serviceReloads, rebootRequired)
    }
    
    /**
     * Apply a live change to the system
     */
    private suspend fun applyLiveChange(change: ConfigChange, options: LiveUpdateOptions) {
        when (change.type) {
            ChangeType.PACKAGE_INSTALL -> {
                systemManager.installPackages(
                    change.newValue as List<Package>, 
                    dryRun = options.dryRun
                )
            }
            ChangeType.PACKAGE_REMOVE -> {
                systemManager.removePackages(
                    change.oldValue as List<Package>,
                    dryRun = options.dryRun
                )
            }
            ChangeType.USER_ADD -> {
                systemManager.createUsers(
                    change.newValue as List<User>,
                    dryRun = options.dryRun
                )
            }
            ChangeType.USER_MODIFY -> {
                systemManager.modifyUser(
                    change.oldValue as User,
                    change.newValue as User,
                    dryRun = options.dryRun
                )
            }
            ChangeType.SERVICE_CONFIG -> {
                systemManager.updateServiceConfig(
                    change.affectedService!!,
                    change.newValue as ServiceConfig,
                    dryRun = options.dryRun
                )
            }
            ChangeType.SYSTEM_CONFIG -> {
                when (change.field) {
                    "timezone" -> systemManager.setTimezone(change.newValue as String, dryRun = options.dryRun)
                    "locale" -> systemManager.setLocale(change.newValue as String, dryRun = options.dryRun)
                    "hostname" -> systemManager.setHostname(change.newValue as String, dryRun = options.dryRun)
                }
            }
            ChangeType.AUTOMATION_WORKFLOW -> {
                systemManager.updateAutomationWorkflow(
                    change.newValue as Workflow,
                    dryRun = options.dryRun
                )
            }
            else -> {
                throw LiveUpdateException("Unsupported live change type: ${change.type}")
            }
        }
    }
    
    /**
     * Apply a service reload
     */
    private suspend fun applyServiceReload(change: ConfigChange) {
        val serviceName = change.affectedService 
            ?: throw LiveUpdateException("Service name required for reload")
            
        serviceReloader.reloadService(serviceName, graceful = true)
    }
    
    /**
     * Check if live updates are possible for the given configuration
     */
    suspend fun canApplyLiveUpdates(
        currentConfig: CompiledConfig,
        newConfig: CompiledConfig
    ): LiveUpdateCapability {
        val changes = changeDetector.detectChanges(currentConfig, newConfig)
        val categorized = categorizeChanges(changes)
        
        return LiveUpdateCapability(
            canFullyUpdate = categorized.rebootRequired.isEmpty(),
            liveUpdatableChanges = categorized.liveUpdatable.size + categorized.serviceReloads.size,
            rebootRequiredChanges = categorized.rebootRequired.size,
            estimatedDuration = estimateUpdateDuration(categorized)
        )
    }
    
    /**
     * Estimate how long the update will take
     */
    private fun estimateUpdateDuration(categorized: CategorizedChanges): Duration {
        var totalSeconds = 0L
        
        // Estimate based on change types
        totalSeconds += categorized.liveUpdatable.sumOf { change ->
            when (change.type) {
                ChangeType.PACKAGE_INSTALL -> 30L * (change.newValue as List<*>).size
                ChangeType.PACKAGE_REMOVE -> 10L * (change.oldValue as List<*>).size
                ChangeType.USER_ADD -> 5L
                ChangeType.USER_MODIFY -> 3L
                else -> 2L
            }
        }
        
        totalSeconds += categorized.serviceReloads.size * 5L
        
        return totalSeconds.seconds
    }
}

// ===== Data Classes =====

data class LiveUpdateOptions(
    val dryRun: Boolean = false,
    val allowPartialUpdate: Boolean = true,
    val continueOnError: Boolean = true,
    val rollbackOnFailure: Boolean = true,
    val userConfirmation: Boolean = true,
    val maxParallelOperations: Int = 4
)

sealed class LiveUpdateResult {
    object NoChangesRequired : LiveUpdateResult()
    
    data class Success(
        val appliedChanges: List<ConfigChange>,
        val pendingRebootChanges: List<ConfigChange> = emptyList()
    ) : LiveUpdateResult()
    
    data class PartialSuccess(
        val appliedChanges: List<ConfigChange>,
        val failedChanges: List<Pair<ConfigChange, Throwable>>,
        val pendingRebootChanges: List<ConfigChange> = emptyList()
    ) : LiveUpdateResult()
    
    data class RebootRequired(
        val changes: List<ConfigChange>
    ) : LiveUpdateResult()
    
    data class Failed(
        val error: Throwable
    ) : LiveUpdateResult()
}

data class LiveUpdateCapability(
    val canFullyUpdate: Boolean,
    val liveUpdatableChanges: Int,
    val rebootRequiredChanges: Int,
    val estimatedDuration: Duration
)

data class CategorizedChanges(
    val liveUpdatable: List<ConfigChange>,
    val serviceReloads: List<ConfigChange>,
    val rebootRequired: List<ConfigChange>
)

class LiveUpdateException(message: String, cause: Throwable? = null) : Exception(message, cause)