#!/bin/bash
# HorizonOS Hyprland Desktop Environment Installer
# Installs all desktop personalities (KDE, Windows 11, macOS)

set -e

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Installation directories
INSTALL_PREFIX="/usr/share/horizonos/desktop/hyprland"
SYSTEMD_USER_DIR="/usr/lib/systemd/user"
APPLICATIONS_DIR="/usr/share/applications"
DESKTOP_DIR="/usr/share/wayland-sessions"

# Function to print colored output
print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to check if running as root
check_root() {
    if [ "$EUID" -ne 0 ]; then
        print_error "This installer must be run as root (use sudo)"
        exit 1
    fi
}

# Function to check dependencies
check_dependencies() {
    print_info "Checking dependencies..."
    
    local deps=(
        "hyprland"
        "waybar"
        "wofi"
        "dunst"
        "swaylock"
        "swayidle"
        "wl-clipboard"
        "cliphist"
        "grim"
        "slurp"
        "wf-recorder"
        "pamixer"
        "brightnessctl"
        "playerctl"
        "nm-applet"
        "blueman-applet"
        "thunar"
        "alacritty"
        "polkit-gnome"
        "xdg-desktop-portal-hyprland"
        "qt5-wayland"
        "qt6-wayland"
    )
    
    local missing=()
    
    for dep in "${deps[@]}"; do
        if ! command -v "$dep" &> /dev/null && ! pacman -Qi "$dep" &> /dev/null; then
            missing+=("$dep")
        fi
    done
    
    if [ ${#missing[@]} -ne 0 ]; then
        print_warning "Missing dependencies: ${missing[*]}"
        print_info "Installing missing dependencies..."
        pacman -S --noconfirm "${missing[@]}" || {
            print_error "Failed to install dependencies"
            exit 1
        }
    fi
    
    print_success "All dependencies satisfied"
}

# Function to create directories
create_directories() {
    print_info "Creating installation directories..."
    
    mkdir -p "$INSTALL_PREFIX"/{configs,waybar,wofi,scripts,wallpapers,themes,dunst,services}
    mkdir -p "$INSTALL_PREFIX"/configs/{kde,windows11,macos}
    mkdir -p "$INSTALL_PREFIX"/waybar/{kde,windows11,macos}
    mkdir -p "$INSTALL_PREFIX"/wofi/{kde,windows11,macos}
    mkdir -p "$INSTALL_PREFIX"/themes/{kde,windows11,macos}/{gtk-3.0,qt}
    
    print_success "Directories created"
}

# Function to install files
install_files() {
    print_info "Installing desktop environment files..."
    
    # Copy all configuration files
    cp -r configs/* "$INSTALL_PREFIX/configs/"
    cp -r waybar/* "$INSTALL_PREFIX/waybar/"
    cp -r wofi/* "$INSTALL_PREFIX/wofi/"
    cp -r scripts/* "$INSTALL_PREFIX/scripts/"
    cp -r wallpapers/* "$INSTALL_PREFIX/wallpapers/" 2>/dev/null || true
    cp -r themes/* "$INSTALL_PREFIX/themes/" 2>/dev/null || true
    cp -r services/* "$INSTALL_PREFIX/services/" 2>/dev/null || true
    
    # Make scripts executable
    chmod +x "$INSTALL_PREFIX"/scripts/*.sh
    
    # Install mode switcher to /usr/local/bin
    install -Dm755 scripts/switch-mode.sh /usr/local/bin/horizonos-switch-mode
    
    print_success "Files installed"
}

# Function to install systemd services
install_services() {
    print_info "Installing systemd user services..."
    
    # Install Hyprland session
    cat > "$DESKTOP_DIR/hyprland-horizonos.desktop" << EOF
[Desktop Entry]
Name=Hyprland (HorizonOS)
Comment=HorizonOS Hyprland compositor with multiple personalities
Exec=/usr/local/bin/horizonos-hyprland
Type=Application
EOF
    
    # Create Hyprland wrapper script
    cat > /usr/local/bin/horizonos-hyprland << 'EOF'
#!/bin/bash
# HorizonOS Hyprland wrapper

# Set environment variables
export XDG_CURRENT_DESKTOP=Hyprland
export XDG_SESSION_TYPE=wayland
export XDG_SESSION_DESKTOP=Hyprland
export MOZ_ENABLE_WAYLAND=1
export QT_QPA_PLATFORM=wayland
export SDL_VIDEODRIVER=wayland
export _JAVA_AWT_WM_NONREPARENTING=1

# Check for default mode
DEFAULT_MODE_FILE="$HOME/.config/horizonos/default-mode"
if [ -f "$DEFAULT_MODE_FILE" ]; then
    DEFAULT_MODE=$(cat "$DEFAULT_MODE_FILE")
else
    DEFAULT_MODE="kde"
fi

# Apply default mode if no current mode is set
CURRENT_MODE_FILE="$HOME/.config/horizonos/current-mode"
if [ ! -f "$CURRENT_MODE_FILE" ]; then
    /usr/local/bin/horizonos-switch-mode "$DEFAULT_MODE"
fi

# Start Hyprland
exec hyprland
EOF
    
    chmod +x /usr/local/bin/horizonos-hyprland
    
    # Install systemd user services
    if [ -d "$INSTALL_PREFIX/services" ]; then
        for service in "$INSTALL_PREFIX"/services/*.service; do
            if [ -f "$service" ]; then
                install -Dm644 "$service" "$SYSTEMD_USER_DIR/$(basename "$service")"
            fi
        done
    fi
    
    print_success "Services installed"
}

# Function to install themes
install_themes() {
    print_info "Installing GTK and icon themes..."
    
    # Windows 11 theme dependencies
    if ! pacman -Qi fluent-gtk-theme &> /dev/null; then
        print_info "Installing Windows 11 theme dependencies..."
        yay -S --noconfirm fluent-gtk-theme win11-icon-theme-git || {
            print_warning "Could not install Windows 11 themes from AUR"
        }
    fi
    
    # macOS theme dependencies
    if ! pacman -Qi whitesur-gtk-theme &> /dev/null; then
        print_info "Installing macOS theme dependencies..."
        yay -S --noconfirm whitesur-gtk-theme-git bigsur-icon-theme-git || {
            print_warning "Could not install macOS themes from AUR"
        }
    fi
    
    print_success "Themes installed"
}

# Function to create default configuration
create_default_config() {
    print_info "Creating default configuration..."
    
    # Create config selection script
    cat > /usr/local/bin/horizonos-desktop-setup << 'EOF'
#!/bin/bash
# HorizonOS Desktop Setup

echo "HorizonOS Desktop Environment Setup"
echo "=================================="
echo ""
echo "Please select your preferred desktop personality:"
echo "1) KDE Plasma style (default)"
echo "2) Windows 11 style"
echo "3) macOS style"
echo ""
read -p "Enter your choice (1-3): " choice

case $choice in
    1) mode="kde" ;;
    2) mode="windows11" ;;
    3) mode="macos" ;;
    *) mode="kde" ;;
esac

mkdir -p "$HOME/.config/horizonos"
echo "$mode" > "$HOME/.config/horizonos/default-mode"

echo ""
echo "Setting up $mode mode..."
/usr/local/bin/horizonos-switch-mode "$mode"

echo ""
echo "Setup complete! You can switch modes anytime with:"
echo "  horizonos-switch-mode [kde|windows11|macos]"
EOF
    
    chmod +x /usr/local/bin/horizonos-desktop-setup
    
    print_success "Default configuration created"
}

# Function to show post-installation message
show_post_install() {
    echo ""
    echo "=========================================="
    echo -e "${GREEN}HorizonOS Desktop Environment Installed!${NC}"
    echo "=========================================="
    echo ""
    echo "Available commands:"
    echo "  horizonos-desktop-setup    - Initial setup wizard"
    echo "  horizonos-switch-mode      - Switch desktop personality"
    echo ""
    echo "Desktop personalities:"
    echo "  kde        - KDE Plasma-like interface"
    echo "  windows11  - Windows 11-like interface"
    echo "  macos      - macOS-like interface"
    echo ""
    echo "To get started:"
    echo "1. Log out of your current session"
    echo "2. Select 'Hyprland (HorizonOS)' from your display manager"
    echo "3. Run 'horizonos-desktop-setup' on first login"
    echo ""
    echo "For more information, see:"
    echo "  /usr/share/horizonos/desktop/README.md"
}

# Main installation process
main() {
    print_info "Starting HorizonOS Desktop Environment installation..."
    
    check_root
    check_dependencies
    create_directories
    install_files
    install_services
    install_themes
    create_default_config
    
    # Create README
    cat > "$INSTALL_PREFIX/README.md" << 'EOF'
# HorizonOS Desktop Environment

## Overview
HorizonOS provides a unique Hyprland-based desktop environment with three distinct personalities:
- **KDE Mode**: Familiar KDE Plasma-like interface
- **Windows 11 Mode**: Windows 11-inspired design
- **macOS Mode**: macOS-style dock and menu bar

## Switching Modes
```bash
# Switch to KDE mode
horizonos-switch-mode kde

# Switch to Windows 11 mode
horizonos-switch-mode windows11

# Switch to macOS mode
horizonos-switch-mode macos

# Check current mode
horizonos-switch-mode --current
```

## Key Bindings
Each mode has its own set of keybindings that match the respective desktop environment:
- **KDE Mode**: Uses KDE Plasma keybindings (Meta key for launcher)
- **Windows 11 Mode**: Uses Windows keybindings (Win key for Start menu)
- **macOS Mode**: Uses macOS keybindings (Cmd key for Spotlight)

## Configuration
User configurations are stored in:
- `~/.config/hypr/` - Hyprland configuration
- `~/.config/waybar/` - Status bar configuration
- `~/.config/wofi/` - Application launcher
- `~/.config/horizonos/` - HorizonOS specific settings

## Troubleshooting
If you encounter issues:
1. Check the Hyprland log: `~/.hyprland.log`
2. Restart the desktop: `hyprctl reload`
3. Reset to default: `horizonos-switch-mode kde --backup`

## Support
For support and bug reports, visit:
https://github.com/horizonos/horizonos
EOF
    
    show_post_install
}

# Run main installation
main "$@"