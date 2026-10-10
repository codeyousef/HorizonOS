# Architecture and ownership

The host owns Codex, the canonical Git checkout, dedicated SSH private keys,
QEMU/QMP/serial control, cold snapshots and pulled reports. The guest owns NixOS,
build tools, immutable source snapshots, application fixtures and CPU inference.
Transfer source over authenticated SSH; no writable home sharing or agent forwarding.

Rust services separate system observation (`aios-state`, `aios-observer`), root
execution (`aios-exec`), unprivileged builds (`aios-build`), independent activation
guard (`aios-guard`), user orchestration (`aios-agent`), CPU runtime (`aios-model`),
per-user indexing/extraction and session UI control. Qt6/KF6 frontends use the same
authenticated control APIs as CLI clients. PID 1, login and networking have no
inference dependency.

The protocol crate supplies bounded UTF-8 transport framing. JSON/schema, peer
authentication and action policy must be applied above framing; a parsed frame
alone grants no authority. The normative action schemas are PRD sections 8, 10
and 27. SQLite stores scoped observations/evidence and a separate durable ledger.

Factual retrieval is protocol-owned and ordered: fresh typed-provider evidence,
documentation bound to the current installed closure and document digest,
authorized file handles bound to current content digests and exact ranges, then
an optional explicitly authorized HTTPS adapter. The default has no external
adapter or credential input; a request that needs fresh external facts returns
`NETWORK_REQUIRED`. Evidence records are request-scoped and complete. Before an
answer is returned, deterministic code resolves every cited ID, rechecks source
freshness/version/scope, and compares observed scalar claims to their structured
JSON values. Hypotheses and missing evidence remain explicitly distinct.
Rendered locators are inert provenance (`executable_uri=false`,
`execution_authority=false`); model text cannot select a viewer, URI, handle,
policy, or effect.

Reviewed base modules own hardware/users/storage/SSH/security. A fixed locked
template imports allowlisted managed JSON for supported declarative changes.
User settings have scoped receipts. The model never edits Nix expressions, locks,
policy, keys or adapter code. Developer deployment is separate guest-root authority
and is excluded from production.

Transaction approval binds exact targets, closure, policy, impact and expiry.
Activation uses the already built closure, an independent retained guard and real
health checks. Configuration rollback does not restore user data or downgrade a
database. Recovery uses verified generations, independent media and host controls.

Required source domains are `crates`, `native`, `nix`, `schemas`, `capabilities`,
`policies`, `models`, `prompts`, `tools`, `dev`, `tests` and `docs`. Add executable
providers and schemas with real implementations and tests, rather than plausible
placeholder responses. Required flake release outputs are specified in PRD 6.2.
