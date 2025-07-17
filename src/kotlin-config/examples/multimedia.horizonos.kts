#!/usr/bin/env kotlin

@file:Repository("https://repo1.maven.org/maven2/")
@file:DependsOn("org.horizonos:kotlin-config:1.0")

import org.horizonos.config.dsl.*

/**
 * HorizonOS Multimedia Configuration
 * 
 * This configuration creates a multimedia-focused system with:
 * - Professional audio and video editing tools
 * - Container-based multimedia environments
 * - High-quality audio and video support
 * - Graphics design applications
 * - Streaming and content creation tools
 */

horizonOS {
    // Basic system configuration
    hostname = "horizon-multimedia"
    timezone = "America/New_York"
    locale = "en_US.UTF-8"
    
    // Container-based multimedia environments
    containers {
        defaultRuntime = ContainerRuntime.DISTROBOX
        
        // Audio production container
        container("audio-production") {
            image = "archlinux/archlinux"
            runtime = ContainerRuntime.DISTROBOX
            purpose = ContainerPurpose.MULTIMEDIA
            packages("audacity", "ardour", "reaper", "lmms", "hydrogen")
            packages("sox", "lame", "flac", "opus-tools", "vorbis-tools")
            packages("jack2", "alsa-utils", "pulseaudio", "pavucontrol")
            export("audacity", "ardour", "lmms", "hydrogen", "sox")
            
            mount("/dev/snd")
            mount("/home/user/Audio")
            mount("/home/user/.config/audacity")
            mount("/home/user/.config/ardour6")
            
            env("JACK_DEFAULT_SERVER", "default")
            env("AUDIO_SAMPLE_RATE", "48000")
            env("AUDIO_BUFFER_SIZE", "256")
            
            privileged = true
            
            label("multimedia.type", "audio")
            label("multimedia.professional", "true")
        }
        
        // Video editing container
        container("video-editing") {
            image = "archlinux/archlinux"
            runtime = ContainerRuntime.DISTROBOX
            purpose = ContainerPurpose.MULTIMEDIA
            packages("kdenlive", "openshot", "blender", "davinci-resolve")
            packages("ffmpeg", "x264", "x265", "vpx", "aom")
            packages("mediainfo", "mkvtoolnix", "handbrake")
            export("kdenlive", "openshot", "blender", "ffmpeg", "mediainfo")
            
            mount("/dev/dri")
            mount("/home/user/Videos")
            mount("/home/user/.config/kdenlive")
            mount("/home/user/.local/share/kdenlive")
            
            env("FFMPEG_HWACCEL", "vaapi")
            env("BLENDER_PYTHON", "/usr/bin/python")
            env("CUDA_VISIBLE_DEVICES", "0")
            
            privileged = true
            
            label("multimedia.type", "video")
            label("multimedia.professional", "true")
        }
        
        // Graphics design container
        container("graphics-design") {
            image = "archlinux/archlinux"
            runtime = ContainerRuntime.DISTROBOX
            purpose = ContainerPurpose.MULTIMEDIA
            packages("gimp", "inkscape", "krita", "scribus", "blender")
            packages("imagemagick", "graphicsmagick", "darktable", "rawtherapee")
            packages("fontforge", "potrace", "libheif", "libjxl")
            export("gimp", "inkscape", "krita", "scribus", "blender")
            
            mount("/dev/dri")
            mount("/home/user/Graphics")
            mount("/home/user/.config/GIMP")
            mount("/home/user/.config/inkscape")
            mount("/home/user/.config/krita")
            
            env("GIMP_DIRECTORY", "/home/user/.config/GIMP")
            env("INKSCAPE_PROFILE_DIR", "/home/user/.config/inkscape")
            
            label("multimedia.type", "graphics")
            label("multimedia.professional", "true")
        }
        
        // Streaming and recording container
        container("streaming-tools") {
            image = "archlinux/archlinux"
            runtime = ContainerRuntime.DISTROBOX
            purpose = ContainerPurpose.MULTIMEDIA
            packages("obs-studio", "ffmpeg", "v4l2loopback-dkms")
            packages("gstreamer", "gst-plugins-good", "gst-plugins-bad", "gst-plugins-ugly")
            packages("streamlink", "youtube-dl", "yt-dlp")
            export("obs", "ffmpeg", "streamlink", "youtube-dl", "yt-dlp")
            
            mount("/dev/dri")
            mount("/dev/video0")
            mount("/dev/snd")
            mount("/home/user/Streaming")
            mount("/home/user/.config/obs-studio")
            
            env("OBS_USE_EGL", "1")
            env("GST_PLUGIN_SYSTEM_PATH", "/usr/lib/gstreamer-1.0")
            
            port("1935:1935")  # RTMP
            port("8080:8080")  # HTTP streaming
            
            privileged = true
            
            label("multimedia.type", "streaming")
            label("multimedia.live", "true")
        }
        
        // 3D and animation container
        container("3d-animation") {
            image = "archlinux/archlinux"
            runtime = ContainerRuntime.DISTROBOX
            purpose = ContainerPurpose.MULTIMEDIA
            packages("blender", "freecad", "wings3d", "meshlab")
            packages("povray", "yafaray", "luxrender")
            packages("opencl-headers", "cuda", "optix")
            export("blender", "freecad", "wings3d", "meshlab", "povray")
            
            mount("/dev/dri")
            mount("/home/user/3D")
            mount("/home/user/.config/blender")
            mount("/home/user/.config/freecad")
            
            env("BLENDER_USER_SCRIPTS", "/home/user/.config/blender/scripts")
            env("CUDA_VISIBLE_DEVICES", "0")
            env("OPENCL_VENDOR_PATH", "/etc/OpenCL/vendors")
            
            privileged = true
            
            label("multimedia.type", "3d")
            label("multimedia.rendering", "true")
        }
        
        globalMount("/home/user/Multimedia")
        globalMount("/home/user/.config/multimedia")
    }
    
    // Multimedia-optimized layers
    layers {
        base {
            minimal = false
            packages("base", "linux-rt", "systemd", "podman", "flatpak")
            packages("pipewire", "pipewire-jack", "pipewire-pulse", "pipewire-alsa")
            packages("mesa", "vulkan-drivers", "opencl-drivers")
            services("systemd-networkd", "systemd-resolved", "podman.socket")
            services("pipewire", "pipewire-pulse", "rtkit")
            commit("multimedia-base")
        }
        
        systemLayer("audio-core", LayerPurpose.MULTIMEDIA) {
            priority = 10
            autoStart = true
            strategy = LayerStrategy.ALWAYS_ON
            
            multimedia {
                packages("pipewire", "pipewire-jack", "pipewire-pulse")
                packages("jack2", "alsa-utils", "rtkit")
                export("pw-jack", "jack_control", "alsamixer")
            }
            
            healthCheck {
                command = "systemctl --user is-active pipewire"
                interval = "30s"
                timeout = "5s"
                retries = 3
            }
        }
        
        systemLayer("video-core", LayerPurpose.MULTIMEDIA) {
            priority = 20
            dependsOn("audio-core")
            
            container {
                image = "archlinux/archlinux"
                runtime = ContainerRuntime.DISTROBOX
                packages("ffmpeg", "gstreamer", "v4l-utils")
                packages("mesa", "vulkan-drivers", "vaapi-drivers")
                export("ffmpeg", "gst-launch-1.0", "v4l2-ctl")
                mount("/dev/dri")
                mount("/dev/video0")
                privileged = true
            }
        }
        
        systemLayer("graphics-core", LayerPurpose.MULTIMEDIA) {
            priority = 30
            dependsOn("video-core")
            
            container {
                image = "archlinux/archlinux"
                runtime = ContainerRuntime.DISTROBOX
                packages("mesa", "vulkan-drivers", "opencl-drivers")
                packages("imagemagick", "graphicsmagick", "exiv2")
                export("convert", "identify", "exiftool")
                mount("/dev/dri")
                privileged = true
            }
        }
        
        user {
            autoUpdates = true
            userScope = true
            
            // Professional multimedia applications
            flatpak("org.gimp.GIMP") {
                permissions("--filesystem=home", "--device=dri")
                allowFilesystem()
                allowDisplay()
            }
            
            flatpak("org.inkscape.Inkscape") {
                permissions("--filesystem=home", "--device=dri")
                allowFilesystem()
                allowDisplay()
            }
            
            flatpak("org.kde.krita") {
                permissions("--filesystem=home", "--device=dri")
                allowFilesystem()
                allowDisplay()
            }
            
            flatpak("org.blender.Blender") {
                permissions("--filesystem=home", "--device=all")
                allowFilesystem()
                allowDisplay()
            }
            
            flatpak("org.audacityteam.Audacity") {
                permissions("--filesystem=home", "--device=all")
                allowFilesystem()
                allowDisplay()
                allowAudio()
            }
            
            flatpak("org.kde.kdenlive") {
                permissions("--filesystem=home", "--device=all")
                allowFilesystem()
                allowDisplay()
                allowAudio()
            }
            
            flatpak("com.obsproject.Studio") {
                permissions("--filesystem=home", "--device=all")
                allowNetwork()
                allowFilesystem()
                allowDisplay()
                allowAudio()
            }
            
            flatpak("org.darktable.Darktable") {
                permissions("--filesystem=home", "--device=dri")
                allowFilesystem()
                allowDisplay()
            }
            
            flatpak("org.freecad.FreeCAD") {
                permissions("--filesystem=home", "--device=dri")
                allowFilesystem()
                allowDisplay()
            }
            
            // Media players
            flatpak("org.videolan.VLC") {
                permissions("--filesystem=home", "--device=all")
                allowFilesystem()
                allowDisplay()
                allowAudio()
            }
            
            flatpak("io.mpv.Mpv") {
                permissions("--filesystem=home", "--device=all")
                allowFilesystem()
                allowDisplay()
                allowAudio()
            }
            
            // Utilities
            flatpak("fr.handbrake.ghb") {
                permissions("--filesystem=home", "--device=dri")
                allowFilesystem()
                allowDisplay()
            }
            
            flatpak("org.gnome.SoundRecorder") {
                permissions("--filesystem=home", "--device=all")
                allowFilesystem()
                allowDisplay()
                allowAudio()
            }
        }
        
        order("base", "audio-core", "video-core", "graphics-core", "user")
        globalMount("/home/user/Multimedia")
        globalMount("/home/user/.config/multimedia")
        sharedVolume("multimedia-data")
    }
    
    // Multimedia-optimized package management
    enhancedPackages {
        autoMigrate = true
        migrationStrategy = MigrationStrategy.CONTAINER_FIRST
        
        system {
            strategy = SystemPackageStrategy.CONTAINER
            defaultRuntime = ContainerRuntime.DISTROBOX
            
            multimedia("audio-tools") {
                packages("audacity", "ardour", "lmms", "hydrogen")
                packages("sox", "lame", "flac", "opus-tools")
                export("audacity", "ardour", "lmms", "sox")
            }
            
            multimedia("video-tools") {
                packages("kdenlive", "openshot", "blender", "ffmpeg")
                packages("x264", "x265", "vpx", "mediainfo")
                export("kdenlive", "openshot", "blender", "ffmpeg")
            }
            
            multimedia("graphics-tools") {
                packages("gimp", "inkscape", "krita", "scribus")
                packages("imagemagick", "darktable", "rawtherapee")
                export("gimp", "inkscape", "krita", "convert")
            }
            
            globalMount("/home/user/Multimedia")
            autoUpdate = true
        }
        
        applications {
            strategy = ApplicationPackageStrategy.FLATPAK
            autoUpdate = true
            
            multimedia()
            
            popular("firefox", "thunderbird")
        }
    }
    
    // Reproducible multimedia environment
    reproducible {
        enabled = true
        strictMode = false
        verifyDigests = true
        
        ostree("horizonos/multimedia/x86_64", "media123def456") {
            version = "1.0-multimedia"
            digest = "sha256:media567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
            url = "https://ostree.horizonos.dev/repo"
        }
        
        systemImage {
            version = "1.0-multimedia"
            
            container("audio-production", "archlinux/archlinux", "sha256:audio-digest") {
                runtime = ContainerRuntime.DISTROBOX
                purpose = ContainerPurpose.MULTIMEDIA
                
                pkg("audacity", "3.3.3") {
                    checksum = "sha256:audacity-checksum"
                    dependencies("gtk3", "alsa-lib")
                }
                
                pkg("ardour", "7.5.0") {
                    checksum = "sha256:ardour-checksum"
                    dependencies("jack2", "lv2")
                }
            }
            
            flatpak("org.gimp.GIMP", "gimp-commit-hash") {
                version = "2.10.34"
                runtime = "org.gnome.Platform"
                runtimeVersion = "44"
            }
            
            metadata("audio-optimized", "true")
            metadata("video-optimized", "true")
            metadata("kernel", "linux-rt")
        }
    }
    
    // Multimedia-optimized desktop
    desktop {
        environment = DesktopEnvironment.PLASMA
        autoLogin = true
        autoLoginUser = "creator"
        
        plasma {
            theme = "breeze-dark"
            lookAndFeel = "org.kde.breezedark.desktop"
            widgets("org.kde.plasma.systemmonitor", "org.kde.plasma.mediacontroller")
        }
    }
    
    // Multimedia-optimized hardware
    hardware {
        cpu {
            enableTurboBoost = true
            governor = "performance"
            isolateCores = false
            realTimeScheduling = true
        }
        
        memory {
            swappiness = 10
            hugepages = true
            transparentHugepages = "madvise"
            overcommit = "never"
        }
        
        gpu {
            driver = "nvidia"
            enableCuda = true
            opengl = true
            vulkan = true
            opencl = true
            
            nvidia {
                persistenceMode = true
                powermizer = "prefer maximum performance"
                enableNvenc = true
                enableNvdec = true
            }
            
            amd {
                enableVaapi = true
                enableVce = true
                enableUvd = true
            }
        }
        
        audio {
            server = AudioServer.PIPEWIRE
            enableLowLatency = true
            sampleRate = 48000
            bufferSize = 256
            periods = 2
            
            pipewire {
                quantumSize = 256
                minQuantum = 32
                maxQuantum = 8192
                enableRealtime = true
                realtimePriority = 20
            }
            
            jack {
                enableRealtime = true
                realtimePriority = 70
                memoryLock = true
                portMax = 256
            }
        }
        
        storage {
            scheduler = "none"
            readAhead = 4096
            enableTrim = true
            
            multimedia {
                optimizeForLargeFiles = true
                enableDirectIO = true
                bufferSize = "16MB"
            }
        }
        
        usb {
            autosuspend = false
            
            multimedia {
                enableUsbAudio = true
                enableMidiDevices = true
                enableVideoDevices = true
            }
        }
    }
    
    // Multimedia-focused security
    security {
        firewall {
            enabled = true
            defaultPolicy = "DROP"
            
            // Streaming ports
            allowPort(1935, "tcp")     # RTMP
            allowPort(8080, "tcp")     # HTTP streaming
            allowPort(8554, "tcp")     # RTSP
            allowPort(5004, "udp")     # RTP
            allowPort(5005, "udp")     # RTCP
            
            // File sharing
            allowPort(21, "tcp")       # FTP
            allowPort(22, "tcp")       # SSH/SFTP
            allowPort(80, "tcp")       # HTTP
            allowPort(443, "tcp")      # HTTPS
            
            // Network rendering
            allowPortRange(3000, 3010, "tcp")  # Blender network rendering
            allowPortRange(8000, 8010, "tcp")  # Custom render farm
        }
        
        selinux {
            enabled = false  # Disabled for multimedia compatibility
        }
        
        apparmor {
            enabled = true
            mode = "complain"
        }
        
        multimedia {
            allowRealtime = true
            allowMemoryLock = true
            allowRawIO = true
            enableHardwareAccess = true
        }
    }
    
    // Multimedia-optimized network
    network {
        hostname = "horizon-multimedia"
        
        interfaces {
            ethernet("enp0s3") {
                dhcp = true
                mtu = 9000  # Jumbo frames for large file transfers
                
                multimedia {
                    enableQos = true
                    priorityClass = "multimedia"
                    bufferSize = "large"
                }
            }
            
            wifi("wlan0") {
                dhcp = true
                powersave = false
                
                multimedia {
                    enableQos = true
                    priorityClass = "multimedia"
                }
            }
        }
        
        dns {
            servers("1.1.1.1", "8.8.8.8")
            searchDomains("local.media")
        }
        
        multimedia {
            enableStreamingOptimization = true
            bufferBloatMitigation = true
            prioritizeMultimedia = true
        }
    }
    
    // Multimedia-optimized storage
    storage {
        filesystem = "btrfs"
        
        subvolumes {
            subvolume("@") {
                mountpoint = "/"
                options = "defaults,noatime,compress=zstd:1"
            }
            
            subvolume("@home") {
                mountpoint = "/home"
                options = "defaults,noatime,compress=zstd:1"
            }
            
            subvolume("@multimedia") {
                mountpoint = "/home/user/Multimedia"
                options = "defaults,noatime,compress=none"  # No compression for media files
            }
            
            subvolume("@cache") {
                mountpoint = "/home/user/.cache"
                options = "defaults,noatime,compress=zstd:3"
            }
        }
        
        multimedia {
            enableFastSeek = true
            optimizeForLargeFiles = true
            cacheSize = "1GB"
            enablePreallocation = true
        }
    }
    
    // Multimedia services
    services {
        enable("pipewire") {
            autoRestart = true
            env("PIPEWIRE_LATENCY", "256/48000")
            env("PIPEWIRE_QUANTUM", "256")
        }
        
        enable("pipewire-pulse") {
            autoRestart = true
        }
        
        enable("pipewire-jack") {
            autoRestart = true
            env("JACK_DEFAULT_SERVER", "pipewire")
        }
        
        enable("rtkit") {
            autoRestart = true
        }
        
        enable("udev") {
            autoRestart = true
            env("UDEV_RULES_D", "/etc/udev/rules.d")
        }
        
        disable("bluetooth")  # Reduce audio latency
        disable("cups")       # Not needed for multimedia
    }
    
    // Multimedia user configuration
    users {
        user("creator") {
            uid = 1000
            shell = "/usr/bin/fish"
            groups("wheel", "audio", "video", "input", "optical", "storage")
            homeDir = "/home/creator"
        }
        
        user("editor") {
            shell = "/usr/bin/bash"
            groups("audio", "video", "input", "optical")
        }
    }
    
    // Multimedia repositories
    repositories {
        add("archlinux-multimedia", "https://mirror.pkgbuild.com/extra/os/x86_64") {
            enabled = true
            priority = 20
        }
        
        add("packman", "https://ftp.gwdg.de/pub/linux/misc/packman/suse/openSUSE_Tumbleweed/") {
            enabled = true
            priority = 30
        }
        
        ostree("horizonos-multimedia", "https://ostree.horizonos.dev/repo") {
            enabled = true
            priority = 5
            branch("horizonos/multimedia/x86_64")
        }
    }
}