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
