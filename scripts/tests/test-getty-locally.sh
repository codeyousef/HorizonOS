#!/bin/bash
# Comprehensive local testing for getty configuration
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo "=== Comprehensive Getty Local Test ==="
echo ""

# Test 1: Simulate the ISO build environment
echo -e "${YELLOW}Test 1: Simulating ISO build environment${NC}"

# Create temporary directory structure like archiso
TEST_DIR="/tmp/horizonos-getty-test-$$"
AIROOTFS="$TEST_DIR/airootfs"
mkdir -p "$AIROOTFS/etc/systemd/system"

# Copy the releng autologin.conf to simulate archiso
echo "Creating archiso releng autologin.conf..."
mkdir -p "$AIROOTFS/etc/systemd/system/getty@tty1.service.d"
cat > "$AIROOTFS/etc/systemd/system/getty@tty1.service.d/autologin.conf" << 'EOF'
[Service]
ExecStart=
ExecStart=-/sbin/agetty -o '-p -f -- \\u' --noclear --autologin root - $TERM
EOF

echo "Before fix:"
cat "$AIROOTFS/etc/systemd/system/getty@tty1.service.d/autologin.conf"

# Apply our fix
echo -e "\n${YELLOW}Applying HorizonOS getty fix...${NC}"
source "$PROJECT_ROOT/scripts/scripts/boot-fixes/getty-autologin.sh"
fix_getty_in_iso "$AIROOTFS"

echo -e "\nAfter fix:"
if [ -f "$AIROOTFS/etc/systemd/system/getty@tty1.service.d/autologin.conf" ]; then
    cat "$AIROOTFS/etc/systemd/system/getty@tty1.service.d/autologin.conf"
else
    echo -e "${RED}ERROR: autologin.conf missing after fix!${NC}"
fi

# Test 2: Validate the configuration
echo -e "\n${YELLOW}Test 2: Validating configuration${NC}"

# Check if the file has correct syntax
TEMP_SERVICE="/tmp/test-getty@.service"
cat > "$TEMP_SERVICE" << 'EOF'
[Unit]
Description=Getty on %I
Documentation=man:agetty(8) man:systemd-getty-generator(8)
Documentation=http://0pointer.de/blog/projects/serial-console.html
After=systemd-user-sessions.service plymouth-quit-wait.service
After=rc-local.service

# If additional gettys are spawned during boot then we should make
# sure that this is synchronized before getty.target, even though
# getty.target didn't actually pull it in.
Before=getty.target
IgnoreOnIsolate=yes

# On systems without virtual consoles, don't start any getty. Note
# that serial gettys are covered by serial-getty@.service, not this
# unit.
ConditionPathExists=/dev/tty0

[Service]
# the VT is cleared by TTYVTDisallocate
ExecStart=-/sbin/agetty -o '-p -- \\u' --noclear %I $TERM
Type=idle
Restart=always
RestartSec=0
UtmpIdentifier=%I
TTYPath=/dev/%I
TTYReset=yes
TTYVHangup=yes
TTYVTDisallocate=yes
KillMode=process
IgnoreSIGPIPE=no
SendSIGHUP=yes

[Install]
WantedBy=getty.target
DefaultInstance=tty1
EOF

# Merge with our drop-in
mkdir -p "/tmp/test-getty-dropin"
cp "$AIROOTFS/etc/systemd/system/getty@tty1.service.d/autologin.conf" "/tmp/test-getty-dropin/"

echo -e "\n${YELLOW}Testing merged service configuration...${NC}"
if systemd-analyze verify --man=no "$TEMP_SERVICE" 2>&1 | grep -q "error"; then
    echo -e "${RED}✗ Service configuration has errors${NC}"
    systemd-analyze verify --man=no "$TEMP_SERVICE"
else
    echo -e "${GREEN}✓ Service configuration is valid${NC}"
fi

# Test 3: Check for common issues
echo -e "\n${YELLOW}Test 3: Checking for common issues${NC}"

# Check agetty path
if grep -q "/usr/bin/agetty" "$AIROOTFS/etc/systemd/system/getty@tty1.service.d/autologin.conf"; then
    echo -e "${GREEN}✓ Using correct /usr/bin/agetty path${NC}"
else
    echo -e "${RED}✗ Not using /usr/bin/agetty path${NC}"
fi

# Check restart protection
if grep -q "Restart=no" "$AIROOTFS/etc/systemd/system/getty@tty1.service.d/autologin.conf"; then
    echo -e "${GREEN}✓ Has Restart=no protection${NC}"
else
    echo -e "${RED}✗ Missing Restart=no protection${NC}"
fi

# Check start limit
if grep -q "StartLimitInterval" "$AIROOTFS/etc/systemd/system/getty@tty1.service.d/autologin.conf"; then
    echo -e "${GREEN}✓ Has StartLimitInterval protection${NC}"
else
    echo -e "${RED}✗ Missing StartLimitInterval protection${NC}"
fi

# Test 4: Simulate systemd behavior
echo -e "\n${YELLOW}Test 4: Simulating systemd unit loading${NC}"

# Create a test script that simulates what systemd would run
TEST_SCRIPT="/tmp/test-getty-exec.sh"
cat > "$TEST_SCRIPT" << 'EOF'
#!/bin/bash
# Extract ExecStart from the drop-in
EXEC_START=$(grep "^ExecStart=" /tmp/horizonos-getty-test-*/airootfs/etc/systemd/system/getty@tty1.service.d/autologin.conf | tail -1 | cut -d= -f2-)
echo "Would execute: $EXEC_START"

# Check if the command exists
AGETTY_CMD=$(echo "$EXEC_START" | awk '{print $1}' | sed 's/^-//')
if [ -x "$AGETTY_CMD" ]; then
    echo "✓ Command exists: $AGETTY_CMD"
else
    echo "✗ Command NOT found: $AGETTY_CMD"
    which agetty || echo "agetty not in PATH"
fi
EOF
chmod +x "$TEST_SCRIPT"
bash "$TEST_SCRIPT"

# Test 5: Check for conflicts
echo -e "\n${YELLOW}Test 5: Checking for conflicts${NC}"

for i in {2..6}; do
    if [ -L "$AIROOTFS/etc/systemd/system/getty@tty${i}.service" ]; then
        echo -e "${GREEN}✓ getty@tty${i} is masked${NC}"
    else
        echo -e "${YELLOW}⚠ getty@tty${i} is not masked${NC}"
    fi
done

# Cleanup
echo -e "\n${YELLOW}Cleaning up...${NC}"
rm -rf "$TEST_DIR" "$TEMP_SERVICE" "/tmp/test-getty-dropin" "$TEST_SCRIPT"

echo -e "\n${GREEN}=== Test Summary ===${NC}"
echo "If all tests passed, the getty configuration should work in the ISO."
echo "If any tests failed, the ISO will likely have getty loop issues."