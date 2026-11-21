CREATE TABLE chatgpt_desktop_grants (
    scope TEXT PRIMARY KEY NOT NULL CHECK (scope IN ('send_reading', 'suggest_todos')),
    dedupe_namespace TEXT NOT NULL UNIQUE CHECK (length(dedupe_namespace) = 32),
    token_hash BLOB CHECK (token_hash IS NULL OR length(token_hash) = 32),
    updated_at TEXT NOT NULL,
    last_used_at TEXT,
    CHECK (token_hash IS NOT NULL OR last_used_at IS NULL)
);

-- SQLite cannot extend a CHECK in place. Keep the FK enforced and copy all
-- historical receipts, namespaces and statuses in the migration transaction.
CREATE TABLE todo_suggestion_sources_v12 (
    dedupe_namespace TEXT PRIMARY KEY NOT NULL CHECK (length(dedupe_namespace) = 32),
    provider TEXT NOT NULL UNIQUE CHECK (provider IN ('codex', 'dumate', 'chatgpt'))
);
INSERT INTO todo_suggestion_sources_v12 (dedupe_namespace, provider)
SELECT dedupe_namespace, provider FROM todo_suggestion_sources;

CREATE TABLE todo_suggestion_batches_v12 (
    id TEXT PRIMARY KEY NOT NULL,
    dedupe_namespace TEXT NOT NULL REFERENCES todo_suggestion_sources_v12(dedupe_namespace),
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
INSERT INTO todo_suggestion_batches_v12 (
    id, dedupe_namespace, request_digest, items_json, status, created_at, updated_at, accepted_todo_ids_json
)
SELECT id, dedupe_namespace, request_digest, items_json, status, created_at, updated_at, accepted_todo_ids_json
FROM todo_suggestion_batches;

DROP TRIGGER register_codex_todo_source;
DROP TABLE todo_suggestion_batches;
DROP TABLE todo_suggestion_sources;
ALTER TABLE todo_suggestion_sources_v12 RENAME TO todo_suggestion_sources;
ALTER TABLE todo_suggestion_batches_v12 RENAME TO todo_suggestion_batches;
CREATE INDEX idx_todo_suggestion_batches_status
    ON todo_suggestion_batches (status, created_at, id);

CREATE TRIGGER register_codex_todo_source
AFTER INSERT ON codex_todo_suggestion_grant
BEGIN
    INSERT INTO todo_suggestion_sources (dedupe_namespace, provider)
    VALUES (NEW.dedupe_namespace, 'codex');
END;

CREATE TRIGGER register_chatgpt_todo_source
AFTER INSERT ON chatgpt_desktop_grants
WHEN NEW.scope = 'suggest_todos'
BEGIN
    INSERT INTO todo_suggestion_sources (dedupe_namespace, provider)
    VALUES (NEW.dedupe_namespace, 'chatgpt');
END;
