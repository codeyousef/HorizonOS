package org.horizonos.config

import com.github.ajalt.clikt.core.CliktCommand
import com.github.ajalt.clikt.parameters.arguments.argument
import com.github.ajalt.clikt.parameters.options.default
import com.github.ajalt.clikt.parameters.options.option
import com.github.ajalt.clikt.parameters.types.file
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import org.horizonos.config.dsl.*
import java.io.File
import kotlin.script.experimental.api.*
import kotlin.script.experimental.host.toScriptSource
import kotlin.script.experimental.jvm.dependenciesFromCurrentContext
import kotlin.script.experimental.jvm.jvm
import kotlin.script.experimental.jvmhost.BasicJvmScriptingHost

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
            // For now, create a simple example configuration
            val config = horizonOS {
                hostname = "compiled-system"
                timezone = "UTC"
                locale = "en_US.UTF-8"

                packages {
                    install("base", "linux", "btrfs-progs", "networkmanager")
                }

                services {
                    enable("NetworkManager")
                    enable("sshd")
                }

                users {
                    user("admin") {
                        uid = 1000
                        shell = "/usr/bin/fish"
                        groups("wheel", "users")
                    }
                }
            }

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
        val script = buildString {
            appendLine("#!/bin/bash")
            appendLine("# HorizonOS Service Configuration Script")
            appendLine()

            config.services.forEach { service ->
                if (service.enabled) {
                    appendLine("systemctl enable ${service.name}")
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
        }

        val scriptFile = File(outputDir, "scripts/deploy.sh")
        scriptFile.writeText(script)
        scriptFile.setExecutable(true)
    }
}

fun main(args: Array<String>) = CompileCommand().main(args)
