# Building HorizonOS ISO

The ISO build system is now ready. All prerequisites are satisfied:

## ✅ Prerequisites Verified
- archiso is installed
- OSTree repository exists with commits
- All required tools are available
- Configuration file is in place
- Scripts have correct paths

## Build Instructions

To build the ISO, you need to run with sudo:

```bash
sudo ./scripts/scripts/build-iso.sh
```

This will:
1. Create an archiso profile based on Arch's releng profile
2. Customize it for HorizonOS with OSTree support
3. Include the OSTree repository in the ISO (if small enough)
4. Create the `horizonos-install` installer script
5. Build the bootable ISO

## Expected Output
The ISO will be created in `build/out/horizonos-*.iso`

## Testing the ISO

Once built, test with QEMU:
```bash
qemu-system-x86_64 -m 4G -enable-kvm -cdrom build/out/horizonos-*.iso
```

Or write to USB:
```bash
sudo dd if=build/out/horizonos-*.iso of=/dev/sdX bs=4M status=progress
```

## What the Installer Does

The `horizonos-install` script in the live environment will:
1. Partition the disk (EFI + Btrfs)
2. Create Btrfs subvolumes (@, @home, @var, @snapshots)
3. Initialize OSTree on the target system
4. Deploy the HorizonOS commit from the ISO
5. Install and configure GRUB for OSTree booting
6. Generate fstab

## Known Issues to Fix

1. **GRUB Configuration**: The installer uses `grub-mkconfig` which won't properly handle OSTree deployments. This needs to be replaced with OSTree-aware GRUB configuration.

2. **Missing arch-chroot**: The installer tries to use `arch-chroot` but it needs to be included in packages.x86_64

3. **OSTree Boot Configuration**: Need to add proper kernel arguments for OSTree booting

## Next Steps

After building and testing the ISO:
1. Fix any boot issues in QEMU
2. Update the installer for proper OSTree deployment
3. Test the Kotlin DSL integration
4. Add more system packages as needed