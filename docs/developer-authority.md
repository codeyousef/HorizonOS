# VM-only developer authority

Minnerite separates host developer authority from product actions. The product
model cannot invoke `aios-dev-deploy`, obtain the dedicated developer SSH key,
change Nix modules or approve code deployment. Developer-supplied NixOS code is
effectively guest-root authority; a typed wrapper does not make that code
unprivileged.

New host-controlled guest jobs and developer deployments require at least 8 GiB
of measured free space on the workspace filesystem before publication or helper
delivery. Guest storage reserves are checked independently. This host preflight
is a minimum floor, not a quota or a concurrency reservation; status and recovery
observations remain available at low disk.

If QEMU pauses on root-disk no-space, `devctl vm resume-storage` requires restored
host headroom, exact recorded process/peer/UUID/storage identity, and that specific
pause reason. It resumes the existing process without restarting or changing its
disk. Guest SSH identity must be freshly verified afterwards. Resume cannot
convert a failed deployment or guard into a passing receipt.

The reusable module defaults `services.aios.development.enable` to false. When
explicitly enabled, it requires a development guest role, canonical
`expectedVmUuid` and `expectedInstallationUuid`, a normal `dev` account outside
wheel, and root-only Nix trusted users. It installs one exact sudo command for
`dev`: `/run/current-system/sw/bin/aios-dev-deploy --request-stdin`, as
`root:root`, with `NOPASSWD` and `NOSETENV`. No caller-selected executable or
shell argument is accepted. The Python interpreter runs with `-I`; installed
imports come from its immutable package. Product identities receive no sudo rule.

`nixosModules.production` rejects the enabled development boundary, the named
helper package in either the executable or systemd package sets, helper sudo
rules, unrestricted passwordless sudo and extra Nix trusted users. The developer
guard additionally requires the enrolled `development` role, `AIOS_DEV_ROOT`
disk and `ssh-development` management channel at runtime.

## Request contract

The helper accepts one UTF-8 JSON object on standard input, at most 64 KiB, with
no duplicate/unknown fields or non-finite values. The only command-line argument
is `--request-stdin`. Every request has `schema_version: 1`, an `operation`, a
canonical UUID `transaction_id` and the full currently verified guest `identity`.

`register`, `test` and `commit` additionally require a lowercase SHA256
`snapshot_digest` and `authority: "guest-root-code-deployment"`. `status` has
neither field. There are no source paths, Nix attributes, shell strings, user
identities or deployment targets supplied by the client. Root checks the actual
sudo caller, installed immutable configuration, QEMU/KVM virtualization,
actual DMI, NixOS identity,
installation UUID, current boot/closure, development role, virtio disk serial and
SSH management channel. It rechecks target identity before every build, handoff,
guard start and durable state transition. A restored, rebooted or otherwise
changed target cannot reuse the old request.

Registration consumes only the already published digest directory under
`/home/dev/aios-releases`. It checks the canonical manifest, bounds, source hashes,
file modes/ownership/link counts and exact directory inventory. Each ancestor is
opened without following symlinks. Because a developer still owns that published
tree, the helper copies and hashes its bytes into a fresh root-owned tree before
atomic publication under `/var/lib/aios/development/releases`. It seals and
reverifies the copy. Later developer changes cannot change it; registration reuse
independently verifies the root copy. Failed unique staging directories are
removed, without deleting another candidate.

`/var/lib/aios/development` is mode 0700. Registration receipts live in a private
SQLite ledger using serialized writes and FULL synchronous commits; copied files
and publication directories are fsynced. Receipts bind transaction, source digest,
Git HEAD/dirty provenance, target identity, authenticated developer UID, helper
source hash, exact built closure and separate test/commit guard transaction IDs.
A repeated transaction ID with different intent is rejected.
Immutable root-owned store configuration may use Nix's legitimate hardlink
optimisation; developer-owned source files and the separately copied candidate
must have a single link. See the [Nix store optimisation contract](https://nix.dev/manual/nix/2.34/command-ref/nix-store/optimise.html).

## Guarded test and commit

Registration alone does not evaluate, build or activate developer code.
`deploy --mode test` first creates an installation-local, root-owned candidate
source from the registered snapshot, adds only the administrator-owned public
enrollment, and builds the fixed `aios-dev` system attribute. Nix runs with pure
evaluation, import-from-derivation disabled, fixed lock files, one build job and
only `cache.nixos.org`; the request cannot select an attribute, command, source
path, substituter or store path.

The helper writes a bounded root-owned handoff for a deterministic test guard
UUID. `aios-dev-guard@.service` invokes the retained `aios-guard` binary through
its dedicated developer entry point. The guard independently re-enrolls the
target, verifies the exact system closure and guard artifacts, retains rollback
roots, applies the candidate, checks fixed mount, unit, user-unit, Executor1 and
Graph1 health, then waits on a root-only Unix control socket. A successful test
deliberately rolls back the exact prior running, profile and boot selections and
records `TESTED`. A missing completion, helper/SSH loss or guard timeout also
rolls back.

`deploy --mode commit` is legal only for that `TESTED` receipt and unchanged
prior pointers. It does not rebuild or accept a new target. A separate guard
transaction test-applies the same closure, receives a nonce-bound heartbeat from
the still-running authenticated helper, commits exact running/profile/boot
pointers, and records the resulting identity as `COMMITTED`. Both developer and
product guards serialize through the same durable guard ledger, so two
privileged activations cannot overlap. Guard recovery after service restart
reconciles durable effects before another transaction can begin.

Status reconciles a failed independent guard to `REJECTED` or
`RECOVERY_REQUIRED`, preserving the frozen candidate and guard identifiers.
These receipts cannot be retried as test/commit and never qualify a candidate.
`RECOVERY_REQUIRED` continues to block privileged activation even when the
running system already matches the baseline; full recovery remains unproven.
Reading status does not restart a failed guard or clear its ledger.

The helper is intentionally guest-root development authority; none of these
operations is reachable through the product model, Executor1 action schema or
production module. Existing bootstrap guests gain the route only after installing
a candidate that contains the helper and developer guard unit.

Run installed denial qualification from the host:

```sh
python3 tools/devctl.py test --suite integration --provider development-boundary --detach --json
python3 tools/devctl.py jobs status --job <returned-uuid> --json
python3 tools/devctl.py artifacts pull --job <returned-uuid> --json
```

Run the end-to-end deployment sequence only against a disposable enrolled
development VM:

```sh
python3 tools/devctl.py deploy --mode register --acknowledge-guest-root --json
python3 tools/devctl.py deploy --mode test --transaction <uuid> --acknowledge-guest-root --json
python3 tools/devctl.py deploy --mode commit --transaction <uuid> --acknowledge-guest-root --json
```

The development-boundary provider evaluates enabled, disabled, production and
invalid role/UUID/trust/sudo cases with locked Nixpkgs and exercises the actual
installed denial surface. The deployment commands provide separate real build,
test rollback and exact commit evidence.
