#!/bin/bash
# Standard archiso getty configuration - EXACT copy from working distros
# Based on EndeavourOS, BlendOS, and standard archiso implementations

echo "=== Applying Standard Archiso Getty Configuration ==="

apply_standard_getty_fix() {
    local AIROOTFS="$1"
    
    echo "Applying standard getty configuration..."
    
    # Create autologin configuration EXACTLY as used by working archiso distros
    mkdir -p "$AIROOTFS/etc/systemd/system/getty@tty1.service.d"
    
    cat > "$AIROOTFS/etc/systemd/system/getty@tty1.service.d/autologin.conf" << 'EOF'
[Service]
ExecStart=
ExecStart=-/sbin/agetty --autologin root --noclear %I 38400 linux
EOF
    
    # That's it! Nothing else needed - standard archiso systemd handles the rest
    
    echo "Standard getty configuration applied"
}

# Export the function
export -f apply_standard_getty_fix