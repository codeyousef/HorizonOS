#!/usr/bin/env bash
# HorizonOS Live Environment Customization Script
# Based on Boot Process and Troubleshooting guide - Complete Working Example

set -e

echo "=== Customizing HorizonOS Live Environment ==="

# Debug: List available desktop configurations
echo "DEBUG: Available desktop configurations:"
ls -la /usr/share/horizonos/desktop/ 2>/dev/null || echo "DEBUG: No desktop configurations found"

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

# Ensure XDG directories are set for proper desktop entry discovery
export XDG_DATA_DIRS="/usr/local/share:/usr/share:$HOME/.local/share"
export XDG_CONFIG_DIRS="/etc/xdg"
export XDG_CACHE_HOME="$HOME/.cache"
export XDG_CONFIG_HOME="$HOME/.config"
export XDG_DATA_HOME="$HOME/.local/share"

echo "Welcome to HorizonOS Live Environment"
echo "To install HorizonOS, run: horizonos-install"
echo ""
EOF

# Set up HorizonOS desktop configurations
echo "Setting up HorizonOS desktop modes..."

# Debug: Check if desktop configurations exist
echo "DEBUG: Checking for desktop configurations:"
ls -la /usr/share/horizonos/desktop/hyprland/ 2>/dev/null || echo "DEBUG: No Hyprland configs found"
ls -la /usr/share/horizonos/desktop/hyprland/configs/ 2>/dev/null || echo "DEBUG: No Hyprland configs directory found"

# Create Hyprland config directory
mkdir -p /home/liveuser/.config/hypr

# Copy configs instead of symlinking to avoid permission issues
if [ -d "/usr/share/horizonos/desktop/hyprland/configs" ]; then
    echo "DEBUG: Copying Hyprland configurations..."
    cp -r /usr/share/horizonos/desktop/hyprland/configs/* /home/liveuser/.config/hypr/
    echo "DEBUG: Hyprland configs copied successfully"
else
    echo "DEBUG: WARNING - No Hyprland configs found, creating minimal fallback"
fi

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
echo "DEBUG: Setting up Waybar configurations..."
mkdir -p /home/liveuser/.config/waybar

# Debug: Check if waybar configs exist
ls -la /usr/share/horizonos/desktop/hyprland/waybar/ 2>/dev/null || echo "DEBUG: No waybar configs found"

if [ -f "/usr/share/horizonos/desktop/hyprland/waybar/config-kde.json" ]; then
    echo "DEBUG: Linking waybar KDE config..."
    ln -sf /usr/share/horizonos/desktop/hyprland/waybar/config-kde.json /home/liveuser/.config/waybar/config
    ln -sf /usr/share/horizonos/desktop/hyprland/waybar/style-kde.css /home/liveuser/.config/waybar/style.css
    echo "DEBUG: Waybar configs linked successfully"
else
    echo "DEBUG: WARNING - Waybar configs not found"
fi

# Debug: Check if scripts exist
echo "DEBUG: Checking for desktop scripts:"
ls -la /usr/share/horizonos/desktop/hyprland/scripts/ 2>/dev/null || echo "DEBUG: No desktop scripts found"

# Create HorizonOS config directory
mkdir -p /home/liveuser/.config/horizonos
echo "kde" > /home/liveuser/.config/horizonos/current-mode

# Create desktop shortcuts - both on Desktop and in applications menu
echo "Creating desktop entries and shortcuts..."
mkdir -p /home/liveuser/Desktop
mkdir -p /home/liveuser/.local/share/applications
mkdir -p /usr/share/applications

# Install HorizonOS
cat > /usr/share/applications/horizonos-install.desktop << 'EOF'
[Desktop Entry]
Version=1.1
Type=Application
Name=Install HorizonOS
Comment=Install HorizonOS to your hard drive
Exec=kitty -e horizonos-install
Icon=system-software-install
Categories=System;
StartupNotify=true
EOF

# Switch Desktop Mode
cat > /usr/share/applications/horizonos-switch-mode.desktop << 'EOF'
[Desktop Entry]
Version=1.1
Name=Switch Desktop Mode
Comment=Switch between KDE, Windows 11, and macOS desktop modes
Exec=kitty -e /usr/share/horizonos/desktop/hyprland/scripts/switch-mode.sh
Icon=preferences-desktop-theme
Terminal=false
Type=Application
Categories=System;Settings;
Keywords=desktop;mode;theme;kde;windows;macos;
StartupNotify=true
EOF

# Try Graph Desktop
cat > /usr/share/applications/horizonos-graph-desktop.desktop << 'EOF'
[Desktop Entry]
Version=1.1
Type=Application
Name=Try Graph Desktop
Comment=Launch the experimental 3D semantic graph desktop
Exec=/usr/share/horizonos/desktop/graph/launch-graph-desktop.sh
Icon=applications-graphics
Categories=System;
StartupNotify=true
EOF

# Copy to user's local applications (wofi searches here too)
cp /usr/share/applications/horizonos-*.desktop /home/liveuser/.local/share/applications/

# Copy to Desktop for easy access
cp /usr/share/applications/horizonos-install.desktop /home/liveuser/Desktop/
cp /usr/share/applications/horizonos-switch-mode.desktop /home/liveuser/Desktop/
cp /usr/share/applications/horizonos-graph-desktop.desktop /home/liveuser/Desktop/

# Make desktop files executable and trusted
chmod +x /home/liveuser/Desktop/*.desktop
chmod +x /home/liveuser/.local/share/applications/*.desktop

# Mark desktop files as trusted (for some desktop environments)
gio set /home/liveuser/Desktop/horizonos-install.desktop "metadata::trusted" true 2>/dev/null || true
gio set /home/liveuser/Desktop/horizonos-switch-mode.desktop "metadata::trusted" true 2>/dev/null || true
gio set /home/liveuser/Desktop/horizonos-graph-desktop.desktop "metadata::trusted" true 2>/dev/null || true

# Update desktop database
echo "Updating desktop database..."
update-desktop-database /usr/share/applications/ || true
update-desktop-database /home/liveuser/.local/share/applications/ || true

# Debug: List created desktop entries
echo "DEBUG: Created desktop entries in /usr/share/applications/:"
ls -la /usr/share/applications/horizonos-*.desktop || echo "No HorizonOS desktop entries found"
echo "DEBUG: Desktop entries in user local applications:"
ls -la /home/liveuser/.local/share/applications/horizonos-*.desktop || echo "No user local desktop entries found"

# Ensure XDG directories are set up for wofi
mkdir -p /home/liveuser/.config/mimeapps.list
echo "DEBUG: XDG directories created"

# Create basic wofi configuration to ensure it searches all application directories
mkdir -p /home/liveuser/.config/wofi
cat > /home/liveuser/.config/wofi/config << 'EOF'
width=600
height=400
location=center
show=drun
prompt=Applications
filter_rate=100
allow_markup=true
no_actions=true
halign=fill
orientation=vertical
content_halign=fill
insensitive=true
allow_images=true
image_size=32
gtk_dark=true
EOF

echo "DEBUG: Wofi configuration created"

# Set up proper XDG environment for desktop entry discovery
cat > /home/liveuser/.profile << 'EOF'
# XDG Base Directory Specification
export XDG_DATA_DIRS="/usr/local/share:/usr/share:$HOME/.local/share"
export XDG_CONFIG_DIRS="/etc/xdg"
export XDG_CACHE_HOME="$HOME/.cache"
export XDG_CONFIG_HOME="$HOME/.config"
export XDG_DATA_HOME="$HOME/.local/share"
EOF

# Also add to system-wide environment
cat > /etc/environment << 'EOF'
XDG_DATA_DIRS="/usr/local/share:/usr/share"
XDG_CONFIG_DIRS="/etc/xdg"
PATH="/usr/local/sbin:/usr/local/bin:/usr/bin:/usr/lib/jvm/default/bin:/usr/bin/site_perl:/usr/bin/vendor_perl:/usr/bin/core_perl"
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