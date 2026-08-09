use async_trait::async_trait;
use sqlx::SqlitePool;

use crate::{
    application::ports::card_repository::CardRepository,
    domain::card::{CardRecord, NewBaseCard},
};

#[derive(Clone)]
pub struct SqliteCardRepository {
    pool: SqlitePool,
}

impl SqliteCardRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CardRepository for SqliteCardRepository {
    async fn create_base(&self, card: &NewBaseCard) -> Result<CardRecord, sqlx::Error> {
        let mut transaction = self.pool.begin().await?;

        sqlx::query(
            r#"
            INSERT INTO cards (
                id, card_type, title, lifecycle, attention, source_kind,
                source_agent_id, source_delivery_id, metadata_json,
                created_at, updated_at
            ) VALUES (?, ?, ?, 'active', NULL, ?, ?, ?, '{}', ?, ?)
            "#,
        )
        .bind(&card.id)
        .bind(card.card_type.as_str())
        .bind(&card.title)
        .bind(&card.source_kind)
        .bind(&card.source_agent_id)
        .bind(&card.source_delivery_id)
        .bind(&card.created_at)
        .bind(&card.created_at)
        .execute(&mut *transaction)
        .await?;

        let created = sqlx::query_as::<_, CardRecord>("SELECT * FROM cards WHERE id = ?")
            .bind(&card.id)
            .fetch_one(&mut *transaction)
            .await?;

        transaction.commit().await?;
        Ok(created)
    }

    async fn get_by_id(&self, id: &str) -> Result<Option<CardRecord>, sqlx::Error> {
        sqlx::query_as::<_, CardRecord>("SELECT * FROM cards WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::card::CardType;

    #[tokio::test]
    async fn creates_and_reads_a_base_card_transactionally() {
        let temp = tempfile::tempdir().expect("temp directory");
        let database =
            crate::persistence::sqlite::connect(&temp.path().join("repository-test.sqlite3"))
                .await
                .expect("database should initialize");
        let repository = SqliteCardRepository::new(database.0);
        let card = NewBaseCard {
            id: "card_test_1".to_string(),
            card_type: CardType::Note,
            title: Some("Foundation card".to_string()),
            source_kind: "user".to_string(),
            source_agent_id: None,
            source_delivery_id: None,
            created_at: "2026-08-10T00:00:00Z".to_string(),
        };

        let created = repository
            .create_base(&card)
            .await
            .expect("base card should be inserted");
        let loaded = repository
            .get_by_id(&card.id)
            .await
            .expect("query should succeed")
            .expect("card should exist");

        assert_eq!(created.id, card.id);
        assert_eq!(loaded.card_type, "note");
        assert_eq!(loaded.lifecycle, "active");
    }

    #[tokio::test]
    async fn rejects_an_agent_card_without_agent_identity() {
        let temp = tempfile::tempdir().expect("temp directory");
        let database =
            crate::persistence::sqlite::connect(&temp.path().join("constraint-test.sqlite3"))
                .await
                .expect("database should initialize");
        let repository = SqliteCardRepository::new(database.0);
        let card = NewBaseCard {
            id: "card_invalid".to_string(),
            card_type: CardType::AgentMessage,
            title: None,
            source_kind: "agent".to_string(),
            source_agent_id: None,
            source_delivery_id: None,
            created_at: "2026-08-10T00:00:00Z".to_string(),
        };

        assert!(repository.create_base(&card).await.is_err());
        assert!(repository
            .get_by_id(&card.id)
            .await
            .expect("query should succeed")
            .is_none());
    }
}
