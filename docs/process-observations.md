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

There is no signal method in the observation library and no termination route.
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
starts `aios-sessiond.service` through the native user manager's `default.target`.
The development image enables this independently of the incomplete global
control-plane switch and model activation. Missing reviewed broker packages
fail NixOS assertions. This wiring preserves the packaged sandbox and requires
installed qualification; package presence alone does not establish a bus owner.

Installed broker transports forward native process reads to `aios-processd`, a
fixed headless same-UID component in the original user namespace. The broker's
private user namespace prevents kernel executable checks against normal user
processes. The process component has an explicit namespace/filesystem exception
documented in the threat model; it drops usable capabilities at startup and
provides no signal, shell or arbitrary-file API. Its mode-0600 fixed socket is
accessible only to the exact managed broker. Native bus references or the actual
originating Unix descriptor bind each retained connection and its process
handles. Shared owner-aware state retains cross-connection permission denials;
the same embedded policy checks bracket native work and original caller proof
is rechecked before results leave the component. Four active connections and
the existing global handle/cursor quotas bound resource retention. Idle bridges
close after 35 seconds; handles still expire at 30 seconds without renewal.
The user-manager module starts this component without a graphical dependency.
