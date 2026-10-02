#!/usr/bin/env bash
# Run manually in the official NixOS installer console, never on the host.
set -euo pipefail
export LC_ALL=C
seed=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
die() { printf 'BOOTSTRAP DENIED: %s\n' "$*" >&2; exit 4; }
[[ $EUID == 0 ]] || die 'Run as root in the installer guest'
[[ $(sed -n 's/^ID=//p' /etc/os-release | tr -d '"') == nixos ]] || die 'OS is not NixOS'
virt=$(systemd-detect-virt --vm) || die 'Not a virtual machine'
[[ $virt == kvm || $virt == qemu ]] || die 'Not the expected QEMU/KVM environment'
cd -- "$seed"
sha256sum --check --strict manifest.sha256 || die 'Seed manifest failed'
guest_uuid=$(cat guest.uuid)
installation_uuid=$(cat installation.uuid)
serial=$(cat disk.serial)
[[ $guest_uuid =~ ^[0-9a-f]{8}(-[0-9a-f]{4}){3}-[0-9a-f]{12}$ ]] || die 'Invalid guest UUID'
[[ $installation_uuid =~ ^[0-9a-f]{8}(-[0-9a-f]{4}){3}-[0-9a-f]{12}$ ]] || die 'Invalid installation UUID'
[[ $serial == AIOS_DEV_ROOT ]] || die 'Unexpected disk serial'
[[ $(tr '[:upper:]' '[:lower:]' </sys/class/dmi/id/product_uuid) == "$guest_uuid" ]] || die 'DMI UUID mismatch'
[[ $(cat authorized.uuid) == "$guest_uuid" ]] || die 'No matching host provisioning authorization'

disks=()
while read -r name observed type; do
  [[ $observed == "$serial" && $type == disk ]] && disks+=("$name")
done < <(lsblk --nodeps --noheadings --output NAME,SERIAL,TYPE)
[[ ${#disks[@]} == 1 ]] || die 'Expected exactly one disk with AIOS_DEV_ROOT serial'
name=${disks[0]}
[[ $name =~ ^vd[a-z]+$ ]] || die 'Target is not a virtio block disk'
disk=/dev/$name
[[ -b $disk ]] || die 'Target is not a block device'
device_path=$(readlink -f "/sys/class/block/$name/device")
[[ $device_path == /sys/devices/pci*/virtio* ]] || die 'Target is not a virtual virtio device'
[[ $(readlink -f "/sys/class/block/$name/device/driver") == /sys/bus/virtio/drivers/virtio_blk ]] || die 'Unexpected block driver'
[[ $(lsblk --list --noheadings --output NAME "$disk" | wc -l) == 1 ]] || die 'Disk has partitions; reinstall requires separate destructive authorization'
[[ -z $(wipefs --no-act --noheadings --output TYPE "$disk") ]] || die 'Disk has existing signatures; reinstall is forbidden'
[[ -z $(findmnt --noheadings --raw --source "$disk" || true) ]] || die 'Disk is mounted'
[[ ! -e /mnt/etc/aios/installation-uuid ]] || die 'Existing mounted AIOS install'
! mountpoint -q /mnt || die 'Installer mountpoint is already in use'
printf 'Verified NixOS installer, virt=%s, DMI=%s, role=development, disk=%s, serial=%s\n' "$virt" "$guest_uuid" "$disk" "$serial"
printf 'Authorization: fresh virtual disk only; installation UUID=%s\n' "$installation_uuid"

# No destructive operation appears above the identity/fresh-disk checks.
sgdisk --clear --disk-guid="$guest_uuid" --new=1:0:+1G --typecode=1:ef00 --change-name=1:AIOS_DEV_EFI \
  --new=2:0:0 --typecode=2:8300 --change-name=2:AIOS_DEV_ROOT "$disk"
udevadm settle
mkfs.fat -F 32 -n AIOS_DEV_EFI "${disk}1"
mkfs.btrfs -U "$installation_uuid" -L AIOS_DEV_ROOT "${disk}2"
mount "${disk}2" /mnt
for subvolume in @root @home @nix @var; do btrfs subvolume create "/mnt/$subvolume"; done
umount /mnt
mount -o subvol=@root,compress=zstd "${disk}2" /mnt
mkdir -p /mnt/{home,nix,var,boot}
for directory in home nix var; do mount -o "subvol=@$directory,compress=zstd" "${disk}2" "/mnt/$directory"; done
mount "${disk}1" /mnt/boot
nixos-generate-config --root /mnt
cp source/nix/machines/aios-dev/bootstrap.nix /mnt/etc/nixos/bootstrap.nix
cp source/tools/guest/identity.py /mnt/etc/nixos/identity.py
cp dev.pub /mnt/etc/nixos/dev.pub
printf '%s\n' "$guest_uuid" >/mnt/etc/nixos/guest.uuid
printf '%s\n' "$installation_uuid" >/mnt/etc/nixos/installation.uuid
cat >/mnt/etc/nixos/configuration.nix <<'NIX'
{ ... }: { imports = [ ./hardware-configuration.nix ./bootstrap.nix ]; }
NIX
nixos-install --root /mnt --no-root-passwd
install -d -m 0700 /mnt/etc/ssh
ssh-keygen -q -t ed25519 -N '' -f /mnt/etc/ssh/ssh_host_ed25519_key
printf '\nInstallation finished. Set tester password locally (never paste it into Codex):\n'
printf "sudo nixos-enter --root /mnt -c 'passwd tester'\n"
printf '\nSave this PUBLIC SSH host key and SHA256 fingerprint from the trusted console:\n'
cat /mnt/etc/ssh/ssh_host_ed25519_key.pub
ssh-keygen -lf /mnt/etc/ssh/ssh_host_ed25519_key.pub -E sha256
printf 'Expected guest UUID: %s\nExpected installation UUID: %s\n' "$guest_uuid" "$installation_uuid"
printf 'Then power off the installer. Host enrollment must pin this console key before SSH operations.\n'
