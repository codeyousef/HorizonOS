#!/usr/bin/env bash
# Registered host console operation in the official installer, never on the host.
set -euo pipefail
export LC_ALL=C
seed=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
die() { printf 'BOOTSTRAP DENIED: %s\n' "$*" >&2; exit 4; }
mode=${1:-fresh}
[[ $# -le 1 && ( $mode == fresh || $mode == --resume-unformatted ) ]] || die 'Unknown bootstrap mode'
[[ $EUID == 0 ]] || die 'Run as root in the installer guest'
[[ $(sed -n 's/^ID=//p' /etc/os-release | tr -d '"') == nixos ]] || die 'OS is not NixOS'
virt=$(systemd-detect-virt --vm) || die 'Not a virtual machine'
[[ $virt == kvm || $virt == qemu ]] || die 'Not the expected QEMU/KVM environment'
cd -- "$seed"
sha256sum --check --strict manifest.sha256 || die 'Seed manifest failed'
guest_uuid=$(cat guest.uuid)
installation_uuid=$(cat installation.uuid)
serial=$(cat disk.serial)
image_target=aios-dev
[[ ! -e image.target ]] || image_target=$(cat image.target)
[[ $image_target == aios-dev || $image_target == aios-desktop-test || $image_target == aios-model-test ]] || die 'Unregistered image target'
[[ $guest_uuid =~ ^[0-9a-f]{8}(-[0-9a-f]{4}){3}-[0-9a-f]{12}$ ]] || die 'Invalid guest UUID'
[[ $installation_uuid =~ ^[0-9a-f]{8}(-[0-9a-f]{4}){3}-[0-9a-f]{12}$ ]] || die 'Invalid installation UUID'
[[ $serial == AIOS_DEV_ROOT ]] || die 'Unexpected disk serial'
[[ $(tr '[:upper:]' '[:lower:]' </sys/class/dmi/id/product_uuid) == "$guest_uuid" ]] || die 'DMI UUID mismatch'
[[ $(cat authorized.uuid) == "$guest_uuid" ]] || die 'No matching host provisioning authorization'
[[ -f source/flake.lock && -f source/Cargo.lock && -f source/nix/machines/aios-dev/default.nix ]] || die 'Pinned Minnerite image source is missing'
[[ ! -e source/nix/machines/aios-dev/enrollment.json ]] || die 'Seed source cannot override machine enrollment'
[[ $(wc -l <dev.pub) == 1 ]] || die 'Expected one public development key'
ssh-keygen -lf dev.pub -E sha256 >/dev/null || die 'Invalid public development key'
public_key=$(awk '{print $1 " " $2}' dev.pub)
[[ $public_key =~ ^ssh-ed25519\ [A-Za-z0-9+/=]+$ ]] || die 'Expected an Ed25519 development key'

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
if [[ $mode == fresh ]]; then
  [[ $(lsblk --list --noheadings --output NAME "$disk" | wc -l) == 1 ]] || die 'Disk has partitions; reinstall requires separate destructive authorization'
  [[ -z $(wipefs --no-act --noheadings --output TYPE "$disk") ]] || die 'Disk has existing signatures; reinstall is forbidden'
else
  # Recovery is limited to our interrupted GPT creation, before ANY filesystem
  # exists. It cannot erase or reinstall an existing filesystem.
  observed_guid=$(sgdisk --print "$disk" | sed -n 's/^Disk identifier (GUID): //p' | tr '[:upper:]' '[:lower:]')
  [[ $observed_guid == "$guest_uuid" ]] || die 'Interrupted GPT is not owned by this provisioning operation'
  [[ $(lsblk --list --noheadings --output NAME "$disk" | wc -l) == 3 ]] || die 'Interrupted GPT must contain exactly two partitions'
  [[ $(lsblk --noheadings --output PARTLABEL "${disk}1" | xargs) == AIOS_DEV_EFI ]] || die 'Unexpected EFI partition label'
  [[ $(lsblk --noheadings --output PARTLABEL "${disk}2" | xargs) == AIOS_DEV_ROOT ]] || die 'Unexpected root partition label'
  [[ $(lsblk --bytes --noheadings --output SIZE "${disk}1" | xargs) == 1073741824 ]] || die 'Unexpected EFI partition size'
  [[ $(lsblk --noheadings --output PARTTYPE "${disk}1" | xargs) == c12a7328-f81f-11d2-ba4b-00a0c93ec93b ]] || die 'Unexpected EFI partition type'
  [[ $(lsblk --noheadings --output PARTTYPE "${disk}2" | xargs) == 0fc63daf-8483-4772-8e79-3d69d8477de4 ]] || die 'Unexpected root partition type'
  for partition in "${disk}1" "${disk}2"; do
    [[ -z $(wipefs --no-act --noheadings --output TYPE "$partition") ]] || die 'Recovery refuses any existing filesystem/signature'
    [[ -z $(findmnt --noheadings --raw --source "$partition" || true) ]] || die 'Recovery partition is mounted'
  done
fi
[[ -z $(findmnt --noheadings --raw --source "$disk" || true) ]] || die 'Disk is mounted'
[[ ! -e /mnt/etc/aios/installation-uuid ]] || die 'Existing mounted AIOS install'
! mountpoint -q /mnt || die 'Installer mountpoint is already in use'
printf 'Verified NixOS installer, virt=%s, DMI=%s, role=development, disk=%s, serial=%s\n' "$virt" "$guest_uuid" "$disk" "$serial"
printf 'Authorization: fresh virtual disk only; installation UUID=%s\n' "$installation_uuid"

if [[ $image_target == aios-model-test ]]; then
  [[ -f model-import.sh && ! -L model-import.sh && -d model ]] || die 'Verified model seed is missing'
  # Only initial provisioning reaches this data import. Reviewed hashes and
  # sizes are checked again inside the fixed generated import operation.
  bash ./model-import.sh --verify
else
  [[ ! -e model-import.sh && ! -e model ]] || die 'Unexpected model seed in a model-disabled image'
fi

# No destructive operation appears above the identity/fresh-disk checks.
if [[ $mode == fresh ]]; then
  sgdisk --clear --disk-guid="$guest_uuid" --new=1:0:+1G --typecode=1:ef00 --change-name=1:AIOS_DEV_EFI \
    --new=2:0:0 --typecode=2:8300 --change-name=2:AIOS_DEV_ROOT "$disk"
fi
udevadm settle
mkfs.fat -F 32 -n AIOS_EFI "${disk}1"
mkfs.btrfs -U "$installation_uuid" -L AIOS_DEV_ROOT "${disk}2"
mount "${disk}2" /mnt
for subvolume in @root @home @nix @var; do btrfs subvolume create "/mnt/$subvolume"; done
umount /mnt
mount -o subvol=@root,compress=zstd "${disk}2" /mnt
mkdir -p /mnt/{home,nix,var,boot}
for directory in home nix var; do mount -o "subvol=@$directory,compress=zstd" "${disk}2" "/mnt/$directory"; done
mount -o umask=0077 "${disk}1" /mnt/boot
nixos-generate-config --root /mnt
# Initial provisioning installs the same pinned full image used by system-build
# verification. This path is reachable only after fresh-disk identity checks;
# it is not an update/deployment route for an existing installation.
candidate=/mnt/etc/nixos/minnerite
[[ ! -e $candidate ]] || die 'Initial image candidate already exists'
mkdir "$candidate"
cp -r source/. "$candidate/"
chmod -R u+w "$candidate"
printf '{"authorized_key":"%s","disk_serial":"AIOS_DEV_ROOT","dmi_uuid":"%s","guest_role":"development","installation_uuid":"%s","management_channel":"ssh-development","schema_version":1}' \
  "$public_key" "$guest_uuid" "$installation_uuid" >"$candidate/nix/machines/aios-dev/enrollment.json"
if [[ $image_target == aios-model-test ]]; then
  # nixos-install builds in the mounted target store, not the live ISO store.
  # Import only the checked public data into that same fixed /mnt store.
  bash ./model-import.sh --import
fi
nixos-install --root /mnt --no-root-passwd --no-channel-copy \
  --flake "path:$candidate#$image_target" --no-update-lock-file --no-write-lock-file \
  --option pure-eval true --option allow-import-from-derivation false \
  --substituters https://cache.nixos.org --max-jobs 2 --cores 4
cmp source/flake.lock "$candidate/flake.lock" || die 'Installer changed the Nix lock'
cmp source/Cargo.lock "$candidate/Cargo.lock" || die 'Installer changed the Cargo lock'
nixos-enter --root /mnt -c 'set -eu
test -r /etc/aios/target-authority.json
test -r /etc/aios/template-authority.json
test -r /etc/aios/approval-authority.json
test -x /run/current-system/sw/bin/aios-dev-deploy
printf "AIOS_INITIAL_IMAGE=%s\n" "$(readlink -f /nix/var/nix/profiles/system)"
'
install -d -m 0755 /mnt/etc/ssh
ssh-keygen -q -t ed25519 -N '' -f /mnt/etc/ssh/ssh_host_ed25519_key
printf '\nInstallation finished. The host console workflow completes tester setup and unmounting.\n'
printf '\nSave this PUBLIC SSH host key and SHA256 fingerprint from the trusted console:\n'
cat /mnt/etc/ssh/ssh_host_ed25519_key.pub
ssh-keygen -lf /mnt/etc/ssh/ssh_host_ed25519_key.pub -E sha256
printf 'Expected guest UUID: %s\nExpected installation UUID: %s\n' "$guest_uuid" "$installation_uuid"
printf 'Host enrollment must pin this console key before installed startup and SSH operations.\n'
