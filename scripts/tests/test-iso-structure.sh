#!/bin/bash
# Test ISO structure and getty configuration
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo "=== ISO Structure Test ==="
echo ""

# Find the latest ISO
ISO_FILE=$(ls -t "$PROJECT_ROOT/build/out"/horizonos-*.iso 2>/dev/null | head -1)
if [ -z "$ISO_FILE" ] || [ ! -f "$ISO_FILE" ]; then
    echo -e "${RED}No ISO found to test${NC}"
    exit 1
fi

echo "Testing ISO: $ISO_FILE"

# Mount the ISO and check its contents
MOUNT_DIR="/tmp/horizonos-iso-test-$$"
EXTRACT_DIR="/tmp/horizonos-extract-$$"
mkdir -p "$MOUNT_DIR" "$EXTRACT_DIR"

# Mount ISO
echo "Mounting ISO..."
sudo mount -o loop,ro "$ISO_FILE" "$MOUNT_DIR"

# Extract the squashfs filesystem
echo "Extracting squashfs..."
SQUASHFS=$(find "$MOUNT_DIR" -name "*.sfs" -o -name "*.squashfs" | head -1)
if [ -z "$SQUASHFS" ]; then
    echo -e "${RED}No squashfs found in ISO${NC}"
    sudo umount "$MOUNT_DIR"
    exit 1
fi

sudo unsquashfs -d "$EXTRACT_DIR" "$SQUASHFS" >/dev/null 2>&1

# Test 1: Check getty configuration
echo -e "\n${YELLOW}Test 1: Getty Configuration${NC}"

AUTOLOGIN="$EXTRACT_DIR/etc/systemd/system/getty@tty1.service.d/autologin.conf"
if [ -f "$AUTOLOGIN" ]; then
    echo "Found autologin.conf:"
    cat "$AUTOLOGIN"
    
    # Check for correct agetty path
    if grep -q "/usr/bin/agetty" "$AUTOLOGIN"; then
        echo -e "${GREEN}✓ Using correct /usr/bin/agetty path${NC}"
    elif grep -q "/sbin/agetty" "$AUTOLOGIN"; then
        echo -e "${RED}✗ Still using wrong /sbin/agetty path!${NC}"
    else
        echo -e "${YELLOW}⚠ Unknown agetty path${NC}"
    fi
    
    # Check for restart protection
    if grep -q "Restart=no" "$AUTOLOGIN"; then
        echo -e "${GREEN}✓ Has restart protection${NC}"
    else
        echo -e "${YELLOW}⚠ No restart protection found${NC}"
    fi
else
    echo -e "${RED}✗ No autologin.conf found!${NC}"
fi

# Test 2: Check for conflicting services
echo -e "\n${YELLOW}Test 2: Conflicting Services${NC}"

for i in {2..6}; do
    if [ -e "$EXTRACT_DIR/etc/systemd/system/getty@tty$i.service" ]; then
        if [ -L "$EXTRACT_DIR/etc/systemd/system/getty@tty$i.service" ]; then
            echo -e "${GREEN}✓ getty@tty$i is masked (symlink)${NC}"
        else
            echo -e "${YELLOW}⚠ getty@tty$i exists but not masked${NC}"
        fi
    fi
done

# Test 3: Check agetty binary
echo -e "\n${YELLOW}Test 3: Agetty Binary${NC}"

if [ -f "$EXTRACT_DIR/usr/bin/agetty" ]; then
    echo -e "${GREEN}✓ agetty found at /usr/bin/agetty${NC}"
elif [ -f "$EXTRACT_DIR/sbin/agetty" ]; then
    echo -e "${YELLOW}⚠ agetty found at /sbin/agetty (legacy)${NC}"
else
    echo -e "${RED}✗ agetty not found!${NC}"
fi

# Test 4: Check debug tools
echo -e "\n${YELLOW}Test 4: Debug Tools${NC}"

if [ -f "$EXTRACT_DIR/usr/local/bin/debug-getty" ]; then
    echo -e "${GREEN}✓ debug-getty tool installed${NC}"
else
    echo -e "${YELLOW}⚠ debug-getty tool not found${NC}"
fi

# Test 5: Check branding
echo -e "\n${YELLOW}Test 5: Branding${NC}"

if [ -f "$EXTRACT_DIR/etc/hostname" ]; then
    HOSTNAME=$(cat "$EXTRACT_DIR/etc/hostname")
    if [ "$HOSTNAME" = "horizonos" ]; then
        echo -e "${GREEN}✓ Hostname set to horizonos${NC}"
    else
        echo -e "${RED}✗ Wrong hostname: $HOSTNAME${NC}"
    fi
fi

# Cleanup
echo -e "\n${YELLOW}Cleaning up...${NC}"
sudo umount "$MOUNT_DIR"
sudo rm -rf "$MOUNT_DIR" "$EXTRACT_DIR"

echo -e "\n${GREEN}ISO structure test complete${NC}"