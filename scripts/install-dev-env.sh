#!/bin/bash
# HorizonOS Development Environment Installer
# Run this from Arch Linux live ISO
#
# FIXES APPLIED:
# 1. Check for UEFI boot mode before starting
# 2. Better partition detection for both /dev/sdX and /dev/nvmeXnX devices  
# 3. Fixed snapper configuration (removed manual @snapshots creation)
# 4. Fixed user creation order (create user before chown)
# 5. Added GRUB debug output and rootflags for btrfs
# 6. Added boot verification steps
# 7. Added EFI fallback boot entry
# 8. Extended reboot timer with troubleshooting info
#
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration variables
HOSTNAME="horizonos-dev"
USERNAME="yousef"
TIMEZONE="UTC"

echo -e "${GREEN}=====================================${NC}"
echo -e "${GREEN}HorizonOS Development Environment${NC}"
echo -e "${GREEN}Installer v1.0${NC}"
echo -e "${GREEN}=====================================${NC}"
echo ""

# Check if running as root
if [ "$EUID" -ne 0 ]; then 
    echo -e "${RED}Please run as root (use sudo)${NC}"
    exit 1
fi

# Check if booted in UEFI mode
if [ ! -d /sys/firmware/efi/efivars ]; then
    echo -e "${RED}ERROR: Not booted in UEFI mode!${NC}"
    echo "Please ensure UEFI boot is enabled in BIOS/UEFI settings"
    exit 1
fi

# Check internet connection
echo -e "${YELLOW}Checking internet connection...${NC}"
if ! ping -c 1 archlinux.org &> /dev/null; then
    echo -e "${RED}No internet connection! Please connect first.${NC}"
    echo "For WiFi, use: iwctl"
    exit 1
fi
echo -e "${GREEN}Internet connected ✓${NC}"

# List available disks
echo -e "${YELLOW}Available disks:${NC}"
lsblk -d -p -n -o NAME,SIZE,MODEL | grep -E '/dev/[sv]d[a-z]|/dev/nvme[0-9]n[0-9]'
echo ""

# Get target disk
read -p "Enter target disk (e.g., /dev/sda or /dev/vda): " DISK

# Confirm disk selection
echo -e "${RED}WARNING: This will ERASE ALL DATA on ${DISK}${NC}"
read -p "Continue? (type 'yes' to confirm): " CONFIRM
if [ "$CONFIRM" != "yes" ]; then
    echo "Installation cancelled."
    exit 1
fi

# Get user password
echo ""
echo -e "${YELLOW}=== User Account Setup ===${NC}"
echo -e "Creating user account: ${GREEN}${USERNAME}${NC}"
echo ""
read -s -p "Enter password for user '${USERNAME}': " USER_PASSWORD
echo ""
read -s -p "Confirm password: " USER_PASSWORD_CONFIRM
echo ""
if [ "$USER_PASSWORD" != "$USER_PASSWORD_CONFIRM" ]; then
    echo -e "${RED}Passwords do not match!${NC}"
    exit 1
fi
if [ -z "$USER_PASSWORD" ]; then
    echo -e "${RED}Password cannot be empty!${NC}"
    exit 1
fi

echo -e "${YELLOW}Starting installation...${NC}"

# Partition the disk
echo -e "${YELLOW}Partitioning ${DISK}...${NC}"
parted -s ${DISK} mklabel gpt
parted -s ${DISK} mkpart ESP fat32 1MiB 512MiB
parted -s ${DISK} set 1 esp on
parted -s ${DISK} mkpart primary btrfs 512MiB 100%

# Wait for kernel to recognize partitions
sleep 2
partprobe ${DISK}
sleep 1

# Format partitions - handle both partition naming schemes
echo -e "${YELLOW}Formatting partitions...${NC}"
if [ -e "${DISK}p1" ]; then
    # NVMe style naming
    BOOT_PART="${DISK}p1"
    ROOT_PART="${DISK}p2"
elif [ -e "${DISK}1" ]; then
    # Standard naming
    BOOT_PART="${DISK}1"
    ROOT_PART="${DISK}2"
else
    echo -e "${RED}ERROR: Cannot find partitions!${NC}"
    exit 1
fi

echo "Boot partition: $BOOT_PART"
echo "Root partition: $ROOT_PART"

mkfs.fat -F32 $BOOT_PART
mkfs.btrfs -f $ROOT_PART

# Mount and create Btrfs subvolumes
echo -e "${YELLOW}Creating Btrfs subvolumes...${NC}"
mount $ROOT_PART /mnt
btrfs subvolume create /mnt/@
btrfs subvolume create /mnt/@home
btrfs subvolume create /mnt/@var
# Don't create @snapshots - let snapper handle it
umount /mnt

# Mount subvolumes
mount -o compress=zstd:1,noatime,subvol=@ $ROOT_PART /mnt
mkdir -p /mnt/{boot,home,var}
mount $BOOT_PART /mnt/boot
mount -o compress=zstd:1,noatime,subvol=@home $ROOT_PART /mnt/home
mount -o compress=zstd:1,noatime,subvol=@var $ROOT_PART /mnt/var

# Install minimal system focused on HorizonOS development
echo -e "${YELLOW}Installing base system and essential tools...${NC}"

# Detect CPU type for microcode
CPU_VENDOR=$(grep -m1 vendor_id /proc/cpuinfo | cut -d: -f2 | tr -d ' ')
if [[ "$CPU_VENDOR" == "GenuineIntel" ]]; then
    MICROCODE="intel-ucode"
elif [[ "$CPU_VENDOR" == "AuthenticAMD" ]]; then
    MICROCODE="amd-ucode"
else
    MICROCODE=""
fi

pacstrap /mnt base base-devel linux linux-firmware $MICROCODE \
    btrfs-progs fish git neovim nano \
    networkmanager openssh \
    plasma-desktop sddm konsole dolphin \
    firefox \
    grub efibootmgr os-prober \
    docker qemu-full libvirt \
    archiso arch-install-scripts ostree \
    kotlin gradle \
    btop bat nano \
    snapper snap-pac grub-btrfs

# Generate fstab
echo -e "${YELLOW}Generating fstab...${NC}"
genfstab -U /mnt >> /mnt/etc/fstab

# Verify fstab has correct entries
echo -e "${YELLOW}Verifying fstab entries...${NC}"
cat /mnt/etc/fstab

# Save installation info for debugging
cat > /mnt/root/install-info.txt << INFO
HorizonOS Installation Info
Date: $(date)
Boot Partition: $BOOT_PART
Root Partition: $ROOT_PART
Disk: $DISK

Partition Layout:
$(lsblk $DISK)

Btrfs Subvolumes:
$(btrfs subvolume list /mnt)

INFO

# Chroot and configure system
echo -e "${YELLOW}Configuring system...${NC}"
arch-chroot /mnt /bin/bash <<'EOF'
# Set timezone
ln -sf /usr/share/zoneinfo/UTC /etc/localtime
hwclock --systohc

# Locale
echo "en_US.UTF-8 UTF-8" > /etc/locale.gen
locale-gen
echo "LANG=en_US.UTF-8" > /etc/locale.conf

# Hostname
echo "horizonos-dev" > /etc/hostname
cat > /etc/hosts <<HOSTS
127.0.0.1   localhost
::1         localhost
127.0.1.1   horizonos-dev.localdomain horizonos-dev
HOSTS

# mkinitcpio with btrfs
sed -i 's/^HOOKS=.*/HOOKS=(base udev autodetect modconf kms keyboard keymap consolefont block btrfs filesystems fsck)/' /etc/mkinitcpio.conf
mkinitcpio -P

# Verify kernel and initramfs were generated
echo "Checking for kernel and initramfs..."
ls -la /boot/

# Install bootloader
echo "Installing GRUB..."
# Ensure efivars are available
mount -t efivarfs efivarfs /sys/firmware/efi/efivars 2>/dev/null || true
grub-install --target=x86_64-efi --efi-directory=/boot --bootloader-id=HorizonOS-Dev --recheck --debug

# Configure GRUB for Btrfs subvolumes
# First, ensure GRUB can see the Btrfs root
echo 'GRUB_CMDLINE_LINUX="rootflags=subvol=@"' >> /etc/default/grub
echo 'GRUB_PRELOAD_MODULES="part_gpt part_msdos btrfs"' >> /etc/default/grub
echo 'GRUB_ENABLE_LINUX_UUID=true' >> /etc/default/grub

# Generate GRUB config
echo "Generating GRUB configuration..."
grub-mkconfig -o /boot/grub/grub.cfg

# Verify EFI boot entry was created
echo "EFI boot entries:"
efibootmgr -v

# Also create a fallback EFI boot entry
mkdir -p /boot/EFI/BOOT
cp /boot/EFI/HorizonOS-Dev/grubx64.efi /boot/EFI/BOOT/BOOTX64.EFI 2>/dev/null || true

# Configure snapper (FIXED)
# Create snapper configurations - it will create its own .snapshots subvolumes
snapper -c root create-config /
snapper -c home create-config /home

# Configure snapper settings for root
snapper -c root set-config "TIMELINE_CREATE=yes"
snapper -c root set-config "TIMELINE_CLEANUP=yes"
snapper -c root set-config "TIMELINE_LIMIT_HOURLY=5"
snapper -c root set-config "TIMELINE_LIMIT_DAILY=7"
snapper -c root set-config "TIMELINE_LIMIT_WEEKLY=0"
snapper -c root set-config "TIMELINE_LIMIT_MONTHLY=0"
snapper -c root set-config "TIMELINE_LIMIT_YEARLY=0"

# Set number cleanup limits for root
snapper -c root set-config "NUMBER_CLEANUP=yes"
snapper -c root set-config "NUMBER_MIN_AGE=1800"
snapper -c root set-config "NUMBER_LIMIT=50"
snapper -c root set-config "NUMBER_LIMIT_IMPORTANT=10"

# Configure snapper settings for home
snapper -c home set-config "TIMELINE_CREATE=yes"
snapper -c home set-config "TIMELINE_CLEANUP=yes"
snapper -c home set-config "TIMELINE_LIMIT_HOURLY=5"
snapper -c home set-config "TIMELINE_LIMIT_DAILY=7"
snapper -c home set-config "TIMELINE_LIMIT_WEEKLY=0"
snapper -c home set-config "TIMELINE_LIMIT_MONTHLY=0"
snapper -c home set-config "TIMELINE_LIMIT_YEARLY=0"

# Enable snapper services
systemctl enable snapper-timeline.timer
systemctl enable snapper-cleanup.timer

# Enable grub-btrfsd only if the .snapshots subvolume exists
if btrfs subvolume list / | grep -q ".snapshots"; then
    systemctl enable grub-btrfsd
fi

# Enable other services
systemctl enable NetworkManager
systemctl enable sddm
systemctl enable docker
systemctl enable libvirtd
systemctl enable sshd

# Make fish default for new users
echo "/usr/bin/fish" >> /etc/shells
sed -i 's|SHELL=/bin/bash|SHELL=/usr/bin/fish|' /etc/default/useradd

# Configure SDDM for autologin (optional, remove for production)
mkdir -p /etc/sddm.conf.d
cat > /etc/sddm.conf.d/autologin.conf <<SDDM
[Autologin]
User=yousef
Session=plasma
SDDM

EOF

# Use the password variables that were read earlier
arch-chroot /mnt /bin/bash <<EOF
# Set root password
echo "root:${USER_PASSWORD}" | chpasswd

# Create user with fish shell
echo "Creating user: ${USERNAME}"
useradd -m -G wheel,docker,libvirt -s /usr/bin/fish ${USERNAME}
echo "${USERNAME}:${USER_PASSWORD}" | chpasswd

# Configure sudoers
echo "%wheel ALL=(ALL:ALL) ALL" > /etc/sudoers.d/wheel

# Create initial setup script for first boot
cat > /home/${USERNAME}/setup-horizonos.sh <<'SETUP'
#!/bin/bash
set -e

echo "Setting up HorizonOS development environment..."

# Install yay AUR helper
if ! command -v yay &> /dev/null; then
    cd /tmp
    git clone https://aur.archlinux.org/yay.git
    cd yay
    makepkg -si --noconfirm
    cd ..
    rm -rf yay
fi

# Install additional tools from AUR
yay -S --noconfirm visual-studio-code-bin

# Optional: Install more tools if needed
echo ""
echo "You can install additional tools later:"
echo "  IntelliJ: yay -S intellij-idea-ultimate-edition"
echo "  Git LFS: sudo pacman -S git-lfs"
echo "  GitHub CLI: sudo pacman -S github-cli"
echo ""

# Set up Fish shell
fish -c "curl -sL https://raw.githubusercontent.com/oh-my-fish/oh-my-fish/master/bin/install | fish"

# Clone HorizonOS project
echo "Cloning HorizonOS repository..."
cd ~
git clone https://github.com/codeyousef/HorizonOS.git horizonos
cd ~/horizonos

# Initialize git LFS
git lfs install

# Initialize OSTree repository (not in git)
ostree init --repo=repo --mode=archive

# Make scripts executable
chmod +x scripts/*.sh 2>/dev/null || echo "No scripts found yet"

# Create any missing directories
mkdir -p {build,iso,src/{base,desktop,kotlin-config,llm-integration}}

# Test if we can run the build script
if [ -f scripts/build-test.sh ]; then
    echo "✓ Build script found in repository"
else
    echo "! Build script not found, you may need to create it"
fi

# Create/update progress document
if [ ! -f docs/PROGRESS.md ]; then
    echo "Creating progress document..."
    mkdir -p docs
    cat > docs/PROGRESS.md << 'PROGRESS'
# HorizonOS Development Progress

## VM Development Environment Setup ✅

- **Date**: $(date +%Y-%m-%d)
- **Username**: yousef
- **Base System**: Arch Linux (no LVM, pure Btrfs)
- **Desktop**: KDE Plasma
- **Development Tools**: All installed

## Next Steps

1. Run first build: `hos-build`
2. Create bootable ISO
3. Start Kotlin DSL development

See full progress at: https://github.com/codeyousef/HorizonOS/blob/main/docs/PROGRESS.md
PROGRESS
fi

# Create Fish functions
mkdir -p ~/.config/fish/functions

echo 'function hos
    cd ~/horizonos
end' > ~/.config/fish/functions/hos.fish

echo 'function hos-build
    cd ~/horizonos && ./scripts/build-test.sh
end' > ~/.config/fish/functions/hos-build.fish

echo 'function hos-status
    cd ~/horizonos && ostree log --repo=repo horizonos/test/x86_64
end' > ~/.config/fish/functions/hos-status.fish

# Install VS Code extensions
code --install-extension ms-vscode.cpptools
code --install-extension rust-lang.rust-analyzer
code --install-extension fwcd.kotlin
code --install-extension redhat.vscode-yaml
code --install-extension skyapps.fish-vscode

echo ""
echo "✅ HorizonOS development environment setup complete!"
echo ""
echo "Your HorizonOS project has been cloned to: ~/horizonos"
echo ""
echo "GitHub Authentication Options:"
echo "1. Use GitHub CLI (recommended):"
echo "   gh auth login"
echo ""
echo "2. Use Personal Access Token:"
echo "   - Create at: https://github.com/settings/tokens"
echo "   - Use token as password when pushing"
echo ""
echo "3. Set up SSH keys:"
echo "   ssh-keygen -t ed25519"
echo "   gh auth login (choose SSH)"
echo ""
echo "Quick commands:"
echo "  hos        - Go to HorizonOS directory"
echo "  hos-build  - Build test system"
echo "  hos-status - Check OSTree status"
echo ""
SETUP

chown ${USERNAME}:${USERNAME} /home/${USERNAME}/setup-horizonos.sh
chmod +x /home/${USERNAME}/setup-horizonos.sh

# Create a desktop entry for the setup script
mkdir -p /home/${USERNAME}/Desktop
cat > /home/${USERNAME}/Desktop/setup-horizonos.desktop << DESKTOP
[Desktop Entry]
Type=Application
Name=Setup HorizonOS Dev
Comment=Complete HorizonOS development setup
Exec=konsole -e bash /home/${USERNAME}/setup-horizonos.sh
Icon=applications-development
Terminal=true
DESKTOP
chmod +x /home/${USERNAME}/Desktop/setup-horizonos.desktop
chown ${USERNAME}:${USERNAME} /home/${USERNAME}/Desktop/setup-horizonos.desktop

# Final verification before ending chroot
echo ""
echo "=== Boot Configuration Verification ==="
echo "Checking boot files..."
if [ -f /boot/vmlinuz-linux ] && [ -f /boot/initramfs-linux.img ]; then
    echo "✓ Kernel and initramfs found"
else
    echo "✗ ERROR: Kernel or initramfs missing!"
fi

echo "Checking EFI boot entries..."
efibootmgr -v

echo "Checking GRUB installation..."
if [ -d /boot/grub ] && [ -f /boot/grub/grub.cfg ]; then
    echo "✓ GRUB installed"
    echo "Root device in GRUB:"
    grep -E "root=|rootflags=" /boot/grub/grub.cfg | head -5
else
    echo "✗ ERROR: GRUB not properly installed!"
fi

echo "Checking fstab..."
cat /etc/fstab
EOF

echo ""
echo -e "${GREEN}=====================================${NC}"
echo -e "${GREEN}Installation Complete!${NC}"
echo -e "${GREEN}=====================================${NC}"
echo ""
echo "System will reboot in 30 seconds..."
echo ""
echo "After reboot:"
echo "1. Login as 'yousef' with your password"
echo "2. Run the setup script on the desktop or:"
echo "   ~/setup-horizonos.sh"
echo ""
echo "Default credentials:"
echo "  Username: ${USERNAME}"
echo "  Password: (what you entered)"
echo ""
echo -e "${YELLOW}=== TROUBLESHOOTING ===${NC}"
echo "If system doesn't boot:"
echo "1. Boot from live ISO again"
echo "2. Mount system: "
echo "   mount -o subvol=@ ${ROOT_PART} /mnt"
echo "   mount ${BOOT_PART} /mnt/boot"
echo "   arch-chroot /mnt"
echo "3. Check logs:"
echo "   journalctl -xb"
echo "   dmesg | grep -i error"
echo "4. Reinstall GRUB:"
echo "   grub-install --target=x86_64-efi --efi-directory=/boot --bootloader-id=HorizonOS --recheck"
echo "   grub-mkconfig -o /boot/grub/grub.cfg"
echo ""
echo -e "${YELLOW}Press Ctrl+C to cancel reboot and debug${NC}"

sleep 30
reboot
