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

# Since we're text-only, set multi-user target
systemctl set-default multi-user.target

# CRITICAL: Handle getty@tty1 to prevent issues
# Option 1 from guide: Mask getty@tty1 completely (simplest, most reliable)
echo "Masking getty@tty1 to prevent conflicts..."
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