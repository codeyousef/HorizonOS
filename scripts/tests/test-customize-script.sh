#!/bin/bash
# Test the customize_airootfs.sh approach

set -e

echo "=== Testing customize_airootfs.sh Implementation ==="

# Check if the script exists
if [ -f "/home/yousef/Development/horizonos/iso/airootfs/root/customize_airootfs.sh" ]; then
    echo "✓ customize_airootfs.sh exists"
else
    echo "✗ customize_airootfs.sh missing"
    exit 1
fi

# Check if it's executable
if [ -x "/home/yousef/Development/horizonos/iso/airootfs/root/customize_airootfs.sh" ]; then
    echo "✓ customize_airootfs.sh is executable"
else
    echo "✗ customize_airootfs.sh is not executable"
    exit 1
fi

# Check key components in the script
echo ""
echo "Checking script contents:"

if grep -q "useradd.*liveuser" /home/yousef/Development/horizonos/iso/airootfs/root/customize_airootfs.sh; then
    echo "✓ Creates liveuser (not root autologin)"
else
    echo "✗ No liveuser creation found"
fi

if grep -q "systemctl mask getty@tty1" /home/yousef/Development/horizonos/iso/airootfs/root/customize_airootfs.sh; then
    echo "✓ Masks getty@tty1 (following guide's recommendation)"
else
    echo "✗ Getty@tty1 not masked"
fi

if grep -q "systemctl enable NetworkManager" /home/yousef/Development/horizonos/iso/airootfs/root/customize_airootfs.sh; then
    echo "✓ Enables NetworkManager"
else
    echo "✗ NetworkManager not enabled"
fi

if grep -q "systemctl set-default multi-user.target" /home/yousef/Development/horizonos/iso/airootfs/root/customize_airootfs.sh; then
    echo "✓ Sets multi-user.target as default"
else
    echo "✗ Default target not set"
fi

echo ""
echo "Checking build script integration:"

if grep -q "customize_airootfs.sh" /home/yousef/Development/horizonos/scripts/scripts/build-iso.sh; then
    echo "✓ build-iso.sh copies customize_airootfs.sh"
else
    echo "✗ build-iso.sh doesn't use customize_airootfs.sh"
fi

if grep -q "^systemd$" /home/yousef/Development/horizonos/scripts/scripts/build-iso.sh; then
    echo "✓ systemd package explicitly included"
else
    echo "✗ systemd package not explicitly listed"
fi

echo ""
echo "=== Test Complete ==="
echo "This implementation follows the Boot Process guide's working example."