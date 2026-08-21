use std::path::Path;

use sqlx::{migrate::MigrateError, sqlite::SqlitePoolOptions, SqlitePool};
use thiserror::Error;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");
const STICKY_MIGRATION_VERSION: i64 = 2;
const LEGACY_STICKY_CHECKSUM: &str =
    "C961BDB6248072EB5DD92E0C28ABE80CE220D2350B05531DB961FB72B10384FC3099213B127B901FA666E42BC9FD9297";

const EXPECTED_STICKY_SCHEMA: [(&str, &str); 6] = [
    (
        "card_placements",
        "CREATE TABLE card_placements (\n    card_id TEXT PRIMARY KEY NOT NULL\n        REFERENCES cards(id) ON DELETE CASCADE,\n    surface TEXT NOT NULL\n        CHECK (surface = 'sticky'),\n    position INTEGER NOT NULL\n        CHECK (position >= 0),\n    created_at TEXT NOT NULL,\n    updated_at TEXT NOT NULL,\n    UNIQUE (surface, position)\n)",
    ),
    (
        "note_payloads",
        "CREATE TABLE note_payloads (\n    card_id TEXT PRIMARY KEY NOT NULL\n        REFERENCES cards(id) ON DELETE CASCADE,\n    body TEXT NOT NULL\n        CHECK (length(trim(body)) BETWEEN 1 AND 4000)\n)",
    ),
    (
        "note_payloads_require_note",
        "CREATE TRIGGER note_payloads_require_note\nBEFORE INSERT ON note_payloads\nFOR EACH ROW\nWHEN COALESCE((SELECT card_type FROM cards WHERE id = NEW.card_id), '') <> 'note'\nBEGIN\n    SELECT RAISE(ABORT, 'note payload requires a note card');\nEND",
    ),
    (
        "sticky_placements_require_sticky_card",
        "CREATE TRIGGER sticky_placements_require_sticky_card\nBEFORE INSERT ON card_placements\nFOR EACH ROW\nWHEN COALESCE((SELECT card_type FROM cards WHERE id = NEW.card_id), '') NOT IN ('note', 'task')\nBEGIN\n    SELECT RAISE(ABORT, 'sticky placement requires a note or task card');\nEND",
    ),
    (
        "task_payloads",
        "CREATE TABLE task_payloads (\n    card_id TEXT PRIMARY KEY NOT NULL\n        REFERENCES cards(id) ON DELETE CASCADE,\n    text TEXT NOT NULL\n        CHECK (length(trim(text)) BETWEEN 1 AND 4000),\n    due_date TEXT\n        CHECK (\n            due_date IS NULL\n            OR (\n                length(due_date) = 10\n                AND due_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'\n            )\n        )\n)",
    ),
    (
        "task_payloads_require_task",
        "CREATE TRIGGER task_payloads_require_task\nBEFORE INSERT ON task_payloads\nFOR EACH ROW\nWHEN COALESCE((SELECT card_type FROM cards WHERE id = NEW.card_id), '') <> 'task'\nBEGIN\n    SELECT RAISE(ABORT, 'task payload requires a task card');\nEND",
    ),
];

fn legacy_sticky_checksum() -> Vec<u8> {
    LEGACY_STICKY_CHECKSUM
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let hex_digit = |digit: u8| match digit {
                b'0'..=b'9' => digit - b'0',
                b'A'..=b'F' => digit - b'A' + 10,
                _ => unreachable!("checksum constant contains only uppercase hex digits"),
            };
            (hex_digit(pair[0]) << 4) | hex_digit(pair[1])
        })
        .collect()
}

#[derive(Debug, Error)]
pub enum PersistenceError {
    #[error("failed to prepare the application data directory")]
    Io(#[from] std::io::Error),
    #[error("database operation failed")]
    Sqlx(#[from] sqlx::Error),
    #[error("database migration failed")]
    Migration(#[from] MigrateError),
}

#[derive(Clone)]
pub struct Database(pub SqlitePool);

pub async fn connect(database_path: &Path) -> Result<Database, PersistenceError> {
    if let Some(parent) = database_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let options = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(database_path)
        .create_if_missing(true)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(options)
        .await?;

    reconcile_legacy_sticky_checksum(&pool).await?;
    MIGRATOR.run(&pool).await?;
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&pool)
        .await?;

    Ok(Database(pool))
}

async fn reconcile_legacy_sticky_checksum(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let migration_table_exists: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = '_sqlx_migrations'",
    )
    .fetch_one(pool)
    .await?;
    if migration_table_exists == 0 {
        return Ok(());
    }

    let migration: Option<(bool, Vec<u8>)> =
        sqlx::query_as("SELECT success, checksum FROM _sqlx_migrations WHERE version = ?")
            .bind(STICKY_MIGRATION_VERSION)
            .fetch_optional(pool)
            .await?;
    let Some((true, checksum)) = migration else {
        return Ok(());
    };
    if checksum != legacy_sticky_checksum() {
        return Ok(());
    }

    let schema_rows: Vec<(String, String)> = sqlx::query_as(
        r#"
        SELECT name, sql
        FROM sqlite_master
        WHERE name IN (
            'note_payloads',
            'task_payloads',
            'card_placements',
            'note_payloads_require_note',
            'task_payloads_require_task',
            'sticky_placements_require_sticky_card'
        )
        ORDER BY name
        "#,
    )
    .fetch_all(pool)
    .await?;
    let expected = EXPECTED_STICKY_SCHEMA
        .iter()
        .map(|(name, sql)| ((*name).to_owned(), (*sql).to_owned()))
        .collect::<Vec<_>>();
    if schema_rows != expected {
        return Ok(());
    }

    let current_checksum = MIGRATOR
        .iter()
        .find(|migration| migration.version == STICKY_MIGRATION_VERSION)
        .expect("sticky migration is embedded")
        .checksum
        .as_ref();
    sqlx::query("UPDATE _sqlx_migrations SET checksum = ? WHERE version = ? AND checksum = ?")
        .bind(current_checksum)
        .bind(STICKY_MIGRATION_VERSION)
        .bind(checksum)
        .execute(pool)
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn applies_foundation_and_sticky_migrations_and_enables_foreign_keys() {
        let temp = tempfile::tempdir().expect("temp directory");
        let database = connect(&temp.path().join("agent-desk.sqlite3"))
            .await
            .expect("database should initialize");

        let table_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM sqlite_master
            WHERE type = 'table'
              AND name IN (
                  'cards', 'note_payloads', 'task_payloads', 'card_placements',
                  'sticky_surface_profile', 'reader_documents', 'record_source_refs', 'deliveries',
                  'reading_plans', 'reading_plan_deliveries', 'reading_sessions', 'agent_proposals'
              )
            "#,
        )
        .fetch_one(&database.0)
        .await
        .expect("cards table query");
        let foreign_keys: i64 = sqlx::query_scalar("PRAGMA foreign_keys")
            .fetch_one(&database.0)
            .await
            .expect("foreign key pragma");

        let applied_versions: Vec<i64> =
            sqlx::query_scalar("SELECT version FROM _sqlx_migrations ORDER BY version")
                .fetch_all(&database.0)
                .await
                .expect("migration history query");

        assert_eq!(table_count, 12);
        assert_eq!(applied_versions, vec![1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(foreign_keys, 1);
    }

    #[tokio::test]
    async fn upgrades_a_0003_database_without_changing_existing_sticky_data() {
        let temp = tempfile::tempdir().expect("temp directory");
        let database_path = temp.path().join("upgrade-0003.sqlite3");
        let database = connect(&database_path)
            .await
            .expect("database should initialize");
        sqlx::raw_sql(
            r#"
            DROP TRIGGER record_source_refs_require_note;
            DROP TABLE record_source_refs;
            DROP TABLE reader_documents;
            DELETE FROM _sqlx_migrations WHERE version = 4;

            INSERT INTO cards (
                id, card_type, title, lifecycle, attention, source_kind,
                source_agent_id, source_delivery_id, metadata_json,
                created_at, updated_at
            ) VALUES
                ('existing-note', 'note', NULL, 'active', NULL, 'user', NULL, NULL, '{}',
                 '2026-08-01T00:00:00Z', '2026-08-01T00:00:00Z'),
                ('existing-task', 'task', NULL, 'active', NULL, 'user', NULL, NULL, '{}',
                 '2026-08-01T00:00:00Z', '2026-08-01T00:00:00Z');
            INSERT INTO note_payloads (card_id, body) VALUES ('existing-note', '保留的记录');
            INSERT INTO task_payloads (card_id, text, due_date)
                VALUES ('existing-task', '保留的待办', '2026-08-30');
            INSERT INTO card_placements (card_id, surface, position, created_at, updated_at)
                VALUES
                    ('existing-note', 'sticky', 0, '2026-08-01T00:00:00Z', '2026-08-01T00:00:00Z'),
                    ('existing-task', 'sticky', 1, '2026-08-01T00:00:00Z', '2026-08-01T00:00:00Z');
            UPDATE sticky_surface_profile
                SET quote_text = '保留的便签一句'
                WHERE surface = 'sticky';
            "#,
        )
        .execute(&database.0)
        .await
        .expect("0003 fixture setup");
        database.0.close().await;

        let upgraded = connect(&database_path)
            .await
            .expect("0003 database should upgrade");
        let versions: Vec<i64> =
            sqlx::query_scalar("SELECT version FROM _sqlx_migrations ORDER BY version")
                .fetch_all(&upgraded.0)
                .await
                .unwrap();
        let note: String =
            sqlx::query_scalar("SELECT body FROM note_payloads WHERE card_id = 'existing-note'")
                .fetch_one(&upgraded.0)
                .await
                .unwrap();
        let task: (String, Option<String>) = sqlx::query_as(
            "SELECT text, due_date FROM task_payloads WHERE card_id = 'existing-task'",
        )
        .fetch_one(&upgraded.0)
        .await
        .unwrap();
        let quote: String = sqlx::query_scalar(
            "SELECT quote_text FROM sticky_surface_profile WHERE surface = 'sticky'",
        )
        .fetch_one(&upgraded.0)
        .await
        .unwrap();
        let placements: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM card_placements")
            .fetch_one(&upgraded.0)
            .await
            .unwrap();

        assert_eq!(versions, vec![1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(note, "保留的记录");
        assert_eq!(
            task,
            ("保留的待办".to_owned(), Some("2026-08-30".to_owned()))
        );
        assert_eq!(quote, "保留的便签一句");
        assert_eq!(placements, 2);
    }

    #[tokio::test]
    async fn upgrades_a_0004_database_without_changing_reader_or_sticky_data() {
        let temp = tempfile::tempdir().expect("temp directory");
        let database_path = temp.path().join("upgrade-0004.sqlite3");
        let database = connect(&database_path).await.unwrap();
        sqlx::raw_sql(
            r#"
            DROP TABLE deliveries;
            DELETE FROM _sqlx_migrations WHERE version = 5;

            INSERT INTO cards (
                id, card_type, title, lifecycle, attention, source_kind,
                source_agent_id, source_delivery_id, metadata_json,
                created_at, updated_at
            ) VALUES (
                'existing-note-0004', 'note', NULL, 'active', NULL, 'user',
                NULL, NULL, '{}', '2026-08-01T00:00:00Z', '2026-08-01T00:00:00Z'
            );
            INSERT INTO note_payloads (card_id, body)
                VALUES ('existing-note-0004', '0004 保留记录');
            INSERT INTO card_placements (card_id, surface, position, created_at, updated_at)
                VALUES ('existing-note-0004', 'sticky', 0,
                        '2026-08-01T00:00:00Z', '2026-08-01T00:00:00Z');
            INSERT INTO reader_documents (
                id, document_type, title, subtitle, content_markdown,
                source_type, source_label, created_at, updated_at
            ) VALUES (
                'existing-reader-0004', 'article', '0004 保留文章', NULL,
                '正文保持不变', 'local', '迁移测试',
                '2026-08-01T00:00:00Z', '2026-08-01T00:00:00Z'
            );
            "#,
        )
        .execute(&database.0)
        .await
        .unwrap();
        database.0.close().await;

        let upgraded = connect(&database_path).await.unwrap();
        let versions: Vec<i64> =
            sqlx::query_scalar("SELECT version FROM _sqlx_migrations ORDER BY version")
                .fetch_all(&upgraded.0)
                .await
                .unwrap();
        let note: String = sqlx::query_scalar(
            "SELECT body FROM note_payloads WHERE card_id = 'existing-note-0004'",
        )
        .fetch_one(&upgraded.0)
        .await
        .unwrap();
        let reader: (String, String) = sqlx::query_as(
            "SELECT title, content_markdown FROM reader_documents WHERE id = 'existing-reader-0004'",
        )
        .fetch_one(&upgraded.0)
        .await
        .unwrap();
        let delivery_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM deliveries")
            .fetch_one(&upgraded.0)
            .await
            .unwrap();
        assert_eq!(versions, vec![1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(note, "0004 保留记录");
        assert_eq!(
            reader,
            ("0004 保留文章".to_owned(), "正文保持不变".to_owned())
        );
        assert_eq!(delivery_count, 0);
    }

    #[tokio::test]
    async fn upgrades_a_0005_database_without_changing_delivery_or_reader_data() {
        let temp = tempfile::tempdir().expect("temp directory");
        let database_path = temp.path().join("upgrade-0005.sqlite3");
        let database = connect(&database_path).await.unwrap();
        sqlx::raw_sql(
            r#"
            DROP TABLE reading_sessions;
            DROP TABLE reading_plan_deliveries;
            DROP TABLE reading_plans;
            DELETE FROM _sqlx_migrations WHERE version = 6;

            INSERT INTO reader_documents (
                id, document_type, title, subtitle, content_markdown,
                source_type, source_label, created_at, updated_at
            ) VALUES (
                'existing-reader-0005', 'brief', '0005 保留交付', NULL,
                '交付正文保持不变', 'agent', '迁移测试',
                '2026-08-01T00:00:00Z', '2026-08-01T00:00:00Z'
            );
            INSERT INTO deliveries (
                id, document_id, idempotency_key, delivered_at, opened_at
            ) VALUES (
                'existing-delivery-0005', 'existing-reader-0005',
                'existing-key-0005', '2026-08-01T00:00:00Z', NULL
            );
            "#,
        )
        .execute(&database.0)
        .await
        .unwrap();
        database.0.close().await;

        let upgraded = connect(&database_path).await.unwrap();
        let versions: Vec<i64> =
            sqlx::query_scalar("SELECT version FROM _sqlx_migrations ORDER BY version")
                .fetch_all(&upgraded.0)
                .await
                .unwrap();
        let delivery: (String, String) = sqlx::query_as(
            r#"
            SELECT d.id, r.content_markdown
            FROM deliveries d
            JOIN reader_documents r ON r.id = d.document_id
            WHERE d.id = 'existing-delivery-0005'
            "#,
        )
        .fetch_one(&upgraded.0)
        .await
        .unwrap();
        let reading_tables: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM sqlite_master
            WHERE type = 'table'
              AND name IN ('reading_plans', 'reading_plan_deliveries', 'reading_sessions')
            "#,
        )
        .fetch_one(&upgraded.0)
        .await
        .unwrap();
        assert_eq!(versions, vec![1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(
            delivery,
            (
                "existing-delivery-0005".to_owned(),
                "交付正文保持不变".to_owned()
            )
        );
        assert_eq!(reading_tables, 3);
    }

    #[tokio::test]
    async fn upgrades_a_0006_database_without_changing_reading_or_delivery_data() {
        let temp = tempfile::tempdir().expect("temp directory");
        let database_path = temp.path().join("upgrade-0006.sqlite3");
        let database = connect(&database_path).await.unwrap();
        sqlx::raw_sql(
            r#"
            DROP TABLE agent_proposals;
            DELETE FROM _sqlx_migrations WHERE version = 7;

            INSERT INTO reader_documents (
                id, document_type, title, subtitle, content_markdown,
                source_type, source_label, created_at, updated_at
            ) VALUES
                ('source-0006', 'brief', '0006 来源', NULL, '来源正文', 'agent', '迁移测试',
                 '2026-08-01T00:00:00Z', '2026-08-01T00:00:00Z'),
                ('session-document-0006', 'reading', '0006 今日阅读', NULL, '会话正文', 'local',
                 '迁移测试', '2026-08-01T00:00:00Z', '2026-08-01T00:00:00Z');
            INSERT INTO deliveries (
                id, document_id, idempotency_key, delivered_at, opened_at
            ) VALUES (
                'delivery-0006', 'source-0006', 'existing-key-0006',
                '2026-08-01T00:00:00Z', '2026-08-01T01:00:00Z'
            );
            INSERT INTO reading_sessions (
                id, source_document_id, reader_document_id, content, estimated_minutes, created_at
            ) VALUES (
                'session-0006', 'source-0006', 'session-document-0006',
                '会话正文', 2, '2026-08-01T01:00:00Z'
            );
            "#,
        )
        .execute(&database.0)
        .await
        .unwrap();
        database.0.close().await;

        let upgraded = connect(&database_path).await.unwrap();
        let versions: Vec<i64> =
            sqlx::query_scalar("SELECT version FROM _sqlx_migrations ORDER BY version")
                .fetch_all(&upgraded.0)
                .await
                .unwrap();
        let preserved: (String, String, i64) = sqlx::query_as(
            r#"
            SELECT d.id, s.content, s.estimated_minutes
            FROM deliveries d
            JOIN reading_sessions s ON s.source_document_id = d.document_id
            WHERE d.id = 'delivery-0006'
            "#,
        )
        .fetch_one(&upgraded.0)
        .await
        .unwrap();
        let proposal_table: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'agent_proposals'",
        )
        .fetch_one(&upgraded.0)
        .await
        .unwrap();
        assert_eq!(versions, vec![1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(
            preserved,
            ("delivery-0006".to_owned(), "会话正文".to_owned(), 2)
        );
        assert_eq!(proposal_table, 1);
    }

    #[tokio::test]
    async fn reconciles_the_schema_equivalent_m1_b1_preview_checksum() {
        let temp = tempfile::tempdir().expect("temp directory");
        let database_path = temp.path().join("agent-desk.sqlite3");
        let database = connect(&database_path)
            .await
            .expect("database should initialize");
        sqlx::raw_sql(
            r#"
            DROP TABLE sticky_surface_profile;
            DROP TRIGGER note_payloads_require_note;
            DROP TABLE note_payloads;
            DELETE FROM _sqlx_migrations WHERE version = 3;
            "#,
        )
        .execute(&database.0)
        .await
        .expect("pre-0003 legacy fixture");
        for name in ["note_payloads", "note_payloads_require_note"] {
            let sql = EXPECTED_STICKY_SCHEMA
                .iter()
                .find(|(candidate, _)| *candidate == name)
                .expect("frozen 0002 schema definition")
                .1;
            sqlx::raw_sql(sql)
                .execute(&database.0)
                .await
                .expect("exact frozen 0002 schema fixture");
        }
        sqlx::query("UPDATE _sqlx_migrations SET checksum = ? WHERE version = 2")
            .bind(legacy_sticky_checksum())
            .execute(&database.0)
            .await
            .expect("legacy checksum fixture");
        database.0.close().await;

        let reopened = connect(&database_path)
            .await
            .expect("schema-equivalent preview database should reopen");
        let stored: Vec<u8> =
            sqlx::query_scalar("SELECT checksum FROM _sqlx_migrations WHERE version = 2")
                .fetch_one(&reopened.0)
                .await
                .expect("stored checksum");
        let current = MIGRATOR
            .iter()
            .find(|migration| migration.version == 2)
            .expect("sticky migration")
            .checksum
            .as_ref();
        assert_eq!(stored, current);
    }

    #[tokio::test]
    async fn refuses_to_reconcile_a_legacy_checksum_when_the_schema_differs() {
        let temp = tempfile::tempdir().expect("temp directory");
        let database_path = temp.path().join("agent-desk.sqlite3");
        let database = connect(&database_path)
            .await
            .expect("database should initialize");
        sqlx::query("DROP TRIGGER task_payloads_require_task")
            .execute(&database.0)
            .await
            .expect("schema drift fixture");
        sqlx::query("UPDATE _sqlx_migrations SET checksum = ? WHERE version = 2")
            .bind(legacy_sticky_checksum())
            .execute(&database.0)
            .await
            .expect("legacy checksum fixture");
        database.0.close().await;

        let error = match connect(&database_path).await {
            Ok(_) => panic!("schema drift must keep failing migration validation"),
            Err(error) => error,
        };
        assert!(matches!(error, PersistenceError::Migration(_)));
    }
}
