# AIOS protocol contracts

Draft 2020-12 schemas are normative. `actions/*.arguments.json`, `*.data.json`
and `*.result.json` cover all 59 initial actions. `registry/actions.json` is the
reviewed contract manifest. An action with `availability=contract-only` has no
executable provider. Public capability discovery advertises qualified providers
separately; adding a contract never grants the model a new tool.

`aios-protocol/build.rs` generates concrete Rust structs, enums, argument defaults,
per-contract validated parsing functions and the discriminated `Action` from
these schemas. Raw `Deserialize` implementations are structural helpers, not an
input boundary. Use `parse_tool_call`, the generated `parse_*` functions and
`validate_result`; duplicate keys are rejected before schema validation. Input is
bounded to 64 KiB for tool calls and 1 MiB for provider frames. Schemas are
compiled from local embedded strings with HTTP/file resolution disabled and
RFC 3339 format assertions enabled.

Resource syntax is separate from authority. Before execution, trusted broker
code must call `registry::validate_references` with an authenticated requester,
current enrolled grants and installed catalogs. The resolver checks owner, kind,
expiry, revocation, query, application and snapshot bindings. It additionally
validates installed app-action schemas, node value/role/state/action schemas,
registered options/settings/preferences, and provider availability. A missing
resolver or unknown dynamic schema cannot authorize an action. No UID, approval,
risk or scope assertion is accepted from model arguments.

The manifest assigns conservative fixed risk, scopes, preconditions, timeout,
verifier, idempotency and recovery. Contract implementation hashes bind the exact
argument/result schema bytes followed by `src/contracts.rs`,
`src/validation.rs`, `src/registry.rs`, and `build.rs`, in that order.
The build rejects a stale hash;
they identify validation contracts, not yet-qualified provider executors.
Provider manifests must separately bind their real implementation and upstream
revision when enabled. Process termination/restart and external effects have no
invented recovery adapter. Repeated prepared operations need durable transaction
idempotency; ambiguous GUI/external effects require state observation before retry.

All approved/signed security JSON goes through `contracts::canonical_json`:
UTF-8 strings, recursively sorted object keys, array order preserved, compact
encoding, integer numbers only (signed i64/unsigned u64). Integer durations use
milliseconds/seconds as named; sizes use bytes. Floating point node values can
be observed, but cannot enter an approved plan through this encoder. Never round
a security-sensitive value silently. Basenames additionally obey UTF-8 filesystem
byte limits; file ranges are ordered, bounded rectangles. A1 formulas, named
ranges, external links, inverted ranges and over 100,000 cells are rejected.

V1 accepts schema version 1 and protocol version 1 exactly. Unknown versions fail
with `UNSUPPORTED_SCHEMA`; no automatic downgrade is permitted. Changing required
fields, defaults, meaning, bounds, result shapes or signed encoding requires a new
action API/schema version and compatibility fixtures before release. New action
IDs may be discovered as capabilities; old clients need not accept them. Risk or
implementation changes require a reviewed manifest revision and invalidate plans
bound to the old policy/implementation. Migrations use explicit trusted code,
never permissive unknown-field parsing. Tool-call JSON follows PRD §27.1 inside
the versioned authenticated request envelope; model text does not issue request IDs.

`compatibility-v1.json` contains synthetic contract fixtures, not observations
of an actual OS. `cargo test -p aios-protocol --test conformance` checks schemas,
generated types/defaults, resource denial, malformed/oversized requests and
result completeness. The registered guest integration provider additionally
runs real authenticated socket FAIL-03 checks and confirms rejected calls leave
the observed service PID, restart count, boot and state unchanged:

```text
python3 tools/devctl.py test --suite unit --detach --json
python3 tools/devctl.py test --suite integration --provider protocol-conformance --detach --json
```
