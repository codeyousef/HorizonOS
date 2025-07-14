#!/bin/bash
# HorizonOS Desktop Personality Switcher
# Switches between KDE, Windows 11, and macOS modes

set -e

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Configuration directories
HYPR_CONFIG_DIR="$HOME/.config/hypr"
WAYBAR_CONFIG_DIR="$HOME/.config/waybar"
WOFI_CONFIG_DIR="$HOME/.config/wofi"
DUNST_CONFIG_DIR="$HOME/.config/dunst"
GTK3_CONFIG_DIR="$HOME/.config/gtk-3.0"
QT_CONFIG_DIR="$HOME/.config/qt5ct"
HORIZONOS_DIR="/usr/share/horizonos/desktop/hyprland"

# Current mode file
CURRENT_MODE_FILE="$HOME/.config/horizonos/current-mode"

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

# Function to backup current configuration
backup_config() {
    local timestamp=$(date +%Y%m%d_%H%M%S)
    local backup_dir="$HOME/.config/horizonos/backups/$timestamp"
    
    print_info "Creating backup of current configuration..."
    mkdir -p "$backup_dir"
    
    # Backup configurations if they exist
    [ -d "$HYPR_CONFIG_DIR" ] && cp -r "$HYPR_CONFIG_DIR" "$backup_dir/"
    [ -d "$WAYBAR_CONFIG_DIR" ] && cp -r "$WAYBAR_CONFIG_DIR" "$backup_dir/"
    [ -d "$WOFI_CONFIG_DIR" ] && cp -r "$WOFI_CONFIG_DIR" "$backup_dir/"
    [ -d "$DUNST_CONFIG_DIR" ] && cp -r "$DUNST_CONFIG_DIR" "$backup_dir/"
    [ -d "$GTK3_CONFIG_DIR" ] && cp -r "$GTK3_CONFIG_DIR" "$backup_dir/"
    
    print_success "Backup created at: $backup_dir"
}

# Function to create symlinks
create_symlinks() {
    local mode=$1
    
    print_info "Creating symlinks for $mode mode..."
    
    # Remove existing configurations
    rm -rf "$HYPR_CONFIG_DIR"/{*.conf,scripts,wallpapers}
    rm -rf "$WAYBAR_CONFIG_DIR"
    rm -rf "$WOFI_CONFIG_DIR"
    
    # Create directories if they don't exist
    mkdir -p "$HYPR_CONFIG_DIR"
    mkdir -p "$WAYBAR_CONFIG_DIR"
    mkdir -p "$WOFI_CONFIG_DIR"
    
    # Symlink Hyprland configs
    case $mode in
        kde)
            ln -sf "$HORIZONOS_DIR/configs/hyprland.conf" "$HYPR_CONFIG_DIR/hyprland.conf"
            ln -sf "$HORIZONOS_DIR/configs/binds.conf" "$HYPR_CONFIG_DIR/binds.conf"
            ln -sf "$HORIZONOS_DIR/configs/rules.conf" "$HYPR_CONFIG_DIR/rules.conf"
            ln -sf "$HORIZONOS_DIR/configs/exec.conf" "$HYPR_CONFIG_DIR/exec.conf"
            ln -sf "$HORIZONOS_DIR/configs/appearance.conf" "$HYPR_CONFIG_DIR/appearance.conf"
            ln -sf "$HORIZONOS_DIR/waybar/config-kde.json" "$WAYBAR_CONFIG_DIR/config"
            ln -sf "$HORIZONOS_DIR/waybar/style-kde.css" "$WAYBAR_CONFIG_DIR/style.css"
            ln -sf "$HORIZONOS_DIR/wofi/kde" "$WOFI_CONFIG_DIR/"
            ;;
        windows11)
            ln -sf "$HORIZONOS_DIR/configs/windows11/hyprland.conf" "$HYPR_CONFIG_DIR/hyprland.conf"
            ln -sf "$HORIZONOS_DIR/configs/windows11/binds.conf" "$HYPR_CONFIG_DIR/binds.conf"
            ln -sf "$HORIZONOS_DIR/configs/windows11/rules.conf" "$HYPR_CONFIG_DIR/rules.conf"
            ln -sf "$HORIZONOS_DIR/configs/windows11/exec.conf" "$HYPR_CONFIG_DIR/exec.conf"
            ln -sf "$HORIZONOS_DIR/configs/windows11/appearance.conf" "$HYPR_CONFIG_DIR/appearance.conf"
            ln -sf "$HORIZONOS_DIR/waybar/windows11/config.json" "$WAYBAR_CONFIG_DIR/config"
            ln -sf "$HORIZONOS_DIR/waybar/windows11/style.css" "$WAYBAR_CONFIG_DIR/style.css"
            ln -sf "$HORIZONOS_DIR/wofi/windows11" "$WOFI_CONFIG_DIR/"
            ;;
        macos)
            ln -sf "$HORIZONOS_DIR/configs/macos/hyprland.conf" "$HYPR_CONFIG_DIR/hyprland.conf"
            ln -sf "$HORIZONOS_DIR/configs/macos/binds.conf" "$HYPR_CONFIG_DIR/binds.conf"
            ln -sf "$HORIZONOS_DIR/configs/macos/rules.conf" "$HYPR_CONFIG_DIR/rules.conf"
            ln -sf "$HORIZONOS_DIR/configs/macos/exec.conf" "$HYPR_CONFIG_DIR/exec.conf"
            ln -sf "$HORIZONOS_DIR/configs/macos/appearance.conf" "$HYPR_CONFIG_DIR/appearance.conf"
            # macOS has both menubar and dock
            mkdir -p "$WAYBAR_CONFIG_DIR/macos"
            ln -sf "$HORIZONOS_DIR/waybar/macos" "$WAYBAR_CONFIG_DIR/"
            ln -sf "$HORIZONOS_DIR/wofi/macos" "$WOFI_CONFIG_DIR/"
            ;;
    esac
    
    # Symlink common scripts and wallpapers
    ln -sf "$HORIZONOS_DIR/scripts" "$HYPR_CONFIG_DIR/"
    ln -sf "$HORIZONOS_DIR/wallpapers" "$HYPR_CONFIG_DIR/"
    
    # Update notification daemon config
    if [ -f "$HORIZONOS_DIR/dunst/$mode.conf" ]; then
        mkdir -p "$DUNST_CONFIG_DIR"
        ln -sf "$HORIZONOS_DIR/dunst/$mode.conf" "$DUNST_CONFIG_DIR/dunstrc"
    fi
    
    print_success "Symlinks created for $mode mode"
}

# Function to apply GTK/Qt themes
apply_themes() {
    local mode=$1
    
    print_info "Applying $mode themes..."
    
    case $mode in
        kde)
            # Apply Breeze theme
            gsettings set org.gnome.desktop.interface gtk-theme 'Breeze-Dark'
            gsettings set org.gnome.desktop.interface icon-theme 'breeze-dark'
            gsettings set org.gnome.desktop.interface cursor-theme 'breeze_cursors'
            gsettings set org.gnome.desktop.interface font-name 'Noto Sans 10'
            export QT_STYLE_OVERRIDE=breeze
            ;;
        windows11)
            # Apply Windows 11 theme
            gsettings set org.gnome.desktop.interface gtk-theme 'HorizonOS-Windows11'
            gsettings set org.gnome.desktop.interface icon-theme 'Win11'
            gsettings set org.gnome.desktop.interface cursor-theme 'Win11'
            gsettings set org.gnome.desktop.interface font-name 'Segoe UI 10'
            # Copy custom GTK theme
            mkdir -p "$GTK3_CONFIG_DIR"
            cp -r "$HORIZONOS_DIR/themes/windows11/gtk-3.0/"* "$GTK3_CONFIG_DIR/"
            export QT_STYLE_OVERRIDE=kvantum
            ;;
        macos)
            # Apply macOS theme
            gsettings set org.gnome.desktop.interface gtk-theme 'HorizonOS-macOS'
            gsettings set org.gnome.desktop.interface icon-theme 'BigSur'
            gsettings set org.gnome.desktop.interface cursor-theme 'macOS-Monterey'
            gsettings set org.gnome.desktop.interface font-name 'SF Pro Display 11'
            # Copy custom GTK theme
            mkdir -p "$GTK3_CONFIG_DIR"
            cp -r "$HORIZONOS_DIR/themes/macos/gtk-3.0/"* "$GTK3_CONFIG_DIR/"
            export QT_STYLE_OVERRIDE=kvantum
            ;;
    esac
    
    print_success "Themes applied for $mode mode"
}

# Function to restart services
restart_services() {
    print_info "Restarting desktop services..."
    
    # Kill existing services
    pkill waybar || true
    pkill dunst || true
    pkill wlsunset || true
    pkill swayidle || true
    
    # Small delay to ensure processes are killed
    sleep 1
    
    # Reload Hyprland configuration
    hyprctl reload
    
    print_success "Desktop services restarted"
}

# Function to save current mode
save_current_mode() {
    local mode=$1
    mkdir -p "$(dirname "$CURRENT_MODE_FILE")"
    echo "$mode" > "$CURRENT_MODE_FILE"
    print_success "Current mode saved: $mode"
}

# Function to get current mode
get_current_mode() {
    if [ -f "$CURRENT_MODE_FILE" ]; then
        cat "$CURRENT_MODE_FILE"
    else
        echo "unknown"
    fi
}

# Function to show usage
show_usage() {
    echo "HorizonOS Desktop Personality Switcher"
    echo ""
    echo "Usage: $0 [MODE]"
    echo ""
    echo "Available modes:"
    echo "  kde        - KDE Plasma-like interface"
    echo "  windows11  - Windows 11-like interface"
    echo "  macos      - macOS-like interface"
    echo ""
    echo "Options:"
    echo "  --current  - Show current mode"
    echo "  --backup   - Create backup before switching"
    echo "  --help     - Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0 kde           # Switch to KDE mode"
    echo "  $0 windows11     # Switch to Windows 11 mode"
    echo "  $0 macos         # Switch to macOS mode"
    echo "  $0 --current     # Show current mode"
}

# Main script logic
main() {
    case "$1" in
        kde|windows11|macos)
            MODE=$1
            
            # Check if backup flag is set
            if [ "$2" == "--backup" ]; then
                backup_config
            fi
            
            print_info "Switching to $MODE mode..."
            
            # Check if running in Hyprland
            if [ -z "$HYPRLAND_INSTANCE_SIGNATURE" ]; then
                print_warning "Not running in Hyprland. Some features may not work properly."
            fi
            
            # Create symlinks
            create_symlinks "$MODE"
            
            # Apply themes
            apply_themes "$MODE"
            
            # Save current mode
            save_current_mode "$MODE"
            
            # Restart services
            restart_services
            
            print_success "Successfully switched to $MODE mode!"
            print_info "You may need to restart some applications for theme changes to take effect."
            ;;
        --current)
            current_mode=$(get_current_mode)
            echo "Current mode: $current_mode"
            ;;
        --help|-h)
            show_usage
            ;;
        *)
            print_error "Invalid mode: $1"
            echo ""
            show_usage
            exit 1
            ;;
    esac
}

# Check if script is run as root
if [ "$EUID" -eq 0 ]; then
    print_error "This script should not be run as root"
    exit 1
fi

# Check if required directories exist
if [ ! -d "$HORIZONOS_DIR" ]; then
    print_error "HorizonOS desktop files not found at $HORIZONOS_DIR"
    print_info "Please ensure HorizonOS is properly installed"
    exit 1
fi

# Run main function
main "$@"