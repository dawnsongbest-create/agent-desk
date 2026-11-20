CREATE TABLE local_mcp_connections (
    provider TEXT PRIMARY KEY NOT NULL CHECK (provider = 'dumate'),
    reading_namespace TEXT NOT NULL UNIQUE CHECK (length(reading_namespace) = 32),
    reading_enabled INTEGER NOT NULL CHECK (reading_enabled IN (0, 1)),
    todo_enabled INTEGER NOT NULL CHECK (todo_enabled IN (0, 1)),
    token_hash BLOB CHECK (token_hash IS NULL OR length(token_hash) = 32),
    issued_at INTEGER,
    expires_at INTEGER,
    generation INTEGER NOT NULL CHECK (generation >= 0),
    CHECK (
        (reading_enabled = 0 AND todo_enabled = 0 AND token_hash IS NULL
            AND issued_at IS NULL AND expires_at IS NULL)
        OR ((reading_enabled = 1 OR todo_enabled = 1) AND token_hash IS NOT NULL
            AND issued_at IS NOT NULL AND expires_at IS NOT NULL AND expires_at = issued_at + 1800)
    )
);

CREATE TABLE todo_suggestion_sources (
    dedupe_namespace TEXT PRIMARY KEY NOT NULL CHECK (length(dedupe_namespace) = 32),
    provider TEXT NOT NULL UNIQUE CHECK (provider IN ('codex', 'dumate'))
);

INSERT INTO todo_suggestion_sources (dedupe_namespace, provider)
SELECT dedupe_namespace, 'codex' FROM codex_todo_suggestion_grant;

CREATE TABLE todo_suggestion_batches_v11 (
    id TEXT PRIMARY KEY NOT NULL,
    dedupe_namespace TEXT NOT NULL REFERENCES todo_suggestion_sources(dedupe_namespace),
    request_digest TEXT NOT NULL CHECK (length(request_digest) = 64),
    items_json TEXT NOT NULL CHECK (json_valid(items_json)
        AND json_type(items_json) = 'array' AND json_array_length(items_json) BETWEEN 1 AND 20),
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'accepted', 'rejected')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    accepted_todo_ids_json TEXT,
    CHECK ((status = 'accepted' AND accepted_todo_ids_json IS NOT NULL
            AND json_valid(accepted_todo_ids_json) AND json_type(accepted_todo_ids_json) = 'array'
            AND json_array_length(accepted_todo_ids_json) = json_array_length(items_json))
        OR (status IN ('pending', 'rejected') AND accepted_todo_ids_json IS NULL)),
    UNIQUE (dedupe_namespace, request_digest)
);

INSERT INTO todo_suggestion_batches_v11 (
    id, dedupe_namespace, request_digest, items_json, status, created_at, updated_at, accepted_todo_ids_json
)
SELECT id, dedupe_namespace, request_digest, items_json, status, created_at, updated_at, accepted_todo_ids_json
FROM todo_suggestion_batches;

DROP TABLE todo_suggestion_batches;
ALTER TABLE todo_suggestion_batches_v11 RENAME TO todo_suggestion_batches;
CREATE INDEX idx_todo_suggestion_batches_status
    ON todo_suggestion_batches (status, created_at, id);

-- Register the initial Codex namespace atomically with the existing grant
-- insert. Its upsert only rotates token state and keeps the original namespace.
CREATE TRIGGER register_codex_todo_source
AFTER INSERT ON codex_todo_suggestion_grant
BEGIN
    INSERT INTO todo_suggestion_sources (dedupe_namespace, provider)
    VALUES (NEW.dedupe_namespace, 'codex');
END;

CREATE TRIGGER retain_codex_todo_namespace
BEFORE UPDATE OF dedupe_namespace ON codex_todo_suggestion_grant
WHEN OLD.dedupe_namespace <> NEW.dedupe_namespace
BEGIN
    SELECT RAISE(ABORT, 'cannot change Codex Todo namespace');
END;
