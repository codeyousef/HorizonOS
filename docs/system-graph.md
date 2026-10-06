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

A successful compatible open records the graph inode and scope in a private
ownership marker. Corruption of a marked graph triggers quarantine of the
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
