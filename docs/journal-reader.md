# Native journal reader contract

`aios-system::journal` reads the local default journal namespace through the
pinned libsystemd API. It accepts a normalized source, optional system/user
unit, boot identity, absolute microsecond time window, priority and entry limit.
The reader captures its effective UID and current machine/boot. Direct user
records must match that UID. The privileged observer can instead bind the
query to a sealed native system-bus user observation; callers cannot construct
that observation from a claimed UID. System records exclude user-manager/session records;
kernel records require the native kernel transport.

Storage is limited to the current machine directory under `/run/log/journal`
and `/var/log/journal`. No input selects paths, journal fields, namespaces or
expressions. File selection includes journald's recoverable `.journal~` archives after an
unclean shutdown. Archive names remain restricted to system or the bound
user's own files. Required journals are opened without following
symlinks; directories/files must be root owned and not writable by other users.
Each file must remain the same object with current DAC/ACL read access. An
inaccessible required file returns an explicit permission error. Zero files
never imply a healthy empty journal. Unmapped ownership in a user namespace
fails closed; an observer must supply correctly authenticated native access,
not relax ownership checks or add the model to a log-reading group.

The API passes checked file descriptors to libsystemd, retaining independent
identity/access descriptors. It verifies native boot membership and applies
machine/boot, unit, user, kernel and priority matches. Returned entries must
satisfy the captured time window and native metadata. Queries have at most 128
files, 10,000 scanned entries and 200 returned entries. A five-second budget
is checked around native calls; the surrounding observer still needs its own
task deadline and resource/cancellation supervision.

Only selected provenance fields and sanitized messages become observations.
Credential labels, bearer/authorization data, connection URLs, environment
assignments and command arguments cause conservative whole-line redaction.
Private-key blocks, invalid UTF-8, deceptive controls and oversized messages
are completely redacted. Normal Unicode diagnostic lines are preserved. Raw
message buffers, arbitrary journal fields, environment and process command-line
fields are never serialized or hashed as evidence. This detects obvious secret
formats; it is not a guarantee that arbitrary unlabelled text is non-sensitive.

A continuation is private process-local state containing the exact normalized
query and native cursor. It is not serializable and cannot be constructed from
request bytes. Changed queries are refused, and resumption must find the exact
cursor; seeking to a nearest entry is not accepted after rotation/vacuum. The
broker must separately bind an opaque wire cursor to its authenticated peer,
policy grant, expiry and query. It checks authorization both before and
after provider I/O and resolves unit/boot scopes and journal namespaces live.
The library does not issue grants or change `system.logs` runtime availability.

The installed root `org.aios.System1` observer checks its authenticated unique
sender, UID, PID, process start, originating logind association and bus/boot
identity. Only normal users listed in the immutable installed
`/etc/aios/journal-readers.json` are eligible; inference system accounts are
excluded. The native journal query independently observes and rechecks the
same bus sender and process. The service does not accept a raw UID, path,
journal expression or caller-created native cursor.

`ResolveLogService` resolves an existing system service into a private,
30-second handle. `Logs` takes the reviewed `system.logs` request schema.
Its scope checks run before and after native I/O. Wire cursors preserve the
original normalized query and are bound to the complete authenticated peer,
argument hash and 30-second expiry. A changed query, reconnect, expired
reference or disappeared native cursor fails explicitly. There are at most
4,096 live service/cursor handles each, with 256 per UID.

`ResolveUserLogService` resolves a service only through the authenticated
caller's native user manager. The root system manager must identify the active
`user@UID.service` and its compiled systemd executable, PID, start time and
invocation. The fixed runtime bus socket must belong to that UID, and its
kernel peer and systemd bus owner must match the root-managed process. The
handle binds the unit object and invocation, manager identity and socket
identity. Restarting the unit or manager invalidates it; callers must resolve
a fresh handle. User handles infer the user source and reject a system or
kernel source. No request selects a UID, bus endpoint or arbitrary method.
Each resolution/recheck has a five-second caller deadline and a maximum of
four native workers. Timed-out workers retain their slot until they finish.

`GetJournalEvidence` accepts an observer-issued evidence ID. It returns only
sanitized payloads, their content hash and native boot/cursor locators to the
same authenticated peer. Evidence expires after 30 seconds, with at most
8,192 live records and 1,024 per UID. Batch records link the selected sanitized
entries; their cursor is empty and must not be treated as an individual
journal entry. Neither these direct normal-user reads nor their temporary
read grants authorize a model task or a mutation. The model-facing
`system.logs` capability remains unadvertised until installed orchestration
and its required scope/timeout qualification are complete.

The development integration command exercises controlled public journal
messages and an independent filtered upstream read. It covers source/unit/
boot/time/priority/entry limits, native cursor continuation and redaction. It
also waits through the actual 30-second handle lifetime and requires explicit
refusal of expired cursor, evidence and service references. Prior-boot cases
require three controlled messages from a previous invocation in the same
guest, followed by a verified reboot. Missing historical fixtures fail the
gate; they are not silently skipped. Independent journalctl metadata binds
each historical entry's cursor, UID, timestamp and boot. Both entry and batch
evidence must retain the selected historical boot, and those messages must
be absent from the corresponding current-boot time window.
Required native cases fail instead of skipping missing access or data:

```fish
python3 tools/devctl.py test --suite integration --provider journal-inspection --json
```

The registered command invokes the installed System1 observer test. It
requires the corresponding service from the same source in a verified guest;
an old installed service or missing native access must fail. Controlled
writing fixtures are ignored by ordinary unit tests. The separate direct
library test remains available as a diagnostic and is not installed observer
evidence. Neither test alone qualifies model task grants or the complete OS.
The probe retains its authenticated SSH/PAM session throughout compilation
and native calls. `--detach` is rejected: a worker left in a closing login
session must not be granted a replacement originating caller identity.

The API signatures and descriptor ownership follow the pinned upstream
[header](https://github.com/systemd/systemd/blob/v260/src/systemd/sd-journal.h)
and [implementation](https://github.com/systemd/systemd/blob/v260/src/libsystemd/sd-journal/sd-journal.c).
