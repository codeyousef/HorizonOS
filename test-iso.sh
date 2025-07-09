#!/bin/bash
# Test HorizonOS ISO in QEMU

ISO_PATH="/home/yousef/horizonos/build/out/horizonos-0.1.0-dev-x86_64.iso"

echo "Testing HorizonOS ISO boot..."
echo "ISO: $ISO_PATH"
echo ""
echo "Starting QEMU with graphical display..."
echo "Once booted, you can test the installer with: horizonos-install"
echo ""

# Run QEMU with proper UEFI firmware
qemu-system-x86_64 \
    -m 4G \
    -enable-kvm \
    -cdrom "$ISO_PATH" \
    -boot d \
    -vga std \
    -usb -device usb-tablet