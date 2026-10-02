# Development VM bootstrap

Run host commands from the checkout in any shell, including fish:

```sh
python3 tools/devctl.py doctor --host --json
python3 tools/devctl.py vm create --json
python3 tools/devctl.py vm create --authorize-provision <printed-guest-uuid> --json
python3 tools/devctl.py vm start --bootstrap --display gtk --json
python3 tools/devctl.py vm console --json
```

The first create command records a private plan and returns exit 5. The second
binds authorization to that VM UUID/configuration and prepares a **fresh** 96 GiB
sparse virtual disk. Existing disks/NVRAM, reused keys and changed plans are
refused. Files remain beneath private `.local` directories. No host package,
network bridge, boot setting or virtualization permission is changed.

The installer is an official NixOS 26.05 minimal ISO. HTTPS checksum provenance,
verified SHA256, firmware hash, seed hash, VM UUID and disk inode/device are
recorded in ignored `.local/provisioning.json`. Source is taken from reviewed
tracked regular files; credentials, special files and symlinks are excluded or
refused. The read-only seed contains public source, UUIDs, disk serial, explicit
fresh-disk authorization and the dedicated public SSH key. The private key stays
on the host.

QEMU runs as the current user with KVM, CPU host, virtio devices, local GTK,
private OVMF VARS, SMBIOS UUID, `AIOS_DEV_ROOT` serial, local Unix control sockets
and IPv4 loopback port forwarding. QMP control verifies PID/start time, user,
executable, exact arguments, peer credentials, UUID and root disk. `vm stop`
powers off that exact VM through QMP; finish/unmount the installer first.
No process-name kill, host mount, agent forwarding or remote display is used.

The initial bootstrap starts a known installer; it does not establish an enrolled
SSH target. Use its local graphical console for these commands:

```sh
sudo mkdir -p /run/aios-seed
sudo mount -o ro /dev/disk/by-label/AIOS_SEED /run/aios-seed
sudo bash /run/aios-seed/bootstrap.sh
sudo nixos-enter --root /mnt -c 'passwd tester'
sudo umount -R /mnt
```

Before partitioning, the script checks NixOS, KVM/QEMU, the exact DMI UUID,
read-only seed manifest, authorization, exactly one expected virtio disk, absence
of partitions/signatures/mounts and a free installer mountpoint. It refuses all
reinstallation. It creates GPT EFI/Btrfs with `@root`, `@home`, `@nix`, `@var`.
`dev` is not wheel or Nix-trusted; `tester` is wheel and gets a password only via
the local console. Service accounts have no enabled AIOS services yet. SSH is
key-only for `dev`, root login is disabled, and only root is Nix-trusted.

Save the **public** host key and SHA256 fingerprint printed by the console. They
must be pinned by the enrollment workflow before any SSH operation. Never copy
or paste a private key or password. Host-key scans alone do not establish trust.
After enrollment, each guest operation checks NixOS, installation/DMI UUIDs and
role; mutations additionally check disk and management identity. No remote guest
command is implemented by the bootstrap provider. Rust/Nix/OS verification must
run through that verified guest workflow.

When using the packaged `devctl`, specify `--workspace /path/to/checkout` before
the command; the package's own Nix store path is not a writable VM workspace.
