# Read-only service inspection

`aiosctl inspect service sshd.service` resolves an existing loaded system service
through the user's private `aios-sessiond` endpoint and prints its native state
with evidence identifiers. `--json` returns the complete typed observation. The
CLI receives an opaque, short-lived service handle before invoking
`system.service_status`. The model action accepts only this issued handle, never
a unit name, command,
D-Bus destination, method or privileged option.

The provider uses native systemd D-Bus reads on the fixed system bus. It checks
the manager's unique owner and root credentials, and returns the actual load,
active and substate, result, main PID, restart counter, exit status and current
job. A present job includes its native type and waiting/running state; its ID
and attached unit must match the service's job reference. The invocation ID
identifies the observed service run. The provider rechecks the owner, invocation,
state, PID, job, failure fields and bounded ordering dependencies before returning.
A changed observation fails with `STALE_EVIDENCE`; missing or denied data never
becomes an inferred healthy state. The provider observes existing units with
`GetUnit` and does not load, start, stop or restart services. Its result covers
one system service. `ordering_after` lists at most 128 canonical dependencies;
`ordering_is_not_causation=true` explicitly prevents interpreting `After=` as a
failure cause. No journal or process diagnosis is supplied. Job observations use
the [native systemd Job interface](https://github.com/systemd/systemd/blob/main/man/org.freedesktop.systemd1.xml).

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
system information and runs constrained local CPU decisions. Each typed read
proposal is checked against the original task grant before its native provider
runs; provider I/O does not hold the task-state mutex. A decision that has enough
evidence transitions to a separate constrained final answer. Both the daemon and
broker parse output independently, and the broker returns the retained evidence. Without an available model endpoint,
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
`org.aios.Settings1`, `org.aios.Audio1`, `org.aios.Power1` and `org.aios.UI1`
at their corresponding `/org/aios/...` paths. The interfaces share the same
16-call admission budget and caller authentication. Files exposes Search,
Metadata, Read, Summarize, Copy, MoveFile, Trash and Restore; Applications
exposes List, Launch, Actions and Invoke; Settings exposes Get and Set; Audio
exposes Outputs, Inputs, DefaultGet, DefaultSet and MuteSet; Power exposes
Status, ProfileSet and ConfirmProfileSet. Ordinary action methods take one
versioned control request with `operation.kind=invoke` and a strict registry
`tool_call`. The method fixes the permitted action ID: Search accepts only
`files.search`, for example.
Callers cannot choose another dispatch namespace through its JSON.
GetCapabilities reports registered contracts separately from available actions.
File and application providers currently report unavailable. Settings Get is
available when at least one of the three registered keys has an authoritative
pinned adapter. `desktop.theme_mode` reads only the Breeze Light/Dark
`kdeglobals` color-scheme key, `display.idle_seconds` reads only the active
PowerDevil profile's Plasma 6 display-idle key, and
`keyboard.backlight_percent` reads the UPower KbdBacklight API. Missing files,
keys, hardware or unrecognized values are unsupported. Audio inventory and
defaults are available only when the Nix-pinned absolute `wpctl status --name`
adapter can be read within fixed time and output bounds. Power status is
available only when the authenticated session's PowerDevil profile API and
system UPower API both answer; absent batteries and unavailable profile choices
are explicit partial fields, never fabricated values. Audio handles derive from
direction and the provider node name, not transient numeric node IDs; duplicate
provider identities fail closed.

Direct provider-interface invocation carries no bounded task grant, so every
write action returns `AUTH_REQUIRED`, including Audio DefaultSet and MuteSet and
Power ProfileSet. An authenticated caller may use Agent1 `ExecuteTaskAction`
with an exact `act` goal and typed R1 action. That route accepts audio
default/mute and all three registered desktop settings. It mints an opaque
caller/boot/policy/action/resource/argument/expiry-bound task grant, revalidates
the live caller and target immediately before a fixed provider effect, verifies
provider readback, and returns the prior typed action as recovery data. Theme
changes map only `light`/`dark` to the pinned Plasma color-scheme helper.
Display idle writes only the current, allowlisted PowerDevil profile's fixed
timeout key and reparses PowerDevil. Keyboard backlight uses only UPower's
bounded brightness method and refuses percentages the hardware cannot represent
and recover exactly. No arbitrary KConfig command, profile, key or scheme enters
the API. It never accepts an approval boolean, command, numeric node ID or
serialized grant.

R2 `power.profile_set` uses Power1 `ConfirmProfileSet`, not the R1 route. Its
strict `confirm_power_profile` request contains only a task ID, selected
graphical-session handle, goal, `act` mode and typed tool call. The broker
derives the prior profile and PowerDevil-advertised allowlist, binds both plus
the authenticated caller, boot, native desktop, policy revision, nonce and
expiry into the immutable native dialog, and revalidates the bus sender,
desktop and provider state immediately before the effect. Only
`power-saver`, `balanced` and `performance` can be displayed or executed.
Readback and the prior-profile recovery action are mandatory. No decision,
token, prior value, profile choices, bus name or object path enters the request.
The shared `aios-policy` evaluator binds each available read to the authenticated
caller, original connection, request, fixed action and concrete broker-issued
resources.
Question tasks retain a volatile read grant for their original goal and the
`system.info` action. Explicit caller-supplied `context_handles` can add
`system.service_status` for owned, unexpired service handles only. The model
cannot resolve a service name or add a handle to the grant. Ownership, expiry,
resource binding and policy are checked before and after each provider read.
Grants expire using suspend-inclusive boot time and are
revoked immediately by Stop, Forget, disconnect, deadline or task completion.
They cannot survive a broker restart or authorize another UID/session/client.
No grant, nonce or write approval enters model context. Native graphical consent
is required before UI access; observing a graphical candidate creates no grant.

The user unit hides home directories with `ProtectHome=tmpfs` and exposes only
its own user runtime directory, including the standard session bus and private
socket. A separate fixed oneshot projection reads only the registered KDE
settings files and copies them mode-0600 into that runtime directory when they
appear or change; the broker cannot read the rest of the home directory. It runs
without network access or added capabilities, with a read-only system image,
private devices, and bounded memory and process counts. Loss of the
public bus owner terminates the daemon so the service manager can restart it.

`aiosctl status --json` reports the authenticated session's capabilities and
availability. `aiosctl ask --mode read-only TEXT` and standalone `ask TEXT`
submit the same read-only request and print its answer plus evidence identifiers.
Appending `--json` emits the complete terminal task status for automation. Both
surfaces accept no shell or alternate mode. Closing the connection revokes
ongoing inference rather than leaving detached work. The client pins and
rechecks
the daemon's live owner identity; default endpoints ignore environment
variables that redirect the bus or select another user's runtime directory.

`aiosctl model status` asks the authenticated per-user session broker to inspect
lifecycle, queue and profile state over its credential-checked local model
socket without loading model weights. `aiosctl model unload` uses the same
caller-pinned D-Bus route and requests an unload only when no generation or
queue entry is active; otherwise it returns `CONFLICT`. Both commands accept
`--json`, never submit model text and never expose a tool or shell capability.

`aiosctl package search QUERY` and `aiosctl package info ID` query the
administrator-reviewed installed catalog through the root-owned
`org.aios.Packages1` service. `aiosctl graph status` queries aggregate
reconciliation health through `org.aios.System1`; the privileged broker reads
the fixed graph-owner socket and returns no graph database path or execution
authority. These commands support `--json`, authenticate the root bus owner
before and after each call, and invoke neither inference nor a shell.

`aiosctl privacy scopes` reports only the authenticated client's live
non-persistent broker scope: enrolled file roots, temporary service handles,
retained-history count and active-task count. `aiosctl history list` returns
metadata for that same originating client's eligible volatile history and never
the prompt or answer text. A fresh CLI connection therefore receives an empty
history rather than another local process's tasks. Both support `--json`, invoke
no model and perform no mutation.

Public `Submit` takes the same versioned control request used by the private
transport, with `operation.kind=submit` and a typed `operation.request`. Other
public methods use the PRD's small typed arguments. Task status, event,
cancellation and deletion JSON include version, task ID and operation.
Stable D-Bus errors are named `org.aios.Error.CODE`, using the PRD's codes.
The Executor1 interface declares no task/evidence broadcast signals.

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
private events, forgetting and unsupported write modes. It also requires a real
model-selected native service read and citation, and runs real constrained CPU
probes using labeled hostile document fixtures; those probes execute no proposal. This qualification is
one real UID; it does not establish the installed model sandbox, two-user
inference isolation, graphical consent or the complete orchestration loop.

## Bounded read reasoning

`aiosctl ask "Is the selected service running?" --json --service sshd.service`
resolves the explicitly named service on its authenticated private connection,
then submits that owned handle with the question. A model-proposed service read
must use that exact scope. The default `ask TEXT --json` uses the public session
bus and offers only system information. Graphical questions retain their separate
native receipt and selected-window evidence; they receive no OS read tools.

The broker offers at most eight tools, admits at most twelve proposed reads per
request and permits one structural-output repair for the entire request.
Unknown/disallowed capabilities, unauthorized resources and stale references
terminate the task without repair or alternate-tool fallback. Decisions use a
192-token limit; final answers use 768. All generations share the original
90-second deadline, and Stop revokes the task grant and active native generation.
Each generation owns a separate authenticated model connection, whose teardown
removes its private retained record.

Context contains explicitly labeled untrusted observations and the authenticated
question. A native Submit caller may explicitly set `retain_for_history: true`
to retain that question and its completed response in volatile memory. Retention
is off by default. A subsequent Submit must separately select up to four unique
`history_handles` (completed task UUIDs). Both choices are bound to the full
original authenticated client, including process, boot, session and connection
or unique bus sender. Reconnects and other clients cannot reuse the history.
Graphical submissions reject history retention and selection; their native read
receipt cannot authorize later reuse of private window content.

Selected historical questions and assistant text are labeled untrusted and
carry no current evidence IDs, provider handles, grants or approval receipts.
Only the new question is current intent, and only fresh observations can support
current system facts. Each selected source digest is bound into the new read
scope and rechecked throughout generation and before final result publication.
Forget, expiry, changed source content or originating-client disconnect revoke
reuse. Failed/cancelled tasks discard captured questions. Completed captures
expire five minutes after completion; disconnect wipes owned captures (private
socket teardown immediately, public bus/process loss during native housekeeping).
Owned string buffers are wiped on drop; allocator/compiler copies cannot be
promised erased. No conversation persistence or ordinary prompt logging is added.

Newest history is preferred among explicitly selected entries, but every fresh
observation precedes history. Oldest history is dropped first, then optional
older observations, while preserving the newest observation and the complete
current question. `history_attached`, `history_task_ids`, `dropped_history_count`,
`dropped_evidence_count` and `context_complete` report retained context and loss.
The transport byte bound is separate from the daemon's actual 6144-token input
check. Only a native zero-output/zero-input `CONTEXT_BUDGET_EXCEEDED` rejection
may trigger another bounded assembly after dropping optional context; it does
not consume structural repair or broaden authority. If mandatory context alone
cannot fit, the task fails without truncating current intent. All attempts share
the original task deadline. Partial provider reads return `PARTIAL_RESULT`; answers
must cite at least one retained evidence ID. These checks bind references, but do
not yet qualify arbitrary factual claims or the immutable write orchestration.

System information is freshly observed before inference. Once all required
native observations are available, the broker selects the final-answer stage
with no offered tools instead of asking the model to repeat completed reads.
A selected-service task starts in a constrained read-decision stage that cannot
emit an answer. It offers service inspection until every explicitly selected
handle has a complete native observation. The final answer must cite that service
evidence; citing system information alone returns `STALE_EVIDENCE`. This gate is
deterministic and does not rely on the model following a prompt instruction.
