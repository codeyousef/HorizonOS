# Per-user file scope contract

Root proposals enumerate only existing Documents, Downloads and Desktop XDG
locations beneath the authenticated account's home on the same mount. XDG
configuration is read through the pinned home descriptor with no-follow,
no-new-mount and bounded regular-file checks. A proposal observes root identities;
it does not permit file content access.

`Files1.EnrollRoots` accepts schema version, the broker-issued proposal ID, an
exact selection of proposed root IDs, access classes, and an originating-client
bound desktop candidate handle obtained through `UI1.SelectSession`.
Approval flags and supplied UID, identity or trust fields are rejected. The
broker resolves the selected native desktop and launches the compile-pinned
protected dialog. It displays root paths, exact identity hashes, access classes,
exclusions and the five-minute grant lifetime. Cancel is the default. Root
mutation access grants no execution authority and cannot replace an operation's
separate plan, preview or required approval.

The native proposal binds authenticated process, unique transport, UID, boot,
logind session, policy revision/incarnation, request, nonce, exact roots/access,
native desktop and suspend-inclusive expiry. Only the owned child transport can
create the single-use native decision. The broker rechecks the originating
client, desktop and root identities while waiting and immediately before root
enrollment. Caller disconnect, identity change, lock, expiry or cancellation
withdraws the pending dialog without issuing a grant. `CancelRootEnrollment`
uses an independent control admission path and only accepts the originating
client. Cancellation and grant creation share a linearization boundary.

Root grants and opaque file handles are volatile and client-bound. Every read,
metadata lookup and mutation precondition reopens beneath the pinned directory
with no-follow/no-symlinks/no-new-mount constraints. File metadata, mount, inode,
owner, grant/access and expiry must still match. A reconnect, different session
or foreign client cannot reuse authority, even with the same UID. Revocation
shrinks authority immediately and does not require a new approval flag.

Mandatory secret-path exclusions apply after selection. Recognizable raw private
key headers are refused before content is returned even under an ordinary renamed
text filename. This is not a universal secret-content classifier: unknown text
formats must not be advertised as automatically sanitized. Secret-reference
mechanisms are distinct from raw file reads.

The scope manager invalidates direct and cached access before removing its
associated private records. Persisted indexes and extraction workers must use
these same authorization and identity checks before releasing snippets and must
purge their own retained material when a root is revoked. This does not promise
forensic erasure of storage, backups or SQLite free pages.

The installed broker keeps consented UTF-8 reads in a private SQLite derived
cache under the authenticated account's `.local/state/aios/files`, independent
of caller environment variables. Directory traversal refuses symlinks/new
mounts and foreign ownership; files are mode 0600 and the directory mode 0700.
A single held lock prevents another broker from clearing live cache data.
The cache is limited to 256 documents, 128 MiB database pages and 2 MiB retained
text per document. Preview/snippet prefixes retain original Unicode. It is a
read cache, not the ranked FTS5 index or a document extraction worker.

Cache rows bind root, handle, full authenticated owner, source identity and
expiry. Every snippet rechecks live authority and source identity before and
after storage lookup. Revocation blocks handles first, then transactionally
deletes the root's documents, chunks, previews and snippets. A failed purge
returns `access_blocked=true` and `purge_pending=true`; only its original client
can retry that purge. Revocation of one root leaves other roots untouched.
On broker restart, old derived rows are purged because volatile grants are lost;
text on disk can never recreate permission. Original source files are untouched.

The optional `file-scopes-native` development test runs the pinned NixOS test
driver inside an enrolled development guest. Its synthetic node uses 768 MiB
RAM, software emulation, two real UIDs and temporary mounts. The fixed probe
tests pre-enrollment denial, a world-readable foreign-owned file, symlink
escape, mount changes after handle creation, root replacement, persisted-cache
revocation and foreign-user cache access. Driver mutations recheck the recorded
NixOS, boot, installation, DMI, disk, management and SSH public-key identities.
No host private directory or credential enters the node.

Run it through `devctl test --suite integration --provider file-scopes-native --detach`, then
inspect and pull the returned job with `devctl jobs status --job UUID` and
`devctl jobs pull --job UUID`. Select the enrolled workspace with the normal
`--workspace` option. This test is outside default fast checks. Its report is
component evidence: internal scope enrollment does not qualify native UI consent,
installed public D-Bus behavior, ranked indexing or CPU performance.

The separate optional `file-scopes-consent` test uses a synthetic Plasma
Wayland account in a 3 GiB, two-core software-emulated NixOS node. It installs
the packaged broker and Qt dialog with their service protections. Its client
uses public `UI1` and `Files1` methods only. An assistive-technology fixture
checks the pinned native dialog's visible roots and access classes, then invokes
its actual enrollment button. It cannot mint a decision or write to the private
approval transport. The cases cover pre-consent denial, forged confirmation
rejection, cancellation, root identity change while approval is pending,
restricted reads, client binding and persisted-cache revocation. Fixed startup
diagnostics emit error codes and synthetic metadata without file contents.

Use `devctl test --suite integration --provider file-scopes-consent --detach`
in an enrolled workspace and inspect/pull that same returned job ID. This test
is also outside default fast checks. Its structured result requires every
case, target identity, installed unit/binary, native process evidence and completed
purge. A successful report qualifies the installed public-service/native-dialog
flow in the synthetic desktop; production installation, ranked-index revocation
and performance require their own verification.

The broker retains `NoNewPrivileges` and syscall/namespace restrictions. Its
unit disables `RestrictSUIDSGID` because that setting blocks even read-only
`openat2` calls; replacing descriptor-safe access with a weaker path fallback is
not allowed. Cache creation modes remain fixed at 0700/0600, with no caller
permission-mode parameter or scoped-file execution. Qualification checks the
actual installed process's UID, namespaces, no-new-privileges, seccomp and empty
effective/permitted/inheritable/ambient capabilities. The observed bounding set
is recorded separately and cannot substitute for those enforced credentials.
