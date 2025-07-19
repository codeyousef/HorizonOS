#!/bin/bash
# Simple getty configuration test - matches EndeavourOS/BlendOS approach
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo "=== Simple Getty Configuration Test ==="
echo ""

TEST_DIR="/tmp/horizonos-getty-test-$$"
mkdir -p "$TEST_DIR/airootfs"

# Apply the standard getty fix
source "$PROJECT_ROOT/scripts/scripts/boot-fixes/getty-archiso-standard.sh"
apply_standard_getty_fix "$TEST_DIR/airootfs"

echo -e "${YELLOW}Checking getty configuration:${NC}"

# Check autologin configuration
AUTOLOGIN="$TEST_DIR/airootfs/etc/systemd/system/getty@tty1.service.d/autologin.conf"

echo -n "1. Autologin configuration exists: "
if [ -f "$AUTOLOGIN" ]; then
    echo -e "${GREEN}✓ Yes${NC}"
else
    echo -e "${RED}✗ No${NC}"
    exit 1
fi

echo -n "2. Using correct agetty path: "
if grep -q "/sbin/agetty" "$AUTOLOGIN"; then
    echo -e "${GREEN}✓ /sbin/agetty (correct)${NC}"
else
    echo -e "${RED}✗ Wrong path${NC}"
    exit 1
fi

echo -n "3. Autologin configured for root: "
if grep -q -- "--autologin root" "$AUTOLOGIN"; then
    echo -e "${GREEN}✓ Yes${NC}"
else
    echo -e "${RED}✗ No${NC}"
    exit 1
fi

echo -n "4. Clear ExecStart first: "
if grep -q "^ExecStart=$" "$AUTOLOGIN"; then
    echo -e "${GREEN}✓ Yes${NC}"
else
    echo -e "${RED}✗ No${NC}"
    exit 1
fi

echo -e "\n${YELLOW}Configuration content:${NC}"
cat "$AUTOLOGIN"

# Cleanup
rm -rf "$TEST_DIR"

echo -e "\n${GREEN}=== Test Passed ===${NC}"
echo "Getty configuration matches standard archiso approach used by:"
echo "- EndeavourOS"
echo "- BlendOS"  
echo "- Official Arch Linux ISO"
echo ""
echo "This simple configuration should boot without issues."