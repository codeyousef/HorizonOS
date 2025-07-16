package org.horizonos.config.runtime

import kotlinx.coroutines.*
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import org.horizonos.config.dsl.CompiledConfig
import java.io.File
import java.time.LocalDateTime
import java.util.concurrent.ConcurrentHashMap

/**
 * Manages system state synchronization and snapshots for recovery
 */
class StateSyncManager(
    private val stateDir: File = File("/var/lib/horizonos/state")
) {
    
    private val json = Json { 
        prettyPrint = true
        encodeDefaults = true
    }
    
    private val currentState = ConcurrentHashMap<String, Any>()
    private var lastConfig: CompiledConfig? = null
    
    init {
        stateDir.mkdirs()
        loadCurrentState()
    }
    
    /**
     * Create a snapshot of the current system state
     */
    suspend fun createSnapshot(): StateSnapshot = coroutineScope {
        val timestamp = LocalDateTime.now()
        val snapshotId = "snapshot-${timestamp.toString().replace(":", "-")}"
        val snapshotDir = File(stateDir, snapshotId)
        snapshotDir.mkdirs()
        
        // Save current configuration
        val configFile = File(snapshotDir, "config.json")
        lastConfig?.let { config ->
            configFile.writeText(json.encodeToString(CompiledConfig.serializer(), config))
        }
        
        // Capture system state
        val systemState = captureSystemState()
        val stateFile = File(snapshotDir, "system-state.json")
        stateFile.writeText(json.encodeToString(systemState))
        
        // Capture service states
        val serviceStates = captureServiceStates()
        val servicesFile = File(snapshotDir, "services.json")
        servicesFile.writeText(json.encodeToString(serviceStates))
        
        // Capture package list
        val packages = captureInstalledPackages()
        val packagesFile = File(snapshotDir, "packages.txt")
        packagesFile.writeText(packages.joinToString("\n"))
        
        return@coroutineScope StateSnapshot(
            id = snapshotId,
            timestamp = timestamp,
            configPath = configFile.absolutePath,
            statePath = stateFile.absolutePath,
            servicesPath = servicesFile.absolutePath,
            packagesPath = packagesFile.absolutePath
        )
    }
    
    /**
     * Restore system state from a snapshot
     */
    suspend fun restoreSnapshot(snapshot: StateSnapshot) = coroutineScope {
        // Load snapshot data
        val configFile = File(snapshot.configPath)
        val stateFile = File(snapshot.statePath)
        val servicesFile = File(snapshot.servicesPath)
        val packagesFile = File(snapshot.packagesPath)
        
        if (!configFile.exists() || !stateFile.exists()) {
            throw StateSyncException("Snapshot files not found: ${snapshot.id}")
        }
        
        // Restore configuration
        val config = json.decodeFromString(CompiledConfig.serializer(), configFile.readText())
        
        // Restore system state
        val systemState = json.decodeFromString<SystemState>(stateFile.readText())
        applySystemState(systemState)
        
        // Restore service states
        if (servicesFile.exists()) {
            val serviceStates = json.decodeFromString<Map<String, ServiceState>>(servicesFile.readText())
            restoreServiceStates(serviceStates)
        }
        
        // Note: Package restoration would need to be handled carefully
        // to avoid breaking the system
        
        lastConfig = config
        saveCurrentState()
    }
    
    /**
     * Sync current state with new configuration
     */
    suspend fun syncState(config: CompiledConfig) {
        lastConfig = config
        
        // Update state tracking
        currentState["last_sync"] = LocalDateTime.now().toString()
        currentState["config_hash"] = config.hashCode().toString()
        currentState["hostname"] = config.system.hostname
        currentState["timezone"] = config.system.timezone
        currentState["locale"] = config.system.locale
        
        // Track services - store as simple strings/booleans
        val serviceStates = config.services.associate { service ->
            service.name to service.enabled.toString()
        }
        currentState["services"] = json.encodeToString(serviceStates)
        
        // Track packages
        val installedPackages = config.packages
            .filter { it.action == org.horizonos.config.dsl.PackageAction.INSTALL }
            .map { it.name }
        currentState["packages"] = json.encodeToString(installedPackages)
        
        // Track users
        val users = config.users.map { it.name }
        currentState["users"] = json.encodeToString(users)
        
        saveCurrentState()
    }
    
    /**
     * Get current system state
     */
    fun getCurrentState(): Map<String, Any> = currentState.toMap()
    
    /**
     * Check if system is in sync with configuration
     */
    suspend fun checkSync(config: CompiledConfig): SyncStatus {
        val issues = mutableListOf<SyncIssue>()
        
        // Check system configuration
        val currentHostname = runCommand("hostname").trim()
        if (currentHostname != config.system.hostname) {
            issues.add(SyncIssue(
                component = "system",
                field = "hostname",
                expected = config.system.hostname,
                actual = currentHostname
            ))
        }
        
        // Check services
        config.services.forEach { service ->
            val isEnabled = checkServiceEnabled(service.name)
            if (isEnabled != service.enabled) {
                issues.add(SyncIssue(
                    component = "service",
                    field = service.name,
                    expected = service.enabled.toString(),
                    actual = isEnabled.toString()
                ))
            }
        }
        
        // Check packages
        val installedPackages = captureInstalledPackages().toSet()
        config.packages
            .filter { it.action == org.horizonos.config.dsl.PackageAction.INSTALL }
            .forEach { pkg ->
                if (!installedPackages.contains(pkg.name)) {
                    issues.add(SyncIssue(
                        component = "package",
                        field = pkg.name,
                        expected = "installed",
                        actual = "not installed"
                    ))
                }
            }
        
        return if (issues.isEmpty()) {
            SyncStatus.InSync
        } else {
            SyncStatus.OutOfSync(issues)
        }
    }
    
    /**
     * List available snapshots
     */
    fun listSnapshots(): List<SnapshotInfo> {
        return stateDir.listFiles { file -> file.isDirectory && file.name.startsWith("snapshot-") }
            ?.mapNotNull { dir ->
                try {
                    val configFile = File(dir, "config.json")
                    // Parse the timestamp carefully - it may have colons replaced with dashes
                    val timestampStr = dir.name.removePrefix("snapshot-")
                    val timestamp = if (timestampStr.count { it == '-' } > 2) {
                        // Format: YYYY-MM-DDTHH-MM-SS.nnnnnnnnn
                        val parts = timestampStr.split("T")
                        if (parts.size == 2) {
                            val datePart = parts[0]
                            val timePart = parts[1].take(8).replace("-", ":")
                            LocalDateTime.parse("${datePart}T${timePart}")
                        } else {
                            LocalDateTime.now() // Fallback
                        }
                    } else {
                        LocalDateTime.now() // Fallback
                    }
                    
                    SnapshotInfo(
                        id = dir.name,
                        timestamp = timestamp,
                        size = dir.walkTopDown().sumOf { it.length() },
                        hasConfig = configFile.exists()
                    )
                } catch (e: Exception) {
                    null // Skip invalid snapshots
                }
            }
            ?.sortedByDescending { it.timestamp }
            ?: emptyList()
    }
    
    /**
     * Clean up old snapshots
     */
    suspend fun cleanupSnapshots(keepCount: Int = 10) {
        val snapshots = listSnapshots()
        if (snapshots.size > keepCount) {
            snapshots.drop(keepCount).forEach { snapshot ->
                File(stateDir, snapshot.id).deleteRecursively()
            }
        }
    }
    
    // ===== Private Helper Methods =====
    
    private fun loadCurrentState() {
        val stateFile = File(stateDir, "current-state.json")
        if (stateFile.exists()) {
            try {
                val loaded = json.decodeFromString<Map<String, Any>>(stateFile.readText())
                currentState.putAll(loaded)
            } catch (e: Exception) {
                // Ignore errors loading state
            }
        }
    }
    
    private fun saveCurrentState() {
        val stateFile = File(stateDir, "current-state.json")
        // Convert to a serializable map with string values
        val serializableState = currentState.mapValues { it.value.toString() }
        stateFile.writeText(json.encodeToString(serializableState))
    }
    
    private suspend fun captureSystemState(): SystemState {
        return SystemState(
            hostname = runCommand("hostname").trim(),
            timezone = runCommand("timedatectl show -p Timezone --value").trim(),
            locale = runCommand("localectl status | grep 'System Locale' | cut -d: -f2").trim(),
            kernel = runCommand("uname -r").trim(),
            uptime = runCommand("uptime -p").trim()
        )
    }
    
    private suspend fun captureServiceStates(): Map<String, ServiceState> {
        val services = runCommand("systemctl list-units --type=service --all --no-pager --plain")
            .lines()
            .drop(1) // Skip header
            .filter { it.isNotBlank() }
            .mapNotNull { line ->
                val parts = line.trim().split(Regex("\\s+"), 5)
                if (parts.size >= 4) {
                    val name = parts[0].removeSuffix(".service")
                    val loaded = parts[1]
                    val active = parts[2]
                    val running = parts[3]
                    name to ServiceState(name, loaded, active, running)
                } else null
            }
            .toMap()
        
        return services
    }
    
    private suspend fun captureInstalledPackages(): List<String> {
        return runCommand("pacman -Qq")
            .lines()
            .filter { it.isNotBlank() }
    }
    
    private suspend fun checkServiceEnabled(serviceName: String): Boolean {
        return try {
            val result = runCommand("systemctl is-enabled $serviceName")
            result.trim() == "enabled"
        } catch (e: Exception) {
            false
        }
    }
    
    private suspend fun applySystemState(state: SystemState) {
        runCommand("hostnamectl set-hostname ${state.hostname}")
        runCommand("timedatectl set-timezone ${state.timezone}")
        // Locale setting would need more careful handling
    }
    
    private suspend fun restoreServiceStates(states: Map<String, ServiceState>) {
        states.forEach { (name, state) ->
            if (state.active == "active") {
                runCommand("systemctl start $name")
            } else {
                runCommand("systemctl stop $name")
            }
        }
    }
    
    private suspend fun runCommand(command: String): String = withContext(Dispatchers.IO) {
        val process = ProcessBuilder("bash", "-c", command)
            .redirectOutput(ProcessBuilder.Redirect.PIPE)
            .redirectError(ProcessBuilder.Redirect.PIPE)
            .start()
        
        process.waitFor()
        process.inputStream.bufferedReader().readText()
    }
}

// ===== Data Classes =====

data class StateSnapshot(
    val id: String,
    val timestamp: LocalDateTime,
    val configPath: String,
    val statePath: String,
    val servicesPath: String,
    val packagesPath: String
)

data class SnapshotInfo(
    val id: String,
    val timestamp: LocalDateTime,
    val size: Long,
    val hasConfig: Boolean
)

@kotlinx.serialization.Serializable
data class SystemState(
    val hostname: String,
    val timezone: String,
    val locale: String,
    val kernel: String,
    val uptime: String
)

@kotlinx.serialization.Serializable
data class ServiceState(
    val name: String,
    val loaded: String,
    val active: String,
    val running: String
)

sealed class SyncStatus {
    object InSync : SyncStatus()
    data class OutOfSync(val issues: List<SyncIssue>) : SyncStatus()
}

data class SyncIssue(
    val component: String,
    val field: String,
    val expected: String,
    val actual: String
)

class StateSyncException(message: String, cause: Throwable? = null) : Exception(message, cause)