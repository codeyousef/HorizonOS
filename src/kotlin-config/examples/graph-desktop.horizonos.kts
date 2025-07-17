import org.horizonos.config.dsl.*
import org.horizonos.config.dsl.graphdesktop.*
import org.horizonos.config.dsl.graphdesktop.nodes.*
import org.horizonos.config.dsl.graphdesktop.edges.*
import org.horizonos.config.dsl.graphdesktop.layout.*
import org.horizonos.config.dsl.graphdesktop.interaction.*
import org.horizonos.config.dsl.graphdesktop.visual.*
import org.horizonos.config.dsl.graphdesktop.ai.*
import org.horizonos.config.dsl.graphdesktop.workspace.*

horizonOS {
    // System configuration
    hostname = "horizonos-graph"
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

        // Graph desktop dependencies
        group("graph-desktop") {
            install(
                "wayland", "wayland-protocols",
                "libinput", "mesa", "vulkan-icd-loader",
                "gtk4", "qt6-wayland",
                "pipewire", "wireplumber"
            )
        }

        // Development tools for graph desktop
        group("development") {
            install(
                "rust", "cargo",
                "git", "cmake", "meson",
                "pkgconf", "clang"
            )
        }

        // AI integration
        group("ai") {
            install("ollama")
        }
    }

    // Graph Desktop Configuration (Flagship Feature)
    graphDesktop {
        enabled = true
        renderingEngine = RenderingEngine.WEBGPU
        enablePhysics = true
        enableGestures = true
        enableKeyboardNavigation = true
        enableVoiceControl = false // Can be enabled with proper hardware
        maxNodes = 10000
        maxEdges = 50000
        performanceMode = PerformanceMode.BALANCED

        // Define node types with visual and behavioral properties
        nodeType("application") {
            displayName = "Application"
            description = "Running applications and programs"
            category = NodeCategory.APPLICATION
            icon = "window"
            color = "#4A90E2"
            shape = NodeShape.ROUNDED_RECTANGLE
            size = NodeSize.MEDIUM
            
            visual {
                useActualIcon = true
                glowOnActivity = true
                scaleWithImportance = true
            }
            
            behavior {
                doubleClickToLaunch = true
                enableContextMenu = true
                connectableAsSource = true
                connectableAsTarget = true
                contextMenuItems = listOf(
                    "Open", "Close", "Minimize", "Maximize",
                    "Move to Workspace", "Pin to Graph", "Properties"
                )
            }
            
            physics {
                mass = 2.0
                charge = -50.0
                friction = 0.85
                repulsionStrength = 120.0
            }
        }

        nodeType("file") {
            displayName = "File"
            description = "Files and documents"
            category = NodeCategory.FILE
            icon = "document"
            color = "#50C878"
            shape = NodeShape.ROUNDED_RECTANGLE
            size = NodeSize.SMALL
            
            visual {
                useFileTypeIcon = true
                showThumbnail = true
                showFileSize = true
            }
            
            behavior {
                doubleClickToOpen = true
                dragAndDropEnabled = true
                contextMenuItems = listOf(
                    "Open", "Open With", "Copy", "Move", "Delete",
                    "Rename", "Properties", "Share"
                )
            }
        }

        nodeType("person") {
            displayName = "Person"
            description = "Contacts and collaborators"
            category = NodeCategory.USER
            icon = "person"
            color = "#FF6B6B"
            shape = NodeShape.CIRCLE
            size = NodeSize.MEDIUM
            
            visual {
                showProfilePicture = true
                showPresenceIndicator = true
                glowOnMessage = true
            }
            
            behavior {
                doubleClickToMessage = true
                contextMenuItems = listOf(
                    "Send Message", "Start Call", "Share Screen",
                    "View Profile", "Recent Activity"
                )
            }
        }

        nodeType("task") {
            displayName = "Task"
            description = "Tasks and todos"
            category = NodeCategory.DATA
            icon = "checkbox"
            color = "#FFC107"
            shape = NodeShape.HEXAGON
            size = NodeSize.SMALL
            
            visual {
                showCompletionStatus = true
                showDueDate = true
                pulseWhenDue = true
            }
        }

        nodeType("ai-agent") {
            displayName = "AI Agent"
            description = "AI assistants and agents"
            category = NodeCategory.AI
            icon = "robot"
            color = "#9C27B0"
            shape = NodeShape.OCTAGON
            size = NodeSize.MEDIUM
            
            visual {
                animatedBorder = true
                glowEffect = true
            }
            
            behavior {
                interactiveChat = true
                contextMenuItems = listOf(
                    "Ask Question", "Request Analysis",
                    "Train on Data", "View History"
                )
            }
        }

        nodeType("device") {
            displayName = "Device"
            description = "Connected devices and peripherals"
            category = NodeCategory.SYSTEM
            icon = "device"
            color = "#607D8B"
            shape = NodeShape.RECTANGLE
            size = NodeSize.MEDIUM
        }

        nodeType("url") {
            displayName = "Web Link"
            description = "Web pages and bookmarks"
            category = NodeCategory.NETWORK
            icon = "link"
            color = "#00BCD4"
            shape = NodeShape.DIAMOND
            size = NodeSize.SMALL
        }

        nodeType("setting") {
            displayName = "Setting"
            description = "System settings and configurations"
            category = NodeCategory.SYSTEM
            icon = "gear"
            color = "#795548"
            shape = NodeShape.HEXAGON
            size = NodeSize.SMALL
        }

        // Define edge types for relationships
        edgeType("data-flow") {
            displayName = "Data Flow"
            description = "Data transfer between nodes"
            category = EdgeCategory.DATA_FLOW
            style = EdgeStyle.GRADIENT
            color = "#4CAF50"
            width = 3
            animated = true
            showArrow = true
            
            visual {
                flowAnimation = true
                pulseOnTransfer = true
                colorByDataType = true
            }
        }

        edgeType("dependency") {
            displayName = "Dependency"
            description = "Dependencies between components"
            category = EdgeCategory.DEPENDENCY
            style = EdgeStyle.DASHED
            color = "#FF9800"
            width = 2
        }

        edgeType("relationship") {
            displayName = "Relationship"
            description = "General relationships"
            category = EdgeCategory.ASSOCIATION
            style = EdgeStyle.SOLID
            color = "#999999"
            width = 1
        }

        edgeType("hierarchy") {
            displayName = "Hierarchy"
            description = "Parent-child relationships"
            category = EdgeCategory.HIERARCHY
            style = EdgeStyle.SOLID
            color = "#3F51B5"
            width = 2
            showArrow = true
        }

        edgeType("temporal") {
            displayName = "Temporal"
            description = "Time-based relationships"
            category = EdgeCategory.SEMANTIC
            style = EdgeStyle.DOTTED
            color = "#00BCD4"
            width = 2
            
            visual {
                fadeWithAge = true
                showTimestamp = true
            }
        }

        edgeType("conflict") {
            displayName = "Conflict"
            description = "Conflicts or issues"
            category = EdgeCategory.CUSTOM
            style = EdgeStyle.WAVY
            color = "#F44336"
            width = 3
            animated = true
            
            visual {
                warningGlow = true
                shakeAnimation = true
            }
        }

        // Layout algorithms configuration
        layout(LayoutAlgorithm.FORCE_DIRECTED) {
            nodeRepulsion = 100.0
            edgeAttraction = 50.0
            centeringForce = 0.1
            damping = 0.9
            iterations = 300
            
            constraints {
                minimumNodeDistance = 50.0
                maximumNodeDistance = 500.0
                clusteringStrength = 0.7
            }
        }

        layout(LayoutAlgorithm.HIERARCHICAL) {
            levelSeparation = 150
            nodeSeparation = 100
            direction = "top-to-bottom"
            sortMethod = "directed"
        }

        layout(LayoutAlgorithm.CIRCULAR) {
            radius = 300
            startAngle = 0.0
            sweep = 360.0
            sortBy = "degree"
        }

        // Interaction configuration
        interaction(InteractionType.MOUSE) {
            enablePan = true
            enableZoom = true
            enableRotate = true
            enableSelection = true
            enableMultiSelect = true
            enableBoxSelect = true
            
            dragBehavior {
                enableNodeDrag = true
                enableEdgeDrag = false
                snapToGrid = false
                gridSize = 20
            }
        }

        interaction(InteractionType.KEYBOARD) {
            shortcuts {
                "Ctrl+A" to "Select All"
                "Delete" to "Delete Selected"
                "Ctrl+C" to "Copy"
                "Ctrl+V" to "Paste"
                "Ctrl+Z" to "Undo"
                "Ctrl+Y" to "Redo"
                "F2" to "Rename"
                "Space" to "Quick Search"
                "Tab" to "Navigate Next"
                "Shift+Tab" to "Navigate Previous"
            }
        }

        interaction(InteractionType.GESTURE) {
            enablePinchZoom = true
            enableTwoFingerRotate = true
            enableSwipeNavigation = true
            enableDoubleTap = true
            
            gestureActions {
                "pinch" to "zoom"
                "two-finger-rotate" to "rotate-view"
                "three-finger-swipe" to "switch-workspace"
                "double-tap" to "focus-node"
            }
        }

        // Visual effects
        visualEffect(VisualEffectType.GLOW) {
            enabled = true
            intensity = 0.8
            color = "#FFFFFF"
            blurRadius = 20
            animationDuration = 1000
        }

        visualEffect(VisualEffectType.PARTICLE) {
            enabled = true
            particleCount = 100
            particleSize = 2
            particleSpeed = 0.5
            particleLifetime = 5000
        }

        visualEffect(VisualEffectType.SHADOW) {
            enabled = true
            offsetX = 0
            offsetY = 4
            blurRadius = 10
            color = "#00000033"
        }

        // AI Integration
        aiIntegration {
            enabled = true
            provider = "ollama"
            model = "llama2"
            
            features {
                enableRelationshipDiscovery = true
                enableContentAnalysis = true
                enableSmartClustering = true
                enableWorkflowPrediction = true
                enableSemanticSearch = true
            }
            
            analysisRules {
                // Analyze file access patterns
                rule("file-access-pattern") {
                    trigger = "file-opened"
                    action = "analyze-related-files"
                    confidence = 0.75
                }
                
                // Discover app-file relationships
                rule("app-file-relationship") {
                    trigger = "app-uses-file"
                    action = "create-relationship"
                    confidence = 0.9
                }
                
                // Suggest clusters based on usage
                rule("usage-clustering") {
                    trigger = "periodic"
                    action = "suggest-clusters"
                    interval = "daily"
                }
            }
        }

        // Workspace configuration
        workspace("Main") {
            autoCluster = true
            showRelatedSuggestions = true
            timeBasedDepth = true
            maxVisibleNodes = 200
            
            focusMode {
                enabled = false
                dimBackground = true
                highlightConnected = true
            }
        }

        workspace("Project Work") {
            autoCluster = true
            clusterBy = listOf("project", "date", "type")
            defaultLayout = LayoutAlgorithm.HIERARCHICAL
            
            filters {
                showOnly = listOf("file", "application", "task")
                hideOlderThan = "30 days"
            }
        }

        workspace("Communications") {
            defaultLayout = LayoutAlgorithm.CIRCULAR
            centerNode = "person"
            
            filters {
                showOnly = listOf("person", "message", "email")
            }
        }

        // Theme configuration
        theme("dark") {
            displayName = "Dark Theme"
            isDark = true
            
            colors {
                background = "#1E1E1E"
                foreground = "#E0E0E0"
                primary = "#64B5F6"
                secondary = "#BA68C8"
                accent = "#FFB74D"
                nodeDefault = "#424242"
                edgeDefault = "#666666"
                selection = "#2196F3"
            }
            
            effects {
                glowIntensity = 1.2
                shadowOpacity = 0.6
                animationSpeed = 1.0
            }
        }

        theme("light") {
            displayName = "Light Theme"
            isDark = false
            
            colors {
                background = "#FAFAFA"
                foreground = "#212121"
                primary = "#1976D2"
                secondary = "#7B1FA2"
                accent = "#F57C00"
                nodeDefault = "#E0E0E0"
                edgeDefault = "#999999"
                selection = "#2196F3"
            }
        }

        // Semantic rules for automatic organization
        semanticRule {
            name = "Group project files"
            condition = "files in same directory"
            action = "create soft cluster"
            strength = 0.8
        }

        semanticRule {
            name = "Connect frequently used"
            condition = "opened within 5 minutes"
            action = "create temporal edge"
            strength = 0.6
            decay = "exponential"
        }

        semanticRule {
            name = "Workspace separation"
            condition = "different projects"
            action = "increase repulsion"
            strength = 1.5
        }

        // Gesture configurations for touch devices
        gesture("pinch-zoom") {
            fingers = 2
            movement = "pinch"
            action = "zoom"
            sensitivity = 1.5
        }

        gesture("three-finger-swipe") {
            fingers = 3
            movement = "swipe-horizontal"
            action = "switch-workspace"
            threshold = 100
        }

        gesture("long-press") {
            fingers = 1
            duration = 500
            action = "context-menu"
        }
    }

    // Services
    services {
        enable("NetworkManager")
        enable("sshd")
        
        // Graph desktop service
        systemd("horizonos-compositor") {
            description = "HorizonOS Graph Desktop Compositor"
            wantedBy = listOf("graphical.target")
            after = listOf("multi-user.target")
            
            service {
                type = "notify"
                execStart = "/usr/bin/horizonos-compositor"
                restart = "on-failure"
                
                environment {
                    "WAYLAND_DISPLAY" to "wayland-0"
                    "XDG_RUNTIME_DIR" to "/run/user/1000"
                    "RUST_LOG" to "info"
                }
            }
        }
        
        // Ollama for AI features
        enable("ollama") {
            autoStart = true
            models = listOf("llama2", "codellama")
        }
    }

    // Storage configuration
    storage {
        // Btrfs with subvolumes
        filesystem("btrfs") {
            mountPoint = "/"
            
            subvolume("@") {
                mountPoint = "/"
            }
            
            subvolume("@home") {
                mountPoint = "/home"
            }
            
            subvolume("@graph") {
                mountPoint = "/var/lib/horizonos/graph"
                // Store graph state and cache
            }
            
            subvolume("@snapshots") {
                mountPoint = "/.snapshots"
            }
        }
    }

    // Hardware configuration
    hardware {
        graphics {
            driver = "auto" // Auto-detect GPU
            enableVulkan = true
            enableOpenGL = true
            
            wayland {
                enableHardwareCursors = true
                enableAdaptiveSync = true
            }
        }
        
        input {
            mouse {
                acceleration = "adaptive"
                naturalScroll = false
            }
            
            touchpad {
                naturalScroll = true
                tapToClick = true
                twoFingerScroll = true
                disableWhileTyping = true
            }
            
            keyboard {
                layout = "us"
                options = listOf("ctrl:nocaps") // Caps Lock as Ctrl
            }
        }
    }

    // Boot configuration
    boot {
        loader = "systemd-boot"
        timeout = 5
        
        kernel {
            version = "latest"
            parameters = listOf(
                "quiet",
                "splash",
                "nvidia-drm.modeset=1" // If NVIDIA GPU
            )
        }
    }

    // Development environment
    development {
        languages {
            rust {
                toolchain = "stable"
                components = listOf("rustfmt", "clippy", "rust-analyzer")
            }
        }
        
        tools {
            install("helix", "zellij", "bacon", "just")
        }
    }
}