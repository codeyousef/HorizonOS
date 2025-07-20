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

# Create Hyprland traditional mode configuration
echo "Setting up Hyprland traditional mode configuration..."
mkdir -p /home/liveuser/.config/hypr
cat > /home/liveuser/.config/hypr/hyprland.conf << 'EOF'
# HorizonOS Hyprland Traditional Mode Configuration

# Monitor configuration
monitor=,preferred,auto,auto

# Input configuration
input {
    kb_layout = us
    kb_variant = 
    kb_model =
    kb_options =
    kb_rules =

    follow_mouse = 1

    touchpad {
        natural_scroll = no
    }

    sensitivity = 0 # -1.0 - 1.0, 0 means no modification.
}

# General configuration for traditional desktop feel
general {
    gaps_in = 5
    gaps_out = 20
    border_size = 2
    col.active_border = rgba(33ccffee) rgba(00ff99ee) 45deg
    col.inactive_border = rgba(595959aa)

    layout = dwindle
}

# Decoration for traditional look
decoration {
    rounding = 5
    
    blur {
        enabled = true
        size = 3
        passes = 1
    }

    drop_shadow = yes
    shadow_range = 4
    shadow_render_power = 3
    col.shadow = rgba(1a1a1aee)
}

# Animations for smooth experience
animations {
    enabled = yes

    bezier = myBezier, 0.05, 0.9, 0.1, 1.05

    animation = windows, 1, 7, myBezier
    animation = windowsOut, 1, 7, default, popin 80%
    animation = border, 1, 10, default
    animation = borderangle, 1, 8, default
    animation = fade, 1, 7, default
    animation = workspaces, 1, 6, default
}

# Layout configuration
dwindle {
    pseudotile = yes
    preserve_split = yes
}

# Window rules for traditional behavior
windowrule = float, file_progress
windowrule = float, confirm
windowrule = float, dialog
windowrule = float, download
windowrule = float, notification
windowrule = float, error
windowrule = float, splash
windowrule = float, confirmreset

# Key bindings for traditional desktop
$mainMod = SUPER

# Basic window management
bind = $mainMod, Q, exec, kitty
bind = $mainMod, C, killactive, 
bind = $mainMod, M, exit, 
bind = $mainMod, E, exec, nautilus
bind = $mainMod, V, togglefloating, 
bind = $mainMod, R, exec, wofi --show drun
bind = $mainMod, P, pseudo, # dwindle
bind = $mainMod, J, togglesplit, # dwindle

# Move focus with mainMod + arrow keys
bind = $mainMod, left, movefocus, l
bind = $mainMod, right, movefocus, r
bind = $mainMod, up, movefocus, u
bind = $mainMod, down, movefocus, d

# Switch workspaces with mainMod + [0-9]
bind = $mainMod, 1, workspace, 1
bind = $mainMod, 2, workspace, 2
bind = $mainMod, 3, workspace, 3
bind = $mainMod, 4, workspace, 4
bind = $mainMod, 5, workspace, 5
bind = $mainMod, 6, workspace, 6
bind = $mainMod, 7, workspace, 7
bind = $mainMod, 8, workspace, 8
bind = $mainMod, 9, workspace, 9
bind = $mainMod, 0, workspace, 10

# Move active window to a workspace with mainMod + SHIFT + [0-9]
bind = $mainMod SHIFT, 1, movetoworkspace, 1
bind = $mainMod SHIFT, 2, movetoworkspace, 2
bind = $mainMod SHIFT, 3, movetoworkspace, 3
bind = $mainMod SHIFT, 4, movetoworkspace, 4
bind = $mainMod SHIFT, 5, movetoworkspace, 5
bind = $mainMod SHIFT, 6, movetoworkspace, 6
bind = $mainMod SHIFT, 7, movetoworkspace, 7
bind = $mainMod SHIFT, 8, movetoworkspace, 8
bind = $mainMod SHIFT, 9, movetoworkspace, 9
bind = $mainMod SHIFT, 0, movetoworkspace, 10

# Scroll through existing workspaces with mainMod + scroll
bind = $mainMod, mouse_down, workspace, e+1
bind = $mainMod, mouse_up, workspace, e-1

# Move/resize windows with mainMod + LMB/RMB and dragging
bindm = $mainMod, mouse:272, movewindow
bindm = $mainMod, mouse:273, resizewindow

# Screenshot bindings
bind = , Print, exec, grim -g "$(slurp)" - | wl-copy
bind = SHIFT, Print, exec, grim - | wl-copy

# Auto-start applications for traditional desktop
exec-once = waybar
exec-once = /usr/lib/polkit-gnome/polkit-gnome-authentication-agent-1
exec-once = dbus-update-activation-environment --systemd WAYLAND_DISPLAY XDG_CURRENT_DESKTOP
EOF

# Create Waybar configuration for traditional desktop experience
mkdir -p /home/liveuser/.config/waybar
cat > /home/liveuser/.config/waybar/config << 'EOF'
{
    "layer": "top",
    "position": "top",
    "height": 30,
    "spacing": 4,
    "modules-left": ["hyprland/workspaces"],
    "modules-center": ["hyprland/window"],
    "modules-right": ["pulseaudio", "network", "cpu", "memory", "clock", "tray"],
    
    "hyprland/workspaces": {
        "disable-scroll": true,
        "all-outputs": true,
        "format": "{name}: {icon}",
        "format-icons": {
            "1": "",
            "2": "",
            "3": "",
            "4": "",
            "5": "",
            "urgent": "",
            "focused": "",
            "default": ""
        }
    },
    
    "hyprland/window": {
        "format": "{}",
        "max-length": 50
    },
    
    "tray": {
        "spacing": 10
    },
    
    "clock": {
        "format": "{:%Y-%m-%d %H:%M:%S}",
        "interval": 1
    },
    
    "cpu": {
        "format": "{usage}% ",
        "tooltip": false
    },
    
    "memory": {
        "format": "{}% "
    },
    
    "network": {
        "format-wifi": "{essid} ({signalStrength}%) ",
        "format-ethernet": "{ipaddr}/{cidr} ",
        "tooltip-format": "{ifname} via {gwaddr} ",
        "format-linked": "{ifname} (No IP) ",
        "format-disconnected": "Disconnected ⚠",
        "format-alt": "{ifname}: {ipaddr}/{cidr}"
    },
    
    "pulseaudio": {
        "format": "{volume}% {icon} {format_source}",
        "format-bluetooth": "{volume}% {icon} {format_source}",
        "format-bluetooth-muted": " {icon} {format_source}",
        "format-muted": " {format_source}",
        "format-source": "{volume}% ",
        "format-source-muted": "",
        "format-icons": {
            "headphone": "",
            "hands-free": "",
            "headset": "",
            "phone": "",
            "portable": "",
            "car": "",
            "default": ["", "", ""]
        },
        "on-click": "pavucontrol"
    }
}
EOF

# Create Waybar style for traditional look
cat > /home/liveuser/.config/waybar/style.css << 'EOF'
* {
    border: none;
    border-radius: 0;
    font-family: "JetBrains Mono", sans-serif;
    font-size: 13px;
    min-height: 0;
}

window#waybar {
    background-color: rgba(43, 48, 59, 0.9);
    border-bottom: 3px solid rgba(100, 114, 125, 0.5);
    color: #ffffff;
    transition-property: background-color;
    transition-duration: .5s;
}

#workspaces button {
    padding: 0 5px;
    background-color: transparent;
    color: #ffffff;
}

#workspaces button:hover {
    background: rgba(0, 0, 0, 0.2);
}

#workspaces button.focused {
    background-color: #64727D;
}

#workspaces button.urgent {
    background-color: #eb4d4b;
}

#clock,
#cpu,
#memory,
#disk,
#temperature,
#network,
#pulseaudio,
#tray {
    padding: 0 10px;
    color: #ffffff;
}

#window,
#workspaces {
    margin: 0 4px;
}

#clock {
    background-color: #64727D;
}

#cpu {
    background-color: #2ecc71;
    color: #000000;
}

#memory {
    background-color: #9b59b6;
}

#network {
    background-color: #2980b9;
}

#pulseaudio {
    background-color: #f1c40f;
    color: #000000;
}

#tray {
    background-color: #2980b9;
}
EOF

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