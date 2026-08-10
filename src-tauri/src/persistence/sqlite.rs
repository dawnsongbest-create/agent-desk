use std::path::Path;

use sqlx::{migrate::MigrateError, sqlite::SqlitePoolOptions, SqlitePool};
use thiserror::Error;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

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

    MIGRATOR.run(&pool).await?;
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&pool)
        .await?;

    Ok(Database(pool))
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
              AND name IN ('cards', 'note_payloads', 'task_payloads', 'card_placements')
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

        assert_eq!(table_count, 4);
        assert_eq!(applied_versions, vec![1, 2]);
        assert_eq!(foreign_keys, 1);
    }
}
