CREATE TABLE codex_todo_suggestion_grant (
    id TEXT PRIMARY KEY NOT NULL CHECK (id = 'codex-todo-suggestions'),
    token_hash BLOB CHECK (token_hash IS NULL OR length(token_hash) = 32),
    state TEXT NOT NULL CHECK (state IN ('active', 'revoked')),
    scope TEXT NOT NULL CHECK (scope = 'suggest_todos'),
    dedupe_namespace TEXT NOT NULL UNIQUE CHECK (length(dedupe_namespace) = 32),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK ((state = 'active' AND token_hash IS NOT NULL)
        OR (state = 'revoked' AND token_hash IS NULL))
);

CREATE TABLE todo_suggestion_batches (
    id TEXT PRIMARY KEY NOT NULL,
    dedupe_namespace TEXT NOT NULL REFERENCES codex_todo_suggestion_grant(dedupe_namespace),
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

CREATE INDEX idx_todo_suggestion_batches_status
    ON todo_suggestion_batches (status, created_at, id);
