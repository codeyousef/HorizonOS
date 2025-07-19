#!/bin/bash
# Test script to verify getty configuration

echo "=== Testing Getty Configuration ==="
echo ""

# Create a temporary directory to test the configuration
TEMP_DIR="/tmp/horizonos-getty-test-$$"
mkdir -p "$TEMP_DIR"

# Source the getty configuration script
source "$(dirname "$0")/boot-fixes/getty-archiso-standard.sh"

# Apply the configuration to the test directory
apply_standard_getty_fix "$TEMP_DIR"

echo "Configuration created. Checking results..."
echo ""

# Check if the configuration was created correctly
if [ -f "$TEMP_DIR/etc/systemd/system/getty@tty1.service.d/autologin.conf" ]; then
    echo "✓ Autologin configuration file created"
    echo ""
    echo "Contents:"
    cat "$TEMP_DIR/etc/systemd/system/getty@tty1.service.d/autologin.conf"
    echo ""
    
    # Check for correct agetty path
    if grep -q "/usr/bin/agetty" "$TEMP_DIR/etc/systemd/system/getty@tty1.service.d/autologin.conf"; then
        echo "✓ Correct agetty path (/usr/bin/agetty)"
    else
        echo "✗ Incorrect agetty path"
    fi
    
    # Check for autologin parameter
    if grep -q -- "--autologin root" "$TEMP_DIR/etc/systemd/system/getty@tty1.service.d/autologin.conf"; then
        echo "✓ Autologin configured for root user"
    else
        echo "✗ Autologin not configured"
    fi
    
else
    echo "✗ Configuration file was not created"
fi

# Clean up
rm -rf "$TEMP_DIR"

echo ""
echo "Test complete."