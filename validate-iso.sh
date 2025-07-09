#!/bin/bash
# Validate HorizonOS ISO contents

ISO_PATH="/home/yousef/horizonos/build/out/horizonos-0.1.0-dev-x86_64.iso"
MOUNT_POINT="/tmp/horizonos-iso-mount"

echo "=== HorizonOS ISO Validation ==="
echo "ISO: $ISO_PATH"
echo ""

# Create mount point
sudo mkdir -p "$MOUNT_POINT"

# Mount ISO
echo "Mounting ISO..."
sudo mount -o loop,ro "$ISO_PATH" "$MOUNT_POINT"

echo ""
echo "ISO Contents:"
ls -la "$MOUNT_POINT/"

echo ""
echo "Checking for key components..."

# Check for bootloader
if [ -d "$MOUNT_POINT/boot" ]; then
    echo "✓ Boot directory found"
else
    echo "✗ Boot directory missing"
fi

# Check for kernel
if ls "$MOUNT_POINT/arch/boot/x86_64/vmlinuz-"* >/dev/null 2>&1; then
    echo "✓ Kernel found"
else
    echo "✗ Kernel missing"
fi

# Check for installer
if sudo mount -o loop,ro "$MOUNT_POINT/arch/x86_64/airootfs.sfs" /mnt 2>/dev/null; then
    if [ -f "/mnt/usr/local/bin/horizonos-install" ]; then
        echo "✓ HorizonOS installer found"
    else
        echo "✗ HorizonOS installer missing"
    fi
    
    # Check for OSTree repository
    if [ -d "/mnt/usr/share/horizonos/repo" ]; then
        echo "✓ OSTree repository included"
        echo "  Repository size: $(du -sh /mnt/usr/share/horizonos/repo | cut -f1)"
    else
        echo "✗ OSTree repository not found"
    fi
    
    sudo umount /mnt
fi

# Unmount ISO
echo ""
echo "Unmounting ISO..."
sudo umount "$MOUNT_POINT"
sudo rmdir "$MOUNT_POINT"

echo ""
echo "=== Validation Complete ==="