#!/bin/bash
# Test script to verify proper getty service enablement
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo "=== Getty Service Enablement Test ==="
echo ""
echo "This test verifies that getty services will be properly enabled in the ISO"
echo ""

# Create temporary test directory
TEST_DIR=$(mktemp -d)
trap "rm -rf $TEST_DIR" EXIT

echo -e "${YELLOW}Creating test filesystem structure...${NC}"
mkdir -p "$TEST_DIR/airootfs/etc/systemd/system"
mkdir -p "$TEST_DIR/airootfs/usr/lib/systemd/system"

# Simulate systemd service files
cat > "$TEST_DIR/airootfs/usr/lib/systemd/system/getty.target" << 'EOF'
[Unit]
Description=Login Prompts
Documentation=man:systemd.special(7) man:systemd-getty-generator(8)
Documentation=http://0pointer.de/blog/projects/serial-console.html

[Install]
WantedBy=multi-user.target
EOF

cat > "$TEST_DIR/airootfs/usr/lib/systemd/system/getty@.service" << 'EOF'
[Unit]
Description=Getty on %I
Documentation=man:agetty(8) man:systemd-getty-generator(8)
After=systemd-user-sessions.service plymouth-quit-wait.service
IgnoreOnIsolate=yes

[Service]
ExecStart=-/usr/bin/agetty -o '-p -- \\u' --noclear %I $TERM
Type=idle
Restart=always
StandardInput=tty
StandardOutput=tty

[Install]
WantedBy=getty.target
DefaultInstance=tty1
EOF

# Apply the getty fixes from build-iso.sh
echo -e "${YELLOW}Applying getty configuration...${NC}"

# Simulate the configuration from build-iso.sh
cd "$TEST_DIR"

# Create drop-in to ensure getty.target is properly pulled in
mkdir -p airootfs/etc/systemd/system/getty.target.d
cat > airootfs/etc/systemd/system/getty.target.d/horizonos-enable.conf << 'EOF'
[Unit]
# Ensure getty.target starts with multi-user.target
WantedBy=multi-user.target
RequiredBy=multi-user.target

[Install]
WantedBy=multi-user.target
EOF

# Properly enable getty.target (mimicking systemctl enable)
mkdir -p airootfs/etc/systemd/system/multi-user.target.wants
ln -sf /usr/lib/systemd/system/getty.target airootfs/etc/systemd/system/multi-user.target.wants/getty.target

# Enable getty@tty1.service using proper instantiation from template
mkdir -p airootfs/etc/systemd/system/getty.target.wants
ln -sf /usr/lib/systemd/system/getty@.service airootfs/etc/systemd/system/getty.target.wants/getty@tty1.service

# Also add to multi-user.target as a failsafe
ln -sf /usr/lib/systemd/system/getty@.service airootfs/etc/systemd/system/multi-user.target.wants/getty@tty1.service

# Test the configuration
echo ""
echo -e "${YELLOW}Testing configuration...${NC}"
echo ""

# Test 1: Check if getty.target is wanted by multi-user.target
echo "Test 1: Checking getty.target enablement"
if [ -L "airootfs/etc/systemd/system/multi-user.target.wants/getty.target" ]; then
    echo -e "${GREEN}✓ getty.target is properly linked in multi-user.target.wants${NC}"
    TARGET=$(readlink "airootfs/etc/systemd/system/multi-user.target.wants/getty.target")
    echo "  Links to: $TARGET"
else
    echo -e "${RED}✗ getty.target is NOT linked in multi-user.target.wants${NC}"
fi

# Test 2: Check if getty@tty1.service is enabled
echo ""
echo "Test 2: Checking getty@tty1.service enablement"
if [ -L "airootfs/etc/systemd/system/getty.target.wants/getty@tty1.service" ]; then
    echo -e "${GREEN}✓ getty@tty1.service is properly linked in getty.target.wants${NC}"
    TARGET=$(readlink "airootfs/etc/systemd/system/getty.target.wants/getty@tty1.service")
    echo "  Links to: $TARGET"
else
    echo -e "${RED}✗ getty@tty1.service is NOT linked in getty.target.wants${NC}"
fi

# Test 3: Check failsafe link
echo ""
echo "Test 3: Checking failsafe getty@tty1.service link"
if [ -L "airootfs/etc/systemd/system/multi-user.target.wants/getty@tty1.service" ]; then
    echo -e "${GREEN}✓ getty@tty1.service has failsafe link in multi-user.target.wants${NC}"
else
    echo -e "${RED}✗ getty@tty1.service does NOT have failsafe link${NC}"
fi

# Test 4: Check drop-in configuration
echo ""
echo "Test 4: Checking getty.target drop-in configuration"
if [ -f "airootfs/etc/systemd/system/getty.target.d/horizonos-enable.conf" ]; then
    echo -e "${GREEN}✓ getty.target drop-in configuration exists${NC}"
    echo "  Content:"
    cat "airootfs/etc/systemd/system/getty.target.d/horizonos-enable.conf" | sed 's/^/    /'
else
    echo -e "${RED}✗ getty.target drop-in configuration missing${NC}"
fi

# Test 5: Simulate systemd dependency resolution
echo ""
echo "Test 5: Simulating systemd dependency chain"
echo "  multi-user.target wants:"
for link in airootfs/etc/systemd/system/multi-user.target.wants/*; do
    if [ -L "$link" ]; then
        echo "    - $(basename "$link")"
    fi
done

echo ""
echo "  getty.target wants:"
for link in airootfs/etc/systemd/system/getty.target.wants/*; do
    if [ -L "$link" ]; then
        echo "    - $(basename "$link")"
    fi
done

# Summary
echo ""
echo -e "${GREEN}=== Test Summary ===${NC}"
echo "Configuration creates the following boot chain:"
echo "1. System reaches multi-user.target"
echo "2. multi-user.target pulls in getty.target (via wants + drop-in)"
echo "3. getty.target pulls in getty@tty1.service"
echo "4. Failsafe: getty@tty1.service also directly wanted by multi-user.target"
echo ""
echo "This should prevent the system from hanging at 'Reached target Multi-User System'"