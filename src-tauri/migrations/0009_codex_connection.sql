CREATE TABLE codex_connection (
    id TEXT PRIMARY KEY NOT NULL CHECK (id = 'codex-reading'),
    token_hash BLOB CHECK (token_hash IS NULL OR length(token_hash) = 32),
    state TEXT NOT NULL CHECK (state IN ('active', 'revoked')),
    scope TEXT NOT NULL CHECK (scope = 'send_reading'),
    dedupe_namespace TEXT NOT NULL UNIQUE CHECK (length(dedupe_namespace) = 32),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    delivered_at TEXT,
    CHECK ((state = 'active' AND token_hash IS NOT NULL)
        OR (state = 'revoked' AND token_hash IS NULL))
);
