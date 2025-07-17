#!/usr/bin/env kotlin

@file:Repository("https://repo1.maven.org/maven2/")
@file:DependsOn("org.horizonos:kotlin-config:1.0")

import org.horizonos.config.dsl.*

/**
 * HorizonOS Developer Configuration
 * 
 * This configuration creates a development-focused system with:
 * - Container-based development tools
 * - Multiple language environments
 * - Development applications via Flatpak
 * - Code editors and IDEs
 * - Version control and collaboration tools
 */

horizonOS {
    // Basic system configuration
    hostname = "horizon-dev"
    timezone = "America/New_York"
    locale = "en_US.UTF-8"
    
    // Container-based development environments
    containers {
        defaultRuntime = ContainerRuntime.DISTROBOX
        
        // Rust development environment
        devContainer("rust-dev") {
            rust("1.70")
            packages("build-essential", "git", "curl", "vim")
            export("rustc", "cargo", "rustfmt", "clippy")
            mount("/home/user/projects")
            env("RUST_BACKTRACE", "1")
        }
        
        // Node.js development environment
        devContainer("node-dev") {
            nodejs("18")
            packages("build-essential", "git", "python3")
            export("node", "npm", "npx", "yarn")
            mount("/home/user/projects")
            env("NODE_ENV", "development")
        }
        
        // Python development environment
        devContainer("python-dev") {
            python("3.11")
            packages("build-essential", "git", "curl", "sqlite3")
            export("python", "pip", "python3")
            mount("/home/user/projects")
            env("PYTHONPATH", "/home/user/projects")
        }
        
        // Go development environment
        devContainer("go-dev") {
            golang("1.20")
            packages("build-essential", "git", "curl")
            export("go", "gofmt", "godoc")
            mount("/home/user/projects")
            env("GOPATH", "/home/user/go")
        }
        
        // General development tools
        container("dev-tools") {
            image = "archlinux/archlinux"
            runtime = ContainerRuntime.DISTROBOX
            purpose = ContainerPurpose.DEVELOPMENT
            packages("git", "curl", "wget", "vim", "tmux", "htop", "tree", "jq")
            export("git", "curl", "wget", "vim", "tmux", "htop", "tree", "jq")
            mount("/home/user/projects")
            mount("/home/user/.ssh")
            mount("/home/user/.gitconfig")
        }
        
        // Database development
        container("db-dev") {
            image = "postgres:15"
            runtime = ContainerRuntime.PODMAN
            purpose = ContainerPurpose.DEVELOPMENT
            packages("postgresql-client", "redis-tools", "sqlite3")
            export("psql", "redis-cli", "sqlite3")
            port("5432:5432")
            port("6379:6379")
            env("POSTGRES_PASSWORD", "dev")
            env("POSTGRES_DB", "development")
        }
        
        globalMount("/home/user/projects")
        globalMount("/home/user/.config")
    }
    
    // Layered architecture for development
    layers {
        base {
            minimal = true
            packages("base", "linux", "systemd", "podman", "flatpak", "git")
            services("systemd-networkd", "systemd-resolved", "podman.socket")
            commit("stable-dev-base")
        }
        
        systemLayer("development", LayerPurpose.DEVELOPMENT) {
            priority = 10
            autoStart = true
            strategy = LayerStrategy.ALWAYS_ON
            
            development {
                packages("git", "curl", "wget", "vim", "tmux", "htop")
                export("git", "curl", "wget", "vim", "tmux", "htop")
            }
            
            healthCheck {
                command = "curl -f http://localhost:8080/health || exit 1"
                interval = "30s"
                timeout = "5s"
                retries = 3
            }
        }
        
        systemLayer("languages", LayerPurpose.DEVELOPMENT) {
            priority = 20
            dependsOn("development")
            
            container {
                image = "archlinux/archlinux"
                runtime = ContainerRuntime.DISTROBOX
                packages("rust", "nodejs", "python", "go", "java-openjdk")
                export("rustc", "cargo", "node", "npm", "python", "go", "java", "javac")
            }
        }
        
        user {
            autoUpdates = true
            userScope = true
            
            // Development applications
            flatpak("com.visualstudio.code") {
                permissions("--filesystem=home", "--share=network")
                allowNetwork()
                allowFilesystem()
                allowDisplay()
            }
            
            flatpak("org.gnome.Builder") {
                permissions("--filesystem=home", "--share=network")
                allowNetwork()
                allowFilesystem()
                allowDisplay()
            }
            
            flatpak("com.jetbrains.IntelliJ-IDEA-Community") {
                permissions("--filesystem=home", "--share=network")
                allowNetwork()
                allowFilesystem()
                allowDisplay()
            }
            
            flatpak("io.github.shiftey.Desktop") {
                allowNetwork()
                allowDisplay()
            }
            
            // Git and version control
            flatpak("com.github.gitfiend.GitFiend") {
                allowNetwork()
                allowFilesystem()
                allowDisplay()
            }
            
            // Terminal and system tools
            flatpak("org.gnome.Terminal") {
                allowDisplay()
            }
            
            flatpak("com.raggesilver.BlackBox") {
                allowDisplay()
            }
            
            // Browser for development
            flatpak("org.mozilla.firefox") {
                allowNetwork()
                allowDisplay()
                allowAudio()
            }
            
            // Communication tools
            flatpak("com.discordapp.Discord") {
                allowNetwork()
                allowDisplay()
                allowAudio()
            }
            
            flatpak("com.slack.Slack") {
                allowNetwork()
                allowDisplay()
                allowAudio()
            }
        }
        
        order("base", "development", "languages", "user")
        globalMount("/home/user/projects")
        globalMount("/home/user/.config")
        sharedVolume("dev-workspace")
    }
    
    // Enhanced package management
    enhancedPackages {
        autoMigrate = true
        migrationStrategy = MigrationStrategy.CONTAINER_FIRST
        
        system {
            strategy = SystemPackageStrategy.CONTAINER
            defaultRuntime = ContainerRuntime.DISTROBOX
            
            development("dev-tools") {
                packages("git", "curl", "wget", "vim", "tmux", "htop", "tree", "jq")
                export("git", "curl", "wget", "vim", "tmux", "htop", "tree", "jq")
            }
            
            multimedia("media-tools") {
                packages("ffmpeg", "imagemagick", "sox")
                export("ffmpeg", "convert", "sox")
            }
            
            globalMount("/home/user/projects")
            autoUpdate = true
        }
        
        applications {
            strategy = ApplicationPackageStrategy.FLATPAK
            autoUpdate = true
            
            development()
            
            flatpak("org.gimp.GIMP") {
                allowFilesystem()
                allowDisplay()
            }
            
            flatpak("org.inkscape.Inkscape") {
                allowFilesystem()
                allowDisplay()
            }
            
            popular("firefox", "thunderbird", "libreoffice")
        }
    }
    
    // Reproducible development environment
    reproducible {
        enabled = true
        strictMode = false
        verifyDigests = true
        
        ostree("horizonos/dev/x86_64", "abc123def456") {
            version = "1.0-dev"
            digest = "sha256:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
            url = "https://ostree.horizonos.dev/repo"
            signature = "dev-key-signature"
        }
        
        systemImage {
            version = "1.0-dev"
            signature = "dev-system-signature"
            
            container("rust-dev", "docker.io/rust:1.70", "sha256:rust-digest") {
                runtime = ContainerRuntime.DISTROBOX
                purpose = ContainerPurpose.DEVELOPMENT
                
                pkg("rustc", "1.70.0") {
                    checksum = "sha256:rustc-checksum"
                    dependencies("glibc", "gcc")
                }
                
                pkg("cargo", "1.70.0") {
                    checksum = "sha256:cargo-checksum"
                    dependencies("rustc")
                }
            }
            
            flatpak("com.visualstudio.code", "stable-commit-hash") {
                version = "1.80.0"
                runtime = "org.freedesktop.Sdk"
                runtimeVersion = "22.08"
                downloadSize = 100000000
                installedSize = 300000000
            }
            
            metadata("build-date", "2024-01-15T10:00:00Z")
            metadata("build-host", "build.horizonos.dev")
            metadata("developer", "HorizonOS Dev Team")
        }
    }
    
    // Desktop configuration
    desktop {
        environment = DesktopEnvironment.HYPRLAND
        autoLogin = true
        autoLoginUser = "developer"
        
        hyprland {
            theme = "breeze-dark"
            animations = true
            gaps = 15
            borderSize = 2
            kdeIntegration = true
            personalityMode = PersonalityMode.I3
        }
    }
    
    // Hardware configuration for development
    hardware {
        cpu {
            enableTurboBoost = true
            governor = "performance"
        }
        
        memory {
            swappiness = 10
            hugepages = true
        }
        
        gpu {
            driver = "nvidia"
            enableCuda = true
            opengl = true
            vulkan = true
        }
        
        audio {
            server = AudioServer.PULSEAUDIO
            enableLowLatency = true
            sampleRate = 48000
        }
    }
    
    // Security configuration
    security {
        firewall {
            enabled = true
            defaultPolicy = "DROP"
            
            allowPort(22, "tcp")  // SSH
            allowPort(3000, "tcp") // Development server
            allowPort(8080, "tcp") // Web development
            allowPort(5432, "tcp") // PostgreSQL
            allowPort(6379, "tcp") // Redis
        }
        
        selinux {
            enabled = false  // Disabled for development convenience
        }
        
        apparmor {
            enabled = true
            mode = "complain"
        }
    }
    
    // Network configuration
    network {
        hostname = "horizon-dev"
        
        interfaces {
            ethernet("enp0s3") {
                dhcp = true
                mtu = 1500
            }
            
            wifi("wlan0") {
                dhcp = true
                powersave = false
            }
        }
        
        dns {
            servers("1.1.1.1", "8.8.8.8")
            searchDomains("local.dev", "docker.local")
        }
    }
    
    // Storage configuration
    storage {
        filesystem = "btrfs"
        
        subvolumes {
            subvolume("@") {
                mountpoint = "/"
                options = "defaults,noatime,compress=zstd"
            }
            
            subvolume("@home") {
                mountpoint = "/home"
                options = "defaults,noatime,compress=zstd"
            }
            
            subvolume("@var") {
                mountpoint = "/var"
                options = "defaults,noatime,compress=zstd"
            }
            
            subvolume("@snapshots") {
                mountpoint = "/.snapshots"
                options = "defaults,noatime"
            }
            
            subvolume("@dev-workspace") {
                mountpoint = "/home/user/projects"
                options = "defaults,noatime,compress=zstd"
            }
        }
        
        encryption {
            enabled = true
            cipher = "aes-xts-plain64"
            keySize = 512
        }
    }
    
    // Development services
    services {
        enable("sshd") {
            autoRestart = true
            env("SSH_PORT", "22")
        }
        
        enable("docker") {
            autoRestart = true
            restartOnFailure = true
        }
        
        enable("podman.socket") {
            autoRestart = true
        }
        
        enable("flatpak-system-helper") {
            autoRestart = true
        }
        
        disable("bluetooth")
        disable("cups")
    }
    
    // User configuration
    users {
        user("developer") {
            shell = "/usr/bin/fish"
            groups("wheel", "docker", "video", "audio", "input")
            homeDir = "/home/developer"
        }
        
        user("guest") {
            shell = "/usr/bin/bash"
            groups("users")
        }
    }
    
    // Package repositories
    repositories {
        add("archlinux-extra", "https://geo.mirror.pkgbuild.com/extra/os/x86_64") {
            enabled = true
            priority = 10
        }
        
        add("archlinux-community", "https://geo.mirror.pkgbuild.com/community/os/x86_64") {
            enabled = true
            priority = 20
        }
        
        ostree("horizonos-dev", "https://ostree.horizonos.dev/repo") {
            enabled = true
            priority = 5
            branch("horizonos/dev/x86_64")
            branch("horizonos/testing/x86_64")
        }
    }
}