#!/bin/bash
# Automated ISO boot testing with QEMU
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Configuration
ISO_PATH="${1:-$PROJECT_ROOT/build/out/horizonos-*.iso}"
QEMU_MEM="2G"
QEMU_CPU="2"
BOOT_TIMEOUT=120
LOG_FILE="/tmp/horizonos-boot-test-$(date +%Y%m%d-%H%M%S).log"
SCREENSHOT_DIR="/tmp/horizonos-boot-screenshots"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo "=== HorizonOS ISO Boot Test ==="
echo "ISO: $ISO_PATH"
echo "Log: $LOG_FILE"
echo ""

# Find ISO
ISO_FILE=$(ls $ISO_PATH 2>/dev/null | head -1)
if [ -z "$ISO_FILE" ] || [ ! -f "$ISO_FILE" ]; then
    echo -e "${RED}Error: ISO file not found at $ISO_PATH${NC}"
    exit 1
fi

echo "Testing ISO: $ISO_FILE"

# Create screenshot directory
mkdir -p "$SCREENSHOT_DIR"

# Start QEMU with serial console for capturing output
echo "Starting QEMU..."
qemu-system-x86_64 \
    -m "$QEMU_MEM" \
    -smp "$QEMU_CPU" \
    -cdrom "$ISO_FILE" \
    -boot d \
    -serial file:"$LOG_FILE" \
    -display none \
    -monitor stdio \
    -device VGA,vgamem_mb=16 \
    -vnc :0 &

QEMU_PID=$!

# Function to capture screenshot
capture_screenshot() {
    local name="$1"
    echo "screendump $SCREENSHOT_DIR/$name.ppm" | nc localhost 1234 2>/dev/null || true
}

# Function to check boot progress
check_boot_progress() {
    local elapsed=0
    local success=false
    
    echo "Monitoring boot progress..."
    
    while [ $elapsed -lt $BOOT_TIMEOUT ]; do
        if [ -f "$LOG_FILE" ]; then
            # Check for successful boot indicators
            if grep -q "Welcome to HorizonOS" "$LOG_FILE" || \
               grep -q "horizonos login:" "$LOG_FILE" || \
               grep -q "root@horizonos" "$LOG_FILE"; then
                echo -e "${GREEN}✓ Boot successful!${NC}"
                success=true
                break
            fi
            
            # Check for getty errors
            if grep -q "getty.*Failed" "$LOG_FILE" || \
               grep -q "start-limit-hit" "$LOG_FILE"; then
                echo -e "${RED}✗ Getty service failed!${NC}"
                grep -A5 -B5 "getty" "$LOG_FILE" | tail -20
                break
            fi
        fi
        
        sleep 5
        elapsed=$((elapsed + 5))
        echo -n "."
    done
    
    echo ""
    
    if [ "$success" = true ]; then
        return 0
    else
        return 1
    fi
}

# Monitor boot
if check_boot_progress; then
    echo -e "${GREEN}ISO boot test PASSED${NC}"
    EXIT_CODE=0
else
    echo -e "${RED}ISO boot test FAILED${NC}"
    echo ""
    echo "Last 50 lines of boot log:"
    tail -50 "$LOG_FILE" 2>/dev/null || echo "No log output captured"
    EXIT_CODE=1
fi

# Kill QEMU
kill $QEMU_PID 2>/dev/null || true

# Summary
echo ""
echo "=== Test Summary ==="
echo "Log file: $LOG_FILE"
echo "Screenshots: $SCREENSHOT_DIR"

exit $EXIT_CODE