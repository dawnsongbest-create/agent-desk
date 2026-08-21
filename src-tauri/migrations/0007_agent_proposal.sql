PRAGMA foreign_keys = ON;

CREATE TABLE agent_proposals (
    id TEXT PRIMARY KEY NOT NULL,
    proposal_type TEXT NOT NULL
        CHECK (proposal_type IN ('todo', 'record', 'reading')),
    title TEXT NOT NULL
        CHECK (length(trim(title)) BETWEEN 1 AND 500),
    description TEXT NOT NULL
        CHECK (length(trim(description)) BETWEEN 1 AND 4000),
    payload_json TEXT NOT NULL
        CHECK (json_valid(payload_json)),
    source_delivery_id TEXT
        REFERENCES deliveries(id) ON DELETE RESTRICT,
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'accepted', 'rejected')),
    created_at TEXT NOT NULL,
    resolved_at TEXT
);

CREATE INDEX idx_agent_proposals_delivery_status
    ON agent_proposals (source_delivery_id, status, created_at DESC);
