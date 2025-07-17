package org.horizonos.config.dsl

import org.junit.jupiter.api.Test
import org.junit.jupiter.api.assertDoesNotThrow
import org.junit.jupiter.api.assertThrows
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertTrue

class LayersTest {
    
    @Test
    fun `layers DSL should create layered architecture`() {
        val config = horizonOS {
            layers {
                base {
                    image = "horizonos/base"
                    tag = "stable"
                    packages("base", "linux", "systemd")
                    services("systemd-networkd", "systemd-resolved")
                    commit("stable-commit-hash")
                }
                
                systemLayer("development", LayerPurpose.DEVELOPMENT) {
                    priority = 10
                    autoStart = true
                    
                    development {
                        packages("git", "curl", "vim")
                        export("git", "curl", "vim")
                    }
                }
                
                user {
                    flatpak("org.mozilla.firefox")
                    flatpak("org.gnome.gedit")
                }
            }
        }
        
        assertNotNull(config.layers)
        
        val layers = config.layers!!
        assertEquals("horizonos/base", layers.base.image)
        assertEquals("stable", layers.base.tag)
        assertEquals(1, layers.system.size)
        assertEquals(2, layers.user.flatpaks.size)
        
        val systemLayer = layers.system[0]
        assertEquals("development", systemLayer.name)
        assertEquals(LayerPurpose.DEVELOPMENT, systemLayer.purpose)
        assertEquals(10, systemLayer.priority)
        assertEquals(true, systemLayer.autoStart)
    }
    
    @Test
    fun `system layer should support dependencies`() {
        val config = horizonOS {
            layers {
                base {
                    packages("base", "linux")
                }
                
                systemLayer("base-tools", LayerPurpose.DEVELOPMENT) {
                    priority = 10
                    container {
                        image = "archlinux/archlinux"
                        packages("git", "curl")
                    }
                }
                
                systemLayer("advanced-tools", LayerPurpose.DEVELOPMENT) {
                    priority = 20
                    dependsOn("base-tools")
                    container {
                        image = "archlinux/archlinux"
                        packages("rust", "go")
                    }
                }
            }
        }
        
        assertNotNull(config.layers)
        val layers = config.layers!!
        assertEquals(2, layers.system.size)
        
        val advancedLayer = layers.system.find { it.name == "advanced-tools" }
        assertNotNull(advancedLayer)
        assertEquals(1, advancedLayer.dependencies.size)
        assertEquals("base-tools", advancedLayer.dependencies[0])
    }
    
    @Test
    fun `layer health check should be configurable`() {
        val config = horizonOS {
            layers {
                base {
                    packages("base")
                }
                
                systemLayer("web-server", LayerPurpose.CUSTOM) {
                    container {
                        image = "nginx"
                        port("80:80")
                    }
                    
                    healthCheck {
                        command = "curl -f http://localhost:80/health"
                        interval = "30s"
                        timeout = "5s"
                        retries = 3
                    }
                }
            }
        }
        
        assertNotNull(config.layers)
        val systemLayer = config.layers!!.system[0]
        assertNotNull(systemLayer.healthCheck)
        
        val healthCheck = systemLayer.healthCheck!!
        assertEquals("curl -f http://localhost:80/health", healthCheck.command)
        assertEquals("30s", healthCheck.interval)
        assertEquals("5s", healthCheck.timeout)
        assertEquals(3, healthCheck.retries)
    }
    
    @Test
    fun `layer order should be configurable`() {
        val config = horizonOS {
            layers {
                base {
                    packages("base")
                }
                
                systemLayer("layer1", LayerPurpose.DEVELOPMENT) {
                    container {
                        image = "archlinux/archlinux"
                        packages("git")
                    }
                }
                
                systemLayer("layer2", LayerPurpose.DEVELOPMENT) {
                    container {
                        image = "archlinux/archlinux"
                        packages("curl")
                    }
                }
                
                user {
                    flatpak("org.mozilla.firefox")
                }
                
                order("base", "layer1", "layer2", "user")
            }
        }
        
        assertNotNull(config.layers)
        val layers = config.layers!!
        assertEquals(4, layers.layerOrder.size)
        assertEquals("base", layers.layerOrder[0])
        assertEquals("layer1", layers.layerOrder[1])
        assertEquals("layer2", layers.layerOrder[2])
        assertEquals("user", layers.layerOrder[3])
    }
    
    @Test
    fun `user layer should support multiple package formats`() {
        val config = horizonOS {
            layers {
                base {
                    packages("base")
                }
                
                user {
                    flatpak("org.mozilla.firefox")
                    flatpak("org.gnome.gedit")
                    appImage("example", "https://example.com/app.AppImage", "sha256:checksum")
                    snap("discord", "stable", classic = true)
                }
            }
        }
        
        assertNotNull(config.layers)
        val userLayer = config.layers!!.user
        assertEquals(2, userLayer.flatpaks.size)
        assertEquals(1, userLayer.appImages.size)
        assertEquals(1, userLayer.snaps.size)
        
        val appImage = userLayer.appImages[0]
        assertEquals("example", appImage.name)
        assertEquals("https://example.com/app.AppImage", appImage.url)
        assertEquals("sha256:checksum", appImage.checksum)
        
        val snap = userLayer.snaps[0]
        assertEquals("discord", snap.name)
        assertEquals("stable", snap.channel)
        assertEquals(true, snap.classic)
    }
    
    @Test
    fun `global mounts should be applied to all layers`() {
        val config = horizonOS {
            layers {
                base {
                    packages("base")
                }
                
                globalMount("/shared/data")
                globalMount("/shared/config")
                
                sharedVolume("workspace")
                sharedVolume("cache")
            }
        }
        
        assertNotNull(config.layers)
        val layers = config.layers!!
        assertEquals(2, layers.globalMounts.size)
        assertEquals(2, layers.sharedVolumes.size)
        assertTrue(layers.globalMounts.contains("/shared/data"))
        assertTrue(layers.globalMounts.contains("/shared/config"))
        assertTrue(layers.sharedVolumes.contains("workspace"))
        assertTrue(layers.sharedVolumes.contains("cache"))
    }
    
    @Test
    fun `layer purpose should convert to container purpose`() {
        assertEquals(ContainerPurpose.DEVELOPMENT, LayerPurpose.DEVELOPMENT.toContainerPurpose())
        assertEquals(ContainerPurpose.GAMING, LayerPurpose.GAMING.toContainerPurpose())
        assertEquals(ContainerPurpose.MULTIMEDIA, LayerPurpose.MULTIMEDIA.toContainerPurpose())
        assertEquals(ContainerPurpose.OFFICE, LayerPurpose.OFFICE.toContainerPurpose())
        assertEquals(ContainerPurpose.SECURITY, LayerPurpose.SECURITY.toContainerPurpose())
        assertEquals(ContainerPurpose.CUSTOM, LayerPurpose.NETWORKING.toContainerPurpose())
        assertEquals(ContainerPurpose.CUSTOM, LayerPurpose.CUSTOM.toContainerPurpose())
    }
    
    @Test
    fun `default layer packages should match purpose`() {
        val devPackages = getDefaultLayerPackages(LayerPurpose.DEVELOPMENT)
        assertTrue(devPackages.contains("git"))
        assertTrue(devPackages.contains("curl"))
        assertTrue(devPackages.contains("build-essential"))
        
        val gamingPackages = getDefaultLayerPackages(LayerPurpose.GAMING)
        assertTrue(gamingPackages.contains("steam"))
        assertTrue(gamingPackages.contains("lutris"))
        
        val multimediaPackages = getDefaultLayerPackages(LayerPurpose.MULTIMEDIA)
        assertTrue(multimediaPackages.contains("ffmpeg"))
        assertTrue(multimediaPackages.contains("imagemagick"))
    }
    
    @Test
    fun `default flatpaks should match purpose`() {
        val devFlatpaks = getDefaultFlatpaks(LayerPurpose.DEVELOPMENT)
        assertTrue(devFlatpaks.contains("com.visualstudio.code"))
        assertTrue(devFlatpaks.contains("org.gnome.Builder"))
        
        val gamingFlatpaks = getDefaultFlatpaks(LayerPurpose.GAMING)
        assertTrue(gamingFlatpaks.contains("com.valvesoftware.Steam"))
        assertTrue(gamingFlatpaks.contains("net.lutris.Lutris"))
        
        val officeFlatpaks = getDefaultFlatpaks(LayerPurpose.OFFICE)
        assertTrue(officeFlatpaks.contains("org.libreoffice.LibreOffice"))
        assertTrue(officeFlatpaks.contains("org.mozilla.firefox"))
    }
    
    @Test
    fun `layer dependency validation should detect circular dependencies`() {
        val layers = listOf(
            SystemLayer("layer1", LayerPurpose.DEVELOPMENT, 
                       SystemContainer("c1", "img1"),
                       dependencies = listOf("layer2")),
            SystemLayer("layer2", LayerPurpose.DEVELOPMENT, 
                       SystemContainer("c2", "img2"),
                       dependencies = listOf("layer1"))
        )
        
        assertThrows<IllegalStateException> {
            sortLayers(layers)
        }
    }
    
    @Test
    fun `layer dependency validation should detect missing dependencies`() {
        val layers = listOf(
            SystemLayer("layer1", LayerPurpose.DEVELOPMENT, 
                       SystemContainer("c1", "img1"),
                       dependencies = listOf("non-existent"))
        )
        
        val errors = validateLayerDependencies(layers)
        assertEquals(1, errors.size)
        assertTrue(errors[0].contains("depends on non-existent layer"))
    }
    
    @Test
    fun `layer sorting should respect dependencies and priority`() {
        val layers = listOf(
            SystemLayer("layer3", LayerPurpose.DEVELOPMENT, 
                       SystemContainer("c3", "img3"), 
                       dependencies = listOf("layer1", "layer2"),
                       priority = 30),
            SystemLayer("layer1", LayerPurpose.DEVELOPMENT, 
                       SystemContainer("c1", "img1"), 
                       priority = 10),
            SystemLayer("layer2", LayerPurpose.DEVELOPMENT, 
                       SystemContainer("c2", "img2"), 
                       dependencies = listOf("layer1"),
                       priority = 20)
        )
        
        val sorted = sortLayers(layers)
        assertEquals(3, sorted.size)
        assertEquals("layer1", sorted[0].name)
        assertEquals("layer2", sorted[1].name)
        assertEquals("layer3", sorted[2].name)
    }
}