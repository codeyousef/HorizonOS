#!/bin/bash
# Proper getty enablement fix for archiso
# This ensures getty services actually start after multi-user.target

echo "=== Proper Getty Service Enablement ==="

fix_getty_enablement() {
    local AIROOTFS="$1"
    
    echo "Applying proper getty service enablement..."
    
    # 1. The KEY issue: archiso may have its own getty configuration that conflicts
    # Remove ALL existing getty configurations first
    rm -rf "$AIROOTFS/etc/systemd/system/getty@tty1.service"
    rm -rf "$AIROOTFS/etc/systemd/system/getty@tty1.service.d"
    rm -rf "$AIROOTFS/etc/systemd/system/getty.target.wants"
    rm -rf "$AIROOTFS/etc/systemd/system/multi-user.target.wants/getty@"*
    
    # 2. Create the EXACT symlinks that systemd expects
    # This is what 'systemctl enable getty@tty1.service' actually does:
    mkdir -p "$AIROOTFS/etc/systemd/system/getty.target.wants"
    ln -sf /usr/lib/systemd/system/getty@.service "$AIROOTFS/etc/systemd/system/getty.target.wants/getty@tty1.service"
    
    # 3. CRITICAL: Ensure getty.target is pulled in by multi-user.target
    # Without this, getty.target may never be reached!
    mkdir -p "$AIROOTFS/etc/systemd/system/multi-user.target.wants"
    ln -sf /usr/lib/systemd/system/getty.target "$AIROOTFS/etc/systemd/system/multi-user.target.wants/getty.target"
    
    # 4. Force multi-user.target to wait for getty.target
    mkdir -p "$AIROOTFS/etc/systemd/system/multi-user.target.d"
    cat > "$AIROOTFS/etc/systemd/system/multi-user.target.d/getty-wait.conf" << 'EOF'
[Unit]
# Ensure getty.target is reached before declaring multi-user.target complete
Wants=getty.target
After=getty.target
EOF
    
    # 5. Create a simple always-working getty service as backup
    cat > "$AIROOTFS/etc/systemd/system/simple-getty.service" << 'EOF'
[Unit]
Description=Simple Getty on tty1
After=multi-user.target
ConditionPathExists=/dev/tty1
Conflicts=getty@tty1.service

[Service]
Type=idle
ExecStartPre=/bin/echo "Starting Simple Getty on tty1..." > /dev/console
ExecStart=/sbin/agetty --autologin root --noclear tty1 38400 linux
Restart=no
StandardInput=tty
StandardOutput=tty
TTYPath=/dev/tty1
TTYReset=yes
TTYVHangup=yes

[Install]
WantedBy=multi-user.target
EOF
    
    # Enable the simple getty
    ln -sf /etc/systemd/system/simple-getty.service "$AIROOTFS/etc/systemd/system/multi-user.target.wants/simple-getty.service"
    
    # 6. Add boot diagnostics
    cat > "$AIROOTFS/usr/local/bin/getty-status" << 'EOF'
#!/bin/bash
echo "=== Getty Status Check ==="
echo "Date: $(date)"
echo ""
echo "Active targets:"
systemctl list-units --type=target --state=active
echo ""
echo "Getty target status:"
systemctl status getty.target
echo ""
echo "Getty services:"
systemctl status 'getty@*.service'
echo ""
echo "Failed services:"
systemctl --failed
echo ""
echo "Dependency tree for multi-user.target:"
systemctl list-dependencies multi-user.target
EOF
    chmod +x "$AIROOTFS/usr/local/bin/getty-status"
    
    # 7. Create a service that runs getty-status at boot for debugging
    cat > "$AIROOTFS/etc/systemd/system/getty-debug.service" << 'EOF'
[Unit]
Description=Getty Debug Information
After=multi-user.target

[Service]
Type=oneshot
ExecStart=/usr/local/bin/getty-status > /dev/tty1 2>&1
RemainAfterExit=yes
StandardOutput=tty
TTYPath=/dev/tty1

[Install]
WantedBy=multi-user.target
EOF
    
    ln -sf /etc/systemd/system/getty-debug.service "$AIROOTFS/etc/systemd/system/multi-user.target.wants/getty-debug.service"
    
    echo "Getty enablement fixes applied"
}

# Export the function
export -f fix_getty_enablement