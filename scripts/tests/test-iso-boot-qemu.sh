#!/bin/bash
# Automated QEMU boot test for HorizonOS ISO
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo "=== HorizonOS ISO QEMU Boot Test ==="
echo ""

# Check for ISO file
ISO_FILE=$(ls -1 "$PROJECT_ROOT"/build/out/horizonos-*.iso 2>/dev/null | head -n1)

if [ ! -f "$ISO_FILE" ]; then
    echo -e "${RED}Error: No ISO file found in build/out/${NC}"
    echo "Please build the ISO first with: sudo ./scripts/scripts/build-iso.sh"
    exit 1
fi

echo "Testing ISO: $ISO_FILE"
echo ""

# Check for QEMU
if ! command -v qemu-system-x86_64 &> /dev/null; then
    echo -e "${RED}Error: qemu-system-x86_64 not found${NC}"
    echo "Install with: sudo pacman -S qemu"
    exit 1
fi

# Create a temporary directory for QEMU output
TEMP_DIR="/tmp/horizonos-qemu-test-$$"
mkdir -p "$TEMP_DIR"

# QEMU serial output file
SERIAL_LOG="$TEMP_DIR/serial.log"

# Expected boot markers
declare -a BOOT_MARKERS=(
    "Welcome to Arch Linux!"
    "Reached target Multi-User System"
    "Started Getty on tty1"
    "Welcome to HorizonOS Live"
)

# Error markers that indicate boot failure
declare -a ERROR_MARKERS=(
    "Reached target Graphical Interface"
    "Failed to start"
    "emergency mode"
    "rescue mode"
    "kernel panic"
    "Service .* failed"
    "getty.*tty1.*failed"
    "dependency failed"
)

echo "Starting QEMU with serial console..."
echo "Boot timeout: 60 seconds"
echo ""

# Start QEMU in background with serial console
timeout 60 qemu-system-x86_64 \
    -m 2G \
    -cdrom "$ISO_FILE" \
    -nographic \
    -serial mon:stdio \
    -boot d \
    -display none \
    > "$SERIAL_LOG" 2>&1 &

QEMU_PID=$!

# Monitor boot progress
echo "Monitoring boot progress..."
echo "--------------------------"

BOOT_SUCCESS=true
BOOT_COMPLETE=false
START_TIME=$(date +%s)

# Function to check for markers
check_markers() {
    local log_file=$1
    local marker=$2
    grep -q "$marker" "$log_file" 2>/dev/null
}

# Monitor the boot process
while [ $(($(date +%s) - START_TIME)) -lt 60 ]; do
    # Check for error markers
    for error in "${ERROR_MARKERS[@]}"; do
        if check_markers "$SERIAL_LOG" "$error"; then
            echo -e "${RED}✗ Error detected: $error${NC}"
            BOOT_SUCCESS=false
            break 2
        fi
    done
    
    # Check for success markers
    for i in "${!BOOT_MARKERS[@]}"; do
        marker="${BOOT_MARKERS[$i]}"
        if check_markers "$SERIAL_LOG" "$marker"; then
            echo -e "${GREEN}✓ Found: $marker${NC}"
            unset 'BOOT_MARKERS[$i]'
            
            # If we found the final marker, boot is complete
            if [[ "$marker" == "Welcome to HorizonOS Live" ]]; then
                BOOT_COMPLETE=true
                break 2
            fi
        fi
    done
    
    # Check if QEMU is still running
    if ! kill -0 $QEMU_PID 2>/dev/null; then
        echo -e "${YELLOW}QEMU process ended${NC}"
        break
    fi
    
    sleep 1
done

# Kill QEMU if still running
kill $QEMU_PID 2>/dev/null || true
wait $QEMU_PID 2>/dev/null || true

echo "--------------------------"
echo ""

# Analyze results
echo -e "${YELLOW}Boot Log Analysis:${NC}"
echo ""

# Show relevant boot messages
echo "Key boot messages found:"
grep -E "(Welcome to|Reached target|Started Getty|graphical|failed|Failed)" "$SERIAL_LOG" | tail -20 || echo "No relevant messages found"

echo ""
echo -e "${YELLOW}Test Results:${NC}"
echo ""

# Check specific issues
echo -n "1. Boot target: "
if grep -q "Reached target Graphical Interface" "$SERIAL_LOG"; then
    echo -e "${RED}✗ Reached graphical.target (should be multi-user.target)${NC}"
    BOOT_SUCCESS=false
elif grep -q "Reached target Multi-User System" "$SERIAL_LOG"; then
    echo -e "${GREEN}✓ Reached multi-user.target${NC}"
else
    echo -e "${YELLOW}⚠ Target not clearly identified${NC}"
fi

echo -n "2. Getty service: "
if grep -q "getty.*tty1.*failed" "$SERIAL_LOG"; then
    echo -e "${RED}✗ Getty failed to start${NC}"
    BOOT_SUCCESS=false
elif grep -q "Started Getty on tty1" "$SERIAL_LOG"; then
    echo -e "${GREEN}✓ Getty started successfully${NC}"
else
    echo -e "${YELLOW}⚠ Getty status unclear${NC}"
fi

echo -n "3. Boot completion: "
if [ "$BOOT_COMPLETE" = true ]; then
    echo -e "${GREEN}✓ Boot completed to live environment${NC}"
else
    echo -e "${RED}✗ Boot did not complete${NC}"
    BOOT_SUCCESS=false
fi

# Save the log for debugging
if [ "$BOOT_SUCCESS" = false ]; then
    DEBUG_LOG="$PROJECT_ROOT/horizonos-boot-debug-$(date +%Y%m%d-%H%M%S).log"
    cp "$SERIAL_LOG" "$DEBUG_LOG"
    echo ""
    echo -e "${YELLOW}Debug log saved to: $DEBUG_LOG${NC}"
fi

# Cleanup
rm -rf "$TEMP_DIR"

# Summary
echo ""
echo "================================"
if [ "$BOOT_SUCCESS" = true ] && [ "$BOOT_COMPLETE" = true ]; then
    echo -e "${GREEN}✓ ISO BOOT TEST PASSED${NC}"
    echo ""
    echo "The ISO successfully boots to the live environment without hanging."
    exit 0
else
    echo -e "${RED}✗ ISO BOOT TEST FAILED${NC}"
    echo ""
    echo "The ISO failed to boot properly. Common issues:"
    echo "- Hanging at 'Reached graphical interface' - systemd target issue"
    echo "- Getty restart loop - autologin configuration issue"
    echo "- Missing boot parameters - kernel cmdline issue"
    echo ""
    echo "Check the debug log for details."
    exit 1
fi