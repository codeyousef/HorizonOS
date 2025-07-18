#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BUILD_DIR="$PROJECT_ROOT/build"
ROOTFS_DIR="$BUILD_DIR/rootfs"

# Load configuration
source "$PROJECT_ROOT/config/dev.conf"

echo "=== HorizonOS Test Build ==="
echo "Project root: $PROJECT_ROOT"
echo "Version: $HORIZONOS_VERSION"
echo "Architecture: Container-based"

# Check if minimal base image exists
BASE_BUILDER="$PROJECT_ROOT/scripts/scripts/build-base-image.sh"
if [ ! -f "$BASE_BUILDER" ]; then
    echo "Error: Base image builder not found at $BASE_BUILDER"
    exit 1
fi

# Check if we have a base image commit
if ! ostree --repo="$PROJECT_ROOT/repo" rev-parse horizonos/base/x86_64 &> /dev/null; then
    echo "Base image not found. Building minimal base image first..."
    echo "This will create a ~500MB container-optimized base image"
    sudo "$BASE_BUILDER"
fi

# Clean previous build
sudo rm -rf "$ROOTFS_DIR"
mkdir -p "$ROOTFS_DIR"

# Extract base image from OSTree
echo "Extracting base image from OSTree..."
BASE_COMMIT=$(ostree --repo="$PROJECT_ROOT/repo" rev-parse horizonos/base/x86_64)
echo "Using base commit: $BASE_COMMIT"

sudo ostree --repo="$PROJECT_ROOT/repo" checkout "$BASE_COMMIT" "$ROOTFS_DIR"

# Container-specific configuration
echo "Configuring container-based system..."

# Add container image definitions for system packages
sudo mkdir -p "$ROOTFS_DIR/etc/containers/system"

# Create development tools container definition
sudo tee "$ROOTFS_DIR/etc/containers/system/development.json" > /dev/null << 'EOF'
{
  "name": "development",
  "image": "quay.io/toolbx/arch-toolbox:latest",
  "purpose": "development",
  "packages": ["git", "curl", "vim", "tmux", "build-essential", "nodejs", "python", "rust", "go"],
  "export_binaries": ["git", "curl", "vim", "tmux", "gcc", "make", "node", "npm", "python", "rustc", "cargo", "go"],
  "auto_start": false,
  "persistent": true,
  "mounts": ["/home", "/tmp", "/var/cache/build"],
  "environment": {
    "CONTAINER_PURPOSE": "development",
    "PATH": "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
  }
}
EOF

# Create multimedia tools container definition
sudo tee "$ROOTFS_DIR/etc/containers/system/multimedia.json" > /dev/null << 'EOF'
{
  "name": "multimedia",
  "image": "docker.io/linuxserver/ffmpeg:latest",
  "purpose": "multimedia",
  "packages": ["ffmpeg", "imagemagick", "sox", "mediainfo"],
  "export_binaries": ["ffmpeg", "convert", "sox", "mediainfo"],
  "auto_start": false,
  "persistent": false,
  "mounts": ["/home", "/tmp"],
  "environment": {
    "CONTAINER_PURPOSE": "multimedia"
  }
}
EOF

# Create gaming container definition
sudo tee "$ROOTFS_DIR/etc/containers/system/gaming.json" > /dev/null << 'EOF'
{
  "name": "gaming",
  "image": "docker.io/steamcmd/steamcmd:latest",
  "purpose": "gaming",
  "packages": ["steam", "lutris", "wine", "gamemode", "mangohud"],
  "export_binaries": ["steam", "lutris", "wine"],
  "auto_start": false,
  "persistent": true,
  "mounts": ["/home", "/tmp", "/dev/dri"],
  "environment": {
    "CONTAINER_PURPOSE": "gaming",
    "DISPLAY": ":0"
  }
}
EOF

# Install HorizonOS tools
echo "Installing HorizonOS tools..."
sudo mkdir -p "$ROOTFS_DIR/usr/local/bin"
sudo cp "$PROJECT_ROOT/scripts/tools/horizon-container" "$ROOTFS_DIR/usr/local/bin/"
sudo cp "$PROJECT_ROOT/scripts/tools/horizon-container-startup" "$ROOTFS_DIR/usr/local/bin/"
sudo cp "$PROJECT_ROOT/scripts/tools/horizon-container-shutdown" "$ROOTFS_DIR/usr/local/bin/"
sudo cp "$PROJECT_ROOT/scripts/tools/horizonos-autoupdate" "$ROOTFS_DIR/usr/local/bin/"
sudo cp "$PROJECT_ROOT/scripts/tools/horizonos-update-notify" "$ROOTFS_DIR/usr/local/bin/"
sudo chmod +x "$ROOTFS_DIR"/usr/local/bin/*

# Install systemd units for auto-update system
echo "Installing auto-update system..."
sudo mkdir -p "$ROOTFS_DIR/etc/systemd/system"
sudo cp "$PROJECT_ROOT/scripts/systemd-units/horizonos-update.service" "$ROOTFS_DIR/etc/systemd/system/"
sudo cp "$PROJECT_ROOT/scripts/systemd-units/horizonos-update.timer" "$ROOTFS_DIR/etc/systemd/system/"
sudo cp "$PROJECT_ROOT/scripts/systemd-units/horizonos-update-notify.service" "$ROOTFS_DIR/etc/systemd/system/"

# Create update configuration
sudo mkdir -p "$ROOTFS_DIR/etc/horizonos"
sudo tee "$ROOTFS_DIR/etc/horizonos/update.conf" > /dev/null << 'EOF'
# HorizonOS Update Configuration
UPDATE_CHANNEL="stable"
AUTO_STAGE="true"
AUTO_REBOOT="false"
CHECK_INTERVAL="86400"
GITHUB_REPO="codeyousef/HorizonOS"
EOF

# Enable update timer by default
sudo mkdir -p "$ROOTFS_DIR/etc/systemd/system/timers.target.wants"
sudo ln -sf /etc/systemd/system/horizonos-update.timer \
    "$ROOTFS_DIR/etc/systemd/system/timers.target.wants/horizonos-update.timer"

# Create update cache directory
sudo mkdir -p "$ROOTFS_DIR/var/cache/horizonos/updates"

# Update version information
sudo tee "$ROOTFS_DIR/etc/horizonos-release" > /dev/null << EOF
HORIZONOS_VERSION="$HORIZONOS_VERSION"
HORIZONOS_CODENAME="$HORIZONOS_CODENAME"
HORIZONOS_ARCHITECTURE="container-based"
BUILD_DATE="$(date -Iseconds)"
BASE_COMMIT="$BASE_COMMIT"
EOF

# Clean up problematic files before OSTree commit
echo "Cleaning up rootfs..."
sudo find "$ROOTFS_DIR" -type s -delete  # Remove socket files
sudo rm -rf "$ROOTFS_DIR"/var/cache/pacman/pkg/* 2>/dev/null || true
sudo rm -rf "$ROOTFS_DIR"/var/log/* 2>/dev/null || true
sudo rm -rf "$ROOTFS_DIR"/tmp/* 2>/dev/null || true
sudo rm -rf "$ROOTFS_DIR"/var/tmp/* 2>/dev/null || true

# Create test OSTree commit
echo "Creating OSTree commit..."
COMMIT_MESSAGE="HorizonOS test build v$HORIZONOS_VERSION (container-based)"
COMMIT_BODY="Container-based HorizonOS build based on minimal base image. Includes system container definitions for development, multimedia, and gaming workloads."

sudo ostree commit \
    --repo="$PROJECT_ROOT/repo" \
    --branch=horizonos/test/x86_64 \
    --subject="$COMMIT_MESSAGE" \
    --body="$COMMIT_BODY" \
    "$ROOTFS_DIR"

echo ""
echo "================================"
echo "Container-based Test Build Complete!"
echo "================================"
echo "Version: $HORIZONOS_VERSION"
echo "Architecture: Container-based"
echo "Base commit: $BASE_COMMIT"
echo ""
echo "Available system containers:"
echo "- development (dev tools, compilers, languages)"
echo "- multimedia (ffmpeg, imagemagick, media tools)"
echo "- gaming (steam, lutris, wine)"
echo ""
echo "Next steps:"
echo "1. Build ISO: sudo ./scripts/scripts/build-iso.sh"
echo "2. Test with QEMU or write to USB"
echo "3. Use horizon-container tool to manage containers"
echo ""
ostree log --repo="$PROJECT_ROOT/repo" horizonos/test/x86_64 | head -10
