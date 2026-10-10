# Per-user file scope contract

Root proposals enumerate only existing Documents, Downloads and Desktop XDG
locations beneath the authenticated account's home on the same mount. XDG
configuration is read through the pinned home descriptor with no-follow,
no-new-mount and bounded regular-file checks. A proposal observes root identities;
it does not permit file content access.

`Files1.EnrollRoots` accepts schema version, the broker-issued proposal ID, an
exact selection of proposed root IDs, access classes, and an originating-client
bound desktop candidate handle obtained through `Agent1.SelectUiSession`.
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
