# HorizonOS Architecture Decisions

## Overview

This document records the key architectural decisions made for HorizonOS and the rationale behind them. These decisions shape the fundamental design of the distribution.

## 1. Why OSTree Over Other Immutable Solutions?

### Decision: Use OSTree for System Immutability

**Alternatives Considered:**
- **Nix/NixOS**: Functional package management with reproducible builds
- **Fedora Silverblue Model**: rpm-ostree (OSTree + RPM)
- **Ubuntu Core**: Snap-based system
- **Custom OverlayFS Solution**: Roll our own immutability

**Rationale for OSTree:**

1. **Proven Technology**: OSTree is battle-tested in production (Fedora Silverblue, Endless OS)
2. **Atomic Updates**: Complete system updates succeed or fail atomically
3. **Efficient Storage**: Content-addressed storage with deduplication
4. **Rollback Capability**: Easy rollback to previous deployments
5. **Integration with Existing Tools**: Works well with existing package managers
6. **Arch Compatibility**: Can work with pacman-generated filesystems

**Trade-offs:**
- Learning curve for OSTree concepts
- Less flexibility than Nix's approach
- Requires careful integration with AUR

## 2. Btrfs Subvolume Layout

### Decision: Structured Btrfs Subvolume Hierarchy

**Layout:**
```
/           → @            (root subvolume)
/home       → @home        (user data)
/var        → @var         (variable data)
/.snapshots → @snapshots   (snapshot storage)
```

**Rationale:**

1. **Separation of Concerns**:
   - System (`@`) can be rolled back independently of user data
   - `/var` isolation prevents log/cache bloat from affecting snapshots
   - Dedicated snapshot location prevents recursive snapshot issues

2. **Snapshot Efficiency**:
   - Smaller, focused snapshots of system changes
   - User data (`@home`) can be snapshotted on different schedule
   - Excludes temporary/cache data from system snapshots

3. **Flexibility**:
   - Different compression settings per subvolume
   - Independent backup strategies
   - Mount options can be optimized per use case

**Why Not Alternatives:**
- **Flat layout** (everything in one subvolume): No granular control
- **Per-directory subvolumes**: Too complex, marginal benefits
- **ZFS**: Licensing concerns, less native kernel integration

## 3. Kotlin DSL for Configuration

### Decision: Type-Safe Kotlin DSL Instead of YAML/TOML

**Alternatives Considered:**
- **YAML** (like Ansible/cloud-init)
- **TOML** (like many Rust projects)
- **Nix Language** (functional configuration)
- **JSON** (universal but verbose)
- **Bash Scripts** (traditional approach)

**Rationale for Kotlin DSL:**

1. **Type Safety**:
   ```kotlin
   // Compile-time validation
   packages {
       install("firefox")  // ✓ Valid
       instal("firefox")   // ✗ Compile error
   }
   ```

2. **IDE Support**:
   - Autocompletion
   - Inline documentation
   - Refactoring support
   - Error highlighting

3. **Expressiveness**:
   ```kotlin
   // Conditional logic
   if (hardware.hasNvidia) {
       packages { install("nvidia-dkms") }
   }
   
   // Loops and functions
   listOf("development", "multimedia").forEach { group ->
       packages { group(name) { install(getPackagesFor(name)) } }
   }
   ```

4. **Familiarity**:
   - Kotlin is approachable for developers
   - Similar to Gradle build scripts
   - Can leverage existing Kotlin ecosystem

**Trade-offs:**
- JVM dependency (mitigated by GraalVM native image)
- Steeper learning curve than YAML
- More complex implementation

## 4. Graph-Based Desktop Concept

### Decision: Graph as Primary Information Organization Metaphor

**Core Concept:**
- Everything is a node (files, apps, people, tasks, devices)
- Relationships are edges with semantic meaning
- Navigation through relationships, not hierarchies

**Rationale:**

1. **Cognitive Alignment**:
   - Matches how humans think (associatively)
   - Reduces artificial categorization decisions
   - Natural for modern interconnected workflows

2. **Rich Semantics**:
   - Relationships have meaning beyond containment
   - Multiple valid paths to information
   - Context-aware computing possibilities

3. **Modern Use Cases**:
   - Project-based work crosses traditional boundaries
   - Collaboration requires understanding relationships
   - AI can leverage graph structure for better assistance

**Implementation Strategy:**
- Start with traditional desktop, add graph view
- Use existing graph viz libraries initially
- Build custom Wayland compositor eventually
- Property graph model (not RDF) for simplicity

## 5. Local-First LLM Integration

### Decision: Ollama with Automatic Model Selection

**Architecture:**
```
Hardware Detection → Model Selection → Runtime
     ├─ GPU (VRAM)      ├─ 70B models     ├─ Ollama
     ├─ RAM             ├─ 7-13B models   ├─ llama.cpp
     └─ CPU cores       └─ 3B models      └─ Cloud fallback
```

**Rationale:**

1. **Privacy**: User data never leaves device by default
2. **Performance**: Local inference for common tasks
3. **Flexibility**: Automatic optimization for available hardware
4. **Fallback**: Cloud services when local isn't sufficient

**Key Decisions:**
- Ollama as primary runtime (good API, model management)
- Automatic VRAM-based model selection
- System-wide API for all applications
- Explicit user consent for cloud services

## 6. Mobile Companion Architecture

### Decision: Flutter + WebRTC + D-Bus Bridge

**Stack:**
```
Mobile (Flutter) ← WebRTC → Desktop (D-Bus Service)
                 ← SSHFS  → File System Access
```

**Rationale:**

1. **Flutter**: Cross-platform, single codebase, good performance
2. **WebRTC**: Low latency, NAT traversal, encrypted by default
3. **D-Bus**: Standard Linux desktop integration
4. **SSHFS**: Secure, established file access protocol

**Features Enabled:**
- Real-time application streaming
- Remote control with sub-200ms latency
- Secure file system browsing
- Notification sync
- Task management

## 7. Development Philosophy

### Decision: Pragmatic Innovation

**Principles:**

1. **Build on Giants**: Use proven components (OSTree, Btrfs, Wayland)
2. **Innovate at the Edges**: Novel UI/UX, Kotlin DSL, LLM integration
3. **User-Centric**: Every feature must solve real user problems
4. **Privacy First**: Local processing, explicit consent for cloud
5. **Community Driven**: Design for contributions from day one

**Non-Goals:**
- Not trying to replace all existing software
- Not competing on minimalism (plenty of minimal distros)
- Not targeting servers (desktop/workstation focus)

## 8. Release and Update Strategy

### Decision: Rolling Release with Stable Snapshots

**Model:**
```
Development → Testing → Stable
    ↓           ↓         ↓
  Daily      Weekly    Monthly
  Builds     Builds    Snapshots
```

**Rationale:**
- Arch's rolling model proven for desktop use
- OSTree makes rollbacks safe
- Stable snapshots provide predictability
- Testing branch allows community validation

## Summary

These architectural decisions establish HorizonOS as a modern, innovative desktop operating system that:

1. Leverages proven technologies (OSTree, Btrfs) for reliability
2. Introduces novel concepts (Kotlin DSL, Graph Desktop) for better UX
3. Prioritizes user privacy and control
4. Builds for the future (LLM integration, mobile companion)

Each decision involves trade-offs, but together they create a coherent vision for a next-generation Linux desktop that's both practical and forward-thinking.

---

*This document is living and will be updated as new architectural decisions are made.*