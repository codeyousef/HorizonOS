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
# Clear the original ExecStart
ExecStart=
# Use the correct agetty path for archiso
ExecStart=-/sbin/agetty --autologin root --noclear %I 38400 linux
# CRITICAL: Disable restart to prevent loops with autologin
Restart=no
# Ensure proper startup timing
Type=idle
# Add restart protection in case Restart gets overridden
RestartSec=1
StartLimitInterval=0
EOF
    
    # CRITICAL: Enable getty@tty1.service by creating the proper symlink
    # This is what 'systemctl enable getty@tty1.service' would do
    mkdir -p "$AIROOTFS/etc/systemd/system/getty.target.wants"
    ln -sf /usr/lib/systemd/system/getty@.service "$AIROOTFS/etc/systemd/system/getty.target.wants/getty@tty1.service"
    
    # Ensure getty.target is reached by multi-user.target
    # Some archiso builds need this explicit dependency
    mkdir -p "$AIROOTFS/etc/systemd/system/multi-user.target.wants"
    ln -sf /usr/lib/systemd/system/getty.target "$AIROOTFS/etc/systemd/system/multi-user.target.wants/getty.target"
    
    echo "Standard getty configuration applied"
}

# Export the function
export -f apply_standard_getty_fix