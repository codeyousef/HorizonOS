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

## Native interaction and qualification

The dialog uses the native palette, fonts, focus and accessibility of Qt Widgets.
Cancel is the default Return action; Escape, withdrawal and expiry cancel. Allow
requires a deliberate action on its separate button. There is no universal
approval option. Its desktop identity is `org.aios.Confirmation` and its window
object identity is `aios-protected-confirmation`; AIOS computer-use providers
must exclude confirmation surfaces from observation and activation. Ordinary
native assistive technology remains available.

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
