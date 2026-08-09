PRAGMA foreign_keys = ON;

CREATE TABLE cards (
    id TEXT PRIMARY KEY NOT NULL,
    card_type TEXT NOT NULL CHECK (card_type IN ('note', 'task', 'reading', 'agent_message')),
    title TEXT,
    lifecycle TEXT NOT NULL DEFAULT 'active'
        CHECK (lifecycle IN ('active', 'completed', 'archived', 'deleted')),
    attention TEXT CHECK (attention IS NULL OR attention IN ('unread', 'read')),
    source_kind TEXT NOT NULL CHECK (source_kind IN ('user', 'agent')),
    source_agent_id TEXT,
    source_delivery_id TEXT,
    metadata_json TEXT NOT NULL DEFAULT '{}'
        CHECK (json_valid(metadata_json)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    completed_at TEXT,
    archived_at TEXT,
    deleted_at TEXT,
    CHECK (
        (source_kind = 'user' AND source_agent_id IS NULL AND source_delivery_id IS NULL)
        OR
        (source_kind = 'agent' AND source_agent_id IS NOT NULL AND source_delivery_id IS NOT NULL)
    )
);

CREATE INDEX idx_cards_type_lifecycle_updated
    ON cards (card_type, lifecycle, updated_at DESC);

CREATE INDEX idx_cards_attention_updated
    ON cards (attention, updated_at DESC)
    WHERE attention IS NOT NULL;
