PRAGMA foreign_keys = ON;

CREATE TABLE agent_connections (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL
        CHECK (length(trim(name)) BETWEEN 1 AND 200),
    token_hash TEXT
        CHECK (token_hash IS NULL OR length(token_hash) = 64),
    status TEXT NOT NULL DEFAULT 'inactive'
        CHECK (status IN ('active', 'inactive')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    last_used_at TEXT
);

CREATE UNIQUE INDEX idx_agent_connections_token_hash
    ON agent_connections (token_hash)
    WHERE token_hash IS NOT NULL;

CREATE UNIQUE INDEX idx_agent_connections_single_active
    ON agent_connections (status)
    WHERE status = 'active';
