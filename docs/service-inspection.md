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
There is currently no inference runtime: submissions explicitly terminate
with `MODEL_UNAVAILABLE` and perform no mutation. Task ownership does not yet
support resuming from a different client process. Cancellation of these
terminal tasks reports that state without pretending to interrupt a model.

Build the `aios-core` and `aios-cli` flake packages in a verified development
guest. `aios-core` contains the user daemon and its hardened systemd user unit.
Default daemon startup owns `org.aios.Session1` and exposes `org.aios.Agent1` at
`/org/aios/Session1`, alongside the private socket. Public lifecycle methods
authenticate the bus-provided unique sender, UID/PID, process start time,
boot identity and originating logind association. Task ownership includes
the bus instance and unique sender; another connection cannot inherit it.
`--socket` fixtures export only the private endpoint.

The user unit hides home directories with `ProtectHome=tmpfs` and exposes only
its own user runtime directory, including the standard session bus and private
socket. It runs without network access or added capabilities, with a read-only
system, private devices, and bounded memory and process counts. Loss of the
public bus owner terminates the daemon so the service manager can restart it.

`aiosctl status --json` reports the authenticated session's capabilities and
availability. `aiosctl ask TEXT --json` submits a read-only question and reads
its status through the same authenticated bus connection. Until inference
is implemented, its explicit failed result is `MODEL_UNAVAILABLE`, with no
answer fabricated and no mutation performed. The client pins and rechecks
the daemon's live owner identity; default endpoints ignore environment
variables that redirect the bus or select another user's runtime directory.

Public `Submit` takes the same versioned control request used by the private
transport, with `operation.kind=submit` and a typed `operation.request`. Other
public methods use the PRD's small typed arguments. Task status, event,
cancellation and deletion JSON include version, task ID and operation.
Stable D-Bus errors are named `org.aios.Error.CODE`, using the PRD's codes.
The AIOS interface declares no task/evidence broadcast signals.

Full module/image activation, other public project interfaces, persistent evidence storage,
interactive grants, journal/process providers and inference are separate
contracts still to be implemented. Full public interface acceptance also
requires two-real-user qualification, enrolled reconnect scopes and live
model cancellation boundaries.

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
