PRAGMA foreign_keys = ON;

CREATE TABLE deliveries (
    id TEXT PRIMARY KEY NOT NULL,
    document_id TEXT NOT NULL
        REFERENCES reader_documents(id) ON DELETE RESTRICT,
    idempotency_key TEXT NOT NULL UNIQUE
        CHECK (length(trim(idempotency_key)) BETWEEN 1 AND 500),
    delivered_at TEXT NOT NULL
        CHECK (length(trim(delivered_at)) BETWEEN 1 AND 100),
    opened_at TEXT
        CHECK (opened_at IS NULL OR length(trim(opened_at)) BETWEEN 1 AND 100)
);

CREATE INDEX idx_deliveries_inbox_order
    ON deliveries (delivered_at DESC, id DESC);

CREATE INDEX idx_deliveries_unread
    ON deliveries (delivered_at DESC, id DESC)
    WHERE opened_at IS NULL;
