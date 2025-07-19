#!/bin/bash
# Test script to validate getty configuration before ISO build
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

echo "=== Getty Configuration Validator ==="
echo ""

# Check if agetty exists in the system
echo "1. Checking agetty availability:"
if command -v agetty &> /dev/null; then
    echo "   ✓ agetty found at: $(which agetty)"
    echo "   Version: $(agetty --version 2>&1 | head -1)"
else
    echo "   ✗ agetty not found in PATH"
fi

# Check actual paths
echo ""
echo "2. Checking agetty paths:"
for path in /usr/bin/agetty /sbin/agetty /usr/sbin/agetty; do
    if [ -f "$path" ]; then
        echo "   ✓ Found: $path"
        ls -la "$path"
    else
        echo "   ✗ Not found: $path"
    fi
done

# Check systemd getty services
echo ""
echo "3. Checking systemd getty services:"
for service in /usr/lib/systemd/system/getty@.service /lib/systemd/system/getty@.service; do
    if [ -f "$service" ]; then
        echo "   ✓ Found: $service"
        # Check ExecStart line
        echo "   ExecStart: $(grep "^ExecStart=" "$service" | head -1)"
    else
        echo "   ✗ Not found: $service"
    fi
done

# Check if running as root for build test
echo ""
echo "4. Build environment check:"
if [ "$EUID" -eq 0 ]; then
    echo "   ✓ Running as root (required for ISO build)"
else
    echo "   ⚠ Not running as root (sudo required for ISO build)"
fi

# Check archiso installation
echo ""
echo "5. Checking archiso installation:"
if command -v mkarchiso &> /dev/null; then
    echo "   ✓ mkarchiso found"
    mkarchiso -V
else
    echo "   ✗ mkarchiso not found (install archiso package)"
fi

# Test the getty fix script
echo ""
echo "6. Testing getty fix script:"
GETTY_FIX="$PROJECT_ROOT/scripts/scripts/boot-fixes/getty-comprehensive-fix.sh"
if [ -f "$GETTY_FIX" ]; then
    echo "   ✓ Getty fix script found"
    if [ -x "$GETTY_FIX" ]; then
        echo "   ✓ Script is executable"
    else
        echo "   ✗ Script is not executable"
    fi
    # Source and test the function
    source "$GETTY_FIX"
    if type -t apply_comprehensive_getty_fix &> /dev/null; then
        echo "   ✓ Function apply_comprehensive_getty_fix is available"
    else
        echo "   ✗ Function apply_comprehensive_getty_fix not found"
    fi
else
    echo "   ✗ Getty fix script not found at: $GETTY_FIX"
fi

echo ""
echo "=== Summary ==="
echo "This validator checks the build environment for getty configuration."
echo "Run 'sudo ./scripts/scripts/build-iso.sh' to build the ISO with the fixes."
echo ""