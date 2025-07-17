package org.horizonos.config.runtime

import org.horizonos.config.dsl.*
import kotlinx.serialization.Serializable
import java.io.File
import java.util.concurrent.TimeUnit

/**
 * Container Runtime Manager for HorizonOS
 * 
 * Manages container lifecycle and integration with the host system:
 * - Container creation and destruction
 * - Binary export to host PATH
 * - Volume mounting and persistence
 * - Health checks and monitoring
 * - Runtime switching (Podman/Docker/Distrobox/Toolbox)
 */

// ===== Runtime Status =====

@Serializable
enum class ContainerStatus {
    CREATED,
    RUNNING,
    STOPPED,
    PAUSED,
    EXITED,
    ERROR,
    UNKNOWN
}

@Serializable
data class ContainerInfo(
    val id: String,
    val name: String,
    val image: String,
    val status: ContainerStatus,
    val created: String,
    val runtime: ContainerRuntime,
    val ports: List<String> = emptyList(),
    val mounts: List<String> = emptyList(),
    val health: HealthStatus = HealthStatus.UNKNOWN
)

@Serializable
enum class HealthStatus {
    HEALTHY,
    UNHEALTHY,
    STARTING,
    UNKNOWN
}

@Serializable
data class RuntimeStats(
    val cpuUsage: Double,
    val memoryUsage: Long,
    val memoryLimit: Long,
    val networkIn: Long,
    val networkOut: Long,
    val diskUsage: Long
)

// ===== Container Manager =====

class ContainerManager {
    private val activeContainers = mutableMapOf<String, ContainerInfo>()
    private val exportedBinaries = mutableMapOf<String, String>() // binary -> container
    
    /**
     * Deploy all containers from configuration
     */
    fun deployContainers(config: ContainersConfig): List<ContainerInfo> {
        val deployed = mutableListOf<ContainerInfo>()
        
        for (container in config.containers) {
            try {
                val info = createContainer(container, config.globalMounts)
                deployed.add(info)
                
                if (container.autoStart) {
                    startContainer(container.name)
                }
                
                // Export binaries to host
                exportBinaries(container)
                
            } catch (e: Exception) {
                println("Failed to deploy container ${container.name}: ${e.message}")
            }
        }
        
        return deployed
    }
    
    /**
     * Create a container from configuration
     */
    private fun createContainer(
        container: SystemContainer,
        globalMounts: List<String>
    ): ContainerInfo {
        val runtime = getRuntimeCommand(container.runtime)
        val imageRef = buildImageReference(container.image, container.tag, container.digest)
        
        val createCmd = mutableListOf<String>()
        createCmd.addAll(listOf(runtime, "create"))
        createCmd.add("--name")
        createCmd.add(container.name)
        
        // Add hostname if specified
        container.hostname?.let {
            createCmd.addAll(listOf("--hostname", it))
        }
        
        // Add user if specified
        container.user?.let {
            createCmd.addAll(listOf("--user", it))
        }
        
        // Add working directory if specified
        container.workingDir?.let {
            createCmd.addAll(listOf("--workdir", it))
        }
        
        // Add environment variables
        for ((key, value) in container.environment) {
            createCmd.addAll(listOf("--env", "$key=$value"))
        }
        
        // Add ports
        for (port in container.ports) {
            createCmd.addAll(listOf("--publish", port))
        }
        
        // Add mounts
        val allMounts = globalMounts + container.persistent
        for (mount in allMounts) {
            createCmd.addAll(listOf("--volume", mount))
        }
        
        // Add labels
        for ((key, value) in container.labels) {
            createCmd.addAll(listOf("--label", "$key=$value"))
        }
        
        // Add network mode
        if (container.networkMode != "bridge") {
            createCmd.addAll(listOf("--network", container.networkMode))
        }
        
        // Add privileged flag if needed
        if (container.privileged) {
            createCmd.add("--privileged")
        }
        
        // Add image reference
        createCmd.add(imageRef)
        
        // Execute create command
        val result = executeCommand(createCmd)
        if (result.exitCode != 0) {
            throw RuntimeException("Failed to create container: ${result.stderr}")
        }
        
        val containerId = result.stdout.trim()
        
        // Install packages if specified
        if (container.packages.isNotEmpty()) {
            installPackages(container.name, container.packages, container.runtime)
        }
        
        // Run post commands
        for (command in container.postCommands) {
            executeInContainer(container.name, command, container.runtime)
        }
        
        val info = ContainerInfo(
            id = containerId,
            name = container.name,
            image = imageRef,
            status = ContainerStatus.CREATED,
            created = System.currentTimeMillis().toString(),
            runtime = container.runtime,
            ports = container.ports,
            mounts = allMounts
        )
        
        activeContainers[container.name] = info
        return info
    }
    
    /**
     * Start a container
     */
    fun startContainer(name: String): Boolean {
        val container = activeContainers[name] ?: return false
        val runtime = getRuntimeCommand(container.runtime)
        
        val result = executeCommand(listOf(runtime, "start", name))
        if (result.exitCode == 0) {
            activeContainers[name] = container.copy(status = ContainerStatus.RUNNING)
            return true
        }
        return false
    }
    
    /**
     * Stop a container
     */
    fun stopContainer(name: String): Boolean {
        val container = activeContainers[name] ?: return false
        val runtime = getRuntimeCommand(container.runtime)
        
        val result = executeCommand(listOf(runtime, "stop", name))
        if (result.exitCode == 0) {
            activeContainers[name] = container.copy(status = ContainerStatus.STOPPED)
            return true
        }
        return false
    }
    
    /**
     * Remove a container
     */
    fun removeContainer(name: String, force: Boolean = false): Boolean {
        val container = activeContainers[name] ?: return false
        val runtime = getRuntimeCommand(container.runtime)
        
        val cmd = mutableListOf(runtime, "rm")
        if (force) cmd.add("--force")
        cmd.add(name)
        
        val result = executeCommand(cmd)
        if (result.exitCode == 0) {
            activeContainers.remove(name)
            // Remove exported binaries
            exportedBinaries.entries.removeAll { it.value == name }
            return true
        }
        return false
    }
    
    /**
     * Get container status
     */
    fun getContainerStatus(name: String): ContainerStatus {
        val container = activeContainers[name] ?: return ContainerStatus.UNKNOWN
        val runtime = getRuntimeCommand(container.runtime)
        
        val result = executeCommand(listOf(
            runtime, "inspect", "--format", "{{.State.Status}}", name
        ))
        
        return when (result.stdout.trim().lowercase()) {
            "created" -> ContainerStatus.CREATED
            "running" -> ContainerStatus.RUNNING
            "stopped", "exited" -> ContainerStatus.STOPPED
            "paused" -> ContainerStatus.PAUSED
            else -> ContainerStatus.UNKNOWN
        }
    }
    
    /**
     * Get container statistics
     */
    fun getContainerStats(name: String): RuntimeStats? {
        val container = activeContainers[name] ?: return null
        val runtime = getRuntimeCommand(container.runtime)
        
        val result = executeCommand(listOf(
            runtime, "stats", "--format", "json", "--no-stream", name
        ))
        
        if (result.exitCode != 0) return null
        
        // Parse JSON stats (simplified)
        return RuntimeStats(
            cpuUsage = 0.0,
            memoryUsage = 0,
            memoryLimit = 0,
            networkIn = 0,
            networkOut = 0,
            diskUsage = 0
        )
    }
    
    /**
     * List all containers
     */
    fun listContainers(): List<ContainerInfo> {
        return activeContainers.values.map { container ->
            container.copy(status = getContainerStatus(container.name))
        }
    }
    
    /**
     * Execute command in container
     */
    fun executeInContainer(
        containerName: String,
        command: String,
        runtime: ContainerRuntime = ContainerRuntime.DISTROBOX
    ): CommandResult {
        val runtimeCmd = getRuntimeCommand(runtime)
        return executeCommand(listOf(runtimeCmd, "exec", containerName, "sh", "-c", command))
    }
    
    /**
     * Install packages in container
     */
    private fun installPackages(
        containerName: String,
        packages: List<String>,
        runtime: ContainerRuntime
    ) {
        val packageManager = getPackageManager(runtime)
        val installCmd = "$packageManager install -y ${packages.joinToString(" ")}"
        
        val result = executeInContainer(containerName, installCmd, runtime)
        if (result.exitCode != 0) {
            println("Warning: Failed to install packages in $containerName: ${result.stderr}")
        }
    }
    
    /**
     * Export binaries to host system
     */
    private fun exportBinaries(container: SystemContainer) {
        val binDir = File("/usr/local/bin")
        if (!binDir.exists()) binDir.mkdirs()
        
        for (binary in container.binaries) {
            val wrapperScript = """#!/bin/bash
# HorizonOS Container Binary Wrapper
# Generated for ${container.name}
exec ${getRuntimeCommand(container.runtime)} exec ${container.name} $binary "$@"
"""
            
            val wrapperFile = File(binDir, binary)
            wrapperFile.writeText(wrapperScript)
            wrapperFile.setExecutable(true)
            
            exportedBinaries[binary] = container.name
        }
    }
    
    /**
     * Health check for container
     */
    fun checkContainerHealth(name: String): HealthStatus {
        val container = activeContainers[name] ?: return HealthStatus.UNKNOWN
        
        // For now, just check if container is running
        return when (getContainerStatus(name)) {
            ContainerStatus.RUNNING -> HealthStatus.HEALTHY
            ContainerStatus.CREATED -> HealthStatus.STARTING
            ContainerStatus.STOPPED, ContainerStatus.EXITED -> HealthStatus.UNHEALTHY
            else -> HealthStatus.UNKNOWN
        }
    }
    
    /**
     * Cleanup all containers
     */
    fun cleanup() {
        for (containerName in activeContainers.keys.toList()) {
            stopContainer(containerName)
            removeContainer(containerName, force = true)
        }
        
        // Remove exported binaries
        for (binary in exportedBinaries.keys) {
            val wrapperFile = File("/usr/local/bin", binary)
            if (wrapperFile.exists()) {
                wrapperFile.delete()
            }
        }
        
        activeContainers.clear()
        exportedBinaries.clear()
    }
    
    // ===== Runtime Helpers =====
    
    private fun getRuntimeCommand(runtime: ContainerRuntime): String {
        return when (runtime) {
            ContainerRuntime.PODMAN -> "podman"
            ContainerRuntime.DOCKER -> "docker"
            ContainerRuntime.TOOLBOX -> "toolbox"
            ContainerRuntime.DISTROBOX -> "distrobox"
        }
    }
    
    private fun getPackageManager(runtime: ContainerRuntime): String {
        return when (runtime) {
            ContainerRuntime.PODMAN, ContainerRuntime.DOCKER -> "pacman -S"
            ContainerRuntime.TOOLBOX -> "dnf install"
            ContainerRuntime.DISTROBOX -> "pacman -S"
        }
    }
    
    private fun buildImageReference(image: String, tag: String, digest: String?): String {
        return if (digest != null) {
            "$image@$digest"
        } else {
            "$image:$tag"
        }
    }
    
    private fun executeCommand(command: List<String>): CommandResult {
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

// ===== Command Result =====

@Serializable
data class CommandResult(
    val exitCode: Int,
    val stdout: String,
    val stderr: String
) {
    val isSuccess: Boolean get() = exitCode == 0
}

// ===== Container Runtime Factory =====

object ContainerRuntimeFactory {
    fun create(runtime: ContainerRuntime): ContainerManager {
        return ContainerManager()
    }
    
    fun isRuntimeAvailable(runtime: ContainerRuntime): Boolean {
        val command = when (runtime) {
            ContainerRuntime.PODMAN -> "podman"
            ContainerRuntime.DOCKER -> "docker"
            ContainerRuntime.TOOLBOX -> "toolbox"
            ContainerRuntime.DISTROBOX -> "distrobox"
        }
        
        return try {
            val process = ProcessBuilder(command, "--version")
                .redirectErrorStream(true)
                .start()
            
            process.waitFor(5, TimeUnit.SECONDS) && process.exitValue() == 0
        } catch (e: Exception) {
            false
        }
    }
    
    fun getAvailableRuntimes(): List<ContainerRuntime> {
        return ContainerRuntime.values().filter { isRuntimeAvailable(it) }
    }
}

// ===== Container Service =====

class ContainerService(private val manager: ContainerManager) {
    
    /**
     * Deploy containers from system configuration
     */
    fun deployFromConfig(config: CompiledConfig) {
        config.containers?.let { containersConfig ->
            manager.deployContainers(containersConfig)
        }
    }
    
    /**
     * Health check all containers
     */
    fun healthCheck(): Map<String, HealthStatus> {
        return manager.listContainers().associate { 
            it.name to manager.checkContainerHealth(it.name) 
        }
    }
    
    /**
     * Get system overview
     */
    fun getSystemOverview(): SystemOverview {
        val containers = manager.listContainers()
        val healthy = containers.count { manager.checkContainerHealth(it.name) == HealthStatus.HEALTHY }
        
        return SystemOverview(
            totalContainers = containers.size,
            runningContainers = containers.count { it.status == ContainerStatus.RUNNING },
            healthyContainers = healthy,
            availableRuntimes = ContainerRuntimeFactory.getAvailableRuntimes()
        )
    }
}

@Serializable
data class SystemOverview(
    val totalContainers: Int,
    val runningContainers: Int,
    val healthyContainers: Int,
    val availableRuntimes: List<ContainerRuntime>
)