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
An incomplete enumeration preserves the previous nodes and records Partial.
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

These library contracts require native adapter, scheduler, scoped viewer and
store-recovery integration. Transaction ledgers use separate paths and recovery
rules; graph storage never opens or resets them.
