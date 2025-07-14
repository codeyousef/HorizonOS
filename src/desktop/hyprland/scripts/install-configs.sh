#!/bin/bash
# HorizonOS Hyprland KDE-style Configuration Installer

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Source and destination directories
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
HYPRLAND_DIR="$(dirname "$SCRIPT_DIR")"
CONFIG_DIR="$HOME/.config"

echo -e "${BLUE}=== HorizonOS Hyprland KDE-Style Configuration Installer ===${NC}"
echo

# Function to create backup
backup_config() {
    local config_path="$1"
    if [ -e "$config_path" ]; then
        local backup_path="${config_path}.backup.$(date +%Y%m%d_%H%M%S)"
        echo -e "${YELLOW}Backing up existing configuration...${NC}"
        mv "$config_path" "$backup_path"
        echo -e "${GREEN}✓ Backed up to: $backup_path${NC}"
    fi
}

# Function to install configuration
install_config() {
    local src="$1"
    local dst="$2"
    local config_name="$3"
    
    echo -e "${BLUE}Installing $config_name...${NC}"
    
    # Create parent directory if needed
    mkdir -p "$(dirname "$dst")"
    
    # Backup existing configuration
    backup_config "$dst"
    
    # Copy configuration
    if [ -d "$src" ]; then
        cp -r "$src" "$dst"
    else
        cp "$src" "$dst"
    fi
    
    echo -e "${GREEN}✓ Installed $config_name${NC}"
    echo
}

# Install Hyprland configuration
echo -e "${BLUE}1. Installing Hyprland configuration...${NC}"
mkdir -p "$CONFIG_DIR/hypr"

# Copy main configs
for config in hyprland.conf binds.conf rules.conf exec.conf appearance.conf; do
    install_config "$HYPRLAND_DIR/configs/$config" "$CONFIG_DIR/hypr/$config" "$config"
done

# Copy scripts
echo -e "${BLUE}2. Installing helper scripts...${NC}"
mkdir -p "$CONFIG_DIR/hypr/scripts"
for script in "$HYPRLAND_DIR/scripts"/*.sh; do
    if [ -f "$script" ]; then
        script_name=$(basename "$script")
        install_config "$script" "$CONFIG_DIR/hypr/scripts/$script_name" "$script_name"
        chmod +x "$CONFIG_DIR/hypr/scripts/$script_name"
    fi
done

# Install Waybar configuration
echo -e "${BLUE}3. Installing Waybar configuration...${NC}"
mkdir -p "$CONFIG_DIR/waybar"
install_config "$HYPRLAND_DIR/waybar/config-kde.json" "$CONFIG_DIR/waybar/config-kde.json" "Waybar config"
install_config "$HYPRLAND_DIR/waybar/style-kde.css" "$CONFIG_DIR/waybar/style-kde.css" "Waybar style"

# Install Wofi configuration
echo -e "${BLUE}4. Installing Wofi configuration...${NC}"
mkdir -p "$CONFIG_DIR/wofi"
install_config "$HYPRLAND_DIR/wofi/config" "$CONFIG_DIR/wofi/config" "Wofi config"
install_config "$HYPRLAND_DIR/wofi/style.css" "$CONFIG_DIR/wofi/style.css" "Wofi style"

# Create necessary directories
echo -e "${BLUE}5. Creating necessary directories...${NC}"
mkdir -p "$HOME/Pictures/Screenshots"
mkdir -p "$CONFIG_DIR/hypr/wallpapers"
echo -e "${GREEN}✓ Created directories${NC}"
echo

# Set up environment
echo -e "${BLUE}6. Setting up environment...${NC}"

# Create .profile entries if not exists
PROFILE_ENTRIES="
# HorizonOS Hyprland Environment
export QT_QPA_PLATFORMTHEME=kde
export QT_STYLE_OVERRIDE=breeze
export GTK_THEME=Breeze-Dark
export XCURSOR_THEME=Breeze_Snow
export XCURSOR_SIZE=24
"

if ! grep -q "HorizonOS Hyprland Environment" "$HOME/.profile" 2>/dev/null; then
    echo "$PROFILE_ENTRIES" >> "$HOME/.profile"
    echo -e "${GREEN}✓ Added environment variables to .profile${NC}"
else
    echo -e "${YELLOW}! Environment variables already configured${NC}"
fi

# Download default wallpaper if not exists
echo -e "${BLUE}7. Setting up wallpaper...${NC}"
WALLPAPER_PATH="$CONFIG_DIR/hypr/wallpapers/breeze-default.jpg"
if [ ! -f "$WALLPAPER_PATH" ]; then
    # Use a placeholder wallpaper command
    echo -e "${YELLOW}! Please add your preferred wallpaper to: $WALLPAPER_PATH${NC}"
    # Create a simple gradient wallpaper as placeholder
    convert -size 1920x1080 gradient:'#31363b'-'#232629' "$WALLPAPER_PATH" 2>/dev/null || \
    echo -e "${YELLOW}! ImageMagick not installed, skipping wallpaper generation${NC}"
else
    echo -e "${GREEN}✓ Wallpaper already exists${NC}"
fi

echo
echo -e "${GREEN}=== Installation Complete! ===${NC}"
echo
echo -e "${BLUE}To start Hyprland with KDE-style configuration:${NC}"
echo "  1. Log out of your current session"
echo "  2. Select Hyprland from your display manager"
echo "  3. Or run: Hyprland"
echo
echo -e "${BLUE}Configuration files installed to:${NC}"
echo "  - Hyprland: $CONFIG_DIR/hypr/"
echo "  - Waybar: $CONFIG_DIR/waybar/"
echo "  - Wofi: $CONFIG_DIR/wofi/"
echo
echo -e "${YELLOW}Note: Make sure you have all required packages installed:${NC}"
echo "  sudo pacman -S hyprland waybar wofi dolphin konsole kate \\"
echo "                 breeze breeze-gtk papirus-icon-theme \\"
echo "                 ttf-noto-fonts ttf-font-awesome \\"
echo "                 polkit-kde-agent xdg-desktop-portal-hyprland \\"
echo "                 qt5-wayland qt6-wayland wl-clipboard \\"
echo "                 grim slurp dunst brightnessctl pamixer"
echo