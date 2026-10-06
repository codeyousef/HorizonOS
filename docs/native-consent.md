# Native consent

`aios-consent-ui` provides a Qt Widgets permission dialog. It displays a single
immutable, bounded `read_scope` proposal: user, explicitly selected desktop,
target, local CPU profile, Ask/Diagnose mode, user goal, named application
windows, concrete resource identities, permitted read actions, evidence,
proposal digest and expiration. Supplied text is literal and cannot create
markup, links, buttons or visually reordered permission boundaries.

This dialog is a presentation component. A response is **not a policy grant**.
The broker owns the original authenticated client, native graphical selection,
resource resolution, canonical proposal hash, policy revision, boot identity,
volatile nonce, revocation and issuance. Those bindings must still match after
the response. A changed resource, disconnected client, cancelled task, expired
proposal or changed policy requires denial and a fresh proposal. The component
does not authorize effects merely by rendering a proposal. Read and process
termination use distinct fixed proposal types and distinct opaque decisions.

The `process_termination` presentation is an exact R2 preview in Act mode. It
contains one selected owner-bound process handle, native PID/UID/start/boot and
executable locator, current immutable system closure, one SIGTERM, verification
timeout, no automatic escalation and irreversible impact. The dialog states
that work may be interrupted or unsaved data lost, an ignored signal or timeout
can leave a partial effect, and there is no automatic SIGKILL or retry. Cancel
remains the default action. The separate button says “Terminate this process”.
Native boot, own UID, fixed action/signal, bounded timeout and exact closed
fields are checked before rendering. No application read proposal can be used
as a termination decision.

`aios-policy::consent::termination` freezes the original authenticated subject,
request and goal, native display, process identity/handle, current closure,
fixed effect/verification, registry/policy revision, broker incarnation, volatile
nonce and expiry into one canonical digest. Its native transport is shared with
read confirmation: only the compile-pinned child, successful exit and exact
bounded digest response can produce an opaque decision. Consuming that decision
revalidates every native resource and returns non-cloneable delivery authority
for at most two final adapter checks. Each check binds the original task and
canonical retained-pidfd preview; denial or exhausted checks permanently revoke
authority. Stop revokes both pending and delivered decisions independently.
The native broker must reauthenticate the original caller at each boundary and
resolve real native resources; the policy library is not an identity adapter.

These process types are not yet connected to installed broker/helper
orchestration or exposed through public signal actions. Policy fixtures and Qt
widget fixtures do not prove an installed native termination consent or effect.
Those require source-bound installed service and native interaction qualification.

## Private transport

The supervising unprivileged broker supplies one anonymous Unix socket as file
descriptor 0. The dialog checks its kernel peer UID and PID against its actual
parent, rejects root, and accepts no command-line arguments. Proposals and
decisions are not public D-Bus messages or model tool outputs.

The parent sends a four-byte big-endian length and canonical compact UTF-8 JSON.
The maximum payload is 64 KiB. The dialog requires exactly one object, sorted
keys, no unknown or duplicate fields and no alternate numeric spellings.
Admission has a two-second deadline. Further bytes or write-side closure withdraw
the displayed proposal; they cannot replace it in place. Expiration uses Linux
`CLOCK_BOOTTIME`, including suspended time, with a maximum lifetime of five
minutes. Both the timer and the Allow callback check expiration.

The exact fields are `schema_version: 1`, `kind: "read_scope"`, `digest` (lowercase
SHA-256), `uid`, `session_id`, `target`, `profile`, `mode` (`ask` or `diagnose`),
`goal`, `apps`, `actions`, `issued_ms`, `expires_ms` and `evidence`. Each of one to
sixteen `apps` has a unique `handle`, `identity_sha256`, `name` and `window`.
`actions` is a unique nonempty subset of `ui.snapshot` and `ui.find`.
`evidence` contains at most sixteen bounded identifiers. There is no `approved`
field or transferable secret in the proposal.

The child returns one compact JSON line containing only `digest` and `decision`
(`allow` or `cancel`) on the inherited socket, then exits. The broker must require
an exact digest match, a successful exit, a bounded response with no extra input,
and a fresh native/policy validation. Failure never yields permission. Launching
another copy of this executable supplies no authority to the broker.

## Shared policy and native task ownership

`aios-policy::consent` freezes the authenticated originating subject, request,
goal, Ask/Diagnose mode, explicitly selected native display, concrete window
identities, presentation, installed policy revision, broker incarnation,
volatile nonce and suspend-inclusive expiration in the confirmation digest.
The first issuer permits only `ui.snapshot`; derived snapshot/node lineage is
required before `ui.find` can be enabled. It cannot authorize writes or roots.
Headless `grant_reads` continues to reject graphical capabilities.

Native accessibility snapshots retain private bus/process/window identity,
object and ancestor paths, snapshot generation, role, full bounded name, native
state and advertised action names. These fields are not serialized as client
authority. Read-only node re-resolution checks the captured ancestor chain and
live properties within two seconds; changed generation, owner, parent or
properties refuse the observation. Protected nodes have no resolvable lineage.
The read-task wrapper also checks the original connection and live read grant
before and after resolution. This does not enable `ui.find` or semantic input.

Native container pages accept only a fresh snapshot's opaque node handle from
the same selected window. The reader validates the previous lineage, captures
a new snapshot generation and checks the paged root again. Each page has a
two-second total query bound, depth eight, at most 300 nodes and 16 KiB observed
text. Captured ancestors stay private; ancestor validation is capped at 64
objects and cannot refresh old node handles into a new page. The read-task
wrapper rechecks the original live window grant and connection around paging.
The original Unix-client read task exposes `get_ui_snapshot_containers` with
`task_id` and `snapshot_id`, then `page_ui_snapshot` with the same fields and
an observed `container_handle`. The provider retains the native grant and
private snapshot; the caller cannot supply paths, owners or approval. Each
successful page replaces the previous generation, returns a readiness receipt,
and is consumed once with `take_ui_snapshot`. Old generations and failed native
revalidation end that read. These operations do not permit semantic input.
The worker holds a bounded command channel and never a task mutex across native
queries; Stop and the original cancellation socket independently revoke it.
The task's original 90-second deadline bounds grant and private-data retention.
Installed acceptance must exercise this provider route with actual native
consent; direct reader tests alone do not qualify the transport.

The native reader also resolves the existing `ui.find` role/name/state selector
contract against private snapshot lineage's immutable observed fields. Full
native names/states remain only for change detection; selectors cannot search
beyond the bounded name or first sixteen observed state identifiers. Names use exact matching by default
or an explicit literal `contains`; role/state identifiers retain the observed
`atspi:N` spelling. The result contains opaque matching handles and an explicit
ambiguity flag. Matches are scoped to that page, not the whole application.
An incomplete page refuses with `PartialResult`, and more than 100 matches
refuses with `ResourceExhausted`; neither silently selects or drops candidates.
The reader revalidates the selected-window root and every matching node under
the original two-second snapshot deadline and independent cancellation. The
task wrapper checks the original caller and live grant around this observation.
Serialized metadata cannot add searchable objects or protected-node lineage.
This native method does not expose a public selector RPC, advertise `ui.find`,
or enable input; those routes require their own scope and installed verification.

The native transport launches the compile-pinned immutable Qt wrapper with a
cleared environment and the selected Wayland socket. Its pollable supervisor
requires the actual pinned native child executable, successful exit and one
exact bounded response line. There is no public constructor for an approved
decision, no transferable grant and no request-selected renderer. Cancellation,
identity or resource drift, policy replacement and expiry withdraw the prompt.
Consuming the opaque native result rechecks every binding and issues a volatile
read grant. Every subsequent read freshly validates all selected resources,
including the display, before resolving that action's references; drift revokes
the grant permanently.

`aios-session::ui_read::NativeReadTask` retains the originating connection FD,
kernel socket cookie, native peer/process/logind identity, selected window,
pending dialog, shared policy and grant inside one provider worker. Disconnect
and independent Stop control withdraw consent and prevent further queries.
Snapshots require a live grant before the accessibility query and fresh
authorization after it, before returning content. The native traversal verifies
each object's parent chain inside the selected window before reading its name,
states or available actions. These library types accept no JSON authority.
The provider bridge must authenticate the fixed broker before accepting an
original connection FD; passing claimed UID/PID/session fields is insufficient.

## Graphical provider and originating client

The fixed `aios-ui-agent.service` belongs to `graphical-session.target`, requires
that target to be active, and stops with it. It runs as the desktop user with
no capabilities, no privilege escalation, only Unix socket address families,
restricted system calls and namespace creation, and bounded memory and tasks.

Native graphical-provider isolation has an explicit exception: mount/network
namespace directives on an unprivileged user unit implicitly create a child
user namespace, even when `PrivateUsers=no`. Linux then denies the native
desktop `/proc/PID/exe` checks on which compositor, accessibility launcher and
application authentication depend. The provider therefore uses the original
user namespace and filesystem view; it does not claim a hidden home, private
devices, a private temporary directory, or a read-only mount namespace.
Ordinary native UID permissions still apply. It is a trusted native component
with fixed operations and no model, arbitrary file/shell route or client-selected
executable. The orchestration broker retains all its separate namespace,
network, filesystem and process protections. Provider startup or identity
failure denies graphical access; it never switches to a weaker identity check.
The user manager also cannot drop a capability bounding set in the original
namespace without `CAP_SETPCAP`. The provider first clears its own effective,
permitted and inheritable sets (including a user manager's inherited
`CAP_WAKE_ALARM`) through a fixed startup `capset`, then verifies its effective,
permitted, inheritable and ambient sets are all zero and that `NoNewPrivileges`
is already active before binding the endpoint. It cannot gain capabilities by
executing a setuid or file-capability program. An empty kernel bounding set is
not claimed for this native user process.

The provider owns `/run/user/UID/aios-ui/provider.sock` (0600) in its unit-owned
0700 runtime directory. Both ends authenticate the actual kernel peer and the
root-managed `user@UID.service`, its configured immutable program and current
invocation. The native user manager must identify that peer as the active main
process of the fixed service with exactly the sibling immutable executable and
no arguments. Broker authentication additionally requires ownership of
`org.aios.Session1`. Each request rechecks this association; a service restart,
changed invocation, disconnected socket or changed original peer invalidates
the connection.

The first byte is `0xa7` with exactly one `SCM_RIGHTS` descriptor: the broker's
server end of the original client connection. The provider obtains the original
UID, PID, process start, boot, logind association and socket cookie from that
descriptor, not from serialized fields. Received descriptors are close-on-exec;
missing, multiple and unexpected ancillary messages are rejected. The provider
acknowledges only after authentication, with a version 2 handoff response whose
native originating identity digest must match the broker's independently
authenticated caller. The digest is a comparison, not authority in place of
the kernel connection. Subsequent messages use the existing
big-endian framing, schema version 1 and UUID correlation IDs, a 64 KiB request
limit and 1 MiB response limit. Unknown operations, duplicate fields and extra
fields are denied. There is no approval operation.

For a D-Bus client the marker is `0xa8` with no descriptors, followed by a
strict bounded reference containing only schema version, unique sender and bus
ID. Only the authenticated managed broker can send this reference. The provider
independently resolves the caller on `/run/user/UID/bus`; serialized UID, PID,
session and decision fields are denied. The very connection used for native
bus queries must have kernel credentials matching the root-authenticated,
active `user@UID.service` main PID, configured program, control group, start and
invocation. The owned runtime/socket identity is checked before and after use.
A replacement bus, changed manager invocation or vanished unique sender
invalidates the proof and every retained clone. Two connections from the same
PID remain distinct policy clients. These checks do not claim isolation from
unsandboxed malware running as the same UID.

Public `org.aios.UI1.SelectSession` and `ListWindows` return owner-bound,
short-lived metadata candidates. A new unique sender cannot use an earlier
sender's session or window handles. Metadata does not enable content reads,
semantic input or a task grant. Native discovery runs outside the global task
state lock, with bounded admission and provider connections.

The private client first selects an explicit native desktop, then requests
window candidates. These names and titles grant no content access. Candidate
handles last 30 seconds on that connection. `start_ui_read` proposes a named
window and original goal in Ask or Diagnose mode through the fixed native
dialog. Status, one-shot snapshot retrieval, Stop and Forget belong exclusively
to the originating connection. Reconnecting requires fresh selection and
consent. Stop withdraws the dialog independently of inspection and deletes
cached content; Forget removes the task. Disconnect cancels every owned task.
Limits are eight provider connections, eight retained tasks per connection,
four active consent/read workers globally and a 90-second task lifetime.

`aiosctl ui read-window SESSION EXACT_TITLE GOAL --json` keeps one original
connection through explicit selection, uniquely matched title, native consent
and snapshot retrieval. It denies absent or ambiguous titles. Snapshots are
bounded observations; this route invokes no model and grants no semantic input
authority. Public graphical Submit requires task, permission, evidence and
cancellation integration before these controls serve the complete task lifecycle.

## Native interaction and qualification

The dialog uses the native palette, fonts, focus and accessibility of Qt Widgets.
Cancel is the default Return action; Escape, withdrawal and expiry cancel. Allow
requires a deliberate action on its separate button. There is no universal
approval option. Its desktop identity is `org.aios.Confirmation` and its window
object identity is `aios-protected-confirmation`; AIOS computer-use providers
must exclude confirmation surfaces from observation and activation. Ordinary
native assistive technology remains available.

The policy-owned launcher explicitly enables Qt's native accessibility bridge
for this dialog even when the desktop reports no active screen reader. This
fixed setting is independent of request fields and does not register the
confirmation executable as an AIOS observation or input target.

`checks.x86_64-linux.consent-ui` contains widget fixtures for literal untrusted
text, accessibility, default cancellation, explicit keyboard choice, immutable
display, expiry and withdrawal. Its activation test binary is absent from the
production UI output. The registered `consent-ui` guest scenario requires the
disposable desktop profile and actual tester UID, verifies the real Wayland
socket owner, runs the fixtures on Wayland, and launches the production dialog
for expiry, changed-input and disconnect checks. Its proposals are synthetic;
these tests neither issue a policy capability nor establish native scope grants.

The registered `accessibility` scenario additionally authenticates a real kernel
client and native Kate window/display, runs the production policy-owned dialog
through expiry and withdrawal, rejects an unconfirmed snapshot, and checks that
closing the originating connection revokes pending consent. Its document is
synthetic. It does not press Allow, issue a native grant or qualify semantic
mutations. Synthetic policy tests exercise issuance postconditions separately
and must not be reported as native human confirmation evidence.

The registered `ui-provider` scenario uses the exact packaged broker/provider
units in the disposable tester desktop. Its test-only assistive actor verifies
the pinned production renderer, its managed-provider parent, native process
stamps, the unique synthetic request, exact displayed window identity, target,
session, profile and read scope. It checks the native Allow button's role,
enabled/sensitive/visible/showing states and action before one input attempt;
input failures or timeouts are never retried. The resulting snapshot must come
through the original authenticated client, validate against the contract and
be retrievable once. Reconnected clients, forged decisions, pending reads and
cancelled tasks remain denied. Production window discovery must exclude the
permission renderer while it is open. The actor is compiled only into the
ignored guest integration test, is absent from installed products, and requires
the disposable profile, tester UID and explicit registered scenario. This is
native assistive-input fixture evidence, never evidence of human review or
AIOS semantic input permission.

The same registered scenario checks real public D-Bus window discovery through
the hardened broker and exact native provider. A separate guarded native test
checks kernel bus/user-manager association, distinct unique senders with the
same PID, forged bus IDs, well-known-name rejection and original disconnect
revocation while another sender remains live.

Public `Agent1.Submit` can bind both selected session and window handles to
the original connection's native metadata. Ask and Diagnose request the same
production read permission, with the fixed normal local CPU inference profile
displayed before consent. The public request ID is also the native read's ID.
Only a successful, one-shot, contract-valid native snapshot creates the broker's
private inference receipt. It binds the original peer, submitted request digest,
window handle/identity, registry revision and suspend-inclusive task expiry.
The receipt allows inference over that captured observation; it cannot authorize
another native query, input or external effect. Truncated observations retain
partial status and incomplete context. No serialized receipt or decision is
accepted from a client, document or model.

Public Stop, Forget, deadline and disconnect revoke a task-specific kernel
channel installed before starting the native read. The exact managed broker
passes one native socket endpoint; the provider authenticates its kernel peer.
Any byte, EOF or failure only cancels that read. This channel is independent
of the provider query mutex, so a busy desktop query cannot block public Stop.
Owner-bound status and events report permission, inspection and terminal states.
Act and Automate still report unavailable orchestration without granting reads,
input, writes or rule enablement.

The registered disposable scenario additionally checks public graphical
Submit/status/events/Stop/Forget, nonce conflict and reconnect ownership through
the exact managed services. Its positive native Allow/capture case deliberately
has no installed model socket and must report `MODEL_UNAVAILABLE` after the
scoped observation event. This checks honest failure and cancellation; it does
not verify a model answer over desktop content or completion of the full OS.


The native selected-window read presentation explicitly includes `ui.snapshot`
and `ui.find` for tasks that support selectors. A snapshot-only proposal still
cannot authorize selectors. The public `find_ui_nodes` request names the task,
current snapshot UUID and a strict registry selector. Original bytes are checked
before conversion to JSON values, including nested duplicate-field rejection.
The provider retains the actual private page; its snapshot scope identity binds
that page generation, selected window and native ancestry. It re-resolves the
page under the original caller and delivered native read grant before and after
matching. This derived check exports no grant and extends no expiry. A stale
page, scope change, Stop or disconnected caller ends the read without retry.
The selector route does not authorize input or advertise a model capability.
