# Trust boundaries

Protect against target confusion, accidental destructive intent, hostile retrieved
content, forged/stale handles, cross-user disclosure, replayed approvals, unsafe
Nix candidates, parser faults and lost management connectivity during activation.

Direct authenticated user intent and approved rules confer bounded task authority.
Documents, UI text, logs, events and model output are untrusted evidence. The model
cannot approve, escalate, load adapters, access credentials or bypass denials through
a different input modality. Same-UID unsandboxed malware, guest root, a compromised
kernel and malicious host administrators are outside the advertised protection.

Runtime paths are canonicalized below project-owned `.local/vm/`; keys and pinned
host trust live below `.local/ssh/`. Relative traversal and symlink escapes are
rejected before any operation. No host credentials enter source snapshots or guests.
Enrollment must establish SSH trust through the console/seed path, never an
unverified key scan. TCP reachability is not guest identity verification.

The model and extractors have no tools/root/network/home/keyring/Nix-daemon access.
The privileged broker executes reviewed typed adapters and durably records effects.
Parser workers are isolated and bounded. Private evidence is scoped per UID/session
and not broadcast. Approval UI, protected fields and terminals cannot be operated
as a policy workaround.

Boot and deterministic tools remain available without inference. Cold snapshots
require stopped images and include UEFI state. Recovery must distinguish reversible
configuration, compensatable file effects, data migrations and irreversible external
effects. Ambiguous identity and unsupported recovery require human decisions.

Security acceptance uses finite adversarial/failure tests from PRD 21; passing
those tests is not a claim of perfect safety. Private test data and diagnostics
remain ignored locally or in Linear; distributed artifacts contain sanitized data.

The disposable model acceptance image contains a fixed initial root fixture
with `CAP_KILL`/`CAP_SYS_PTRACE` for one named model unit's restart/PSS checks,
and `CAP_SETUID`/`CAP_SETGID` to permanently drop five forked clients to fixed
normal UIDs. These clients close inherited coordinator/model descriptors before
connecting to inference and have zero effective/permitted/inheritable/ambient
capabilities. Coordination is bounded JSON on private socket pairs, with actual
kernel UID/GID/start identity rechecks. There is no public root RPC, arbitrary
unit/process/command selector or model invocation of the fixture. Three extra
load accounts cannot log in and have no SSH keys/home. The module asserts a
development image; production excludes this fixture and its privilege bounds.
Public failure instrumentation additionally binds only `/run/user/1001` under
an empty home view, starts a copied original hardened user unit as the permanently
dropped tester, and requests only fixed CLI ask/service-inspection and read-only
KWin operations. Its root write exception is one root-only runtime model drop-in
directory: the compiled one-byte corruption profile supplies a fixed read-only
GGUF binding, never a caller-selected path or writable mount. The fixture checks
actual daemon-view and unchanged store digests, removes only its exact owned
drop-in, restores the original view and verifies recovery. It has no SYS_ADMIN
capability and exposes no root RPC. Pending phase files are observations for
host verification, never approvals, task authority or production heartbeats.

The graphical native provider requires the desktop's original user namespace
for kernel process/executable verification. Unprivileged user-unit mount/network
isolation implicitly creates another user namespace and blocks those checks.
This provider has an explicit filesystem/namespace exception, described in
[native consent](native-consent.md): it retains the native UID's filesystem view
and kernel capability bounding set, verifies zero effective/permitted/inheritable/
ambient capabilities and active NoNewPrivileges at startup, allows only Unix
socket address families, and bounds system calls, namespace creation, memory and
tasks. It contains no model, arbitrary file/shell route or dynamic executor.
Its exact managed service and original client descriptor are reauthenticated
per request. The orchestration broker keeps its complete mount/user/network
sandbox. A compromised native provider would have ordinary same-UID filesystem
access; this exception is part of the trusted native component boundary.

The fixed headless `aios-processd` component uses the same original-user-namespace
exception for native own-user proc/pidfd observations. Its user unit retains the
native filesystem view, restricts address families to Unix sockets, and drops
and verifies all usable capability sets and NoNewPrivileges before admission.
Only the exact installed `aios-sessiond` managed peer may connect. The peer
transfers an originating Unix descriptor or a unique bus sender resolved against
the kernel-authenticated root-associated user bus. No serialized UID/PID/session
can confer authority. Embedded user policy checks bracket native observations;
the original caller is reauthenticated before and after each operation. The
fixed observation bridge accepts process.list and process.inspect, with bounded
owner/expiry handles and cursor state. Its separate human Act lifecycle permits
one retained-pidfd SIGTERM only after exact native desktop confirmation of the
original owner, process identity, selected session and current closure. Model
tool calls and serialized approval fields cannot enter this route. A dedicated
broker cancellation socket can only revoke that task, including at final
delivery checks; no signal override, escalation or numeric-PID fallback exists.
The component has no shell, arbitrary file, model or desktop input route.
Four admitted connections and four effect workers bound native descriptor
retention; disconnect cancels work before shared caller cleanup. Its potential compromise has
ordinary same-UID filesystem access, explicitly within this native component
boundary. The orchestration broker's full sandbox remains intact. The component
starts in the user manager independently of a graphical session and inference.

The root journal observer uses empty home-directory views (`ProtectHome=tmpfs`)
and a read-only binding of the fixed `/run/user` directory. The binding must
include runtime directories created after observer startup; individually
binding absent user sockets would not support later logins. This explicitly
exposes user runtime metadata and Unix sockets to the trusted root component.
It does not expose home directories or provide a caller-selected file or bus
endpoint. Only immutable configured normal readers may resolve user units.
The adapter derives the UID from the native originating system-bus sender,
checks the root-managed user manager's compiled executable and incarnation,
then checks the actual kernel socket peer and manager's unique bus owner.
Its fixed calls read unit identity/properties; no arbitrary method, launch,
environment, credential or user-runtime file read is exposed. Unit/manager
restart, endpoint replacement, owner change and caller loss invalidate the
binding. A five-second caller deadline and four retained worker slots bound
stalled user buses without changing authorization or escalating permissions.
