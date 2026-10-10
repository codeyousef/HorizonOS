# Minnerite managed state

The reviewed Rust `aios-state` compiler accepts data, never Nix expressions.
Its catalog comes from the installed administrator template. `Catalog::from_installed`
checks schemas, revisions and metadata hashes; the caller must establish the file's
trusted ownership and origin. A hash alone does not confer authority.

The initial reviewed catalog maps `blender`, `kate`, `kcalc` and `postgresql-17`
to fixed attributes in locked Nixpkgs. Generation extracts exact version, license,
free/unfree status, platform, binaries and desktop IDs, and binds each metadata
record and the entire catalog with SHA-256. Package mappings are template code;
clients select only IDs. All initial entries are free. Future unfree entries need
an exact per-ID permission acknowledgement; broad `allowUnfree` is denied.

The base revision binds the canonical path/mode/size/hash inventory of every
reviewed public template source file, including the machine modules, compiler,
canonicalizer, native runtime, service definitions and build tooling. Public
enrollment is separately bound by the template manifest. The catalog also binds
the lock hash, Nixpkgs revision, platform
and installation `system.stateVersion`. The initial baseline is `26.05`; updating
a package set does not change it. A root broker must bind these artifacts to its
installed template before accepting a build. This compiler is not that broker.

## Supported fields

`schema_version`, `base_template_revision` and `catalog_revision` are mandatory.
The compiler supplies the following omitted defaults. All structures reject
unknown and duplicate keys; integers, booleans and enums have strict types.
Runtime user settings, hardware, users, keys, storage, imports, overlays, options,
fetchers and policy are separate administrator/user domains, absent from this schema.

| Managed field | Default | Ownership and risk | Verification | Data and recovery |
| --- | --- | --- | --- | --- |
| `system_packages` | empty list | Catalog desktop IDs, system declaration, R2 | Exact closure membership and each binary/desktop capability | Removing a declaration preserves user data; retained dependencies need closure evidence; restore prior configuration |
| `services.postgresql.enabled` | false | Fixed service, R2 | Service health plus fixed `pg_isready` under postgres over `/run/postgresql` | Initial enable requires observed absent data; unknown/existing data requires separate review; new data may remain after rollback |
| `services.postgresql.package_id` | `postgresql-17` | Approved major only, R2 | Exact package provenance and readiness | No autonomous major migration; backup/migration/restoration is a separate reviewed plan |
| `services.postgresql.listen_mode` | `unix-only` | Public TCP is unsupported | Empty listen addresses, TCP disabled, peer-only local authentication and actual listener check | Restore exact prior configuration; preserve data |
| `services.openssh.enabled` | true | Protected management transport | Host-observed management heartbeat and SSH service | Disabling is rejected until a tested transport-safe product route exists |
| `services.openssh.open_firewall` | true | Protected independently from enablement | Firewall/port and host heartbeat | Closing is rejected; no implicit network-exposure approval |
| `power_policy.profile_on_ac` | `balanced` | `balanced`, `power-saver`, `performance`; R2 | Runtime support and actual applied profile | Store desired policy separately; runtime application and recovery executor remain to qualify |
| `power_policy.profile_on_battery` | `power-saver` | Same reviewed profile enum; R2 | Power-source transition and actual applied profile | Restore prior desired/applied profiles through the fixed executor |

Compilation normalizes package ordering and produces one materialized canonical
JSON representation using the protocol canonicalizer. Duplicate package IDs fail.
The fixed Nix module imports these bytes with `builtins.fromJSON`, checks the exact
full schema and revisions, and requires an identical canonical round trip. This
rejects duplicate keys and noncanonical raw input even when a JSON reader would
collapse duplicate keys. Managed data belongs inside the frozen candidate;
evaluation must never read live `/var/lib/aios` files.

## Preliminary previews

Typed intents install/remove catalog packages, toggle the fixed PostgreSQL or SSH
fields, or select power profiles. The compiler records exact before/after values,
manifest hashes, package changes, risk, validators and recovery limits. It does not
spawn commands, write state, evaluate Nix, inspect personal data or authorize changes.
Authenticated grants and database observations are separate trusted Rust values.
They cannot be inserted into a serialized intent as `approved`, `grants` or data
claims. Existing/unknown database data blocks initial enablement.

`aios-state-check` embeds the generated catalog when built by the pinned flake.
Its only interfaces are `--catalog`, `--defaults`, `--check-manifest` and `--preview`.
The latter accepts `{ "managed": <manifest>, "intent": <typed intent> }` on bounded
stdin, uses unknown database state and no grants, and labels the baseline as an
offline manifest. It is a read-only inspector, not the privileged Prepare/GetPlan API.

Previews explicitly leave candidate closure, retained dependency paths, reboot
requirement and build/download quantities unknown until authoritative build evidence
exists. They never claim installation, capability verification or final authorization.
The broker must prepare its own plan from actual intended/running/profile/boot-selected
state, build its registered candidate, and freeze the final plan before approval.

`aiosctl plan "Install Blender"` and `aiosctl plan "Remove Blender"` map only
that bounded package grammar to typed catalog intents. Executor1 binds prepared
plans to the originating UID, boot, system-bus instance and exact live logind
session. A later CLI process in that same session can inspect the plan, while
another user, session, boot or bus cannot. Client-supplied plan hashes are never
accepted. The returned UUID and digest identify the immutable plan.
`aiosctl transaction inspect|authorize|apply|cancel|rollback-plan UUID` obtains
plan hashes from the broker rather than accepting client-supplied hashes.
`cancel` terminally closes a pre-effect prepared, building, or awaiting-approval
transaction and releases its active-plan slot; an already authorized guard owns
recovery instead. In a headless session, authorization and apply fail with
`AUTH_REQUIRED` while retaining the concrete plan UUID in the typed error; they
never infer consent.

## Verification scope

Through the protected host workflow:

```fish
cd /mnt/Storage/Projects/HorizonOS
python3 tools/devctl.py test --suite unit --detach --json
python3 tools/devctl.py test --suite integration --provider managed-state --detach --json
```

The registered provider generates the actual locked catalog, builds/runs the checker,
compares Rust/Nix canonical bytes, evaluates NixOS package/PostgreSQL settings, and
checks input/transport/stateVersion/unfree denials. It realizes all four reviewed
packages, checks actual application versions and desktop launch entries, and starts
a disposable nonroot PostgreSQL cluster with peer authentication and no TCP listener.
The fixed readiness probe, an actual SQL query, stopped-server denial and teardown
are checked under the fixture owner's identity. A failed stop preserves the private
cluster for recovery.

Machine manifests and Rust grants/data observations are fixtures. These checks do
not establish graphical application workflows, a system PostgreSQL service under
the postgres account, root registration, activation or applied power policy. The
running machine's managed manifest and existing databases are not changed.
