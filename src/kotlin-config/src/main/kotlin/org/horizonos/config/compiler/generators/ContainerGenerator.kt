package org.horizonos.config.compiler.generators

import org.horizonos.config.dsl.*
import java.io.File

/**
 * Container Generator for HorizonOS
 * 
 * Generates container deployment scripts and configurations:
 * - Podman/Docker/Distrobox container creation scripts
 * - Container startup and management scripts
 * - Binary export scripts
 * - Health check scripts
 * - Container network and volume configurations
 */

class ContainerGenerator {
    
    /**
     * Generate container deployment scripts
     */
    fun generateContainerDeployment(
        config: CompiledConfig,
        outputDir: File
    ) {
        val containersConfig = config.containers ?: return
        
        val containerDir = File(outputDir, "containers")
        containerDir.mkdirs()
        
        // Generate main deployment script
        generateMainDeploymentScript(containersConfig, containerDir)
        
        // Generate individual container scripts
        for (container in containersConfig.containers) {
            generateContainerScript(container, containerDir)
            generateContainerHealthCheck(container, containerDir)
            generateBinaryExportScript(container, containerDir)
        }
        
        // Generate container management scripts
        generateContainerManagementScript(containersConfig, containerDir)
        generateContainerCleanupScript(containersConfig, containerDir)
    }
    
    /**
     * Generate main container deployment script
     */
    private fun generateMainDeploymentScript(
        config: ContainersConfig,
        outputDir: File
    ) {
        val script = buildString {
            appendLine("#!/bin/bash")
            appendLine("# HorizonOS Container Deployment Script")
            appendLine("# Auto-generated - do not edit manually")
            appendLine()
            
            appendLine("set -euo pipefail")
            appendLine()
            
            appendLine("# Configuration")
            appendLine("CONTAINER_DIR=\"\$(dirname \"\$0\")\"")
            appendLine("LOG_FILE=\"/var/log/horizonos-containers.log\"")
            appendLine("DEFAULT_RUNTIME=\"${config.defaultRuntime.name.lowercase()}\"")
            appendLine()
            
            appendLine("# Logging function")
            appendLine("log() {")
            appendLine("    echo \"[\$(date '+%Y-%m-%d %H:%M:%S')] \$1\" | tee -a \"\$LOG_FILE\"")
            appendLine("}")
            appendLine()
            
            appendLine("# Check if runtime is available")
            appendLine("check_runtime() {")
            appendLine("    local runtime=\"\$1\"")
            appendLine("    if ! command -v \"\$runtime\" &> /dev/null; then")
            appendLine("        log \"ERROR: Runtime \$runtime not found\"")
            appendLine("        return 1")
            appendLine("    fi")
            appendLine("}")
            appendLine()
            
            appendLine("# Deploy containers")
            appendLine("deploy_containers() {")
            appendLine("    log \"Starting container deployment...\"")
            appendLine()
            
            // Add global mounts
            if (config.globalMounts.isNotEmpty()) {
                appendLine("    # Create global mount directories")
                for (mount in config.globalMounts) {
                    appendLine("    mkdir -p \"$mount\"")
                }
                appendLine()
            }
            
            // Deploy each container
            for (container in config.containers) {
                appendLine("    # Deploy ${container.name}")
                appendLine("    log \"Deploying container: ${container.name}\"")
                appendLine("    if ! \"\$CONTAINER_DIR/deploy-${container.name}.sh\"; then")
                appendLine("        log \"ERROR: Failed to deploy ${container.name}\"")
                appendLine("        return 1")
                appendLine("    fi")
                appendLine()
            }
            
            appendLine("    log \"Container deployment completed successfully\"")
            appendLine("}")
            appendLine()
            
            appendLine("# Start containers if auto-start is enabled")
            appendLine("start_containers() {")
            for (container in config.containers.filter { it.autoStart }) {
                appendLine("    log \"Starting container: ${container.name}\"")
                appendLine("    ${container.runtime.name.lowercase()} start ${container.name} || true")
            }
            appendLine("}")
            appendLine()
            
            appendLine("# Main execution")
            appendLine("main() {")
            appendLine("    check_runtime \"\$DEFAULT_RUNTIME\"")
            appendLine("    deploy_containers")
            if (config.autoStart) {
                appendLine("    start_containers")
            }
            appendLine("}")
            appendLine()
            
            appendLine("main \"\$@\"")
        }
        
        val scriptFile = File(outputDir, "deploy-containers.sh")
        scriptFile.writeText(script)
        scriptFile.setExecutable(true)
    }
    
    /**
     * Generate individual container deployment script
     */
    private fun generateContainerScript(
        container: SystemContainer,
        outputDir: File
    ) {
        val script = buildString {
            appendLine("#!/bin/bash")
            appendLine("# Container deployment script for ${container.name}")
            appendLine("# Auto-generated - do not edit manually")
            appendLine()
            
            appendLine("set -euo pipefail")
            appendLine()
            
            val runtime = container.runtime.name.lowercase()
            val imageRef = buildImageReference(container.image, container.tag, container.digest)
            
            appendLine("# Configuration")
            appendLine("CONTAINER_NAME=\"${container.name}\"")
            appendLine("RUNTIME=\"$runtime\"")
            appendLine("IMAGE=\"$imageRef\"")
            appendLine("PURPOSE=\"${container.purpose}\"")
            appendLine()
            
            appendLine("# Check if container already exists")
            appendLine("if \"\$RUNTIME\" ps -a --format=\"{{.Names}}\" | grep -q \"^\$CONTAINER_NAME\$\"; then")
            appendLine("    echo \"Container \$CONTAINER_NAME already exists, removing...\"")
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
            
            // Install packages if specified
            if (container.packages.isNotEmpty()) {
                appendLine("# Install packages")
                appendLine("echo \"Installing packages in \$CONTAINER_NAME...\"")
                
                val packageManager = getPackageManager(container.runtime)
                val packages = container.packages.joinToString(" ")
                
                appendLine("if ! \"\$RUNTIME\" exec \"\$CONTAINER_NAME\" $packageManager $packages; then")
                appendLine("    echo \"Warning: Failed to install some packages\"")
                appendLine("fi")
                appendLine()
            }
            
            // Run post commands
            if (container.postCommands.isNotEmpty()) {
                appendLine("# Run post-installation commands")
                for (command in container.postCommands) {
                    appendLine("echo \"Running: $command\"")
                    appendLine("\"\$RUNTIME\" exec \"\$CONTAINER_NAME\" sh -c '$command' || true")
                }
                appendLine()
            }
            
            appendLine("echo \"Container \$CONTAINER_NAME created successfully\"")
        }
        
        val scriptFile = File(outputDir, "deploy-${container.name}.sh")
        scriptFile.writeText(script)
        scriptFile.setExecutable(true)
    }
    
    /**
     * Generate container health check script
     */
    private fun generateContainerHealthCheck(
        container: SystemContainer,
        outputDir: File
    ) {
        val script = buildString {
            appendLine("#!/bin/bash")
            appendLine("# Health check script for ${container.name}")
            appendLine("# Auto-generated - do not edit manually")
            appendLine()
            
            appendLine("set -euo pipefail")
            appendLine()
            
            val runtime = container.runtime.name.lowercase()
            
            appendLine("CONTAINER_NAME=\"${container.name}\"")
            appendLine("RUNTIME=\"$runtime\"")
            appendLine()
            
            appendLine("# Check if container exists")
            appendLine("if ! \"\$RUNTIME\" ps -a --format=\"{{.Names}}\" | grep -q \"^\$CONTAINER_NAME\$\"; then")
            appendLine("    echo \"UNHEALTHY: Container \$CONTAINER_NAME does not exist\"")
            appendLine("    exit 1")
            appendLine("fi")
            appendLine()
            
            appendLine("# Check container status")
            appendLine("STATUS=\$(\"\$RUNTIME\" inspect --format=\"{{.State.Status}}\" \"\$CONTAINER_NAME\")")
            appendLine("case \"\$STATUS\" in")
            appendLine("    \"running\")")
            appendLine("        echo \"HEALTHY: Container \$CONTAINER_NAME is running\"")
            appendLine("        exit 0")
            appendLine("        ;;")
            appendLine("    \"exited\")")
            appendLine("        echo \"UNHEALTHY: Container \$CONTAINER_NAME has exited\"")
            appendLine("        exit 1")
            appendLine("        ;;")
            appendLine("    *)")
            appendLine("        echo \"UNKNOWN: Container \$CONTAINER_NAME status: \$STATUS\"")
            appendLine("        exit 2")
            appendLine("        ;;")
            appendLine("esac")
        }
        
        val scriptFile = File(outputDir, "health-${container.name}.sh")
        scriptFile.writeText(script)
        scriptFile.setExecutable(true)
    }
    
    /**
     * Generate binary export script
     */
    private fun generateBinaryExportScript(
        container: SystemContainer,
        outputDir: File
    ) {
        if (container.binaries.isEmpty()) return
        
        val script = buildString {
            appendLine("#!/bin/bash")
            appendLine("# Binary export script for ${container.name}")
            appendLine("# Auto-generated - do not edit manually")
            appendLine()
            
            appendLine("set -euo pipefail")
            appendLine()
            
            val runtime = container.runtime.name.lowercase()
            
            appendLine("CONTAINER_NAME=\"${container.name}\"")
            appendLine("RUNTIME=\"$runtime\"")
            appendLine("BIN_DIR=\"/usr/local/bin\"")
            appendLine()
            
            appendLine("# Create bin directory if it doesn't exist")
            appendLine("mkdir -p \"\$BIN_DIR\"")
            appendLine()
            
            appendLine("# Export binaries")
            for (binary in container.binaries) {
                appendLine("# Export $binary")
                appendLine("cat > \"\$BIN_DIR/$binary\" << 'EOF'")
                appendLine("#!/bin/bash")
                appendLine("# HorizonOS Container Binary Wrapper")
                appendLine("# Generated for ${container.name}")
                appendLine("exec $runtime exec \"\$CONTAINER_NAME\" $binary \"\$@\"")
                appendLine("EOF")
                appendLine()
                appendLine("chmod +x \"\$BIN_DIR/$binary\"")
                appendLine("echo \"Exported $binary to \$BIN_DIR/$binary\"")
                appendLine()
            }
            
            appendLine("echo \"Binary export completed for \$CONTAINER_NAME\"")
        }
        
        val scriptFile = File(outputDir, "export-${container.name}.sh")
        scriptFile.writeText(script)
        scriptFile.setExecutable(true)
    }
    
    /**
     * Generate container management script
     */
    private fun generateContainerManagementScript(
        config: ContainersConfig,
        outputDir: File
    ) {
        val script = buildString {
            appendLine("#!/bin/bash")
            appendLine("# Container management script")
            appendLine("# Auto-generated - do not edit manually")
            appendLine()
            
            appendLine("set -euo pipefail")
            appendLine()
            
            appendLine("# Configuration")
            appendLine("CONTAINER_DIR=\"\$(dirname \"\$0\")\"")
            appendLine("DEFAULT_RUNTIME=\"${config.defaultRuntime.name.lowercase()}\"")
            appendLine()
            
            val containerNames = config.containers.map { it.name }
            appendLine("CONTAINERS=(${containerNames.joinToString(" ")})")
            appendLine()
            
            appendLine("# Functions")
            appendLine("list_containers() {")
            appendLine("    echo \"HorizonOS Containers:\"")
            appendLine("    for container in \"\${CONTAINERS[@]}\"; do")
            appendLine("        if \"\$DEFAULT_RUNTIME\" ps -a --format=\"{{.Names}}\" | grep -q \"^\$container\$\"; then")
            appendLine("            status=\$(\"\$DEFAULT_RUNTIME\" inspect --format=\"{{.State.Status}}\" \"\$container\")")
            appendLine("            echo \"  \$container: \$status\"")
            appendLine("        else")
            appendLine("            echo \"  \$container: not found\"")
            appendLine("        fi")
            appendLine("    done")
            appendLine("}")
            appendLine()
            
            appendLine("start_all() {")
            appendLine("    for container in \"\${CONTAINERS[@]}\"; do")
            appendLine("        echo \"Starting \$container...\"")
            appendLine("        \"\$DEFAULT_RUNTIME\" start \"\$container\" || echo \"Failed to start \$container\"")
            appendLine("    done")
            appendLine("}")
            appendLine()
            
            appendLine("stop_all() {")
            appendLine("    for container in \"\${CONTAINERS[@]}\"; do")
            appendLine("        echo \"Stopping \$container...\"")
            appendLine("        \"\$DEFAULT_RUNTIME\" stop \"\$container\" || echo \"Failed to stop \$container\"")
            appendLine("    done")
            appendLine("}")
            appendLine()
            
            appendLine("restart_all() {")
            appendLine("    stop_all")
            appendLine("    start_all")
            appendLine("}")
            appendLine()
            
            appendLine("health_check() {")
            appendLine("    for container in \"\${CONTAINERS[@]}\"; do")
            appendLine("        if [ -f \"\$CONTAINER_DIR/health-\$container.sh\" ]; then")
            appendLine("            echo \"Checking \$container...\"")
            appendLine("            \"\$CONTAINER_DIR/health-\$container.sh\"")
            appendLine("        fi")
            appendLine("    done")
            appendLine("}")
            appendLine()
            
            appendLine("# Main execution")
            appendLine("case \"\${1:-list}\" in")
            appendLine("    \"list\")")
            appendLine("        list_containers")
            appendLine("        ;;")
            appendLine("    \"start\")")
            appendLine("        start_all")
            appendLine("        ;;")
            appendLine("    \"stop\")")
            appendLine("        stop_all")
            appendLine("        ;;")
            appendLine("    \"restart\")")
            appendLine("        restart_all")
            appendLine("        ;;")
            appendLine("    \"health\")")
            appendLine("        health_check")
            appendLine("        ;;")
            appendLine("    *)")
            appendLine("        echo \"Usage: \$0 {list|start|stop|restart|health}\"")
            appendLine("        exit 1")
            appendLine("        ;;")
            appendLine("esac")
        }
        
        val scriptFile = File(outputDir, "manage-containers.sh")
        scriptFile.writeText(script)
        scriptFile.setExecutable(true)
    }
    
    /**
     * Generate container cleanup script
     */
    private fun generateContainerCleanupScript(
        config: ContainersConfig,
        outputDir: File
    ) {
        val script = buildString {
            appendLine("#!/bin/bash")
            appendLine("# Container cleanup script")
            appendLine("# Auto-generated - do not edit manually")
            appendLine()
            
            appendLine("set -euo pipefail")
            appendLine()
            
            appendLine("# Configuration")
            appendLine("DEFAULT_RUNTIME=\"${config.defaultRuntime.name.lowercase()}\"")
            appendLine()
            
            val containerNames = config.containers.map { it.name }
            appendLine("CONTAINERS=(${containerNames.joinToString(" ")})")
            appendLine()
            
            appendLine("# Remove containers")
            appendLine("echo \"Cleaning up HorizonOS containers...\"")
            appendLine("for container in \"\${CONTAINERS[@]}\"; do")
            appendLine("    if \"\$DEFAULT_RUNTIME\" ps -a --format=\"{{.Names}}\" | grep -q \"^\$container\$\"; then")
            appendLine("        echo \"Removing container \$container...\"")
            appendLine("        \"\$DEFAULT_RUNTIME\" rm -f \"\$container\" || echo \"Failed to remove \$container\"")
            appendLine("    fi")
            appendLine("done")
            appendLine()
            
            appendLine("# Remove exported binaries")
            appendLine("echo \"Removing exported binaries...\"")
            for (container in config.containers) {
                for (binary in container.binaries) {
                    appendLine("rm -f \"/usr/local/bin/$binary\"")
                }
            }
            appendLine()
            
            appendLine("echo \"Cleanup completed\"")
        }
        
        val scriptFile = File(outputDir, "cleanup-containers.sh")
        scriptFile.writeText(script)
        scriptFile.setExecutable(true)
    }
    
    // Helper functions
    
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
        
        // Add hostname if specified
        container.hostname?.let {
            cmd.add("--hostname \"$it\"")
        }
        
        // Add user if specified
        container.user?.let {
            cmd.add("--user \"$it\"")
        }
        
        // Add working directory if specified
        container.workingDir?.let {
            cmd.add("--workdir \"$it\"")
        }
        
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
        
        // Add privileged flag if needed
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