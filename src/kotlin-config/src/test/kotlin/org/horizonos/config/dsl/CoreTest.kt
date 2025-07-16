package org.horizonos.config.dsl

import io.kotest.core.spec.style.StringSpec
import io.kotest.matchers.shouldBe
import io.kotest.matchers.shouldNotBe
import io.kotest.matchers.collections.shouldContain
import io.kotest.matchers.collections.shouldContainExactly
import io.kotest.matchers.collections.shouldHaveSize
import io.kotest.matchers.nulls.shouldNotBeNull
import io.kotest.matchers.types.shouldBeInstanceOf

class CoreTest : StringSpec({
    
    "should create basic system configuration" {
        val config = horizonOS {
            hostname = "test-system"
            timezone = "America/New_York"
            locale = "en_US.UTF-8"
        }
        
        config.system.hostname shouldBe "test-system"
        config.system.timezone shouldBe "America/New_York"
        config.system.locale shouldBe "en_US.UTF-8"
    }
    
    "should handle package installations" {
        val config = horizonOS {
            packages {
                install("vim", "git", "docker")
            }
        }
        
        config.packages shouldHaveSize 3
        config.packages.map { it.name } shouldContainExactly listOf("vim", "git", "docker")
        config.packages.all { it.action == PackageAction.INSTALL } shouldBe true
    }
    
    "should handle package removals" {
        val config = horizonOS {
            packages {
                remove("nano", "vim")
            }
        }
        
        config.packages shouldHaveSize 2
        config.packages.all { it.action == PackageAction.REMOVE } shouldBe true
    }
    
    "should handle mixed package operations" {
        val config = horizonOS {
            packages {
                install("git", "docker")
                remove("nano")
            }
        }
        
        config.packages shouldHaveSize 3
        config.packages.filter { it.action == PackageAction.INSTALL } shouldHaveSize 2
        config.packages.filter { it.action == PackageAction.REMOVE } shouldHaveSize 1
    }
    
    "should handle package groups" {
        val config = horizonOS {
            packages {
                group("development") {
                    install("git", "gcc", "make")
                }
            }
        }
        
        config.packages shouldHaveSize 3
        config.packages.all { it.group == "development" } shouldBe true
    }
    
    "should handle service configurations" {
        val config = horizonOS {
            services {
                enable("NetworkManager")
                enable("sshd") {
                    autoRestart = true
                    restartOnFailure = false
                    env("SSH_PORT", "2222")
                }
                disable("bluetooth")
            }
        }
        
        config.services shouldHaveSize 3
        
        val networkManager = config.services.find { it.name == "NetworkManager" }
        networkManager shouldNotBe null
        networkManager!!.enabled shouldBe true
        
        val sshd = config.services.find { it.name == "sshd" }
        sshd shouldNotBe null
        sshd!!.enabled shouldBe true
        sshd.config shouldNotBe null
        sshd.config!!.autoRestart shouldBe true
        sshd.config!!.restartOnFailure shouldBe false
        sshd.config!!.environment["SSH_PORT"] shouldBe "2222"
        
        val bluetooth = config.services.find { it.name == "bluetooth" }
        bluetooth shouldNotBe null
        bluetooth!!.enabled shouldBe false
    }
    
    "should handle user management" {
        val config = horizonOS {
            users {
                user("admin") {
                    uid = 1000
                    shell = "/usr/bin/fish"
                    groups("wheel", "docker", "video")
                }
                user("guest") {
                    shell = "/usr/bin/bash"
                }
            }
        }
        
        config.users shouldHaveSize 2
        
        val admin = config.users.find { it.name == "admin" }
        admin shouldNotBe null
        admin!!.uid shouldBe 1000
        admin.shell shouldBe "/usr/bin/fish"
        admin.groups shouldContainExactly listOf("wheel", "docker", "video")
        admin.homeDir shouldBe "/home/admin"
        
        val guest = config.users.find { it.name == "guest" }
        guest shouldNotBe null
        guest!!.uid shouldBe null
        guest.shell shouldBe "/usr/bin/bash"
        guest.homeDir shouldBe "/home/guest"
    }
    
    "should handle repository configuration" {
        val config = horizonOS {
            repositories {
                add("core", "https://mirror.archlinux.org/core/os/x86_64") {
                    priority = 10
                    gpgCheck = true
                }
                add("extra", "https://mirror.archlinux.org/extra/os/x86_64") {
                    enabled = false
                }
                ostree("horizonos", "https://ostree.horizonos.org") {
                    branch("stable")
                    branch("testing")
                    gpgCheck = true
                }
            }
        }
        
        config.repositories shouldHaveSize 3
        
        val core = config.repositories.find { it.name == "core" }
        core shouldNotBe null
        core!!.shouldBeInstanceOf<PackageRepository>()
        core.priority shouldBe 10
        core.gpgCheck shouldBe true
        
        val extra = config.repositories.find { it.name == "extra" }
        extra shouldNotBe null
        extra!!.enabled shouldBe false
        
        val ostree = config.repositories.find { it.name == "horizonos" }
        ostree shouldNotBe null
        ostree!!.shouldBeInstanceOf<OstreeRepository>()
        val ostreeRepo = ostree as OstreeRepository
        ostreeRepo.branches shouldContainExactly listOf("stable", "testing")
    }
    
    "should handle desktop environment configuration" {
        val config = horizonOS {
            users {
                user("admin") {
                    shell = "/bin/bash"
                }
            }
            
            desktop {
                environment = DesktopEnvironment.HYPRLAND
                autoLogin = true
                autoLoginUser = "admin"
                
                hyprland {
                    theme = "breeze-dark"
                    animations = true
                    gaps = 15
                    borderSize = 3
                    kdeIntegration = false
                    personalityMode = PersonalityMode.MACOS
                }
            }
        }
        
        config.desktop shouldNotBe null
        config.desktop!!.environment shouldBe DesktopEnvironment.HYPRLAND
        config.desktop!!.autoLogin shouldBe true
        config.desktop!!.autoLoginUser shouldBe "admin"
        
        val hyprland = config.desktop!!.hyprlandConfig
        hyprland shouldNotBe null
        hyprland!!.theme shouldBe "breeze-dark"
        hyprland.animations shouldBe true
        hyprland.gaps shouldBe 15
        hyprland.borderSize shouldBe 3
        hyprland.kdeIntegration shouldBe false
        hyprland.personalityMode shouldBe PersonalityMode.MACOS
    }
    
    "should handle plasma desktop configuration" {
        val config = horizonOS {
            desktop {
                environment = DesktopEnvironment.PLASMA
                
                plasma {
                    theme = "breeze-light"
                    lookAndFeel = "org.kde.breeze.desktop"
                    widgets("org.kde.plasma.systemtray", "org.kde.plasma.taskmanager")
                }
            }
        }
        
        config.desktop shouldNotBe null
        config.desktop!!.environment shouldBe DesktopEnvironment.PLASMA
        
        val plasma = config.desktop!!.plasmaConfig
        plasma shouldNotBe null
        plasma!!.theme shouldBe "breeze-light"
        plasma.lookAndFeel shouldBe "org.kde.breeze.desktop"
        plasma.widgets shouldContainExactly listOf("org.kde.plasma.systemtray", "org.kde.plasma.taskmanager")
    }
    
    "should create complete configuration" {
        val config = horizonOS {
            hostname = "complete-system"
            timezone = "UTC"
            locale = "en_US.UTF-8"
            
            packages {
                install("base", "linux", "networkmanager")
                group("desktop") {
                    install("plasma-meta", "firefox")
                }
            }
            
            services {
                enable("NetworkManager")
                enable("sshd") {
                    env("PORT", "22")
                }
            }
            
            users {
                user("admin") {
                    uid = 1000
                    shell = "/usr/bin/fish"
                    groups("wheel", "users")
                }
            }
            
            repositories {
                add("core", "https://mirror.archlinux.org")
                ostree("horizonos", "https://ostree.horizonos.org") {
                    branch("stable")
                }
            }
            
            desktop {
                environment = DesktopEnvironment.PLASMA
                autoLogin = true
                autoLoginUser = "admin"
            }
        }
        
        // Verify all components are present
        config.system.hostname shouldBe "complete-system"
        config.packages shouldHaveSize 5
        config.services shouldHaveSize 2
        config.users shouldHaveSize 1
        config.repositories shouldHaveSize 2
        config.desktop shouldNotBe null
    }
})