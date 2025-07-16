package org.horizonos.config

import io.kotest.core.spec.style.StringSpec
import io.kotest.matchers.shouldBe
import io.kotest.matchers.shouldNotBe
import io.kotest.matchers.string.shouldContain
import io.kotest.matchers.string.shouldNotContain
import kotlinx.serialization.json.Json
import org.horizonos.config.dsl.*
import org.horizonos.config.compiler.*
import java.io.File
import java.nio.file.Files

class CompilerTest : StringSpec({

    "should generate JSON configuration" {
        val config = horizonOS {
            hostname = "test-system"
            packages {
                install("vim", "git")
            }
            services {
                enable("NetworkManager")
            }
        }
        
        val tempDir = Files.createTempDirectory("horizonos-test").toFile()
        val generator = EnhancedConfigGenerator(tempDir)
        generator.generate(config)
        
        val jsonFile = File(tempDir, "json/config.json")
        jsonFile.exists() shouldBe true
        
        val jsonContent = jsonFile.readText()
        jsonContent shouldContain "test-system"
        jsonContent shouldContain "vim"
        jsonContent shouldContain "git"
        jsonContent shouldContain "NetworkManager"
        
        // Verify it's valid JSON
        val json = Json { ignoreUnknownKeys = true }
        val parsedConfig = json.decodeFromString<CompiledConfig>(jsonContent)
        parsedConfig.system.hostname shouldBe "test-system"
        
        tempDir.deleteRecursively()
    }
    
    "should generate package installation script" {
        val config = horizonOS {
            packages {
                install("vim", "git", "docker")
                remove("nano")
            }
        }
        
        val tempDir = Files.createTempDirectory("horizonos-test").toFile()
        val generator = EnhancedConfigGenerator(tempDir)
        generator.generate(config)
        
        val scriptFile = File(tempDir, "scripts/package-manager.sh")
        scriptFile.exists() shouldBe true
        scriptFile.canExecute() shouldBe true
        
        val scriptContent = scriptFile.readText()
        scriptContent shouldContain "#!/bin/bash"
        scriptContent shouldContain "pacman -S --needed --noconfirm"
        scriptContent shouldContain "vim"
        scriptContent shouldContain "git"
        scriptContent shouldContain "docker"
        scriptContent shouldContain "pacman -R --noconfirm"
        scriptContent shouldContain "nano"
        
        tempDir.deleteRecursively()
    }
    
    "should generate service configuration script" {
        val config = horizonOS {
            services {
                enable("NetworkManager")
                enable("sshd")
                disable("bluetooth")
            }
        }
        
        val tempDir = Files.createTempDirectory("horizonos-test").toFile()
        val generator = EnhancedConfigGenerator(tempDir)
        generator.generate(config)
        
        val scriptFile = File(tempDir, "scripts/service-manager.sh")
        scriptFile.exists() shouldBe true
        scriptFile.canExecute() shouldBe true
        
        val scriptContent = scriptFile.readText()
        scriptContent shouldContain "#!/bin/bash"
        scriptContent shouldContain "systemctl enable NetworkManager"
        scriptContent shouldContain "systemctl enable sshd"
        scriptContent shouldContain "systemctl disable bluetooth"
        
        tempDir.deleteRecursively()
    }
    
    "should generate user creation script" {
        val config = horizonOS {
            users {
                user("admin") {
                    uid = 1000
                    shell = "/usr/bin/fish"
                    groups("wheel", "docker")
                }
                user("guest") {
                    shell = "/usr/bin/bash"
                }
            }
        }
        
        val tempDir = Files.createTempDirectory("horizonos-test").toFile()
        val generator = EnhancedConfigGenerator(tempDir)
        generator.generate(config)
        
        val scriptFile = File(tempDir, "scripts/user-manager.sh")
        scriptFile.exists() shouldBe true
        scriptFile.canExecute() shouldBe true
        
        val scriptContent = scriptFile.readText()
        scriptContent shouldContain "#!/bin/bash"
        scriptContent shouldContain "useradd -m -u 1000 -s /usr/bin/fish -G wheel,docker admin"
        scriptContent shouldContain "useradd -m -s /usr/bin/bash guest"
        
        tempDir.deleteRecursively()
    }
    
    "should generate repository configuration" {
        val config = horizonOS {
            repositories {
                add("core", "https://mirror.archlinux.org/core") {
                    gpgCheck = false
                }
                add("extra", "https://mirror.archlinux.org/extra") {
                    priority = 20
                }
                ostree("horizonos", "https://ostree.horizonos.org") {
                    branch("stable")
                }
            }
        }
        
        val tempDir = Files.createTempDirectory("horizonos-test").toFile()
        val generator = EnhancedConfigGenerator(tempDir)
        generator.generate(config)
        
        // The new generator includes repository configuration in the repository-config.sh script
        val repoScript = File(tempDir, "scripts/repository-config.sh")
        repoScript.exists() shouldBe true
        
        val repoContent = repoScript.readText()
        repoContent shouldContain "[core]"
        repoContent shouldContain "Server = https://mirror.archlinux.org/core"
        repoContent shouldContain "SigLevel = Never"
        repoContent shouldContain "[extra]"
        repoContent shouldContain "Server = https://mirror.archlinux.org/extra"
        repoContent shouldContain "ostree remote add"  // OSTree repos are handled separately
        
        tempDir.deleteRecursively()
    }
    
    "should generate OSTree deployment script" {
        val config = horizonOS {
            hostname = "test-system"
            timezone = "America/New_York"
            locale = "en_US.UTF-8"
        }
        
        val tempDir = Files.createTempDirectory("horizonos-test").toFile()
        val generator = EnhancedConfigGenerator(tempDir)
        generator.generate(config)
        
        val scriptFile = File(tempDir, "scripts/deploy.sh")
        scriptFile.exists() shouldBe true
        scriptFile.canExecute() shouldBe true
        
        val scriptContent = scriptFile.readText()
        scriptContent shouldContain "#!/bin/bash"
        scriptContent shouldContain "# Run all configuration scripts"
        scriptContent shouldContain "./system-config.sh"
        scriptContent shouldContain "./package-manager.sh"
        scriptContent shouldContain "./service-manager.sh"
        scriptContent shouldContain "./user-manager.sh"
        scriptContent shouldContain "./repository-config.sh"
        
        tempDir.deleteRecursively()
    }
    
    "should create proper directory structure" {
        val config = horizonOS {
            hostname = "test"
        }
        
        val tempDir = Files.createTempDirectory("horizonos-test").toFile()
        val generator = EnhancedConfigGenerator(tempDir)
        generator.generate(config)
        
        File(tempDir, "scripts").exists() shouldBe true
        File(tempDir, "systemd").exists() shouldBe true
        File(tempDir, "configs").exists() shouldBe true
        File(tempDir, "ostree").exists() shouldBe true
        
        tempDir.deleteRecursively()
    }
    
    "should handle empty configurations gracefully" {
        val config = horizonOS {
            hostname = "minimal"
        }
        
        val tempDir = Files.createTempDirectory("horizonos-test").toFile()
        val generator = EnhancedConfigGenerator(tempDir)
        generator.generate(config)
        
        // Should still generate basic files
        File(tempDir, "json/config.json").exists() shouldBe true
        File(tempDir, "scripts/deploy.sh").exists() shouldBe true
        
        // Package script should exist but be minimal
        val packageScript = File(tempDir, "scripts/package-manager.sh")
        packageScript.exists() shouldBe true
        val packageContent = packageScript.readText()
        packageContent shouldContain "#!/bin/bash"
        packageContent shouldNotContain "pacman -S"  // No packages to install
        
        tempDir.deleteRecursively()
    }
})