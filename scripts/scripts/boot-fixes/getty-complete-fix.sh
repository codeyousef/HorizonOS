#!/bin/bash
# Complete getty fix for archiso based on deep research
# This ensures getty services actually start after multi-user.target

echo "=== Complete Getty Fix for Archiso ==="

apply_complete_getty_fix() {
    local AIROOTFS="$1"
    
    echo "Applying complete getty fix..."
    
    # 1. Create autologin configuration
    echo "Creating autologin configuration..."
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
    
    # 2. CRITICAL: Enable getty@tty1.service
    echo "Enabling getty@tty1.service..."
    mkdir -p "$AIROOTFS/etc/systemd/system/getty.target.wants"
    ln -sf /usr/lib/systemd/system/getty@.service "$AIROOTFS/etc/systemd/system/getty.target.wants/getty@tty1.service"
    
    # 3. CRITICAL: Ensure getty.target is pulled in by multi-user.target
    # This is often missing in custom archiso builds
    echo "Ensuring getty.target is enabled..."
    mkdir -p "$AIROOTFS/etc/systemd/system/multi-user.target.wants"
    # Create a service that forces getty.target to start
    cat > "$AIROOTFS/etc/systemd/system/force-getty.service" << 'EOF'
[Unit]
Description=Force Getty Target
After=multi-user.target
Wants=getty.target
Requires=getty.target

[Service]
Type=oneshot
ExecStart=/bin/true
RemainAfterExit=yes

[Install]
WantedBy=multi-user.target
EOF
    ln -sf /etc/systemd/system/force-getty.service "$AIROOTFS/etc/systemd/system/multi-user.target.wants/force-getty.service"
    
    # 4. Add a direct getty service as absolute failsafe
    echo "Creating direct getty service..."
    cat > "$AIROOTFS/etc/systemd/system/direct-getty@.service" << 'EOF'
[Unit]
Description=Direct Getty on %I
After=multi-user.target
ConditionPathExists=/dev/%I

[Service]
Type=idle
ExecStart=-/sbin/agetty --autologin root --noclear %I 38400 linux
Restart=always
RestartSec=1
StandardInput=tty
StandardOutput=tty
StandardError=journal
TTYPath=/dev/%I
TTYReset=yes
TTYVHangup=yes

[Install]
WantedBy=multi-user.target
EOF
    ln -sf /etc/systemd/system/direct-getty@.service "$AIROOTFS/etc/systemd/system/multi-user.target.wants/direct-getty@tty1.service"
    
    # 5. Create debug service to log what's happening
    echo "Creating boot debug service..."
    cat > "$AIROOTFS/etc/systemd/system/boot-getty-debug.service" << 'EOF'
[Unit]
Description=Boot Getty Debug
After=multi-user.target

[Service]
Type=oneshot
ExecStart=/bin/bash -c 'echo "=== Getty Debug ===" > /dev/console; systemctl status getty.target > /dev/console 2>&1; systemctl status getty@tty1.service > /dev/console 2>&1; systemctl list-dependencies multi-user.target > /dev/console 2>&1'
RemainAfterExit=yes
StandardOutput=journal+console
StandardError=journal+console

[Install]
WantedBy=multi-user.target
EOF
    ln -sf /etc/systemd/system/boot-getty-debug.service "$AIROOTFS/etc/systemd/system/multi-user.target.wants/boot-getty-debug.service"
    
    # 6. Create a custom target that ensures getty starts
    echo "Creating custom getty enforcement..."
    cat > "$AIROOTFS/etc/systemd/system/getty-force.target" << 'EOF'
[Unit]
Description=Force Getty Services
Requires=getty.target
After=getty.target
Wants=getty@tty1.service

[Install]
WantedBy=multi-user.target
EOF
    ln -sf /etc/systemd/system/getty-force.target "$AIROOTFS/etc/systemd/system/multi-user.target.wants/getty-force.target"
    
    # 7. Add kernel command line debugging hint
    echo "Adding debug hints..."
    cat > "$AIROOTFS/etc/systemd/system/getty-hint.service" << 'EOF'
[Unit]
Description=Getty Debug Hint
After=multi-user.target
ConditionKernelCommandLine=systemd.debug

[Service]
Type=oneshot
ExecStart=/bin/echo "TIP: Boot with 'systemd.log_level=debug systemd.log_target=console' for more info"
StandardOutput=journal+console

[Install]
WantedBy=multi-user.target
EOF
    
    echo "Complete getty fix applied"
}

# Export the function
export -f apply_complete_getty_fix