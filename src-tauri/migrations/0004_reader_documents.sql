PRAGMA foreign_keys = ON;

CREATE TABLE reader_documents (
    id TEXT PRIMARY KEY NOT NULL,
    document_type TEXT NOT NULL
        CHECK (document_type IN ('article', 'brief', 'reading', 'report')),
    title TEXT NOT NULL
        CHECK (length(trim(title)) BETWEEN 1 AND 500),
    subtitle TEXT
        CHECK (subtitle IS NULL OR length(subtitle) <= 1000),
    content_markdown TEXT NOT NULL
        CHECK (length(trim(content_markdown)) BETWEEN 1 AND 1000000),
    source_type TEXT NOT NULL
        CHECK (source_type IN ('local', 'builtin', 'agent')),
    source_label TEXT
        CHECK (source_label IS NULL OR length(source_label) <= 500),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX idx_reader_documents_updated
    ON reader_documents (updated_at DESC, created_at DESC);

CREATE TABLE record_source_refs (
    record_id TEXT PRIMARY KEY NOT NULL
        REFERENCES cards(id) ON DELETE CASCADE,
    document_id TEXT NOT NULL
        REFERENCES reader_documents(id) ON DELETE RESTRICT,
    source_type TEXT NOT NULL
        CHECK (source_type = 'reader_selection'),
    selected_text TEXT NOT NULL
        CHECK (length(trim(selected_text)) BETWEEN 1 AND 100000),
    document_title_snapshot TEXT NOT NULL
        CHECK (length(trim(document_title_snapshot)) BETWEEN 1 AND 500),
    captured_at TEXT NOT NULL
);

CREATE INDEX idx_record_source_refs_document
    ON record_source_refs (document_id, captured_at DESC);

CREATE TRIGGER record_source_refs_require_note
BEFORE INSERT ON record_source_refs
FOR EACH ROW
WHEN COALESCE((SELECT card_type FROM cards WHERE id = NEW.record_id), '') <> 'note'
BEGIN
    SELECT RAISE(ABORT, 'reader selection source requires a note card');
END;
