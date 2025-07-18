```
#!/bin/bash
# Add to scripts/scripts/build-test.sh or create new file

# Function to add update system to rootfs
install_update_system() {
    local ROOTFS_DIR="$1"
    
    echo "Installing HorizonOS update system..."
    
    # Create directories
    sudo mkdir -p "$ROOTFS_DIR/usr/local/bin"
    sudo mkdir -p "$ROOTFS_DIR/etc/systemd/system"
    sudo mkdir -p "$ROOTFS_DIR/etc/horizonos"
    sudo mkdir -p "$ROOTFS_DIR/var/cache/horizonos/updates"
    
    # Install update script
    sudo cp scripts/tools/horizonos-autoupdate "$ROOTFS_DIR/usr/local/bin/"
    sudo chmod +x "$ROOTFS_DIR/usr/local/bin/horizonos-autoupdate"
    
    # Install systemd units
    sudo tee "$ROOTFS_DIR/etc/systemd/system/horizonos-update.service" > /dev/null << 'EOF'
[Unit]
Description=HorizonOS Automatic Update Service
After=network-online.target
Wants=network-online.target

[Service]
Type=oneshot
ExecStart=/usr/local/bin/horizonos-autoupdate timer
StandardOutput=journal
StandardError=journal
User=root
Restart=on-failure
RestartSec=1h
EOF

    sudo tee "$ROOTFS_DIR/etc/systemd/system/horizonos-update.timer" > /dev/null << 'EOF'
[Unit]
Description=HorizonOS Automatic Update Timer
After=network-online.target
Wants=network-online.target

[Timer]
OnCalendar=daily
Persistent=true
RandomizedDelaySec=1h

[Install]
WantedBy=timers.target
EOF

    # Create default update configuration
    sudo tee "$ROOTFS_DIR/etc/horizonos/update.conf" > /dev/null << 'EOF'
# HorizonOS Update Configuration
UPDATE_CHANNEL="stable"
AUTO_STAGE="true"
AUTO_REBOOT="false"
CHECK_INTERVAL="86400"
GITHUB_REPO="codeyousef/HorizonOS"
EOF

    # Create update notification script
    sudo tee "$ROOTFS_DIR/usr/local/bin/horizonos-update-notify" > /dev/null << 'EOF'
#!/bin/bash
# Desktop notification for updates

if [ -f /var/cache/horizonos/updates/available-version ]; then
    VERSION=$(cat /var/cache/horizonos/updates/available-version)
    
    # Try different notification methods
    if command -v notify-send &>/dev/null; then
        notify-send -i system-software-update \
            "HorizonOS Update Available" \
            "Version $VERSION is ready to install. Run 'horizonos-autoupdate update' or reboot to apply."
    elif command -v kdialog &>/dev/null; then
        kdialog --title "HorizonOS Update" \
            --passivepopup "Version $VERSION is available" 10
    fi
fi
EOF
    sudo chmod +x "$ROOTFS_DIR/usr/local/bin/horizonos-update-notify"
    
    # Enable systemd timer by default
    sudo ln -sf /etc/systemd/system/horizonos-update.timer \
        "$ROOTFS_DIR/etc/systemd/system/timers.target.wants/horizonos-update.timer"
    
    echo "Update system installed successfully"
}

# Function to create update metadata
create_update_metadata() {
    local REPO_DIR="$1"
    local VERSION="$2"
    
    # Create metadata file for the update
    cat > "$REPO_DIR/horizonos-update.json" << EOF
{
  "version": "$VERSION",
  "channel": "stable",
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "commit": "$(ostree log --repo="$REPO_DIR" horizonos/test/x86_64 | head -1 | cut -d' ' -f2)",
  "changes": [
    "OSTree atomic updates",
    "Container-based architecture",
    "Auto-update system"
  ]
}
EOF
}

# Add to your build process
if [ "${1:-}" = "--install-updates" ]; then
    install_update_system "$2"
    create_update_metadata "$3" "$4"
fi
```