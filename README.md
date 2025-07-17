# HorizonOS

An immutable Linux distribution built on Arch Linux with OSTree for atomic updates, featuring a revolutionary Kotlin DSL for type-safe system configuration and the world's first graph-native desktop environment.

## 🌟 What Makes HorizonOS Special

- **Type-Safe Configuration**: Kotlin DSL replaces traditional YAML/TOML with compile-time validation
- **Immutable Infrastructure**: OSTree provides atomic updates and rollbacks
- **Graph Desktop**: Revolutionary UI paradigm where everything is a node with semantic relationships
- **Local-First Privacy**: Built-in LLM integration runs entirely locally with hardware optimization
- **Enterprise Ready**: Comprehensive networking, security, and automation capabilities

## 🚀 Quick Start

### Building HorizonOS

```bash
# Build OSTree commit
sudo ./scripts/scripts/build-test.sh

# Build bootable ISO
sudo ./scripts/scripts/build-iso.sh

# Test in QEMU
qemu-system-x86_64 -m 4G -enable-kvm -cdrom build/out/horizonos-*.iso
```

### Kotlin DSL Configuration

```bash
# Setup Kotlin DSL environment
./scripts/setup-kotlin-dsl.sh

# Navigate to DSL directory
cd src/kotlin-config

# Run tests
./gradlew test

# Compile a configuration
./gradlew compileConfig -PconfigFile=examples/desktop.horizonos.kts
```

## 📋 Roadmap & Implementation Status

### ✅ **Phase 1: Core Infrastructure (COMPLETED)**

#### Immutable System Foundation
- [x] **OSTree Integration** - Atomic updates and rollbacks
- [x] **Btrfs Layout** - Structured subvolumes (@, @home, @var, @snapshots)
- [x] **Build System** - Arch-based with custom ISO generation
- [x] **Installation System** - Custom installer with OSTree deployment

#### Kotlin DSL Core (100% Complete)
- [x] **Core Configuration DSL** - Packages, services, users, repositories
- [x] **AI/LLM Integration** - Local execution with Ollama, hardware optimization
- [x] **Automation Framework** - RPA workflows, teaching modes, browser automation
- [x] **Network Configuration** - Interfaces, WiFi, VPN, firewall, DNS, bridges, VLANs
- [x] **Compiler Pipeline** - Parser, validator, multi-format generator
- [x] **Runtime Components** - Live updates, change detection, state management
- [x] **Comprehensive Testing** - 86+ tests with 100% pass rate
- [x] **Documentation** - API docs, guides, examples

### 🔄 **Phase 2: System Configuration Modules (IN PROGRESS)**

#### Core System Modules (25% Complete)
- [x] **Network Module** - Complete networking configuration with enterprise features
- [ ] **Boot & Kernel Module** - Bootloader, kernel parameters, modules *(Next)*
- [ ] **Hardware Module** - GPU drivers, display, power management
- [ ] **Storage Module** - Filesystems, RAID, encryption, snapshots
- [ ] **Security Module** - PAM, sudo, SSH, SELinux/AppArmor, certificates

#### Service & Environment Modules (0% Complete)
- [ ] **Enhanced Services** - Databases, web servers, containers, monitoring
- [ ] **Development Environment** - Language toolchains, IDEs, virtual environments
- [ ] **Shell & Environment** - Shell configuration, dotfiles, environment variables

### 🎯 **Phase 3: Desktop Revolution (IN PROGRESS - 55% Complete)**

#### Graph Desktop Implementation (55% Complete)
- [x] **Rendering Engine** - WebGPU/wgpu-rs based graph visualization with WGSL shaders
- [x] **Node System** - Complete node architecture with 8 node types (Application, File, Person, Task, Device, AIAgent, Concept, System)
- [x] **Semantic Relationships** - Smart edge types with automated relationship discovery and strength analysis
- [x] **Layout Algorithms** - 6 complete algorithms: force-directed, hierarchical, circular, grid, cluster, temporal
- [ ] **Multi-touch Support** - Gestures, pinch-zoom, rotation *(Next)*
- [ ] **AI Integration** - Node suggestions, auto-organization, semantic search
- [ ] **Real-time Collaboration** - Multi-user graph editing and synchronization

#### Enhanced Desktop Experience (0% Complete)
- [ ] **Wayland Compositor** - Custom compositor optimized for graph interactions
- [ ] **Traditional Mode** - Fallback to conventional desktop paradigms
- [ ] **Theme System** - Deep customization of visual appearance
- [ ] **Accessibility** - Full a11y support for graph navigation

### 🏢 **Phase 4: Enterprise Features (PLANNED)**

#### Advanced Configuration (0% Complete)
- [ ] **Configuration Templates** - Reusable patterns and inheritance
- [ ] **Multi-Environment Support** - Dev/staging/production configurations
- [ ] **Migration Tools** - Import from NixOS, Ansible, Docker Compose
- [ ] **Visual Configuration** - GUI builder for complex configurations

#### Enterprise Integration (0% Complete)
- [ ] **Directory Services** - LDAP, Active Directory integration
- [ ] **Certificate Management** - Automated cert provisioning and rotation
- [ ] **Compliance** - SOC2, HIPAA, GDPR configuration templates
- [ ] **Monitoring** - Built-in observability and alerting

## 🛠 Technical Architecture

### Core Technologies
- **Base OS**: Arch Linux (rolling release)
- **Update System**: OSTree (atomic updates)
- **Filesystem**: Btrfs (snapshots, subvolumes)
- **Configuration**: Kotlin DSL (type-safe, compile-time validated)
- **Build System**: Gradle + Custom scripts
- **Package Management**: Pacman (with OSTree overlay)

### Key Advantages Over NixOS
1. **Better Ergonomics**: Intuitive Kotlin syntax vs functional Nix language
2. **IDE Integration**: Full IntelliJ/VSCode support with auto-completion
3. **Type Safety**: Compile-time validation prevents configuration errors
4. **Semantic Types**: `firewall.allow.ssh` vs `networking.firewall.allowedTCPPorts = [22]`
5. **Live Updates**: Partial configuration application without rebuilds
6. **Visual Diff**: See exactly what changes before applying
7. **Graph-Native**: First-class support for graph-based computing

### Directory Structure
```
horizonos/
├── src/
│   ├── kotlin-config/          # Kotlin DSL implementation
│   │   ├── src/main/kotlin/    # Core DSL code
│   │   ├── src/test/kotlin/    # Comprehensive test suite
│   │   └── examples/           # Configuration examples
│   └── desktop/                # Graph Desktop Environment
│       ├── graph-engine/       # WebGPU rendering engine
│       ├── graph-nodes/        # Node system implementation
│       ├── graph-edges/        # Edge and relationship system
│       ├── graph-layout/       # Layout algorithms (6 complete)
│       ├── graph-interaction/  # Input and interaction handling
│       ├── graph-ai/           # AI integration layer
│       ├── graph-workspaces/   # Workspace management
│       ├── graph-integration/  # System integration
│       ├── graph-config/       # Configuration and theming
│       └── graph-bridge/       # Traditional mode compatibility
├── scripts/                    # Build and deployment scripts
├── config/                     # Development configuration
├── docs/                       # Architecture and API documentation
└── build/                      # Build artifacts (gitignored)
```

## 📚 Documentation

- **[Developer Guide](src/kotlin-config/docs/DEVELOPER_GUIDE.md)** - Complete development setup and workflow
- **[Automation Guide](src/kotlin-config/docs/AUTOMATION_GUIDE.md)** - RPA and workflow automation
- **[Implementation Progress](kotlin-dsl-implementation-progress.md)** - Detailed development roadmap
- **[Graph Desktop Progress](GRAPH_DESKTOP_PROGRESS.md)** - Graph desktop environment implementation status
- **[API Reference](src/kotlin-config/docs/)** - Complete DSL API documentation

## 🧪 Example Configuration

```kotlin
horizonOS {
    hostname = "my-horizonos"
    timezone = "America/New_York"
    locale = "en_US.UTF-8"

    // Type-safe package management
    packages {
        group("base") {
            install("base", "linux", "btrfs-progs")
        }
        group("desktop") {
            install("plasma-meta", "firefox")
        }
    }

    // Network configuration with semantic firewall rules
    network {
        networkInterface("eth0") {
            ipv4 { method = IPv4Method.DHCP }
        }
        
        firewall {
            allow {
                ssh(from = "192.168.1.0/24")
                https()
                port(8080, from = "localhost")
            }
        }
        
        wifi {
            network("HomeWiFi") {
                password = "secret123"
                security = WiFiSecurity.WPA3_PSK
            }
        }
    }

    // AI/LLM integration
    ai {
        enabled = true
        model("llama3") {
            provider = "ollama"
            size = ModelSize.MEDIUM
            capabilities(TEXT_GENERATION, CODE_GENERATION)
        }
        privacy = Privacy.LOCAL_ONLY
    }

    // Automation workflows
    automation {
        workflow("daily-update") {
            trigger { time("02:00") }
            actions { runCommand("pacman -Syu --noconfirm") }
        }
    }
}
```

## 🔬 Development Status

HorizonOS is in active development with focus on core system modules and the revolutionary graph desktop implementation.

## 🎯 Project Goals

- **Replace traditional config management** with type-safe, validated configurations
- **Pioneer graph-based computing** as the next evolution of desktop interfaces  
- **Achieve enterprise reliability** through immutable infrastructure
- **Maintain rolling release** benefits while ensuring system stability
- **Privacy by design** with local-first AI and zero telemetry

## 📊 Current Statistics

### Kotlin DSL
- **86+ Tests** - Comprehensive test coverage
- **2000+ Lines** - Kotlin DSL implementation
- **10+ Modules** - System configuration coverage
- **5+ Examples** - Real-world configuration demonstrations
- **100% Type Safety** - Compile-time validation for all configurations

### Graph Desktop Environment
- **4/10 Core Components** - Complete (Rendering, Nodes, Edges, Layout)
- **6 Layout Algorithms** - Force-directed, hierarchical, circular, grid, cluster, temporal
- **8 Node Types** - Application, File, Person, Task, Device, AIAgent, Concept, System
- **8 Edge Types** - Contains, DependsOn, CommunicatesWith, CreatedBy, RelatedTo, Temporal, TaggedAs, WorksOn
- **3000+ Lines** - Rust implementation with WebGPU/wgpu-rs
- **50+ Tests** - Comprehensive test coverage for all components

---

**HorizonOS**: Where immutable infrastructure meets intelligent automation and revolutionary user experiences.