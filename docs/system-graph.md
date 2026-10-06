# System graph contracts

`aios-state::graph::store::GraphStore` owns one private SQLite database and one
bounded serialized worker. Native adapters select the database scope from their
authenticated identity. Model fields do not select scopes, providers, clocks,
authorization bindings or source revisions. Graph results never authorize an
effect; execution acquires critical preconditions from live providers.

The six graph tables hold nodes, edges, observations, evidence, events and
provider checkpoints. A separate metadata row pins the database scope. Startup
checks application/schema versions, exact DDL, integrity and foreign keys before
opening for mutation. Files are 0600 in a canonical private 0700 directory,
with ownership, link and symlink checks and an exclusive owner lock. SQLite uses
FULL-synchronous WAL. Incompatible stores are refused and preserved.

## Provider snapshots and reconciliation

A complete native snapshot atomically replaces only its provider, scope and
source-of-truth class. Absence from a complete enumeration tombstones a node.
An incomplete enumeration merges actually observed nodes, preserves unobserved
nodes and records Partial. Only explicit verified native absences tombstone
individual nodes; omission from a partial enumeration never implies absence.
Partial updates do not advance the last complete checkpoint/hash/success.
Stable identities cannot be rebound through ID reuse. Intended, built, running,
boot-selected, user/application and unmanaged nodes remain distinct.

Each checkpoint has a compare-and-swap token. A new event, upstream loss or
worker-queue overflow invalidates it, so a snapshot collected before that
invalidation cannot replace newer observations. Events coalesce by exact event
ID/content; conflicting reuse is refused. Admission loss is persisted before the
next queued operation. Every owner restart invalidates persisted provider
snapshots, covering a crash before an admission-loss flag reached disk.

`reconciliation_plan` compares authoritative boot, generation/profile/source
revisions and monotonic time. It requests native resampling after invalidation,
a partial/failed observation, a changed boot or revision, a backwards clock or
900 elapsed seconds. It does not invoke inference. The runtime owner must call
this planner at startup, on native events/revision changes and periodically.
Its returned token must precede collection of the replacement snapshot.

Reads of young evidence still become stale if its provider needs reconciliation.
Diagnostic reads expose captured time and explicit freshness. Access expiry and
caller association remain mandatory even for diagnostics. Monotonic clocks are
compared only within their original boot.

## Evidence and relations

Observation/evidence pairs are immutable and committed together. SHA-256 seals
cover payload and native metadata, locator, scope, caller binding, clocks,
expiry and excerpt. Node revision changes, wrong caller bindings, access expiry and
changed seals cause explicit refusal. Source locators produce typed viewer
descriptors; a file's display URI is excluded from its opener descriptor. A
viewer rechecks live scope, identity and content hash before opening the source.

Observed edges must correspond to exact relations explicitly declared in the
sealed native observation. A citation identifier alone proves no relation.
`OrderedAfter` represents systemd ordering separately from `DependsOn`.
Hypotheses use a distinct insertion API and provider label, remain hypotheses
on reads, and cannot acquire observed certainty through a cache update.

## Bounds

The worker queue holds sixteen requests. Node/edge queries, ordinary node writes
and event batches contain at most 64 entries. A complete snapshot contains at
most 8192 nodes and eight MiB of bounded node data; larger enumerations are
explicitly refused, never silently treated as complete. Properties have an
8192-byte serialized bound, depth 32 and 4096-value budget. Revision fields have
2048-byte bounds before enqueueing. A store contains at most 128 provider
checkpoints and retains the latest 4096 event sequences. Evidence access lives
for at most 900 seconds. Retention does not convert expired evidence into a grant.

## Rebuildable cache recovery

A successful compatible open records the graph inode, scope and filesystem
identity in a private version-2 ownership marker. On Btrfs, read-only descriptor
ioctls bind the filesystem UUID, subvolume ID and subvolume UUID; transient mount
device numbers are not persisted as durable identity. Same-boot pathname versus
open-descriptor device/inode checks still reject substitution races. Recovery
manifests use the same durable identity for every file and destination directory.

On other filesystems, identity is explicitly bound to the boot ID and device
number: these stores cannot reopen across boots. Version-1 ownership markers and
recovery plans lack provable durable identity and are retained and refused as
incompatible, with no automatic re-enrollment, reset or quarantine. This format
version is independent of the SQLite schema version.

Corruption of a marked graph triggers quarantine of the
fixed database, WAL, SHM, rollback journal and marker. A durable bounded manifest
binds their identities, lengths and SHA-256 hashes before any move. Renames never
overwrite files; files and both directories are synced. Interrupted moves resume
under the same exclusive owner lock before SQLite can create a replacement.
All old sidecars must be archived before a fresh graph can open. Replacement
state starts unknown and requires native provider resampling. A diagnostic
recovery receipt names the retained quarantine; it confers no execution rights.

Unknown unmarked corrupt files, foreign scopes/inodes, unsafe links, schema
changes and unsupported versions fail closed. Recovery does not delete any
quarantine or inspect transaction paths. Partial manifests or changed inventories
block opening for deterministic operator repair, preserving the bytes rather
than claiming a successful rebuild. A damaged ownership marker also blocks.

These library contracts require native adapter, scheduler, scoped viewer and
installed recovery notification integration. Transaction ledgers use separate paths and recovery
rules; graph storage never opens or resets them.

## Native own-user process adapter

Native clocks read the fixed kernel boot identifier from verified procfs and
sample kernel real/monotonic clocks, checking the boot before and after. The
process adapter uses the existing own-UID inventory with retained pidfds and
proc descriptors. Graph identities bind the actual UID, boot, PID and start
ticks. Its checkpoint is captured before enumeration; notifications racing a
collection cannot be acknowledged by moving the checkpoint after collection.

A complete native census replaces this provider's running nodes. Access denial,
exit or executable-identity change during collection/recheck makes the snapshot
partial and preserves unobserved diagnostic nodes. Live observations update
their nodes; only retained-identity TargetNotFound results remove exact nodes. Collection and apply must fit
within the two-second process freshness window. Errors signal reconciliation
loss; older snapshots cannot extend their lifetime by resampling only the clock.

Live reads re-inspect the retained native object and never reopen a PID stored
in graph properties. A retained object may be read after a cache snapshot
expires because the result is a new native observation, with its own timestamp.
The enclosing authenticated broker still owns read scopes, grants, revocation
and disconnect lifetimes. This adapter is not a root/other-UID process census,
an execution permission or an installed graph daemon.

## Private system graph owner

`aios-stated` runs under the dedicated unprivileged `aios-state` identity. It
owns `/var/lib/aios/state` and `/run/aios-state` at mode 0700; graph files and its
control socket are 0600. The runtime path is separate from model-owned runtime
parents. Startup and a fifteen-minute timer/fallback collect loaded system
service facts through fixed read-only calls to the UID-0 systemd manager. Unit
descriptions are omitted. Service-specific result/MainPID/ordering fields remain
unknown unless separately observed; loaded units are not the installed-unit
catalog, intended state or proof of a dependency.

The private control protocol accepts status, reconciliation, fixed generation,
block-disk and built-configuration diagnostics and a single
validated service-name cache lookup, with peer credentials, fixed frame bounds
and timeouts. Only root and the graph identity may use it. It is an internal
component/operator channel; authenticated product graph retrieval is separate.
Cache lookups show captured time and freshness and never authorize effects.
The service has no model, home, activation or transaction-ledger access. It is
wanted by normal boot and does not become a requirement of desktop or SSH.
Running NixOS activation and the selected system profile are observed separately
through fixed root-owned paths and retained as separate source-truth providers.
The native pointer pair is rechecked before/after persistence. Generation
diagnostics compare the cached pair with a new native read; missing, invalidated
or failed reads cannot claim current state. The profile does not prove a
bootloader entry or approved managed transaction; those fields remain unknown.
Once per second, the owner samples the two fixed pointers. An observed change
invalidates generation and service snapshots, records bounded native generation
events and reconciles without inference. Unchanged samples coalesce.

A separate native sd-bus connection subscribes to the UID-0 system manager at
its fixed system-bus endpoint. Manager and unit-property notifications from
that pinned unique sender invalidate the service provider; signal bodies never
become graph facts. Each drain has a 64-operation/20ms budget. Notifications
coalesce into one fresh native reconciliation. A bounded drain, disconnected
bus or changed manager identity reports possible loss and drops the old
subscription. Reconnection every five seconds rebuilds the native snapshot;
there is no inferred healthy state during loss. Status exposes subscription
availability and errors separately from snapshot completeness. The independent
fifteen-minute timer/fallback remains active. Complete closure metadata and user
overlay reconciliation remain separate providers.


The system owner also samples native block-disk properties through libudev at
startup, native generation/service refresh and the periodic fallback. This
provider covers block disks; other hardware classes and udev event monitoring
remain separate work. Serial, WWN, bus, model and vendor are native optional
properties. A missing serial remains null. WWN/serial-based keys are preferred;
otherwise a boot-scoped kernel locator is explicitly labeled without durable
identity or retained live identity. Duplicate stable properties are ambiguous
and refuse publication. Native inventories are compared before/after applying
an opaque two-second snapshot. Only the system-scoped database accepts them.
The root-private `--devices` diagnostic includes capture time and a conservative
two-second cache eligibility limit. Partial snapshots cannot claim Current.
These observations do not grant storage execution authority or expose raw
block-device access; effect providers must acquire critical identity live.

The built-configuration provider reads only `etc/aios/managed.json` and
`etc/aios/catalog.json` inside the observed running closure. It requires
root-owned immutable store objects and ancestors, bounded descriptor reads,
stable native pointers, exact file hashes and typed catalog/manifest validation.
The complete canonical managed data must match its template and catalog
revisions. An opaque snapshot expires after two seconds before application;
the provider checkpoint is captured before collection and checked on publication.

Configuration and declared package-selection nodes have `built` source truth.
The catalog is a selectable-package catalog; it is not a complete installed
package inventory. A declared package does not supply an observed package
closure or prove runtime availability. Service and power declarations likewise
do not prove runtime postconditions. Approval receipts, a current approved
intended manifest and managed-transaction provenance remain separate authority.
The root-private `--metadata` diagnostic compares the cache with fresh native
metadata and returns capture time and freshness without execution authority.

The root-private `--service-evidence UNIT.service` captures fixed native systemd
properties for one loaded unit and records a sealed observation and service
locator. It does not load or start the unit. The provider scope is that selected
unit; a complete selected read does not establish a complete service census.
The pre-read provider checkpoint, boot, five-second deadline, native property
comparison and entity revision bind the descriptor. A subsequent selected read
invalidates older revisions.

`--evidence ID` resolves only a current, sealed service descriptor through the
compiled native systemd property viewer. Scope, locator kind and lifetime are
checked before the native query, then exact properties and the original seal
are checked again. Changed properties refuse the citation rather than relabel
it as current. These controls use the existing root/graph kernel peer checks;
they are not a model tool, user task grant, executable URI opener or effect.
File, journal, option, application and UI viewers require their own scoped
native access adapters; this service route cannot substitute for them.
