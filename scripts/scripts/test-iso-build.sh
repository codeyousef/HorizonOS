#!/usr/bin/env bash
set -euo pipefail

# Test script to verify ISO build prerequisites

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BUILD_DIR="$PROJECT_ROOT/build"
ISO_DIR="$PROJECT_ROOT/iso"

echo "=== HorizonOS ISO Build Test ==="
echo "Project root: $PROJECT_ROOT"
echo ""

# Check configuration file
echo "1. Checking configuration..."
if [ -f "$PROJECT_ROOT/config/dev.conf" ]; then
    source "$PROJECT_ROOT/config/dev.conf"
    echo "   ✓ Configuration loaded"
    echo "     Version: $HORIZONOS_VERSION"
    echo "     Codename: $HORIZONOS_CODENAME"
else
    echo "   ✗ Configuration file not found"
    exit 1
fi

# Check required tools
echo ""
echo "2. Checking required tools..."
for tool in mkarchiso ostree parted mkfs.btrfs mkfs.fat; do
    if command -v $tool &> /dev/null; then
        echo "   ✓ $tool found"
    else
        echo "   ✗ $tool not found"
    fi
done

# Check OSTree repository
echo ""
echo "3. Checking OSTree repository..."
if [ -d "$PROJECT_ROOT/repo" ]; then
    echo "   ✓ OSTree repository exists"
    if ostree log --repo="$PROJECT_ROOT/repo" horizonos/test/x86_64 &> /dev/null; then
        echo "   ✓ OSTree commits found"
        LATEST_COMMIT=$(ostree --repo="$PROJECT_ROOT/repo" rev-parse horizonos/test/x86_64)
        echo "     Latest commit: ${LATEST_COMMIT:0:12}..."
    else
        echo "   ✗ No OSTree commits found"
    fi
else
    echo "   ✗ OSTree repository not found"
fi

# Check archiso profile template
echo ""
echo "4. Checking archiso profile template..."
if [ -d "/usr/share/archiso/configs/releng" ]; then
    echo "   ✓ Archiso releng profile found"
else
    echo "   ✗ Archiso releng profile not found"
fi

# Check if running as root
echo ""
echo "5. Checking permissions..."
if [ "$EUID" -ne 0 ]; then 
    echo "   ⚠ Not running as root (will be required for actual build)"
else
    echo "   ✓ Running as root"
fi

# Check disk space
echo ""
echo "6. Checking disk space..."
AVAILABLE_SPACE=$(df -BG "$PROJECT_ROOT" | tail -1 | awk '{print $4}' | sed 's/G//')
if [ "$AVAILABLE_SPACE" -gt 10 ]; then
    echo "   ✓ Sufficient disk space (${AVAILABLE_SPACE}G available)"
else
    echo "   ⚠ Low disk space (${AVAILABLE_SPACE}G available, recommend >10G)"
fi

echo ""
echo "=== Test Complete ==="
echo ""
echo "To build the ISO, run:"
echo "  sudo ./scripts/scripts/build-iso.sh"