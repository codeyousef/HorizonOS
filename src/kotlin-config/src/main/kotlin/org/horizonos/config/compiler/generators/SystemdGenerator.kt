package org.horizonos.config.compiler.generators

import org.horizonos.config.dsl.CompiledConfig
import org.horizonos.config.compiler.GeneratedFile
import org.horizonos.config.compiler.FileType
import java.io.File

/**
 * Systemd units generator for HorizonOS configurations
 * Generates systemd service files and timers for system integration
 */
class SystemdGenerator(
    private val outputDir: File,
    private val generatedFiles: MutableList<GeneratedFile>
) {
    
    /**
     * Generate systemd unit files for HorizonOS services
     */
    fun generate(config: CompiledConfig) {
        generateMainConfigService()
        generateUpdateTimer()
        generateAutomationService(config)
    }
    
    private fun generateMainConfigService() {
        val serviceFile = File(outputDir, "systemd/horizonos-config.service")
        serviceFile.writeText("""
            [Unit]
            Description=HorizonOS Configuration Service
            After=multi-user.target
            
            [Service]
            Type=oneshot
            ExecStart=/usr/bin/horizonos-apply /etc/horizonos/config.json
            RemainAfterExit=yes
            StandardOutput=journal
            StandardError=journal
            
            [Install]
            WantedBy=multi-user.target
        """.trimIndent())
        generatedFiles.add(GeneratedFile("systemd/horizonos-config.service", FileType.SYSTEMD))
    }
    
    private fun generateUpdateTimer() {
        val timerFile = File(outputDir, "systemd/horizonos-update.timer")
        timerFile.writeText("""
            [Unit]
            Description=HorizonOS Configuration Update Timer
            
            [Timer]
            OnBootSec=5min
            OnUnitActiveSec=1h
            
            [Install]
            WantedBy=timers.target
        """.trimIndent())
        generatedFiles.add(GeneratedFile("systemd/horizonos-update.timer", FileType.SYSTEMD))
    }
    
    private fun generateAutomationService(config: CompiledConfig) {
        config.automation?.let {
            val automationService = File(outputDir, "systemd/horizonos-automation.service")
            automationService.writeText("""
                [Unit]
                Description=HorizonOS Automation Service
                After=network.target
                
                [Service]
                Type=simple
                ExecStart=/usr/bin/horizonos-automation-engine
                Restart=always
                RestartSec=30
                User=horizonos-automation
                
                [Install]
                WantedBy=default.target
            """.trimIndent())
            generatedFiles.add(GeneratedFile("systemd/horizonos-automation.service", FileType.SYSTEMD))
        }
    }
}