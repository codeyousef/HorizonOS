#!/bin/bash
# Test that we're not corrupting boot configurations

echo "=== Testing Boot Configuration Integrity ==="
echo ""

ERRORS=0

# Check that we're NOT modifying boot parameters incorrectly
echo "1. Checking for harmful boot modifications..."

# Check for removed harmful patterns
if grep -q "sed.*archisobasedir.*systemd.unit" /home/yousef/Development/horizonos/scripts/scripts/build-iso.sh; then
    echo "✗ Still modifying archisobasedir parameters"
    ((ERRORS++))
else
    echo "✓ Not modifying archisobasedir parameters"
fi

if grep -q "sed.*HORIZONOS_%Y%m%d%H%M%S.*quiet loglevel" /home/yousef/Development/horizonos/scripts/scripts/build-iso.sh; then
    echo "✗ Still modifying boot parameters with sed"
    ((ERRORS++))
else
    echo "✓ Not modifying boot parameters with sed"
fi

if grep -q "menuentry.*HorizonOS.*Debug Mode" /home/yousef/Development/horizonos/scripts/scripts/build-iso.sh; then
    echo "✗ Still adding custom boot entries"
    ((ERRORS++))
else
    echo "✓ Not adding custom boot entries"
fi

# Check profiledef.sh handling
echo ""
echo "2. Checking profiledef.sh handling..."

if grep -q "sed.*install_dir" /home/yousef/Development/horizonos/scripts/scripts/build-iso.sh; then
    echo "✗ Modifying install_dir"
    ((ERRORS++))
else
    echo "✓ Not modifying install_dir"
fi

if grep -q "Keep install_dir as.*arch" /home/yousef/Development/horizonos/scripts/scripts/build-iso.sh; then
    echo "✓ Comment about keeping install_dir found"
else
    echo "⚠ Warning: No comment about keeping install_dir"
fi

# Check for archiso compatibility
echo ""
echo "3. Checking archiso compatibility..."

if grep -q "cp -r /usr/share/archiso/configs/releng" /home/yousef/Development/horizonos/scripts/scripts/build-iso.sh; then
    echo "✓ Using releng profile as base"
else
    echo "✗ Not using releng profile"
    ((ERRORS++))
fi

echo ""
echo "=== Summary ==="
if [ $ERRORS -eq 0 ]; then
    echo "✓ Boot configuration integrity maintained"
    echo "The build script should now:"
    echo "  - Let archiso handle all boot parameters"
    echo "  - Keep standard install_dir='arch'"
    echo "  - Not corrupt boot configuration files"
    echo "  - Boot properly past SMBus/EDD messages"
else
    echo "✗ $ERRORS issues found"
    echo "Boot configuration may still be corrupted"
fi