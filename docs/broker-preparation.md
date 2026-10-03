# Horizon OS broker preparation

The Rust `aios-exec` preparation core separates trusted installed code, managed
intent data, registered candidates and final plans. It does not yet expose a live
D-Bus endpoint, construct worker verification proofs, obtain polkit approval or
activate systems. The packaged `aios-execd` refuses all runtime requests while
those adapters remain unavailable. Non-root callers cannot gain authority through
command-line arguments, environment variables or fixture switches.

## Candidate artifacts

`InstalledTemplate` accepts an administrator-selected immutable Nix package and
its independently installed manifest SHA-256. The calling root adapter must select
both from its trusted installation configuration, never a client or model message.
A store prefix alone is not trusted provenance. Production constructors require
actual UID/EUID zero and have no fixture/path override exposed by the executable.

The administrator flake produces `aios-template` from the reviewed public source
domains in `nix/state/template-inputs.nix`. Every selected source file is included
in the code revision by path, normalized mode, byte length and SHA-256. The package
adds the generated locked catalog and the enrolled public management identity;
its canonical `template.json` binds that exact inventory. Mutable managed data,
candidate metadata, development job manifests and host private files are excluded.
Source size/path bounds agree with the broker contract. Nix copies source data as
0444 files in 0555 directories; installed Nix store objects are root owned.

The development NixOS configuration installs an independent 0444
`/etc/aios/template-authority.json` with the package path, manifest SHA-256 and
base/catalog/lock revisions. Its reference keeps the template in the system
closure. Protected tmpfiles declarations create candidate and ledger directories.
This configuration does not enable a broker or grant a user root authority.
It requires an enrolled public management identity before a template can be built.
The installed record must be read and verified by a native authenticated root
adapter; merely passing its serialized fields remains insufficient authority.

The template manifest lists bounded, normalized paths, modes, sizes and hashes.
It must contain the fixed flake, lock and generated catalog. Reserved managed/
candidate metadata cannot be supplied by template entries. Root-owned readonly
source files may use legitimate Nix store hardlinks; separately copied candidates
require single links. Unlisted files/directories, symlinks, writable objects,
traversal, changed bytes and mismatched catalog/lock metadata are rejected.

Every directory is traversed with pinned descriptors and `O_NOFOLLOW`. Existing
Nix store objects may sit beneath the root-owned sticky group-writable store;
other writable ancestors are rejected. The broker rechecks directory identity,
source inventory and all hashes before publication. It creates a unique private
staging directory, writes/fsyncs the reviewed template and canonical `managed.json`,
and seals files/directories readonly. Publication uses atomic no-replace rename,
followed by parent fsync. Reuse verifies the existing bytes and never replaces a
corrupt candidate. Failure removes only this invocation's unpublished staging tree.
A successfully published but unregistered orphan cannot be treated as a plan.

Candidate identity binds template, base/catalog revisions, lock, managed data and
all source file entries. `Candidate` is a non-deserializable capability obtained
from preparation, not an arbitrary client path. Before ledger registration, the
store revalidates it and requires the plan's manifest/revisions/digests to match.

The enrolled development machine consumes root-level managed JSON when present
and requires its catalog bytes to equal the generated locked template catalog.
It uses the strict managed module inside the pure flake. Candidates without managed
data remain the explicit development base image; a product candidate always contains
the compiler's materialized file. No live `/var/lib/aios` reads occur in evaluation.

## Plan register

The preparation ledger is root-private SQLite with FULL synchronous writes,
DELETE journals, serialized transactions and one active system-change slot.
The root opener requires a 0700 protected directory and 0600 single-link regular
DB, suppresses trusted schemas, refuses symlinks and does not recreate an existing
empty or unreadable ledger. Required schema/index absence blocks opening.

Immutable canonical prepared bytes and their digest record original requester,
logind session/unique sender, act mode, intent text, typed intent, target identity,
policy/capability revisions, candidate/template hashes and preliminary semantics.
Intended manifest, running closure, system profile and boot-selected closure/
metadata are distinct fields. Native adapters must supply authenticated subjects
and actually observed baselines; serialized values are not authentication.

Preparation durably records RECEIVED, VALIDATING and PLANNED before BUILDING.
Build permission is distinct from final activation approval: it binds requester,
boot, plan/candidate digest, approved cache, integer cost ceilings, recovery reserve
and expiry. The worker must enforce those limits; this core only validates and
records permission. Repeated build preparation cannot replace an existing grant.

Build result/worker-stop capabilities cannot be deserialized or manufactured by
clients. Their production constructors remain absent until the privileged adapter
independently verifies candidate, derivation, Nix outputs/roots, target, baseline and
worker PID/start/cgroup identity. Unit tests explicitly use fixture proofs.

A verified result binds exact candidate, derivation, realized closure, inventory
hash and ordered disjoint closure additions/removals. Final freeze records exact
build/semantic data, reboot requirement, fixed ordered steps and five-minute expiry
under a canonical digest. Retrying freeze returns identical immutable bytes;
changing its reboot classification is a conflict. This is an artifact, not an
approval receipt. No live authorization nonce is persisted in these artifacts.

Cancellation before a worker runs is terminal. During BUILDING it records a
request and retains the exclusive slot until a verified worker-stop capability is
provided. Timeouts are not termination evidence. Reopening a pending ledger does
not replay builds or activation. Transition/history commits are atomic and checked;
SQLite aborts do not publish successful transitions. Execute currently returns
`ActivationUnavailable`; no preparation history can claim a committed installation.
The independent guard's activation/recovery ledger remains a separate component
until the qualified broker/guard orchestration is connected.

## Verification and remaining integration

```fish
cd /mnt/Storage/Projects/HorizonOS
python3 tools/devctl.py test --suite integration --provider broker-preparation --detach --json
python3 tools/devctl.py test --suite integration --provider managed-state --detach --json
python3 tools/devctl.py test --suite unit --detach --json
```

The registered provider runs actual Rust filesystem/SQLite tests and builds/runs
the executor package for non-root denials. Tests use dev-owned fixture directories,
fixture templates/targets/grants/build outputs and SQL-abort injection. They do not
prove root installation, native guest/peer verification, enforced worker quotas,
real disk-full recovery, polkit, guard survival, activation or boot. These remain
required, including installed template qualification, root GC retention and native
runtime adapters. Host private keys and credentials remain outside all candidates.
