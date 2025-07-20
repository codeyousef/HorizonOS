#!/bin/bash
# validate-iso-config.sh - Run this before building ISO
# Based on Boot Process and Troubleshooting guide

echo "=== HorizonOS ISO Configuration Validator ==="

PROFILE_DIR="${1:-./iso/horizonos-profile}"
ERRORS=0
WARNINGS=0

# Color codes
RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
NC='\033[0m'

error() {
    echo -e "${RED}[ERROR]${NC} $1"
    ((ERRORS++))
}

warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
    ((WARNINGS++))
}

success() {
    echo -e "${GREEN}[OK]${NC} $1"
}

# Check 1: Profile structure
echo -e "\n1. Checking profile structure..."
for dir in airootfs efiboot syslinux grub; do
    if [[ -d "$PROFILE_DIR/$dir" ]]; then
        success "Directory $dir exists"
    else
        error "Missing directory: $dir"
    fi
done

# Check 2: Critical files
echo -e "\n2. Checking critical files..."
if [[ -f "$PROFILE_DIR/packages.x86_64" ]]; then
    success "Package list found"
    # Check for essential packages
    for pkg in base linux linux-firmware mkinitcpio; do
        if grep -q "^$pkg$" "$PROFILE_DIR/packages.x86_64"; then
            success "Essential package '$pkg' present"
        else
            error "Missing essential package: $pkg"
        fi
    done
    
    # Check for xorg-server (not required for text-only)
    if grep -q "^xorg-server$" "$PROFILE_DIR/packages.x86_64"; then
        success "xorg-server found (graphical support)"
    else
        warning "xorg-server not found (text-only mode)"
    fi
else
    error "packages.x86_64 not found"
fi

# Check 3: Display manager configuration
echo -e "\n3. Checking display manager..."
DM_FOUND=0
for dm in sddm lightdm gdm lxdm; do
    if grep -q "^$dm$" "$PROFILE_DIR/packages.x86_64" 2>/dev/null; then
        success "Display manager '$dm' found"
        DM_FOUND=1
        
        # Check for corresponding greeter
        case $dm in
            lightdm)
                if ! grep -qE "^lightdm-(gtk|webkit2|slick)-greeter$" "$PROFILE_DIR/packages.x86_64"; then
                    warning "No greeter found for LightDM"
                fi
                ;;
            sddm)
                if ! grep -q "^qt5-declarative$" "$PROFILE_DIR/packages.x86_64"; then
                    warning "qt5-declarative recommended for SDDM themes"
                fi
                ;;
        esac
    fi
done
if [[ $DM_FOUND -eq 0 ]]; then
    warning "No display manager found - TEXT-ONLY mode"
    echo "  Note: This may cause 'Started Getty on tty1' issues without proper configuration"
fi

# Check 4: Graphics drivers
echo -e "\n4. Checking graphics drivers..."
DRIVERS_FOUND=0
for driver in xf86-video-vesa xf86-video-intel xf86-video-amdgpu xf86-video-nouveau nvidia; do
    if grep -q "^$driver" "$PROFILE_DIR/packages.x86_64" 2>/dev/null; then
        success "Graphics driver '$driver' included"
        ((DRIVERS_FOUND++))
    fi
done
if [[ $DRIVERS_FOUND -eq 0 ]]; then
    warning "No graphics drivers found (expected for text-only)"
elif [[ $DRIVERS_FOUND -lt 3 ]]; then
    warning "Limited graphics driver coverage (only $DRIVERS_FOUND drivers)"
fi

# Check 5: Service configuration
echo -e "\n5. Checking service configuration..."
if [[ -f "$PROFILE_DIR/airootfs/root/customize_airootfs.sh" ]]; then
    success "customize_airootfs.sh found"
    if [[ ! -x "$PROFILE_DIR/airootfs/root/customize_airootfs.sh" ]]; then
        error "customize_airootfs.sh is not executable"
    fi
    
    # Check for common issues in the script
    if grep -q "systemctl enable" "$PROFILE_DIR/airootfs/root/customize_airootfs.sh"; then
        success "Service enablement found"
    else
        warning "No 'systemctl enable' commands found"
    fi
    
    if grep -q "useradd" "$PROFILE_DIR/airootfs/root/customize_airootfs.sh"; then
        success "User creation found"
    else
        warning "No user creation found - using root autologin"
    fi
else
    warning "customize_airootfs.sh not found"
fi

# Check 6: Getty conflict prevention
echo -e "\n6. Checking getty configuration..."
if [[ -d "$PROFILE_DIR/airootfs/etc/systemd/system/getty@tty1.service.d" ]] || 
   [[ -L "$PROFILE_DIR/airootfs/etc/systemd/system/getty@tty1.service" ]]; then
    success "Getty@tty1 configuration found"
    
    # Check for Restart=no in autologin.conf
    AUTOLOGIN_CONF="$PROFILE_DIR/airootfs/etc/systemd/system/getty@tty1.service.d/autologin.conf"
    if [[ -f "$AUTOLOGIN_CONF" ]]; then
        if grep -q "Restart=no" "$AUTOLOGIN_CONF"; then
            success "Getty has Restart=no (prevents loops)"
        else
            error "Getty missing Restart=no (will cause flashing loop!)"
        fi
    fi
else
    warning "No getty@tty1 configuration - may cause display manager conflicts"
fi

# Check 7: Default target
echo -e "\n7. Checking default systemd target..."
if [[ -L "$PROFILE_DIR/airootfs/etc/systemd/system/default.target" ]]; then
    TARGET=$(readlink "$PROFILE_DIR/airootfs/etc/systemd/system/default.target")
    if [[ "$TARGET" == *"graphical.target" ]]; then
        if [[ $DM_FOUND -eq 0 ]]; then
            error "Default target is graphical but no display manager found!"
        else
            success "Default target set to graphical"
        fi
    elif [[ "$TARGET" == *"multi-user.target" ]]; then
        if [[ $DM_FOUND -eq 0 ]]; then
            success "Default target set to multi-user (correct for text-only)"
        else
            warning "Display manager found but default is multi-user"
        fi
    else
        error "Default target is not graphical.target or multi-user.target"
    fi
else
    warning "Default target not explicitly set"
fi

# Check 8: Desktop environment
echo -e "\n8. Checking desktop environment..."
DE_FOUND=0
for de in plasma-desktop gnome xfce4 lxde mate cinnamon; do
    if grep -q "^$de" "$PROFILE_DIR/packages.x86_64" 2>/dev/null; then
        success "Desktop environment '$de' found"
        DE_FOUND=1
    fi
done
if [[ $DE_FOUND -eq 0 ]]; then
    if [[ $DM_FOUND -eq 0 ]]; then
        warning "No desktop environment (expected for text-only)"
    else
        error "Display manager found but no desktop environment!"
    fi
fi

# Check 9: HorizonOS specific
echo -e "\n9. Checking HorizonOS specific components..."
if grep -q "^ostree$" "$PROFILE_DIR/packages.x86_64" 2>/dev/null; then
    success "OSTree package included"
else
    error "OSTree package missing (required for HorizonOS)"
fi

# Summary
echo -e "\n=== Validation Summary ==="
echo "Errors: $ERRORS"
echo "Warnings: $WARNINGS"

if [[ $ERRORS -gt 0 ]]; then
    echo -e "${RED}Build will likely fail or produce non-working ISO${NC}"
    echo -e "${RED}Fix errors before building!${NC}"
    exit 1
else
    if [[ $WARNINGS -gt 5 ]]; then
        echo -e "${YELLOW}Many warnings detected - ISO may have issues${NC}"
    else
        echo -e "${GREEN}Configuration looks good!${NC}"
    fi
    exit 0
fi