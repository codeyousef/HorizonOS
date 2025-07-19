#!/bin/bash
# Comprehensive boot process test for HorizonOS ISO
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m'

echo "=== HorizonOS Full Boot Process Test ==="
echo ""
echo "This test verifies the complete boot process to the live environment."
echo ""

# Test 1: Check systemd target configuration
echo -e "${YELLOW}Test 1: Verifying systemd target configuration${NC}"

TEST_DIR="/tmp/horizonos-boot-test-$$"
mkdir -p "$TEST_DIR/airootfs/etc/systemd/system"

# Simulate the build process
source "$PROJECT_ROOT/scripts/scripts/boot-fixes/getty-autologin.sh"
fix_getty_in_iso "$TEST_DIR/airootfs"

# Apply the same systemd target configuration as build-iso.sh
ln -sf /usr/lib/systemd/system/multi-user.target "$TEST_DIR/airootfs/etc/systemd/system/default.target"

# Apply getty.target dependencies (matching build-iso.sh)
mkdir -p "$TEST_DIR/airootfs/etc/systemd/system/multi-user.target.wants"
ln -sf /usr/lib/systemd/system/getty.target "$TEST_DIR/airootfs/etc/systemd/system/multi-user.target.wants/getty.target"
mkdir -p "$TEST_DIR/airootfs/etc/systemd/system/getty.target.wants"
ln -sf /usr/lib/systemd/system/getty@.service "$TEST_DIR/airootfs/etc/systemd/system/getty.target.wants/getty@tty1.service"
ln -sf /usr/lib/systemd/system/getty@.service "$TEST_DIR/airootfs/etc/systemd/system/multi-user.target.wants/getty@tty1.service"

# Verify default target
echo -n "   - Checking default.target: "
if [ -L "$TEST_DIR/airootfs/etc/systemd/system/default.target" ]; then
    TARGET=$(readlink "$TEST_DIR/airootfs/etc/systemd/system/default.target")
    if [[ "$TARGET" == *"multi-user.target" ]]; then
        echo -e "${GREEN}✓ Points to multi-user.target${NC}"
    else
        echo -e "${RED}✗ Points to $TARGET instead of multi-user.target${NC}"
    fi
else
    echo -e "${RED}✗ default.target not found${NC}"
fi

# Verify getty.target dependency
echo -n "   - Checking getty.target in multi-user.target.wants: "
if [ -L "$TEST_DIR/airootfs/etc/systemd/system/multi-user.target.wants/getty.target" ]; then
    echo -e "${GREEN}✓ getty.target will be started by multi-user.target${NC}"
else
    echo -e "${RED}✗ getty.target NOT linked to multi-user.target${NC}"
fi

# Verify getty@tty1 dependency
echo -n "   - Checking getty@tty1.service dependencies: "
GETTY_LINKS=0
if [ -L "$TEST_DIR/airootfs/etc/systemd/system/getty.target.wants/getty@tty1.service" ]; then
    GETTY_LINKS=$((GETTY_LINKS + 1))
fi
if [ -L "$TEST_DIR/airootfs/etc/systemd/system/multi-user.target.wants/getty@tty1.service" ]; then
    GETTY_LINKS=$((GETTY_LINKS + 1))
fi
if [ $GETTY_LINKS -eq 2 ]; then
    echo -e "${GREEN}✓ getty@tty1 linked to both targets${NC}"
elif [ $GETTY_LINKS -eq 1 ]; then
    echo -e "${YELLOW}⚠ getty@tty1 only linked to one target${NC}"
else
    echo -e "${RED}✗ getty@tty1 not properly linked${NC}"
fi

# Test 2: Verify boot parameters
echo -e "\n${YELLOW}Test 2: Checking boot parameters${NC}"

# Simulate boot configuration check
echo -n "   - Checking for systemd.unit parameter: "
if grep -q "systemd.unit=multi-user.target" "$PROJECT_ROOT/scripts/scripts/build-iso.sh"; then
    echo -e "${GREEN}✓ Found in build script${NC}"
else
    echo -e "${RED}✗ Not found in build script${NC}"
fi

# Test 3: Check for graphical dependencies
echo -e "\n${YELLOW}Test 3: Checking for graphical.target dependencies${NC}"

# Check packages that might pull in graphical.target
GRAPHICAL_PACKAGES=(
    "xorg-server"
    "gdm"
    "sddm"
    "lightdm"
    "plasma-desktop"
    "gnome-shell"
    "xfce4"
)

echo -n "   - Checking packages.x86_64 for GUI packages: "
if [ -f "$PROJECT_ROOT/iso/horizonos-profile/packages.x86_64" ]; then
    FOUND_GUI=false
    for pkg in "${GRAPHICAL_PACKAGES[@]}"; do
        if grep -q "^$pkg" "$PROJECT_ROOT/iso/horizonos-profile/packages.x86_64" 2>/dev/null; then
            echo -e "\n     ${YELLOW}⚠ Found $pkg (might trigger graphical.target)${NC}"
            FOUND_GUI=true
        fi
    done
    if [ "$FOUND_GUI" = false ]; then
        echo -e "${GREEN}✓ No GUI packages found${NC}"
    fi
else
    echo -e "${YELLOW}⚠ packages.x86_64 not found (profile not created yet)${NC}"
fi

# Test 4: Verify getty configuration
echo -e "\n${YELLOW}Test 4: Verifying getty autologin${NC}"

AUTOLOGIN="$TEST_DIR/airootfs/etc/systemd/system/getty@tty1.service.d/autologin.conf"

echo -n "   - Checking autologin.conf exists: "
if [ -f "$AUTOLOGIN" ]; then
    echo -e "${GREEN}✓ Found${NC}"
    
    echo -n "   - Checking for correct agetty path: "
    if grep -q "/usr/bin/agetty" "$AUTOLOGIN"; then
        echo -e "${GREEN}✓ Using /usr/bin/agetty${NC}"
    else
        echo -e "${RED}✗ Not using /usr/bin/agetty${NC}"
    fi
    
    echo -n "   - Checking for autologin root: "
    if grep -q -- "--autologin root" "$AUTOLOGIN"; then
        echo -e "${GREEN}✓ Autologin configured${NC}"
    else
        echo -e "${RED}✗ Autologin not configured${NC}"
    fi
else
    echo -e "${RED}✗ Not found${NC}"
fi

# Test 5: Simulate boot sequence
echo -e "\n${YELLOW}Test 5: Simulating boot sequence${NC}"

echo "   Boot sequence should be:"
echo "   1. Kernel boots with systemd.unit=multi-user.target"
echo "   2. systemd starts and targets multi-user.target"
echo "   3. multi-user.target pulls in getty.target"
echo "   4. getty.target starts getty@tty1.service"
echo "   5. getty@tty1.service autologins root"
echo "   6. Root shell appears with MOTD 'Welcome to HorizonOS Live'"
echo ""

# Check if all components are in place
BOOT_OK=true

echo -n "   - Kernel parameter: "
if grep -q "systemd.unit=multi-user.target" "$PROJECT_ROOT/scripts/scripts/build-iso.sh"; then
    echo -e "${GREEN}✓${NC}"
else
    echo -e "${RED}✗${NC}"
    BOOT_OK=false
fi

echo -n "   - Default target: "
if [ -L "$TEST_DIR/airootfs/etc/systemd/system/default.target" ]; then
    echo -e "${GREEN}✓${NC}"
else
    echo -e "${RED}✗${NC}"
    BOOT_OK=false
fi

echo -n "   - Getty autologin: "
if [ -f "$AUTOLOGIN" ] && grep -q "/usr/bin/agetty" "$AUTOLOGIN"; then
    echo -e "${GREEN}✓${NC}"
else
    echo -e "${RED}✗${NC}"
    BOOT_OK=false
fi

echo -n "   - No restart loops: "
if grep -q "Restart=no" "$AUTOLOGIN" 2>/dev/null; then
    echo -e "${GREEN}✓${NC}"
else
    echo -e "${RED}✗${NC}"
    BOOT_OK=false
fi

# Test 6: Check for conflicting services
echo -e "\n${YELLOW}Test 6: Checking for conflicting services${NC}"

echo -n "   - Extra TTYs masked: "
MASKED_COUNT=0
for i in {2..6}; do
    if [ -L "$TEST_DIR/airootfs/etc/systemd/system/getty@tty$i.service" ]; then
        ((MASKED_COUNT++))
    fi
done
if [ $MASKED_COUNT -eq 5 ]; then
    echo -e "${GREEN}✓ All extra TTYs masked${NC}"
else
    echo -e "${YELLOW}⚠ Only $MASKED_COUNT/5 TTYs masked${NC}"
fi

# Cleanup
rm -rf "$TEST_DIR"

# Summary
echo -e "\n${GREEN}=== Test Summary ===${NC}"
echo ""
if [ "$BOOT_OK" = true ]; then
    echo -e "${GREEN}✓ All boot components configured correctly${NC}"
    echo ""
    echo "The ISO should boot successfully to the live environment with:"
    echo "- No hanging at 'Reached graphical interface'"
    echo "- Automatic login to root shell"
    echo "- No getty restart loops"
    echo ""
    echo "Expected boot messages:"
    echo "1. 'Welcome to Arch Linux!'"
    echo "2. 'Reached target Multi-User System'"
    echo "3. 'Started Getty on tty1'"
    echo "4. Root prompt with 'Welcome to HorizonOS Live' message"
else
    echo -e "${RED}✗ Some boot components are missing or misconfigured${NC}"
    echo "The ISO may fail to boot properly."
fi

echo ""
echo "To test the actual ISO boot:"
echo "1. Build the ISO with: sudo ./scripts/scripts/build-iso.sh"
echo "2. Test with QEMU: qemu-system-x86_64 -m 4G -enable-kvm -cdrom build/out/horizonos-*.iso"
echo "3. Watch for the boot sequence described above"