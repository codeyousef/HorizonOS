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

New disks on Btrfs also bind the native filesystem UUID, subvolume ID and
subvolume UUID. The kernel's device number can change after a host reboot;
the persistent binding still requires the original disk inode and owner.
Other filesystems retain the device/inode check. A legacy Btrfs record without
this binding stops on a changed device number. An operator can acknowledge
the exact retained disk and current filesystem identity while the VM is stopped:

```fish
python3 tools/devctl.py vm rebind-storage --previous-device OLD_DEVICE --previous-inode ORIGINAL_INODE --filesystem-uuid FILESYSTEM_UUID --subvolume-uuid SUBVOLUME_UUID --json
```

This requires the original configuration, provisioning authority, media hashes,
disk inode and owner to match. It rejects retained VM control state and preserves
the original record in a private receipt. It leaves disk bytes, SSH trust and
guest enrollment intact. Subsequent operations still verify native guest identity;
the storage receipt alone does not establish a guest target. Existing desktop and
snapshot records keep their original device/inode binding to the provisioning
record; persistent filesystem identity is checked when loading that record.

Fresh provisioning installs the pinned `nixosConfigurations.aios-dev` image,
including the desktop, installed target/template/approval records and the narrow
VM-only developer helper. Enrollment is generated from the verified VM and
installation UUIDs and the dedicated public key; seed source cannot provide its
own enrollment file. The installer builds in the target virtual disk, keeps both
locks unchanged, uses pure evaluation and disables import-from-derivation. This
is initial installation after fresh-disk checks. Updates use the separate guarded
developer deployment contract. Existing minimal bootstrap guests retain their
current configuration until an explicitly supported recovery or deployment.

Initial setup installs a fixed development preflight probe for the first boot.
Its root-owned systemd unit invokes only the installed executor's native startup
checks and writes a read-only result under `/run/aios-initial-preflight`. It accepts
no requests, confirms no user intent and applies no product effects. A successful
preflight still reports that the runtime adapter is unavailable. This probe is
initial-image verification instrumentation, not the product executor service.

QEMU runs as the current user with KVM, CPU host, virtio devices, local GTK,
private OVMF VARS, SMBIOS UUID, `AIOS_DEV_ROOT` serial, local Unix control sockets
and IPv4 loopback port forwarding. QMP control verifies PID/start time, user,
executable, exact arguments, peer credentials, UUID and root disk. `vm stop`
powers off that exact VM through QMP; finish/unmount the installer first.
No process-name kill, host mount, agent forwarding or remote display is used.
For an installed guest, `vm stop --graceful --json` requests ACPI shutdown and
waits for the exact recorded process to exit. A 60-second timeout retains control
state and returns failure; it never escalates to force-off. Use a clean shutdown
before offline audits and cold snapshots.

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
in a mode-0600 guest root file outside the Nix store. SSH is key-only for `dev`,
root login is disabled, and only root is Nix-trusted.

The installed M1 service access plan is:

| Unit | Identity and activation | Permitted persistent writes |
| --- | --- | --- |
| `aios-state.service` / `aios-reconcile.service` | `aios-state`; boot plus timer | `/var/lib/aios/state` only |
| `aios-execd.service` | root; system bus and multi-user target | `/var/lib/aios/candidates` and `/var/lib/aios/transactions` |
| `aios-model.socket` / `.service` | root-owned group socket; isolated `aios-model` on demand | none; model artifact is read-only |
| `aios-sessiond.service` | authenticated non-root user; default user target | private `%t/aios` runtime state |
| `aios-processd.service` | authenticated non-root user; default user target | private runtime state only |
| `aios-ui-agent.service` | authenticated non-root user; graphical session only | private runtime state only |

The development image keeps model activation disabled while deterministic
system and user APIs remain enabled. Index, automation, and recovery daemons are
explicitly disabled until their owning milestones install them; reserved
service accounts do not imply a running service.

The disposable `aios-desktop-test` image additionally contains a fixed lifecycle
qualification path unit. A mode-0600 correlation marker owned by `dev` can
trigger only the installed no-argument verifier. The test-only root unit retains
only DAC-read/search and UID/GID-switch capabilities so it can read that marker,
enter the `tester` identity, and address the tester user manager. It restarts
the fixed AIOS system and tester user units, proves SSH/network/display
independence, terminates the tester login, and publishes a bounded public report
under `/run`. The path unit and verifier are absent from `aios-dev` and
production composition; they accept no unit, command, path, or argument from
the requester.

The host captures the **public** host key and SHA256 fingerprint through a serial
connection tied to the exact QEMU PID/UID, after checking QMP UUID and root disk.
Console operations accept registered bootstrap/finish actions, not caller shell
strings or arbitrary keystrokes. Completion requires an installer exit marker;
delivery alone is not success. Evidence and receipts stay in private `.local`
paths under the checkout. Console pinning checks media/seed/target identity,
evidence digest, public key encoding and matching fingerprint before writing the
private known_hosts file. Host-key scans alone do not establish trust.
For a deliberately replaced installation, run full re-enrollment with the exact
prior installation UUID; ordinary `enroll` never accepts a changed key/target:

```fish
python3 tools/devctl.py enroll --re-enroll PRIOR_INSTALLATION_UUID --json
```

Managed QEMU uses the new successful verified bootstrap console receipt. An
external provider also requires `--trust-file .local/ssh/reinstalled-console.json`,
using the same explicit console material schema as initial adoption. The new
installation UUID must differ. Re-enrollment does not install or erase a disk;
destructive reinstallation is a separately authorized operation. Configure the
new SSH endpoint/key and optional expected UUIDs explicitly before re-enrollment.
The dedicated private key stays on the host. The tool authenticates the new fixed
identity endpoint with staged console-pinned trust before replacing active files.
It preserves prior trust/identity evidence and rejects old deployment intents
through their exact installation/boot/key/configuration bindings.

An interrupted multi-file trust update blocks normal guest operations. Resume
the same command with unchanged console material and prior UUID; the staged
receipt and hashes prevent an unrelated update from replacing it. No old
approval, task grant or deployment authorization is restored.

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

Build and unit-test commands synchronize source, recheck identity, and launch a
registered worker with a durable UUID job record under the guest dev home. The
worker detaches from SSH, keeps exact command/status/source/lock/identity
evidence, bounds command output and deadlines, and sanitizes exported logs.
`--detach` returns after submission; `jobs status --job <uuid>` reads the durable
record. `jobs cancel --job <uuid>` verifies the worker PID/start time and sends a
signal through its process descriptor. The worker cancels only its own child
process group. A disconnected or timed-out host wait leaves the job tracked.

`lock` generates real Nix and Cargo locks in a separate writable guest copy;
`artifacts pull --job <uuid>` copies reports, readable summaries, sanitized logs
and generated public locks into a fresh ignored `.local/reports` directory.
Lock adoption into the host checkout requires matching artifact hashes and
unchanged flake/Cargo source. Ordinary builds require those locks and refuse
to update them. The host never runs Nix or Cargo.

`build --target packages` selects all five required non-image AIOS packages;
missing outputs fail rather than silently reducing the target. An explicitly
selected package such as `--package aios-dev-tools` supports an upstream smoke
build. `build --target system` prepares the enrolled development candidate and
builds `nixosConfigurations.aios-dev` without activation. See the system candidate
contract below. `test --suite unit` runs locked Rust workspace tests in the guest
Nix development shell and host-tool Python fixtures in the guest. Their results
do not establish provider or model acceptance. Integration/desktop, benchmark,
deployment and journal operations remain unsupported until their registered
providers exist. The fixed 30-second `jobs probe` is labeled as a supervision
fixture for disconnect/cancellation checks; it is never a CPU benchmark.

`test --suite integration --bootstrap-case all` explicitly selects the disposable
installer guard qualification. A single case may be selected with `wrong-disk`,
`wrong-dmi`, `wrong-authorization` or `reinstall`. The host verifies the enrolled
development guest, copies only public source into isolated workspaces under
`.local/a`, and launches fresh headless KVM guests with separate disks, firmware,
UUIDs, keys, ports and control sockets. Registered console actions verify the
official ISO/seed and actual guest DMI, virtualization and virtio disk before
running the real installer denial cases. Whole-disk SHA256 digests before and
after a denial must match. Reinstall uses synthetic existing Btrfs data on its
fresh disposable disk. Reports label this scope explicitly; default integration
and desktop suites still require their full providers. Each disposable VM stops
through its verified QMP endpoint, retaining its disk and evidence for review.
No host Nix driver, nested KVM, host mounts or caller-supplied guest commands are
used, and the development VM remains running.

For a bootstrap access failure, the host can return to the verified installer and
run `vm console --bootstrap-audit`: it mounts the installed subvolumes read-only,
checks installation/role identity, and inspects public key metadata/account status
and SSH journal evidence. `--bootstrap-repair-access` checks both UUIDs, virtio
disk/serial, role, pinned public host key and private-key ownership/mode before
making the SSH directory traversable and rebuilding the reviewed bootstrap
configuration for next boot. It never repartitions or formats the existing disk.
Private host-key files remain mode 0600; the shared SSH directory is mode 0755
so `dev` can read its public authorized-key file.

The registered audit additionally checks GPT GUID/types/labels, the 1 GiB EFI
partition, Btrfs installation UUID and all four subvolumes. Read-only Btrfs mounts
disable tree-log replay. Before the installed Python auditor runs, the wrapper
checks the installed role and pinned public SSH fingerprint. Structured evidence
checks `dev`/`tester` groups, locked service accounts with nologin shells, root-only
Nix trust, sandboxing, SSH forwarding/login restrictions, fstab subvolumes and
private EFI masks. Credential evidence contains only file ownership, mode and
outside-store location; tester secret bytes and password hashes never leave the
guest. This verifies bootstrap storage/accounts, not production modules or the
running product services.

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

On the first `vm create`, an unconfigured fresh workspace measures host resources
and freezes its selection in `.local/vm.json` before issuing a provisioning UUID.
It caps the baseline at 8 vCPUs, 16 GiB RAM and a 96 GiB sparse disk; CPU allocation
uses at most half the logical CPUs where possible. RAM leaves at least 2 GiB or
one quarter of currently available memory for the host, whichever is larger;
storage leaves 8 GiB. Reduced development allocations require at least 4 GiB
guest RAM and a 48 GiB disk. These are provisioning limits, not measured inference
minimums. Discovery and the choice are recorded in `.local/provisioning-resources.json`.
Explicit local configuration and existing VM plans are preserved. Missing or
insufficient measured resources fail before creating a disk or key. Production
module assertions exclude graphical/console acceptance autologin and the reserved
`dev`/`tester` accounts.

When using the packaged `devctl`, specify `--workspace /path/to/checkout` before
the command; the package's own Nix store path is not a writable VM workspace.

## Enrolled development system candidate

The installed VM-only helper can register an immutable source snapshot through
the pinned host transport:

```sh
python3 tools/devctl.py deploy --mode register --acknowledge-guest-root --json
python3 tools/devctl.py deploy --mode status --transaction <returned-uuid> --json
```

Developer-supplied Nix code is guest-root authority. The explicit acknowledgement
applies to registration and to test/commit requests. This route accepts typed
operations and UUIDs only; it invokes the fixed installed helper through its
development-only sudo rule without a password prompt. It never transfers a
private key or runs a caller-provided command. The host saves transaction intent
before calling the helper, validates the root receipt against the published
manifest and installed helper source hash, and rechecks identity after the call.
Retry registration with the same UUID or inspect status after a disconnect;
changed boot, closure, configuration or target requires a fresh transaction.
Registration retains a separate root-owned readonly source copy and a durable
receipt. It does not build or activate that source. `deploy --mode test` and
`deploy --mode commit` with the transaction UUID and acknowledgement currently
return `GUARDED_ACTIVATION_UNAVAILABLE` until independent recovery is qualified.

`nixosConfigurations.aios-dev` uses the administrator-owned development machine
module. It describes the enrolled Btrfs subvolumes/EFI layout, key-only SSH,
non-wheel developer account, root-only Nix trust, Plasma 6 on Wayland and the
packaged CLI/session/model/guard executables. The VM-only developer helper is
enabled with the exact installation and DMI identities. Its guarded test/commit
operations remain unavailable until the activation adapter is qualified. The
incomplete product control plane and model services remain disabled.

The registered system-build job captures the already enrolled guest identity and
administrator-owned public SSH enrollment key. It copies the verified source
into its private job directory, adds `nix/machines/aios-dev/enrollment.json`, hashes
the complete candidate and makes all files/directories read-only before Nix
evaluation. The generated enrollment file is installation-local and ignored by
Git. A source tree cannot override it. The flake reads only data inside this
frozen candidate; it never reads live `/etc`, host credentials or mutable
`/var/lib` during evaluation. Direct builds without enrollment fail explicitly.
The subprocess ignores user Nix configuration, uses the system daemon, requires
pure evaluation, disallows import-from-derivation and selects the approved
`cache.nixos.org` substituter. Neither lock can be updated or rewritten.

The job checks an 8 GiB store recovery reserve before and after building, adds development-user GC roots
for the prior running/profile/booted closures and roots the exact build output.
It checks built enrollment, both unchanged locks, source integrity, current
identity and unchanged running/profile/booted pointers after the build. Reports
include candidate/source/enrollment digests, exact subprocess exits and an
inventory-based closure diff with NAR sizes. NAR size is not a download estimate
or a filesystem-space guarantee. Booted state is distinct from the selected boot
default; the unprivileged job does not claim to inspect protected boot metadata.

This is the development build route. Production still requires the Rust
`aios-buildd` worker, root broker registration/retention, trusted template/catalog
validation, semantic previews and enforced resource/download permissions. A
successful build does not prove boot, graphical login, service isolation or
rollback and does not authorize activation of a client-provided store path.

```fish
cd /mnt/Storage/Projects/HorizonOS
python3 tools/devctl.py build --target system --detach --json
python3 tools/devctl.py jobs status --job JOB_UUID --json
python3 tools/devctl.py artifacts pull --job JOB_UUID --json
```

### Disposable desktop runner

`test --suite desktop` creates a separate project-owned KVM guest below
`.local/d/<run-prefix>` and stores reports below `.local/reports/<run-uuid>`.
The dedicated `aios-desktop-test` image imports the enrolled development base
and enables a synthetic tester Wayland autologin with no wheel membership.
The normal development image keeps autologin disabled; production assertions
reject acceptance autologin. No host private directory is mounted in the VM.

The host controls the verified official installer, pins its console-published
SSH key, boots the installed image, verifies installation/DMI/disk/management
identity, and checks one active local tester Wayland session with live KWin and
Plasma processes. It captures the synthetic desktop through QMP and requests
the scoped QMP stop for that owned disposable VM. The read-only probe is fixed
public source sent over pinned SSH, so corrected probes can inspect a retained
image without reinstalling it. Host-controller and image-source digests are
reported separately. Disposable QMP stop is not evidence of graceful KDE logout.
The report qualifies this runner and base desktop only;
product application actions and AI functionality have their own acceptance gates.

The image can be built without activation in the verified development guest:

```fish
python3 tools/devctl.py build --target desktop-test --detach --json
python3 tools/devctl.py test --suite desktop --json
python3 tools/devctl.py test --suite desktop --desktop-run RUN_UUID --json
```

The resume command accepts only its registered workspace, source manifest and
disk identity. Completed installer/setup receipts are reused. An attempted
installation without a success receipt stops for inspection and never formats
again, resets a disk or silently starts another guest. Evidence and VM images
remain under `/mnt/Storage`; the runner requires a managed owner there.
