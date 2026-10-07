# Own-user process observations

`aios-system::processes::OwnProcess` retains a native Linux pidfd and the
original proc directory. Construction captures the current non-root effective
UID, PID, process start ticks, boot ID and executable device/inode/timestamps.
It verifies procfs type, directory ownership and all four native UID fields.
Mixed privileged credentials are refused. Inspection checks the original
identity twice and refuses a changed executable, ownership, boot, start time
or exited process. A reused PID cannot redirect the retained proc directory
or pidfd to a newly selected process.

The observation contains CPU time, resident memory and thread count. Numeric
values fit the protocol's safe integer range. Fixed proc fields are bounded to
64 KiB. Process names, executable paths, command lines, environment values and
arbitrary proc fields are not returned. The executable locator identifies the
native file object; it is not a binary content hash.

The session broker's typed direct observation route issues private process
handles and immutable snapshot cursors. Both bind the full originating peer
(including connection identity), expire after 30 seconds on a suspend-inclusive
clock and are revoked on private-client disconnect. A cursor also binds the
query limit, snapshot and offset. Every returned process is inspected live;
an exited or changed snapshot member produces an explicit error. Inventories
are capped at 256 retained processes, 32768 proc entries and two seconds;
pages contain at most 100 entries. Quotas bound retained native descriptors.
Unreadable own-user processes produce a `partial` result with a
`PERMISSION_DENIED` explanation, including on the final page. An unreadable
inventory never becomes a complete healthy empty result. Resource exhaustion
and malformed native records return errors rather than truncated success.
Application filters are explicitly unsupported until native application
associations are implemented. Reads check the same policy grant before and
after native work, and reauthenticate the originating peer at both boundaries.
`org.aios.Agent1.ListProcesses` and `InspectProcess` accept strict versioned
requests and fix their action identifiers on the server. They cannot dispatch
other actions or accept a caller-selected UID, PID or privileged namespace.

The public observation routes provide no signal or termination method.
Authenticated `Submit.context_handles` can select existing process handles as
well as service handles. The broker resolves each selected process through the
same original client connection before minting the task's opaque read grant.
Its frozen resource includes PID, UID, start ticks, boot and executable identity.
Only selected `process.inspect` calls can be offered to inference; the model
cannot enumerate additional processes, create handles or change the selection.
The original task grant is checked before and after native I/O without holding
the task-state mutex across the call. Cancel, Forget, disconnect and expiry
therefore prevent an in-flight result from being admitted as task evidence.
The fixed helper's internal task read rechecks the original client and live
native handle; it does not mint a new grant from model arguments. Task authority
stays in the managed broker and is never exported as a serialized token.
Selected processes require their own current evidence citations before a final
answer. Process evidence does not substitute for selected service evidence.

Global process capability availability and graceful effects remain unavailable
until the corresponding installed/model and consent gates are qualified.
Broker unit tests use actual proc/pidfd observations and native
caller identity; the two-entry cursor corpus is controlled, not a claim of an
installed inventory qualification.

The native descriptor lifetime follows the Linux
[pidfd documentation](https://man7.org/linux/man-pages/man2/pidfd_open.2.html).

Installed qualification uses the fixed foreground command:

```fish
python3 tools/devctl.py test --suite integration --provider process-inspection --json
```

The probe retains the actual SSH/PAM caller, verifies the user broker against
the root-associated native user manager and its installed ExecStart, and calls
the public typed methods. A controlled child exits naturally; independent
proc/pidfd observations check its identity and exit. Real cursor continuation,
same-PID connection isolation, query drift, claimed-UID refusal and 30-second
expiry are required. Missing or failed cases cannot yield a passing report.
This gate does not establish approved termination, model task grants,
cross-UID caller isolation or actual PID reuse. Its ignored Rust test is
required by this native gate and does not count as an ordinary unit-test pass.

The separate selected-process task gate requires the installed normal CPU-model
development image:

```fish
python3 tools/devctl.py test --suite integration --provider process-task --json
```

It retains the original authenticated Unix client, selects that client's own
process, and compares native PID/start/boot/executable identity with the cited
process evidence in the actual model answer. Foreign handles, foreign task
access, unknown selections, forgotten tasks and expired selections must fail.
The original managed broker is attested and reused; cleanup never stops or
replaces it. This gate does not establish persistent bus task behavior,
in-flight revocation/disconnection/expiry, cross-UID callers, signal effects or
actual PID reuse. Missing completion, native citations or required refusal
cases cannot pass merely because the probe exited successfully.

A native regression test creates a controlled non-dumpable own-user child.
It verifies actual process credentials and inaccessible executable identity,
requires an explicitly incomplete inventory, and lets the child exit naturally.
This is library verification in the guest, not installed broker qualification.

`services.aios.session.enable` registers the reviewed package's user units and
starts `aios-sessiond.service` through the native user manager's
`default.target`. A fixed path unit watches only the two registered KDE settings
files and projects bounded copies into the broker's private runtime directory
when either appears or changes. The broker reads only those projected files,
including after late Plasma initialization or atomic configuration replacement;
it cannot read the rest of the home directory.
The development image enables this independently of the incomplete global
control-plane switch and model activation. Missing reviewed broker packages
fail NixOS assertions. This wiring preserves the packaged sandbox and requires
installed qualification; package presence alone does not establish a bus owner.

Installed broker transports forward native process reads to `aios-processd`, a
fixed headless same-UID component in the original user namespace. The broker's
private user namespace prevents kernel executable checks against normal user
processes. The process component has an explicit namespace/filesystem exception
documented in the threat model; it drops usable capabilities at startup and
provides no shell or arbitrary-file API. Its fixed native-confirmed termination
lifecycle is separate from public process tool calls. Its mode-0600 socket is
accessible only to the exact managed broker. Native bus references or the actual
originating Unix descriptor bind each retained connection and its process
handles. Shared owner-aware state retains cross-connection permission denials;
the same embedded policy checks bracket native work and original caller proof
is rechecked before results leave the component. Four active connections and
the existing global handle/cursor quotas bound resource retention. Idle bridges
close after 95 seconds; handles still expire at 30 seconds without renewal.
The user-manager module starts this component without a graphical dependency.

The foreground `process-task-bus` integration provider compiles a native test
client from frozen source inside the verified guest. It retains one actual
D-Bus unique sender through process selection, Submit and the CPU answer, and
compares the cited PID/start/executable locator with independent native identity.
A second connection from the same process must not reuse the selection or read,
cancel or forget the original task. The provider also observes active inference
before cancelling and forgetting separate tasks: cancellation must finish within
two seconds with no output, and forgotten data must remain inaccessible during
the subsequent bounded observation. It attests the original installed broker
before and after, and requires the installed normal model lock to match source.
This does not establish cross-UID isolation, in-flight handle expiry, disconnect
cleanup, graceful signal effects or real PID reuse. No replacement broker or
model is started by this provider.

The native termination adapter freezes a retained own-user proc directory and
pidfd into an immutable preview with PID, UID, start time, boot and executable
identity. Preparing a plan sends no signal. The adapter accepts only one SIGTERM
attempt, refuses self-termination, and provides suspend-inclusive bounded exit
verification through that same pidfd. It never falls back to a numeric PID,
process group, shell or SIGKILL. An ignored signal, timeout, post-signal Stop or
verification error retains a partial-effect receipt; it cannot claim rollback.
Polling does not sleep or acquire a broker task lock. Dropping the verifier
does not signal the target.

This adapter does not issue permission. Its trusted broker callback must check
the original authenticated client/task, exact R2 approval, canonical preview,
policy/boot/closure, current resource and expiry on both checks before delivery.
Native child tests use synthetic authority callbacks and independently observe
SIGTERM acknowledgement, natural exit, refusal and ignored-signal survival.
Those tests are library evidence, not installed consent or public termination
qualification. The generic public tool bridge still rejects signal actions and the
registry remains contract-only until native consent and original task ownership
are connected and installed verification succeeds. A pidfd prevents PID reuse;
process executable identity is rechecked before delivery, without promising
atomic exclusion of an unsandboxed same-UID concurrent exec.

The native process task worker joins that adapter to the exact native policy
confirmation. A detached selection duplicates the retained proc/pidfd objects,
preserves the original owner and handle expiry, and shares a revocation flag
with its source handle. Removing that handle revokes detached selections; it
never signals the process. The worker retains the original kernel connection
or native bus proof, explicitly selected verified display, current immutable
system closure and one prepared effect. It checks all bindings through native
confirmation and both final delivery callbacks, then checks caller, handle
lifetime, Stop and approval deadline again before returning to kernel delivery.
Pre-effect setup failures emit only a fixed stage name and typed error code to
the service journal. Diagnostics never include the goal, target, process
identity, session locator, resource handle or approval material.

Stop withdraws the actual prompt channel and cancels the effect verifier
independently of native queries. After signal delivery, receipts preserve the
sent signal and report independently observed exit or partial effects;
Stop or original-client disconnect cannot falsely report no mutation.
The worker never retries an effect and returns stable terminal receipts. It runs
outside global task state. Its retained managed-broker proof also binds the
exact provider connection and original service invocation, including both final
delivery checks. A disconnected or replaced broker cannot leave a pending
effect authorized through a still-live originating client.

The private managed bridge accepts closed `start_process_termination`,
`get_process_termination`, `cancel_process_termination` and
`forget_process_termination` operations. Start requires an original Act request,
UUID task/process handles, bounded goal and an explicitly selected native
session. Credentials, signal choices, approval tokens and caller-supplied
closure/timeout fields are rejected. The native display is resolved inside the
worker; no inherited display or cross-user fallback is used. Start queues work
and returns promptly. Only native confirmation can authorize its one effect.

At most four termination workers run across provider connections. Each
connection retains at most eight task records and 4096 used task IDs; forgetting
does not permit replay. Admission is released on worker completion or failed
thread creation. Tasks have a fixed 90-second suspend-inclusive budget; process
handles keep their original 30-second lifetime. Idle transport expires after
95 seconds. Status and Stop exchange only small owned task records, while
native consent/observations run outside those locks. Disconnect latches Stop
before shared inventory cleanup, even if another native read holds process
state. Forget cancels work and removes its owner-visible status; worker-owned
transient state is released when that bounded worker exits.

Cancel acknowledges the request separately from the final outcome. A signal
already sent remains in the final partial receipt, with verified exit and
completion reported independently. Terminal facts are not erased by Stop.
Records are accessible only through their original managed bridge connection;
reconnection cannot recover or control another connection's task. The model
tool route still refuses termination. Installed production consent and
end-to-end effect qualification remain required before advertising this
capability in the registry.

Authenticated human clients can use the separate public termination lifecycle
on the private Unix connection or original native D-Bus unique sender. Start
requires Act mode, a UUID task ID, an existing process handle and an owner-bound
selected desktop handle. The broker resolves that desktop selection and checks
the process observation before queuing work on the same retained native helper
connection. It freezes the observed process identity and compares any final
receipt against that identity, the original UID and boot. A plain State call,
claimed credentials, native session string or model tool invocation cannot
construct this route's authority. Approval remains exclusively native.

Start transfers a one-way cancellation socket from the verified broker. Stop
closes it without waiting for the process observation/client lock; the helper
watcher latches cancellation and withdraws its prompt even while status state
is contended. Final delivery also polls the native socket directly, independently
of watcher scheduling. Any byte, EOF, socket failure or deadline expiry only
revokes the task. It cannot select a target, renew authority or approve work.
Forget requests Stop first. If the native channel is busy, it returns a bounded
resource error while retaining owner state so deletion can be retried. Terminal
partial receipts remain truthful. A failed/unconfirmed start is never replayed;
transport errors do not imply that an irreversible effect was rolled back.
Four reserved public control admissions keep process Stop/Forget separate from
the sixteen ordinary request slots; both pools remain bounded. Native provider
connection setup and reads run outside the broker's connection-table lock.

The deterministic CLI keeps one native bus sender through inventory, session
selection, start and polling:

```text
aiosctl process list --json
aiosctl process terminate SESSION BOOT_UUID PID START_TICKS GOAL --json
```

Inventory includes its observed native boot UUID. Termination re-lists on its
own connection and requires that exact boot/PID/start tuple. It refuses a reused
PID, ambiguous inventory or a target not observed in the bounded own-user
snapshot. Numeric selectors are never sent to the provider's effect route;
only the retained process handle is passed. The native desktop shows the exact
SIGTERM preview and defaults to Cancel. The CLI returns the final receipt and
uses a nonzero exit for partial, failed or cancelled outcomes. Its source/API
presence alone does not establish installed desktop/effect qualification.
If start acknowledgement or later status cannot be obtained, the CLI requests
Stop and reports an explicitly unverified effect outcome with the real error.
It does not label a missing receipt as no mutation or retry termination.
