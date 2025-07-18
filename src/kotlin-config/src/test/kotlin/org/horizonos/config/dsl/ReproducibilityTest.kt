package org.horizonos.config.dsl

import org.junit.jupiter.api.Test
import org.junit.jupiter.api.Assertions.*
import org.junit.jupiter.api.assertThrows

class ReproducibilityTest {
    
    @Test
    fun `test basic config creation`() {
        val config = horizonOS {
            hostname = "test-system"
            timezone = "UTC"
            locale = "en_US.UTF-8"
        }
        
        // Verify basic system config
        assertEquals("test-system", config.system.hostname)
        assertEquals("UTC", config.system.timezone)
        assertEquals("en_US.UTF-8", config.system.locale)
    }
    
    @Test
    fun `test identical configs produce identical output`() {
        val config1 = horizonOS {
            hostname = "test"
            timezone = "UTC"
            locale = "en_US.UTF-8"
        }
        
        val config2 = horizonOS {
            hostname = "test"
            timezone = "UTC"
            locale = "en_US.UTF-8"
        }
        
        // Compare key fields
        assertEquals(config1.system.hostname, config2.system.hostname)
        assertEquals(config1.system.timezone, config2.system.timezone)
        assertEquals(config1.system.locale, config2.system.locale)
    }
    
    @Test
    fun `test service configuration`() {
        val config = horizonOS {
            hostname = "test-system"
            
            services {
                enable("sshd") {
                    autoRestart = true
                    env("SSH_PORT", "22")
                }
                disable("bluetooth")
            }
        }
        
        // Verify services config
        assertEquals(2, config.services.size)
        val sshd = config.services.find { it.name == "sshd" }
        assertNotNull(sshd)
        assertTrue(sshd?.enabled ?: false)
        
        val bluetooth = config.services.find { it.name == "bluetooth" }
        assertNotNull(bluetooth)
        assertFalse(bluetooth?.enabled ?: true)
    }
    
    @Test
    fun `test user configuration`() {
        val config = horizonOS {
            hostname = "test-system"
            
            users {
                user("developer") {
                    uid = 1000
                    shell = "/usr/bin/bash"
                    groups("wheel", "users")
                }
            }
        }
        
        // Verify users config
        assertEquals(1, config.users.size)
        val developer = config.users.first()
        assertEquals("developer", developer.name)
        assertEquals(1000, developer.uid)
        assertEquals("/usr/bin/bash", developer.shell)
        assertTrue(developer.groups.contains("wheel"))
        assertTrue(developer.groups.contains("users"))
    }
    
    @Test
    fun `test packages configuration`() {
        val config = horizonOS {
            hostname = "test-system"
            
            packages {
                install("git")
                install("vim")
                remove("nano")
            }
        }
        
        // Verify packages config
        assertEquals(3, config.packages.size)
        val git = config.packages.find { it.name == "git" }
        assertNotNull(git)
        assertEquals(PackageAction.INSTALL, git?.action)
        
        val nano = config.packages.find { it.name == "nano" }
        assertNotNull(nano)
        assertEquals(PackageAction.REMOVE, nano?.action)
    }
    
    @Test
    fun `test repositories configuration`() {
        val config = horizonOS {
            hostname = "test-system"
            
            repositories {
                add("extra", "https://mirrors.kernel.org/archlinux/extra/os/x86_64") {
                    enabled = true
                    gpgCheck = true
                    priority = 50
                }
            }
        }
        
        // Verify repositories config
        assertEquals(1, config.repositories.size)
        val extra = config.repositories.first() as PackageRepository
        assertEquals("extra", extra.name)
        assertEquals("https://mirrors.kernel.org/archlinux/extra/os/x86_64", extra.url)
        assertTrue(extra.enabled)
        assertTrue(extra.gpgCheck)
        assertEquals(50, extra.priority)
    }
}