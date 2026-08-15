# Migration policy

- Migration files are append-only once merged or shipped.
- Names use `<version>_<description>.sql` and monotonically increasing integer versions.
- SQL files are normalized to LF by the root `.gitattributes` so embedded migration hashes are stable across Windows and macOS.
- Released schema fixtures must be migrated forward in CI before destructive migrations are approved.
- A migration failure must preserve the existing database and stop normal startup; the application must never silently replace it with an empty database.

`0001_card_foundation.sql` intentionally creates only the common Card table and remains frozen.

`0002_sticky_cards.sql` adds only the approved M1-B1 Note/Task payloads and unified Sticky placement ordering. Delivery tables, Reader state and Agent Gateway storage remain deferred to their approved gates.

`0003_sticky_record_profile.sql` expands the shipped long-record capacity and adds the Sticky quote profile. It remains frozen.

`0004_reader_documents.sql` adds the minimal durable ReaderDocument table and the one-to-one Reader selection provenance for existing note cards. It does not add delivery, Inbox, progress, annotation, or cursor state.
