package org.horizonos.config.compiler.generators

import org.horizonos.config.dsl.CompiledConfig
import org.horizonos.config.compiler.FileType
import org.horizonos.config.compiler.GeneratedFile
import org.horizonos.config.dsl.*
import org.horizonos.config.dsl.services.*
import org.horizonos.config.dsl.security.*
import org.horizonos.config.dsl.hardware.*
import org.horizonos.config.dsl.network.*
import java.io.File
import java.time.Instant
import java.time.ZoneId
import java.time.format.DateTimeFormatter

/**
 * Generator for system documentation
 */
class DocumentationGenerator(
    private val outputDir: File,
    private val generatedFiles: MutableList<GeneratedFile>
) {
    
    fun generate(config: CompiledConfig) {
        generateReadme(config)
        generateSystemOverview(config)
        generateServiceDocumentation(config)
        generateNetworkDocumentation(config)
        generateSecurityDocumentation(config)
        generateMaintenanceGuide(config)
    }
    
    private fun generateReadme(config: CompiledConfig) {
        val readmeFile = File(outputDir, "docs/README.md")
        readmeFile.writeText(buildString {
            appendLine("# ${config.system.hostname} System Configuration")
            appendLine()
            appendLine("Generated on: ${getCurrentTimestamp()}")
            appendLine("Version: rolling")
            appendLine()
            appendLine("## Overview")
            appendLine("HorizonOS system configuration")
            appendLine()
            
            // System information
            appendLine("## System Information")
            appendLine("- **Hostname**: ${config.system.hostname}")
            appendLine("- **Timezone**: ${config.system.timezone}")
            appendLine("- **Locale**: ${config.system.locale}")
            appendLine("- **Architecture**: x86_64")
            appendLine()
            
            // Enabled features
            appendLine("## Enabled Features")
            if (config.boot != null) appendLine("- Boot configuration")
            if (config.hardware != null) appendLine("- Hardware optimization")
            if (config.network != null) appendLine("- Network configuration")
            if (config.storage != null) appendLine("- Storage management")
            if (config.security != null) appendLine("- Security hardening")
            if (config.desktop != null) appendLine("- Desktop environment")
            if (config.services.isNotEmpty()) appendLine("- System services")
            if (config.automation != null) appendLine("- Automation workflows")
            appendLine()
            
            // Quick links
            appendLine("## Documentation")
            appendLine("- [System Overview](system-overview.md)")
            appendLine("- [Service Documentation](services/)")
            appendLine("- [Network Configuration](network.md)")
            appendLine("- [Security Configuration](security.md)")
            appendLine("- [Maintenance Guide](maintenance.md)")
        })
        generatedFiles.add(GeneratedFile("docs/README.md", FileType.DOCUMENTATION))
    }
    
    private fun generateSystemOverview(config: CompiledConfig) {
        val overviewFile = File(outputDir, "docs/system-overview.md")
        overviewFile.writeText(buildString {
            appendLine("# System Overview")
            appendLine()
            
            appendLine("## Hardware Configuration")
            config.hardware?.let { hw ->
                appendLine("Hardware configuration is available.")
                appendLine()
            }
            
            appendLine("## Storage Configuration")
            config.storage?.let { storage ->
                appendLine("### Filesystems")
                storage.filesystems.forEach { fs ->
                    appendLine("- **${fs.mountPoint}**: ${fs.type} on ${fs.device}")
                    appendLine("  - Filesystem configured")
                }
                appendLine()
            }
            
            appendLine("## Boot Configuration")
            config.boot?.let { boot ->
                appendLine("- **Bootloader Type**: ${boot.bootloader.type}")
                appendLine("- Boot configuration available")
                appendLine()
            }
        })
        generatedFiles.add(GeneratedFile("docs/system-overview.md", FileType.DOCUMENTATION))
    }
    
    private fun generateServiceDocumentation(config: CompiledConfig) {
        val servicesDir = File(outputDir, "docs/services")
        servicesDir.mkdirs()
        
        // Service index
        val indexFile = File(servicesDir, "index.md")
        indexFile.writeText(buildString {
            appendLine("# System Services")
            appendLine()
            appendLine("## Configured Services")
            config.services.forEach { service ->
                appendLine("- [${service.name}](${service.name}.md) - System service")
            }
        })
        generatedFiles.add(GeneratedFile("docs/services/index.md", FileType.DOCUMENTATION))
        
        // Individual service documentation
        config.services.forEach { service ->
            val serviceFile = File(servicesDir, "${service.name}.md")
            serviceFile.writeText(generateServiceDoc(service))
            generatedFiles.add(GeneratedFile("docs/services/${service.name}.md", FileType.DOCUMENTATION))
        }
    }
    
    private fun generateServiceDoc(service: Service): String {
        return buildString {
            appendLine("# ${service.name} Service")
            appendLine()
            appendLine("## Overview")
            appendLine("System service: ${service.name}")
            appendLine()
            
            appendLine("## Configuration")
            appendLine("- **Enabled**: ${service.enabled}")
            appendLine()
            
            appendLine("## Management Commands")
            appendLine("```bash")
            appendLine("# Start the service")
            appendLine("sudo systemctl start ${service.name}")
            appendLine()
            appendLine("# Stop the service")
            appendLine("sudo systemctl stop ${service.name}")
            appendLine()
            appendLine("# Check service status")
            appendLine("sudo systemctl status ${service.name}")
            appendLine()
            appendLine("# View logs")
            appendLine("sudo journalctl -u ${service.name}")
            appendLine("```")
        }
    }
    
    private fun generateNetworkDocumentation(config: CompiledConfig) {
        config.network?.let { network ->
            val networkFile = File(outputDir, "docs/network.md")
            networkFile.writeText(buildString {
                appendLine("# Network Configuration")
                appendLine()
                
                appendLine("## Hostname")
                appendLine("- **Hostname**: ${network.hostname.ifEmpty { "Not configured" }}")
                appendLine("- **Domain**: ${network.domainName.ifEmpty { "Not configured" }}")
                appendLine()
                
                if (network.interfaces.isNotEmpty()) {
                    appendLine("## Network Interfaces")
                    network.interfaces.forEach { iface ->
                        appendLine("### ${iface.name}")
                        appendLine("- **Type**: ${iface.type}")
                        appendLine("- **Enabled**: ${iface.enabled}")
                        appendLine()
                    }
                }
                
                appendLine("## Firewall Configuration")
                appendLine("- **Enabled**: ${network.firewall.enabled}")
                appendLine()
            })
            generatedFiles.add(GeneratedFile("docs/network.md", FileType.DOCUMENTATION))
        }
    }
    
    private fun generateSecurityDocumentation(config: CompiledConfig) {
        config.security?.let { security ->
            val securityFile = File(outputDir, "docs/security.md")
            securityFile.writeText(buildString {
                appendLine("# Security Configuration")
                appendLine()
                
                appendLine("## Overview")
                appendLine("This document outlines the security measures configured for this system.")
                appendLine()
                
                security.selinux?.let { selinux ->
                    appendLine("## SELinux")
                    appendLine("- **Mode**: ${selinux.mode}")
                    appendLine()
                }
                
                security.apparmor?.let { apparmor ->
                    appendLine("## AppArmor")
                    appendLine("- **Enabled**: ${apparmor.enabled}")
                    appendLine("- **Mode**: ${apparmor.mode}")
                    appendLine()
                }
            })
            generatedFiles.add(GeneratedFile("docs/security.md", FileType.DOCUMENTATION))
        }
    }
    
    private fun generateMaintenanceGuide(config: CompiledConfig) {
        val maintenanceFile = File(outputDir, "docs/maintenance.md")
        maintenanceFile.writeText(buildString {
            appendLine("# System Maintenance Guide")
            appendLine()
            appendLine("## Regular Maintenance Tasks")
            appendLine()
            
            appendLine("### Daily Tasks")
            appendLine("- Check system logs: `journalctl -p err -b`")
            appendLine("- Monitor disk usage: `df -h`")
            appendLine("- Check service status: `systemctl status`")
            appendLine()
            
            appendLine("### Weekly Tasks")
            appendLine("- Update system packages: `sudo pacman -Syu`")
            appendLine("- Check for failed services: `systemctl --failed`")
            appendLine("- Review security logs: `sudo aureport --summary`")
            appendLine()
            
            appendLine("### Monthly Tasks")
            appendLine("- Clean package cache: `sudo pacman -Sc`")
            appendLine("- Check filesystem integrity: `sudo btrfs scrub start /`")
            appendLine("- Review user accounts: `sudo lastlog`")
            appendLine()
            
            config.automation?.let { automation ->
                if (automation.workflows.isNotEmpty()) {
                    appendLine("## Automation Workflows")
                    automation.workflows.forEach { workflow ->
                        appendLine("### ${workflow.name}")
                        appendLine("- **Description**: ${workflow.description}")
                        appendLine("- **Enabled**: ${workflow.enabled}")
                        appendLine("- **Priority**: ${workflow.priority}")
                        appendLine()
                    }
                }
            }
            
            appendLine("## Backup Procedures")
            config.storage?.let { storage ->
                appendLine("Storage configuration available")
            } ?: run {
                appendLine("No automated backups configured. Consider setting up regular backups.")
            }
            appendLine()
            
            appendLine("## Troubleshooting")
            appendLine()
            appendLine("### System Logs")
            appendLine("- System log: `journalctl -xe`")
            appendLine("- Kernel log: `dmesg`")
            appendLine("- Authentication log: `journalctl -u sshd`")
            appendLine()
            
            appendLine("### Performance Issues")
            appendLine("- Check CPU usage: `top` or `htop`")
            appendLine("- Check memory usage: `free -h`")
            appendLine("- Check disk I/O: `iotop`")
            appendLine("- Check network: `iftop` or `nethogs`")
        })
        generatedFiles.add(GeneratedFile("docs/maintenance.md", FileType.DOCUMENTATION))
    }
    
    private fun getCurrentTimestamp(): String {
        val formatter = DateTimeFormatter.ofPattern("yyyy-MM-dd HH:mm:ss z")
            .withZone(ZoneId.systemDefault())
        return formatter.format(Instant.now())
    }
}