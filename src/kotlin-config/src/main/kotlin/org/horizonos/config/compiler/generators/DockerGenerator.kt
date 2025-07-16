package org.horizonos.config.compiler.generators

import org.horizonos.config.dsl.CompiledConfig
import org.horizonos.config.compiler.GeneratedFile
import org.horizonos.config.compiler.FileType
import java.io.File

/**
 * Docker generator for containerized HorizonOS configurations
 */
class DockerGenerator(
    private val outputDir: File,
    private val generatedFiles: MutableList<GeneratedFile>
) {
    
    fun generate(config: CompiledConfig) {
        val dockerfile = File(outputDir, "docker/Dockerfile")
        dockerfile.writeText(buildString {
            appendLine("# HorizonOS Configuration Container")
            appendLine("# Generated from Kotlin DSL")
            appendLine()
            appendLine("FROM archlinux:latest")
            appendLine()
            appendLine("# Install base packages")
            appendLine("RUN pacman -Syu --noconfirm")
            appendLine("RUN pacman -S --needed --noconfirm base-devel git")
            appendLine()
            appendLine("# Copy configuration files")
            appendLine("COPY scripts/ /opt/horizonos/scripts/")
            appendLine("COPY json/ /opt/horizonos/config/")
            appendLine()
            appendLine("# Set working directory")
            appendLine("WORKDIR /opt/horizonos")
            appendLine()
            appendLine("# Run configuration")
            appendLine("CMD [\"./scripts/deploy.sh\"]")
        })
        generatedFiles.add(GeneratedFile("docker/Dockerfile", FileType.DOCKER))
    }
}