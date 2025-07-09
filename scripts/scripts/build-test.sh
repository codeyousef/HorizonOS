#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD_DIR="$PROJECT_ROOT/build"
ROOTFS_DIR="$BUILD_DIR/rootfs"

echo "=== HorizonOS Test Build ==="
echo "Project root: $PROJECT_ROOT"

# Clean previous build
sudo rm -rf "$ROOTFS_DIR"
mkdir -p "$ROOTFS_DIR"

# Create a minimal Arch rootfs
echo "Creating base rootfs..."
sudo pacstrap -c "$ROOTFS_DIR" \
    base base-devel \
    linux linux-firmware \
    btrfs-progs \
    networkmanager \
    fish \
    sudo \
    htop \
    git

# Basic configuration
echo "Configuring rootfs..."
echo "horizonos" | sudo tee "$ROOTFS_DIR/etc/hostname" > /dev/null
echo "en_US.UTF-8 UTF-8" | sudo tee "$ROOTFS_DIR/etc/locale.gen" > /dev/null
sudo arch-chroot "$ROOTFS_DIR" locale-gen

# Clean up problematic files before OSTree commit
echo "Cleaning up rootfs..."
sudo find "$ROOTFS_DIR" -type s -delete  # Remove socket files
sudo rm -rf "$ROOTFS_DIR"/var/cache/pacman/pkg/*
sudo rm -rf "$ROOTFS_DIR"/var/log/*
sudo rm -rf "$ROOTFS_DIR"/tmp/*
sudo rm -rf "$ROOTFS_DIR"/var/tmp/*

# Create test OSTree commit
echo "Creating OSTree commit..."
sudo ostree commit \
    --repo="$PROJECT_ROOT/repo" \
    --branch=horizonos/test/x86_64 \
    --subject="Test build $(date +%Y%m%d-%H%M%S)" \
    "$ROOTFS_DIR"

echo "Build complete! OSTree commit created."
ostree log --repo="$PROJECT_ROOT/repo" horizonos/test/x86_64
