#!/bin/bash
# Standard archiso getty configuration - based on EndeavourOS, Manjaro, etc.
# This is the PROVEN approach used by all major Arch-based live ISOs

echo "=== Standard Archiso Getty Configuration ==="

apply_archiso_getty_fix() {
    local AIROOTFS="$1"
    
    echo "Applying standard archiso getty configuration..."
    
    # 1. Create the autologin configuration exactly as other Arch distros do
    mkdir -p "$AIROOTFS/etc/systemd/system/getty@tty1.service.d"
    cat > "$AIROOTFS/etc/systemd/system/getty@tty1.service.d/autologin.conf" << 'EOF'
[Service]
ExecStart=
ExecStart=-/sbin/agetty --autologin root --noclear %I 38400 linux
Type=idle
Restart=no
StandardInput=tty
StandardOutput=journal+console
StandardError=journal
EOF
    
    # 2. Enable getty@tty1.service by creating the proper symlink
    # This is exactly what systemctl enable would do
    mkdir -p "$AIROOTFS/etc/systemd/system/getty.target.wants"
    ln -sf /usr/lib/systemd/system/getty@.service "$AIROOTFS/etc/systemd/system/getty.target.wants/getty@tty1.service"
    
    # That's it! This is all that's needed based on other Arch distros
    # No need to modify getty.target or multi-user.target dependencies
    # The standard systemd dependency chain handles everything
    
    echo "Standard archiso getty configuration applied"
}

# Export the function
export -f apply_archiso_getty_fix