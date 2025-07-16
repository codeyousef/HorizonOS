package org.horizonos.config.compiler.generators

import org.horizonos.config.dsl.CompiledConfig
import org.horizonos.config.compiler.FileType
import org.horizonos.config.compiler.GeneratedFile
import org.horizonos.config.dsl.*
import org.horizonos.config.dsl.network.*
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import kotlinx.serialization.Serializable
import java.io.File

/**
 * Generator for OSTree manifests
 */
class OSTreeManifestGenerator(
    private val outputDir: File,
    private val generatedFiles: MutableList<GeneratedFile>
) {
    private val json = Json { 
        prettyPrint = true
        encodeDefaults = true
    }
    
    fun generate(config: CompiledConfig) {
        val manifest = createOSTreeManifest(config)
        
        // Generate OSTree manifest JSON
        val manifestFile = File(outputDir, "ostree/manifest.json")
        manifestFile.writeText(json.encodeToString(manifest))
        generatedFiles.add(GeneratedFile("ostree/manifest.json", FileType.JSON))
        
        // Generate OSTree kickstart file
        val kickstartFile = File(outputDir, "ostree/horizonos.ks")
        kickstartFile.writeText(generateKickstart(config))
        generatedFiles.add(GeneratedFile("ostree/horizonos.ks", FileType.CONFIG))
        
        // Generate repo configuration
        val repoConfigFile = File(outputDir, "ostree/repo.conf")
        repoConfigFile.writeText(generateRepoConfig(config))
        generatedFiles.add(GeneratedFile("ostree/repo.conf", FileType.CONFIG))
    }
    
    private fun createOSTreeManifest(config: CompiledConfig): OSTreeManifest {
        return OSTreeManifest(
            ref = "horizonos/stable/x86_64/${config.system.hostname}",
            metadata = mapOf(
                "version" to "rolling",
                "hostname" to config.system.hostname,
                "description" to "HorizonOS System",
                "build-date" to java.time.Instant.now().toString()
            ),
            packages = buildList {
                addAll(config.packages.map { it.name })
                addAll(config.services.map { it.name })
            }.distinct(),
            repos = config.repositories.map { it.name }
        )
    }
    
    private fun generateKickstart(config: CompiledConfig): String {
        return buildString {
            appendLine("# HorizonOS OSTree Kickstart File")
            appendLine("# Generated from configuration")
            appendLine()
            appendLine("# System configuration")
            appendLine("lang en_US.UTF-8")
            appendLine("keyboard us")
            appendLine("timezone ${config.system.timezone}")
            appendLine("rootpw --lock")
            appendLine()
            
            // OSTree configuration
            appendLine("# OSTree deployment")
            appendLine("ostreesetup --osname=horizonos --url=http://repo.horizonos.org/ostree --ref=horizonos/stable/x86_64/base --nogpg")
            appendLine()
            
            // Network configuration
            appendLine("# Network configuration")
            config.network?.interfaces?.forEach { iface ->
                // TODO: Network interface configuration
                appendLine("network --device=${iface.name} --bootproto=dhcp")
            }
            appendLine()
            
            // Partitioning
            appendLine("# Disk partitioning")
            appendLine("zerombr")
            appendLine("clearpart --all --initlabel")
            appendLine("part /boot/efi --fstype=efi --size=512")
            appendLine("part /boot --fstype=ext4 --size=1024") 
            appendLine("part / --fstype=btrfs --grow")
            appendLine()
            
            // Post-install scripts
            appendLine("%post")
            appendLine("# Post-installation configuration")
            appendLine("echo 'HorizonOS deployment complete' > /etc/horizonos-release")
            appendLine("echo 'Version: rolling' >> /etc/horizonos-release")
            appendLine("%end")
        }
    }
    
    private fun generateRepoConfig(config: CompiledConfig): String {
        return buildString {
            appendLine("[core]")
            appendLine("repo_version=1")
            appendLine("mode=archive")
            appendLine()
            appendLine("[remote \"origin\"]")
            appendLine("url=https://repo.horizonos.org/ostree")
            appendLine("gpg-verify=true")
            appendLine()
            config.repositories.forEach { repo ->
                if (repo.url.contains("ostree")) {
                    appendLine("[remote \"${repo.name}\"]")
                    appendLine("url=${repo.url}")
                    appendLine("gpg-verify=${repo.gpgCheck}")
                    appendLine()
                }
            }
        }
    }
}

// Data class for OSTree manifest
@Serializable
data class OSTreeManifest(
    val ref: String,
    val metadata: Map<String, String>,
    val packages: List<String>,
    val repos: List<String>
)