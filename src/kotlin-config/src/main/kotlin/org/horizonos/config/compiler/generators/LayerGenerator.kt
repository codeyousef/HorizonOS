package org.horizonos.config.compiler.generators

import org.horizonos.config.dsl.*
import java.io.File

/**
 * Layer Generator for HorizonOS
 * 
 * Generates layer deployment scripts and configurations:
 * - Base layer (OSTree) deployment
 * - System layer container deployment
 * - User layer Flatpak deployment
 * - Layer dependency management
 * - Layer health monitoring
 */

class LayerGenerator {
    
    /**
     * Generate layer deployment scripts
     */
    fun generateLayerDeployment(
        config: CompiledConfig,
        outputDir: File
    ) {
        val layersConfig = config.layers ?: return
        
        val layerDir = File(outputDir, "layers")
        layerDir.mkdirs()
        
        // Generate main layer deployment script
        generateMainLayerScript(layersConfig, layerDir)
        
        // Generate base layer script
        generateBaseLayerScript(layersConfig.base, layerDir)
        
        // Generate system layer scripts
        for (layer in layersConfig.system) {
            generateSystemLayerScript(layer, layerDir)
        }
        
        // Generate user layer script
        generateUserLayerScript(layersConfig.user, layerDir)
        
        // Generate layer management scripts
        generateLayerManagementScript(layersConfig, layerDir)
        generateLayerHealthCheck(layersConfig, layerDir)
    }
    
    /**
     * Generate main layer deployment script
     */
    private fun generateMainLayerScript(
        config: LayersConfig,
        outputDir: File
    ) {
        val script = buildString {
            appendLine("#!/bin/bash")
            appendLine("# HorizonOS Layer Deployment Script")
            appendLine("# Auto-generated - do not edit manually")
            appendLine()
            
            appendLine("set -euo pipefail")
            appendLine()
            
            appendLine("# Configuration")
            appendLine("LAYER_DIR=\"\$(dirname \"\$0\")\"")
            appendLine("LOG_FILE=\"/var/log/horizonos-layers.log\"")
            appendLine()
            
            appendLine("# Logging function")
            appendLine("log() {")
            appendLine("    echo \"[\$(date '+%Y-%m-%d %H:%M:%S')] \$1\" | tee -a \"\$LOG_FILE\"")
            appendLine("}")
            appendLine()
            
            appendLine("# Deploy layers in order")
            appendLine("deploy_layers() {")
            appendLine("    log \"Starting layer deployment...\"")
            appendLine()
            
            // Deploy base layer first
            appendLine("    # Deploy base layer")
            appendLine("    log \"Deploying base layer...\"")
            appendLine("    if ! \"\$LAYER_DIR/deploy-base.sh\"; then")
            appendLine("        log \"ERROR: Failed to deploy base layer\"")
            appendLine("        return 1")
            appendLine("    fi")
            appendLine()
            
            // Deploy system layers in dependency order
            val sortedLayers = sortLayersByDependencies(config.system)
            for (layer in sortedLayers) {
                appendLine("    # Deploy system layer: ${layer.name}")
                appendLine("    log \"Deploying system layer: ${layer.name}\"")
                appendLine("    if ! \"\$LAYER_DIR/deploy-${layer.name}.sh\"; then")
                appendLine("        log \"ERROR: Failed to deploy system layer: ${layer.name}\"")
                appendLine("        return 1")
                appendLine("    fi")
                appendLine()
            }
            
            // Deploy user layer last
            appendLine("    # Deploy user layer")
            appendLine("    log \"Deploying user layer...\"")
            appendLine("    if ! \"\$LAYER_DIR/deploy-user.sh\"; then")
            appendLine("        log \"ERROR: Failed to deploy user layer\"")
            appendLine("        return 1")
            appendLine("    fi")
            appendLine()
            
            appendLine("    log \"Layer deployment completed successfully\"")
            appendLine("}")
            appendLine()
            
            appendLine("# Start auto-start layers")
            appendLine("start_layers() {")
            appendLine("    log \"Starting auto-start layers...\"")
            for (layer in config.system.filter { it.autoStart }) {
                appendLine("    log \"Starting layer: ${layer.name}\"")
                appendLine("    systemctl start horizonos-layer-${layer.name} || true")
            }
            appendLine("}")
            appendLine()
            
            appendLine("# Main execution")
            appendLine("main() {")
            appendLine("    deploy_layers")
            appendLine("    start_layers")
            appendLine("}")
            appendLine()
            
            appendLine("main \"\$@\"")
        }
        
        val scriptFile = File(outputDir, "deploy-layers.sh")
        scriptFile.writeText(script)
        scriptFile.setExecutable(true)
    }
    
    /**
     * Generate base layer deployment script
     */
    private fun generateBaseLayerScript(
        base: BaseLayer,
        outputDir: File
    ) {
        val script = buildString {
            appendLine("#!/bin/bash")
            appendLine("# Base layer deployment script")
            appendLine("# Auto-generated - do not edit manually")
            appendLine()
            
            appendLine("set -euo pipefail")
            appendLine()
            
            appendLine("# Configuration")
            appendLine("OSTREE_REF=\"${base.ostreeRef}\"")
            base.ostreeCommit?.let {
                appendLine("OSTREE_COMMIT=\"$it\"")
            }
            appendLine("VERSION=\"${base.version}\"")
            appendLine()
            
            appendLine("# Deploy base layer")
            appendLine("echo \"Deploying base layer...\"")
            appendLine()
            
            // Check if OSTree is available
            appendLine("if ! command -v ostree &> /dev/null; then")
            appendLine("    echo \"ERROR: OSTree not found\"")
            appendLine("    exit 1")
            appendLine("fi")
            appendLine()
            
            // Deploy OSTree commit if specified
            base.ostreeCommit?.let {
                appendLine("# Deploy specific OSTree commit")
                appendLine("echo \"Deploying OSTree commit: $it\"")
                appendLine("if ! ostree admin deploy \"\$OSTREE_REF\" --retain; then")
                appendLine("    echo \"ERROR: Failed to deploy OSTree commit\"")
                appendLine("    exit 1")
                appendLine("fi")
                appendLine()
            }
            
            // Install base packages
            if (base.packages.isNotEmpty()) {
                appendLine("# Install base packages")
                appendLine("echo \"Installing base packages...\"")
                val packages = base.packages.joinToString(" ")
                appendLine("if ! pacman -S --noconfirm $packages; then")
                appendLine("    echo \"WARNING: Failed to install some base packages\"")
                appendLine("fi")
                appendLine()
            }
            
            // Enable base services
            if (base.services.isNotEmpty()) {
                appendLine("# Enable base services")
                appendLine("echo \"Enabling base services...\"")
                for (service in base.services) {
                    appendLine("systemctl enable $service || echo \"WARNING: Failed to enable $service\"")
                }
                appendLine()
            }
            
            appendLine("echo \"Base layer deployment completed\"")
        }
        
        val scriptFile = File(outputDir, "deploy-base.sh")
        scriptFile.writeText(script)
        scriptFile.setExecutable(true)
    }
    
    /**
     * Generate system layer deployment script
     */
    private fun generateSystemLayerScript(
        layer: SystemLayer,
        outputDir: File
    ) {
        val script = buildString {
            appendLine("#!/bin/bash")
            appendLine("# System layer deployment script: ${layer.name}")
            appendLine("# Auto-generated - do not edit manually")
            appendLine()
            
            appendLine("set -euo pipefail")
            appendLine()
            
            appendLine("# Configuration")
            appendLine("LAYER_NAME=\"${layer.name}\"")
            appendLine("LAYER_PURPOSE=\"${layer.purpose}\"")
            appendLine("LAYER_PRIORITY=\"${layer.priority}\"")
            appendLine("AUTO_START=\"${layer.autoStart}\"")
            appendLine()
            
            // Check dependencies
            if (layer.dependencies.isNotEmpty()) {
                appendLine("# Check dependencies")
                appendLine("echo \"Checking dependencies for \$LAYER_NAME...\"")
                for (dep in layer.dependencies) {
                    appendLine("if ! systemctl is-active --quiet horizonos-layer-$dep; then")
                    appendLine("    echo \"ERROR: Dependency $dep is not active\"")
                    appendLine("    exit 1")
                    appendLine("fi")
                }
                appendLine()
            }
            
            appendLine("# Deploy system layer")
            appendLine("echo \"Deploying system layer: \$LAYER_NAME\"")
            appendLine()
            
            // Deploy container
            val container = layer.container
            val runtime = container.runtime.name.lowercase()
            val imageRef = buildImageReference(container.image, container.tag, container.digest)
            
            appendLine("# Deploy container")
            appendLine("CONTAINER_NAME=\"\$LAYER_NAME\"")
            appendLine("RUNTIME=\"$runtime\"")
            appendLine("IMAGE=\"$imageRef\"")
            appendLine()
            
            appendLine("# Remove existing container if it exists")
            appendLine("if \"\$RUNTIME\" ps -a --format=\"{{.Names}}\" | grep -q \"^\$CONTAINER_NAME\$\"; then")
            appendLine("    echo \"Removing existing container \$CONTAINER_NAME...\"")
            appendLine("    \"\$RUNTIME\" rm -f \"\$CONTAINER_NAME\" || true")
            appendLine("fi")
            appendLine()
            
            appendLine("# Create container")
            appendLine("echo \"Creating container \$CONTAINER_NAME...\"")
            
            // Build container creation command
            val createCmd = buildCreateCommand(container, runtime)
            appendLine("$createCmd \\")
            appendLine("    \"\$IMAGE\"")
            appendLine()
            
            // Install packages
            if (container.packages.isNotEmpty()) {
                appendLine("# Install packages")
                appendLine("echo \"Installing packages in \$CONTAINER_NAME...\"")
                val packageManager = getPackageManager(container.runtime)
                val packages = container.packages.joinToString(" ")
                appendLine("if ! \"\$RUNTIME\" exec \"\$CONTAINER_NAME\" $packageManager $packages; then")
                appendLine("    echo \"WARNING: Failed to install some packages\"")
                appendLine("fi")
                appendLine()
            }
            
            // Create systemd service
            appendLine("# Create systemd service")
            generateSystemdService(layer, outputDir)
            
            appendLine("# Enable systemd service")
            appendLine("systemctl daemon-reload")
            appendLine("systemctl enable horizonos-layer-\$LAYER_NAME")
            appendLine()
            
            appendLine("echo \"System layer \$LAYER_NAME deployed successfully\"")
        }
        
        val scriptFile = File(outputDir, "deploy-${layer.name}.sh")
        scriptFile.writeText(script)
        scriptFile.setExecutable(true)
    }
    
    /**
     * Generate user layer deployment script
     */
    private fun generateUserLayerScript(
        userLayer: UserLayer,
        outputDir: File
    ) {
        val script = buildString {
            appendLine("#!/bin/bash")
            appendLine("# User layer deployment script")
            appendLine("# Auto-generated - do not edit manually")
            appendLine()
            
            appendLine("set -euo pipefail")
            appendLine()
            
            appendLine("# Configuration")
            appendLine("AUTO_UPDATES=\"${userLayer.autoUpdates}\"")
            appendLine("USER_SCOPE=\"${userLayer.userScope}\"")
            appendLine()
            
            appendLine("# Deploy user layer")
            appendLine("echo \"Deploying user layer...\"")
            appendLine()
            
            // Install Flatpak applications
            if (userLayer.flatpaks.isNotEmpty()) {
                appendLine("# Install Flatpak applications")
                appendLine("echo \"Installing Flatpak applications...\"")
                appendLine()
                
                val scope = if (userLayer.userScope) "--user" else "--system"
                
                for (flatpak in userLayer.flatpaks) {
                    appendLine("# Install ${flatpak.id}")
                    appendLine("echo \"Installing Flatpak: ${flatpak.id}\"")
                    appendLine("if ! flatpak install $scope -y \"${flatpak.id}\"; then")
                    appendLine("    echo \"WARNING: Failed to install ${flatpak.id}\"")
                    appendLine("fi")
                }
                appendLine()
            }
            
            // Install AppImages
            if (userLayer.appImages.isNotEmpty()) {
                appendLine("# Install AppImages")
                appendLine("echo \"Installing AppImages...\"")
                appendLine("mkdir -p /opt/appimages")
                appendLine()
                
                for (appImage in userLayer.appImages) {
                    appendLine("# Install ${appImage.name}")
                    appendLine("echo \"Installing AppImage: ${appImage.name}\"")
                    appendLine("if ! wget -O \"/opt/appimages/${appImage.name}.AppImage\" \"${appImage.url}\"; then")
                    appendLine("    echo \"WARNING: Failed to download ${appImage.name}\"")
                    appendLine("else")
                    appendLine("    chmod +x \"/opt/appimages/${appImage.name}.AppImage\"")
                    appendLine("fi")
                }
                appendLine()
            }
            
            // Install Snap packages
            if (userLayer.snaps.isNotEmpty()) {
                appendLine("# Install Snap packages")
                appendLine("echo \"Installing Snap packages...\"")
                appendLine()
                
                for (snap in userLayer.snaps) {
                    appendLine("# Install ${snap.name}")
                    appendLine("echo \"Installing Snap: ${snap.name}\"")
                    val snapCmd = buildString {
                        append("snap install ${snap.name}")
                        if (snap.channel != "stable") append(" --channel=${snap.channel}")
                        if (snap.classic) append(" --classic")
                        if (snap.devmode) append(" --devmode")
                    }
                    appendLine("if ! $snapCmd; then")
                    appendLine("    echo \"WARNING: Failed to install ${snap.name}\"")
                    appendLine("fi")
                }
                appendLine()
            }
            
            appendLine("echo \"User layer deployment completed\"")
        }
        
        val scriptFile = File(outputDir, "deploy-user.sh")
        scriptFile.writeText(script)
        scriptFile.setExecutable(true)
    }
    
    /**
     * Generate layer management script
     */
    private fun generateLayerManagementScript(
        config: LayersConfig,
        outputDir: File
    ) {
        val script = buildString {
            appendLine("#!/bin/bash")
            appendLine("# Layer management script")
            appendLine("# Auto-generated - do not edit manually")
            appendLine()
            
            appendLine("set -euo pipefail")
            appendLine()
            
            appendLine("# Configuration")
            appendLine("LAYER_DIR=\"\$(dirname \"\$0\")\"")
            appendLine()
            
            val systemLayers = config.system.map { it.name }
            appendLine("SYSTEM_LAYERS=(${systemLayers.joinToString(" ")})")
            appendLine()
            
            appendLine("# Functions")
            appendLine("list_layers() {")
            appendLine("    echo \"HorizonOS Layers:\"")
            appendLine("    echo \"  base: \$(systemctl is-active ostree-finalize-staged || echo 'inactive')\"")
            appendLine("    for layer in \"\${SYSTEM_LAYERS[@]}\"; do")
            appendLine("        status=\$(systemctl is-active horizonos-layer-\$layer || echo 'inactive')")
            appendLine("        echo \"  \$layer: \$status\"")
            appendLine("    done")
            appendLine("    echo \"  user: active\"")
            appendLine("}")
            appendLine()
            
            appendLine("start_layers() {")
            appendLine("    for layer in \"\${SYSTEM_LAYERS[@]}\"; do")
            appendLine("        echo \"Starting layer \$layer...\"")
            appendLine("        systemctl start horizonos-layer-\$layer || echo \"Failed to start \$layer\"")
            appendLine("    done")
            appendLine("}")
            appendLine()
            
            appendLine("stop_layers() {")
            appendLine("    for layer in \"\${SYSTEM_LAYERS[@]}\"; do")
            appendLine("        echo \"Stopping layer \$layer...\"")
            appendLine("        systemctl stop horizonos-layer-\$layer || echo \"Failed to stop \$layer\"")
            appendLine("    done")
            appendLine("}")
            appendLine()
            
            appendLine("restart_layers() {")
            appendLine("    stop_layers")
            appendLine("    start_layers")
            appendLine("}")
            appendLine()
            
            appendLine("# Main execution")
            appendLine("case \"\${1:-list}\" in")
            appendLine("    \"list\")")
            appendLine("        list_layers")
            appendLine("        ;;")
            appendLine("    \"start\")")
            appendLine("        start_layers")
            appendLine("        ;;")
            appendLine("    \"stop\")")
            appendLine("        stop_layers")
            appendLine("        ;;")
            appendLine("    \"restart\")")
            appendLine("        restart_layers")
            appendLine("        ;;")
            appendLine("    *)")
            appendLine("        echo \"Usage: \$0 {list|start|stop|restart}\"")
            appendLine("        exit 1")
            appendLine("        ;;")
            appendLine("esac")
        }
        
        val scriptFile = File(outputDir, "manage-layers.sh")
        scriptFile.writeText(script)
        scriptFile.setExecutable(true)
    }
    
    /**
     * Generate layer health check script
     */
    private fun generateLayerHealthCheck(
        config: LayersConfig,
        outputDir: File
    ) {
        val script = buildString {
            appendLine("#!/bin/bash")
            appendLine("# Layer health check script")
            appendLine("# Auto-generated - do not edit manually")
            appendLine()
            
            appendLine("set -euo pipefail")
            appendLine()
            
            appendLine("# Check base layer")
            appendLine("echo \"Checking base layer...\"")
            appendLine("if systemctl is-active --quiet ostree-finalize-staged; then")
            appendLine("    echo \"  base: HEALTHY\"")
            appendLine("else")
            appendLine("    echo \"  base: UNHEALTHY\"")
            appendLine("fi")
            appendLine()
            
            appendLine("# Check system layers")
            for (layer in config.system) {
                appendLine("echo \"Checking system layer: ${layer.name}...\"")
                appendLine("if systemctl is-active --quiet horizonos-layer-${layer.name}; then")
                appendLine("    echo \"  ${layer.name}: HEALTHY\"")
                
                // Add custom health check if specified
                layer.healthCheck?.let { healthCheck ->
                    appendLine("    # Custom health check")
                    appendLine("    if ${healthCheck.command}; then")
                    appendLine("        echo \"  ${layer.name}: HEALTH_CHECK_PASSED\"")
                    appendLine("    else")
                    appendLine("        echo \"  ${layer.name}: HEALTH_CHECK_FAILED\"")
                    appendLine("    fi")
                }
                
                appendLine("else")
                appendLine("    echo \"  ${layer.name}: UNHEALTHY\"")
                appendLine("fi")
                appendLine()
            }
            
            appendLine("# Check user layer")
            appendLine("echo \"Checking user layer...\"")
            appendLine("if flatpak list &> /dev/null; then")
            appendLine("    echo \"  user: HEALTHY\"")
            appendLine("else")
            appendLine("    echo \"  user: UNHEALTHY\"")
            appendLine("fi")
        }
        
        val scriptFile = File(outputDir, "health-layers.sh")
        scriptFile.writeText(script)
        scriptFile.setExecutable(true)
    }
    
    /**
     * Generate systemd service for system layer
     */
    private fun generateSystemdService(
        layer: SystemLayer,
        outputDir: File
    ) {
        val service = buildString {
            appendLine("[Unit]")
            appendLine("Description=HorizonOS System Layer: ${layer.name}")
            appendLine("After=multi-user.target")
            
            // Add dependencies
            if (layer.dependencies.isNotEmpty()) {
                val deps = layer.dependencies.joinToString(" ") { "horizonos-layer-$it.service" }
                appendLine("Requires=$deps")
                appendLine("After=$deps")
            }
            
            appendLine()
            appendLine("[Service]")
            appendLine("Type=forking")
            appendLine("RemainAfterExit=yes")
            
            val runtime = layer.container.runtime.name.lowercase()
            appendLine("ExecStart=$runtime start ${layer.name}")
            appendLine("ExecStop=$runtime stop ${layer.name}")
            appendLine("ExecReload=$runtime restart ${layer.name}")
            
            appendLine("TimeoutStartSec=60")
            appendLine("TimeoutStopSec=30")
            appendLine("Restart=on-failure")
            appendLine("RestartSec=5")
            
            appendLine()
            appendLine("[Install]")
            appendLine("WantedBy=multi-user.target")
        }
        
        val serviceFile = File(outputDir, "horizonos-layer-${layer.name}.service")
        serviceFile.writeText(service)
    }
    
    // Helper functions
    
    private fun sortLayersByDependencies(layers: List<SystemLayer>): List<SystemLayer> {
        val sorted = mutableListOf<SystemLayer>()
        val remaining = layers.toMutableList()
        val processed = mutableSetOf<String>()
        
        while (remaining.isNotEmpty()) {
            val readyLayers = remaining.filter { layer ->
                layer.dependencies.all { it in processed }
            }
            
            if (readyLayers.isEmpty()) {
                // Add remaining layers without dependencies
                sorted.addAll(remaining)
                break
            }
            
            val nextLayer = readyLayers.minByOrNull { it.priority }!!
            sorted.add(nextLayer)
            processed.add(nextLayer.name)
            remaining.remove(nextLayer)
        }
        
        return sorted
    }
    
    private fun buildImageReference(image: String, tag: String, digest: String?): String {
        return if (digest != null) {
            "$image@$digest"
        } else {
            "$image:$tag"
        }
    }
    
    private fun buildCreateCommand(container: SystemContainer, runtime: String): String {
        val cmd = mutableListOf<String>()
        cmd.add("\"$runtime\" create")
        cmd.add("--name \"${container.name}\"")
        
        // Add basic container options
        container.hostname?.let { cmd.add("--hostname \"$it\"") }
        container.user?.let { cmd.add("--user \"$it\"") }
        container.workingDir?.let { cmd.add("--workdir \"$it\"") }
        
        // Add environment variables
        for ((key, value) in container.environment) {
            cmd.add("--env \"$key=$value\"")
        }
        
        // Add ports
        for (port in container.ports) {
            cmd.add("--publish \"$port\"")
        }
        
        // Add mounts
        for (mount in container.persistent) {
            cmd.add("--volume \"$mount\"")
        }
        
        // Add labels
        for ((key, value) in container.labels) {
            cmd.add("--label \"$key=$value\"")
        }
        
        // Add network mode
        if (container.networkMode != "bridge") {
            cmd.add("--network \"${container.networkMode}\"")
        }
        
        // Add privileged flag
        if (container.privileged) {
            cmd.add("--privileged")
        }
        
        return cmd.joinToString(" \\\n    ")
    }
    
    private fun getPackageManager(runtime: ContainerRuntime): String {
        return when (runtime) {
            ContainerRuntime.PODMAN, ContainerRuntime.DOCKER -> "pacman -S --noconfirm"
            ContainerRuntime.TOOLBOX -> "dnf install -y"
            ContainerRuntime.DISTROBOX -> "pacman -S --noconfirm"
        }
    }
}