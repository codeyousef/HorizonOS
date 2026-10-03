# Horizon OS activation guard

`aios-guard` defines the deterministic transaction protocol for Horizon OS test
activation, exact-pointer commit and recovery. The library uses SQLite for the
immutable plan and effect journal. Its executable exposes the pure
`--check-plan` stdin interface and fixed root-only preflight below. Other invocations return exit 9 with
`GUARD_RUNTIME_ADAPTER_UNAVAILABLE`.

The fixed root-only `--native-preflight` mode captures installed target identity,
running and profile closures, the selected systemd-boot generation and its EFI
payloads, the installed guard, Nix and systemd executables, and `CLOCK_BOOTTIME`.
Native target checks are reused as a library; this does not depend on the broker
daemon or the model. Store fingerprints use bounded streaming reads, protected
traversal, root ownership, readonly files and before/after inode/content metadata.
The running executable must match the installed guard by path, digest and inode.

The boot reader supports the locked development image's UEFI/systemd-boot layout.
It requires one exact generation, one immutable system init, and one kernel and
initrd entry. EFI kernel/initrd bytes must match the selected closure. Ambiguous
generation patterns, malformed EFI variables, a pending one-shot entry or a
conflicting firmware default fail intake. Running state and the system profile
are recorded independently of boot selection. This proves the selected loader
entry; it does not promise recovery from firmware or kernel failure.

`NativeIntake` is not deserializable. Its artifact-binding method compares the
actual prior pointers, retained guard and candidate file hashes with a typed
plan. It provides no authorization, build provenance, durable plan registration,
GC retention, health approval, armed process or activation effect. Model-only
runtime intake is still unavailable. These remaining checks must precede any
use of the activation adapter.

Fresh development image instrumentation records the actual installed root guard
preflight in a protected RAM file. The registered `installed-guard` observer
checks root ownership, executable hash, boot/installation identity and EFI
payload agreement before accepting that proof. Nonroot invocation is denied.

Native health intake authenticates the fixed system bus owner as root PID 1,
matches its executable and inode to the running closure's immutable systemd,
and reads only the seven fixed protected units with `GetUnit` and fresh property
queries. It never loads, starts or changes a unit. Two captures must agree on
load/active/substate, invocation and queued job. Missing units are explicit;
transitions and queued jobs do not count as active. The fixed kernel mount
inventory comes from PID 1's namespace, including exact Btrfs subvolumes and
EFI device ancestry, so the observer's readonly service sandbox is not mistaken
for the guest's mount state. Target, manager and mount identities are rechecked.

The opaque native health capability compares previously active and failed units
separately: a pre-existing failure can remain, but a new failure or disappearance
of a previously active protected unit fails. Fresh comparisons expire after one
second. Core bus/mount/SSH availability does not prove product API, user-service,
action postcondition or authenticated host heartbeat health; those remain
explicitly unverified and cannot authorize activation. The reader is based on
the pinned [systemd D-Bus interface](https://github.com/systemd/systemd/blob/v260/man/org.freedesktop.systemd1.xml)
and NixOS [system closure layout](https://github.com/NixOS/nixpkgs/blob/774debe7a0d1b496e35677ad955a1011c6ff74f3/nixos/modules/system/activation/top-level.nix).

## Plan and ledger contract

Version 1 plans bind the installation, DMI, machine and boot identities, guest
role, disk and management channel. They include source/candidate digests, the
pinned Nixpkgs revision, exact prior running/profile/boot closures, prior managed
manifest and optional model, candidate, retained guard digest, deadline and
health policy. Paths are syntactically checked Nix store roots; parsing does not
prove their existence, ownership, contents or build provenance.

Unknown fields, duplicate object fields, unsupported versions, ambiguous health
policies and payloads above 64 KiB are rejected. The digest hashes the validated
typed serialization; input object key order has no effect. It is a guard plan
digest, not a replacement for product approval or authenticated plan intake.

The ledger commits effect intent before calling the adapter. SQLite uses FULL
synchronization, a DELETE journal and immediate write transactions. State and
revision checks reject stale controllers. Only one unfinished transaction may
exist; `RECOVERY_REQUIRED` blocks subsequent transactions. Unknown and partial
ledger schemas fail rather than silently regenerating records. The root adapter
must securely open and validate the ledger and its parent directory.

## Activation and recovery order

The core checks target identity before each effect. Runtime adapters must repeat
identity, authority and artifact checks at the privileged operation boundary.

1. Verify the baseline health and exact prior pointers; retain the required
   closures and trusted guard; arm the deadline before activation.
2. Test the exact candidate. Fixed health checks require the system bus, required
   mounts, protected-unit baseline, product APIs and selected user services.
   Duplicate/contradictory observations fail. Existing failed protected units are
   assessed separately from newly failed units. Unlisted model drift during a
   system transaction cannot be committed.
3. Require a fresh transaction-specific heartbeat bound to target identity,
   plan/candidate digests and a volatile challenge. The transport must authenticate
   the host authority; matching payload fields or an open TCP port cannot do so.
4. Commit the exact system profile, boot default and managed manifest. Recheck
   health, deadline, heartbeat and guard identity; verify all pointers; persist
   `COMMITTED`; then disarm. Model-only transactions preserve system pointers.
5. On deadline or failed activation/commit, restore the exact prior running,
   profile, boot and manifest state. Model-only recovery restores the prior model
   independently. Failed or identity-mismatched recovery persists
   `RECOVERY_REQUIRED`. A restarted controller recovers rather than replaying an
   uncertain activation or restoring a lost challenge.

Kernel, initrd or boot adapter changes enter `AWAITING_REBOOT` without activation;
they require a separate explicit reboot workflow. This userspace protocol cannot
recover a hung kernel, failed disk or firmware fault.

## Pinned command descriptions

The activation module describes a fixed command set; it does not spawn commands.
It selects the exact approved closure's wrapped `switch-to-configuration test`,
the retained Nix package's `nix-env --profile /nix/var/nix/profiles/system --set`
with the approved closure, and that closure's `switch-to-configuration boot`.
Recovery selects the separately recorded prior closures. The environment is
cleared, including upstream activation bypass variables. No rebuild, mutable
configuration path, arbitrary Nix expression or client-provided argv is accepted.

The adapter targets Nixpkgs revision
`774debe7a0d1b496e35677ad955a1011c6ff74f3`. Its interface is grounded in the pinned
[switchable-system module](https://github.com/NixOS/nixpkgs/blob/774debe7a0d1b496e35677ad955a1011c6ff74f3/nixos/modules/system/activation/switchable-system.nix)
and [activation implementation](https://github.com/NixOS/nixpkgs/blob/774debe7a0d1b496e35677ad955a1011c6ff74f3/pkgs/by-name/sw/switch-to-configuration-ng/src/main.rs).

## Verification boundary

Rust tests use real SQLite files and a separate connection to witness durable
intent, with simulated effects, clocks, identities and health observations. They
cover commit order, controller crash, target drift, deadline/heartbeat failures,
separate prior pointers, model-only recovery, schema corruption and effect
failures. These tests establish library behavior, not live rollback.

The registered guest smoke builds the Nix package and exercises the actual
executable with explicitly labelled fixture plans. The following run through
`devctl`'s pinned guest identity gates:

```fish
cd /mnt/Storage/Projects/HorizonOS
python3 tools/devctl.py test --suite unit --detach --json
python3 tools/devctl.py test --suite integration --provider guard-state --detach --json
```

Live privileged activation remains unavailable. It requires secure frozen-plan
intake and authorization, retained artifacts/GC roots, the root effect adapter,
an independent retained guard process with real boot-time deadlines, authenticated
management heartbeats, real system/user/API/action observations, boot-default
proof and disposable activation/SSH-loss/guard-survival qualification. Production
or developer deployment must not enable live activation on the strength of
fixture success.
