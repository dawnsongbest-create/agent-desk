PRAGMA foreign_keys = ON;

CREATE TABLE note_payloads (
    card_id TEXT PRIMARY KEY NOT NULL
        REFERENCES cards(id) ON DELETE CASCADE,
    body TEXT NOT NULL
        CHECK (length(trim(body)) BETWEEN 1 AND 4000)
);

CREATE TABLE task_payloads (
    card_id TEXT PRIMARY KEY NOT NULL
        REFERENCES cards(id) ON DELETE CASCADE,
    text TEXT NOT NULL
        CHECK (length(trim(text)) BETWEEN 1 AND 4000),
    due_date TEXT
        CHECK (
            due_date IS NULL
            OR (
                length(due_date) = 10
                AND due_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'
            )
        )
);

CREATE TABLE card_placements (
    card_id TEXT PRIMARY KEY NOT NULL
        REFERENCES cards(id) ON DELETE CASCADE,
    surface TEXT NOT NULL
        CHECK (surface = 'sticky'),
    position INTEGER NOT NULL
        CHECK (position >= 0),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (surface, position)
);

CREATE TRIGGER note_payloads_require_note
BEFORE INSERT ON note_payloads
FOR EACH ROW
WHEN COALESCE((SELECT card_type FROM cards WHERE id = NEW.card_id), '') <> 'note'
BEGIN
    SELECT RAISE(ABORT, 'note payload requires a note card');
END;
CREATE TRIGGER task_payloads_require_task
BEFORE INSERT ON task_payloads
FOR EACH ROW
WHEN COALESCE((SELECT card_type FROM cards WHERE id = NEW.card_id), '') <> 'task'
BEGIN
    SELECT RAISE(ABORT, 'task payload requires a task card');
END;

CREATE TRIGGER sticky_placements_require_sticky_card
BEFORE INSERT ON card_placements
FOR EACH ROW
WHEN COALESCE((SELECT card_type FROM cards WHERE id = NEW.card_id), '') NOT IN ('note', 'task')
BEGIN
    SELECT RAISE(ABORT, 'sticky placement requires a note or task card');
END;
