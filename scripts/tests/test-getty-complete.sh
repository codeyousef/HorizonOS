#!/bin/bash
# Complete getty configuration test
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo "=== Complete Getty Configuration Test ==="
echo ""

TEST_DIR="/tmp/horizonos-getty-test-$$"
mkdir -p "$TEST_DIR/airootfs"

# Apply the complete getty fix
source "$PROJECT_ROOT/scripts/scripts/boot-fixes/getty-complete-fix.sh"
apply_complete_getty_fix "$TEST_DIR/airootfs"

echo -e "\n${YELLOW}Checking all getty configurations:${NC}"

# Check autologin configuration
echo -n "1. Autologin configuration: "
if [ -f "$TEST_DIR/airootfs/etc/systemd/system/getty@tty1.service.d/autologin.conf" ]; then
    if grep -q "/sbin/agetty" "$TEST_DIR/airootfs/etc/systemd/system/getty@tty1.service.d/autologin.conf"; then
        echo -e "${GREEN}✓ Present with correct path${NC}"
    else
        echo -e "${RED}✗ Wrong agetty path${NC}"
    fi
else
    echo -e "${RED}✗ Missing${NC}"
fi

# Check getty@tty1 enablement
echo -n "2. getty@tty1.service enabled: "
if [ -L "$TEST_DIR/airootfs/etc/systemd/system/getty.target.wants/getty@tty1.service" ]; then
    echo -e "${GREEN}✓ Yes${NC}"
else
    echo -e "${RED}✗ No${NC}"
fi

# Check force-getty service
echo -n "3. Force getty service: "
if [ -L "$TEST_DIR/airootfs/etc/systemd/system/multi-user.target.wants/force-getty.service" ]; then
    echo -e "${GREEN}✓ Enabled${NC}"
else
    echo -e "${RED}✗ Not enabled${NC}"
fi

# Check direct getty service
echo -n "4. Direct getty failsafe: "
if [ -L "$TEST_DIR/airootfs/etc/systemd/system/multi-user.target.wants/direct-getty@tty1.service" ]; then
    echo -e "${GREEN}✓ Enabled${NC}"
else
    echo -e "${RED}✗ Not enabled${NC}"
fi

# Check debug service
echo -n "5. Boot debug service: "
if [ -L "$TEST_DIR/airootfs/etc/systemd/system/multi-user.target.wants/boot-getty-debug.service" ]; then
    echo -e "${GREEN}✓ Enabled${NC}"
else
    echo -e "${RED}✗ Not enabled${NC}"
fi

# Check getty-force target
echo -n "6. Getty force target: "
if [ -L "$TEST_DIR/airootfs/etc/systemd/system/multi-user.target.wants/getty-force.target" ]; then
    echo -e "${GREEN}✓ Enabled${NC}"
else
    echo -e "${RED}✗ Not enabled${NC}"
fi

# List all services that will start with multi-user.target
echo -e "\n${YELLOW}Services enabled for multi-user.target:${NC}"
ls -la "$TEST_DIR/airootfs/etc/systemd/system/multi-user.target.wants/" 2>/dev/null | grep -v "^total\|^d" || echo "None found"

# List all services that will start with getty.target
echo -e "\n${YELLOW}Services enabled for getty.target:${NC}"
ls -la "$TEST_DIR/airootfs/etc/systemd/system/getty.target.wants/" 2>/dev/null | grep -v "^total\|^d" || echo "None found"

# Cleanup
rm -rf "$TEST_DIR"

echo -e "\n${GREEN}=== Test Summary ===${NC}"
echo ""
echo "With this configuration:"
echo "1. getty@tty1 is enabled via getty.target"
echo "2. force-getty.service ensures getty.target starts"
echo "3. direct-getty@tty1.service provides a failsafe"
echo "4. boot-getty-debug.service provides diagnostics"
echo "5. getty-force.target provides additional enforcement"
echo ""
echo "At least ONE of these should ensure a getty starts!"