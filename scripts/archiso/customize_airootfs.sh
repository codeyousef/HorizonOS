#!/usr/bin/env bash
# HorizonOS Live Environment Customization Script
# Based on Boot Process and Troubleshooting guide - Complete Working Example

set -e

echo "=== Customizing HorizonOS Live Environment ==="

# Create live user (following the guide's recommendation - NOT root autologin)
echo "Creating liveuser..."
useradd -m -G wheel,audio,video,storage,power -s /bin/bash liveuser
echo "liveuser:live" | chpasswd

# Enable sudo without password for wheel group
sed -i 's/^# %wheel ALL=(ALL) NOPASSWD: ALL/%wheel ALL=(ALL) NOPASSWD: ALL/' /etc/sudoers

# Set system locale
echo "Setting locale..."
echo "en_US.UTF-8 UTF-8" > /etc/locale.gen
locale-gen
echo "LANG=en_US.UTF-8" > /etc/locale.conf

# Set timezone to UTC for live environment
ln -sf /usr/share/zoneinfo/UTC /etc/localtime

# Enable essential services
echo "Enabling services..."
systemctl enable NetworkManager.service
systemctl enable sddm.service

# Set default target to graphical for GUI
systemctl set-default graphical.target

# Configure SDDM for autologin
echo "Configuring SDDM autologin..."
mkdir -p /etc/sddm.conf.d
cat > /etc/sddm.conf.d/autologin.conf << 'EOF'
[Autologin]
User=liveuser
Session=plasma.desktop
EOF

# CRITICAL: Disable getty@tty1 to prevent conflicts with SDDM
echo "Masking getty@tty1 to prevent conflicts with display manager..."
systemctl mask getty@tty1.service

# Create .bashrc for liveuser with helpful aliases
cat > /home/liveuser/.bashrc << 'EOF'
# HorizonOS Live Environment
PS1='[\u@horizonos \W]\$ '
alias ll='ls -la'
alias horizonos-install='/usr/local/bin/horizonos-install'

echo "Welcome to HorizonOS Live Environment"
echo "To install HorizonOS, run: horizonos-install"
echo ""
EOF

# Create desktop shortcuts for easy access
mkdir -p /home/liveuser/Desktop
cat > /home/liveuser/Desktop/Install\ HorizonOS.desktop << 'EOF'
[Desktop Entry]
Type=Application
Name=Install HorizonOS
Exec=konsole -e horizonos-install
Icon=applications-system
Categories=System;
EOF

chmod +x /home/liveuser/Desktop/Install\ HorizonOS.desktop

# Fix permissions
chown -R liveuser:liveuser /home/liveuser
chmod 755 /home/liveuser

# Create a simple motd
cat > /etc/motd << 'EOF'
Welcome to HorizonOS Live
To install: horizonos-install

For debugging: debug-boot
EOF

# Ensure essential directories exist
mkdir -p /etc/systemd/system
mkdir -p /etc/systemd/system/getty.target.wants
mkdir -p /etc/systemd/system/multi-user.target.wants

# Clean up
rm -f /etc/machine-id
rm -rf /tmp/*
rm -rf /var/cache/pacman/pkg/*

echo "=== Customization Complete ==="