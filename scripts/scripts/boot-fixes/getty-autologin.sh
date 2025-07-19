#!/bin/bash
# Comprehensive fix for getty loop issue

echo "=== Comprehensive Getty Fix ==="

# This function will be called from build-iso.sh
fix_getty_in_iso() {
    local AIROOTFS="$1"
    
    echo "Applying comprehensive getty fixes..."
    
    # 1. Remove any existing autologin configuration from releng
    rm -rf "$AIROOTFS/etc/systemd/system/getty@tty1.service.d"
    
    # 2. Create our own autologin configuration with all protections
    mkdir -p "$AIROOTFS/etc/systemd/system/getty@tty1.service.d"
    
    cat > "$AIROOTFS/etc/systemd/system/getty@tty1.service.d/autologin.conf" << 'EOF'
[Unit]
# Prevent rapid restarts
StartLimitInterval=60s
StartLimitBurst=3

[Service]
# Clear any existing ExecStart
ExecStart=
# Use the correct path and parameters
ExecStart=-/usr/bin/agetty --autologin root --noclear %I 38400 linux
# Ensure service type is correct
Type=idle
# Add restart protection
Restart=no
RestartSec=1s
# Ensure proper terminal setup
StandardInput=tty
StandardOutput=journal+console
StandardError=journal+console
TTYPath=/dev/%I
TTYReset=yes
TTYVHangup=yes
EOF

    # 3. Create a fallback configuration
    cat > "$AIROOTFS/etc/systemd/system/getty@tty1.service.d/fallback.conf" << 'EOF'
[Service]
# If autologin fails, fall back to manual login
RestartForceExitStatus=1
EOF

    # 4. Mask other TTYs to prevent conflicts
    for i in {2..6}; do
        ln -sf /dev/null "$AIROOTFS/etc/systemd/system/getty@tty$i.service"
    done
    
    # 5. Create debug helper
    mkdir -p "$AIROOTFS/usr/local/bin"
    cat > "$AIROOTFS/usr/local/bin/debug-getty" << 'EOF'
#!/bin/bash
echo "Getty Debug Information:"
echo "======================="
systemctl status getty@tty1.service
echo ""
echo "Journal logs:"
journalctl -u getty@tty1.service -n 50
echo ""
echo "Configuration:"
cat /etc/systemd/system/getty@tty1.service.d/*.conf
EOF
    chmod +x "$AIROOTFS/usr/local/bin/debug-getty"
    
    # 6. Ensure no conflicting services
    rm -f "$AIROOTFS/etc/systemd/system/autovt@.service"
    rm -f "$AIROOTFS/etc/systemd/system/serial-getty@*.service"
    
    # 7. Create a verification script
    cat > "$AIROOTFS/usr/local/bin/verify-getty" << 'EOF'
#!/bin/bash
if [ -f /usr/bin/agetty ]; then
    echo "✓ agetty found at /usr/bin/agetty"
else
    echo "✗ agetty NOT found at /usr/bin/agetty"
    which agetty || echo "agetty not in PATH"
fi

if [ -f /etc/systemd/system/getty@tty1.service.d/autologin.conf ]; then
    echo "✓ autologin.conf exists"
    grep -q "/usr/bin/agetty" /etc/systemd/system/getty@tty1.service.d/autologin.conf && \
        echo "✓ Using correct agetty path" || \
        echo "✗ Wrong agetty path"
else
    echo "✗ autologin.conf missing"
fi

# Check if getty services are properly enabled
echo ""
echo "Getty service enablement:"
if [ -L /etc/systemd/system/getty.target.wants/getty@tty1.service ]; then
    echo "✓ getty@tty1.service is enabled in getty.target"
else
    echo "✗ getty@tty1.service is NOT enabled in getty.target"
fi

if [ -L /etc/systemd/system/multi-user.target.wants/getty.target ]; then
    echo "✓ getty.target is wanted by multi-user.target"
else
    echo "✗ getty.target is NOT wanted by multi-user.target"
fi
EOF
    chmod +x "$AIROOTFS/usr/local/bin/verify-getty"
    
    # 8. Ensure getty@tty1.service has proper [Install] section handling
    # Create a drop-in that ensures the service can be properly enabled
    cat > "$AIROOTFS/etc/systemd/system/getty@tty1.service.d/install.conf" << 'EOF'
[Install]
WantedBy=getty.target
Alias=getty@tty1.service
EOF
    
    echo "Getty fixes applied successfully"
}

# Export the function so it can be used
export -f fix_getty_in_iso