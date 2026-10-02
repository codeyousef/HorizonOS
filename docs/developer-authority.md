# VM-only developer authority

Horizon OS separates host developer authority from product actions. The product
model cannot invoke `aios-dev-deploy`, obtain the dedicated developer SSH key,
change Nix modules or approve code deployment. Developer-supplied NixOS code is
effectively guest-root authority; a typed wrapper does not make that code
unprivileged.

The reusable module defaults `services.aios.development.enable` to false. When
explicitly enabled, it requires a development guest role, canonical
`expectedVmUuid` and `expectedInstallationUuid`, a normal `dev` account outside
wheel, and root-only Nix trusted users. It installs one exact sudo command for
`dev`: `/run/current-system/sw/bin/aios-dev-deploy --request-stdin`, as
`root:root`, with `NOPASSWD` and `NOSETENV`. No caller-selected executable or
shell argument is accepted. The Python interpreter runs with `-I`; installed
imports come from its immutable package. Product identities receive no sudo rule.

`nixosModules.production` rejects the enabled development boundary, the named
helper package or helper sudo rules, unrestricted passwordless sudo and extra
Nix trusted users. These are evaluation assertions; final production image and
service access qualification are separate requirements.

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
SSH management channel. It rechecks target identity before persisting a new
registration. A restored/rebooted/changed target cannot reuse the old request.

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
Git HEAD/dirty provenance, target identity, authenticated developer UID and helper
source hash. A repeated transaction ID with different intent is rejected.
Immutable root-owned store configuration may use Nix's legitimate hardlink
optimisation; developer-owned source files and the separately copied candidate
must have a single link. See the [Nix store optimisation contract](https://nix.dev/manual/nix/2.34/command-ref/nix-store/optimise.html).

## Implemented and remaining behavior

Registration does not evaluate, build or activate developer code. `test` and
`commit` currently return exit 9 with `GUARDED_ACTIVATION_UNAVAILABLE`, before
creating deployment state. They cannot fall back to unguarded `nixos-rebuild` or
an arbitrary store path. Exact candidate building, the retained independent guard,
fixed health checks, transaction-specific host heartbeat, recovery and exact
closure/profile/boot commit must be implemented and qualified before these modes
can succeed. The host deployment CLI remains unsupported until that adapter is
available. Existing bootstrap guests do not gain this helper automatically.

Run the registered guest qualification from the host, including fish:

```sh
python3 tools/devctl.py test --suite integration --provider development-boundary --detach --json
python3 tools/devctl.py jobs status --job <returned-uuid> --json
python3 tools/devctl.py artifacts pull --job <returned-uuid> --json
```

It evaluates development/disabled/production and invalid role/UUID/trust/sudo
cases with locked Nixpkgs, builds the actual helper package, and proves actual
non-root invocation and forged sudo/Python environments return authorization
denial. Source copy, corruption/race/replay/target checks use explicitly labeled
filesystem fixtures as the guest dev UID. These checks do not establish installed
root execution, activation/recovery, production boot or model isolation.
