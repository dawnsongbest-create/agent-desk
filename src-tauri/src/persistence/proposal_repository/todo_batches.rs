use sqlx::FromRow;
use uuid::Uuid;

use super::{ProposalRepositoryError, SqliteProposalRepository, SqliteStickyRepository};
use crate::{
    application::sticky_service::StickyService,
    domain::{
        proposal::{
            todo_batch::{
                AcceptedTodoBatch, TodoSuggestionBatch, TodoSuggestionItem, TodoSuggestionSource,
            },
            AgentProposalStatus,
        },
        sticky::StickyCardKind,
    },
};

#[derive(FromRow)]
struct BatchRow {
    id: String,
    source: String,
    items_json: String,
    status: String,
    created_at: String,
    accepted_todo_ids_json: Option<String>,
}

fn invalid_batch() -> ProposalRepositoryError {
    ProposalRepositoryError::Storage("Invalid persisted Todo batch".into())
}

impl BatchRow {
    fn batch(&self) -> Result<TodoSuggestionBatch, ProposalRepositoryError> {
        let status = match self.status.as_str() {
            "pending" => AgentProposalStatus::Pending,
            "accepted" => AgentProposalStatus::Accepted,
            "rejected" => AgentProposalStatus::Rejected,
            _ => return Err(invalid_batch()),
        };
        Ok(TodoSuggestionBatch {
            id: self.id.clone(),
            source: match self.source.as_str() {
                "codex" => TodoSuggestionSource::Codex,
                "dumate" => TodoSuggestionSource::Dumate,
                _ => return Err(invalid_batch()),
            },
            items: serde_json::from_str(&self.items_json).map_err(|_| invalid_batch())?,
            status,
            created_at: self.created_at.clone(),
        })
    }

    fn receipt(&self) -> Result<AcceptedTodoBatch, ProposalRepositoryError> {
        let ids = self
            .accepted_todo_ids_json
            .as_deref()
            .ok_or_else(invalid_batch)?;
        Ok(AcceptedTodoBatch {
            todo_ids: serde_json::from_str(ids).map_err(|_| invalid_batch())?,
        })
    }
}

impl SqliteProposalRepository {
    pub(super) async fn submit_batch(
        &self,
        namespace: &str,
        request_digest: &str,
        items: &[TodoSuggestionItem],
    ) -> Result<(TodoSuggestionBatch, bool), ProposalRepositoryError> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let json = serde_json::to_string(items).map_err(|_| invalid_batch())?;
        let result = sqlx::query(
            "INSERT INTO todo_suggestion_batches
                (id, dedupe_namespace, request_digest, items_json, created_at, updated_at)
            VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
            ON CONFLICT(dedupe_namespace, request_digest) DO NOTHING",
        )
        .bind(format!("todo_batch_{}", Uuid::new_v4().simple()))
        .bind(namespace)
        .bind(request_digest)
        .bind(&json)
        .execute(&mut *tx)
        .await?;
        let row: BatchRow = sqlx::query_as(
            "SELECT b.*, s.provider AS source FROM todo_suggestion_batches b
            JOIN todo_suggestion_sources s USING (dedupe_namespace)
            WHERE b.dedupe_namespace=? AND b.request_digest=?",
        )
        .bind(namespace)
        .bind(request_digest)
        .fetch_one(&mut *tx)
        .await?;
        if row.items_json != json {
            return Err(ProposalRepositoryError::TodoBatchConflict);
        }
        let batch = row.batch()?;
        tx.commit().await?;
        Ok((batch, result.rows_affected() == 1))
    }

    pub(super) async fn pending_batches(
        &self,
    ) -> Result<Vec<TodoSuggestionBatch>, ProposalRepositoryError> {
        sqlx::query_as::<_, BatchRow>(
            "SELECT b.*, s.provider AS source FROM todo_suggestion_batches b
            JOIN todo_suggestion_sources s USING (dedupe_namespace)
            WHERE b.status='pending' ORDER BY b.created_at ASC, b.id ASC",
        )
        .fetch_all(&self.pool)
        .await?
        .iter()
        .map(BatchRow::batch)
        .collect()
    }

    pub(super) async fn accept_batch(
        &self,
        id: &str,
    ) -> Result<AcceptedTodoBatch, ProposalRepositoryError> {
        // Take the write lock before reading status or allocating positions.
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let row: BatchRow = sqlx::query_as(
            "SELECT b.*, s.provider AS source FROM todo_suggestion_batches b
            JOIN todo_suggestion_sources s USING (dedupe_namespace) WHERE b.id=?",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(ProposalRepositoryError::NotFound)?;
        let batch = row.batch()?;
        match batch.status {
            AgentProposalStatus::Accepted => return row.receipt(),
            AgentProposalStatus::Rejected => return Err(ProposalRepositoryError::AlreadyResolved),
            AgentProposalStatus::Pending => {}
        }
        let mut todo_ids = Vec::with_capacity(batch.items.len());
        for item in batch.items {
            let card =
                StickyService::prepare_create(StickyCardKind::Task, item.text, item.due_date)
                    .map_err(|_| invalid_batch())?;
            let card = SqliteStickyRepository::create_in_transaction(&mut tx, &card)
                .await
                .map_err(|error| ProposalRepositoryError::Storage(error.to_string()))?;
            todo_ids.push(card.id);
        }
        let ids_json = serde_json::to_string(&todo_ids).map_err(|_| invalid_batch())?;
        let updated = sqlx::query(
            "UPDATE todo_suggestion_batches SET status='accepted',
            updated_at=CURRENT_TIMESTAMP, accepted_todo_ids_json=? WHERE id=? AND status='pending'",
        )
        .bind(ids_json)
        .bind(id)
        .execute(&mut *tx)
        .await?;
        if updated.rows_affected() != 1 {
            return Err(ProposalRepositoryError::AlreadyResolved);
        }
        if let Err(error) = tx.commit().await {
            // Read the durable receipt, never perform a second insertion here.
            let resolved = sqlx::query_as::<_, BatchRow>(
                "SELECT b.*, s.provider AS source FROM todo_suggestion_batches b
                JOIN todo_suggestion_sources s USING (dedupe_namespace)
                WHERE b.id=? AND b.status='accepted'",
            )
            .bind(id)
            .fetch_optional(&self.pool)
            .await;
            return match resolved {
                Ok(Some(row)) => row.receipt(),
                _ => Err(error.into()),
            };
        }
        Ok(AcceptedTodoBatch { todo_ids })
    }

    pub(super) async fn reject_batch(&self, id: &str) -> Result<(), ProposalRepositoryError> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let result = sqlx::query(
            "UPDATE todo_suggestion_batches SET status='rejected',
            updated_at=CURRENT_TIMESTAMP WHERE id=? AND status='pending'",
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;
        if result.rows_affected() != 1 {
            let status: Option<String> =
                sqlx::query_scalar("SELECT status FROM todo_suggestion_batches WHERE id=?")
                    .bind(id)
                    .fetch_optional(&mut *tx)
                    .await?;
            return match status.as_deref() {
                Some("rejected") => Ok(()),
                Some(_) => Err(ProposalRepositoryError::AlreadyResolved),
                None => Err(ProposalRepositoryError::NotFound),
            };
        }
        tx.commit().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::{
        application::{
            ports::sticky_repository::{StickyRepository, StickyRepositoryError},
            proposal_service::{ProposalService, ProposalServiceError},
        },
        domain::proposal::todo_batch::TodoPriority,
        persistence::sqlite,
    };

    const NAMESPACE: &str = "11112222333344445555666677778888";
    fn item(text: &str, priority: TodoPriority) -> TodoSuggestionItem {
        TodoSuggestionItem {
            text: text.into(),
            priority,
            due_date: None,
            reason: None,
        }
    }

    #[tokio::test]
    async fn batch_is_atomic_append_only_and_replays_its_receipt_after_restart() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("batch.sqlite3");
        let pool = sqlite::connect(&path).await.unwrap().0;
        sqlx::query(
            "INSERT INTO codex_todo_suggestion_grant VALUES
            ('codex-todo-suggestions', NULL, 'revoked', 'suggest_todos', ?, 'fixture', 'fixture')",
        )
        .bind(NAMESPACE)
        .execute(&pool)
        .await
        .unwrap();
        let sticky = SqliteStickyRepository::new(pool.clone());
        for (kind, text) in [
            (StickyCardKind::Task, "Original task"),
            (StickyCardKind::Note, "Original Record"),
        ] {
            sticky
                .create(&StickyService::prepare_create(kind, text.into(), None).unwrap())
                .await
                .unwrap();
        }
        let original = sticky.list().await.unwrap();
        let service = ProposalService::new(Arc::new(SqliteProposalRepository::new(pool.clone())));
        use TodoPriority::{High, Low, Medium};
        let items = vec![
            item("Low", Low),
            item("High first", High),
            item("Medium", Medium),
            item("High second", High),
        ];
        let (a, b) = tokio::join!(
            service.submit_todo_batch(NAMESPACE, "send", items.clone()),
            service.submit_todo_batch(NAMESPACE, "send", items.clone()),
        );
        let (batch, a_created) = a.unwrap();
        let (other, b_created) = b.unwrap();
        assert_eq!(batch, other);
        assert_ne!(a_created, b_created);
        assert_eq!(sticky.list().await.unwrap(), original);
        assert_eq!(
            batch
                .items
                .iter()
                .map(|item| item.text.as_str())
                .collect::<Vec<_>>(),
            ["High first", "High second", "Medium", "Low"]
        );
        assert!(matches!(
            service
                .submit_todo_batch(NAMESPACE, "send", vec![item("Changed", High)])
                .await,
            Err(ProposalServiceError::Repository(
                ProposalRepositoryError::TodoBatchConflict
            ))
        ));
        let (rejected, created) = service
            .submit_todo_batch(NAMESPACE, "independent", items.clone())
            .await
            .unwrap();
        assert!(created);
        assert_ne!(batch.id, rejected.id);
        service.reject_todo_batch(&rejected.id).await.unwrap();
        service.reject_todo_batch(&rejected.id).await.unwrap();
        assert!(service.accept_todo_batch(&rejected.id).await.is_err());
        let (replay, created) = service
            .submit_todo_batch(NAMESPACE, "independent", items.clone())
            .await
            .unwrap();
        assert!(!created);
        assert_eq!(replay.status, AgentProposalStatus::Rejected);
        let (pending, _) = service
            .submit_todo_batch(NAMESPACE, "still-pending", items.clone())
            .await
            .unwrap();
        sqlx::raw_sql(
            "CREATE TRIGGER fail_middle_task BEFORE INSERT ON task_payloads
            WHEN NEW.text='High second' BEGIN SELECT RAISE(ABORT, 'synthetic fixture'); END;",
        )
        .execute(&pool)
        .await
        .unwrap();
        assert!(service.accept_todo_batch(&batch.id).await.is_err());
        assert_eq!(sticky.list().await.unwrap(), original);
        assert!(service
            .list_pending_todo_batches()
            .await
            .unwrap()
            .contains(&batch));
        sqlx::query("DROP TRIGGER fail_middle_task")
            .execute(&pool)
            .await
            .unwrap();
        let (first, second) = tokio::join!(
            service.accept_todo_batch(&batch.id),
            service.accept_todo_batch(&batch.id)
        );
        let receipt = first.unwrap();
        assert_eq!(second.unwrap(), receipt);
        let cards = sticky.list().await.unwrap();
        assert_eq!(&cards[..original.len()], original.as_slice());
        assert_eq!(
            cards[original.len()..]
                .iter()
                .map(|card| &card.id)
                .collect::<Vec<_>>(),
            receipt.todo_ids.iter().collect::<Vec<_>>()
        );
        assert!(matches!(
            sticky
                .reorder(
                    &original
                        .iter()
                        .map(|card| card.id.clone())
                        .collect::<Vec<_>>()
                )
                .await,
            Err(StickyRepositoryError::InvalidOrder)
        ));
        let reversed: Vec<_> = cards.iter().rev().map(|card| card.id.clone()).collect();
        assert_eq!(
            sticky
                .reorder(&reversed)
                .await
                .unwrap()
                .iter()
                .map(|card| &card.id)
                .collect::<Vec<_>>(),
            reversed.iter().collect::<Vec<_>>()
        );
        drop(service);
        drop(sticky);
        pool.close().await;
        let pool = sqlite::connect(&path).await.unwrap().0;
        let service = ProposalService::new(Arc::new(SqliteProposalRepository::new(pool.clone())));
        assert_eq!(service.accept_todo_batch(&batch.id).await.unwrap(), receipt);
        let (replay, created) = service
            .submit_todo_batch(NAMESPACE, "send", items.clone())
            .await
            .unwrap();
        assert!(!created);
        assert_eq!(replay.status, AgentProposalStatus::Accepted);
        assert_eq!(
            service
                .submit_todo_batch(NAMESPACE, "independent", items.clone())
                .await
                .unwrap()
                .0
                .status,
            AgentProposalStatus::Rejected
        );
        assert_eq!(
            service.list_pending_todo_batches().await.unwrap(),
            [pending]
        );
        let (race, _) = service
            .submit_todo_batch(NAMESPACE, "race", items)
            .await
            .unwrap();
        let (accepted, rejected) = tokio::join!(
            service.accept_todo_batch(&race.id),
            service.reject_todo_batch(&race.id)
        );
        assert_ne!(accepted.is_ok(), rejected.is_ok());
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM task_payloads")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 5 + if accepted.is_ok() { 4 } else { 0 });
    }
}
