#!/bin/bash
# Comprehensive Getty Fix for HorizonOS Live ISO
# Based on analysis of working Arch-based ISOs (archiso, EndeavourOS, Manjaro)

echo "=== Applying Comprehensive Getty Configuration ==="

apply_comprehensive_getty_fix() {
    local AIROOTFS="$1"
    
    echo "1. Detecting agetty binary location..."
    
    # Check where agetty actually is in the build environment
    local AGETTY_PATH=""
    if [ -f "$AIROOTFS/usr/bin/agetty" ]; then
        AGETTY_PATH="/usr/bin/agetty"
        echo "   Found agetty at /usr/bin/agetty"
    elif [ -f "$AIROOTFS/sbin/agetty" ]; then
        AGETTY_PATH="/sbin/agetty"
        echo "   Found agetty at /sbin/agetty"
    else
        # Default to /sbin/agetty as per standard
        AGETTY_PATH="/sbin/agetty"
        echo "   Warning: agetty not found in airootfs, using default /sbin/agetty"
    fi
    
    echo "2. Creating getty@tty1 autologin configuration..."
    
    # Create the override directory
    mkdir -p "$AIROOTFS/etc/systemd/system/getty@tty1.service.d"
    
    # Create autologin configuration with restart protection
    cat > "$AIROOTFS/etc/systemd/system/getty@tty1.service.d/autologin.conf" << EOF
[Unit]
# Add condition to prevent starting if device doesn't exist
ConditionPathExists=/dev/%I

[Service]
# Clear the existing ExecStart
ExecStart=
# Set the new ExecStart with detected agetty path
ExecStart=-${AGETTY_PATH} --autologin root --noclear %I 38400 linux
# Add restart delay to prevent rapid restart loops
RestartSec=1
# Ensure service restarts on failure
Restart=always
EOF
    
    echo "3. Creating getty@tty2-6 configurations for manual login..."
    
    # Create configurations for other TTYs without autologin
    for tty in tty2 tty3 tty4 tty5 tty6; do
        mkdir -p "$AIROOTFS/etc/systemd/system/getty@${tty}.service.d"
        cat > "$AIROOTFS/etc/systemd/system/getty@${tty}.service.d/override.conf" << EOF
[Unit]
ConditionPathExists=/dev/%I

[Service]
RestartSec=1
EOF
    done
    
    echo "4. Creating getty.target.wants symlinks..."
    
    # Create getty.target.wants directory
    mkdir -p "$AIROOTFS/etc/systemd/system/getty.target.wants"
    
    # Create symlinks for TTY1-6 (standard archiso approach)
    for i in {1..6}; do
        ln -sf /usr/lib/systemd/system/getty@.service \
            "$AIROOTFS/etc/systemd/system/getty.target.wants/getty@tty${i}.service"
    done
    
    echo "5. Creating serial console configuration..."
    
    # Add serial console support for debugging
    mkdir -p "$AIROOTFS/etc/systemd/system/serial-getty@ttyS0.service.d"
    cat > "$AIROOTFS/etc/systemd/system/serial-getty@ttyS0.service.d/autologin.conf" << EOF
[Service]
ExecStart=
ExecStart=-${AGETTY_PATH} --autologin root --noclear --keep-baud 115200,57600,38400,9600 %I $TERM
RestartSec=1
EOF
    
    echo "6. Creating container getty fix (if needed)..."
    
    # Fix for container-getty services that might cause loops
    mkdir -p "$AIROOTFS/etc/systemd/system/container-getty@.service.d"
    cat > "$AIROOTFS/etc/systemd/system/container-getty@.service.d/fix-restart.conf" << EOF
[Unit]
ConditionPathExists=/dev/pts/%I

[Service]
RestartSec=1
EOF
    
    echo "7. Creating debug helper script..."
    
    # Create a helper script to check getty status
    mkdir -p "$AIROOTFS/usr/local/bin"
    cat > "$AIROOTFS/usr/local/bin/check-getty" << 'SCRIPT'
#!/bin/bash
echo "=== Getty Status Check ==="
echo "Agetty location: $(which agetty)"
echo "Getty services:"
systemctl list-units 'getty@*.service' --all --no-pager
echo ""
echo "Failed services:"
systemctl --failed --no-pager
echo ""
echo "Getty target dependencies:"
systemctl list-dependencies getty.target --no-pager
SCRIPT
    chmod +x "$AIROOTFS/usr/local/bin/check-getty"
    
    echo "8. Ensuring multi-user.target pulls in getty.target..."
    
    # Create multi-user.target.wants if it doesn't exist
    mkdir -p "$AIROOTFS/etc/systemd/system/multi-user.target.wants"
    
    # Ensure getty.target is wanted by multi-user.target (should be automatic, but being explicit)
    # Note: getty.target is usually pulled in automatically, but we can add a drop-in to be sure
    mkdir -p "$AIROOTFS/etc/systemd/system/multi-user.target.d"
    cat > "$AIROOTFS/etc/systemd/system/multi-user.target.d/wants-getty.conf" << EOF
[Unit]
# Explicitly want getty.target to ensure consoles are available
Wants=getty.target
EOF
    
    echo "9. Creating fallback emergency getty..."
    
    # Create an emergency getty service as fallback
    cat > "$AIROOTFS/etc/systemd/system/emergency-getty@.service" << EOF
[Unit]
Description=Emergency Getty on %I
Documentation=man:agetty(8) man:systemd-getty-generator(8)
ConditionPathExists=/dev/%I
After=systemd-user-sessions.service plymouth-quit-wait.service
After=rc-local.service
Before=getty.target
IgnoreOnIsolate=yes

[Service]
ExecStart=-${AGETTY_PATH} --noclear %I 38400 linux
Type=idle
Restart=always
RestartSec=5
UtmpIdentifier=%I
TTYPath=/dev/%I
TTYReset=yes
TTYVHangup=yes
TTYVTDisallocate=yes
KillMode=process
IgnoreSIGPIPE=no
SendSIGHUP=yes

[Install]
WantedBy=getty.target
EOF
    
    echo "Comprehensive getty configuration completed!"
    echo ""
    echo "Configuration summary:"
    echo "- Agetty path: ${AGETTY_PATH}"
    echo "- Autologin enabled on tty1"
    echo "- Manual login on tty2-6"
    echo "- RestartSec=1 to prevent rapid loops"
    echo "- ConditionPathExists to check device availability"
    echo "- Serial console support on ttyS0"
    echo "- Emergency fallback getty service"
    echo "- Debug helper at /usr/local/bin/check-getty"
}

# Export the function
export -f apply_comprehensive_getty_fix