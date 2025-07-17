# HorizonOS Container Quick Start Guide

This guide will get you up and running with HorizonOS's container-based architecture in minutes.

## Prerequisites

- HorizonOS installed (or running from Live USB)
- Basic familiarity with command line

## First Steps

### 1. Check System Status

```bash
# Verify you're running container-based HorizonOS
cat /etc/horizonos-release

# Check container runtime
podman --version
horizon-container --version
```

### 2. List Available Containers

```bash
# See what containers are available
horizon-container list

# Example output:
# System containers:
#   development        (development)    quay.io/toolbx/arch-toolbox:latest
#   multimedia         (multimedia)     docker.io/linuxserver/ffmpeg:latest
#   gaming            (gaming)         docker.io/steamcmd/steamcmd:latest
```

## Basic Container Usage

### Installing Your First Container

Let's install the development container:

```bash
# Install development container
horizon-container install development

# This will:
# 1. Pull the container image
# 2. Create a persistent container instance
# 3. Configure it for HorizonOS integration
```

### Running Commands

```bash
# Run a single command
horizon-container run development gcc --version

# Open an interactive shell
horizon-container shell development

# Use the development helper (easier!)
horizon-dev shell
```

### Exporting Binaries

Make container tools available system-wide:

```bash
# Export git to host system
horizon-container export git

# Now use it normally
git --version
git clone https://github.com/user/repo.git
```

## Common Workflows

### Development Setup

```bash
# 1. Install development container
horizon-container install development

# 2. Set up your preferred language
horizon-dev setup rust    # Install Rust
horizon-dev setup python  # Install Python
horizon-dev setup node    # Install Node.js

# 3. Export common tools
horizon-container export git
horizon-container export vim
horizon-container export make

# 4. Start coding!
horizon-dev shell
```

### Multimedia Tasks

```bash
# Install multimedia container
horizon-container install multimedia

# Convert video
horizon-container run multimedia ffmpeg -i input.mp4 output.webm

# Batch convert images
horizon-container run multimedia mogrify -resize 50% *.jpg
```

### Gaming Setup

```bash
# Install gaming container
horizon-container install gaming

# Run Steam
horizon-container run gaming steam

# Or export it for regular use
horizon-container export steam
steam
```

## Creating Custom Containers

### Basic Container Definition

Create `~/.config/horizonos/containers/my-tools.json`:

```json
{
  "name": "my-tools",
  "image": "docker.io/library/archlinux:latest",
  "purpose": "custom",
  "packages": ["neovim", "ripgrep", "fzf", "tmux"],
  "export_binaries": ["nvim", "rg", "fzf", "tmux"],
  "persistent": true,
  "mounts": ["/home", "/tmp"]
}
```

Install and use:

```bash
horizon-container install my-tools
horizon-container shell my-tools
```

### Development Environment Example

Create a full Rust development environment:

```json
{
  "name": "rust-dev",
  "image": "docker.io/library/rust:latest",
  "purpose": "development",
  "packages": ["git", "pkg-config", "libssl-dev"],
  "export_binaries": ["cargo", "rustc", "rustup"],
  "persistent": true,
  "mounts": ["/home", "/tmp"],
  "environment": {
    "RUST_BACKTRACE": "1",
    "CARGO_HOME": "/home/$USER/.cargo"
  }
}
```

## Tips and Tricks

### 1. Shell Aliases

Add to `~/.bashrc` or `~/.config/fish/config.fish`:

```bash
# Quick shortcuts
alias dev="horizon-dev shell"
alias hc="horizon-container"

# Direct tool access
alias npm="horizon-dev npm"
alias cargo="horizon-dev cargo"
```

### 2. Multiple Versions

Run different versions of the same tool:

```bash
# Create Python 3.11 container
cat > ~/.config/horizonos/containers/python311.json << EOF
{
  "name": "python311",
  "image": "docker.io/library/python:3.11",
  "export_binaries": ["python3.11", "pip3.11"],
  "persistent": true
}
EOF

# Create Python 3.12 container  
cat > ~/.config/horizonos/containers/python312.json << EOF
{
  "name": "python312", 
  "image": "docker.io/library/python:3.12",
  "export_binaries": ["python3.12", "pip3.12"],
  "persistent": true
}
EOF

# Install both
horizon-container install python311
horizon-container install python312

# Use specific versions
python3.11 --version
python3.12 --version
```

### 3. Container Compose

For complex applications with multiple containers:

```yaml
# Create docker-compose.yml
version: '3'
services:
  web:
    image: nginx:alpine
    ports:
      - "8080:80"
  db:
    image: postgres:15
    environment:
      POSTGRES_PASSWORD: example

# Run with podman-compose
podman-compose up
```

### 4. Performance Optimization

```bash
# Pre-pull images for faster first run
podman pull quay.io/toolbx/arch-toolbox:latest

# Create volume for build cache
podman volume create build-cache

# Use in container
horizon-container run development -v build-cache:/cache make
```

## Troubleshooting

### Container Won't Start

```bash
# Check logs
podman logs horizonos-development

# Recreate container
podman rm horizonos-development
horizon-container install development
```

### Command Not Found

```bash
# Re-export the binary
horizon-container export <command>

# Check if container has the tool
horizon-container run development which <command>
```

### Permission Issues

```bash
# Ensure proper subuid/subgid mapping
echo "$USER:100000:65536" | sudo tee -a /etc/subuid
echo "$USER:100000:65536" | sudo tee -a /etc/subgid

# Restart podman
systemctl --user restart podman
```

## Next Steps

- Read the full [Container Architecture](CONTAINER-ARCHITECTURE.md) documentation
- Explore available [container definitions](../scripts/configs/containers/)
- Create your own custom containers
- Join the HorizonOS community for tips and support

Remember: In HorizonOS, containers aren't just for deployment—they're how we manage system packages!