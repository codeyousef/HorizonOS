# Executor1 preparation transport

`aios-execd --serve` owns `org.aios.Executor1` on the system bus at
`/org/aios/Executor1`. The packaged systemd unit starts it as root. Bus policy
reserves `org.aios` ownership and allows only the fixed control methods and
introspection at reviewed paths. It also owns `org.aios.System1` and exports
`/org/aios/System1` and `/org/aios/Packages1`. All three interfaces authenticate
native callers through the same admission and identity checks. The running broker denies program execution,
including inherited worker threads and the x32 syscall route.

System1 `Info` observes NixOS and systemd directly without executing a detector.
`Hardware` reads a fixed, bounded CPU source plus native libudev/sysfs block-disk
properties and cross-checks each supported block identity/size against the fixed
UDisks2 Block object. It issues caller-scoped opaque device handles, redacts
serial and WWN values to availability flags, and reports missing UDisks objects,
classes or properties as unsupported. `StorageStatus` reads only
`/proc/self/mountinfo`, `statvfs` for
visible mounts and the authenticated UID's native account home. It exposes fixed
system mount paths plus caller-owned home/removable mount paths; block
major/minor/filesystem identity is distinct from capacity and mount path. Neither
provider opens raw block devices, writes storage, accepts a path, or offers
format/repair actions. Reads are bounded to 1 MiB/4,096 mount rows, 100 returned
mounts, 1 MiB CPU data, 4,096 udev entries and 256 block disks; each observation
has a two-second native device deadline. The executor's only persistent writable
sets remain `/var/lib/aios/candidates` and `/var/lib/aios/transactions`.

`NetworkStatus` reads the fixed NetworkManager service and exposes link state,
connectivity, DNS state, and endpoint reachability as separate fields. Native
object paths stay internal; interface handles bind the authenticated caller.
Wi-Fi radio absence remains explicit rather than becoming a healthy disabled
radio. `BluetoothStatus` reads the fixed BlueZ object manager and kernel rfkill
state, redacts hardware addresses into caller-scoped handles, and reports an
absent adapter as `UNSUPPORTED_CAPABILITY`. Both reads pin the upstream D-Bus
owner and bound object counts. `NetworkSetWifiEnabled` is R3 and requires the
`transport-guard`; `BluetoothSetEnabled` is R2. Both mutation members remain
unavailable and return `AUTH_REQUIRED` until exact-plan native approval,
live-baseline verification, postconditions, and recovery are installed. This
also prevents either action from disabling the development SSH transport.

Packages1 `Info` and `Search` read the installed administrator-reviewed catalog.
Action members accept the strict versioned invoke envelope and fix the reviewed
action in server code. Duplicate/unknown fields, action substitution and forged
authority are rejected. Provider outputs pass their normative result schema.
Search cursors bind the full native caller, exact query/limit and catalog revision,
expire after 30 seconds, and have finite per-UID/global budgets. A reconnect cannot
reuse a cursor. No private results are emitted as signals. Capabilities identify
the available read providers; the remaining action contracts stay unavailable.
Direct service writes require authorization, and package planning methods await
their native plan adapters. Catalog metadata alone never starts a package install.

`Prepare` accepts the strict `urn:aios:executor-prepare-request:v1` contract in
`schemas/api/executor-prepare-request.json`. It contains a client UUID nonce,
Act mode, request text and a reviewed typed intent. The complete UTF-8 request
must fit 64 KiB. Caller UID, target, baseline, template paths, database facts,
permission receipts and approval flags are never request fields.

Native bus credentials, process start time, boot identity and logind association
determine the original requester. A desktop process delegated to the per-user
systemd manager may have no direct logind session; only in that case, or when
logind reports the manager session, the broker resolves the same UID's root-owned
`User.Display` pointer and requires an active, local X11/Wayland user session.
A concrete SSH or TTY session is never replaced by the display session.
Preparation reads the system profile's persisted managed declaration
independently of the running system. It records the running closure, system
profile closure, exact boot-selected closure and boot metadata separately.
Unknown firmware selection, payload disagreement, revision drift and target
changes fail closed.

The broker seals the candidate and registers its plan in the root-owned durable
ledger. A repeated nonce from the same authenticated connection returns the same
plan; different request bytes conflict. Plan reads and cancellation bind the full
caller identity, including the unique bus sender. Reconnecting requires new
authorization and cannot retrieve an old plan by knowing its ID.

`GetPlan` returns the immutable preparation and `GetTransaction` returns ledger
state/history privately. `CancelTransaction` records pre-effect cancellation
durably and is idempotent. Lost caller authority cancels pre-effect work. Broker
restart cancels orphaned pre-effect plans; a building transaction retains its
slot until worker termination is independently proven.

`Authorize` has two exact, broker-owned phases. For a `PLANNED` preparation it
renders the immutable preliminary semantic preview and fixed 16 GiB build,
4 GiB download, 8 GiB recovery-reserve and approved-cache limits on the
authenticated caller's foreground TTY. The exact `BUILD <plan-id>` phrase creates
only an in-memory resource permission; it cannot authorize activation. The
broker records that permission durably before connecting asynchronously to the
fixed installed `aios-buildd` socket.

The broker authenticates the builder UID, process, systemd cgroup, store
executable and socket both before and after the bounded request. It sends only
the registered candidate, installed-template digest, managed digest, observed
running closure and approved resource limits. A reply becomes a non-serializable
capability only after the candidate and separately retained prior/candidate GC
roots are rechecked. Native target and baseline are re-read before the ledger
records the result and freezes the final plan. `GetPlan` then returns that final
plan and exact hash. A second `Authorize` performs trusted exact-plan TTY plus
fresh polkit approval. Because that volatile receipt is bound to the exact
process and system-bus connection, `aiosctl transaction apply` performs final
authorization and `Execute` on one connection; a standalone final
`transaction authorize` intentionally cannot transfer authority to a later CLI
process. An authenticated, request-bound worker failure durably enters `FAILED`
and releases the active-plan slot. Ambiguous worker termination or baseline
drift retains the build lock and never yields a final plan or activation
authority.

The system-bus policy exposes only the typed `GuardStatus` and
`GuardHeartbeat` management members alongside the transaction API. Runtime
code still requires the transaction owner UID, an active remote management
session, the enrolled SSH development channel, exact authorized ledger state,
and a matching durable guard handoff before either call reaches the guard.

Database-data and unfree acknowledgement adapters are not supplied by request
JSON. Intents needing these facts or grants remain refused until their trusted
adapters exist. Protected SSH transport changes remain refused.

The fixed `--native-preflight` entry point checks installed target/template/
approval authority without minting a caller, confirmation or activation grant.
Installed-image qualification runs the explicitly ignored `installed_transport`
test through the `installed-executor` guest provider, which retains its verified
SSH caller session until completion. Package/SQLite fixtures in
`broker-preparation` qualify their narrower behavior separately.
