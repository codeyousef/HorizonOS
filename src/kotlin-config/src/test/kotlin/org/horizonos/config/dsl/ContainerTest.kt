package org.horizonos.config.dsl

import org.junit.jupiter.api.Test
import org.junit.jupiter.api.assertDoesNotThrow
import org.junit.jupiter.api.assertThrows
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertTrue

class ContainerTest {
    
    @Test
    fun `container DSL should create basic container configuration`() {
        val config = horizonOS {
            containers {
                container("test-container") {
                    image = "archlinux/archlinux"
                    tag = "latest"
                    runtime = ContainerRuntime.PODMAN
                    purpose = ContainerPurpose.DEVELOPMENT
                    packages("git", "curl", "vim")
                    export("git", "curl", "vim")
                    mount("/home/user/projects")
                    env("TEST_VAR", "test_value")
                    port("8080:8080")
                    label("app", "test")
                }
            }
        }
        
        assertNotNull(config.containers)
        assertEquals(1, config.containers!!.containers.size)
        
        val container = config.containers!!.containers[0]
        assertEquals("test-container", container.name)
        assertEquals("archlinux/archlinux", container.image)
        assertEquals("latest", container.tag)
        assertEquals(ContainerRuntime.PODMAN, container.runtime)
        assertEquals(ContainerPurpose.DEVELOPMENT, container.purpose)
        assertEquals(3, container.packages.size)
        assertEquals(3, container.binaries.size)
        assertEquals(1, container.persistent.size)
        assertEquals(1, container.environment.size)
        assertEquals(1, container.ports.size)
        assertEquals(1, container.labels.size)
    }
    
    @Test
    fun `dev container should have predefined configurations`() {
        val config = horizonOS {
            containers {
                devContainer("rust-dev") {
                    rust("1.70")
                    export("rustc", "cargo")
                    mount("/home/user/projects")
                }
            }
        }
        
        assertNotNull(config.containers)
        val container = config.containers!!.containers[0]
        assertEquals("rust-dev", container.name)
        assertEquals(ContainerPurpose.DEVELOPMENT, container.purpose)
        assertTrue(container.binaries.contains("rustc"))
        assertTrue(container.binaries.contains("cargo"))
    }
    
    @Test
    fun `distrobox container should use correct runtime`() {
        val config = horizonOS {
            containers {
                distrobox("arch-box") {
                    archlinux()
                    packages("git", "vim")
                    export("git", "vim")
                }
            }
        }
        
        assertNotNull(config.containers)
        val container = config.containers!!.containers[0]
        assertEquals("arch-box", container.name)
        assertEquals(ContainerRuntime.DISTROBOX, container.runtime)
        assertEquals("docker.io/archlinux/archlinux", container.image)
        assertEquals("latest", container.tag)
    }
    
    @Test
    fun `toolbox container should use correct runtime`() {
        val config = horizonOS {
            containers {
                toolbox("fedora-toolbox") {
                    fedora("38")
                    packages("git", "gcc")
                    export("git", "gcc")
                }
            }
        }
        
        assertNotNull(config.containers)
        val container = config.containers!!.containers[0]
        assertEquals("fedora-toolbox", container.name)
        assertEquals(ContainerRuntime.TOOLBOX, container.runtime)
        assertEquals("registry.fedoraproject.org/fedora-toolbox", container.image)
        assertEquals("38", container.tag)
    }
    
    @Test
    fun `container with digest should pin to specific version`() {
        val config = horizonOS {
            containers {
                container("pinned-container") {
                    image = "archlinux/archlinux"
                    tag = "latest"
                    digest = "sha256:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
                    packages("git")
                }
            }
        }
        
        assertNotNull(config.containers)
        val container = config.containers!!.containers[0]
        assertEquals("sha256:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef", 
                     container.digest)
    }
    
    @Test
    fun `container validation should accept valid image reference`() {
        assertDoesNotThrow {
            assertTrue(validateImageReference("archlinux/archlinux", "latest", null))
        }
        
        assertDoesNotThrow {
            assertTrue(validateImageReference("archlinux/archlinux", "latest", 
                     "sha256:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"))
        }
    }
    
    @Test
    fun `container validation should reject invalid image reference`() {
        assertDoesNotThrow {
            assertTrue(!validateImageReference("", "latest", null))
        }
        
        assertDoesNotThrow {
            assertTrue(!validateImageReference("archlinux/archlinux", "", null))
        }
        
        assertDoesNotThrow {
            assertTrue(!validateImageReference("archlinux/archlinux", "latest", "invalid-digest"))
        }
    }
    
    @Test
    fun `global mounts should be applied to all containers`() {
        val config = horizonOS {
            containers {
                globalMount("/shared/data")
                globalMount("/shared/config")
                
                container("container1") {
                    image = "archlinux/archlinux"
                    packages("git")
                }
                
                container("container2") {
                    image = "ubuntu"
                    packages("curl")
                }
            }
        }
        
        assertNotNull(config.containers)
        assertEquals(2, config.containers!!.globalMounts.size)
        assertTrue(config.containers!!.globalMounts.contains("/shared/data"))
        assertTrue(config.containers!!.globalMounts.contains("/shared/config"))
    }
    
    @Test
    fun `container runtime recommendations should match purpose`() {
        assertEquals(ContainerRuntime.DISTROBOX, getRecommendedRuntime(ContainerPurpose.DEVELOPMENT))
        assertEquals(ContainerRuntime.PODMAN, getRecommendedRuntime(ContainerPurpose.GAMING))
        assertEquals(ContainerRuntime.DISTROBOX, getRecommendedRuntime(ContainerPurpose.MULTIMEDIA))
        assertEquals(ContainerRuntime.DISTROBOX, getRecommendedRuntime(ContainerPurpose.OFFICE))
        assertEquals(ContainerRuntime.PODMAN, getRecommendedRuntime(ContainerPurpose.SECURITY))
        assertEquals(ContainerRuntime.DISTROBOX, getRecommendedRuntime(ContainerPurpose.CUSTOM))
    }
    
    @Test
    fun `default packages should match container purpose`() {
        val devPackages = getDefaultPackages(ContainerPurpose.DEVELOPMENT)
        assertTrue(devPackages.contains("git"))
        assertTrue(devPackages.contains("curl"))
        assertTrue(devPackages.contains("build-essential"))
        
        val gamingPackages = getDefaultPackages(ContainerPurpose.GAMING)
        assertTrue(gamingPackages.contains("steam"))
        assertTrue(gamingPackages.contains("lutris"))
        
        val multimediaPackages = getDefaultPackages(ContainerPurpose.MULTIMEDIA)
        assertTrue(multimediaPackages.contains("ffmpeg"))
        assertTrue(multimediaPackages.contains("imagemagick"))
    }
}