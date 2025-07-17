package org.horizonos.config.runtime

import org.horizonos.config.dsl.*
import kotlinx.serialization.Serializable
import java.io.File
import java.time.Instant

/**
 * Layer Manager for HorizonOS
 * 
 * Manages the layered system architecture:
 * - Base Layer: OSTree immutable base system
 * - System Layers: Container-based system tools
 * - User Layer: Flatpak user applications
 * 
 * Handles:
 * - Layer deployment and ordering
 * - Dependency resolution
 * - Layer health monitoring
 * - Layer updates and rollbacks
 */

// ===== Layer Status =====

@Serializable
enum class LayerStatus {
    DEPLOYED,
    STARTING,
    RUNNING,
    STOPPED,
    FAILED,
    UPDATING,
    UNKNOWN
}

@Serializable
data class LayerInfo(
    val name: String,
    val type: LayerType,
    val purpose: LayerPurpose,
    val status: LayerStatus,
    val deployTime: String,
    val dependencies: List<String>,
    val containerId: String? = null,
    val health: HealthStatus = HealthStatus.UNKNOWN
)

@Serializable
data class LayerDeploymentResult(
    val success: Boolean,
    val message: String,
    val layerInfo: LayerInfo? = null
)

// ===== Layer Manager =====

class LayerManager(private val containerManager: ContainerManager) {
    private val deployedLayers = mutableMapOf<String, LayerInfo>()
    private val layerDependencies = mutableMapOf<String, List<String>>()
    
    /**
     * Deploy layers from configuration
     */
    fun deployLayers(config: LayersConfig): List<LayerDeploymentResult> {
        val results = mutableListOf<LayerDeploymentResult>()
        
        try {
            // Deploy base layer first
            val baseResult = deployBaseLayer(config.base)
            results.add(baseResult)
            
            if (!baseResult.success) {
                return results
            }
            
            // Sort system layers by dependencies and priority
            val sortedLayers = sortLayers(config.system)
            
            // Deploy system layers
            for (layer in sortedLayers) {
                val result = deploySystemLayer(layer)
                results.add(result)
                
                if (!result.success) {
                    println("Failed to deploy layer ${layer.name}: ${result.message}")
                }
            }
            
            // Deploy user layer
            val userResult = deployUserLayer(config.user)
            results.add(userResult)
            
        } catch (e: Exception) {
            results.add(LayerDeploymentResult(
                success = false,
                message = "Layer deployment failed: ${e.message}"
            ))
        }
        
        return results
    }
    
    /**
     * Deploy base OSTree layer
     */
    private fun deployBaseLayer(base: BaseLayer): LayerDeploymentResult {
        try {
            // Verify OSTree commit if specified
            base.ostreeCommit?.let { commit ->
                if (!verifyOstreeCommit(base.ostreeRef, commit)) {
                    return LayerDeploymentResult(
                        success = false,
                        message = "OSTree commit verification failed for ${base.ostreeRef}@$commit"
                    )
                }
            }
            
            // Check if base packages are available
            for (pkg in base.packages) {
                if (!isPackageAvailable(pkg)) {
                    println("Warning: Base package $pkg not found")
                }
            }
            
            // Enable base services
            for (service in base.services) {
                enableSystemService(service)
            }
            
            val layerInfo = LayerInfo(
                name = "base",
                type = LayerType.BASE,
                purpose = LayerPurpose.CUSTOM,
                status = LayerStatus.DEPLOYED,
                deployTime = Instant.now().toString(),
                dependencies = emptyList()
            )
            
            deployedLayers["base"] = layerInfo
            
            return LayerDeploymentResult(
                success = true,
                message = "Base layer deployed successfully",
                layerInfo = layerInfo
            )
            
        } catch (e: Exception) {
            return LayerDeploymentResult(
                success = false,
                message = "Base layer deployment failed: ${e.message}"
            )
        }
    }
    
    /**
     * Deploy system layer
     */
    private fun deploySystemLayer(layer: SystemLayer): LayerDeploymentResult {
        try {
            // Check dependencies
            for (dep in layer.dependencies) {
                if (!deployedLayers.containsKey(dep) || 
                    deployedLayers[dep]?.status != LayerStatus.DEPLOYED) {
                    return LayerDeploymentResult(
                        success = false,
                        message = "Dependency '$dep' not available for layer ${layer.name}"
                    )
                }
            }
            
            // Create container configuration
            val containerConfig = ContainersConfig(
                containers = listOf(layer.container)
            )
            
            // Deploy container
            val containerInfo = containerManager.deployContainers(containerConfig)
            if (containerInfo.isEmpty()) {
                return LayerDeploymentResult(
                    success = false,
                    message = "Failed to deploy container for layer ${layer.name}"
                )
            }
            
            val container = containerInfo.first()
            
            // Start container if auto-start is enabled
            if (layer.autoStart) {
                val started = containerManager.startContainer(layer.name)
                if (!started) {
                    return LayerDeploymentResult(
                        success = false,
                        message = "Failed to start container for layer ${layer.name}"
                    )
                }
            }
            
            val layerInfo = LayerInfo(
                name = layer.name,
                type = LayerType.SYSTEM,
                purpose = layer.purpose,
                status = if (layer.autoStart) LayerStatus.RUNNING else LayerStatus.DEPLOYED,
                deployTime = Instant.now().toString(),
                dependencies = layer.dependencies,
                containerId = container.id
            )
            
            deployedLayers[layer.name] = layerInfo
            layerDependencies[layer.name] = layer.dependencies
            
            return LayerDeploymentResult(
                success = true,
                message = "System layer ${layer.name} deployed successfully",
                layerInfo = layerInfo
            )
            
        } catch (e: Exception) {
            return LayerDeploymentResult(
                success = false,
                message = "System layer deployment failed: ${e.message}"
            )
        }
    }
    
    /**
     * Deploy user layer
     */
    private fun deployUserLayer(userLayer: UserLayer): LayerDeploymentResult {
        try {
            // Install Flatpak applications
            for (flatpak in userLayer.flatpaks) {
                installFlatpak(flatpak, userLayer.userScope)
            }
            
            // Install AppImages
            for (appImage in userLayer.appImages) {
                installAppImage(appImage)
            }
            
            // Install Snap packages
            for (snap in userLayer.snaps) {
                installSnap(snap)
            }
            
            val layerInfo = LayerInfo(
                name = "user",
                type = LayerType.USER,
                purpose = LayerPurpose.OFFICE,
                status = LayerStatus.DEPLOYED,
                deployTime = Instant.now().toString(),
                dependencies = emptyList()
            )
            
            deployedLayers["user"] = layerInfo
            
            return LayerDeploymentResult(
                success = true,
                message = "User layer deployed successfully",
                layerInfo = layerInfo
            )
            
        } catch (e: Exception) {
            return LayerDeploymentResult(
                success = false,
                message = "User layer deployment failed: ${e.message}"
            )
        }
    }
    
    /**
     * Start a system layer
     */
    fun startLayer(layerName: String): Boolean {
        val layer = deployedLayers[layerName] ?: return false
        
        if (layer.type == LayerType.SYSTEM && layer.containerId != null) {
            val started = containerManager.startContainer(layerName)
            if (started) {
                deployedLayers[layerName] = layer.copy(status = LayerStatus.RUNNING)
                return true
            }
        }
        
        return false
    }
    
    /**
     * Stop a system layer
     */
    fun stopLayer(layerName: String): Boolean {
        val layer = deployedLayers[layerName] ?: return false
        
        if (layer.type == LayerType.SYSTEM && layer.containerId != null) {
            val stopped = containerManager.stopContainer(layerName)
            if (stopped) {
                deployedLayers[layerName] = layer.copy(status = LayerStatus.STOPPED)
                return true
            }
        }
        
        return false
    }
    
    /**
     * Get layer status
     */
    fun getLayerStatus(layerName: String): LayerStatus {
        val layer = deployedLayers[layerName] ?: return LayerStatus.UNKNOWN
        
        if (layer.type == LayerType.SYSTEM && layer.containerId != null) {
            val containerStatus = containerManager.getContainerStatus(layerName)
            return when (containerStatus) {
                ContainerStatus.RUNNING -> LayerStatus.RUNNING
                ContainerStatus.STOPPED -> LayerStatus.STOPPED
                ContainerStatus.CREATED -> LayerStatus.DEPLOYED
                ContainerStatus.ERROR -> LayerStatus.FAILED
                else -> LayerStatus.UNKNOWN
            }
        }
        
        return layer.status
    }
    
    /**
     * List all deployed layers
     */
    fun listLayers(): List<LayerInfo> {
        return deployedLayers.values.map { layer ->
            layer.copy(
                status = getLayerStatus(layer.name),
                health = getLayerHealth(layer.name)
            )
        }
    }
    
    /**
     * Get layer health
     */
    private fun getLayerHealth(layerName: String): HealthStatus {
        val layer = deployedLayers[layerName] ?: return HealthStatus.UNKNOWN
        
        if (layer.type == LayerType.SYSTEM && layer.containerId != null) {
            return containerManager.checkContainerHealth(layerName)
        }
        
        return HealthStatus.HEALTHY
    }
    
    /**
     * Remove a layer
     */
    fun removeLayer(layerName: String): Boolean {
        val layer = deployedLayers[layerName] ?: return false
        
        // Check if other layers depend on this one
        val dependentLayers = layerDependencies.entries.filter { 
            it.value.contains(layerName) 
        }.map { it.key }
        
        if (dependentLayers.isNotEmpty()) {
            println("Cannot remove layer $layerName: required by ${dependentLayers.joinToString(", ")}")
            return false
        }
        
        // Remove container if it's a system layer
        if (layer.type == LayerType.SYSTEM && layer.containerId != null) {
            containerManager.removeContainer(layerName, force = true)
        }
        
        deployedLayers.remove(layerName)
        layerDependencies.remove(layerName)
        
        return true
    }
    
    /**
     * Update a layer
     */
    fun updateLayer(layerName: String): Boolean {
        val layer = deployedLayers[layerName] ?: return false
        
        deployedLayers[layerName] = layer.copy(status = LayerStatus.UPDATING)
        
        // For system layers, update the container
        if (layer.type == LayerType.SYSTEM && layer.containerId != null) {
            // Stop container
            containerManager.stopContainer(layerName)
            
            // Remove old container
            containerManager.removeContainer(layerName, force = true)
            
            // This would need the original layer configuration to redeploy
            // For now, just mark as failed
            deployedLayers[layerName] = layer.copy(status = LayerStatus.FAILED)
            return false
        }
        
        return true
    }
    
    /**
     * Get system overview
     */
    fun getSystemOverview(): LayerSystemOverview {
        val layers = listLayers()
        val healthy = layers.count { it.health == HealthStatus.HEALTHY }
        val running = layers.count { it.status == LayerStatus.RUNNING }
        
        return LayerSystemOverview(
            totalLayers = layers.size,
            runningLayers = running,
            healthyLayers = healthy,
            baseLayerStatus = layers.find { it.type == LayerType.BASE }?.status ?: LayerStatus.UNKNOWN,
            systemLayers = layers.filter { it.type == LayerType.SYSTEM }.size,
            userLayerStatus = layers.find { it.type == LayerType.USER }?.status ?: LayerStatus.UNKNOWN
        )
    }
    
    // ===== Helper Methods =====
    
    private fun verifyOstreeCommit(ref: String, commit: String): Boolean {
        // TODO: Implement OSTree commit verification
        return true
    }
    
    private fun isPackageAvailable(packageName: String): Boolean {
        // TODO: Check if package is available in repositories
        return true
    }
    
    private fun enableSystemService(serviceName: String) {
        // TODO: Enable systemd service
        println("Enabling service: $serviceName")
    }
    
    private fun installFlatpak(flatpak: FlatpakApplication, userScope: Boolean) {
        val scope = if (userScope) "--user" else "--system"
        val cmd = "flatpak install $scope -y ${flatpak.id}"
        
        // TODO: Execute flatpak install command
        println("Installing Flatpak: $cmd")
    }
    
    private fun installAppImage(appImage: AppImage) {
        // TODO: Download and install AppImage
        println("Installing AppImage: ${appImage.name} from ${appImage.url}")
    }
    
    private fun installSnap(snap: Snap) {
        val cmd = buildString {
            append("snap install ${snap.name}")
            if (snap.channel != "stable") append(" --channel=${snap.channel}")
            if (snap.classic) append(" --classic")
            if (snap.devmode) append(" --devmode")
        }
        
        // TODO: Execute snap install command
        println("Installing Snap: $cmd")
    }
}

// ===== Layer System Overview =====

@Serializable
data class LayerSystemOverview(
    val totalLayers: Int,
    val runningLayers: Int,
    val healthyLayers: Int,
    val baseLayerStatus: LayerStatus,
    val systemLayers: Int,
    val userLayerStatus: LayerStatus
)

// ===== Layer Service =====

class LayerService(private val layerManager: LayerManager) {
    
    /**
     * Deploy layers from system configuration
     */
    fun deployFromConfig(config: CompiledConfig) {
        config.layers?.let { layersConfig ->
            val results = layerManager.deployLayers(layersConfig)
            
            for (result in results) {
                if (!result.success) {
                    println("Layer deployment failed: ${result.message}")
                } else {
                    println("Layer deployed: ${result.layerInfo?.name}")
                }
            }
        }
    }
    
    /**
     * Health check all layers
     */
    fun healthCheck(): Map<String, HealthStatus> {
        return layerManager.listLayers().associate { 
            it.name to it.health
        }
    }
    
    /**
     * Get system overview
     */
    fun getSystemOverview(): LayerSystemOverview {
        return layerManager.getSystemOverview()
    }
}