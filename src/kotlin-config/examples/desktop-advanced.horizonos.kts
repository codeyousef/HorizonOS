#!/usr/bin/env kotlin

import org.horizonos.config.dsl.*
import kotlin.time.Duration.Companion.GB
import kotlin.time.Duration.Companion.seconds

/**
 * Advanced Desktop Configuration with Automation
 * 
 * This example demonstrates a feature-rich desktop configuration with:
 * - Development environment setup
 * - Automation workflows for daily tasks
 * - Security hardening
 * - Performance optimizations
 */

horizonOS {
    // System configuration
    hostname = "powerstation"
    timezone = "America/Chicago"
    locale = "en_US.UTF-8"
    
    // Comprehensive package setup
    packages {
        // System essentials
        install("base-devel", "linux-headers", "linux-lts", "linux-lts-headers")
        install("intel-ucode", "nvidia", "nvidia-settings", "nvidia-utils")
        
        // Development environment
        group("development") {
            install("git", "git-lfs", "github-cli", "gitlab-runner")
            install("neovim", "visual-studio-code-bin", "sublime-text-4")
            install("docker", "docker-compose", "podman", "buildah")
            install("rustup", "go", "nodejs", "npm", "yarn", "pnpm")
            install("python", "python-pip", "python-poetry", "pyenv")
            install("jdk-openjdk", "kotlin", "gradle", "maven")
            install("clang", "cmake", "ninja", "meson")
            install("terraform", "ansible", "vagrant")
        }
        
        // Productivity tools
        group("productivity") {
            install("firefox", "chromium", "brave-bin")
            install("thunderbird", "evolution")
            install("libreoffice-fresh", "onlyoffice-bin")
            install("obsidian", "logseq", "notion-app")
            install("bitwarden", "keepassxc")
            install("nextcloud-client", "syncthing")
        }
        
        // Multimedia
        group("multimedia") {
            install("mpv", "vlc", "celluloid")
            install("spotify", "strawberry", "cmus")
            install("obs-studio", "kdenlive", "davinci-resolve")
            install("gimp", "inkscape", "blender")
        }
        
        // System utilities
        group("utilities") {
            install("htop", "btop", "nvtop", "iotop")
            install("ncdu", "duf", "dust")
            install("bat", "exa", "ripgrep", "fd", "fzf")
            install("tmux", "zellij", "screen")
            install("rsync", "rclone", "restic")
            install("ventoy-bin", "etcher-bin")
        }
        
        // Security tools
        group("security") {
            install("firejail", "apparmor")
            install("ufw", "fail2ban")
            install("rkhunter", "clamav")
            install("wireshark-qt", "nmap", "tcpdump")
        }
        
        // Remove unwanted packages
        remove("nano")  // vim/neovim is preferred
    }
    
    // Service configuration with security focus
    services {
        // Essential services
        enable("NetworkManager")
        enable("systemd-resolved")
        enable("systemd-timesyncd")
        
        // Development services
        enable("docker") {
            autoRestart = true
            environment["DOCKER_OPTS"] = "--log-level=warn"
        }
        enable("libvirtd")
        
        // Security services
        enable("apparmor")
        enable("fail2ban") {
            autoRestart = true
            restartOnFailure = true
        }
        enable("ufw")
        enable("clamav-freshclam")  // Auto-update virus definitions
        
        // Desktop services
        enable("bluetooth")
        enable("cups")  // Printing
        enable("avahi-daemon")  // Network discovery
        
        // Performance services
        enable("earlyoom")  // Prevent OOM freezes
        enable("systemd-oomd")
        enable("irqbalance")
        
        // Backup service
        enable("restic-backup.timer")
        
        // Optional services
        enable("sshd") {
            environment["PORT"] = "2222"  // Non-standard port for security
        }
        enable("syncthing@mainuser")
        
        // Disabled for security/performance
        disable("telnet")
        disable("rpcbind")
        disable("nfs-server")
    }
    
    // User management with groups
    users {
        user("mainuser") {
            uid = 1000
            shell = "/usr/bin/fish"
            groups(
                "wheel",      // sudo access
                "video",      // GPU access
                "audio",      // Audio devices
                "input",      // Input devices
                "docker",     // Docker without sudo
                "libvirt",    // Virtualization
                "wireshark",  // Network analysis
                "vboxusers"   // VirtualBox
            )
        }
        
        // Service account for backups
        user("backup") {
            uid = 999
            shell = "/bin/false"  // No login
            homeDir = "/var/lib/backup"
        }
    }
    
    // Repository configuration
    repositories {
        // Arch Linux repositories
        add("core", "https://mirror.rackspace.com/archlinux/core/os/x86_64") {
            priority = 1
        }
        add("extra", "https://mirror.rackspace.com/archlinux/extra/os/x86_64") {
            priority = 2
        }
        add("multilib", "https://mirror.rackspace.com/archlinux/multilib/os/x86_64") {
            priority = 3
        }
        
        // AUR helper repository
        add("chaotic-aur", "https://cdn-mirror.chaotic.cx/chaotic-aur/x86_64") {
            gpgCheck = true
            priority = 10
        }
        
        // HorizonOS OSTree
        ostree("horizonos", "https://ostree.horizonos.org") {
            branch("stable", "testing", "unstable")
            gpgVerify = true
            gpgKeyUrl = "https://horizonos.org/keys/ostree-signing-key.asc"
        }
    }
    
    // Desktop environment with customization
    desktop {
        environment = DesktopEnvironment.HYPRLAND
        autoLogin = false  // Security: require password
        
        hyprland {
            theme = "catppuccin-mocha"
            animations = true
            gaps = 12
            borderSize = 3
            rounding = 10
            blur = true
            shadows = true
            
            // Multi-monitor setup
            monitors {
                monitor("DP-1") {
                    resolution = "3440x1440"
                    refreshRate = 144
                    position = "0x0"
                    scale = 1.0
                    primary = true
                }
                monitor("HDMI-A-1") {
                    resolution = "1920x1080"
                    refreshRate = 60
                    position = "3440x180"
                    scale = 1.0
                }
            }
            
            workspaces = 10
            kdeIntegration = true
            personalityMode = PersonalityMode.MACOS
        }
    }
    
    // Comprehensive automation setup
    automation {
        // System maintenance workflow
        workflow("system-maintenance") {
            description = "Comprehensive system maintenance"
            priority = 100
            
            trigger {
                time("03:00")
                onDays(SUNDAY, WEDNESDAY)
            }
            
            conditions {
                systemIdle(30.minutes)
                batteryLevel(50.percent)  // If on laptop
            }
            
            actions {
                // Update mirror list for fastest servers
                runCommand("reflector --latest 20 --protocol https --sort rate --save /etc/pacman.d/mirrorlist")
                
                // System updates
                runCommand("pacman -Syu --noconfirm")
                runCommand("yay -Sua --noconfirm")  // AUR updates
                
                // Clean package cache (keep 2 versions)
                runCommand("paccache -rk2")
                runCommand("paccache -ruk0")  // Remove uninstalled packages
                
                // Clean systemd journal (keep 2 weeks)
                runCommand("journalctl --vacuum-time=2weeks")
                
                // Docker cleanup
                runCommand("docker system prune -af --volumes")
                runCommand("docker image prune -af")
                
                // Trim SSD
                runCommand("fstrim -av")
                
                // Update virus definitions
                runCommand("freshclam")
                
                // Check for rootkits
                runCommand("rkhunter --update")
                runCommand("rkhunter --check --skip-keypress")
                
                notification("Maintenance Complete", "System maintenance completed successfully", NotificationUrgency.LOW)
            }
            
            onError {
                notification("Maintenance Failed", "System maintenance encountered errors", NotificationUrgency.HIGH)
                email("admin@example.com", "Maintenance Failure", "Check system logs")
            }
        }
        
        // Security monitoring
        workflow("security-monitor") {
            description = "Monitor system security"
            
            trigger {
                interval(1.hours)
            }
            
            actions {
                // Check for failed login attempts
                scriptBlock {
                    """
                    FAILED_LOGINS=$(journalctl --since "1 hour ago" | grep -c "authentication failure")
                    if [ ${'$'}FAILED_LOGINS -gt 10 ]; then
                        notify-send -u critical "Security Alert" "${'$'}FAILED_LOGINS failed login attempts"
                    fi
                    """
                }
                
                // Check for unusual network connections
                scriptBlock {
                    """
                    SUSPICIOUS=$(ss -tunap | grep ESTABLISHED | grep -vE ':(22|80|443|3000|8080) ')
                    if [ ! -z "${'$'}SUSPICIOUS" ]; then
                        notify-send -u critical "Network Alert" "Unusual network connections detected"
                    fi
                    """
                }
            }
        }
        
        // Development environment setup
        workflow("dev-environment") {
            description = "Set up development environment on boot"
            
            trigger {
                systemEvent(SystemEvent.BOOT_COMPLETE)
                delay(30.seconds)  // Wait for system to settle
            }
            
            actions {
                // Start development services
                runCommand("systemctl --user start docker-desktop")
                runCommand("systemctl --user start podman.socket")
                
                // Mount development drives
                runCommand("udisksctl mount -b /dev/disk/by-label/PROJECTS || true")
                
                // Start tmux sessions
                scriptBlock {
                    """
                    tmux new-session -d -s main
                    tmux new-session -d -s docker 'docker ps -a'
                    tmux new-session -d -s logs 'journalctl -f'
                    """
                }
                
                // Warm up development tools
                runCommand("code --list-extensions > /dev/null")  // Pre-load VS Code
                runCommand("docker pull alpine:latest")  // Ensure base image is available
            }
        }
        
        // Backup automation
        workflow("incremental-backup") {
            description = "Incremental backup of important data"
            
            trigger {
                time("22:00")
                onDays(DAILY)
            }
            
            conditions {
                pathExists("/mnt/backup")
                diskSpaceAvailable("/mnt/backup", 100.GB)
            }
            
            actions {
                // Create snapshot tag
                variable("SNAPSHOT_TAG", "daily-$(date +%Y%m%d)")
                
                // Backup using restic
                runCommand("""
                    restic -r /mnt/backup/restic-repo backup \
                        --tag ${'$'}SNAPSHOT_TAG \
                        --exclude-caches \
                        --exclude /home/mainuser/.cache \
                        --exclude /home/mainuser/.local/share/Trash \
                        /home/mainuser/Documents \
                        /home/mainuser/Projects \
                        /home/mainuser/.config \
                        /home/mainuser/.ssh
                """)
                
                // Verify backup
                runCommand("restic -r /mnt/backup/restic-repo check")
                
                // Clean old snapshots (keep 7 daily, 4 weekly, 12 monthly)
                runCommand("restic -r /mnt/backup/restic-repo forget --keep-daily 7 --keep-weekly 4 --keep-monthly 12 --prune")
                
                notification("Backup Complete", "Daily backup completed successfully")
            }
            
            onError {
                notification("Backup Failed", "Backup encountered errors - check logs", NotificationUrgency.CRITICAL)
            }
        }
        
        // Screenshot organization
        workflow("organize-screenshots") {
            description = "Organize screenshots by date"
            
            trigger {
                fileCreated("~/Pictures/Screenshots/*.png")
                fileCreated("~/Pictures/Screenshots/*.jpg")
            }
            
            actions {
                fileOperation {
                    move {
                        from = "~/Pictures/Screenshots/*"
                        to = "~/Pictures/Screenshots/{{date:yyyy}}/{{date:MM-MMMM}}/{{filename}}"
                        createTargetDir = true
                    }
                }
            }
        }
        
        // Meeting assistant
        workflow("meeting-prep") {
            description = "Prepare for video meetings"
            
            trigger {
                calendarEvent("meeting", 5.minutes.before)
            }
            
            actions {
                // Mute audio
                runCommand("pactl set-sink-mute @DEFAULT_SINK@ 1")
                
                // Close distracting applications
                runCommand("pkill -f discord || true")
                runCommand("pkill -f spotify || true")
                
                // Set do not disturb
                runCommand("dunstctl set-paused true")
                
                // Open meeting app
                browserOpen("https://meet.google.com")
                
                notification("Meeting Starting", "Your meeting starts in 5 minutes")
            }
        }
        
        // Teaching mode for repetitive tasks
        teach("code-review-process") {
            description = "Learn code review workflow"
            watchPath = "~/Projects"
            watchApplications = listOf("firefox", "vscode", "terminal")
            
            onLearned { workflow ->
                // System creates a workflow based on observed actions
                workflow.trigger {
                    fileModified("~/Projects/*/pull-request.md")
                }
            }
        }
    }
}