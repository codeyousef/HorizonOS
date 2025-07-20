#!/bin/bash
# test-iso-vm.sh - Automated VM testing for HorizonOS ISO
# Based on Boot Process and Troubleshooting guide

ISO_FILE="${1:-./build/out/horizonos-*.iso}"
ISO_FILE=$(ls $ISO_FILE 2>/dev/null | head -1)

if [[ ! -f "$ISO_FILE" ]]; then
    echo "ISO file not found: $ISO_FILE"
    echo "Usage: $0 [path-to-iso]"
    exit 1
fi

echo "=== Testing HorizonOS ISO in QEMU ==="
echo "ISO: $ISO_FILE"

# Test function
test_vm() {
    local NAME=$1
    local EXTRA_ARGS=$2
    local TIMEOUT=${3:-120}
    
    echo -e "\nTest: $NAME"
    echo "Starting VM with: $EXTRA_ARGS"
    
    # Create log file
    LOG_FILE="/tmp/vm_output_$NAME.log"
    
    # Start QEMU in background
    qemu-system-x86_64 \
        -m 2G \
        -cdrom "$ISO_FILE" \
        -boot d \
        -vga virtio \
        -display none \
        -serial mon:stdio \
        -monitor none \
        $EXTRA_ARGS \
        > "$LOG_FILE" 2>&1 &
    
    QEMU_PID=$!
    
    # Wait for boot and check for common issues
    SECONDS=0
    while [[ $SECONDS -lt $TIMEOUT ]]; do
        if grep -q "Started Getty on tty1" "$LOG_FILE"; then
            echo "✓ System reached getty"
            
            # Check if it's stuck there (for text-only systems, this is success)
            sleep 5
            if tail -5 "$LOG_FILE" | grep -q "Started Getty on tty1"; then
                echo "✓ Getty started successfully (text-only mode)"
                
                # Check for the flashing issue
                if grep -c "Started Getty on tty1" "$LOG_FILE" > 3; then
                    echo "✗ Getty restarting loop detected!"
                    kill $QEMU_PID 2>/dev/null
                    return 1
                fi
                
                kill $QEMU_PID 2>/dev/null
                return 0
            fi
        fi
        
        if grep -q "HorizonOS Live" "$LOG_FILE"; then
            echo "✓ HorizonOS branding detected"
        fi
        
        if grep -q "Failed to start" "$LOG_FILE"; then
            echo "✗ Service failures detected:"
            grep "Failed to start" "$LOG_FILE" | tail -5
            kill $QEMU_PID 2>/dev/null
            return 1
        fi
        
        if grep -q "emergency mode" "$LOG_FILE"; then
            echo "✗ System entered emergency mode"
            kill $QEMU_PID 2>/dev/null
            return 1
        fi
        
        sleep 2
    done
    
    echo "✗ Timeout waiting for boot completion"
    kill $QEMU_PID 2>/dev/null
    return 1
}

# Run tests
FAILED=0

# Test 1: Basic boot
if ! test_vm "basic" ""; then
    ((FAILED++))
fi

# Test 2: UEFI boot (if OVMF available)
if [ -f /usr/share/ovmf/OVMF.fd ]; then
    if ! test_vm "uefi" "-bios /usr/share/ovmf/OVMF.fd"; then
        ((FAILED++))
    fi
else
    echo -e "\nSkipping UEFI test (OVMF not found)"
fi

# Test 3: Low memory
if ! test_vm "lowmem" "-m 1G" 180; then
    ((FAILED++))
fi

# Test 4: No KMS
if ! test_vm "nokms" "-append nomodeset"; then
    ((FAILED++))
fi

echo -e "\n=== Test Summary ==="
echo "Failed tests: $FAILED"

# Check for getty loop specifically
echo -e "\n=== Getty Loop Analysis ==="
for log in /tmp/vm_output_*.log; do
    if [ -f "$log" ]; then
        COUNT=$(grep -c "Started Getty on tty1" "$log" || true)
        NAME=$(basename "$log" .log | sed 's/vm_output_//')
        if [ $COUNT -gt 3 ]; then
            echo "✗ $NAME: Getty restarted $COUNT times (LOOP DETECTED)"
        else
            echo "✓ $NAME: Getty started normally ($COUNT times)"
        fi
    fi
done

if [[ $FAILED -gt 0 ]]; then
    echo -e "\nSome tests failed. Check /tmp/vm_output_*.log files for details."
    echo "To debug getty issues, look for repeated 'Started Getty on tty1' messages."
    exit 1
else
    echo -e "\nAll tests passed!"
    echo "The getty configuration appears to be working correctly."
fi