package org.horizonos.config.runtime

import kotlinx.coroutines.*
import java.util.concurrent.ConcurrentHashMap
import kotlin.time.Duration
import kotlin.time.Duration.Companion.seconds

/**
 * Manages service configuration reloads without full restarts
 */
class ServiceReloader {
    
    private val reloadStrategies = ConcurrentHashMap<String, ReloadStrategy>()
    private val serviceStates = ConcurrentHashMap<String, ServiceReloadState>()
    
    init {
        // Register known service reload strategies
        registerCommonServices()
    }
    
    /**
     * Reload a service with its new configuration
     */
    suspend fun reloadService(
        serviceName: String,
        graceful: Boolean = true,
        timeout: Duration = 30.seconds
    ): ReloadResult = withTimeout(timeout) {
        
        val state = serviceStates.getOrPut(serviceName) { ServiceReloadState(serviceName) }
        state.lastReloadAttempt = System.currentTimeMillis()
        
        try {
            // Check if service is running
            if (!isServiceRunning(serviceName)) {
                return@withTimeout ReloadResult.ServiceNotRunning(serviceName)
            }
            
            // Get reload strategy
            val strategy = reloadStrategies[serviceName] ?: detectReloadStrategy(serviceName)
            
            // Execute reload based on strategy
            val result = when (strategy) {
                is ReloadStrategy.Signal -> reloadWithSignal(serviceName, strategy.signal)
                is ReloadStrategy.Command -> reloadWithCommand(serviceName, strategy.command)
                is ReloadStrategy.SystemdReload -> reloadWithSystemd(serviceName)
                is ReloadStrategy.RestartRequired -> {
                    if (graceful) {
                        gracefulRestart(serviceName, strategy.gracePeriod)
                    } else {
                        hardRestart(serviceName)
                    }
                }
                is ReloadStrategy.Custom -> strategy.reloadFunction(serviceName)
                ReloadStrategy.NotSupported -> {
                    return@withTimeout ReloadResult.ReloadNotSupported(serviceName)
                }
            }
            
            // Update state
            state.lastSuccessfulReload = System.currentTimeMillis()
            state.reloadCount++
            
            result
        } catch (e: Exception) {
            state.lastError = e
            ReloadResult.Failed(serviceName, e)
        }
    }
    
    /**
     * Reload multiple services in dependency order
     */
    suspend fun reloadServices(
        services: List<String>,
        parallel: Boolean = false
    ): Map<String, ReloadResult> = coroutineScope {
        
        if (parallel) {
            // Reload in parallel
            services.associateWith { service ->
                async { reloadService(service) }
            }.mapValues { it.value.await() }
        } else {
            // Reload sequentially in dependency order
            val ordered = orderByDependencies(services)
            ordered.associateWith { service ->
                reloadService(service)
            }
        }
    }
    
    /**
     * Register a custom reload strategy for a service
     */
    fun registerReloadStrategy(serviceName: String, strategy: ReloadStrategy) {
        reloadStrategies[serviceName] = strategy
    }
    
    /**
     * Get reload state for a service
     */
    fun getServiceReloadState(serviceName: String): ServiceReloadState? {
        return serviceStates[serviceName]
    }
    
    // ===== Private Helper Methods =====
    
    private fun registerCommonServices() {
        // Web servers
        reloadStrategies["nginx"] = ReloadStrategy.Signal("HUP")
        reloadStrategies["apache2"] = ReloadStrategy.Command("apachectl graceful")
        reloadStrategies["httpd"] = ReloadStrategy.Command("apachectl graceful")
        
        // Mail servers
        reloadStrategies["postfix"] = ReloadStrategy.Command("postfix reload")
        reloadStrategies["dovecot"] = ReloadStrategy.Command("doveadm reload")
        
        // DNS servers
        reloadStrategies["bind9"] = ReloadStrategy.Command("rndc reload")
        reloadStrategies["named"] = ReloadStrategy.Command("rndc reload")
        
        // Other services
        reloadStrategies["sshd"] = ReloadStrategy.Signal("HUP")
        reloadStrategies["rsyslog"] = ReloadStrategy.Signal("HUP")
        reloadStrategies["NetworkManager"] = ReloadStrategy.SystemdReload
        reloadStrategies["systemd-resolved"] = ReloadStrategy.SystemdReload
        reloadStrategies["systemd-timesyncd"] = ReloadStrategy.SystemdReload
        
        // Desktop environments
        reloadStrategies["gdm"] = ReloadStrategy.RestartRequired(10.seconds)
        reloadStrategies["sddm"] = ReloadStrategy.RestartRequired(10.seconds)
        reloadStrategies["lightdm"] = ReloadStrategy.RestartRequired(10.seconds)
    }
    
    private suspend fun detectReloadStrategy(serviceName: String): ReloadStrategy {
        // Check if systemd service supports reload
        val supportsReload = try {
            val result = runCommand("systemctl show -p CanReload $serviceName")
            result.contains("CanReload=yes")
        } catch (e: Exception) {
            false
        }
        
        return if (supportsReload) {
            ReloadStrategy.SystemdReload
        } else {
            // Check if service has a known reload command
            val knownCommands = mapOf(
                "reload" to "$serviceName reload",
                "force-reload" to "$serviceName force-reload",
                "graceful" to "$serviceName graceful"
            )
            
            for ((_, command) in knownCommands) {
                if (commandExists(command.split(" ")[0])) {
                    return ReloadStrategy.Command(command)
                }
            }
            
            ReloadStrategy.RestartRequired(5.seconds)
        }
    }
    
    private suspend fun isServiceRunning(serviceName: String): Boolean {
        return try {
            val result = runCommand("systemctl is-active $serviceName")
            result.trim() == "active"
        } catch (e: Exception) {
            false
        }
    }
    
    private suspend fun reloadWithSignal(serviceName: String, signal: String): ReloadResult {
        val pid = getServicePid(serviceName) ?: return ReloadResult.Failed(
            serviceName, 
            Exception("Could not find PID for service")
        )
        
        runCommand("kill -$signal $pid")
        delay(1.seconds) // Wait for reload
        
        return if (isServiceRunning(serviceName)) {
            ReloadResult.Success(serviceName, ReloadMethod.SIGNAL)
        } else {
            ReloadResult.Failed(serviceName, Exception("Service stopped after signal"))
        }
    }
    
    private suspend fun reloadWithCommand(serviceName: String, command: String): ReloadResult {
        return try {
            runCommand(command)
            ReloadResult.Success(serviceName, ReloadMethod.COMMAND)
        } catch (e: Exception) {
            ReloadResult.Failed(serviceName, e)
        }
    }
    
    private suspend fun reloadWithSystemd(serviceName: String): ReloadResult {
        return try {
            runCommand("systemctl reload $serviceName")
            ReloadResult.Success(serviceName, ReloadMethod.SYSTEMD)
        } catch (e: Exception) {
            ReloadResult.Failed(serviceName, e)
        }
    }
    
    private suspend fun gracefulRestart(serviceName: String, gracePeriod: Duration): ReloadResult {
        // For services that need restart, try to do it gracefully
        try {
            // Send SIGTERM for graceful shutdown
            runCommand("systemctl stop $serviceName")
            delay(gracePeriod)
            runCommand("systemctl start $serviceName")
            
            return if (isServiceRunning(serviceName)) {
                ReloadResult.Success(serviceName, ReloadMethod.RESTART)
            } else {
                ReloadResult.Failed(serviceName, Exception("Service failed to start after restart"))
            }
        } catch (e: Exception) {
            return ReloadResult.Failed(serviceName, e)
        }
    }
    
    private suspend fun hardRestart(serviceName: String): ReloadResult {
        return try {
            runCommand("systemctl restart $serviceName")
            ReloadResult.Success(serviceName, ReloadMethod.RESTART)
        } catch (e: Exception) {
            ReloadResult.Failed(serviceName, e)
        }
    }
    
    private suspend fun getServicePid(serviceName: String): Int? {
        return try {
            val result = runCommand("systemctl show -p MainPID $serviceName")
            val pidStr = result.substringAfter("MainPID=").trim()
            pidStr.toIntOrNull()
        } catch (e: Exception) {
            null
        }
    }
    
    private suspend fun commandExists(command: String): Boolean {
        return try {
            runCommand("which $command")
            true
        } catch (e: Exception) {
            false
        }
    }
    
    private fun orderByDependencies(services: List<String>): List<String> {
        // Simple ordering - in production this would analyze actual dependencies
        val priorityOrder = listOf(
            "NetworkManager",
            "systemd-resolved", 
            "sshd",
            "nginx",
            "apache2",
            "httpd"
        )
        
        return services.sortedBy { service ->
            val index = priorityOrder.indexOf(service)
            if (index >= 0) index else priorityOrder.size
        }
    }
    
    private suspend fun runCommand(command: String): String = withContext(Dispatchers.IO) {
        val process = ProcessBuilder("bash", "-c", command)
            .redirectOutput(ProcessBuilder.Redirect.PIPE)
            .redirectError(ProcessBuilder.Redirect.PIPE)
            .start()
        
        val exitCode = process.waitFor()
        if (exitCode != 0) {
            val error = process.errorStream.bufferedReader().readText()
            throw Exception("Command failed with exit code $exitCode: $error")
        }
        
        process.inputStream.bufferedReader().readText()
    }
}

// ===== Data Classes =====

sealed class ReloadStrategy {
    data class Signal(val signal: String) : ReloadStrategy()
    data class Command(val command: String) : ReloadStrategy()
    object SystemdReload : ReloadStrategy()
    data class RestartRequired(val gracePeriod: Duration) : ReloadStrategy()
    data class Custom(val reloadFunction: suspend (String) -> ReloadResult) : ReloadStrategy()
    object NotSupported : ReloadStrategy()
}

sealed class ReloadResult {
    data class Success(
        val serviceName: String,
        val method: ReloadMethod
    ) : ReloadResult()
    
    data class Failed(
        val serviceName: String,
        val error: Throwable
    ) : ReloadResult()
    
    data class ServiceNotRunning(
        val serviceName: String
    ) : ReloadResult()
    
    data class ReloadNotSupported(
        val serviceName: String
    ) : ReloadResult()
}

enum class ReloadMethod {
    SIGNAL,
    COMMAND,
    SYSTEMD,
    RESTART
}

data class ServiceReloadState(
    val serviceName: String,
    var lastReloadAttempt: Long = 0,
    var lastSuccessfulReload: Long = 0,
    var reloadCount: Int = 0,
    var lastError: Throwable? = null
)