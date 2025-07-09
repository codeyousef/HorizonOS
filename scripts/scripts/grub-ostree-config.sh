#!/usr/bin/env bash
# Generate GRUB configuration for OSTree deployments

set -euo pipefail

SYSROOT=${1:-/}
GRUB_CFG="$SYSROOT/boot/grub/grub.cfg"

echo "Generating OSTree-aware GRUB configuration..."

cat > "$GRUB_CFG" << 'EOF'
# GRUB configuration for HorizonOS with OSTree

set default=0
set timeout=5

# Load modules
insmod part_gpt
insmod btrfs
insmod ext2
insmod fat

# Set root based on where /boot is
search --no-floppy --fs-uuid --set=root $(findmnt -no UUID /boot)

# OSTree deployments will be added by ostree-grub-generator
# This is a placeholder that will be replaced during boot

menuentry 'HorizonOS' {
    linux /boot/ostree/horizonos-*/vmlinuz root=UUID=$(findmnt -no UUID /) rootflags=subvol=@ rw quiet
    initrd /boot/ostree/horizonos-*/initramfs.img
}

menuentry 'HorizonOS (Recovery Mode)' {
    linux /boot/ostree/horizonos-*/vmlinuz root=UUID=$(findmnt -no UUID /) rootflags=subvol=@ rw single
    initrd /boot/ostree/horizonos-*/initramfs.img
}
EOF

echo "GRUB configuration written to $GRUB_CFG"