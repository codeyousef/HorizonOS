CREATE TABLE graph_metadata (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    scope_uid INTEGER NOT NULL CHECK (scope_uid BETWEEN -1 AND 4294967295)
) STRICT;
CREATE TABLE nodes (
    id TEXT PRIMARY KEY NOT NULL,
    kind TEXT NOT NULL,
    scope_uid INTEGER NOT NULL CHECK (scope_uid BETWEEN -1 AND 4294967295),
    provider TEXT NOT NULL,
    stable_key TEXT NOT NULL,
    properties_json TEXT NOT NULL CHECK (json_valid(properties_json)),
    source_truth TEXT NOT NULL CHECK (source_truth IN ('intended','built','running','boot_selected','user_app','unmanaged')),
    first_seen INTEGER NOT NULL CHECK (first_seen >= 0),
    last_seen INTEGER NOT NULL CHECK (last_seen >= first_seen),
    revision INTEGER NOT NULL CHECK (revision > 0),
    deleted_at INTEGER,
    UNIQUE (scope_uid, provider, kind, stable_key, source_truth)
) STRICT;
CREATE TABLE observations (
    id TEXT PRIMARY KEY NOT NULL,
    provider TEXT NOT NULL,
    entity_id TEXT NOT NULL REFERENCES nodes(id),
    boot_id TEXT NOT NULL,
    realtime_ns INTEGER NOT NULL CHECK (realtime_ns >= 0),
    monotonic_ns INTEGER NOT NULL CHECK (monotonic_ns >= 0),
    payload_json TEXT NOT NULL CHECK (json_valid(payload_json)),
    sensitivity TEXT NOT NULL CHECK (sensitivity IN ('public','private','restricted')),
    content_hash TEXT NOT NULL CHECK (length(content_hash) = 64)
) STRICT;
CREATE TABLE evidence (
    id TEXT PRIMARY KEY NOT NULL,
    source_kind TEXT NOT NULL,
    source_locator_json TEXT NOT NULL CHECK (json_valid(source_locator_json)),
    observation_id TEXT NOT NULL REFERENCES observations(id),
    content_hash TEXT NOT NULL CHECK (length(content_hash) = 64),
    excerpt TEXT NOT NULL,
    captured_at INTEGER NOT NULL CHECK (captured_at >= 0),
    expires_at INTEGER CHECK (expires_at >= captured_at),
    scope_uid INTEGER NOT NULL CHECK (scope_uid BETWEEN -1 AND 4294967295)
) STRICT;
CREATE TABLE edges (
    id TEXT PRIMARY KEY NOT NULL,
    from_id TEXT NOT NULL REFERENCES nodes(id),
    relation TEXT NOT NULL,
    to_id TEXT NOT NULL REFERENCES nodes(id),
    provider TEXT NOT NULL,
    evidence_id TEXT NOT NULL REFERENCES evidence(id),
    observed_at INTEGER NOT NULL CHECK (observed_at >= 0),
    valid_until INTEGER CHECK (valid_until >= observed_at),
    certainty TEXT NOT NULL CHECK (certainty IN ('observed','hypothesis'))
) STRICT;
CREATE TABLE events (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id TEXT UNIQUE NOT NULL,
    entity_id TEXT REFERENCES nodes(id),
    event_type TEXT NOT NULL,
    boot_id TEXT NOT NULL,
    timestamp INTEGER NOT NULL CHECK (timestamp >= 0),
    origin_transaction_id TEXT,
    payload_json TEXT NOT NULL CHECK (json_valid(payload_json))
) STRICT;
CREATE TABLE provider_state (
    provider TEXT PRIMARY KEY NOT NULL,
    cursor_json TEXT NOT NULL CHECK (json_valid(cursor_json)),
    last_success INTEGER CHECK (last_success >= 0),
    status TEXT NOT NULL CHECK (status IN ('unknown','ready','partial','failed','reconcile_required')),
    error_json TEXT CHECK (error_json IS NULL OR json_valid(error_json))
) STRICT;
