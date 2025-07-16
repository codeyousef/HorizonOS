#!/usr/bin/env kotlin

import org.horizonos.config.dsl.*
import kotlin.time.Duration.Companion.minutes
import kotlin.time.Duration.Companion.GB

/**
 * Gaming-Focused Configuration
 * 
 * Optimized for gaming performance with:
 * - GPU drivers and optimization
 * - Gaming platforms and tools
 * - Performance tuning
 * - RGB control
 */

horizonOS {
    // System configuration
    hostname = "gaming-rig"
    timezone = "America/Los_Angeles"
    locale = "en_US.UTF-8"
    
    // Gaming-optimized packages
    packages {
        // Graphics drivers (choose based on your GPU)
        group("nvidia-graphics") {
            install("nvidia", "nvidia-utils", "nvidia-settings")
            install("lib32-nvidia-utils")  // 32-bit support for older games
            install("nvidia-prime")  // For laptops with hybrid graphics
            install("nvtop")  // GPU monitoring
        }
        
        // Alternative for AMD users (comment out nvidia group above)
        // group("amd-graphics") {
        //     install("mesa", "lib32-mesa", "vulkan-radeon", "lib32-vulkan-radeon")
        //     install("libva-mesa-driver", "lib32-libva-mesa-driver")
        //     install("radeontop")  // GPU monitoring
        // }
        
        // Gaming platforms
        group("gaming-platforms") {
            install("steam", "steam-native-runtime")
            install("lutris")  // Multi-platform gaming
            install("heroic-games-launcher-bin")  // Epic & GOG
            install("bottles")  // Windows app management
            install("itch")  // Indie games
        }
        
        // Gaming tools and utilities
        group("gaming-tools") {
            install("gamemode", "lib32-gamemode")  // Performance optimization
            install("mangohud", "lib32-mangohud")  // In-game overlay
            install("goverlay-bin")  // MangoHud GUI
            install("corectrl")  // GPU/CPU control
            install("piper")  // Gaming mouse configuration
            install("solaar")  // Logitech device manager
        }
        
        // Wine and Windows compatibility
        group("wine-compat") {
            install("wine-staging", "wine-gecko", "wine-mono")
            install("winetricks")
            install("dxvk-bin")  // DirectX to Vulkan
            install("vkd3d")  // Direct3D 12
            install("lib32-vkd3d")
        }
        
        // Performance tools
        group("performance") {
            install("cpupower")  // CPU frequency scaling
            install("irqbalance")  // IRQ distribution
            install("earlyoom")  // Prevent OOM freezes
            install("zram-generator")  // Compressed swap
            install("ananicy-cpp")  // Auto nice daemon
        }
        
        // Audio for gaming
        group("gaming-audio") {
            install("pipewire", "pipewire-alsa", "pipewire-pulse", "pipewire-jack")
            install("lib32-pipewire")
            install("easyeffects")  // Audio effects
            install("noise-suppression-for-voice")  // AI noise suppression
        }
        
        // Streaming and recording
        group("streaming") {
            install("obs-studio")
            install("obs-vkcapture")  // Vulkan game capture
            install("obs-gstreamer")
            install("v4l2loopback-dkms")  // Virtual camera
        }
        
        // Communication
        install("discord", "teamspeak3")
        install("mumble")
        
        // RGB control
        install("openrgb")
        install("ckb-next")  // Corsair devices
        install("razergenie")  // Razer devices
        
        // System monitoring
        install("btop")  // Beautiful process monitor
        install("nvtop")  // GPU monitor
        install("radeontop")  // AMD GPU monitor
        
        // Media and entertainment
        install("mpv")  // Lightweight video player
        install("spotify")
    }
    
    // Gaming-optimized services
    services {
        // Essential services
        enable("NetworkManager")
        enable("systemd-resolved")
        
        // Performance services
        enable("gamemode") {
            autoRestart = true
        }
        enable("irqbalance")
        enable("earlyoom") {
            environment["EARLYOOM_ARGS"] = "-r 3600 -m 5 -s 10"
        }
        enable("ananicy-cpp")
        
        // CPU performance
        enable("cpupower") {
            environment["governor"] = "performance"  // Maximum performance
        }
        
        // RGB services
        enable("openrgb")
        enable("ckb-next-daemon")
        
        // Audio
        enable("pipewire")
        enable("pipewire-pulse")
        enable("wireplumber")
        
        // Steam integration
        enable("steam-devices")  // Controller support
        
        // Optional services
        enable("bluetooth")  // For wireless controllers
        enable("cups")  // Printing support
        
        // Disable unnecessary services for performance
        disable("updatedb")  // File indexing
        disable("man-db.timer")  // Manual page updates
    }
    
    // User configuration
    users {
        user("gamer") {
            uid = 1000
            shell = "/usr/bin/bash"
            groups(
                "wheel",     // Admin access
                "video",     // GPU access
                "audio",     // Audio devices
                "input",     // Controllers
                "gamemode",  // GameMode access
                "realtime"   // Real-time scheduling
            )
        }
    }
    
    // Desktop environment for gaming
    desktop {
        environment = DesktopEnvironment.PLASMA  // Good gaming support
        autoLogin = true  // Quick boot to gaming
        autoLoginUser = "gamer"
        
        plasma {
            theme = "breeze-dark"
            lookAndFeel = "org.kde.breezedark.desktop"
            
            // Disable compositing for better performance
            // Will be toggled by GameMode automatically
            compositingEnabled = true
            
            // Single click to open (faster navigation)
            singleClick = true
        }
    }
    
    // Gaming automation workflows
    automation {
        // Game mode activation
        workflow("auto-gamemode") {
            description = "Automatically enable game mode for games"
            
            trigger {
                processStarted("steam", "lutris", "heroic", "wine")
            }
            
            actions {
                // Enable performance mode
                runCommand("cpupower frequency-set -g performance")
                
                // Disable compositor
                runCommand("qdbus org.kde.KWin /Compositor suspend")
                
                // Set GPU to performance mode
                runCommand("nvidia-settings -a '[gpu:0]/GpuPowerMizerMode=1'")
                
                // Kill unnecessary processes
                runCommand("pkill -f dropbox || true")
                runCommand("pkill -f nextcloud || true")
                runCommand("pkill -f syncthing || true")
                
                notification("Game Mode", "Performance optimizations activated")
            }
        }
        
        // Restore normal mode
        workflow("exit-gamemode") {
            description = "Restore normal system settings"
            
            trigger {
                processExited("steam", "lutris", "heroic", "wine")
                delay(30.seconds)  // Wait to ensure game is closed
            }
            
            conditions {
                processNotRunning("*.exe")  // No Windows executables
            }
            
            actions {
                // Restore balanced CPU mode
                runCommand("cpupower frequency-set -g schedutil")
                
                // Re-enable compositor
                runCommand("qdbus org.kde.KWin /Compositor resume")
                
                // Set GPU to auto mode
                runCommand("nvidia-settings -a '[gpu:0]/GpuPowerMizerMode=0'")
                
                notification("Normal Mode", "System restored to balanced settings")
            }
        }
        
        // GPU driver updates
        workflow("gpu-driver-check") {
            description = "Check for GPU driver updates"
            
            trigger {
                time("10:00")
                onDays(SATURDAY)  // Weekly check
            }
            
            actions {
                scriptBlock {
                    """
                    # Check for NVIDIA driver updates
                    CURRENT=$(nvidia-smi --query-gpu=driver_version --format=csv,noheader)
                    AVAILABLE=$(pacman -Si nvidia | grep Version | awk '{print $3}')
                    
                    if [ "${'$'}CURRENT" != "${'$'}AVAILABLE" ]; then
                        notify-send "GPU Driver Update" "New NVIDIA driver available: ${'$'}AVAILABLE"
                    fi
                    """
                }
            }
        }
        
        // Steam cache cleanup
        workflow("steam-cleanup") {
            description = "Clean Steam shader cache and downloads"
            
            trigger {
                diskSpaceBelow("/", 20.GB)
            }
            
            actions {
                // Clean shader cache
                runCommand("rm -rf ~/.local/share/Steam/steamapps/shadercache/*")
                
                // Clean download cache
                runCommand("rm -rf ~/.local/share/Steam/steamapps/downloading/*")
                
                // Clean old Proton versions
                scriptBlock {
                    """
                    # Keep only the 3 newest Proton versions
                    cd ~/.local/share/Steam/steamapps/common
                    ls -dt Proton* | tail -n +4 | xargs rm -rf
                    """
                }
                
                notification("Steam Cleanup", "Freed up disk space")
            }
        }
        
        // RGB synchronization
        workflow("rgb-sync") {
            description = "Synchronize RGB across devices"
            
            trigger {
                systemEvent(SystemEvent.USER_LOGIN)
            }
            
            actions {
                // Set all RGB to gaming theme
                runCommand("openrgb --mode static --color FF0000")  // Red theme
                
                // Sync keyboard if Corsair
                runCommand("ckb-next-cmd --mode wave --color1 FF0000 --color2 000000")
                
                // Sync mouse if Razer
                runCommand("razercfg -l all:FF0000")
            }
        }
        
        // Performance monitoring
        workflow("performance-log") {
            description = "Log gaming performance metrics"
            
            trigger {
                processRunning("steam", "lutris")
                interval(5.minutes)
            }
            
            actions {
                scriptBlock {
                    """
                    # Log system metrics
                    echo "$(date): CPU: $(grep 'cpu MHz' /proc/cpuinfo | head -1 | awk '{print $4}') MHz" >> ~/gaming-performance.log
                    echo "$(date): GPU: $(nvidia-smi --query-gpu=utilization.gpu --format=csv,noheader,nounits)%" >> ~/gaming-performance.log
                    echo "$(date): RAM: $(free -h | grep Mem | awk '{print $3 "/" $2}')" >> ~/gaming-performance.log
                    echo "$(date): Temp: $(nvidia-smi --query-gpu=temperature.gpu --format=csv,noheader,nounits)°C" >> ~/gaming-performance.log
                    """
                }
            }
        }
        
        // Discord optimization
        workflow("discord-optimizer") {
            description = "Optimize Discord for gaming"
            
            trigger {
                processStarted("discord")
            }
            
            actions {
                delay(10.seconds)  // Let Discord start
                
                // Disable Discord hardware acceleration (can cause issues)
                scriptBlock {
                    """
                    sed -i 's/"SKIP_HOST_UPDATE":false/"SKIP_HOST_UPDATE":true/' ~/.config/discord/settings.json
                    sed -i 's/"HARDWARE_ACCELERATION":true/"HARDWARE_ACCELERATION":false/' ~/.config/discord/settings.json
                    """
                }
                
                // Set Discord to use less resources
                runCommand("renice -n 10 $(pgrep Discord)")
            }
        }
    }
}