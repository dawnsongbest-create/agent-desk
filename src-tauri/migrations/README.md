# Migration policy

- Migration files are append-only once merged or shipped.
- Names use `<version>_<description>.sql` and monotonically increasing integer versions.
- SQL files are normalized to LF by the root `.gitattributes` so embedded migration hashes are stable across Windows and macOS.
- Released schema fixtures must be migrated forward in CI before destructive migrations are approved.
- A migration failure must preserve the existing database and stop normal startup; the application must never silently replace it with an empty database.

`0001_card_foundation.sql` intentionally creates only the common Card table. Note/Task payload tables, delivery tables, Reader state and Agent Gateway storage are deferred to their approved gates.
