use std::collections::HashSet;

use async_trait::async_trait;
use sqlx::{Sqlite, SqlitePool, Transaction};

use crate::{
    application::ports::sticky_repository::{StickyRepository, StickyRepositoryError},
    domain::sticky::{NewStickyCard, StickyCard, StickyCardKind, StickyProfile},
};

const SELECT_STICKY_CARD: &str = r#"
    SELECT
        c.id,
        c.card_type AS kind,
        CASE c.card_type
            WHEN 'note' THEN note.body
            WHEN 'task' THEN task.text
        END AS text,
        CASE WHEN c.lifecycle = 'completed' THEN 1 ELSE 0 END AS completed,
        task.due_date,
        placement.position,
        c.created_at,
        c.updated_at
    FROM cards AS c
    INNER JOIN card_placements AS placement
        ON placement.card_id = c.id AND placement.surface = 'sticky'
    LEFT JOIN note_payloads AS note ON note.card_id = c.id
    LEFT JOIN task_payloads AS task ON task.card_id = c.id
    WHERE c.id = ?
      AND c.card_type IN ('note', 'task')
      AND c.lifecycle IN ('active', 'completed')
"#;

const SELECT_STICKY_CARDS: &str = r#"
    SELECT
        c.id,
        c.card_type AS kind,
        CASE c.card_type
            WHEN 'note' THEN note.body
            WHEN 'task' THEN task.text
        END AS text,
        CASE WHEN c.lifecycle = 'completed' THEN 1 ELSE 0 END AS completed,
        task.due_date,
        placement.position,
        c.created_at,
        c.updated_at
    FROM cards AS c
    INNER JOIN card_placements AS placement
        ON placement.card_id = c.id AND placement.surface = 'sticky'
    LEFT JOIN note_payloads AS note ON note.card_id = c.id
    LEFT JOIN task_payloads AS task ON task.card_id = c.id
    WHERE c.card_type IN ('note', 'task')
      AND c.lifecycle IN ('active', 'completed')
    ORDER BY placement.position ASC, c.created_at ASC
"#;

#[derive(Debug, sqlx::FromRow)]
struct StickyCardRow {
    id: String,
    kind: String,
    text: String,
    completed: i64,
    due_date: Option<String>,
    position: i64,
    created_at: String,
    updated_at: String,
}

impl TryFrom<StickyCardRow> for StickyCard {
    type Error = StickyRepositoryError;

    fn try_from(row: StickyCardRow) -> Result<Self, Self::Error> {
        let kind = match row.kind.as_str() {
            "note" => StickyCardKind::Note,
            "task" => StickyCardKind::Task,
            _ => {
                return Err(StickyRepositoryError::Storage(
                    "unexpected Sticky card type".to_string(),
                ))
            }
        };
        Ok(Self {
            id: row.id,
            kind,
            text: row.text,
            completed: row.completed != 0,
            due_date: row.due_date,
            position: row.position,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

#[derive(Clone)]
pub struct SqliteStickyRepository {
    pool: SqlitePool,
}

impl SqliteStickyRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    async fn fetch_in_transaction(
        transaction: &mut Transaction<'_, Sqlite>,
        id: &str,
    ) -> Result<StickyCard, StickyRepositoryError> {
        let row = sqlx::query_as::<_, StickyCardRow>(SELECT_STICKY_CARD)
            .bind(id)
            .fetch_optional(&mut **transaction)
            .await?
            .ok_or(StickyRepositoryError::NotFound)?;
        row.try_into()
    }

    async fn list_in_transaction(
        transaction: &mut Transaction<'_, Sqlite>,
    ) -> Result<Vec<StickyCard>, StickyRepositoryError> {
        let rows = sqlx::query_as::<_, StickyCardRow>(SELECT_STICKY_CARDS)
            .fetch_all(&mut **transaction)
            .await?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    async fn timestamp(
        transaction: &mut Transaction<'_, Sqlite>,
    ) -> Result<String, StickyRepositoryError> {
        Ok(
            sqlx::query_scalar("SELECT strftime('%Y-%m-%dT%H:%M:%fZ', 'now')")
                .fetch_one(&mut **transaction)
                .await?,
        )
    }

    async fn active_kind(
        transaction: &mut Transaction<'_, Sqlite>,
        id: &str,
    ) -> Result<String, StickyRepositoryError> {
        sqlx::query_scalar(
            r#"
            SELECT card_type
            FROM cards
            WHERE id = ?
              AND card_type IN ('note', 'task')
              AND lifecycle IN ('active', 'completed')
            "#,
        )
        .bind(id)
        .fetch_optional(&mut **transaction)
        .await?
        .ok_or(StickyRepositoryError::NotFound)
    }
}

#[async_trait]
impl StickyRepository for SqliteStickyRepository {
    async fn list(&self) -> Result<Vec<StickyCard>, StickyRepositoryError> {
        let rows = sqlx::query_as::<_, StickyCardRow>(SELECT_STICKY_CARDS)
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    async fn create(&self, card: &NewStickyCard) -> Result<StickyCard, StickyRepositoryError> {
        let mut transaction = self.pool.begin().await?;
        let id: String = sqlx::query_scalar("SELECT 'card_' || lower(hex(randomblob(16)))")
            .fetch_one(&mut *transaction)
            .await?;
        let timestamp = Self::timestamp(&mut transaction).await?;
        let position: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(position) + 1, 0) FROM card_placements WHERE surface = 'sticky'",
        )
        .fetch_one(&mut *transaction)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO cards (
                id, card_type, title, lifecycle, attention, source_kind,
                source_agent_id, source_delivery_id, metadata_json,
                created_at, updated_at
            ) VALUES (?, ?, NULL, 'active', NULL, 'user', NULL, NULL, '{}', ?, ?)
            "#,
        )
        .bind(&id)
        .bind(card.kind.as_str())
        .bind(&timestamp)
        .bind(&timestamp)
        .execute(&mut *transaction)
        .await?;

        match card.kind {
            StickyCardKind::Note => {
                sqlx::query("INSERT INTO note_payloads (card_id, body) VALUES (?, ?)")
                    .bind(&id)
                    .bind(&card.text)
                    .execute(&mut *transaction)
                    .await?;
            }
            StickyCardKind::Task => {
                sqlx::query("INSERT INTO task_payloads (card_id, text, due_date) VALUES (?, ?, ?)")
                    .bind(&id)
                    .bind(&card.text)
                    .bind(&card.due_date)
                    .execute(&mut *transaction)
                    .await?;
            }
        }

        sqlx::query(
            r#"
            INSERT INTO card_placements (card_id, surface, position, created_at, updated_at)
            VALUES (?, 'sticky', ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(position)
        .bind(&timestamp)
        .bind(&timestamp)
        .execute(&mut *transaction)
        .await?;

        let created = Self::fetch_in_transaction(&mut transaction, &id).await?;
        transaction.commit().await?;
        Ok(created)
    }

    async fn update_text(&self, id: &str, text: &str) -> Result<StickyCard, StickyRepositoryError> {
        let mut transaction = self.pool.begin().await?;
        let kind = Self::active_kind(&mut transaction, id).await?;
        let result = match kind.as_str() {
            "note" => {
                sqlx::query("UPDATE note_payloads SET body = ? WHERE card_id = ?")
                    .bind(text)
                    .bind(id)
                    .execute(&mut *transaction)
                    .await?
            }
            "task" => {
                sqlx::query("UPDATE task_payloads SET text = ? WHERE card_id = ?")
                    .bind(text)
                    .bind(id)
                    .execute(&mut *transaction)
                    .await?
            }
            _ => return Err(StickyRepositoryError::InvalidOperation),
        };
        if result.rows_affected() != 1 {
            return Err(StickyRepositoryError::NotFound);
        }
        let timestamp = Self::timestamp(&mut transaction).await?;
        sqlx::query("UPDATE cards SET updated_at = ? WHERE id = ?")
            .bind(&timestamp)
            .bind(id)
            .execute(&mut *transaction)
            .await?;
        let updated = Self::fetch_in_transaction(&mut transaction, id).await?;
        transaction.commit().await?;
        Ok(updated)
    }

    async fn set_task_completed(
        &self,
        id: &str,
        completed: bool,
    ) -> Result<StickyCard, StickyRepositoryError> {
        let mut transaction = self.pool.begin().await?;
        let kind = Self::active_kind(&mut transaction, id).await?;
        if kind != "task" {
            return Err(StickyRepositoryError::InvalidOperation);
        }
        let timestamp = Self::timestamp(&mut transaction).await?;
        let lifecycle = if completed { "completed" } else { "active" };
        sqlx::query(
            r#"
            UPDATE cards
            SET lifecycle = ?,
                completed_at = CASE WHEN ? THEN ? ELSE NULL END,
                updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(lifecycle)
        .bind(completed)
        .bind(&timestamp)
        .bind(&timestamp)
        .bind(id)
        .execute(&mut *transaction)
        .await?;
        let updated = Self::fetch_in_transaction(&mut transaction, id).await?;
        transaction.commit().await?;
        Ok(updated)
    }

    async fn set_task_due_date(
        &self,
        id: &str,
        due_date: Option<&str>,
    ) -> Result<StickyCard, StickyRepositoryError> {
        let mut transaction = self.pool.begin().await?;
        let kind = Self::active_kind(&mut transaction, id).await?;
        if kind != "task" {
            return Err(StickyRepositoryError::InvalidOperation);
        }
        let result = sqlx::query("UPDATE task_payloads SET due_date = ? WHERE card_id = ?")
            .bind(due_date)
            .bind(id)
            .execute(&mut *transaction)
            .await?;
        if result.rows_affected() != 1 {
            return Err(StickyRepositoryError::NotFound);
        }
        let timestamp = Self::timestamp(&mut transaction).await?;
        sqlx::query("UPDATE cards SET updated_at = ? WHERE id = ?")
            .bind(&timestamp)
            .bind(id)
            .execute(&mut *transaction)
            .await?;
        let updated = Self::fetch_in_transaction(&mut transaction, id).await?;
        transaction.commit().await?;
        Ok(updated)
    }

    async fn delete(&self, id: &str) -> Result<(), StickyRepositoryError> {
        let mut transaction = self.pool.begin().await?;
        let timestamp = Self::timestamp(&mut transaction).await?;
        let result = sqlx::query(
            r#"
            UPDATE cards
            SET lifecycle = 'deleted', deleted_at = ?, updated_at = ?
            WHERE id = ?
              AND card_type IN ('note', 'task')
              AND lifecycle IN ('active', 'completed')
            "#,
        )
        .bind(&timestamp)
        .bind(&timestamp)
        .bind(id)
        .execute(&mut *transaction)
        .await?;
        if result.rows_affected() != 1 {
            return Err(StickyRepositoryError::NotFound);
        }
        sqlx::query("DELETE FROM card_placements WHERE card_id = ? AND surface = 'sticky'")
            .bind(id)
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        Ok(())
    }

    async fn reorder(
        &self,
        ordered_ids: &[String],
    ) -> Result<Vec<StickyCard>, StickyRepositoryError> {
        let mut transaction = self.pool.begin().await?;
        let current_ids: Vec<String> = sqlx::query_scalar(
            r#"
            SELECT placement.card_id
            FROM card_placements AS placement
            INNER JOIN cards AS card ON card.id = placement.card_id
            WHERE placement.surface = 'sticky'
              AND card.lifecycle IN ('active', 'completed')
            ORDER BY placement.position ASC
            "#,
        )
        .fetch_all(&mut *transaction)
        .await?;
        let current_set: HashSet<_> = current_ids.iter().collect();
        let requested_set: HashSet<_> = ordered_ids.iter().collect();
        if current_ids.len() != ordered_ids.len()
            || requested_set.len() != ordered_ids.len()
            || current_set != requested_set
        {
            return Err(StickyRepositoryError::InvalidOrder);
        }
        if current_ids == ordered_ids {
            return Self::list_in_transaction(&mut transaction).await;
        }

        let max_position: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(position), 0) FROM card_placements WHERE surface = 'sticky'",
        )
        .fetch_one(&mut *transaction)
        .await?;
        let offset = max_position + ordered_ids.len() as i64 + 1;
        sqlx::query("UPDATE card_placements SET position = position + ? WHERE surface = 'sticky'")
            .bind(offset)
            .execute(&mut *transaction)
            .await?;

        let timestamp = Self::timestamp(&mut transaction).await?;
        for (position, id) in ordered_ids.iter().enumerate() {
            let result = sqlx::query(
                r#"
                UPDATE card_placements
                SET position = ?, updated_at = ?
                WHERE card_id = ? AND surface = 'sticky'
                "#,
            )
            .bind(position as i64)
            .bind(&timestamp)
            .bind(id)
            .execute(&mut *transaction)
            .await?;
            if result.rows_affected() != 1 {
                return Err(StickyRepositoryError::InvalidOrder);
            }
        }

        let reordered = Self::list_in_transaction(&mut transaction).await?;
        transaction.commit().await?;
        Ok(reordered)
    }

    async fn get_profile(&self) -> Result<StickyProfile, StickyRepositoryError> {
        let row: (String, String) = sqlx::query_as(
            "SELECT quote_text, updated_at FROM sticky_surface_profile WHERE surface = 'sticky'",
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(StickyProfile {
            quote_text: row.0,
            updated_at: row.1,
        })
    }

    async fn update_quote(&self, quote_text: &str) -> Result<StickyProfile, StickyRepositoryError> {
        let mut transaction = self.pool.begin().await?;
        let timestamp = Self::timestamp(&mut transaction).await?;
        let result = sqlx::query(
            "UPDATE sticky_surface_profile SET quote_text = ?, updated_at = ? WHERE surface = 'sticky'",
        )
        .bind(quote_text)
        .bind(&timestamp)
        .execute(&mut *transaction)
        .await?;
        if result.rows_affected() != 1 {
            return Err(StickyRepositoryError::NotFound);
        }
        transaction.commit().await?;
        Ok(StickyProfile {
            quote_text: quote_text.to_owned(),
            updated_at: timestamp,
        })
    }

    async fn get_record_text(&self, id: &str) -> Result<String, StickyRepositoryError> {
        sqlx::query_scalar(
            r#"
            SELECT note.body
            FROM note_payloads AS note
            INNER JOIN cards AS card ON card.id = note.card_id
            WHERE note.card_id = ?
              AND card.card_type = 'note'
              AND card.lifecycle = 'active'
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(StickyRepositoryError::NotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn repository(name: &str) -> SqliteStickyRepository {
        let temp = tempfile::tempdir().expect("temp directory");
        let path = temp.path().join(name);
        let database = crate::persistence::sqlite::connect(&path)
            .await
            .expect("database should initialize");
        // Keep the temporary directory alive for the pooled connection.
        std::mem::forget(temp);
        SqliteStickyRepository::new(database.0)
    }

    #[tokio::test]
    async fn creates_reads_updates_and_deletes_a_note() {
        let repository = repository("note-crud.sqlite3").await;
        let created = repository
            .create(&NewStickyCard {
                kind: StickyCardKind::Note,
                text: "A quiet note".to_string(),
                due_date: None,
            })
            .await
            .expect("note should be created");
        assert_eq!(created.kind, StickyCardKind::Note);
        assert_eq!(repository.list().await.unwrap(), vec![created.clone()]);

        let updated = repository
            .update_text(&created.id, "A revised\nmultiline note")
            .await
            .expect("note should update");
        assert_eq!(updated.text, "A revised\nmultiline note");

        repository
            .delete(&created.id)
            .await
            .expect("note should delete");
        assert!(repository.list().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn persists_a_5000_character_record_update_and_delete() {
        let repository = repository("long-record.sqlite3").await;
        let original = "长".repeat(5_500);
        let created = repository
            .create(&NewStickyCard {
                kind: StickyCardKind::Note,
                text: original.clone(),
                due_date: None,
            })
            .await
            .expect("long record should be created");
        assert_eq!(
            repository.get_record_text(&created.id).await.unwrap(),
            original
        );

        let updated = format!("更新后的第一行\n{}", "文".repeat(5_200));
        repository.update_text(&created.id, &updated).await.unwrap();
        assert_eq!(
            repository.get_record_text(&created.id).await.unwrap(),
            updated
        );
        repository.delete(&created.id).await.unwrap();
        assert!(matches!(
            repository.get_record_text(&created.id).await,
            Err(StickyRepositoryError::NotFound)
        ));
    }

    #[tokio::test]
    async fn quote_create_update_and_restart_restore() {
        let temp = tempfile::tempdir().expect("temp directory");
        let path = temp.path().join("quote.sqlite3");
        let database = crate::persistence::sqlite::connect(&path).await.unwrap();
        let repository = SqliteStickyRepository::new(database.0.clone());
        assert_eq!(repository.get_profile().await.unwrap().quote_text, "");
        repository.update_quote("多花点时间玩。").await.unwrap();
        database.0.close().await;

        let reopened = crate::persistence::sqlite::connect(&path).await.unwrap();
        let restored = SqliteStickyRepository::new(reopened.0);
        assert_eq!(
            restored.get_profile().await.unwrap().quote_text,
            "多花点时间玩。"
        );
    }

    #[tokio::test]
    async fn invalid_profile_update_rolls_back() {
        let repository = repository("quote-rollback.sqlite3").await;
        repository.update_quote("保留原文").await.unwrap();
        let invalid = "字".repeat(10_001);
        assert!(repository.update_quote(&invalid).await.is_err());
        assert_eq!(
            repository.get_profile().await.unwrap().quote_text,
            "保留原文"
        );
    }

    #[tokio::test]
    async fn persists_task_text_completion_and_due_date() {
        let repository = repository("task-crud.sqlite3").await;
        let created = repository
            .create(&NewStickyCard {
                kind: StickyCardKind::Task,
                text: "Write the PRD".to_string(),
                due_date: Some("2026-08-12".to_string()),
            })
            .await
            .expect("task should be created");
        assert_eq!(created.due_date.as_deref(), Some("2026-08-12"));
        assert!(!created.completed);

        let edited = repository
            .update_text(&created.id, "Review the PRD")
            .await
            .expect("task text should update");
        assert_eq!(edited.text, "Review the PRD");
        let completed = repository
            .set_task_completed(&created.id, true)
            .await
            .expect("task should complete");
        assert!(completed.completed);
        let restored = repository
            .set_task_completed(&created.id, false)
            .await
            .expect("task should restore");
        assert!(!restored.completed);
        let cleared = repository
            .set_task_due_date(&created.id, None)
            .await
            .expect("due date should clear");
        assert_eq!(cleared.due_date, None);

        repository
            .delete(&created.id)
            .await
            .expect("task should delete");
        assert!(repository.list().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn reorders_notes_and_tasks_in_one_transaction() {
        let repository = repository("reorder.sqlite3").await;
        let note = repository
            .create(&NewStickyCard {
                kind: StickyCardKind::Note,
                text: "A note".to_string(),
                due_date: None,
            })
            .await
            .unwrap();
        let first_task = repository
            .create(&NewStickyCard {
                kind: StickyCardKind::Task,
                text: "First task".to_string(),
                due_date: None,
            })
            .await
            .unwrap();
        let second_task = repository
            .create(&NewStickyCard {
                kind: StickyCardKind::Task,
                text: "Second task".to_string(),
                due_date: None,
            })
            .await
            .unwrap();

        let order = vec![
            second_task.id.clone(),
            note.id.clone(),
            first_task.id.clone(),
        ];
        let reordered = repository
            .reorder(&order)
            .await
            .expect("reorder should commit");
        assert_eq!(
            reordered.iter().map(|card| &card.id).collect::<Vec<_>>(),
            order.iter().collect::<Vec<_>>()
        );
        assert_eq!(
            reordered
                .iter()
                .map(|card| card.position)
                .collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
    }

    #[tokio::test]
    async fn invalid_reorder_rolls_back_without_changing_placement() {
        let repository = repository("reorder-rollback.sqlite3").await;
        let note = repository
            .create(&NewStickyCard {
                kind: StickyCardKind::Note,
                text: "Keep me first".to_string(),
                due_date: None,
            })
            .await
            .unwrap();
        let task = repository
            .create(&NewStickyCard {
                kind: StickyCardKind::Task,
                text: "Keep me second".to_string(),
                due_date: None,
            })
            .await
            .unwrap();
        let before = repository.list().await.unwrap();

        let error = repository
            .reorder(std::slice::from_ref(&task.id))
            .await
            .expect_err("partial order should fail");
        assert!(matches!(error, StickyRepositoryError::InvalidOrder));
        assert_eq!(repository.list().await.unwrap(), before);
        assert_eq!(before[0].id, note.id);
    }
}
