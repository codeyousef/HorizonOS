# Development VM bootstrap

Run host commands from the checkout in any shell, including fish:

```sh
python3 tools/devctl.py doctor --host --json
python3 tools/devctl.py vm create --json
python3 tools/devctl.py vm create --authorize-provision <printed-guest-uuid> --json
python3 tools/devctl.py vm start --bootstrap --display gtk --json
python3 tools/devctl.py vm console --capture --json
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
SSH target. Inspect the captured screen and wait for the NixOS installer shell
before submitting the registered console bootstrap. All commands below run on
the host, including when its shell is fish. No manual guest sudo step is needed:

```sh
python3 tools/devctl.py vm console --bootstrap-run --json
python3 tools/devctl.py enroll --pin-console-only --json
python3 tools/devctl.py vm console --bootstrap-finish --json
python3 tools/devctl.py vm stop --json
python3 tools/devctl.py vm start --display gtk --json
python3 tools/devctl.py enroll --json
python3 tools/devctl.py doctor --guest --json
```

Before partitioning, the script checks NixOS, KVM/QEMU, the exact DMI UUID,
read-only seed manifest, authorization, exactly one expected virtio disk, absence
of partitions/signatures/mounts and a free installer mountpoint. It refuses all
reinstallation. Recovery with `--bootstrap-recover-unformatted` accepts only an
interrupted GPT owned by the same provisioning UUID, with exactly the expected
two partition labels/types and EFI size, no filesystems/signatures and no mounts.
It does not repartition or overwrite a filesystem. Public seed updates use
`vm create --refresh-seed` with the VM stopped; the disk and SSH key are preserved.
It creates GPT EFI/Btrfs with `@root`, `@home`, `@nix`, `@var`.
`dev` is not wheel or Nix-trusted; `tester` is wheel and gets a password only via
the root-only console finish operation, generated inside the guest and retained
in a mode-0600 guest root file outside the Nix store. Service accounts have no
enabled AIOS services yet. SSH is
key-only for `dev`, root login is disabled, and only root is Nix-trusted.

The host captures the **public** host key and SHA256 fingerprint through a serial
connection tied to the exact QEMU PID/UID, after checking QMP UUID and root disk.
Console operations accept registered bootstrap/finish actions, not caller shell
strings or arbitrary keystrokes. Completion requires an installer exit marker;
delivery alone is not success. Evidence and receipts stay in private `.local`
paths under the checkout. Console pinning checks media/seed/target identity,
evidence digest, public key encoding and matching fingerprint before writing the
private known_hosts file. Host-key scans alone do not establish trust.
After enrollment, each guest operation checks NixOS, installation/DMI UUIDs and
role; mutations additionally check disk and management identity. SSH exposes the
read-only identity endpoint and a registered unprivileged source receiver through
host tooling. Rust/Nix/OS verification must run through that verified workflow.

Publish source from the host with `python3 tools/devctl.py sync --json`. It takes
tracked files and Git's explicitly non-ignored new files, excludes private state,
credentials and generated artifacts, and rejects symlinks, special files and
unsafe modes. Public model metadata remains source; model weights are excluded.
The normalized manifest records relative paths, modes, sizes, SHA256 content
hashes, HEAD and the actual dirty state. A concurrent checkout change aborts
collection. Neither Git nor private keys are transferred into the guest.

The pinned SSH receiver rechecks the complete observed target identity before
creating a unique staging directory below the configured user's release root.
It validates all paths, content hashes and byte counts, then publishes atomically
to `<guest_source_root>/<snapshot-digest>` with read-only files/directories.
Existing releases must pass full verification before reuse; they are never
updated in place. Concurrent transfers use separate staging directories, and
cleanup only removes the directory created by that transfer. Build tooling must
verify the published source and use the returned `path:<release>#...` reference.
The source receipt proves publication, not a successful build or OS acceptance.

For a bootstrap access failure, the host can return to the verified installer and
run `vm console --bootstrap-audit`: it mounts the installed subvolumes read-only,
checks installation/role identity, and inspects public key metadata/account status
and SSH journal evidence. `--bootstrap-repair-access` checks both UUIDs, virtio
disk/serial, role, pinned public host key and private-key ownership/mode before
making the SSH directory traversable and rebuilding the reviewed bootstrap
configuration for next boot. It never repartitions or formats the existing disk.
Private host-key files remain mode 0600; the shared SSH directory is mode 0755
so `dev` can read its public authorized-key file.

External-provider adoption accepts `enroll --trust-file .local/ssh/console.json`
with an explicitly supplied, owned mode-0600 console record. Its exact schema is
`schema_version: 1`, `host_public_key`, matching SHA256 `fingerprint`, canonical
`guest_uuid` and `installation_uuid`, `guest_role`, `disk_serial` and
`management_channel: "ssh-development"`. A key scan cannot supply this trust.
External guests use the same pinned SSH identity checks; power and snapshot
commands return unsupported until a verified provider adapter exists.

Keep this checkout, VM disks, installer media, logs, caches, model data and build
artifacts under `/mnt/Storage`. Guest `/nix`, `/home`, `/var` and root data reside
on the virtual disk stored there; installer `/run` is guest runtime memory.

When using the packaged `devctl`, specify `--workspace /path/to/checkout` before
the command; the package's own Nix store path is not a writable VM workspace.
