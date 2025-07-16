package org.horizonos.config.compiler.generators.scripts

import org.horizonos.config.dsl.*
import org.horizonos.config.compiler.GeneratedFile
import org.horizonos.config.compiler.FileType
import java.io.File

/**
 * Boot Script Generator
 * Generates shell scripts for boot system configuration including bootloader, kernel, and initramfs setup
 */
class BootScriptGenerator(
    private val outputDir: File,
    private val generatedFiles: MutableList<GeneratedFile>
) {
    
    fun generateBootScript(config: CompiledConfig) {
        config.boot?.let { boot ->
            val script = File(outputDir, "scripts/boot-config.sh")
            script.writeText(buildString {
                appendLine("#!/bin/bash")
                appendLine("# Boot Configuration")
                appendLine()
                appendLine("echo 'Configuring boot system...'")
                appendLine()
                
                // Bootloader configuration
                generateBootloaderConfig(boot)
                
                // Kernel configuration
                generateKernelConfig(boot)
                
                // Initramfs configuration
                generateInitramfsConfig(boot)
                
                // Plymouth configuration
                generatePlymouthConfig(boot)
                
                // Secure Boot configuration
                generateSecureBootConfig(boot)
                
                appendLine("echo 'Boot configuration completed.'")
            })
            script.setExecutable(true)
            generatedFiles.add(GeneratedFile("scripts/boot-config.sh", FileType.SHELL))
        }
    }
    
    private fun StringBuilder.generateBootloaderConfig(boot: BootConfig) {
        when (boot.bootloader.type) {
            BootloaderType.SYSTEMD_BOOT -> {
                appendLine("# Configure systemd-boot")
                appendLine("bootctl install || true")
                appendLine("mkdir -p /boot/loader/entries")
                appendLine()
                
                // Main loader configuration
                appendLine("cat > /boot/loader/loader.conf <<EOF")
                appendLine("default      ${boot.bootloader.defaultEntry ?: "horizonos"}")
                appendLine("timeout      ${boot.bootloader.timeout.inWholeSeconds}")
                appendLine("console-mode ${boot.bootloader.consoleMode.name.lowercase()}")
                appendLine("editor       ${if (boot.bootloader.editor) "yes" else "no"}")
                boot.bootloader.resolution?.let { appendLine("resolution   $it") }
                appendLine("EOF")
                appendLine()
                
                // Generate boot entries
                boot.bootloader.entries.forEach { entry ->
                    appendLine("# Boot entry: ${entry.title}")
                    appendLine("cat > /boot/loader/entries/${entry.title.lowercase().replace(" ", "-")}.conf <<EOF")
                    appendLine("title      ${entry.title}")
                    appendLine("linux      ${entry.linux}")
                    entry.initrd?.let { appendLine("initrd     $it") }
                    if (entry.options.isNotEmpty()) {
                        appendLine("options    ${entry.options.joinToString(" ")}")
                    }
                    entry.devicetree?.let { appendLine("devicetree $it") }
                    entry.architecture?.let { appendLine("architecture $it") }
                    entry.version?.let { appendLine("version    $it") }
                    entry.machineId?.let { appendLine("machine-id $it") }
                    appendLine("EOF")
                    appendLine()
                }
            }
            BootloaderType.GRUB -> {
                appendLine("# Configure GRUB")
                appendLine("grub-install --target=x86_64-efi --efi-directory=/boot --bootloader-id=HorizonOS")
                appendLine()
                
                boot.bootloader.grubConfig?.let { grub ->
                    appendLine("# Update GRUB configuration")
                    appendLine("sed -i 's/GRUB_DISTRIBUTOR=.*/GRUB_DISTRIBUTOR=\"${grub.distributor}\"/' /etc/default/grub")
                    appendLine("sed -i 's/GRUB_TIMEOUT=.*/GRUB_TIMEOUT=${grub.defaultTimeout.inWholeSeconds}/' /etc/default/grub")
                    grub.theme?.let { appendLine("echo 'GRUB_THEME=\"$it\"' >> /etc/default/grub") }
                    grub.background?.let { appendLine("echo 'GRUB_BACKGROUND=\"$it\"' >> /etc/default/grub") }
                    appendLine("echo 'GRUB_GFXMODE=${grub.gfxMode}' >> /etc/default/grub")
                    appendLine("echo 'GRUB_GFXPAYLOAD=${grub.gfxPayload}' >> /etc/default/grub")
                    if (!grub.recordFailCount) {
                        appendLine("echo 'GRUB_RECORDFAIL_TIMEOUT=0' >> /etc/default/grub")
                    }
                    if (grub.disableRecovery) {
                        appendLine("echo 'GRUB_DISABLE_RECOVERY=true' >> /etc/default/grub")
                    }
                    if (grub.disableOsProber) {
                        appendLine("echo 'GRUB_DISABLE_OS_PROBER=true' >> /etc/default/grub")
                    }
                }
                appendLine("grub-mkconfig -o /boot/grub/grub.cfg")
            }
            else -> {
                appendLine("# ${boot.bootloader.type} configuration not yet implemented")
            }
        }
        appendLine()
    }
    
    private fun StringBuilder.generateKernelConfig(boot: BootConfig) {
        if (boot.kernel.parameters.isNotEmpty()) {
            appendLine("# Kernel parameters")
            val kernelCmdline = boot.kernel.parameters.joinToString(" ") { param ->
                if (param.value != null) "${param.name}=${param.value}" else param.name
            }
            appendLine("echo 'Kernel command line: $kernelCmdline'")
            
            when (boot.bootloader.type) {
                BootloaderType.SYSTEMD_BOOT -> {
                    appendLine("# Update systemd-boot entries with kernel parameters")
                    appendLine("find /boot/loader/entries -name '*.conf' -exec sed -i 's/^options.*/options $kernelCmdline/' {} \\;")
                }
                BootloaderType.GRUB -> {
                    appendLine("# Update GRUB with kernel parameters")
                    appendLine("sed -i 's/GRUB_CMDLINE_LINUX_DEFAULT=.*/GRUB_CMDLINE_LINUX_DEFAULT=\"$kernelCmdline\"/' /etc/default/grub")
                    appendLine("grub-mkconfig -o /boot/grub/grub.cfg")
                }
                else -> {}
            }
            appendLine()
        }
        
        // Kernel module configuration
        if (boot.kernel.modules.blacklist.isNotEmpty()) {
            appendLine("# Blacklist kernel modules")
            appendLine("cat > /etc/modprobe.d/horizonos-blacklist.conf <<EOF")
            boot.kernel.modules.blacklist.forEach { module ->
                appendLine("blacklist $module")
            }
            appendLine("EOF")
            appendLine()
        }
        
        if (boot.kernel.modules.load.isNotEmpty()) {
            appendLine("# Load kernel modules")
            appendLine("cat > /etc/modules-load.d/horizonos.conf <<EOF")
            boot.kernel.modules.load.forEach { module ->
                appendLine(module)
            }
            appendLine("EOF")
            appendLine()
        }
        
        if (boot.kernel.modules.options.isNotEmpty()) {
            appendLine("# Module options")
            appendLine("cat > /etc/modprobe.d/horizonos-options.conf <<EOF")
            boot.kernel.modules.options.forEach { (module, options) ->
                appendLine("options $module $options")
            }
            appendLine("EOF")
            appendLine()
        }
    }
    
    private fun StringBuilder.generateInitramfsConfig(boot: BootConfig) {
        when (boot.initramfs.generator) {
            InitramfsGenerator.MKINITCPIO -> {
                appendLine("# Configure mkinitcpio")
                if (boot.initramfs.modules.isNotEmpty() || boot.initramfs.hooks.isNotEmpty()) {
                    appendLine("cp /etc/mkinitcpio.conf /etc/mkinitcpio.conf.backup")
                    
                    if (boot.initramfs.modules.isNotEmpty()) {
                        val modules = boot.initramfs.modules.joinToString(" ")
                        appendLine("sed -i 's/MODULES=(.*)/MODULES=($modules)/' /etc/mkinitcpio.conf")
                    }
                    
                    if (boot.initramfs.hooks.isNotEmpty()) {
                        val hooks = boot.initramfs.hooks.joinToString(" ")
                        appendLine("sed -i 's/HOOKS=(.*)/HOOKS=($hooks)/' /etc/mkinitcpio.conf")
                    }
                    
                    appendLine("mkinitcpio -P")
                }
            }
            InitramfsGenerator.DRACUT -> {
                appendLine("# Configure dracut")
                appendLine("mkdir -p /etc/dracut.conf.d")
                appendLine("cat > /etc/dracut.conf.d/horizonos.conf <<EOF")
                appendLine("compress=\"${boot.initramfs.compression.name.lowercase()}\"")
                if (boot.initramfs.modules.isNotEmpty()) {
                    appendLine("add_dracutmodules+=\" ${boot.initramfs.modules.joinToString(" ")} \"")
                }
                appendLine("EOF")
                appendLine("dracut --force")
            }
            else -> {
                appendLine("# ${boot.initramfs.generator} configuration not yet implemented")
            }
        }
        appendLine()
    }
    
    private fun StringBuilder.generatePlymouthConfig(boot: BootConfig) {
        if (boot.plymouth.enabled) {
            appendLine("# Configure Plymouth")
            appendLine("plymouth-set-default-theme ${boot.plymouth.theme}")
            if (boot.plymouth.modules.isNotEmpty()) {
                appendLine("echo 'plymouth.modules=${boot.plymouth.modules.joinToString(",")}' >> /etc/kernel/cmdline")
            }
            appendLine("mkinitcpio -P")
            appendLine()
        }
    }
    
    private fun StringBuilder.generateSecureBootConfig(boot: BootConfig) {
        if (boot.secureBoot.enabled) {
            appendLine("# Configure Secure Boot")
            appendLine("echo 'Secure Boot configuration requires manual key enrollment'")
            appendLine("echo 'Please refer to the documentation for detailed instructions'")
            boot.secureBoot.keys?.let { keys ->
                keys.platform?.let { appendLine("# Platform key: $it") }
                keys.keyExchange?.let { appendLine("# Key exchange key: $it") }
                keys.signature?.let { appendLine("# Signature database: $it") }
            }
            appendLine()
        }
    }
}