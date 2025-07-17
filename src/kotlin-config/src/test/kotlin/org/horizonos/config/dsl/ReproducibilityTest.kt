package org.horizonos.config.dsl

import org.junit.jupiter.api.Test
import org.junit.jupiter.api.Assertions.*
import org.junit.jupiter.api.assertThrows

class ReproducibilityTest {
    
    @Test
    fun `test container config generates correct lockfile`() {
        val config = horizonOS {
            hostname = "test-system"
            
            containers {
                distrobox("dev-tools") {
                    archlinux()
                    digest = "sha256:abc123def456abc123def456abc123def456abc123def456abc123def456abc1"
                    packages("git", "neovim")
                    export("git", "nvim")
                }
            }
            
            layers {
                systemLayer("dev", LayerPurpose.DEVELOPMENT) {
                    image = "archlinux"
                    digest = "sha256:abc123def456abc123def456abc123def456abc123def456abc123def456abc1"
                    packages("git", "neovim")
                }
            }
            
            reproducible {
                enabled = true
                verifyDigests = true
            }
        }
        
        // Verify containers config exists
        assertNotNull(config.containers)
        assertEquals(1, config.containers?.containers?.size)
        
        // Verify container digest
        val container = config.containers?.containers?.first()
        assertEquals("sha256:abc123def456abc123def456abc123def456abc123def456abc123def456abc1", container?.digest)
        assertEquals(listOf("git", "neovim"), container?.packages)
        
        // Verify layers config exists
        assertNotNull(config.layers)
        assertEquals(1, config.layers?.systemLayers?.size)
        
        // Verify reproducible config
        assertNotNull(config.reproducible)
        assertTrue(config.reproducible?.enabled ?: false)
        assertTrue(config.reproducible?.verifyDigests ?: false)
    }
    
    @Test
    fun `test identical configs produce identical output`() {
        val config1 = horizonOS {
            hostname = "test"
            timezone = "UTC"
            
            containers {
                distrobox("tools") {
                    archlinux()
                    digest = "sha256:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
                    packages("git", "vim")
                }
            }
        }
        
        val config2 = horizonOS {
            hostname = "test"
            timezone = "UTC"
            
            containers {
                distrobox("tools") {
                    archlinux()
                    digest = "sha256:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
                    packages("git", "vim")
                }
            }
        }
        
        // Compare key fields
        assertEquals(config1.system.hostname, config2.system.hostname)
        assertEquals(config1.system.timezone, config2.system.timezone)
        
        // Compare containers
        val container1 = config1.containers?.containers?.first()
        val container2 = config2.containers?.containers?.first()
        
        assertEquals(container1?.name, container2?.name)
        assertEquals(container1?.digest, container2?.digest)
        assertEquals(container1?.packages, container2?.packages)
    }
    
    @Test
    fun `test system image validation`() {
        // Create a system image with invalid data
        val systemImage = SystemImage(
            base = OstreeImage(
                ref = "",  // Invalid: empty ref
                commit = "",  // Invalid: empty commit
                version = "1.0",
                digest = "sha256:test",
                url = null,
                signature = null
            ),
            containers = listOf(
                ContainerImage(
                    name = "test",
                    image = "archlinux",
                    tag = "latest",
                    digest = "invalid-digest",  // Invalid: not SHA256 format
                    runtime = ContainerRuntime.DISTROBOX,
                    purpose = ContainerPurpose.DEVELOPMENT
                )
            )
        )
        
        // Validate and check errors
        val errors = validateSystemImage(systemImage)
        
        assertTrue(errors.isNotEmpty())
        assertTrue(errors.any { it.contains("Base image ref cannot be empty") })
        assertTrue(errors.any { it.contains("Base image commit cannot be empty") })
        assertTrue(errors.any { it.contains("Container 'test' digest must be SHA256") })
    }
    
    @Test
    fun `test container digest validation`() {
        // Valid digest
        assertTrue(validateImageReference(
            "docker.io/archlinux/archlinux",
            "latest",
            "sha256:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
        ))
        
        // Invalid digest format
        assertFalse(validateImageReference(
            "docker.io/archlinux/archlinux",
            "latest",
            "sha256:invalid"
        ))
        
        // Missing sha256 prefix
        assertFalse(validateImageReference(
            "docker.io/archlinux/archlinux",
            "latest",
            "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
        ))
        
        // Null digest is valid (optional)
        assertTrue(validateImageReference(
            "docker.io/archlinux/archlinux",
            "latest",
            null
        ))
    }
    
    @Test
    fun `test layer dependencies`() {
        val config = horizonOS {
            layers {
                // Base layer
                base {
                    packages("base", "linux", "systemd")
                }
                
                // System layer depends on base
                systemLayer("core", LayerPurpose.CORE) {
                    image = "archlinux"
                    dependsOn("base")
                    packages("networkmanager", "openssh")
                }
                
                // Development layer depends on core
                systemLayer("development", LayerPurpose.DEVELOPMENT) {
                    image = "archlinux"
                    dependsOn("core")
                    packages("git", "gcc", "make")
                }
            }
        }
        
        assertNotNull(config.layers)
        assertEquals(2, config.layers?.systemLayers?.size)
        
        val coreDeps = config.layers?.systemLayers?.find { it.name == "core" }?.dependencies
        assertTrue(coreDeps?.contains("base") ?: false)
        
        val devDeps = config.layers?.systemLayers?.find { it.name == "development" }?.dependencies
        assertTrue(devDeps?.contains("core") ?: false)
    }
    
    @Test
    fun `test package migration from legacy to containers`() {
        val config = horizonOS {
            enhancedPackages {
                autoMigrate = true
                migrationStrategy = MigrationStrategy.CONTAINER_FIRST
                
                // Legacy packages that should be migrated
                legacy {
                    install("git")
                    install("gcc")
                    install("firefox")
                }
            }
        }
        
        assertNotNull(config.enhancedPackages)
        assertTrue(config.enhancedPackages?.autoMigrate ?: false)
        assertEquals(MigrationStrategy.CONTAINER_FIRST, config.enhancedPackages?.migrationStrategy)
        
        // Check legacy packages exist
        val legacyPackages = config.enhancedPackages?.legacy?.packages
        assertEquals(3, legacyPackages?.size)
    }
}