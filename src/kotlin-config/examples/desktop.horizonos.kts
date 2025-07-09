// src/kotlin-config/examples/desktop.horizonos.kts
import org.horizonos.config.dsl.*

horizonOS {
    // System configuration
    hostname = "my-horizonos"
    timezone = "America/New_York"
    locale = "en_US.UTF-8"
    
    // Package management
    packages {
        // Core system packages
        group("base") {
            install(
                "base", "base-devel",
                "linux", "linux-firmware",
                "btrfs-progs", "grub", "efibootmgr",
                "networkmanager", "openssh"
            )
        }
        
        // Development tools
        group("development") {
            install(
                "git", "neovim", "tmux",
                "docker", "docker-compose",
                "rustup", "go", "nodejs", "npm"
            )
        }
        
        // Desktop environment
        group("desktop") {
            install(
                "plasma-meta",
                "kde-applications-meta",
                "firefox", "thunderbird"
            )
        }
        
        // Shell
        install("fish", "starship", "bat", "exa", "ripgrep", "fd")
        
        // Remove unwanted packages
        remove("nano")  // We prefer neovim
    }
    
    // Service configuration
    services {
        enable("NetworkManager")
        enable("sshd") {
            autoRestart = true
            restartOnFailure = true
        }
        enable("docker") {
            env("DOCKER_OPTS", "--storage-driver=btrfs")
        }
        enable("snapper-timeline.timer")
        enable("snapper-cleanup.timer")
        
        // Disable unnecessary services
        disable("bluetooth")  // Enable only when needed
    }
    
    // User management
    users {
        user("admin") {
            uid = 1000
            shell = "/usr/bin/fish"
            groups("wheel", "docker", "video", "audio")
        }
        
        user("guest") {
            uid = 1001
            shell = "/usr/bin/bash"  // More familiar for guests
            groups("users")
        }
    }
    
    // Repository configuration
    repositories {
        // Main Arch repositories
        add("core", "https://mirror.archlinux.org/core/os/x86_64") {
            priority = 10
        }
        
        add("extra", "https://mirror.archlinux.org/extra/os/x86_64") {
            priority = 20
        }
        
        add("community", "https://mirror.archlinux.org/community/os/x86_64") {
            priority = 30
        }
        
        // HorizonOS OSTree repository
        ostree("horizonos", "https://ostree.horizonos.org") {
            branch("stable")
            branch("testing")
            branch("dev")
            gpgCheck = true
        }
    }
}