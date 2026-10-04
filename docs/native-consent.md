# Native application read consent

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
does not authorize input, writes, external effects or system transactions.

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
acknowledges only after authentication. Subsequent messages use the existing
big-endian framing, schema version 1 and UUID correlation IDs, a 64 KiB request
limit and 1 MiB response limit. Unknown operations, duplicate fields and extra
fields are denied. There is no approval operation.

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
authority. Public graphical Submit and D-Bus origin proof require their own
integration before these controls can serve the complete task lifecycle.

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
