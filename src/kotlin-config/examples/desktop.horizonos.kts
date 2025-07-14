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
                "docker", "docker-compose"
            )
        }

        // Desktop environment
        group("desktop") {
            install(
                "plasma-meta",
                "firefox"
            )
        }

        // Shell
        install("fish", "starship", "bat")
    }

    // Service configuration
    services {
        enable("NetworkManager")
        enable("sshd") {
            autoRestart = true
            restartOnFailure = true
        }
        enable("docker")

        // Disable unnecessary services
        disable("bluetooth")
    }

    // User management
    users {
        user("admin") {
            uid = 1000
            shell = "/usr/bin/fish"
            groups("wheel", "docker", "video", "audio")
        }
    }

    // Repository configuration
    repositories {
        add("core", "https://mirror.archlinux.org/core/os/x86_64") {
            priority = 10
        }

        add("extra", "https://mirror.archlinux.org/extra/os/x86_64") {
            priority = 20
        }
    }
}
