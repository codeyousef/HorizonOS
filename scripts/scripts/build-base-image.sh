#!/usr/bin/env bash
set -euo pipefail

# HorizonOS Minimal Base Image Builder
# Creates a minimal (~500MB) base image for container-based architecture

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BUILD_DIR="$PROJECT_ROOT/build"
BASE_DIR="$BUILD_DIR/base"
CACHE_DIR="$BUILD_DIR/cache"

# Load configuration
source "$PROJECT_ROOT/config/dev.conf"

echo "=== HorizonOS Minimal Base Image Builder ==="
echo "Version: $HORIZONOS_VERSION"
echo "Target: Container-based architecture"
echo "Expected size: ~500MB"

# Check if running as root
if [ "$EUID" -ne 0 ]; then 
    echo "This script must be run as root (use sudo)"
    exit 1
fi

# Check for required tools
for tool in pacstrap ostree podman; do
    if ! command -v $tool &> /dev/null; then
        echo "Error: $tool is not installed"
        echo "Install with: pacman -S $tool"
        exit 1
    fi
done

# Initialize OSTree repository if it doesn't exist
if [ ! -d "$PROJECT_ROOT/repo" ]; then
    echo "Initializing OSTree repository..."
    ostree init --repo="$PROJECT_ROOT/repo" --mode=archive
fi

# Clean previous build
echo "Cleaning previous build..."
rm -rf "$BASE_DIR"
mkdir -p "$BASE_DIR" "$CACHE_DIR"

# Create minimal base rootfs
echo "Creating minimal base rootfs..."
echo "This creates a ~500MB base image with essential packages only"

# Minimal package set for container-based architecture
MINIMAL_PACKAGES=(
    # Essential base system
    "base"
    "linux"
    "linux-firmware"
    "systemd"
    "systemd-sysvcompat"
    
    # Container runtime
    "podman"
    "crun"
    "slirp4netns"
    "fuse-overlayfs"
    "buildah"
    "skopeo"
    
    # Package managers for user applications
    "flatpak"
    
    # Essential tools
    "fish"
    "neovim"
    "git"
    "curl"
    "wget"
    "sudo"
    "htop"
    "man-db"
    "man-pages"
    
    # Filesystem support
    "btrfs-progs"
    "e2fsprogs"
    "dosfstools"
    "xfsprogs"
    
    # Network essentials
    "networkmanager"
    "openssh"
    "iproute2"
    "iputils"
    "dhcpcd"
    
    # Hardware support
    "udev"
    "pciutils"
    "usbutils"
    "lshw"
    
    # Archive and compression
    "tar"
    "gzip"
    "xz"
    "zstd"
    
    # Security essentials
    "gnupg"
    "ca-certificates"
    "ca-certificates-mozilla"
)

# Create base system with minimal packages
echo "Installing minimal package set (${#MINIMAL_PACKAGES[@]} packages)..."
pacstrap -c "$BASE_DIR" "${MINIMAL_PACKAGES[@]}"

# Configure the minimal system
echo "Configuring minimal base system..."

# Set hostname
echo "horizonos" > "$BASE_DIR/etc/hostname"

# Configure hosts file
cat > "$BASE_DIR/etc/hosts" << EOF
127.0.0.1   localhost
127.0.1.1   horizonos
::1         localhost
EOF

# Set system branding (replace Arch Linux branding)
cat > "$BASE_DIR/etc/os-release" << EOF
NAME="HorizonOS"
VERSION="$HORIZONOS_VERSION"
ID=horizonos
ID_LIKE=arch
PRETTY_NAME="HorizonOS $HORIZONOS_VERSION"
ANSI_COLOR="0;36"
HOME_URL="https://github.com/codeyousef/HorizonOS"
DOCUMENTATION_URL="https://github.com/codeyousef/HorizonOS/wiki"
SUPPORT_URL="https://github.com/codeyousef/HorizonOS/issues"
BUG_REPORT_URL="https://github.com/codeyousef/HorizonOS/issues"
LOGO=horizonos
EOF

# Create issue file for login prompt
cat > "$BASE_DIR/etc/issue" << 'EOF'
Welcome to HorizonOS - The Future of Computing
\S Kernel \r on an \m (\l)

EOF

# Configure locale
echo "en_US.UTF-8 UTF-8" > "$BASE_DIR/etc/locale.gen"
arch-chroot "$BASE_DIR" locale-gen
echo "LANG=en_US.UTF-8" > "$BASE_DIR/etc/locale.conf"

# Configure timezone (UTC by default)
arch-chroot "$BASE_DIR" ln -sf /usr/share/zoneinfo/UTC /etc/localtime

# Configure container runtime
echo "Configuring container runtime..."
mkdir -p "$BASE_DIR/etc/containers"

# Create containers.conf for rootless containers
cat > "$BASE_DIR/etc/containers/containers.conf" << 'EOF'
[containers]
# Enable rootless containers by default
default_ulimits = [
    "nofile=1048576:1048576",
    "nproc=1048576:1048576"
]

[engine]
# Use crun as the default OCI runtime
runtime = "crun"
EOF

# Create registries.conf for container registries
cat > "$BASE_DIR/etc/containers/registries.conf" << 'EOF'
[registries.search]
registries = ['docker.io', 'quay.io', 'ghcr.io']

[registries.insecure]
registries = []

[registries.block]
registries = []
EOF

# Create policy.json for container security
cat > "$BASE_DIR/etc/containers/policy.json" << 'EOF'
{
    "default": [
        {
            "type": "insecureAcceptAnything"
        }
    ],
    "transports": {
        "docker-daemon": {
            "": [
                {
                    "type": "insecureAcceptAnything"
                }
            ]
        }
    }
}
EOF

# Configure Flatpak repositories
echo "Configuring Flatpak repositories..."
arch-chroot "$BASE_DIR" flatpak remote-add --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo

# Configure systemd services
echo "Configuring systemd services..."
arch-chroot "$BASE_DIR" systemctl enable NetworkManager
arch-chroot "$BASE_DIR" systemctl enable sshd
arch-chroot "$BASE_DIR" systemctl enable podman.socket
arch-chroot "$BASE_DIR" systemctl enable flatpak-system-helper

# Boot configuration for installed system (not live ISO)
echo "Configuring boot services..."

# Set default target to multi-user (no graphical)
arch-chroot "$BASE_DIR" systemctl set-default multi-user.target

# Note: Getty configuration is handled differently for:
# - Live ISO: Uses archiso defaults (configured in build-iso.sh)
# - Installed system: Will be configured by horizonos-install script

# Create container management directories
mkdir -p "$BASE_DIR/var/lib/containers"
mkdir -p "$BASE_DIR/var/lib/flatpak"
mkdir -p "$BASE_DIR/home/.containers"

# Configure sudoers for container management
cat > "$BASE_DIR/etc/sudoers.d/containers" << 'EOF'
# Allow users to manage containers without password
%wheel ALL=(ALL) NOPASSWD: /usr/bin/podman, /usr/bin/buildah, /usr/bin/skopeo
EOF

# Create container helper scripts
mkdir -p "$BASE_DIR/usr/local/bin"

# Copy first boot script
if [ -f "$PROJECT_ROOT/scripts/scripts/files/horizonos-firstboot" ]; then
    cp "$PROJECT_ROOT/scripts/scripts/files/horizonos-firstboot" "$BASE_DIR/usr/local/bin/"
    chmod +x "$BASE_DIR/usr/local/bin/horizonos-firstboot"
fi

# Copy firstboot service
if [ -f "$PROJECT_ROOT/scripts/systemd-units/horizonos-firstboot.service" ]; then
    cp "$PROJECT_ROOT/scripts/systemd-units/horizonos-firstboot.service" "$BASE_DIR/etc/systemd/system/"
    arch-chroot "$BASE_DIR" systemctl enable horizonos-firstboot.service
fi

# Create container wrapper script
cat > "$BASE_DIR/usr/local/bin/container-run" << 'EOF'
#!/bin/bash
# Container wrapper script for common container operations

set -e

CONTAINER_NAME="$1"
shift

if [ -z "$CONTAINER_NAME" ]; then
    echo "Usage: container-run <container-name> [command...]"
    echo "Available containers:"
    ls -1 /etc/containers/definitions/ 2>/dev/null || echo "  No container definitions found"
    exit 1
fi

DEFINITION_FILE="/etc/containers/definitions/$CONTAINER_NAME.conf"
if [ ! -f "$DEFINITION_FILE" ]; then
    echo "Error: Container definition '$CONTAINER_NAME' not found"
    exit 1
fi

# Source container definition
source "$DEFINITION_FILE"

# Run container with appropriate settings
exec podman run --rm -it \
    --name "$CONTAINER_NAME" \
    --hostname "$CONTAINER_NAME" \
    --security-opt label=disable \
    --userns=keep-id \
    --volume /home:/home \
    --volume /tmp:/tmp \
    --workdir "$(pwd)" \
    "$CONTAINER_IMAGE" \
    "$@"
EOF

chmod +x "$BASE_DIR/usr/local/bin/container-run"

# Create container definitions directory
mkdir -p "$BASE_DIR/etc/containers/definitions"

# Create a sample development container definition
cat > "$BASE_DIR/etc/containers/definitions/development.conf" << 'EOF'
# Development container definition
CONTAINER_IMAGE="quay.io/toolbx/arch-toolbox:latest"
CONTAINER_PACKAGES="git curl vim tmux build-essential"
CONTAINER_EXPORT_BINARIES="git curl vim tmux gcc make"
EOF

# Clean up to reduce image size
echo "Cleaning up to minimize image size..."

# Remove package cache
rm -rf "$BASE_DIR/var/cache/pacman/pkg"/*

# Remove temporary files
rm -rf "$BASE_DIR/tmp"/*
rm -rf "$BASE_DIR/var/tmp"/*

# Remove logs
rm -rf "$BASE_DIR/var/log"/*

# Remove socket files that cause OSTree issues
find "$BASE_DIR" -type s -delete

# Remove other unnecessary files
rm -rf "$BASE_DIR/usr/share/doc"
rm -rf "$BASE_DIR/usr/share/man"
rm -rf "$BASE_DIR/usr/share/info"
rm -rf "$BASE_DIR/usr/share/locale"
rm -rf "$BASE_DIR/var/lib/pacman/sync"

# Create HorizonOS identification
cat > "$BASE_DIR/etc/os-release" << EOF
NAME="HorizonOS"
PRETTY_NAME="HorizonOS $HORIZONOS_VERSION ($HORIZONOS_CODENAME)"
ID=horizonos
ID_LIKE=arch
VERSION_ID=$HORIZONOS_VERSION
VERSION_CODENAME=$HORIZONOS_CODENAME
BUILD_ID=$(date +%Y%m%d)
HOME_URL="https://github.com/codeyousef/HorizonOS"
SUPPORT_URL="https://github.com/codeyousef/HorizonOS/issues"
BUG_REPORT_URL="https://github.com/codeyousef/HorizonOS/issues"
LOGO=horizonos
EOF

# Calculate final size
BASE_SIZE=$(du -sh "$BASE_DIR" | cut -f1)
echo "Base image size: $BASE_SIZE"

# Create OSTree commit
echo "Creating OSTree commit for minimal base image..."
COMMIT_MESSAGE="HorizonOS minimal base image v$HORIZONOS_VERSION (container-based architecture)"
COMMIT_HASH=$(ostree commit \
    --repo="$PROJECT_ROOT/repo" \
    --branch=horizonos/base/x86_64 \
    --subject="$COMMIT_MESSAGE" \
    --body="Minimal base image (~500MB) with container runtime support. Package count: ${#MINIMAL_PACKAGES[@]}. Architecture: Container-based with Podman, Flatpak, and Snap support." \
    "$BASE_DIR")

echo ""
echo "================================"
echo "Minimal Base Image Build Complete!"
echo "================================"
echo "Base image size: $BASE_SIZE"
echo "Package count: ${#MINIMAL_PACKAGES[@]}"
echo "OSTree commit: $COMMIT_HASH"
echo ""
echo "Base image features:"
echo "- Container runtime (Podman, Buildah, Skopeo)"
echo "- Application packaging (Flatpak, Snap)"
echo "- Essential system tools"
echo "- Network management"
echo "- Security foundations"
echo ""
echo "Next steps:"
echo "1. Run './scripts/scripts/build-test.sh' to create full system image"
echo "2. Use './scripts/tools/horizon-container' to manage system containers"
echo "3. Install user applications with Flatpak or container definitions"
echo ""
echo "OSTree branches:"
ostree refs --repo="$PROJECT_ROOT/repo"