PRAGMA foreign_keys = ON;

CREATE TABLE reading_plans (
    id TEXT PRIMARY KEY NOT NULL,
    title TEXT NOT NULL
        CHECK (length(trim(title)) BETWEEN 1 AND 500),
    source_name TEXT,
    content_markdown TEXT NOT NULL
        CHECK (length(trim(content_markdown)) BETWEEN 1 AND 1000000),
    total_content_length INTEGER NOT NULL
        CHECK (total_content_length > 0),
    daily_minutes INTEGER NOT NULL
        CHECK (daily_minutes BETWEEN 1 AND 240),
    schedule_time TEXT NOT NULL
        CHECK (
            length(schedule_time) = 5
            AND schedule_time GLOB '[0-2][0-9]:[0-5][0-9]'
        ),
    difficulty TEXT NOT NULL
        CHECK (difficulty IN ('normal', 'technical')),
    status TEXT NOT NULL DEFAULT 'active'
        CHECK (status IN ('active', 'paused', 'completed')),
    current_offset INTEGER NOT NULL DEFAULT 0
        CHECK (current_offset >= 0 AND current_offset <= total_content_length),
    current_day INTEGER NOT NULL DEFAULT 0
        CHECK (current_day >= 0),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX idx_reading_plans_status_updated
    ON reading_plans (status, updated_at DESC);

CREATE TABLE reading_plan_deliveries (
    plan_id TEXT NOT NULL
        REFERENCES reading_plans(id) ON DELETE CASCADE,
    day INTEGER NOT NULL
        CHECK (day > 0),
    delivery_id TEXT NOT NULL UNIQUE
        REFERENCES deliveries(id) ON DELETE RESTRICT,
    document_id TEXT NOT NULL UNIQUE
        REFERENCES reader_documents(id) ON DELETE RESTRICT,
    content_start INTEGER NOT NULL
        CHECK (content_start >= 0),
    content_end INTEGER NOT NULL
        CHECK (content_end > content_start),
    estimated_minutes INTEGER NOT NULL
        CHECK (estimated_minutes > 0),
    created_at TEXT NOT NULL,
    PRIMARY KEY (plan_id, day)
);

CREATE INDEX idx_reading_plan_deliveries_document
    ON reading_plan_deliveries (document_id);

CREATE TABLE reading_sessions (
    id TEXT PRIMARY KEY NOT NULL,
    source_document_id TEXT NOT NULL
        REFERENCES reader_documents(id) ON DELETE RESTRICT,
    reader_document_id TEXT NOT NULL UNIQUE
        REFERENCES reader_documents(id) ON DELETE RESTRICT,
    content TEXT NOT NULL
        CHECK (length(trim(content)) BETWEEN 1 AND 100000),
    estimated_minutes INTEGER NOT NULL
        CHECK (estimated_minutes > 0),
    created_at TEXT NOT NULL
);

CREATE INDEX idx_reading_sessions_source
    ON reading_sessions (source_document_id, created_at DESC);
