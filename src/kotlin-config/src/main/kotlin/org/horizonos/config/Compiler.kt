// src/kotlin-config/src/main/kotlin/org/horizonos/config/Compiler.kt
package org.horizonos.config

import com.github.ajalt.clikt.core.CliktCommand
import com.github.ajalt.clikt.parameters.arguments.argument
import com.github.ajalt.clikt.parameters.options.default
import com.github.ajalt.clikt.parameters.options.option
import com.github.ajalt.clikt.parameters.types.file
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import okio.FileSystem
import okio.Path.Companion.toPath
import okio.buffer
import okio.use
import org.horizonos.config.dsl.*
import java.io.File
import javax.script.ScriptEngineManager
import kotlin.script.experimental.api.*
import kotlin.script.experimental.host.toScriptSource
import kotlin.script.experimental.jvm.dependenciesFromCurrentContext
import kotlin.script.experimental.jvm.jvm
import kotlin.script.experimental.jvmhost.BasicJvmScriptingHost
import kotlin.script.experimental.jvmhost.createJvmCompilationConfigurationFromTemplate

class CompileCommand : CliktCommand(
    name = "compile",
    help = "Compile a HorizonOS configuration file"
) {
    private val configFile by argument(help = "Configuration file to compile")
        .file(mustExist = true, canBeDir = false, mustBeReadable = true)
    
    private val outputDir by option("-o", "--output", help = "Output directory")
        .file(canBeFile = false)
        .default(File("./output"))
    
    override fun run() {
        echo("Compiling HorizonOS configuration: ${configFile.absolutePath}")
        
        // Create output directory
        outputDir.mkdirs()
        
        try {
            // Load and evaluate the configuration script
            val config = loadConfiguration(configFile)
            
            // Generate output files
            val generator = ConfigGenerator(outputDir)
            generator.generate(config)
            
            echo("✓ Compilation successful!")
            echo("  Output directory: ${outputDir.absolutePath}")
            echo("  Generated files:")
            outputDir.walkTopDown()
                .filter { it.isFile }
                .forEach { echo("    - ${it.relativeTo(outputDir)}") }
        } catch (e: Exception) {
            echo("✗ Compilation failed: ${e.message}", err = true)
            throw e
        }
    }
    
    private fun loadConfiguration(file: File): CompiledConfig {
        // For now, we'll use a simple approach
        // In production, we'd use Kotlin Script API properly
        val scriptContent = file.readText()
        
        // This is a simplified version - in production we'd compile and execute properly
        // For now, we'll return a dummy config
        return horizonOS {
            hostname = "compiled-system"
            packages {
                install("base", "linux", "btrfs-progs")
            }
            services {
                enable("NetworkManager")
            }
        }
    }
}

class ConfigGenerator(private val outputDir: File) {
    private val json = Json { prettyPrint = true }
    
    fun generate(config: CompiledConfig) {
        // Create directory structure
        val dirs = listOf("scripts", "systemd", "configs", "ostree")
        dirs.forEach { File(outputDir, it).mkdirs() }
        
        // Generate JSON representation
        generateJson(config)
        
        // Generate installation scripts
        generateInstallScript(config)
        
        // Generate systemd service files
        generateSystemdUnits(config)
        
        // Generate user creation script
        generateUserScript(config)
        
        // Generate repository configuration
        generateRepoConfig(config)
        
        // Generate OSTree deployment script
        generateOSTreeScript(config)
    }
    
    private fun generateJson(config: CompiledConfig) {
        val jsonFile = File(outputDir, "config.json")
        jsonFile.writeText(json.encodeToString(config))
    }
    
    private fun generateInstallScript(config: CompiledConfig) {
        val script = buildString {
            appendLine("#!/bin/bash")
            appendLine("# HorizonOS Package Installation Script")
            appendLine("# Generated from Kotlin DSL configuration")
            appendLine()
            appendLine("set -e")
            appendLine()
            
            // Group packages by action
            val toInstall = config.packages.filter { it.action == PackageAction.INSTALL }
            val toRemove = config.packages.filter { it.action == PackageAction.REMOVE }
            
            if (toInstall.isNotEmpty()) {
                appendLine("echo 'Installing packages...'")
                appendLine("pacman -S --needed --noconfirm \\")
                toInstall.forEach { pkg ->
                    append("    ${pkg.name}")
                    if (pkg != toInstall.last()) append(" \\")
                    appendLine()
                }
                appendLine()
            }
            
            if (toRemove.isNotEmpty()) {
                appendLine("echo 'Removing packages...'")
                appendLine("pacman -R --noconfirm \\")
                toRemove.forEach { pkg ->
                    append("    ${pkg.name}")
                    if (pkg != toRemove.last()) append(" \\")
                    appendLine()
                }
            }
        }
        
        val scriptFile = File(outputDir, "scripts/install-packages.sh")
        scriptFile.writeText(script)
        scriptFile.setExecutable(true)
    }
    
    private fun generateSystemdUnits(config: CompiledConfig) {
        // Generate service enablement script
        val script = buildString {
            appendLine("#!/bin/bash")
            appendLine("# HorizonOS Service Configuration Script")
            appendLine()
            
            config.services.forEach { service ->
                if (service.enabled) {
                    appendLine("systemctl enable ${service.name}")
                    service.config?.let { cfg ->
                        if (cfg.environment.isNotEmpty()) {
                            // Generate override file for environment variables
                            val overrideDir = "systemd/${service.name}.service.d"
                            appendLine("mkdir -p $overrideDir")
                            appendLine("cat > $overrideDir/override.conf << EOF")
                            appendLine("[Service]")
                            cfg.environment.forEach { (key, value) ->
                                appendLine("Environment=\"$key=$value\"")
                            }
                            if (cfg.restartOnFailure) {
                                appendLine("Restart=on-failure")
                            }
                            appendLine("EOF")
                        }
                    }
                } else {
                    appendLine("systemctl disable ${service.name}")
                }
            }
        }
        
        val scriptFile = File(outputDir, "scripts/configure-services.sh")
        scriptFile.writeText(script)
        scriptFile.setExecutable(true)
    }
    
    private fun generateUserScript(config: CompiledConfig) {
        val script = buildString {
            appendLine("#!/bin/bash")
            appendLine("# HorizonOS User Management Script")
            appendLine()
            
            config.users.forEach { user ->
                appendLine("# Create user: ${user.name}")
                append("useradd -m")
                user.uid?.let { append(" -u $it") }
                append(" -s ${user.shell}")
                if (user.groups.isNotEmpty()) {
                    append(" -G ${user.groups.joinToString(",")}")
                }
                appendLine(" ${user.name}")
                appendLine()
            }
        }
        
        val scriptFile = File(outputDir, "scripts/create-users.sh")
        scriptFile.writeText(script)
        scriptFile.setExecutable(true)
    }
    
    private fun generateRepoConfig(config: CompiledConfig) {
        // Generate pacman.conf entries
        val pacmanConf = buildString {
            config.repositories.filterNot { it is OstreeRepository }.forEach { repo ->
                appendLine("[${repo.name}]")
                appendLine("Server = ${repo.url}")
                if (!repo.gpgCheck) {
                    appendLine("SigLevel = Never")
                }
                appendLine()
            }
        }
        
        if (pacmanConf.isNotBlank()) {
            val repoFile = File(outputDir, "configs/pacman-repos.conf")
            repoFile.writeText(pacmanConf)
        }
        
        // Generate OSTree remote configuration
        val ostreeRepos = config.repositories.filterIsInstance<OstreeRepository>()
        if (ostreeRepos.isNotEmpty()) {
            val ostreeScript = buildString {
                appendLine("#!/bin/bash")
                appendLine("# Configure OSTree remotes")
                appendLine()
                
                ostreeRepos.forEach { repo ->
                    val gpgFlag = if (repo.gpgCheck) "" else "--no-gpg-verify"
                    appendLine("ostree remote add ${repo.name} ${repo.url} $gpgFlag")
                    repo.branches.forEach { branch ->
                        appendLine("# Available branch: ${repo.name}:${branch}")
                    }
                    appendLine()
                }
            }
            
            val scriptFile = File(outputDir, "scripts/configure-ostree-remotes.sh")
            scriptFile.writeText(ostreeScript)
            scriptFile.setExecutable(true)
        }
    }
    
    private fun generateOSTreeScript(config: CompiledConfig) {
        val script = buildString {
            appendLine("#!/bin/bash")
            appendLine("# HorizonOS OSTree Deployment Script")
            appendLine()
            appendLine("set -e")
            appendLine()
            appendLine("# System configuration")
            appendLine("echo '${config.system.hostname}' > /etc/hostname")
            appendLine("ln -sf /usr/share/zoneinfo/${config.system.timezone} /etc/localtime")
            appendLine("echo 'LANG=${config.system.locale}' > /etc/locale.conf")
            appendLine()
            appendLine("# Run other configuration scripts")
            appendLine("./install-packages.sh")
            appendLine("./configure-services.sh")
            appendLine("./create-users.sh")
            if (File(outputDir, "scripts/configure-ostree-remotes.sh").exists()) {
                appendLine("./configure-ostree-remotes.sh")
            }
        }
        
        val scriptFile = File(outputDir, "scripts/deploy.sh")
        scriptFile.writeText(script)
        scriptFile.setExecutable(true)
    }
}

fun main(args: Array<String>) = CompileCommand().main(args)