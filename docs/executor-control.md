# Executor1 preparation transport

`aios-execd --serve` owns `org.aios.Executor1` on the system bus at
`/org/aios/Executor1`. The packaged systemd unit starts it as root. Bus policy
reserves `org.aios` ownership and allows only the fixed control methods and
introspection at this path. The running broker denies program execution,
including inherited worker threads and the x32 syscall route.

`Prepare` accepts the strict `urn:aios:executor-prepare-request:v1` contract in
`schemas/api/executor-prepare-request.json`. It contains a client UUID nonce,
Act mode, request text and a reviewed typed intent. The complete UTF-8 request
must fit 64 KiB. Caller UID, target, baseline, template paths, database facts,
permission receipts and approval flags are never request fields.

Native bus credentials, process start time, boot identity and logind association
determine the original requester. Preparation reads the system profile's
persisted managed declaration independently of the running system. It records
the running closure, system profile closure, exact boot-selected closure and
boot metadata separately. Unknown firmware selection, payload disagreement,
revision drift and target changes fail closed.

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

This transport does not start builds or activate systems. `GetCapabilities`
reports these operations unavailable. `Authorize` and `Execute` reject missing
trusted confirmation with `AUTH_REQUIRED`; an incorrect digest returns
`PLAN_CHANGED`, expiry returns `APPROVAL_EXPIRED`, and a cancelled plan returns
`CANCELLED`. `RequestRollback` reports `UNSUPPORTED_CAPABILITY`. A preliminary
preview is never final authorization for an exact built closure.

Database-data and unfree acknowledgement adapters are not supplied by request
JSON. Intents needing these facts or grants remain refused until their trusted
adapters exist. Protected SSH transport changes remain refused.

The fixed `--native-preflight` entry point checks installed target/template/
approval authority without minting a caller, confirmation or activation grant.
Installed-image qualification runs the explicitly ignored `installed_transport`
test through the `installed-executor` guest provider, which retains its verified
SSH caller session until completion. Package/SQLite fixtures in
`broker-preparation` qualify their narrower behavior separately.
