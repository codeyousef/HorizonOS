# Horizon OS broker preparation

The Rust `aios-exec` preparation core separates trusted installed code, managed
intent data, registered candidates and final plans. It does not yet expose a live
D-Bus endpoint, construct worker verification proofs, obtain polkit approval or
activate systems. The packaged `aios-execd` refuses all runtime requests while
those adapters remain unavailable. Non-root callers cannot gain authority through
command-line arguments, environment variables or fixture switches.
Root daemon startup runs the fixed native target, installed-template and system-bus preflight;
failure returns `BROKER_NATIVE_PREFLIGHT_FAILED`. A successful preflight still
returns the unavailable-runtime response until authenticated request/approval
adapters exist. Startup does not create candidate directories or a SQLite ledger.

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
The native root adapter reads this fixed record from the current system closure;
client path/hash selection must equal that installed authority. Canonical schema
and base/catalog/lock/manifest bindings are checked independently before intake.

`VerifiedTarget` is a non-deserializable native capability. Its production
constructor requires UID/EUID zero with the full root UID mapping, then reads
the canonical installed target enrollment and actual NixOS release, installation
UUID, hardware DMI UUID, boot ID, machine ID, guest role, disk serial and management
marker. The UUID device link must resolve to the enrolled root partition; actual
root mount filesystem/source and sysfs partition-to-disk ancestry must agree.
All resolved ancestors and links must be administrator/kernel owned, with no
untrusted directory writes except the protected sticky Nix store. Reads are
bounded, reject final symlinks and compare file identity/change metadata.

Candidate/template and ledger production openers mint this capability before
accessing persistent roots. They recheck enrollment and native observations at
use time; template preparation also rechecks the current installed authority.
New plans and build/finalization operations must match the live verified target.
Old plan inspection can expose historical data after reboot without authorizing
another build. Host-side pinned SSH verification remains the development transport
gate. This local code does not replace authenticated system-bus/logind subjects;
the native caller capability described below and native running/profile/boot-selected
baseline capture must be connected before runtime endpoints can be enabled.

## Native requesting subject

`SystemBus` requires the installed `VerifiedTarget` before opening the fixed
`/run/dbus/system_bus_socket`. Its protected root-owned ancestors and socket are
checked; caller-controlled bus addresses are ignored. `VerifiedCaller` has private
fields and no serialization or public identity constructor. Only a root broker's
injected D-Bus method-call header can supply its unique sender to this intake.
Root/service identity does not become an authenticated original user by proxying
a user request; requester UID zero is denied.

The bus supplies UID/PID. Bounded kernel proc reads check all four process UIDs,
PID and process start ticks against those credentials. Boot and bus-instance IDs
are recorded. The logind unique owner must have actual root bus credentials and
matching kernel process/boot identity. Session lookup uses this owner and the
requester's PID; the returned session UID must match. Session ID, remote status,
type, class, state and activity are read directly without property caches. Closing
sessions are rejected. A missing PID/session association remains absent: there is
no fallback to the newest desktop, inferred session, or client-supplied session ID.
Desktop availability is an observation and grants no UI permission.

Credentials, kernel process, bus instance and logind owner are checked again during
capture. Revalidation requires the exact snapshot and broker connection epoch;
reconnect, process reuse, boot/bus/logind restart, and changed session properties
invalidate the capability. This does not establish sandbox/application isolation
against same-UID unsandboxed malware. It confers neither authenticated user intent
nor plan approval. An originating user-daemon association, native polkit, exact-plan
volatile authorization and executor method adapters remain required integrations.

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
prove root installation, native root caller intake, enforced worker quotas,
real disk-full recovery, polkit, guard survival, activation or boot. These remain
required, including installed template qualification, root GC retention and native
runtime adapters. Host private keys and credentials remain outside all candidates.
Separate read-only tests query the actual guest system bus, kernel and logind for
the dev process, and verify two real connections, disconnect and forged-owner
denials. They do not mint production `VerifiedCaller` or approval capabilities.
