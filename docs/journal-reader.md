# Native journal reader contract

`aios-system::journal` reads the local default journal namespace through the
pinned libsystemd API. It accepts a normalized source, optional system/user
unit, boot identity, absolute microsecond time window, priority and entry limit.
The reader captures its effective UID and current machine/boot. User records
must match that UID. System records exclude user-manager/session records;
kernel records require the native kernel transport.

Storage is limited to the current machine directory under `/run/log/journal`
and `/var/log/journal`. No input selects paths, journal fields, namespaces or
expressions. Required system and own-user files are opened without following
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
policy grant, expiry and query. It must check authorization both before and
after provider I/O and resolve unit/boot scopes and journal namespaces live.
The library does not issue grants or change `system.logs` runtime availability.

The development integration command exercises controlled public journal
messages and an independent filtered upstream read. It covers source/unit/
boot/time/priority/entry limits, native cursor continuation and redaction.
Required native cases fail instead of skipping missing access or data:

```fish
python3 tools/devctl.py test --suite integration --provider journal-inspection --json
```

This is native library qualification, distinct from installed broker and
caller-grant qualification. The native writing fixture is ignored by ordinary
unit tests and executed explicitly by that registered integration command.

The API signatures and descriptor ownership follow the pinned upstream
[header](https://github.com/systemd/systemd/blob/v260/src/systemd/sd-journal.h)
and [implementation](https://github.com/systemd/systemd/blob/v260/src/libsystemd/sd-journal/sd-journal.c).
