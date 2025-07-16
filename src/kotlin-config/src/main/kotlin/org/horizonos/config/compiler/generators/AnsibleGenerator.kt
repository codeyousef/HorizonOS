package org.horizonos.config.compiler.generators

import org.horizonos.config.dsl.CompiledConfig
import org.horizonos.config.compiler.GeneratedFile
import org.horizonos.config.compiler.FileType
import java.io.File

/**
 * Ansible playbook generator for HorizonOS configurations
 */
class AnsibleGenerator(
    private val outputDir: File,
    private val generatedFiles: MutableList<GeneratedFile>
) {
    
    fun generate(config: CompiledConfig) {
        val playbook = File(outputDir, "ansible/horizonos-playbook.yml")
        playbook.writeText(buildString {
            appendLine("---")
            appendLine("# HorizonOS Ansible Playbook")
            appendLine("# Generated from Kotlin DSL")
            appendLine()
            appendLine("- name: Deploy HorizonOS Configuration")
            appendLine("  hosts: all")
            appendLine("  become: yes")
            appendLine("  tasks:")
            appendLine()
            appendLine("    - name: Set hostname")
            appendLine("      hostname:")
            appendLine("        name: ${config.system.hostname}")
            appendLine()
            appendLine("    - name: Set timezone")
            appendLine("      timezone:")
            appendLine("        name: ${config.system.timezone}")
            appendLine()
            
            if (config.packages.isNotEmpty()) {
                val installPackages = config.packages.filter { it.action == org.horizonos.config.dsl.PackageAction.INSTALL }
                if (installPackages.isNotEmpty()) {
                    appendLine("    - name: Install packages")
                    appendLine("      pacman:")
                    appendLine("        name:")
                    installPackages.forEach { pkg ->
                        appendLine("          - ${pkg.name}")
                    }
                    appendLine("        state: present")
                    appendLine()
                }
            }
            
            if (config.services.isNotEmpty()) {
                appendLine("    - name: Configure services")
                appendLine("      systemd:")
                appendLine("        name: \"{{ item.name }}\"")
                appendLine("        enabled: \"{{ item.enabled }}\"")
                appendLine("        state: \"{{ 'started' if item.enabled else 'stopped' }}\"")
                appendLine("      loop:")
                config.services.forEach { service ->
                    appendLine("        - { name: '${service.name}', enabled: ${service.enabled} }")
                }
            }
        })
        generatedFiles.add(GeneratedFile("ansible/horizonos-playbook.yml", FileType.ANSIBLE))
    }
}