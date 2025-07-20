#!/bin/bash
# Test complete implementation following Boot Process guide

set -e

echo "=== Testing Complete Implementation ==="
echo ""

ERRORS=0

# Test 1: Check mkinitcpio configuration will be copied
echo "1. Checking mkinitcpio configuration..."
if grep -q "cp.*mkinitcpio.conf.d/archiso.conf" /home/yousef/Development/horizonos/scripts/scripts/build-iso.sh; then
    echo "✓ mkinitcpio archiso.conf will be copied"
else
    echo "✗ mkinitcpio configuration not copied"
    ((ERRORS++))
fi

# Test 2: Check critical packages
echo ""
echo "2. Checking critical packages..."
PACKAGES_SECTION=$(sed -n '/^cat > packages.x86_64/,/^EOF/p' /home/yousef/Development/horizonos/scripts/scripts/build-iso.sh)

for pkg in "amd-ucode" "intel-ucode" "linux-firmware-marvell" "mkinitcpio-archiso" "mkinitcpio-nfs-utils"; do
    if echo "$PACKAGES_SECTION" | grep -q "^$pkg$"; then
        echo "✓ $pkg included"
    else
        echo "✗ $pkg missing"
        ((ERRORS++))
    fi
done

# Test 3: Check boot parameters
echo ""
echo "3. Checking boot parameter fixes..."
if grep -q "quiet loglevel=3" /home/yousef/Development/horizonos/scripts/scripts/build-iso.sh; then
    echo "✓ Boot parameters include 'quiet loglevel=3'"
else
    echo "✗ Boot parameters not fixed"
    ((ERRORS++))
fi

# Test 4: Check diagnostic tools
echo ""
echo "4. Checking diagnostic tools..."
if [ -x /home/yousef/Development/horizonos/scripts/tools/live-diagnostic ]; then
    echo "✓ live-diagnostic tool exists and is executable"
else
    echo "✗ live-diagnostic tool missing"
    ((ERRORS++))
fi

# Test 5: Check customize_airootfs.sh
echo ""
echo "5. Checking customize_airootfs.sh..."
if [ -f /home/yousef/Development/horizonos/scripts/archiso/customize_airootfs.sh ]; then
    echo "✓ customize_airootfs.sh exists"
    if grep -q "systemctl mask getty@tty1" /home/yousef/Development/horizonos/scripts/archiso/customize_airootfs.sh; then
        echo "✓ Getty masking configured"
    else
        echo "✗ Getty not masked"
        ((ERRORS++))
    fi
else
    echo "✗ customize_airootfs.sh missing"
    ((ERRORS++))
fi

# Test 6: Check validation script updates
echo ""
echo "6. Checking validation script..."
if grep -q "mkinitcpio archiso.conf" /home/yousef/Development/horizonos/scripts/validate-iso-config.sh; then
    echo "✓ Validation checks mkinitcpio configuration"
else
    echo "✗ Validation doesn't check mkinitcpio"
    ((ERRORS++))
fi

echo ""
echo "=== Summary ==="
if [ $ERRORS -eq 0 ]; then
    echo "✓ All tests passed!"
    echo "Implementation follows Boot Process guide requirements:"
    echo "  - mkinitcpio archiso hooks configured"
    echo "  - Microcode packages included"
    echo "  - Boot parameters suppress hardware warnings"
    echo "  - Diagnostic tools available"
    echo "  - Getty properly masked"
else
    echo "✗ $ERRORS errors found"
    echo "Fix these issues before building ISO"
fi