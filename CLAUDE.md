# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

HorizonOS is an Arch Linux-based immutable distribution using OSTree for atomic updates, Btrfs for snapshots, and featuring a Kotlin DSL for type-safe configuration. The project aims to create a modern Linux desktop with graph-based UI concepts, local-first LLM integration, and a Flutter-based mobile companion app.

## Common Development Commands

### Build Commands
```bash
# Create a test OSTree commit (requires sudo)
sudo ./scripts/scripts/build-test.sh

# Build bootable ISO (requires OSTree commit to exist first)
sudo ./scripts/scripts/build-iso.sh

# Serve OSTree repository over HTTP
./scripts/scripts/serve-ostree.sh
```

### OSTree Management
```bash
# Initialize OSTree repository (already done)
ostree init --repo=repo --mode=archive

# View OSTree commits
ostree log --repo=repo horizonos/test/x86_64

# Check OSTree repository status
ostree summary --repo=repo
```

### Kotlin DSL Development
```bash
# Setup Kotlin DSL environment
./scripts/setup-kotlin-dsl.sh

# Navigate to Kotlin DSL directory
cd src/kotlin-config

# Run tests
./gradlew test

# Build the DSL compiler
./gradlew build

# Compile a configuration file
./gradlew compileConfig -PconfigFile=examples/desktop.horizonos.kts
```

### Testing
```bash
# Test ISO with QEMU (after building ISO)
qemu-system-x86_64 -m 4G -enable-kvm -cdrom build/out/horizonos-*.iso

# Run Kotlin DSL tests (from src/kotlin-config)
./gradlew test
```

## High-Level Architecture

### Directory Structure
- **build/** - Build artifacts (gitignored)
- **config/** - Development configuration files
- **docs/** - Architecture decisions and documentation
- **iso/** - ISO build staging directory
- **repo/** - OSTree repository (gitignored, must be initialized)
- **scripts/** - Build and development scripts
- **src/** - Source code organized by component:
  - **base/** - Base system configuration
  - **desktop/** - Desktop environment (graph-based UI concept)
  - **kotlin-config/** - Kotlin DSL implementation
  - **llm-integration/** - Local LLM integration with Ollama

### Key Design Principles

1. **Immutable System**: Uses OSTree for atomic updates and rollbacks
2. **Btrfs Subvolumes**: Structured layout (@, @home, @var, @snapshots) for efficient snapshots
3. **Type-Safe Configuration**: Kotlin DSL instead of YAML/TOML for compile-time validation
4. **Local-First Privacy**: LLM integration runs locally with automatic hardware optimization
5. **Graph-Based Desktop**: Novel UI paradigm where everything is a node with semantic relationships

### Build System Flow

1. **OSTree Commit Creation**: `build-test.sh` creates a minimal Arch rootfs and commits it to OSTree
2. **ISO Building**: `build-iso.sh` creates a bootable ISO with the OSTree commit embedded
3. **Installation**: The ISO includes a custom installer (`horizonos-install`) that deploys OSTree to the target system

### Development Considerations

- Scripts require sudo for pacstrap and OSTree operations
- OSTree repository must exist at `./repo` before building ISO
- The project uses Arch's rolling release model with OSTree providing stability through atomic updates
- Kotlin DSL is still in development - core implementation needed in `src/kotlin-config/src/main/kotlin/org/horizonos/config/dsl/Core.kt`
- Mobile companion app and graph desktop are planned features not yet implemented

### Configuration Management

The project uses `config/dev.conf` for build settings:
- Version and codename tracking
- Feature flags (immutable, kotlin_config, llm_integration, graph_desktop)
- Package lists for base and desktop environments

### OSTree Branching Strategy
- `horizonos/test/x86_64` - Development builds
- `horizonos/stable/x86_64` - Stable releases (planned)
- `horizonos/testing/x86_64` - Testing branch (planned)