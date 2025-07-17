package org.horizonos.config.runtime

import org.horizonos.config.dsl.*
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import java.io.File
import java.time.Instant

/**
 * System Manager for HorizonOS
 * 
 * Coordinates all runtime components:
 * - Container management
 * - Layer deployment
 * - Reproducible system states
 * - System monitoring and health
 * - Configuration updates
 */

@Serializable
data class HorizonSystemState(
    val version: String,
    val timestamp: String,
    val containers: List<ContainerInfo>,
    val layers: List<LayerInfo>,
    val reproducibleConfig: ReproducibleConfig?,
    val systemHealth: SystemHealth
)

@Serializable
data class SystemHealth(
    val overall: HealthStatus,
    val containers: Map<String, HealthStatus>,
    val layers: Map<String, HealthStatus>,
    val services: Map<String, Boolean>,
    val lastCheck: String
)

@Serializable
data class DeploymentResult(
    val success: Boolean,
    val message: String,
    val timestamp: String,
    val containersDeployed: Int,
    val layersDeployed: Int,
    val errors: List<String>
)

class HorizonSystemManager {
    private val containerManager = ContainerManager()
    private val layerManager = LayerManager(containerManager)
    private val containerService = ContainerService(containerManager)
    private val layerService = LayerService(layerManager)
    
    private val json = Json { 
        prettyPrint = true 
        ignoreUnknownKeys = true
    }
    
    private val stateFile = File("/var/lib/horizonos/system-state.json")
    
    /**
     * Deploy complete system from configuration
     */
    fun deploySystem(config: CompiledConfig): DeploymentResult {
        val errors = mutableListOf<String>()
        val timestamp = Instant.now().toString()
        
        try {
            println("Starting HorizonOS system deployment...")
            
            // Validate configuration
            validateConfiguration(config)
            
            // Deploy containers first
            var containersDeployed = 0
            config.containers?.let { containersConfig ->
                val containers = containerManager.deployContainers(containersConfig)
                containersDeployed = containers.size
                
                val failed = containers.filter { it.status == ContainerStatus.ERROR }
                if (failed.isNotEmpty()) {
                    errors.addAll(failed.map { "Container ${it.name} failed to deploy" })
                }
            }
            
            // Deploy layers
            var layersDeployed = 0
            config.layers?.let { layersConfig ->
                val results = layerManager.deployLayers(layersConfig)
                layersDeployed = results.count { it.success }
                
                val failed = results.filter { !it.success }
                errors.addAll(failed.map { it.message })
            }
            
            // Apply reproducible configuration if present
            config.reproducible?.let { reproducibleConfig ->
                applyReproducibleConfig(reproducibleConfig)
            }
            
            // Save system state
            saveSystemState(config)
            
            val success = errors.isEmpty()
            val message = if (success) {
                "System deployed successfully"
            } else {
                "System deployed with ${errors.size} errors"
            }
            
            return DeploymentResult(
                success = success,
                message = message,
                timestamp = timestamp,
                containersDeployed = containersDeployed,
                layersDeployed = layersDeployed,
                errors = errors
            )
            
        } catch (e: Exception) {
            return DeploymentResult(
                success = false,
                message = "System deployment failed: ${e.message}",
                timestamp = timestamp,
                containersDeployed = 0,
                layersDeployed = 0,
                errors = listOf(e.message ?: "Unknown error")
            )
        }
    }
    
    /**
     * Get current system state
     */
    fun getSystemState(): HorizonSystemState {
        val containers = containerManager.listContainers()
        val layers = layerManager.listLayers()
        val health = checkSystemHealth()
        
        return HorizonSystemState(
            version = "1.0.0",
            timestamp = Instant.now().toString(),
            containers = containers,
            layers = layers,
            reproducibleConfig = loadReproducibleConfig(),
            systemHealth = health
        )
    }
    
    /**
     * Check system health
     */
    fun checkSystemHealth(): SystemHealth {
        val containerHealth = containerService.healthCheck()
        val layerHealth = layerService.healthCheck()
        val serviceHealth = checkSystemServices()
        
        val overallHealth = when {
            containerHealth.values.all { it == HealthStatus.HEALTHY } &&
            layerHealth.values.all { it == HealthStatus.HEALTHY } &&
            serviceHealth.values.all { it } -> HealthStatus.HEALTHY
            
            containerHealth.values.any { it == HealthStatus.UNHEALTHY } ||
            layerHealth.values.any { it == HealthStatus.UNHEALTHY } ||
            serviceHealth.values.any { !it } -> HealthStatus.UNHEALTHY
            
            else -> HealthStatus.STARTING
        }
        
        return SystemHealth(
            overall = overallHealth,
            containers = containerHealth,
            layers = layerHealth,
            services = serviceHealth,
            lastCheck = Instant.now().toString()
        )
    }
    
    /**
     * Update system configuration
     */
    fun updateSystem(config: CompiledConfig): DeploymentResult {
        val currentState = getSystemState()
        
        // For now, just redeploy everything
        // In a real implementation, this would do incremental updates
        return deploySystem(config)
    }
    
    /**
     * Rollback system to previous state
     */
    fun rollbackSystem(): Boolean {
        // TODO: Implement rollback using OSTree
        return false
    }
    
    /**
     * Start system services
     */
    fun startSystem(): Boolean {
        try {
            // Start all layers that should be auto-started
            val layers = layerManager.listLayers()
            for (layer in layers) {
                if (layer.type == LayerType.SYSTEM && layer.status == LayerStatus.DEPLOYED) {
                    layerManager.startLayer(layer.name)
                }
            }
            
            return true
        } catch (e: Exception) {
            println("Failed to start system: ${e.message}")
            return false
        }
    }
    
    /**
     * Stop system services
     */
    fun stopSystem(): Boolean {
        try {
            // Stop all running layers
            val layers = layerManager.listLayers()
            for (layer in layers) {
                if (layer.status == LayerStatus.RUNNING) {
                    layerManager.stopLayer(layer.name)
                }
            }
            
            return true
        } catch (e: Exception) {
            println("Failed to stop system: ${e.message}")
            return false
        }
    }
    
    /**
     * Cleanup system
     */
    fun cleanupSystem() {
        containerManager.cleanup()
        
        // Clean up state file
        if (stateFile.exists()) {
            stateFile.delete()
        }
    }
    
    /**
     * Get system overview
     */
    fun getSystemOverview(): SystemOverview {
        val containerOverview = containerService.getSystemOverview()
        val layerOverview = layerService.getSystemOverview()
        
        return SystemOverview(
            totalContainers = containerOverview.totalContainers,
            runningContainers = containerOverview.runningContainers,
            healthyContainers = containerOverview.healthyContainers,
            availableRuntimes = containerOverview.availableRuntimes
        )
    }
    
    /**
     * Execute command in container
     */
    fun executeCommand(containerName: String, command: String): CommandResult {
        return containerManager.executeInContainer(containerName, command)
    }
    
    /**
     * Get container logs
     */
    fun getContainerLogs(containerName: String, lines: Int = 100): String {
        val container = containerManager.listContainers().find { it.name == containerName }
            ?: return "Container not found"
        
        val runtime = when (container.runtime) {
            ContainerRuntime.PODMAN -> "podman"
            ContainerRuntime.DOCKER -> "docker"
            ContainerRuntime.TOOLBOX -> "toolbox"
            ContainerRuntime.DISTROBOX -> "distrobox"
        }
        
        val result = executeSystemCommand(listOf(runtime, "logs", "--tail", lines.toString(), containerName))
        return result.stdout
    }
    
    /**
     * Export system configuration
     */
    fun exportConfiguration(): String {
        val state = getSystemState()
        return json.encodeToString(HorizonSystemState.serializer(), state)
    }
    
    /**
     * Import system configuration
     */
    fun importConfiguration(configJson: String): Boolean {
        return try {
            val state = json.decodeFromString(HorizonSystemState.serializer(), configJson)
            // TODO: Apply imported configuration
            true
        } catch (e: Exception) {
            println("Failed to import configuration: ${e.message}")
            false
        }
    }
    
    // ===== Private Helper Methods =====
    
    private fun validateConfiguration(config: CompiledConfig) {
        // Basic validation
        if (config.system.hostname.isBlank()) {
            throw IllegalArgumentException("Hostname cannot be empty")
        }
        
        // Validate container configuration
        config.containers?.let { containers ->
            for (container in containers.containers) {
                if (container.name.isBlank()) {
                    throw IllegalArgumentException("Container name cannot be empty")
                }
                if (container.image.isBlank()) {
                    throw IllegalArgumentException("Container image cannot be empty")
                }
            }
        }
        
        // Validate layer configuration
        config.layers?.let { layers ->
            val layerNames = layers.system.map { it.name }
            for (layer in layers.system) {
                for (dep in layer.dependencies) {
                    if (dep !in layerNames) {
                        throw IllegalArgumentException("Layer ${layer.name} depends on non-existent layer $dep")
                    }
                }
            }
        }
    }
    
    private fun applyReproducibleConfig(config: ReproducibleConfig) {
        if (!config.enabled) return
        
        // TODO: Apply reproducible configuration
        // - Pin OSTree commits
        // - Verify container digests
        // - Apply system image state
        
        println("Applying reproducible configuration...")
    }
    
    private fun saveSystemState(config: CompiledConfig) {
        try {
            val state = getSystemState()
            stateFile.parentFile?.mkdirs()
            stateFile.writeText(json.encodeToString(HorizonSystemState.serializer(), state))
        } catch (e: Exception) {
            println("Failed to save system state: ${e.message}")
        }
    }
    
    private fun loadReproducibleConfig(): ReproducibleConfig? {
        return try {
            if (stateFile.exists()) {
                val state = json.decodeFromString(HorizonSystemState.serializer(), stateFile.readText())
                state.reproducibleConfig
            } else {
                null
            }
        } catch (e: Exception) {
            null
        }
    }
    
    private fun checkSystemServices(): Map<String, Boolean> {
        val services = listOf(
            "systemd-networkd",
            "systemd-resolved",
            "podman.socket",
            "flatpak-system-helper"
        )
        
        return services.associateWith { service ->
            val result = executeSystemCommand(listOf("systemctl", "is-active", service))
            result.stdout.trim() == "active"
        }
    }
    
    private fun executeSystemCommand(command: List<String>): CommandResult {
        return try {
            val process = ProcessBuilder(command)
                .redirectErrorStream(false)
                .start()
            
            val stdout = process.inputStream.bufferedReader().readText()
            val stderr = process.errorStream.bufferedReader().readText()
            
            val exitCode = process.waitFor()
            
            CommandResult(exitCode, stdout, stderr)
        } catch (e: Exception) {
            CommandResult(1, "", e.message ?: "Unknown error")
        }
    }
}

// ===== System Service =====

class HorizonSystemService {
    private val systemManager = HorizonSystemManager()
    
    /**
     * Initialize system
     */
    fun initialize() {
        println("Initializing HorizonOS system...")
        
        // Check available runtimes
        val runtimes = ContainerRuntimeFactory.getAvailableRuntimes()
        println("Available container runtimes: ${runtimes.joinToString(", ")}")
        
        if (runtimes.isEmpty()) {
            println("Warning: No container runtimes available")
        }
    }
    
    /**
     * Deploy system from configuration file
     */
    fun deployFromFile(configPath: String): DeploymentResult {
        // TODO: Load configuration from file
        // For now, return a placeholder
        return DeploymentResult(
            success = false,
            message = "Configuration loading not implemented",
            timestamp = Instant.now().toString(),
            containersDeployed = 0,
            layersDeployed = 0,
            errors = listOf("Configuration loading not implemented")
        )
    }
    
    /**
     * Start system daemon
     */
    fun startDaemon() {
        println("Starting HorizonOS system daemon...")
        
        // Start system
        systemManager.startSystem()
        
        // Monitor system health
        Thread {
            while (true) {
                val health = systemManager.checkSystemHealth()
                if (health.overall == HealthStatus.UNHEALTHY) {
                    println("System health warning: ${health.overall}")
                }
                
                Thread.sleep(30000) // Check every 30 seconds
            }
        }.start()
    }
    
    /**
     * Stop system daemon
     */
    fun stopDaemon() {
        println("Stopping HorizonOS system daemon...")
        systemManager.stopSystem()
    }
    
    /**
     * Get system status
     */
    fun getStatus(): HorizonSystemState {
        return systemManager.getSystemState()
    }
}

// ===== CLI Interface =====

object HorizonOSCLI {
    private val systemService = HorizonSystemService()
    
    fun main(args: Array<String>) {
        when (args.getOrNull(0)) {
            "deploy" -> {
                val configPath = args.getOrNull(1) ?: "config.horizonos.kts"
                val result = systemService.deployFromFile(configPath)
                println(result.message)
            }
            
            "start" -> {
                systemService.startDaemon()
            }
            
            "stop" -> {
                systemService.stopDaemon()
            }
            
            "status" -> {
                val state = systemService.getStatus()
                println("System Status: ${state.systemHealth.overall}")
                println("Containers: ${state.containers.size}")
                println("Layers: ${state.layers.size}")
            }
            
            "health" -> {
                val health = HorizonSystemManager().checkSystemHealth()
                println("Overall Health: ${health.overall}")
                println("Last Check: ${health.lastCheck}")
            }
            
            else -> {
                println("HorizonOS System Manager")
                println("Usage: horizonos [command]")
                println("Commands:")
                println("  deploy <config>  - Deploy system from configuration")
                println("  start           - Start system daemon")
                println("  stop            - Stop system daemon")
                println("  status          - Show system status")
                println("  health          - Check system health")
            }
        }
    }
}