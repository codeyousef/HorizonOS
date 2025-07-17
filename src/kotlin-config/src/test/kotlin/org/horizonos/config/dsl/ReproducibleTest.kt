package org.horizonos.config.dsl

import org.junit.jupiter.api.Test
import org.junit.jupiter.api.assertDoesNotThrow
import org.junit.jupiter.api.assertThrows
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertTrue
import kotlin.test.assertFalse

class ReproducibleTest {
    
    @Test
    fun `reproducible DSL should create reproducible configuration`() {
        val config = horizonOS {
            reproducible {
                enabled = true
                strictMode = true
                verifyDigests = true
                
                ostree("horizonos/stable/x86_64", "abc123def456") {
                    version = "1.0"
                    digest = "sha256:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
                    url = "https://ostree.example.com/repo"
                    signature = "gpg-signature"
                }
            }
        }
        
        assertNotNull(config.reproducible)
        val reproducible = config.reproducible!!
        
        assertTrue(reproducible.enabled)
        assertTrue(reproducible.strictMode)
        assertTrue(reproducible.verifyDigests)
        
        assertNotNull(reproducible.systemImage)
        val systemImage = reproducible.systemImage!!
        assertEquals("horizonos/stable/x86_64", systemImage.base.ref)
        assertEquals("abc123def456", systemImage.base.commit)
        assertEquals("1.0", systemImage.base.version)
        assertEquals("sha256:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef", 
                     systemImage.base.digest)
        assertEquals("https://ostree.example.com/repo", systemImage.base.url)
        assertEquals("gpg-signature", systemImage.base.signature)
    }
    
    @Test
    fun `system image should support containers and flatpaks`() {
        val config = horizonOS {
            reproducible {
                enabled = true
                
                systemImage {
                    version = "1.0"
                    
                    base("horizonos/stable/x86_64", "abc123def456") {
                        digest = "sha256:base-digest"
                        signature = "base-signature"
                    }
                    
                    container("dev-container", "archlinux/archlinux", "sha256:container-digest") {
                        tag = "latest"
                        runtime = ContainerRuntime.DISTROBOX
                        purpose = ContainerPurpose.DEVELOPMENT
                        
                        pkg("git", "2.40.0") {
                            checksum = "sha256:git-checksum"
                            dependencies("glibc", "openssl")
                        }
                    }
                    
                    flatpak("org.mozilla.firefox", "firefox-commit-hash") {
                        version = "115.0"
                        runtime = "org.freedesktop.Platform"
                        runtimeVersion = "22.08"
                        downloadSize = 100000000
                        installedSize = 300000000
                    }
                    
                    metadata("build-date", "2024-01-15T10:00:00Z")
                    metadata("build-host", "build.example.com")
                }
            }
        }
        
        assertNotNull(config.reproducible)
        val systemImage = config.reproducible!!.systemImage!!
        
        assertEquals("1.0", systemImage.version)
        assertEquals("abc123def456", systemImage.base.commit)
        assertEquals("sha256:base-digest", systemImage.base.digest)
        
        assertEquals(1, systemImage.containers.size)
        val container = systemImage.containers[0]
        assertEquals("dev-container", container.name)
        assertEquals("archlinux/archlinux", container.image)
        assertEquals("sha256:container-digest", container.digest)
        assertEquals(ContainerRuntime.DISTROBOX, container.runtime)
        assertEquals(ContainerPurpose.DEVELOPMENT, container.purpose)
        
        assertEquals(1, container.packages.size)
        val pkg = container.packages[0]
        assertEquals("git", pkg.name)
        assertEquals("2.40.0", pkg.version)
        assertEquals("sha256:git-checksum", pkg.checksum)
        assertEquals(2, pkg.dependencies.size)
        
        assertEquals(1, systemImage.flatpaks.size)
        val flatpak = systemImage.flatpaks[0]
        assertEquals("org.mozilla.firefox", flatpak.id)
        assertEquals("firefox-commit-hash", flatpak.commit)
        assertEquals("115.0", flatpak.version)
        assertEquals("org.freedesktop.Platform", flatpak.runtime)
        assertEquals("22.08", flatpak.runtimeVersion)
        
        assertEquals(2, systemImage.metadata.size)
        assertEquals("2024-01-15T10:00:00Z", systemImage.metadata["build-date"])
        assertEquals("build.example.com", systemImage.metadata["build-host"])
    }
    
    @Test
    fun `system image validation should detect missing base`() {
        val systemImage = SystemImage(
            version = "1.0",
            timestamp = "2024-01-15T10:00:00Z",
            base = OstreeImage(
                ref = "",
                commit = "",
                version = "1.0",
                digest = "sha256:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
            )
        )
        
        val errors = validateSystemImage(systemImage)
        assertTrue(errors.isNotEmpty())
        assertTrue(errors.any { it.contains("Base image ref cannot be empty") })
        assertTrue(errors.any { it.contains("Base image commit cannot be empty") })
    }
    
    @Test
    fun `system image validation should detect invalid container digests`() {
        val systemImage = SystemImage(
            version = "1.0",
            timestamp = "2024-01-15T10:00:00Z",
            base = OstreeImage(
                ref = "horizonos/stable/x86_64",
                commit = "abc123def456",
                version = "1.0",
                digest = "sha256:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
            ),
            containers = listOf(
                ContainerImage(
                    name = "test-container",
                    image = "archlinux/archlinux",
                    tag = "latest",
                    digest = "invalid-digest",
                    runtime = ContainerRuntime.DISTROBOX,
                    purpose = ContainerPurpose.DEVELOPMENT
                )
            )
        )
        
        val errors = validateSystemImage(systemImage)
        assertTrue(errors.isNotEmpty())
        assertTrue(errors.any { it.contains("digest must be SHA256") })
    }
    
    @Test
    fun `system image validation should detect missing flatpak commits`() {
        val systemImage = SystemImage(
            version = "1.0",
            timestamp = "2024-01-15T10:00:00Z",
            base = OstreeImage(
                ref = "horizonos/stable/x86_64",
                commit = "abc123def456",
                version = "1.0",
                digest = "sha256:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
            ),
            flatpaks = listOf(
                FlatpakImage(
                    id = "org.mozilla.firefox",
                    version = "115.0",
                    branch = "stable",
                    commit = "",
                    runtime = "org.freedesktop.Platform",
                    runtimeVersion = "22.08"
                )
            )
        )
        
        val errors = validateSystemImage(systemImage)
        assertTrue(errors.isNotEmpty())
        assertTrue(errors.any { it.contains("must have a commit") })
    }
    
    @Test
    fun `system image comparison should detect changes`() {
        val oldSystem = SystemImage(
            version = "1.0",
            timestamp = "2024-01-15T10:00:00Z",
            base = OstreeImage(
                ref = "horizonos/stable/x86_64",
                commit = "old-commit",
                version = "1.0",
                digest = "sha256:old-digest"
            ),
            containers = listOf(
                ContainerImage(
                    name = "container1",
                    image = "archlinux/archlinux",
                    tag = "latest",
                    digest = "sha256:old-container-digest",
                    runtime = ContainerRuntime.DISTROBOX,
                    purpose = ContainerPurpose.DEVELOPMENT
                )
            ),
            flatpaks = listOf(
                FlatpakImage(
                    id = "org.mozilla.firefox",
                    version = "115.0",
                    branch = "stable",
                    commit = "old-firefox-commit",
                    runtime = "org.freedesktop.Platform",
                    runtimeVersion = "22.08"
                )
            )
        )
        
        val newSystem = SystemImage(
            version = "1.1",
            timestamp = "2024-01-16T10:00:00Z",
            base = OstreeImage(
                ref = "horizonos/stable/x86_64",
                commit = "new-commit",
                version = "1.1",
                digest = "sha256:new-digest"
            ),
            containers = listOf(
                ContainerImage(
                    name = "container1",
                    image = "archlinux/archlinux",
                    tag = "latest",
                    digest = "sha256:new-container-digest",
                    runtime = ContainerRuntime.DISTROBOX,
                    purpose = ContainerPurpose.DEVELOPMENT
                ),
                ContainerImage(
                    name = "container2",
                    image = "ubuntu",
                    tag = "latest",
                    digest = "sha256:ubuntu-digest",
                    runtime = ContainerRuntime.DISTROBOX,
                    purpose = ContainerPurpose.DEVELOPMENT
                )
            ),
            flatpaks = listOf(
                FlatpakImage(
                    id = "org.mozilla.firefox",
                    version = "116.0",
                    branch = "stable",
                    commit = "new-firefox-commit",
                    runtime = "org.freedesktop.Platform",
                    runtimeVersion = "22.08"
                )
            )
        )
        
        val diff = compareSystemImages(oldSystem, newSystem)
        
        assertTrue(diff.hasChanges())
        assertTrue(diff.baseChanged)
        assertEquals(2, diff.containerChanges.size)
        assertEquals(1, diff.flatpakChanges.size)
        
        val containerChange = diff.containerChanges.find { it.name == "container1" }
        assertNotNull(containerChange)
        assertEquals(ChangeType.UPDATED, containerChange.type)
        assertEquals("sha256:old-container-digest", containerChange.oldDigest)
        assertEquals("sha256:new-container-digest", containerChange.newDigest)
        
        val newContainerChange = diff.containerChanges.find { it.name == "container2" }
        assertNotNull(newContainerChange)
        assertEquals(ChangeType.ADDED, newContainerChange.type)
        assertEquals(null, newContainerChange.oldDigest)
        assertEquals("sha256:ubuntu-digest", newContainerChange.newDigest)
        
        val flatpakChange = diff.flatpakChanges[0]
        assertEquals("org.mozilla.firefox", flatpakChange.id)
        assertEquals(ChangeType.UPDATED, flatpakChange.type)
        assertEquals("old-firefox-commit", flatpakChange.oldCommit)
        assertEquals("new-firefox-commit", flatpakChange.newCommit)
    }
    
    @Test
    fun `system image checksum generation should be deterministic`() {
        val systemImage = SystemImage(
            version = "1.0",
            timestamp = "2024-01-15T10:00:00Z",
            base = OstreeImage(
                ref = "horizonos/stable/x86_64",
                commit = "abc123def456",
                version = "1.0",
                digest = "sha256:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
            )
        )
        
        val checksum1 = generateSystemImageChecksum(systemImage)
        val checksum2 = generateSystemImageChecksum(systemImage)
        
        assertEquals(checksum1, checksum2)
        assertTrue(checksum1.startsWith("sha256:"))
    }
    
    @Test
    fun `validation mode should control error handling`() {
        val config = horizonOS {
            reproducible {
                enabled = true
                strictMode = true
                validationMode = ValidationMode.STRICT
                
                ostree("horizonos/stable/x86_64", "abc123def456")
            }
        }
        
        assertNotNull(config.reproducible)
        assertEquals(ValidationMode.STRICT, config.reproducible!!.validationMode)
    }
    
    @Test
    fun `signature validation should be configurable`() {
        val config = horizonOS {
            reproducible {
                enabled = true
                signatureValidation = true
                allowUnsigned = false
                
                ostree("horizonos/stable/x86_64", "abc123def456") {
                    signature = "gpg-signature"
                }
            }
        }
        
        assertNotNull(config.reproducible)
        assertTrue(config.reproducible!!.signatureValidation)
        assertFalse(config.reproducible!!.allowUnsigned)
        assertEquals("gpg-signature", config.reproducible!!.systemImage!!.base.signature)
    }
}