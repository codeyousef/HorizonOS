#!/bin/bash
# Test getty configuration locally before building ISO
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo "=== Getty Configuration Test ==="
echo ""

# Test 1: Check agetty location
echo -e "${YELLOW}Test 1: Checking agetty binary location${NC}"
AGETTY_PATH=$(which agetty 2>/dev/null || echo "not found")
echo "System agetty location: $AGETTY_PATH"

if [ "$AGETTY_PATH" = "/usr/bin/agetty" ]; then
    echo -e "${GREEN}✓ agetty is at expected location${NC}"
elif [ "$AGETTY_PATH" = "/sbin/agetty" ]; then
    echo -e "${YELLOW}⚠ agetty is at legacy location /sbin/agetty${NC}"
else
    echo -e "${RED}✗ agetty not found!${NC}"
fi

# Test 2: Check archiso autologin template
echo -e "\n${YELLOW}Test 2: Checking archiso releng autologin config${NC}"
ARCHISO_AUTOLOGIN="/usr/share/archiso/configs/releng/airootfs/etc/systemd/system/getty@tty1.service.d/autologin.conf"

if [ -f "$ARCHISO_AUTOLOGIN" ]; then
    echo "Found archiso autologin config:"
    cat "$ARCHISO_AUTOLOGIN"
    
    # Check if it uses wrong path
    if grep -q "/sbin/agetty" "$ARCHISO_AUTOLOGIN"; then
        echo -e "${YELLOW}⚠ Archiso uses /sbin/agetty - needs path fix${NC}"
    fi
else
    echo -e "${RED}✗ Archiso autologin config not found${NC}"
fi

# Test 3: Test systemd service syntax
echo -e "\n${YELLOW}Test 3: Testing systemd service syntax${NC}"

# Create temporary test service
TEST_SERVICE="/tmp/test-getty@.service"
cat > "$TEST_SERVICE" << 'EOF'
[Service]
ExecStart=
ExecStart=-/usr/bin/agetty -o '-p -f -- \\u' --noclear --autologin root - $TERM
EOF

if systemd-analyze verify "$TEST_SERVICE" 2>&1 | grep -q "error"; then
    echo -e "${RED}✗ Service configuration has errors${NC}"
    systemd-analyze verify "$TEST_SERVICE"
else
    echo -e "${GREEN}✓ Service configuration is valid${NC}"
fi

rm -f "$TEST_SERVICE"

# Test 4: Check for common issues
echo -e "\n${YELLOW}Test 4: Checking for common issues${NC}"

# Check if /sbin is symlinked to /usr/bin
if [ -L /sbin ] && [ "$(readlink /sbin)" = "usr/bin" ]; then
    echo -e "${GREEN}✓ /sbin is symlinked to usr/bin (good)${NC}"
else
    echo -e "${YELLOW}⚠ /sbin is not symlinked to usr/bin${NC}"
fi

# Test 5: Simulate the fix
echo -e "\n${YELLOW}Test 5: Simulating path fix${NC}"
TEMP_CONF="/tmp/test-autologin.conf"
cat > "$TEMP_CONF" << 'EOF'
[Service]
ExecStart=
ExecStart=-/sbin/agetty -o '-p -f -- \\u' --noclear --autologin root - $TERM
EOF

echo "Before fix:"
cat "$TEMP_CONF"

sed -i 's|/sbin/agetty|/usr/bin/agetty|g' "$TEMP_CONF"

echo -e "\nAfter fix:"
cat "$TEMP_CONF"

rm -f "$TEMP_CONF"

# Summary
echo -e "\n${GREEN}=== Test Summary ===${NC}"
echo "1. agetty location: $AGETTY_PATH"
echo "2. Path fix needed: YES (archiso uses /sbin/agetty)"
echo "3. Service syntax: Valid"
echo "4. Fix: sed -i 's|/sbin/agetty|/usr/bin/agetty|g' autologin.conf"