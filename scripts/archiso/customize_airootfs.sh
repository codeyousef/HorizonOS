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

# Enable audio services for Hyprland
systemctl --user enable pipewire.service
systemctl --user enable pipewire-pulse.service
systemctl --user enable wireplumber.service

# Enable VM guest tools detection service
systemctl enable horizonos-vm-setup.service

# Set default target to graphical for GUI
systemctl set-default graphical.target

# Configure SDDM for autologin with Hyprland
echo "Configuring SDDM autologin for Hyprland..."
mkdir -p /etc/sddm.conf.d
cat > /etc/sddm.conf.d/autologin.conf << 'EOF'
[Autologin]
User=liveuser
Session=hyprland.desktop
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

# Set up HorizonOS desktop configurations
echo "Setting up HorizonOS desktop modes..."

# Create Hyprland config directory
mkdir -p /home/liveuser/.config/hypr

# Copy configs instead of symlinking to avoid permission issues
cp -r /usr/share/horizonos/desktop/hyprland/configs/* /home/liveuser/.config/hypr/

# Also create a fallback config in /etc/skel for new users
mkdir -p /etc/skel/.config/hypr
cp -r /usr/share/horizonos/desktop/hyprland/configs/* /etc/skel/.config/hypr/

# Ensure configs are readable
chmod -R 755 /usr/share/horizonos/desktop/hyprland/
chmod -R 755 /home/liveuser/.config/hypr/

# Create a minimal backup config as fallback
cat > /home/liveuser/.config/hypr/hyprland.conf.backup << 'EOF'
# Minimal HorizonOS Hyprland Config (Fallback)
monitor=,preferred,auto,auto
exec-once = waybar
exec-once = swaybg -c "#1e1e2e"

input {
    kb_layout = us
    follow_mouse = 1
}

general {
    gaps_in = 5
    gaps_out = 10
    border_size = 2
    col.active_border = rgba(33ccffee) rgba(00ff99ee) 45deg
    col.inactive_border = rgba(595959aa)
}

decoration {
    rounding = 10
}

$mainMod = SUPER

bind = $mainMod, Return, exec, kitty
bind = $mainMod, Q, killactive
bind = $mainMod SHIFT, E, exit
bind = $mainMod, D, exec, wofi --show drun
EOF

# Set up Waybar with KDE style as default
mkdir -p /home/liveuser/.config/waybar
ln -sf /usr/share/horizonos/desktop/hyprland/waybar/config-kde.json /home/liveuser/.config/waybar/config
ln -sf /usr/share/horizonos/desktop/hyprland/waybar/style-kde.css /home/liveuser/.config/waybar/style.css

# Create HorizonOS config directory
mkdir -p /home/liveuser/.config/horizonos
echo "kde" > /home/liveuser/.config/horizonos/current-mode

# Create desktop shortcuts for easy access
mkdir -p /home/liveuser/Desktop
cat > /home/liveuser/Desktop/Install\ HorizonOS.desktop << 'EOF'
[Desktop Entry]
Type=Application
Name=Install HorizonOS
Exec=kitty -e horizonos-install
Icon=applications-system
Categories=System;
EOF

cat > /home/liveuser/Desktop/Switch\ Desktop\ Mode.desktop << 'EOF'
[Desktop Entry]
Type=Application
Name=Switch Desktop Mode
Comment=Switch between KDE, Windows 11, and macOS desktop modes
Exec=kitty -e /usr/share/horizonos/desktop/hyprland/scripts/switch-mode.sh
Icon=preferences-desktop-theme
Categories=System;Settings;
EOF

cat > /home/liveuser/Desktop/Try\ Graph\ Desktop.desktop << 'EOF'
[Desktop Entry]
Type=Application
Name=Try Graph Desktop
Comment=Launch the experimental 3D semantic graph desktop
Exec=/usr/share/horizonos/desktop/graph/launch-graph-desktop.sh
Icon=applications-graphics
Categories=System;
EOF

chmod +x /home/liveuser/Desktop/*.desktop

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