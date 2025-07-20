# Complete Report: Solving Arch-Based Live Desktop Environment Boot Issues

## Executive Summary

Creating a bootable live environment for a custom Arch-based distro requires careful attention to several critical components: kernel modules, initramfs configuration, graphics drivers, display managers, and the squashfs implementation. The most common issues preventing boot to desktop are missing kernel modules, incorrect mkinitcpio hooks, graphics driver conflicts, and display manager misconfiguration.

**Quick Fix for "Started Getty on tty1" Issue:** If your live environment gets stuck at this message, it means the display manager isn't starting. Jump to [Step 5](https://claude.ai/chat/2b595ca1-bb78-4896-a851-2270d9cc1d9f#step5-fix) in the troubleshooting section for immediate solutions.

## Table of Contents

1. [Common Boot Issues and Root Causes](https://claude.ai/chat/2b595ca1-bb78-4896-a851-2270d9cc1d9f#common-boot-issues)
2. [Essential Components for Live Environment](https://claude.ai/chat/2b595ca1-bb78-4896-a851-2270d9cc1d9f#essential-components)
3. [Best Practices from Popular Arch-Based Distros](https://claude.ai/chat/2b595ca1-bb78-4896-a851-2270d9cc1d9f#best-practices)
4. [Detailed Troubleshooting Steps](https://claude.ai/chat/2b595ca1-bb78-4896-a851-2270d9cc1d9f#troubleshooting-steps)
5. [Implementation Guide](https://claude.ai/chat/2b595ca1-bb78-4896-a851-2270d9cc1d9f#implementation-guide)
6. [Testing and Validation](https://claude.ai/chat/2b595ca1-bb78-4896-a851-2270d9cc1d9f#testing-validation)

## Common Boot Issues and Root Causes {#common-boot-issues}

### 1. Black Screen After Boot

**Symptoms:**

- System boots but shows only a black screen
- No display manager appears
- Can't access TTY with Ctrl+Alt+F1-F6

**Common Causes:**

- Missing or conflicting graphics drivers
- Kernel Mode Setting (KMS) issues
- Display manager failing to start
- Incorrect framebuffer configuration

### 2. "Can't Access TTY: Job Control Turned Off"

**Causes:**

- Missing essential kernel modules in initramfs
- Corrupted initramfs image
- Incorrect root device specification

### 3. Display Manager Failures

**Common Error Messages:**

- "Failed to start Light Display Manager"
- "Reached target graphical interface" but no GUI
- Display manager crashes in a loop

### 4. Graphics Driver Conflicts

**Symptoms:**

- Need to use `nomodeset` to boot
- X server fails to detect graphics hardware
- Module loading errors in journal

### 5. Stuck at "Started Getty on tty1"

**This is a very common issue in live environments**

**Symptoms:**

- System boots successfully but stops at "Started Getty on tty1"
- No graphical interface appears
- Can switch to other TTYs (Ctrl+Alt+F2-F6) but tty1 shows only text prompt

**Root Causes:**

- Display manager service not starting properly
- Conflict between getty@tty1.service and display manager
- Missing or misconfigured autologin user
- Display manager trying to use same TTY as getty
- Service ordering issues in systemd

## Essential Components for Live Environment {#essential-components}

### 1. Kernel Modules (mkinitcpio.conf)

```bash
# Essential modules for live environment
MODULES=(squashfs loop overlay aufs vfat)

# Graphics modules (add as needed)
MODULES+=(i915 amdgpu radeon nouveau nvidia nvidia_modeset nvidia_uvm nvidia_drm)

# Additional filesystem support
MODULES+=(ext4 btrfs xfs ntfs3)
```

### 2. Required Hooks

```bash
# Minimal working configuration
HOOKS=(base udev autodetect modconf block filesystems keyboard keymap)

# For live environment with archiso
HOOKS=(base udev archiso block filesystems)

# Custom live environment
HOOKS=(base udev autodetect modconf block filesystems keyboard keymap consolefont)
```

### 3. SquashFS Configuration

- Use compression: `lz4hc` for speed or `xz -9e` for size
- Ensure squashfs-tools is installed
- Proper permissions on squashfs images

### 4. Display Manager Requirements

**SDDM (Recommended for Qt-based environments):**

```bash
pacman -S sddm qt5-declarative
systemctl enable sddm.service
```

**LightDM (Lightweight alternative):**

```bash
pacman -S lightdm lightdm-gtk-greeter
systemctl enable lightdm.service
```

**GDM (For GNOME):**

```bash
pacman -S gdm
systemctl enable gdm.service
```

## Best Practices from Popular Arch-Based Distros {#best-practices}

### 1. Manjaro's Approach

- Uses **mhwd** (Manjaro Hardware Detection) for automatic driver configuration
- Implements proprietary driver detection and installation
- Uses Calamares installer with live environment detection

### 2. EndeavourOS Implementation

- Minimal approach with essential packages only
- Uses **archiso** as base with custom configurations
- Implements welcome app for post-boot configuration

### 3. Garuda Linux Method

- Heavy use of **dracut** instead of mkinitcpio (some editions)
- Automatic driver installation based on hardware detection
- Performance-oriented kernel configurations

### 4. ArcoLinux Strategy

- Multiple ISO variants for different hardware
- Extensive testing scripts
- Fallback configurations for problematic hardware

## Detailed Troubleshooting Steps {#troubleshooting-steps}

### Step 1: Boot with Fallback Options

```bash
# At boot, edit kernel parameters:
nomodeset          # Disable kernel mode setting
systemd.unit=multi-user.target  # Boot to console only
loglevel=7         # Verbose logging
```

### Step 2: Check System Logs

```bash
# After booting to console:
journalctl -b -p err    # Show errors from current boot
journalctl -xe          # Show recent entries with explanations
systemctl status display-manager.service
```

### Step 3: Verify Graphics Configuration

```bash
# Check loaded modules
lsmod | grep -E "nvidia|nouveau|radeon|amdgpu|i915"

# Check for X errors
cat /var/log/Xorg.0.log | grep -E "(EE)|(WW)"

# Verify KMS is working
dmesg | grep -i "drm\|modeset"
```

### Step 4: Test Display Manager Manually

```bash
# Stop current display manager
systemctl stop display-manager

# Test SDDM
sddm-greeter-qt6 --test-mode --theme /usr/share/sddm/themes/breeze

# Test LightDM
lightdm --test-mode --debug

# Test GDM
gdm --fatal-warnings
```

### Step 5: Fix "Started Getty on tty1" Issue {#step5-fix}

**Quick Diagnostic:** When stuck at this message, press `Ctrl+Alt+F2` to switch to tty2 and login as root (no password in most live environments). Then run:

```bash
systemctl status display-manager.service
systemctl --failed
journalctl -xe | grep -E "(sddm|lightdm|gdm|display-manager)"
```

This common issue occurs when the display manager fails to start properly. Here are specific fixes:

#### Option 1: Disable getty@tty1 conflict

```bash
# Create a drop-in to prevent conflict
mkdir -p ~/customiso/airootfs/etc/systemd/system/getty@tty1.service.d/
cat > ~/customiso/airootfs/etc/systemd/system/getty@tty1.service.d/override.conf << 'EOF'
[Unit]
# Prevent getty from conflicting with display manager
Conflicts=
EOF
```

#### Option 2: Configure display manager to use different TTY

```bash
# For SDDM - use tty7
cat > ~/customiso/airootfs/etc/sddm.conf.d/tty.conf << 'EOF'
[X11]
MinimumVT=7
EOF

# For LightDM
cat > ~/customiso/airootfs/etc/lightdm/lightdm.conf.d/50-tty.conf << 'EOF'
[LightDM]
minimum-vt=7
EOF
```

#### Option 3: Proper service ordering

```bash
# Ensure display manager starts after getty
mkdir -p ~/customiso/airootfs/etc/systemd/system/display-manager.service.d/
cat > ~/customiso/airootfs/etc/systemd/system/display-manager.service.d/override.conf << 'EOF'
[Unit]
After=getty@tty1.service
Wants=getty@tty1.service
EOF
```

#### Option 4: Create autologin configuration

```bash
# For live environment with automatic graphical login
mkdir -p ~/customiso/airootfs/etc/systemd/system/getty@tty1.service.d/
cat > ~/customiso/airootfs/etc/systemd/system/getty@tty1.service.d/autologin.conf << 'EOF'
[Service]
ExecStart=
ExecStart=-/sbin/agetty --autologin liveuser --noclear %I 38400 linux
EOF

# Also ensure the display manager autologin is configured
# For SDDM:
cat > ~/customiso/airootfs/etc/sddm.conf.d/autologin.conf << 'EOF'
[Autologin]
User=liveuser
Session=plasma.desktop
Relogin=false
EOF
```

## Implementation Guide {#implementation-guide}

### 1. Creating the SquashFS Root Image

```bash
# Prepare the root filesystem
mkdir -p ~/livecd/airootfs

# Install base system
pacstrap ~/livecd/airootfs base linux linux-firmware

# Install desktop environment and display manager
pacstrap ~/livecd/airootfs xorg-server xorg-xinit
pacstrap ~/livecd/airootfs plasma-desktop sddm  # For KDE
# OR
pacstrap ~/livecd/airootfs gnome gdm            # For GNOME

# Configure the system
arch-chroot ~/livecd/airootfs

# Inside chroot:
# Set locale, timezone, etc.
# Enable display manager
systemctl enable sddm  # or gdm/lightdm

# Exit chroot
exit

# Create squashfs image
mksquashfs ~/livecd/airootfs ~/livecd/airootfs.sfs \
    -noappend -comp lz4hc -Xhc
```

### 2. Configuring mkinitcpio for Live Boot

```bash
# /etc/mkinitcpio-live.conf
MODULES=(squashfs loop overlay)
BINARIES=()
FILES=()
HOOKS=(base udev autodetect modconf block filesystems keyboard)

# For archiso-based systems
HOOKS=(base udev archiso block filesystems)
```

### 3. Creating Custom Hooks

```bash
# /etc/initcpio/hooks/liveroot
#!/usr/bin/ash

run_hook() {
    # Mount squashfs
    mkdir -p /run/archiso/sfs
    mount -t squashfs -o loop,ro /dev/disk/by-label/ARCH_LIVE /run/archiso/sfs/airootfs.sfs
    
    # Setup overlay
    mkdir -p /run/archiso/cowspace
    mount -t tmpfs -o size=75% tmpfs /run/archiso/cowspace
    
    mkdir -p /run/archiso/cowspace/{upper,work}
    mount -t overlay -o lowerdir=/run/archiso/sfs,upperdir=/run/archiso/cowspace/upper,workdir=/run/archiso/cowspace/work overlay /new_root
}

# /etc/initcpio/install/liveroot
#!/bin/bash

build() {
    add_module overlay
    add_module loop
    add_module squashfs
    add_runscript
}

help() {
    cat <<HELPEOF
Enables booting from squashfs with overlay for live environment
HELPEOF
}
```

### 4. Bootloader Configuration

```bash
# For GRUB
menuentry "Arch Linux Live" {
    set gfxpayload=keep
    linux /boot/vmlinuz-linux archisolabel=ARCH_LIVE
    initrd /boot/initramfs-linux.img
}

# For systemd-boot
title   Arch Linux Live
linux   /vmlinuz-linux
initrd  /initramfs-linux.img
options archisolabel=ARCH_LIVE rw
```

### 5. Graphics Driver Handling

```bash
#!/bin/bash
# Auto-detect and load appropriate graphics drivers

detect_gpu() {
    if lspci | grep -i nvidia > /dev/null; then
        echo "nvidia"
    elif lspci | grep -i amd > /dev/null; then
        echo "amd"
    elif lspci | grep -i intel > /dev/null; then
        echo "intel"
    fi
}

load_drivers() {
    GPU=$(detect_gpu)
    case $GPU in
        nvidia)
            modprobe nvidia
            modprobe nvidia_modeset
            modprobe nvidia_drm
            ;;
        amd)
            modprobe amdgpu
            ;;
        intel)
            modprobe i915
            ;;
    esac
}

# Run detection
load_drivers
```

## Testing and Validation {#testing-validation}

### 1. Pre-Build Validation

Create a validation script to check your configuration before building:

```bash
#!/bin/bash
# validate-config.sh - Run this before building ISO

echo "=== Archiso Configuration Validator ==="

PROFILE_DIR="${1:-./customiso}"
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
    for pkg in base linux linux-firmware mkinitcpio xorg-server; do
        if grep -q "^$pkg$" "$PROFILE_DIR/packages.x86_64"; then
            success "Essential package '$pkg' present"
        else
            error "Missing essential package: $pkg"
        fi
    done
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
    error "No display manager found in packages"
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
    error "No graphics drivers found"
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
        error "No user creation found - will cause autologin issues"
    fi
else
    error "customize_airootfs.sh not found"
fi

# Check 6: Getty conflict prevention
echo -e "\n6. Checking getty configuration..."
if [[ -d "$PROFILE_DIR/airootfs/etc/systemd/system/getty@tty1.service.d" ]] || 
   [[ -L "$PROFILE_DIR/airootfs/etc/systemd/system/getty@tty1.service" ]]; then
    success "Getty@tty1 configuration found"
else
    warning "No getty@tty1 configuration - may cause display manager conflicts"
fi

# Check 7: Default target
echo -e "\n7. Checking default systemd target..."
if [[ -L "$PROFILE_DIR/airootfs/etc/systemd/system/default.target" ]]; then
    TARGET=$(readlink "$PROFILE_DIR/airootfs/etc/systemd/system/default.target")
    if [[ "$TARGET" == *"graphical.target" ]]; then
        success "Default target set to graphical"
    else
        error "Default target is not graphical.target"
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
    error "No desktop environment found"
fi

# Summary
echo -e "\n=== Validation Summary ==="
echo "Errors: $ERRORS"
echo "Warnings: $WARNINGS"

if [[ $ERRORS -gt 0 ]]; then
    echo -e "${RED}Build will likely fail or produce non-working ISO${NC}"
    exit 1
else
    echo -e "${GREEN}Configuration looks good!${NC}"
    exit 0
fi
```

### 2. Build-Time Testing

Add this to your build script to catch errors during build:

```bash
#!/bin/bash
# build-with-checks.sh

set -euo pipefail  # Exit on error

PROFILE_DIR="./customiso"
WORK_DIR="/tmp/archiso-tmp"
OUT_DIR="./out"

echo "=== Starting ISO build with validation ==="

# Pre-build validation
if ! ./validate-config.sh "$PROFILE_DIR"; then
    echo "Pre-build validation failed. Fix errors before building."
    exit 1
fi

# Build ISO with verbose output
echo -e "\n=== Building ISO ==="
if sudo mkarchiso -v -w "$WORK_DIR" "$PROFILE_DIR" 2>&1 | tee build.log; then
    echo -e "\n${GREEN}Build completed successfully${NC}"
else
    echo -e "\n${RED}Build failed! Check build.log for details${NC}"
    exit 1
fi

# Post-build validation
ISO_FILE=$(find "$OUT_DIR" -name "*.iso" -type f | head -1)
if [[ -f "$ISO_FILE" ]]; then
    echo -e "\n=== Post-build validation ==="
    ISO_SIZE=$(du -h "$ISO_FILE" | cut -f1)
    echo "ISO created: $ISO_FILE"
    echo "ISO size: $ISO_SIZE"
    
    # Check ISO size
    SIZE_MB=$(du -m "$ISO_FILE" | cut -f1)
    if [[ $SIZE_MB -lt 500 ]]; then
        echo -e "${YELLOW}Warning: ISO seems too small ($SIZE_MB MB)${NC}"
    elif [[ $SIZE_MB -gt 4000 ]]; then
        echo -e "${YELLOW}Warning: ISO is large ($SIZE_MB MB), may not fit on DVD${NC}"
    fi
else
    echo -e "${RED}ISO file not found!${NC}"
    exit 1
fi
```

### 3. Virtual Machine Testing Script

Automated VM testing to catch runtime issues:

```bash
#!/bin/bash
# test-iso-vm.sh

ISO_FILE="${1:-./out/*.iso}"
ISO_FILE=$(ls $ISO_FILE | head -1)

if [[ ! -f "$ISO_FILE" ]]; then
    echo "ISO file not found: $ISO_FILE"
    exit 1
fi

echo "=== Testing ISO in QEMU ==="

# Test function
test_vm() {
    local NAME=$1
    local EXTRA_ARGS=$2
    local TIMEOUT=${3:-120}
    
    echo -e "\nTest: $NAME"
    echo "Starting VM with: $EXTRA_ARGS"
    
    # Start QEMU in background
    qemu-system-x86_64 \
        -m 2G \
        -cdrom "$ISO_FILE" \
        -boot d \
        -vga virtio \
        -display none \
        -serial mon:stdio \
        -monitor none \
        $EXTRA_ARGS \
        > vm_output_$NAME.log 2>&1 &
    
    QEMU_PID=$!
    
    # Wait for boot and check for common issues
    SECONDS=0
    while [[ $SECONDS -lt $TIMEOUT ]]; do
        if grep -q "Started Getty on tty1" vm_output_$NAME.log; then
            echo "✓ System reached getty"
            
            # Check if it's stuck there
            sleep 10
            if tail -5 vm_output_$NAME.log | grep -q "Started Getty on tty1"; then
                echo "✗ Stuck at getty - display manager not starting"
                kill $QEMU_PID 2>/dev/null
                return 1
            fi
        fi
        
        if grep -qE "(Started|Reached target) Graphical Interface" vm_output_$NAME.log; then
            echo "✓ Reached graphical target"
            kill $QEMU_PID 2>/dev/null
            return 0
        fi
        
        if grep -q "Failed to start" vm_output_$NAME.log; then
            echo "✗ Service failures detected:"
            grep "Failed to start" vm_output_$NAME.log | tail -5
            kill $QEMU_PID 2>/dev/null
            return 1
        fi
        
        sleep 2
    done
    
    echo "✗ Timeout waiting for graphical interface"
    kill $QEMU_PID 2>/dev/null
    return 1
}

# Run tests
FAILED=0

# Test 1: Basic boot
if ! test_vm "basic" ""; then
    ((FAILED++))
fi

# Test 2: UEFI boot
if ! test_vm "uefi" "-bios /usr/share/ovmf/OVMF.fd"; then
    ((FAILED++))
fi

# Test 3: Low memory
if ! test_vm "lowmem" "-m 1G" 180; then
    ((FAILED++))
fi

# Test 4: No KMS
if ! test_vm "nokms" "-append nomodeset"; then
    ((FAILED++))
fi

echo -e "\n=== Test Summary ==="
echo "Failed tests: $FAILED"

if [[ $FAILED -gt 0 ]]; then
    echo "Some tests failed. Check vm_output_*.log files for details."
    exit 1
else
    echo "All tests passed!"
fi
```

### 4. Live Testing Checklist

Create a checklist script for manual testing:

````bash
#!/bin/bash
# generate-test-checklist.sh

cat > live-test-checklist.md << 'EOF'
# Live ISO Testing Checklist

## Boot Testing
- [ ] **BIOS/Legacy Boot**: System boots to desktop
- [ ] **UEFI Boot**: System boots to desktop
- [ ] **Secure Boot**: System boots with secure boot enabled
- [ ] **Different USB Ports**: Test USB 2.0 and 3.0 ports

## Graphics Testing
- [ ] **Intel Graphics**: Desktop loads properly
- [ ] **AMD Graphics**: Desktop loads properly
- [ ] **NVIDIA Graphics**: Desktop loads properly
- [ ] **VirtualBox**: Desktop works in VM
- [ ] **VMware**: Desktop works in VM
- [ ] **QEMU/KVM**: Desktop works in VM

## Display Manager Testing
- [ ] **No Getty Hang**: System doesn't stop at "Started Getty on tty1"
- [ ] **Auto-login Works**: Logs in automatically to live user
- [ ] **Manual Login**: Can login if auto-login disabled
- [ ] **TTY Switching**: Ctrl+Alt+F1-F6 works properly
- [ ] **Logout/Login**: Can logout and login again

## Desktop Environment Testing
- [ ] **Desktop Loads**: Full DE appears after boot
- [ ] **Terminal Works**: Can open terminal emulator
- [ ] **Applications Launch**: Basic apps start properly
- [ ] **System Tray**: Shows properly (network, sound, etc.)
- [ ] **Window Manager**: Windows can be moved/resized

## Hardware Testing
- [ ] **Keyboard**: All keys work
- [ ] **Mouse/Touchpad**: Cursor moves, clicks work
- [ ] **Sound**: Audio plays through speakers
- [ ] **Network**: Ethernet detected and works
- [ ] **WiFi**: Wireless networks visible
- [ ] **USB Storage**: Can mount USB drives

## System Testing
- [ ] **RAM Usage**: Check with `free -h` (should be reasonable)
- [ ] **CPU Usage**: Check with `top` (should be low when idle)
- [ ] **Disk Space**: Check with `df -h` (enough free space)
- [ ] **Services**: Check `systemctl --failed` (no failures)
- [ ] **Logs**: Check `journalctl -p err` (no critical errors)

## Common Issue Checks
Run these commands on the live system:

```bash
# Check for service failures
systemctl --failed

# Check display manager status
systemctl status display-manager

# Check graphics driver
lspci -k | grep -A2 VGA

# Check for errors
journalctl -b -p err | grep -v audit

# Check autologin user
id liveuser
ls -la /home/liveuser

# Check default target
systemctl get-default

# Check X server
ps aux | grep -E "(Xorg|Xwayland)"
````

## Performance Benchmarks

- [ ] **Boot Time**: Under 60 seconds to desktop
- [ ] **Application Launch**: Firefox opens in < 5 seconds
- [ ] **Memory Usage**: Under 1GB at idle
- [ ] **Shutdown Time**: Clean shutdown in < 30 seconds

## Notes

_Record any issues or observations here:_

---

---

---

Tested by: _________________ Date: ___________ EOF

echo "Checklist generated: live-test-checklist.md"

````

### 5. Automated Error Detection Script

Add this to your live environment for quick diagnostics:

```bash
#!/bin/bash
# /usr/local/bin/live-diagnostic
# Include this in your ISO for quick troubleshooting

echo "=== Live Environment Diagnostic Tool ==="
echo "Running automated checks..."

ERRORS=0
WARNINGS=0

# Function to check and report
check() {
    local TEST_NAME=$1
    local COMMAND=$2
    local EXPECTED=$3
    
    echo -n "Checking $TEST_NAME... "
    
    if eval "$COMMAND"; then
        echo "✓ OK"
    else
        echo "✗ FAILED"
        ((ERRORS++))
    fi
}

# System checks
check "Default target" "systemctl get-default | grep -q graphical"
check "Display manager running" "systemctl is-active display-manager.service -q"
check "No failed services" "! systemctl --failed | grep -q failed"
check "Live user exists" "id liveuser &>/dev/null"
check "Home directory exists" "test -d /home/liveuser"
check "Graphics driver loaded" "lsmod | grep -qE '(i915|amdgpu|nouveau|nvidia)'"
check "X server running" "pgrep -x Xorg &>/dev/null"
check "Network manager running" "systemctl is-active NetworkManager -q"

# Getty conflict check
echo -n "Checking getty/DM conflict... "
if systemctl is-active getty@tty1 -q && systemctl is-active display-manager -q; then
    echo "⚠ WARNING: Both services active"
    ((WARNINGS++))
else
    echo "✓ OK"
fi

# Memory check
echo -n "Checking available memory... "
AVAILABLE_MB=$(free -m | awk '/^Mem:/ {print $7}')
if [[ $AVAILABLE_MB -lt 500 ]]; then
    echo "✗ Low memory: ${AVAILABLE_MB}MB"
    ((ERRORS++))
else
    echo "✓ OK: ${AVAILABLE_MB}MB available"
fi

# Display check
echo -n "Checking display configuration... "
if xrandr &>/dev/null; then
    echo "✓ OK"
else
    echo "✗ No display found"
    ((ERRORS++))
fi

echo -e "\n=== Summary ==="
echo "Errors: $ERRORS"
echo "Warnings: $WARNINGS"

if [[ $ERRORS -gt 0 ]]; then
    echo -e "\nRun 'journalctl -xe' for detailed error logs"
    echo "Run 'systemctl status display-manager' to check display manager"
fi
````

### 6. Continuous Integration Testing

For automated builds, create a CI testing script:

```bash
#!/bin/bash
# ci-test.sh - For automated testing in CI/CD

set -euo pipefail

echo "=== CI Testing for Arch Live ISO ==="

# 1. Validate configuration
./validate-config.sh || exit 1

# 2. Build ISO
./build-with-checks.sh || exit 1

# 3. Run VM tests
./test-iso-vm.sh || exit 1

# 4. Extract and verify ISO contents
ISO_FILE=$(find ./out -name "*.iso" -type f | head -1)
mkdir -p /tmp/iso-mount
sudo mount -o loop "$ISO_FILE" /tmp/iso-mount

# Verify critical files exist
for file in /boot/vmlinuz* /boot/initramfs* /arch/x86_64/airootfs.sfs; do
    if [[ ! -f "/tmp/iso-mount$file" ]]; then
        echo "Missing critical file: $file"
        sudo umount /tmp/iso-mount
        exit 1
    fi
done

sudo umount /tmp/iso-mount

echo "=== All CI tests passed! ==="
```

## Quick Reference Summary

### Most Critical Points to Avoid "Started Getty on tty1"

1. **Always set default target to graphical**:
    
    ```bash
    ln -sf /usr/lib/systemd/system/graphical.target \
        airootfs/etc/systemd/system/default.target
    ```
    
2. **Mask or configure getty@tty1**:
    
    ```bash
    # Either mask it completely:
    ln -s /dev/null airootfs/etc/systemd/system/getty@tty1.service
    
    # OR configure it for autologin:
    mkdir -p airootfs/etc/systemd/system/getty@tty1.service.d/
    echo -e "[Service]\nExecStart=\nExecStart=-/sbin/agetty --autologin liveuser --noclear %I 38400 linux" \
        > airootfs/etc/systemd/system/getty@tty1.service.d/autologin.conf
    ```
    
3. **Properly enable display manager**:
    
    ```bash
    # Create proper symlinks
    mkdir -p airootfs/etc/systemd/system/graphical.target.wants
    ln -s /usr/lib/systemd/system/sddm.service \
        airootfs/etc/systemd/system/graphical.target.wants/
    ```
    
4. **Create and configure live user**:
    
    ```bash
    # In customize_airootfs.sh
    useradd -m -G wheel -s /bin/bash liveuser
    echo "liveuser:live" | chpasswd
    systemctl set-default graphical.target
    ```
    
5. **Include all necessary packages**:
    
    - Display server: `xorg-server` or `wayland`
    - Display manager: `sddm`, `lightdm`, or `gdm`
    - Graphics drivers: `xf86-video-*` packages
    - Desktop environment: Complete DE packages

### Emergency Recovery

If your ISO gets stuck at "Started Getty on tty1":

1. Press `Ctrl+Alt+F2` to switch to tty2
2. Login as root (usually no password)
3. Run these diagnostic commands:
    
    ```bash
    systemctl start sddm  # Or your display managersystemctl status display-managerjournalctl -xe | grep -i error
    ```
    

### Testing Commands Before Deployment

Always run these checks:

```bash
# Pre-build validation
./validate-config.sh

# Automated VM testing
./test-iso-vm.sh

# On live system
systemctl --failed
systemctl status display-manager
journalctl -b -p err
```

Remember: The "Started Getty on tty1" issue is almost always a display manager configuration problem, not a boot failure!

## Using Archiso for Custom Live Environment

### 1. Setting Up Archiso Profile

```bash
# Install archiso
pacman -S archiso

# Copy releng profile as base
cp -r /usr/share/archiso/configs/releng ~/customiso

# Directory structure
~/customiso/
├── airootfs/          # Overlay filesystem (becomes /)
├── efiboot/           # EFI boot files
├── grub/              # GRUB configuration
├── syslinux/          # Syslinux configuration
├── packages.x86_64    # Package list
├── pacman.conf        # Pacman configuration
└── profiledef.sh      # Profile definition
```

### 2. Customizing for Desktop Environment

Edit `packages.x86_64` to add desktop packages:

```bash
# Base system
base
base-devel
linux
linux-firmware
mkinitcpio
mkinitcpio-archiso
openssh
syslinux

# Networking
networkmanager
network-manager-applet
wireless_tools
wpa_supplicant

# Graphics drivers
xf86-video-vesa
xf86-video-intel
xf86-video-amdgpu
xf86-video-nouveau
nvidia
nvidia-utils

# Display server
xorg-server
xorg-xinit
xorg-xrandr

# Display manager
sddm
# or lightdm lightdm-gtk-greeter
# or gdm

# Desktop environment (choose one)
## KDE Plasma
plasma-desktop
konsole
dolphin
kate

## XFCE
xfce4
xfce4-goodies
xfce4-terminal

## GNOME
gnome
gnome-extra
```

### 3. Enabling Services

Create systemd service links in `airootfs`:

```bash
# Create service directories
mkdir -p ~/customiso/airootfs/etc/systemd/system/multi-user.target.wants
mkdir -p ~/customiso/airootfs/etc/systemd/system/display-manager.service.d

# Enable NetworkManager
ln -s /usr/lib/systemd/system/NetworkManager.service \
    ~/customiso/airootfs/etc/systemd/system/multi-user.target.wants/

# Enable display manager (example for SDDM)
# Method 1: Direct symlink (may cause getty conflict)
ln -s /usr/lib/systemd/system/sddm.service \
    ~/customiso/airootfs/etc/systemd/system/display-manager.service

# Method 2: Better approach - use graphical.target
mkdir -p ~/customiso/airootfs/etc/systemd/system/graphical.target.wants
ln -s /usr/lib/systemd/system/sddm.service \
    ~/customiso/airootfs/etc/systemd/system/graphical.target.wants/

# Set default target to graphical
ln -sf /usr/lib/systemd/system/graphical.target \
    ~/customiso/airootfs/etc/systemd/system/default.target
```

### 4. Preventing "Started Getty on tty1" Issue

This is crucial for live environments. Choose one of these approaches:

#### Approach A: Configure getty and display manager to coexist

```bash
# Modify getty@tty1 to not conflict with display manager
mkdir -p ~/customiso/airootfs/etc/systemd/system/getty@tty1.service.d/
cat > ~/customiso/airootfs/etc/systemd/system/getty@tty1.service.d/override.conf << 'EOF'
[Unit]
# Don't conflict with display manager
Conflicts=

[Service]
# Autologin for live user
ExecStart=
ExecStart=-/sbin/agetty --autologin liveuser --noclear %I 38400 linux
EOF

# Configure display manager to use tty7
mkdir -p ~/customiso/airootfs/etc/sddm.conf.d/
cat > ~/customiso/airootfs/etc/sddm.conf.d/live.conf << 'EOF'
[General]
DisplayServer=x11

[X11]
ServerPath=/usr/bin/X
ServerArguments=-nolisten tcp -noreset
MinimumVT=7

[Autologin]
User=liveuser
Session=plasma.desktop
EOF
```

#### Approach B: Disable getty@tty1 entirely

```bash
# Mask getty@tty1 to prevent it from starting
ln -s /dev/null ~/customiso/airootfs/etc/systemd/system/getty@tty1.service
```

#### Approach C: Use custom startup script

```bash
# Create a custom service that handles the transition
cat > ~/customiso/airootfs/etc/systemd/system/live-desktop.service << 'EOF'
[Unit]
Description=Live Desktop Startup
After=multi-user.target
Wants=multi-user.target
Before=graphical.target

[Service]
Type=oneshot
ExecStart=/usr/bin/systemctl stop getty@tty1.service
ExecStart=/usr/bin/systemctl start sddm.service
RemainAfterExit=yes

[Install]
WantedBy=graphical.target
EOF

# Enable it
ln -s /etc/systemd/system/live-desktop.service \
    ~/customiso/airootfs/etc/systemd/system/graphical.target.wants/
```

### 5. Creating Auto-login User

Create user configuration in `airootfs`:

```bash
# Create user setup script
cat > ~/customiso/airootfs/root/customize_airootfs.sh << 'EOF'
#!/usr/bin/env bash

# Create live user
useradd -m -G wheel,audio,video,storage,power -s /bin/bash liveuser
echo "liveuser:live" | chpasswd

# Enable sudo without password for wheel group
sed -i 's/^# %wheel ALL=(ALL) NOPASSWD: ALL/%wheel ALL=(ALL) NOPASSWD: ALL/' /etc/sudoers

# Set keyboard layout
localectl set-x11-keymap us

# CRITICAL: Ensure proper permissions and home directory
chown -R liveuser:liveuser /home/liveuser
chmod 755 /home/liveuser

# Create .xinitrc for fallback
cat > /home/liveuser/.xinitrc << 'EOL'
#!/bin/sh
if [ -d /etc/X11/xinit/xinitrc.d ] ; then
 for f in /etc/X11/xinit/xinitrc.d/?*.sh ; do
  [ -x "$f" ] && . "$f"
 done
 unset f
fi
exec startplasma-x11
EOL
chmod +x /home/liveuser/.xinitrc
chown liveuser:liveuser /home/liveuser/.xinitrc

# Configure autologin for SDDM
mkdir -p /etc/sddm.conf.d
cat > /etc/sddm.conf.d/autologin.conf << 'EOL'
[Autologin]
User=liveuser
Session=plasma.desktop
Relogin=false
EOL

# For LightDM (if using instead of SDDM)
# mkdir -p /etc/lightdm/lightdm.conf.d
# cat > /etc/lightdm/lightdm.conf.d/50-autologin.conf << 'EOL'
# [Seat:*]
# autologin-user=liveuser
# autologin-user-timeout=0
# EOL

# Fix common permission issues
mkdir -p /run/sddm
chown sddm:sddm /run/sddm
chmod 755 /run/sddm

# Ensure systemd user directory exists
mkdir -p /run/user/1000
chown liveuser:liveuser /run/user/1000
chmod 700 /run/user/1000
EOF

chmod +x ~/customiso/airootfs/root/customize_airootfs.sh
```

### 6. Debugging Boot Issues

Add debugging capabilities to your live environment:

```bash
# Create debug script
cat > ~/customiso/airootfs/usr/local/bin/debug-boot << 'EOF'
#!/bin/bash
echo "=== Boot Debug Information ==="
echo "Current runlevel:"
systemctl get-default

echo -e "\nDisplay Manager Status:"
systemctl status display-manager.service

echo -e "\nGetty Status:"
systemctl status getty@tty1.service

echo -e "\nGraphical Target:"
systemctl status graphical.target

echo -e "\nFailed Services:"
systemctl --failed

echo -e "\nX Server Logs:"
cat /var/log/Xorg.0.log 2>/dev/null | grep -E "(EE)|(WW)" | head -20

echo -e "\nJournal Errors:"
journalctl -b -p err | tail -20
EOF

chmod +x ~/customiso/airootfs/usr/local/bin/debug-boot

# Add to root's .bashrc for easy access
echo "alias debug-boot='/usr/local/bin/debug-boot'" >> ~/customiso/airootfs/root/.bashrc
```

### 7. Building the ISO

```bash
# Build with proper work directory (use tmpfs if possible)
sudo mkarchiso -v -w /tmp/archiso-tmp ~/customiso

# The ISO will be in the out/ directory
```

## Common Mistakes That Cause "Started Getty on tty1"

1. **Not setting default target to graphical**
    
    - Fix: Ensure `default.target` symlinks to `graphical.target`
2. **Display manager not properly enabled**
    
    - Fix: Create proper symlinks in `graphical.target.wants/`
3. **Missing graphics drivers**
    
    - Fix: Include all necessary video drivers in `packages.x86_64`
4. **User creation script not executed**
    
    - Fix: Ensure `customize_airootfs.sh` is executable and runs during build
5. **Conflicting autologin configurations**
    
    - Fix: Configure either getty autologin OR display manager autologin, not both
6. **Wrong TTY allocation**
    
    - Fix: Configure display manager to use tty7 if getty uses tty1
7. **Missing display server**
    
    - Fix: Ensure `xorg-server` or `wayland` is installed
8. **Service ordering issues**
    
    - Fix: Use proper `After=` and `Before=` directives in service files

### Boot Parameters for Troublesome Hardware

```bash
# If stuck at "Started Getty on tty1", try:
systemd.unit=multi-user.target    # Boot to text mode first
systemd.unit=graphical.target     # Force graphical target

# For NVIDIA issues
nomodeset nvidia-drm.modeset=1

# For AMD issues
amdgpu.dc=0 amdgpu.dpm=0

# For Intel issues
i915.modeset=1 i915.enable_psr=0

# General fallback
nomodeset vga=792

# Debug boot issues
systemd.log_level=debug
rd.debug
```

## Recent Developments and Future Direction

### Arch Linux Graphical ISO Development

As of 2024, the Arch Linux team is actively working on adding graphical environment support to the official ISO:

1. **New graphical profiles** are being developed (releng-gui) that include:
    
    - XFCE as the default desktop environment
    - NetworkManager with GUI applet
    - Firefox for accessing the installation guide
    - Improved accessibility features
2. **Boot loader changes**: Arch has switched from GRUB to systemd-boot for UEFI systems due to:
    
    - Better hardware compatibility
    - Simpler Secure Boot implementation
    - More reliable graphics initialization
3. **User experience improvements**:
    
    - Unprivileged user creation for graphical sessions
    - Auto-login capabilities
    - Better keyboard layout detection

## Complete Working Example

Here's a minimal configuration that avoids the "Started Getty on tty1" issue:

```bash
# packages.x86_64 (essential packages only)
base
linux
linux-firmware
mkinitcpio
mkinitcpio-archiso
syslinux
systemd
xorg-server
xorg-xinit
sddm
plasma-desktop
konsole
networkmanager

# airootfs/root/customize_airootfs.sh
#!/bin/bash
# Create user
useradd -m -G wheel -s /bin/bash liveuser
echo "liveuser:live" | chpasswd
echo "%wheel ALL=(ALL) NOPASSWD: ALL" >> /etc/sudoers

# Enable services
systemctl enable NetworkManager
systemctl enable sddm

# Set default target
systemctl set-default graphical.target

# Configure SDDM
mkdir -p /etc/sddm.conf.d
cat > /etc/sddm.conf.d/autologin.conf << EOF
[Autologin]
User=liveuser
Session=plasma.desktop
EOF

# Disable getty@tty1 to prevent conflicts
systemctl mask getty@tty1
```

### Key Takeaways for Custom Distros

When building your custom Arch-based live environment:

1. **Follow upstream changes**: Monitor archiso development for best practices
2. **Consider accessibility**: Include screen readers and high-contrast themes
3. **Hardware detection**: Implement automatic driver selection based on detected hardware
4. **Fallback mechanisms**: Always provide text-mode fallbacks for troubleshooting
5. **Test the getty/display manager interaction**: This is the most common failure point

## Conclusion

Creating a reliable live desktop environment for an Arch-based distro requires careful attention to:

1. **Proper kernel module loading** in the initramfs
2. **Correct graphics driver configuration** for various hardware
3. **Display manager compatibility** with your chosen desktop environment
4. **SquashFS and overlay filesystem** setup for the live environment
5. **Comprehensive testing** on various hardware configurations
6. **Proper service ordering** to avoid getty/display manager conflicts

The "Started Getty on tty1" issue is typically caused by display manager services failing to start or conflicting with getty@tty1.service. The solutions provided in this report address these conflicts through proper service configuration, TTY allocation, and systemd target management.

By following these guidelines and implementing proper fallback mechanisms, you can create a robust live environment that boots successfully on most hardware configurations.

Remember to always test your live environment on various hardware before release, and provide clear documentation for users who might encounter boot issues. Stay updated with archiso development to benefit from upstream improvements and maintain compatibility with the latest Arch Linux standards.