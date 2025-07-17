package org.horizonos.config.dsl

import org.junit.jupiter.api.Test
import org.junit.jupiter.api.assertDoesNotThrow
import org.junit.jupiter.api.assertThrows
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertTrue

class PackagesTest {
    
    @Test
    fun `enhanced packages DSL should create package configuration`() {
        val config = horizonOS {
            enhancedPackages {
                autoMigrate = true
                migrationStrategy = MigrationStrategy.CONTAINER_FIRST
                
                system {
                    strategy = SystemPackageStrategy.CONTAINER
                    defaultRuntime = ContainerRuntime.DISTROBOX
                    
                    development("dev-tools") {
                        packages("git", "curl", "vim")
                        export("git", "curl", "vim")
                    }
                    
                    multimedia("media-tools") {
                        packages("ffmpeg", "imagemagick")
                        export("ffmpeg", "convert")
                    }
                    
                    globalMount("/home/user/projects")
                    autoUpdate = true
                }
                
                applications {
                    strategy = ApplicationPackageStrategy.FLATPAK
                    autoUpdate = true
                    
                    flatpak("org.mozilla.firefox")
                    flatpak("org.gnome.gedit") {
                        branch = "stable"
                        allowFilesystem()
                        allowDisplay()
                    }
                    
                    appImage("example", "https://example.com/app.AppImage")
                    snap("discord", "stable", classic = true)
                }
            }
        }
        
        assertNotNull(config.enhancedPackages)
        val packages = config.enhancedPackages!!
        
        assertTrue(packages.autoMigrate)
        assertEquals(MigrationStrategy.CONTAINER_FIRST, packages.migrationStrategy)
        
        // Check system packages
        assertEquals(SystemPackageStrategy.CONTAINER, packages.system.strategy)
        assertEquals(ContainerRuntime.DISTROBOX, packages.system.defaultRuntime)
        assertEquals(2, packages.system.containers.size)
        assertEquals(1, packages.system.globalMounts.size)
        assertTrue(packages.system.autoUpdate)
        
        // Check applications
        assertEquals(ApplicationPackageStrategy.FLATPAK, packages.applications.strategy)
        assertTrue(packages.applications.autoUpdate)
        assertEquals(2, packages.applications.flatpaks.size)
        assertEquals(1, packages.applications.appImages.size)
        assertEquals(1, packages.applications.snaps.size)
    }
    
    @Test
    fun `flatpak configuration should support permissions`() {
        val config = horizonOS {
            enhancedPackages {
                applications {
                    flatpak("org.mozilla.firefox") {
                        branch = "stable"
                        userInstall = true
                        autoUpdate = true
                        
                        permission("--share=network")
                        permission("--filesystem=home")
                        allowNetwork()
                        allowFilesystem()
                        allowDisplay()
                        allowAudio()
                    }
                }
            }
        }
        
        assertNotNull(config.enhancedPackages)
        val flatpak = config.enhancedPackages!!.applications.flatpaks[0]
        
        assertEquals("org.mozilla.firefox", flatpak.id)
        assertEquals("stable", flatpak.branch)
        assertTrue(flatpak.userInstall)
        assertTrue(flatpak.autoUpdate)
        assertTrue(flatpak.permissions.contains("--share=network"))
        assertTrue(flatpak.permissions.contains("--filesystem=home"))
        assertTrue(flatpak.permissions.contains("--share=ipc"))
        assertTrue(flatpak.permissions.contains("--socket=x11"))
        assertTrue(flatpak.permissions.contains("--socket=wayland"))
        assertTrue(flatpak.permissions.contains("--socket=pulseaudio"))
    }
    
    @Test
    fun `quick install should choose appropriate format`() {
        val config = horizonOS {
            enhancedPackages {
                quickInstall("firefox", "git", "steam")
            }
        }
        
        assertNotNull(config.enhancedPackages)
        val packages = config.enhancedPackages!!
        
        // Firefox should be installed as Flatpak
        assertTrue(packages.applications.flatpaks.any { it.id == "org.mozilla.firefox" })
        
        // Steam should be installed as Flatpak
        assertTrue(packages.applications.flatpaks.any { it.id == "com.valvesoftware.Steam" })
        
        // Git should fall back to legacy packages (would be migrated to container)
        assertTrue(packages.legacy.packages.any { it.name == "git" })
    }
    
    @Test
    fun `popular applications should be available`() {
        val config = horizonOS {
            enhancedPackages {
                applications {
                    popular("firefox", "thunderbird", "libreoffice")
                }
            }
        }
        
        assertNotNull(config.enhancedPackages)
        val flatpaks = config.enhancedPackages!!.applications.flatpaks
        
        assertTrue(flatpaks.any { it.id == "org.mozilla.firefox" })
        assertTrue(flatpaks.any { it.id == "org.mozilla.Thunderbird" })
        assertTrue(flatpaks.any { it.id == "org.libreoffice.LibreOffice" })
    }
    
    @Test
    fun `application suites should be available`() {
        val config = horizonOS {
            enhancedPackages {
                applications {
                    office()
                    development()
                    multimedia()
                    gaming()
                }
            }
        }
        
        assertNotNull(config.enhancedPackages)
        val flatpaks = config.enhancedPackages!!.applications.flatpaks
        
        // Office suite
        assertTrue(flatpaks.any { it.id == "org.libreoffice.LibreOffice" })
        assertTrue(flatpaks.any { it.id == "org.mozilla.Thunderbird" })
        
        // Development suite
        assertTrue(flatpaks.any { it.id == "com.visualstudio.code" })
        assertTrue(flatpaks.any { it.id == "org.gnome.Builder" })
        
        // Multimedia suite
        assertTrue(flatpaks.any { it.id == "org.gimp.GIMP" })
        assertTrue(flatpaks.any { it.id == "org.audacityteam.Audacity" })
        
        // Gaming suite
        assertTrue(flatpaks.any { it.id == "com.valvesoftware.Steam" })
        assertTrue(flatpaks.any { it.id == "net.lutris.Lutris" })
    }
    
    @Test
    fun `system containers should have purpose-specific configurations`() {
        val config = horizonOS {
            enhancedPackages {
                system {
                    development("dev-tools") {
                        packages("git", "curl", "vim", "build-essential")
                        export("git", "curl", "vim", "gcc", "make")
                    }
                    
                    multimedia("media-tools") {
                        packages("ffmpeg", "imagemagick", "sox", "mediainfo")
                        export("ffmpeg", "convert", "sox", "mediainfo")
                    }
                    
                    gaming("gaming-tools") {
                        packages("steam", "lutris", "wine", "gamemode")
                        export("steam", "lutris", "wine")
                    }
                }
            }
        }
        
        assertNotNull(config.enhancedPackages)
        val containers = config.enhancedPackages!!.system.containers
        assertEquals(3, containers.size)
        
        val devContainer = containers.find { it.purpose == ContainerPurpose.DEVELOPMENT }
        assertNotNull(devContainer)
        assertEquals("dev-tools", devContainer.name)
        assertTrue(devContainer.packages.contains("git"))
        assertTrue(devContainer.binaries.contains("git"))
        
        val mediaContainer = containers.find { it.purpose == ContainerPurpose.MULTIMEDIA }
        assertNotNull(mediaContainer)
        assertEquals("media-tools", mediaContainer.name)
        assertTrue(mediaContainer.packages.contains("ffmpeg"))
        assertTrue(mediaContainer.binaries.contains("ffmpeg"))
        
        val gamingContainer = containers.find { it.purpose == ContainerPurpose.GAMING }
        assertNotNull(gamingContainer)
        assertEquals("gaming-tools", gamingContainer.name)
        assertTrue(gamingContainer.packages.contains("steam"))
        assertTrue(gamingContainer.binaries.contains("steam"))
    }
    
    @Test
    fun `package format recommendations should be accurate`() {
        assertEquals(PackageFormat.FLATPAK, getRecommendedFormat("firefox"))
        assertEquals(PackageFormat.FLATPAK, getRecommendedFormat("chromium"))
        assertEquals(PackageFormat.FLATPAK, getRecommendedFormat("steam"))
        assertEquals(PackageFormat.FLATPAK, getRecommendedFormat("discord"))
        
        assertEquals(PackageFormat.CONTAINER, getRecommendedFormat("git"))
        assertEquals(PackageFormat.CONTAINER, getRecommendedFormat("curl"))
        assertEquals(PackageFormat.CONTAINER, getRecommendedFormat("python3"))
        assertEquals(PackageFormat.CONTAINER, getRecommendedFormat("nodejs"))
        
        assertEquals(PackageFormat.NATIVE, getRecommendedFormat("unknown-package"))
    }
    
    @Test
    fun `flatpak ID mapping should be correct`() {
        assertEquals("org.mozilla.firefox", getFlatpakId("firefox"))
        assertEquals("org.chromium.Chromium", getFlatpakId("chromium"))
        assertEquals("org.mozilla.Thunderbird", getFlatpakId("thunderbird"))
        assertEquals("org.libreoffice.LibreOffice", getFlatpakId("libreoffice"))
        assertEquals("org.gimp.GIMP", getFlatpakId("gimp"))
        assertEquals("com.valvesoftware.Steam", getFlatpakId("steam"))
        assertEquals("com.discordapp.Discord", getFlatpakId("discord"))
        assertEquals("com.visualstudio.code", getFlatpakId("code"))
        assertEquals("com.visualstudio.code", getFlatpakId("vscode"))
        assertEquals(null, getFlatpakId("unknown-package"))
    }
    
    @Test
    fun `container recommendations should be correct`() {
        val gitRec = getContainerForPackage("git")
        assertNotNull(gitRec)
        assertEquals("dev-tools", gitRec.container)
        assertEquals("archlinux/archlinux", gitRec.image)
        assertTrue(gitRec.packages.contains("git"))
        
        val pythonRec = getContainerForPackage("python3")
        assertNotNull(pythonRec)
        assertEquals("python-dev", pythonRec.container)
        assertEquals("python:3.11", pythonRec.image)
        assertTrue(pythonRec.packages.contains("python3"))
        
        val nodeRec = getContainerForPackage("nodejs")
        assertNotNull(nodeRec)
        assertEquals("node-dev", nodeRec.container)
        assertEquals("node:18", nodeRec.image)
        assertTrue(nodeRec.packages.contains("nodejs"))
        
        val unknownRec = getContainerForPackage("unknown-package")
        assertEquals(null, unknownRec)
    }
    
    @Test
    fun `package migration should work correctly`() {
        val legacyPackages = listOf(
            Package("firefox", PackageAction.INSTALL),
            Package("git", PackageAction.INSTALL),
            Package("steam", PackageAction.INSTALL),
            Package("unknown-package", PackageAction.INSTALL)
        )
        
        val migrationResult = migratePackages(legacyPackages, MigrationStrategy.CONTAINER_FIRST)
        
        // Git should be migrated to container
        assertTrue(migrationResult.containers.containsKey("dev-tools"))
        assertTrue(migrationResult.containers["dev-tools"]!!.contains("git"))
        
        // Firefox and Steam should be migrated to Flatpak
        assertTrue(migrationResult.flatpaks.any { it.id == "org.mozilla.firefox" })
        assertTrue(migrationResult.flatpaks.any { it.id == "com.valvesoftware.Steam" })
        
        // Unknown package should remain unmigrated
        assertTrue(migrationResult.unmigrated.any { it.name == "unknown-package" })
    }
    
    @Test
    fun `migration strategy should affect package placement`() {
        val legacyPackages = listOf(
            Package("firefox", PackageAction.INSTALL),
            Package("git", PackageAction.INSTALL)
        )
        
        val containerFirstResult = migratePackages(legacyPackages, MigrationStrategy.CONTAINER_FIRST)
        val flatpakFirstResult = migratePackages(legacyPackages, MigrationStrategy.FLATPAK_FIRST)
        
        // Container first should prioritize containers for git
        assertTrue(containerFirstResult.containers.containsKey("dev-tools"))
        assertTrue(containerFirstResult.flatpaks.any { it.id == "org.mozilla.firefox" })
        
        // Flatpak first should prioritize flatpaks for firefox
        assertTrue(flatpakFirstResult.flatpaks.any { it.id == "org.mozilla.firefox" })
        assertTrue(flatpakFirstResult.containers.containsKey("dev-tools"))
    }
    
    @Test
    fun `legacy package compatibility should be maintained`() {
        val config = horizonOS {
            packages {
                install("git", "curl", "vim")
                remove("unwanted-package")
                
                group("development") {
                    install("gcc", "make", "cmake")
                }
            }
        }
        
        assertEquals(7, config.packages.size)
        
        val installPackages = config.packages.filter { it.action == PackageAction.INSTALL }
        val removePackages = config.packages.filter { it.action == PackageAction.REMOVE }
        val groupPackages = config.packages.filter { it.group == "development" }
        
        assertEquals(6, installPackages.size)
        assertEquals(1, removePackages.size)
        assertEquals(3, groupPackages.size)
        
        assertTrue(installPackages.any { it.name == "git" })
        assertTrue(installPackages.any { it.name == "curl" })
        assertTrue(installPackages.any { it.name == "vim" })
        assertTrue(removePackages.any { it.name == "unwanted-package" })
        assertTrue(groupPackages.any { it.name == "gcc" })
    }
}