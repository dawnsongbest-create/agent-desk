PRAGMA foreign_keys = ON;

CREATE TABLE note_payloads_v3 (
    card_id TEXT PRIMARY KEY NOT NULL
        REFERENCES cards(id) ON DELETE CASCADE,
    body TEXT NOT NULL
        CHECK (length(trim(body)) BETWEEN 1 AND 100000)
);

INSERT INTO note_payloads_v3 (card_id, body)
SELECT card_id, body FROM note_payloads;

DROP TABLE note_payloads;
ALTER TABLE note_payloads_v3 RENAME TO note_payloads;

CREATE TRIGGER note_payloads_require_note
BEFORE INSERT ON note_payloads
FOR EACH ROW
WHEN COALESCE((SELECT card_type FROM cards WHERE id = NEW.card_id), '') <> 'note'
BEGIN
    SELECT RAISE(ABORT, 'note payload requires a note card');
END;

CREATE TABLE sticky_surface_profile (
    surface TEXT PRIMARY KEY NOT NULL
        CHECK (surface = 'sticky'),
    quote_text TEXT NOT NULL DEFAULT ''
        CHECK (length(quote_text) <= 10000),
    updated_at TEXT NOT NULL
);

INSERT INTO sticky_surface_profile (surface, quote_text, updated_at)
VALUES ('sticky', '', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));
