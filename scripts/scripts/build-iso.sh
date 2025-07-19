#!/usr/bin/env bash
set -euo pipefail

# HorizonOS ISO Builder
# This script creates a bootable ISO with OSTree support

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BUILD_DIR="$PROJECT_ROOT/build"
ISO_DIR="$PROJECT_ROOT/iso"
WORK_DIR="$BUILD_DIR/archiso-work"
OUT_DIR="$BUILD_DIR/out"

# Load configuration
source "$PROJECT_ROOT/config/dev.conf"

echo "=== HorizonOS ISO Builder ==="
echo "Version: $HORIZONOS_VERSION"
echo "Codename: $HORIZONOS_CODENAME"

# Check if running as root
if [ "$EUID" -ne 0 ]; then 
    echo "This script must be run as root (use sudo)"
    exit 1
fi

# Check for required tools
for tool in mkarchiso ostree; do
    if ! command -v $tool &> /dev/null; then
        echo "Error: $tool is not installed"
        exit 1
    fi
done

# Check if OSTree repo exists and has commits
if [ ! -d "$PROJECT_ROOT/repo" ]; then
    echo "Error: OSTree repository not found at $PROJECT_ROOT/repo"
    echo "Run build-test.sh first to create an OSTree commit"
    exit 1
fi

if ! ostree log --repo="$PROJECT_ROOT/repo" horizonos/test/x86_64 &> /dev/null; then
    echo "Error: No OSTree commits found for horizonos/test/x86_64"
    echo "Run build-test.sh first to create an OSTree commit"
    exit 1
fi

# Clean previous builds
echo "Cleaning previous builds..."
rm -rf "$WORK_DIR" "$OUT_DIR"
mkdir -p "$ISO_DIR" "$OUT_DIR"

# Create archiso profile
echo "Creating archiso profile..."
cp -r /usr/share/archiso/configs/releng "$ISO_DIR/horizonos-profile"

# Customize the profile for HorizonOS
cd "$ISO_DIR/horizonos-profile"

# Update packages.x86_64
cat > packages.x86_64 << 'EOF'
# Base system
base
base-devel
linux
linux-firmware
mkinitcpio
mkinitcpio-archiso
syslinux
efibootmgr

# Filesystem
btrfs-progs
dosfstools
e2fsprogs

# Networking
networkmanager
openssh

# System
grub
os-prober
ostree
fish
sudo

# Archive tools
libarchive
squashfs-tools

# Live environment
archinstall
arch-install-scripts
gptfdisk
parted
reflector
rsync
screen
tmux
vim
nano
htop

# HorizonOS specific
EOF

# Create custom airootfs overlay
mkdir -p airootfs/etc/systemd/system
mkdir -p airootfs/etc/skel
mkdir -p airootfs/usr/local/bin
mkdir -p airootfs/usr/share/horizonos

# Create HorizonOS installer script
cat > airootfs/usr/local/bin/horizonos-install << 'INSTALLER'
#!/bin/bash
# HorizonOS Installer
set -e

echo "================================"
echo "HorizonOS Installer"
echo "Version: ${HORIZONOS_VERSION:-dev}"
echo "================================"
echo ""

# Function to list disks
list_disks() {
    echo "Available disks:"
    lsblk -d -p -n -o NAME,SIZE,MODEL | grep -E '/dev/[sv]d[a-z]|/dev/nvme[0-9]n[0-9]'
}

# Function to create partitions
create_partitions() {
    local disk=$1
    echo "Creating partitions on $disk..."
    
    parted -s $disk mklabel gpt
    parted -s $disk mkpart ESP fat32 1MiB 512MiB
    parted -s $disk set 1 esp on
    parted -s $disk mkpart primary btrfs 512MiB 100%
    
    # Wait for partitions to appear
    sleep 2
    partprobe $disk
}

# Function to setup btrfs
setup_btrfs() {
    local root_part=$1
    echo "Setting up Btrfs on $root_part..."
    
    mkfs.btrfs -f $root_part
    mount $root_part /mnt
    
    # Create subvolumes
    btrfs subvolume create /mnt/@
    btrfs subvolume create /mnt/@home
    btrfs subvolume create /mnt/@var
    btrfs subvolume create /mnt/@snapshots
    
    umount /mnt
}

# Main installation flow
main() {
    echo "Welcome to HorizonOS installation!"
    echo ""
    
    list_disks
    echo ""
    read -p "Select target disk (e.g., /dev/sda): " DISK
    
    if [ ! -b "$DISK" ]; then
        echo "Error: $DISK is not a valid block device"
        exit 1
    fi
    
    echo ""
    echo "WARNING: This will ERASE ALL DATA on $DISK"
    read -p "Continue? (yes/no): " CONFIRM
    
    if [ "$CONFIRM" != "yes" ]; then
        echo "Installation cancelled."
        exit 0
    fi
    
    # Detect if we need partition number suffix
    if [[ "$DISK" =~ nvme ]]; then
        BOOT_PART="${DISK}p1"
        ROOT_PART="${DISK}p2"
    else
        BOOT_PART="${DISK}1"
        ROOT_PART="${DISK}2"
    fi
    
    # Create partitions
    create_partitions $DISK
    
    # Format boot partition
    echo "Formatting boot partition..."
    mkfs.fat -F32 $BOOT_PART
    
    # Setup btrfs
    setup_btrfs $ROOT_PART
    
    # Mount everything
    echo "Mounting filesystems..."
    mount -o compress=zstd:1,noatime,subvol=@ $ROOT_PART /mnt
    mkdir -p /mnt/{boot,home,var,.snapshots}
    mount $BOOT_PART /mnt/boot
    mount -o compress=zstd:1,noatime,subvol=@home $ROOT_PART /mnt/home
    mount -o compress=zstd:1,noatime,subvol=@var $ROOT_PART /mnt/var
    mount -o compress=zstd:1,noatime,subvol=@snapshots $ROOT_PART /mnt/.snapshots
    
    # Initialize OSTree
    echo "Initializing OSTree..."
    ostree admin init-fs --modern /mnt
    
    # Copy OSTree repository from live environment
    echo "Copying OSTree repository..."
    if [ -d "/usr/share/horizonos/repo" ]; then
        cp -a /usr/share/horizonos/repo /mnt/ostree/
    else
        echo "Warning: OSTree repository not found in live environment"
        echo "You'll need to pull from a remote repository later"
    fi
    
    # Deploy OSTree
    echo "Deploying HorizonOS..."
    ostree admin os-init horizonos --sysroot=/mnt
    
    if [ -d "/mnt/ostree/repo" ]; then
        # Deploy from local repo
        COMMIT=$(ostree --repo=/mnt/ostree/repo rev-parse horizonos/test/x86_64)
        ostree admin deploy --sysroot=/mnt --os=horizonos $COMMIT
    fi
    
    # Configure bootloader
    echo "Installing bootloader..."
    # Mount necessary filesystems for chroot
    mount --bind /dev /mnt/dev
    mount --bind /proc /mnt/proc
    mount --bind /sys /mnt/sys
    
    # Install bootloader from outside chroot first
    grub-install --target=x86_64-efi --efi-directory=/mnt/boot --boot-directory=/mnt/boot --bootloader-id=HorizonOS --removable
    
    # Create basic GRUB config for OSTree
    mkdir -p /mnt/boot/grub
    cat > /mnt/boot/grub/grub.cfg << 'GRUBCFG'
set default=0
set timeout=5

menuentry 'HorizonOS' {
    # OSTree will manage the actual boot entries
    echo "Loading HorizonOS..."
    # This will be replaced by ostree-grub-generator
}
GRUBCFG
    
    # Unmount bind mounts
    umount /mnt/dev /mnt/proc /mnt/sys || true
    
    # Basic configuration
    echo "Configuring system..."
    echo "horizonos" > /mnt/etc/hostname
    
    # Configure container runtime for installed system
    echo "Setting up container runtime..."
    
    # Create container directories
    mkdir -p /mnt/etc/containers
    mkdir -p /mnt/var/lib/containers
    mkdir -p /mnt/var/lib/flatpak
    
    # Copy container configurations
    cp -r /etc/containers/* /mnt/etc/containers/ 2>/dev/null || true
    
    # Configure systemd services for containers
    arch-chroot /mnt systemctl enable podman.socket
    arch-chroot /mnt systemctl enable flatpak-system-helper
    
    # Create container user group
    arch-chroot /mnt groupadd -f containers
    
    # Copy HorizonOS tools to installed system
    cp /usr/local/bin/horizon-container /mnt/usr/local/bin/
    chmod +x /mnt/usr/local/bin/horizon-container
    
    # Set up subuid/subgid for rootless containers
    echo "containers:100000:65536" >> /mnt/etc/subuid
    echo "containers:100000:65536" >> /mnt/etc/subgid
    
    # Create fstab
    genfstab -U /mnt >> /mnt/etc/fstab
    
    echo ""
    echo "================================"
    echo "Installation complete!"
    echo "================================"
    echo ""
    echo "Remove installation media and reboot."
}

# Run main function
main
INSTALLER

chmod +x airootfs/usr/local/bin/horizonos-install

# Copy HorizonOS tools
echo "Including HorizonOS container management tools..."
mkdir -p airootfs/usr/local/bin
cp "$PROJECT_ROOT/scripts/tools/horizon-container" airootfs/usr/local/bin/
cp "$PROJECT_ROOT/scripts/tools/horizonos-autoupdate" airootfs/usr/local/bin/
cp "$PROJECT_ROOT/scripts/tools/horizonos-update-notify" airootfs/usr/local/bin/
chmod +x airootfs/usr/local/bin/*

# Add boot debug service for troubleshooting getty issues
cat > airootfs/etc/systemd/system/horizonos-boot-debug.service << 'EOF'
[Unit]
Description=HorizonOS Boot Debug Logger
DefaultDependencies=no
After=multi-user.target
Before=getty.target

[Service]
Type=oneshot
ExecStart=/usr/bin/bash -c '
    echo "=== HorizonOS Boot Debug ===" > /dev/tty1
    echo "Current target: $(systemctl get-default)" > /dev/tty1
    echo "" > /dev/tty1
    echo "getty.target status:" > /dev/tty1
    systemctl status getty.target --no-pager > /dev/tty1 2>&1
    echo "" > /dev/tty1
    echo "getty@tty1.service status:" > /dev/tty1
    systemctl status getty@tty1.service --no-pager > /dev/tty1 2>&1
    echo "" > /dev/tty1
    echo "Active targets:" > /dev/tty1
    systemctl list-units --type=target --state=active --no-pager > /dev/tty1 2>&1
    echo "" > /dev/tty1
    echo "Failed services:" > /dev/tty1
    systemctl list-units --failed --no-pager > /dev/tty1 2>&1
    sleep 5
'
RemainAfterExit=yes
StandardOutput=tty
StandardError=tty
TTYPath=/dev/tty1

[Install]
WantedBy=multi-user.target
EOF

mkdir -p airootfs/etc/systemd/system/multi-user.target.wants
ln -sf /etc/systemd/system/horizonos-boot-debug.service airootfs/etc/systemd/system/multi-user.target.wants/

# Copy OSTree repository to ISO (if it exists and is small enough)
if [ -d "$PROJECT_ROOT/repo" ]; then
    REPO_SIZE=$(du -sm "$PROJECT_ROOT/repo" | cut -f1)
    if [ $REPO_SIZE -lt 500 ]; then
        echo "Including OSTree repository in ISO (${REPO_SIZE}MB)..."
        cp -a "$PROJECT_ROOT/repo" airootfs/usr/share/horizonos/
    else
        echo "OSTree repository too large (${REPO_SIZE}MB), skipping inclusion"
    fi
fi

# Configure boot parameters for all boot methods
echo "Configuring boot parameters..."

# Update GRUB configuration for UEFI
if [ -f grub/grub.cfg ]; then
    sed -i 's/archisobasedir=%INSTALL_DIR%/archisobasedir=%INSTALL_DIR% systemd.unit=multi-user.target/g' grub/grub.cfg
fi

# Update syslinux configuration for BIOS
if [ -d syslinux ]; then
    for cfg in syslinux/*.cfg; do
        if [ -f "$cfg" ]; then
            sed -i 's/archisobasedir=%INSTALL_DIR%/archisobasedir=%INSTALL_DIR% systemd.unit=multi-user.target/g' "$cfg"
        fi
    done
fi

# Update systemd-boot entries for UEFI
if [ -d efiboot/loader/entries ]; then
    for entry in efiboot/loader/entries/*.conf; do
        if [ -f "$entry" ]; then
            sed -i 's/archisobasedir=%INSTALL_DIR%/archisobasedir=%INSTALL_DIR% systemd.unit=multi-user.target/g' "$entry"
        fi
    done
fi

# Apply comprehensive getty fix and minimal branding
echo "Applying HorizonOS customizations..."

# Set hostname
echo "horizonos" > airootfs/etc/hostname

# Apply comprehensive getty fix
# This removes conflicting configs and creates a working autologin setup
source "$PROJECT_ROOT/scripts/scripts/boot-fixes/getty-autologin.sh"
fix_getty_in_iso "airootfs"

# CRITICAL: Set default systemd target to multi-user (text mode) to prevent hanging at graphical.target
# This prevents the ISO from trying to start a graphical interface
echo "Setting default systemd target to multi-user.target..."
mkdir -p airootfs/etc/systemd/system
ln -sf /usr/lib/systemd/system/multi-user.target airootfs/etc/systemd/system/default.target

# Ensure getty.target is properly configured and pulled in by multi-user.target
# This fixes the hang at "Reached target Multi-User System"
echo "Configuring getty.target dependencies..."

# Create drop-in to ensure getty.target is properly pulled in
mkdir -p airootfs/etc/systemd/system/getty.target.d
cat > airootfs/etc/systemd/system/getty.target.d/horizonos-enable.conf << 'EOF'
[Unit]
# Ensure getty.target starts with multi-user.target
WantedBy=multi-user.target
RequiredBy=multi-user.target

[Install]
WantedBy=multi-user.target
EOF

# Properly enable getty.target (mimicking systemctl enable)
mkdir -p airootfs/etc/systemd/system/multi-user.target.wants
ln -sf /usr/lib/systemd/system/getty.target airootfs/etc/systemd/system/multi-user.target.wants/getty.target

# Enable getty@tty1.service using proper instantiation from template
# This mimics what 'systemctl enable getty@tty1.service' would do
mkdir -p airootfs/etc/systemd/system/getty.target.wants
ln -sf /usr/lib/systemd/system/getty@.service airootfs/etc/systemd/system/getty.target.wants/getty@tty1.service

# Also add to multi-user.target as a failsafe
ln -sf /usr/lib/systemd/system/getty@.service airootfs/etc/systemd/system/multi-user.target.wants/getty@tty1.service

# Create a rescue getty service as an absolute failsafe
cat > airootfs/etc/systemd/system/horizonos-rescue-getty.service << 'EOF'
[Unit]
Description=HorizonOS Rescue Getty on tty2
Documentation=man:agetty(8)
After=systemd-user-sessions.service
After=multi-user.target
ConditionPathExists=/dev/tty2

[Service]
Type=idle
ExecStart=-/usr/bin/agetty --noclear tty2 38400 linux
Restart=always
RestartSec=5s
StandardInput=tty
StandardOutput=tty
StandardError=journal
TTYPath=/dev/tty2
TTYReset=yes
TTYVHangup=yes

[Install]
WantedBy=multi-user.target
EOF

# Enable the rescue getty service
ln -sf /etc/systemd/system/horizonos-rescue-getty.service airootfs/etc/systemd/system/multi-user.target.wants/

# Minimal branding - no ASCII art that could interfere
cat > airootfs/etc/motd << 'EOF'
Welcome to HorizonOS Live
To install: horizonos-install

If you see getty errors, run: debug-getty
EOF

# Basic os-release for identification
cat > airootfs/etc/os-release << 'EOF'
NAME="HorizonOS"
PRETTY_NAME="HorizonOS Live"
ID=horizonos
ID_LIKE=arch
VERSION_ID="0.1.0"
ANSI_COLOR="0;36"
HOME_URL="https://github.com/codeyousef/HorizonOS"
EOF


# Customize profiledef.sh
sed -i 's/iso_name=.*/iso_name="horizonos"/' profiledef.sh
sed -i 's/iso_label=.*/iso_label="HORIZONOS_'"$(date +%Y%m)"'"/' profiledef.sh
sed -i 's/iso_publisher=.*/iso_publisher="HorizonOS Project"/' profiledef.sh
sed -i 's/iso_application=.*/iso_application="HorizonOS Live\/Installation Medium"/' profiledef.sh
sed -i 's/iso_version=.*/iso_version="'"$HORIZONOS_VERSION"'"/' profiledef.sh

# Build the ISO
echo "Building ISO..."
mkarchiso -v -w "$WORK_DIR" -o "$OUT_DIR" "$ISO_DIR/horizonos-profile"

# Get the ISO filename
ISO_FILE=$(ls -1 "$OUT_DIR"/*.iso | head -n1)

if [ -f "$ISO_FILE" ]; then
    echo ""
    echo "================================"
    echo "ISO build complete!"
    echo "File: $ISO_FILE"
    echo "Size: $(du -h "$ISO_FILE" | cut -f1)"
    echo ""
    echo "Test with QEMU:"
    echo "  qemu-system-x86_64 -m 4G -enable-kvm -cdrom \"$ISO_FILE\""
    echo ""
    echo "Or write to USB:"
    echo "  sudo dd if=\"$ISO_FILE\" of=/dev/sdX bs=4M status=progress"
    echo "================================"
else
    echo "Error: ISO build failed!"
    exit 1
fi