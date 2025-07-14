# HorizonOS Hyprland Desktop - KDE Style

This directory contains the Hyprland compositor configuration that provides a KDE Plasma-like experience for HorizonOS users.

## Overview

The Hyprland KDE-style desktop provides:
- Familiar KDE keyboard shortcuts and window management
- KDE application integration (Dolphin, Konsole, Kate)
- Breeze theming throughout the desktop
- Waybar configured to look and behave like KDE's panel
- Wofi launcher styled like KDE's application menu

## Directory Structure

```
hyprland/
├── configs/              # Core Hyprland configuration files
│   ├── hyprland.conf    # Main configuration
│   ├── binds.conf       # KDE-style keybindings
│   ├── rules.conf       # Window rules and workspace assignments
│   ├── exec.conf        # Autostart applications
│   └── appearance.conf  # Theming and visual settings
├── waybar/              # Status bar configuration
│   ├── config-kde.json  # KDE-style panel layout
│   └── style-kde.css    # Breeze theme styling
├── wofi/                # Application launcher
│   ├── config           # Launcher behavior settings
│   └── style.css        # KDE-style appearance
├── scripts/             # Helper scripts
│   ├── install-configs.sh    # Install configurations to user directory
│   ├── screenshot.sh         # KDE Spectacle-like screenshot tool
│   └── mode-indicator.sh     # Desktop mode indicator
├── services/            # Systemd user services
│   ├── hyprland-session.service
│   ├── waybar.service
│   └── xdg-desktop-portal-hyprland.service
├── themes/              # Theme configurations
│   └── breeze-dark/     # KDE Breeze Dark theme
└── packages.list        # Required packages

```

## Installation

### 1. Install Required Packages

```bash
# Install from the packages.list
sudo pacman -S --needed $(grep -v '^#' packages.list | grep -v '^$')
```

### 2. Install Configurations

Run the installation script:
```bash
./scripts/install-configs.sh
```

This will:
- Back up any existing configurations
- Install all Hyprland configs to `~/.config/hypr/`
- Install Waybar configs to `~/.config/waybar/`
- Install Wofi configs to `~/.config/wofi/`
- Set up environment variables

### 3. Enable Systemd Services (Optional)

For session management:
```bash
systemctl --user enable hyprland-session.service
systemctl --user enable waybar.service
systemctl --user enable xdg-desktop-portal-hyprland.service
```

## Usage

### Starting Hyprland

From a TTY:
```bash
Hyprland
```

Or select "Hyprland" from your display manager.

### Default Keybindings (KDE-style)

#### Window Management
- `Super + Q` - Close window
- `Super + M` - Maximize window
- `Super + F` - Fullscreen
- `Super + Shift + F` - Toggle floating
- `Alt + Tab` - Switch windows
- `Super + Tab` - Switch workspaces

#### Applications
- `Super` - Open application launcher
- `Super + E` - File manager (Dolphin)
- `Ctrl + Alt + T` - Terminal (Konsole)
- `Super + R` - Run command
- `Super + L` - Lock screen

#### Workspace Navigation
- `Super + 1-9` - Switch to workspace 1-9
- `Ctrl + F1-F4` - Switch to workspace 1-4 (KDE style)
- `Super + Shift + 1-9` - Move window to workspace

#### Screenshots
- `Print` - Region screenshot to clipboard
- `Shift + Print` - Full screenshot to clipboard
- `Ctrl + Print` - Region screenshot to file
- `Ctrl + Shift + Print` - Full screenshot to file

#### System
- `Super + Shift + L` - Logout
- `Super + X` - Reload configuration

## Configuration

### Modifying Settings

All configuration files follow a modular approach:
- Edit `~/.config/hypr/hyprland.conf` for general settings
- Edit `~/.config/hypr/binds.conf` for keybindings
- Edit `~/.config/hypr/appearance.conf` for theming

### Waybar Customization

The panel can be customized by editing:
- `~/.config/waybar/config-kde.json` - Module configuration
- `~/.config/waybar/style-kde.css` - Visual styling

### Theme Selection

Currently includes Breeze Dark theme. To add more themes:
1. Create a new directory under `themes/`
2. Add `theme.conf` with color definitions
3. Update `appearance.conf` to use the new theme

## Integration with HorizonOS

### Kotlin DSL Configuration

In your HorizonOS configuration, specify Hyprland with KDE integration:

```kotlin
horizonOS {
    desktop {
        environment = DesktopEnvironment.HYPRLAND
        
        hyprland {
            theme = "breeze-dark"
            animations = true
            gaps = 10
            borderSize = 2
            kdeIntegration = true
            personalityMode = PersonalityMode.KDE
        }
    }
}
```

### Mode Switching

The desktop supports multiple personality modes:
- KDE (default) - Full KDE experience
- GNOME - GNOME-like behavior (planned)
- macOS - macOS-like behavior (planned)
- Windows 11 - Windows-like behavior (planned)

Switch modes using:
```bash
~/.config/hypr/scripts/mode-indicator.sh kde
```

## Troubleshooting

### Common Issues

1. **Black screen on startup**
   - Check if all required packages are installed
   - Verify GPU drivers are properly configured
   - Check `~/.config/hypr/hyprland.log`

2. **Waybar not showing**
   - Ensure waybar is installed
   - Check if the config files are in the correct location
   - Run waybar manually to see errors

3. **Applications not themed correctly**
   - Set `QT_QPA_PLATFORMTHEME=kde` in your environment
   - Install `qt5ct` and `qt6ct` for Qt configuration
   - Check GTK theme settings

4. **Screenshot tool not working**
   - Ensure `grim` and `slurp` are installed
   - Check if `wl-clipboard` is available
   - Verify the screenshot directory exists

### Debug Mode

Enable debug logging in Hyprland:
```bash
HYPRLAND_LOG_LEVEL=DEBUG Hyprland
```

### Getting Help

- Check Hyprland wiki: https://wiki.hyprland.org
- HorizonOS issues: https://github.com/horizonos/horizonos/issues
- Hyprland Discord: https://discord.gg/hQ9XvMUjjr

## Performance Tuning

### GPU Configuration

For NVIDIA users, add to `/etc/environment`:
```
WLR_NO_HARDWARE_CURSORS=1
LIBVA_DRIVER_NAME=nvidia
GBM_BACKEND=nvidia-drm
__GLX_VENDOR_LIBRARY_NAME=nvidia
```

### Animation Performance

Reduce animations for better performance:
```bash
# Edit ~/.config/hypr/appearance.conf
animations {
    enabled = false  # or reduce animation durations
}
```

### Memory Usage

Monitor with:
```bash
hyprctl clients
hyprctl workspaces
hyprctl monitors
```

## Contributing

To contribute improvements:
1. Test changes thoroughly
2. Maintain KDE compatibility
3. Document new features
4. Submit pull requests to HorizonOS repository

## License

This configuration is part of HorizonOS and follows the project's licensing terms.