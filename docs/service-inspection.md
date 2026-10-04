# Read-only service inspection

`aiosctl inspect service sshd.service --json` resolves an existing loaded system
service through the user's private `aios-sessiond` endpoint. The CLI receives an
opaque, short-lived service handle before invoking `system.service_status`.
The model action accepts only this issued handle, never a unit name, command,
D-Bus destination, method or privileged option.

The provider uses native systemd D-Bus reads on the fixed system bus. It checks
the manager's unique owner and root credentials, and returns the actual load,
active and substate, result, main PID, restart counter, exit status and current
job. It rechecks the owner, service invocation, state and PID before returning.
A changed observation fails with `STALE_EVIDENCE`; missing or denied data never
becomes an inferred healthy state. The provider observes existing units with
`GetUnit` and does not load, start, stop or restart services. Its result covers
one system service; it supplies no journal, process diagnosis or dependency
causality claims.

The daemon runs as the user and binds `/run/user/UID/aios/session.sock` in an
owned private directory. For an isolated deployment or test, both binaries
accept `--socket PRIVATE_PATH`; the daemon requires an absolute path in an
existing owned private directory and refuses to replace an existing socket.
No socket address grants additional authority. The daemon and client verify
kernel peer credentials, process start time, boot identity and the originating
process's logind association on every request. The daemon accepts only its own
UID. A missing logind session remains headless; UI attachment is unavailable.

Control requests use four-byte big-endian framing, schema version 1, a UUID
correlation ID and a strict typed operation. Requests are capped at 64 KiB
before allocation; replies are capped at 1 MiB. Unknown or duplicate fields,
caller-supplied authority and malformed frames fail closed. Connections,
requests, service handles and tasks have finite quotas and timeouts.

The private API exposes capabilities, system information, service resolution,
typed invocation, submission, status, private paginated events, cancellation
and forgetting. Task IDs and service handles are bound to the authenticated
process identity and expire. A repeated nonce with the same typed request
returns the same task; a changed request with that nonce returns `CONFLICT`.
Default daemon startup connects on demand to the fixed, root-owned model
socket for read-only `ask`/`diagnose` requests. A separate worker obtains fresh
system information and requests a constrained final answer from local CPU
inference. It independently parses the output and returns the attached evidence;
it never dispatches a model-proposed action. Without an available model endpoint,
the task terminates with `MODEL_UNAVAILABLE`. `act`/`automate` orchestration is
currently unsupported. The isolated `--socket` fixture remains model-disabled.
Task ownership does not support resuming from a different client connection.

The hardened user service authenticates the installed model socket using kernel
credentials. Its user namespace can map the system's root and inference group
to overflow IDs. The client translates expected ownership through the kernel's
UID/GID maps and still requires the socket peer to be PID 1, checks the fixed
protected directories, socket mode and group, and rechecks the socket inode.
An overflow UID alone never proves root identity. These checks require no
exception to the user service's namespaces or filesystem/network restrictions.

Queued cancellation stops before inference. Running cancellation remains in
`cancelling` until the native context terminates; transport failures report their
actual error. Forgetting or losing a requester cancels outstanding work. The
90-second task deadline includes queueing and observation, and terminal results
expire after five minutes. Events record real transitions without broadcasting
question text or evidence.

Build the `aios-core` and `aios-cli` flake packages in a verified development
guest. `aios-core` contains the user daemon and its hardened systemd user unit.
Default daemon startup owns `org.aios.Session1` and exposes `org.aios.Agent1` at
`/org/aios/Session1`, alongside the private socket. Public lifecycle methods
authenticate the bus-provided unique sender, UID/PID, process start time,
boot identity and originating logind association. Task ownership includes
the bus instance and unique sender; another connection cannot inherit it.
`--socket` fixtures export only the private endpoint.

The same bus owner exports `org.aios.Files1`, `org.aios.Applications1`,
`org.aios.Settings1` and `org.aios.UI1` at their corresponding `/org/aios/...`
paths. All five interfaces share the same 16-call admission budget and caller
authentication. Files exposes Search, Metadata, Read, Summarize, Copy,
MoveFile, Trash and Restore; Applications exposes List, Launch, Actions and
Invoke; Settings exposes Get and Set. Each action method takes one versioned
control request with `operation.kind=invoke` and a strict registry `tool_call`.
The method fixes the permitted action ID: Search accepts only `files.search`,
for example. Callers cannot choose another dispatch namespace through its JSON.
GetCapabilities reports registered contracts separately from available actions.
These file, application and settings providers currently report unavailable.
Direct invocation carries no approved write plan; every write action returns
`AUTH_REQUIRED`, even if a future provider is registered. The shared
`aios-policy` evaluator binds each available read to the authenticated caller,
original connection, request, fixed action and concrete broker-issued resources.
Successful reads still require the provider's current scope and resource checks.
Question tasks retain a volatile read grant for their original goal and the
`system.info` action. Grants expire using suspend-inclusive boot time and are
revoked immediately by Stop, Forget, disconnect, deadline or task completion.
They cannot survive a broker restart or authorize another UID/session/client.
No grant, nonce or write approval enters model context. Native graphical consent
is required before UI access; observing a graphical candidate creates no grant.

The user unit hides home directories with `ProtectHome=tmpfs` and exposes only
its own user runtime directory, including the standard session bus and private
socket. It runs without network access or added capabilities, with a read-only
system, private devices, and bounded memory and process counts. Loss of the
public bus owner terminates the daemon so the service manager can restart it.

`aiosctl status --json` reports the authenticated session's capabilities and
availability. `aiosctl ask TEXT --json` submits a read-only question and reads
its terminal status through the same authenticated bus connection. Closing that
connection revokes ongoing inference rather than leaving detached work. The
client pins and rechecks
the daemon's live owner identity; default endpoints ignore environment
variables that redirect the bus or select another user's runtime directory.

Public `Submit` takes the same versioned control request used by the private
transport, with `operation.kind=submit` and a typed `operation.request`. Other
public methods use the PRD's small typed arguments. Task status, event,
cancellation and deletion JSON include version, task ID and operation.
Stable D-Bus errors are named `org.aios.Error.CODE`, using the PRD's codes.
The AIOS interface declares no task/evidence broadcast signals.

Module/image activation, persistent evidence enrollment, interactive grants,
journal/process diagnosis and the multi-step tool loop have separate contracts.
Reconnecting creates a new authenticated connection and cannot inherit a
previous connection's authority. Graphical selection remains an observation;
selection alone does not authorize UI control.

`python3 tools/devctl.py test --suite integration --provider service-inspection
--json` runs the registered real guest CLI and daemon tests against the guest's
systemd, then builds both Nix packages and exercises their actual binaries.
Ownership fixtures are labeled separately from kernel-credential
tests; they do not establish two-user desktop or privileged-policy acceptance.

`python3 tools/devctl.py test --suite integration --provider public-session
--json` introspects the actual user-bus interface, then exercises the packaged
systemd user service, its effective hardening, default endpoints and restart.
It refuses to replace an existing user service and cleans up only its own
temporary runtime registration.

`python3 tools/devctl.py test --suite integration --provider installed-session-inference
--json` exercises the installed hardened user service against the real installed
CPU model in the disposable model acceptance image. Both client and broker come
from the current system closure's root-owned immutable packages, with their
executable hashes recorded. It checks a read-only answer
with fresh system evidence, unchanged unit bytes, effective restrictions, service
inspection and broker restart. It requires an enrolled normal user and retains
failed responses. It does not qualify graphical inference, model crash reporting
or the complete orchestration loop.

`python3 tools/devctl.py test --suite integration --provider session-inference
--json` runs actual native session and model daemons with private development
qualification sockets. It verifies an evidence-backed question, nonce conflict,
reconnect denials, queued and active cancellation, native context termination,
private events, forgetting and unsupported write modes. This qualification is
one real UID; it does not establish the installed model sandbox, two-user
inference isolation, graphical consent or the complete orchestration loop.
