#!/bin/bash
# HorizonOS Development Environment Installer
# Run this from Arch Linux live ISO
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

# Format partitions
echo -e "${YELLOW}Formatting partitions...${NC}"
mkfs.fat -F32 ${DISK}1 2>/dev/null || mkfs.fat -F32 ${DISK}p1
mkfs.btrfs -f ${DISK}2 2>/dev/null || mkfs.btrfs -f ${DISK}p2

# Set up partition variables
if [ -e "${DISK}p1" ]; then
    BOOT_PART="${DISK}p1"
    ROOT_PART="${DISK}p2"
else
    BOOT_PART="${DISK}1"
    ROOT_PART="${DISK}2"
fi

# Mount and create Btrfs subvolumes
echo -e "${YELLOW}Creating Btrfs subvolumes...${NC}"
mount $ROOT_PART /mnt
btrfs subvolume create /mnt/@
btrfs subvolume create /mnt/@home
btrfs subvolume create /mnt/@snapshots
btrfs subvolume create /mnt/@var
umount /mnt

# Mount subvolumes
mount -o compress=zstd:1,noatime,subvol=@ $ROOT_PART /mnt
mkdir -p /mnt/{boot,home,.snapshots,var}
mount $BOOT_PART /mnt/boot
mount -o compress=zstd:1,noatime,subvol=@home $ROOT_PART /mnt/home
mount -o compress=zstd:1,noatime,subvol=@snapshots $ROOT_PART /mnt/.snapshots
mount -o compress=zstd:1,noatime,subvol=@var $ROOT_PART /mnt/var

# Install base system with development tools
echo -e "${YELLOW}Installing base system and development tools...${NC}"
pacstrap /mnt base base-devel linux linux-firmware \
    btrfs-progs fish git git-lfs github-cli neovim vim nano \
    networkmanager openssh wget curl \
    plasma-meta kde-applications-meta sddm \
    firefox konsole dolphin kate \
    grub efibootmgr os-prober \
    docker docker-compose qemu-full virt-manager \
    archiso arch-install-scripts ostree flatpak \
    kotlin java-runtime-common gradle \
    rust go nodejs npm python python-pip \
    htop btop neofetch \
    snapper snap-pac grub-btrfs

# Generate fstab
echo -e "${YELLOW}Generating fstab...${NC}"
genfstab -U /mnt >> /mnt/etc/fstab

# Chroot and configure system
echo -e "${YELLOW}Configuring system...${NC}"
arch-chroot /mnt /bin/bash <<EOF
# Set timezone
ln -sf /usr/share/zoneinfo/${TIMEZONE} /etc/localtime
hwclock --systohc

# Locale
echo "en_US.UTF-8 UTF-8" > /etc/locale.gen
locale-gen
echo "LANG=en_US.UTF-8" > /etc/locale.conf

# Hostname
echo "${HOSTNAME}" > /etc/hostname
cat > /etc/hosts <<HOSTS
127.0.0.1   localhost
::1         localhost
127.0.1.1   ${HOSTNAME}.localdomain ${HOSTNAME}
HOSTS

# mkinitcpio with btrfs
sed -i 's/^HOOKS=.*/HOOKS=(base udev autodetect modconf kms keyboard keymap consolefont block btrfs filesystems fsck)/' /etc/mkinitcpio.conf
mkinitcpio -P

# Install bootloader
grub-install --target=x86_64-efi --efi-directory=/boot --bootloader-id=HorizonOS-Dev
grub-mkconfig -o /boot/grub/grub.cfg

# Set root password
echo "root:${USER_PASSWORD}" | chpasswd

# Create user with fish shell
echo "Creating user: yousef"
useradd -m -G wheel,docker,libvirt -s /usr/bin/fish ${USERNAME}
echo "${USERNAME}:${USER_PASSWORD}" | chpasswd

# Configure sudoers
echo "%wheel ALL=(ALL:ALL) ALL" > /etc/sudoers.d/wheel

# Enable services
systemctl enable NetworkManager
systemctl enable sddm
systemctl enable docker
systemctl enable libvirtd
systemctl enable sshd

# Configure snapper
snapper -c root create-config /
snapper -c home create-config /home
snapper -c root set-config "TIMELINE_CREATE=yes"
snapper -c root set-config "TIMELINE_CLEANUP=yes"
snapper -c root set-config "TIMELINE_LIMIT_HOURLY=5"
snapper -c root set-config "TIMELINE_LIMIT_DAILY=7"
snapper -c root set-config "TIMELINE_LIMIT_WEEKLY=0"
systemctl enable snapper-timeline.timer
systemctl enable snapper-cleanup.timer
systemctl enable grub-btrfsd

# Make fish default for new users
echo "/usr/bin/fish" >> /etc/shells
sed -i 's|SHELL=/bin/bash|SHELL=/usr/bin/fish|' /etc/default/useradd

# Configure SDDM for autologin (optional, remove for production)
mkdir -p /etc/sddm.conf.d
cat > /etc/sddm.conf.d/autologin.conf <<SDDM
[Autologin]
User=${USERNAME}
Session=plasma
SDDM
EOF

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
yay -S --noconfirm visual-studio-code-bin intellij-idea-ultimate-edition

echo ""
echo "Note: IntelliJ IDEA Ultimate requires a license."
echo "You can use the 30-day trial or enter your license key."
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

echo ""
echo -e "${GREEN}=====================================${NC}"
echo -e "${GREEN}Installation Complete!${NC}"
echo -e "${GREEN}=====================================${NC}"
echo ""
echo "System will reboot in 10 seconds..."
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

sleep 10
reboot