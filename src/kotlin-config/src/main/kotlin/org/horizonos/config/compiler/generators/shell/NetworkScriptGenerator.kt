package org.horizonos.config.compiler.generators.shell

import org.horizonos.config.dsl.CompiledConfig
import org.horizonos.config.compiler.GeneratedFile
import org.horizonos.config.compiler.FileType
import java.io.File

/**
 * Network script generator for network configuration
 */
class NetworkScriptGenerator(
    private val outputDir: File,
    private val generatedFiles: MutableList<GeneratedFile>
) {
    
    fun generateNetworkScript(config: CompiledConfig) {
        config.network?.let { network ->
            val script = File(outputDir, "scripts/network-config.sh")
            script.writeText(buildString {
                appendLine("#!/bin/bash")
                appendLine("# Network Configuration")
                appendLine("# Generated from HorizonOS Kotlin DSL")
                appendLine()
                appendLine("echo 'Setting up network configuration...'")
                appendLine()
                
                // Network manager setup
                appendLine("# Network Manager: ${network.networkManager}")
                when (network.networkManager) {
                    org.horizonos.config.dsl.NetworkManagerType.NETWORKMANAGER -> {
                        appendLine("systemctl enable NetworkManager")
                        appendLine("systemctl start NetworkManager")
                    }
                    org.horizonos.config.dsl.NetworkManagerType.SYSTEMD_NETWORKD -> {
                        appendLine("systemctl enable systemd-networkd")
                        appendLine("systemctl start systemd-networkd")
                    }
                    else -> {
                        appendLine("# Custom network manager configuration")
                    }
                }
                
                // Network interfaces
                if (network.interfaces.isNotEmpty()) {
                    appendLine()
                    appendLine("# Network interfaces")
                    network.interfaces.forEach { iface ->
                        appendLine("echo 'Configuring interface: ${iface.name}'")
                        appendLine("# Interface ${iface.name} configuration")
                    }
                }
                
                // WiFi networks
                if (network.wifiNetworks.isNotEmpty()) {
                    appendLine()
                    appendLine("# WiFi networks")
                    network.wifiNetworks.forEach { wifi ->
                        appendLine("echo 'Configuring WiFi: ${wifi.ssid}'")
                        appendLine("# WiFi ${wifi.ssid} configuration")
                    }
                }
                
                // VPN connections
                if (network.vpnConnections.isNotEmpty()) {
                    appendLine()
                    appendLine("# VPN connections")
                    network.vpnConnections.forEach { vpn ->
                        appendLine("echo 'Configuring VPN: ${vpn.name}'")
                        appendLine("# VPN ${vpn.name} configuration")
                    }
                }
                
                // Firewall configuration
                if (network.firewall.enabled) {
                    appendLine()
                    appendLine("# Firewall configuration")
                    appendLine("systemctl enable firewalld")
                    appendLine("systemctl start firewalld")
                }
                
                appendLine()
                appendLine("echo 'Network configuration completed.'")
            })
            script.setExecutable(true)
            generatedFiles.add(GeneratedFile("scripts/network-config.sh", FileType.SHELL))
        }
    }
}