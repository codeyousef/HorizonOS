# HorizonOS Container Architecture

## Overview

HorizonOS implements a revolutionary container-based architecture that fundamentally changes how system packages are managed. Instead of installing packages directly to the immutable base system, all system software runs in containers, providing isolation, reproducibility, and flexibility.

## Architecture Layers

```
┌─────────────────────────────────────────┐
│          User Applications              │
│         (Flatpak / Snap)               │
├─────────────────────────────────────────┤
│        System Containers                │
│    (Development, Desktop, Gaming)       │
├─────────────────────────────────────────┤
│        Container Runtime                │
│      (Podman + Buildah + Skopeo)      │
├─────────────────────────────────────────┤
│      Minimal Base System (~500MB)       │
│        (OSTree + Core Tools)          │
└─────────────────────────────────────────┘
```

### 1. Minimal Base System
- **Size**: ~500MB
- **Contents**: Linux kernel, systemd, container runtime, essential tools
- **Immutable**: Managed by OSTree for atomic updates
- **Purpose**: Provides only the foundation needed to run containers

### 2. Container Runtime Layer
- **Engine**: Podman (rootless containers by default)
- **Build**: Buildah for creating custom containers
- **Registry**: Skopeo for image management
- **Security**: SELinux + seccomp + user namespaces

### 3. System Containers
- **Purpose-based**: Development, Desktop, Gaming, Multimedia, etc.
- **Persistent**: Can maintain state between reboots
- **Integrated**: Binaries exported to host PATH
- **Flexible**: Easy to update, rollback, or replace

### 4. User Applications
- **Flatpak**: Primary method for GUI applications
- **Snap**: Alternative for certain applications
- **AppImage**: Supported but not managed

## Key Benefits

### 1. Minimal Attack Surface
The base system contains only essential components, dramatically reducing potential vulnerabilities.

### 2. Package Isolation
Each container runs in its own namespace with limited capabilities and resources.

### 3. Reproducible Environments
Container definitions ensure consistent environments across installations.

### 4. Easy Updates
Update containers independently without touching the base system.

### 5. Multiple Versions
Run different versions of the same software in separate containers.

## Container Management

### Command Line Interface

The `horizon-container` tool manages all system containers:

```bash
# List available containers
horizon-container list

# Install a container
horizon-container install development

# Run command in container
horizon-container run development git status

# Open shell in container
horizon-container shell development

# Export binary to host
horizon-container export gcc

# Update container
horizon-container update development
```

### Container Definitions

Containers are defined in JSON format:

```json
{
  "name": "development",
  "image": "quay.io/toolbx/arch-toolbox:latest",
  "purpose": "development",
  "packages": ["git", "gcc", "make", "python", "nodejs"],
  "export_binaries": ["git", "gcc", "make", "python", "node"],
  "persistent": true,
  "mounts": ["/home", "/tmp"],
  "environment": {
    "CONTAINER_PURPOSE": "development"
  }
}
```

### System vs User Containers

- **System Containers** (`/etc/containers/system/`): Pre-configured, available to all users
- **User Containers** (`~/.config/horizonos/containers/`): User-specific customizations

## Development Workflow

### Quick Start

```bash
# Install development container
horizon-container install development

# Use the development helper
horizon-dev shell  # Opens development shell
horizon-dev gcc --version  # Run gcc in container
horizon-dev setup rust  # Install Rust toolchain
```

### Creating Custom Containers

1. Create a container definition:
```bash
cat > ~/.config/horizonos/containers/my-tools.json << EOF
{
  "name": "my-tools",
  "image": "docker.io/library/ubuntu:22.04",
  "packages": ["vim", "tmux", "htop"],
  "export_binaries": ["vim", "tmux"],
  "persistent": true
}
EOF
```

2. Install and use:
```bash
horizon-container install my-tools
horizon-container shell my-tools
```

## System Integration

### Binary Export

Exported binaries are available in the host PATH through wrapper scripts:

```bash
# Export git from development container
horizon-container export git

# Now git is available system-wide
git --version  # Runs git inside the container transparently
```

### Systemd Services

Containers can be managed by systemd:

```bash
# Enable container to start at boot
systemctl --user enable container@development.service

# Check container status
systemctl --user status container@development.service
```

### Desktop Integration

Desktop environments run in privileged containers with access to:
- Display server (X11/Wayland)
- Audio subsystem (PipeWire)
- GPU acceleration (Mesa/Vulkan)
- Input devices

## Security Model

### Default Security Features

1. **User Namespaces**: Containers run with mapped UIDs
2. **Seccomp Filters**: System call filtering
3. **Capability Dropping**: Minimal capabilities by default
4. **Read-only Bind Mounts**: Protected system directories
5. **Network Isolation**: Private network namespaces

### Container Capabilities

Default capabilities (can be extended per container):
- CHOWN, DAC_OVERRIDE, FOWNER, FSETID
- KILL, NET_BIND_SERVICE, SETFCAP
- SETGID, SETPCAP, SETUID, SYS_CHROOT

### Privileged Containers

Only specific containers (desktop, security tools) run with extended privileges:
```json
{
  "privileged": true,
  "capabilities": ["SYS_ADMIN", "NET_ADMIN"]
}
```

## Storage Management

### Container Storage Locations

- **Images**: `/var/lib/containers/storage/`
- **Volumes**: `/var/lib/containers/storage/volumes/`
- **Runtime**: `/run/containers/storage/`

### User Data

User data remains in `/home` and is bind-mounted into containers as needed.

### Cleanup

```bash
# Remove unused images
podman image prune

# Remove stopped containers
podman container prune

# Full cleanup
horizon-container cleanup
```

## Updates and Rollbacks

### System Updates

```bash
# Update everything
horizon-update --all

# Update only containers
horizon-update --containers

# Check for updates
horizon-update --check
```

### Container Updates

```bash
# Update specific container
horizon-container update development

# Pull latest image
podman pull quay.io/toolbx/arch-toolbox:latest
```

### Rollbacks

```bash
# System rollback (OSTree)
horizon-update --rollback

# Container rollback
podman tag development:latest development:backup
podman run development:backup
```

## Performance Considerations

### Container Overhead
- **CPU**: Near-native performance (<1% overhead)
- **Memory**: Minimal overhead with shared libraries
- **Storage**: Deduplication through overlay filesystem
- **I/O**: Native performance with bind mounts

### Optimization Tips

1. Use persistent containers for frequently accessed tools
2. Export commonly used binaries
3. Enable zstd compression for images
4. Use thin provisioning for production

## Troubleshooting

### Common Issues

1. **Container won't start**
   ```bash
   podman logs horizonos-development
   horizon-container status development
   ```

2. **Permission denied**
   ```bash
   # Check subuid/subgid mappings
   cat /etc/subuid /etc/subgid
   
   # Verify user namespaces
   sysctl kernel.unprivileged_userns_clone
   ```

3. **Binary not found**
   ```bash
   # Re-export the binary
   horizon-container export <binary>
   
   # Check PATH
   echo $PATH
   ```

### Debug Mode

```bash
# Run container with debug output
HORIZON_DEBUG=1 horizon-container run development bash

# Check container configuration
podman inspect horizonos-development
```

## Future Enhancements

### Planned Features

1. **Container Orchestration**: Kubernetes/Podman-compose support
2. **Image Signing**: Cryptographic verification of containers
3. **A/B Container Updates**: Atomic container updates
4. **GPU Passthrough**: Enhanced GPU support for AI/ML
5. **Distributed Storage**: Cluster-wide container sharing

### Integration with Kotlin DSL

Future versions will allow container definitions in Kotlin:

```kotlin
horizonOS {
    containers {
        system("development") {
            image = "quay.io/toolbx/arch-toolbox:latest"
            packages = ["git", "gcc", "rust"]
            exportBinaries = ["git", "gcc", "cargo"]
            persistent = true
        }
    }
}
```

## Best Practices

### Container Design

1. **Single Purpose**: Each container should serve one primary function
2. **Minimal Size**: Include only necessary packages
3. **Version Pinning**: Use specific image tags for reproducibility
4. **Documentation**: Document custom containers and their purpose

### Security

1. **Least Privilege**: Request only needed capabilities
2. **Regular Updates**: Keep container images current
3. **Image Sources**: Use trusted registries
4. **Secrets Management**: Never embed secrets in images

### Performance

1. **Layer Caching**: Structure Dockerfiles for optimal caching
2. **Multi-stage Builds**: Reduce final image size
3. **Resource Limits**: Set appropriate CPU/memory limits
4. **Storage Drivers**: Use overlay2 for best performance

## Conclusion

HorizonOS's container-based architecture represents a paradigm shift in system design. By combining an immutable base with containerized system packages, we achieve:

- **Security**: Reduced attack surface and strong isolation
- **Flexibility**: Easy to customize and extend
- **Reliability**: Atomic updates and easy rollbacks
- **Performance**: Near-native speed with minimal overhead

This architecture makes HorizonOS ideal for developers, power users, and anyone who values a secure, flexible, and maintainable system.