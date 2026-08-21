use async_trait::async_trait;
use sqlx::{Sqlite, SqlitePool, Transaction};

use crate::{
    application::ports::proposal_repository::{
        AcceptProposalResult, ProposalAcceptance, ProposalRepository, ProposalRepositoryError,
    },
    domain::proposal::{AgentProposal, AgentProposalStatus, AgentProposalType, NewAgentProposal},
    persistence::{
        reading_repository::SqliteReadingRepository, sticky_repository::SqliteStickyRepository,
    },
};

#[derive(Debug, sqlx::FromRow)]
struct AgentProposalRow {
    id: String,
    proposal_type: String,
    title: String,
    description: String,
    payload_json: String,
    source_delivery_id: Option<String>,
    status: String,
    created_at: String,
    resolved_at: Option<String>,
}

impl TryFrom<AgentProposalRow> for AgentProposal {
    type Error = ProposalRepositoryError;

    fn try_from(row: AgentProposalRow) -> Result<Self, Self::Error> {
        let proposal_type = match row.proposal_type.as_str() {
            "todo" => AgentProposalType::Todo,
            "record" => AgentProposalType::Record,
            "reading" => AgentProposalType::Reading,
            _ => return Err(Self::Error::Storage("unexpected proposal type".to_owned())),
        };
        let status = match row.status.as_str() {
            "pending" => AgentProposalStatus::Pending,
            "accepted" => AgentProposalStatus::Accepted,
            "rejected" => AgentProposalStatus::Rejected,
            _ => {
                return Err(Self::Error::Storage(
                    "unexpected proposal status".to_owned(),
                ))
            }
        };
        Ok(Self {
            id: row.id,
            proposal_type,
            title: row.title,
            description: row.description,
            payload_json: row.payload_json,
            source_delivery_id: row.source_delivery_id,
            status,
            created_at: row.created_at,
            resolved_at: row.resolved_at,
        })
    }
}

#[derive(Clone)]
pub struct SqliteProposalRepository {
    pool: SqlitePool,
}

impl SqliteProposalRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    async fn timestamp(
        transaction: &mut Transaction<'_, Sqlite>,
    ) -> Result<String, ProposalRepositoryError> {
        Ok(
            sqlx::query_scalar("SELECT strftime('%Y-%m-%dT%H:%M:%fZ', 'now')")
                .fetch_one(&mut **transaction)
                .await?,
        )
    }

    async fn fetch_in_transaction(
        transaction: &mut Transaction<'_, Sqlite>,
        id: &str,
    ) -> Result<AgentProposal, ProposalRepositoryError> {
        sqlx::query_as::<_, AgentProposalRow>("SELECT * FROM agent_proposals WHERE id = ?")
            .bind(id)
            .fetch_optional(&mut **transaction)
            .await?
            .ok_or(ProposalRepositoryError::NotFound)?
            .try_into()
    }

    async fn resolve(
        transaction: &mut Transaction<'_, Sqlite>,
        id: &str,
        status: AgentProposalStatus,
    ) -> Result<AgentProposal, ProposalRepositoryError> {
        let timestamp = Self::timestamp(transaction).await?;
        let result = sqlx::query(
            "UPDATE agent_proposals SET status = ?, resolved_at = ? WHERE id = ? AND status = 'pending'",
        )
        .bind(status.as_str())
        .bind(&timestamp)
        .bind(id)
        .execute(&mut **transaction)
        .await?;
        if result.rows_affected() != 1 {
            let exists: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM agent_proposals WHERE id = ?")
                    .bind(id)
                    .fetch_one(&mut **transaction)
                    .await?;
            return Err(if exists == 0 {
                ProposalRepositoryError::NotFound
            } else {
                ProposalRepositoryError::AlreadyResolved
            });
        }
        Self::fetch_in_transaction(transaction, id).await
    }
}

#[async_trait]
impl ProposalRepository for SqliteProposalRepository {
    async fn create(
        &self,
        proposal: &NewAgentProposal,
    ) -> Result<AgentProposal, ProposalRepositoryError> {
        let mut transaction = self.pool.begin().await?;
        let id: String = sqlx::query_scalar("SELECT 'proposal_' || lower(hex(randomblob(16)))")
            .fetch_one(&mut *transaction)
            .await?;
        let timestamp = Self::timestamp(&mut transaction).await?;
        sqlx::query(
            r#"
            INSERT INTO agent_proposals (
                id, proposal_type, title, description, payload_json,
                source_delivery_id, status, created_at, resolved_at
            ) VALUES (?, ?, ?, ?, ?, ?, 'pending', ?, NULL)
            "#,
        )
        .bind(&id)
        .bind(proposal.proposal_type.as_str())
        .bind(&proposal.title)
        .bind(&proposal.description)
        .bind(&proposal.payload_json)
        .bind(&proposal.source_delivery_id)
        .bind(&timestamp)
        .execute(&mut *transaction)
        .await?;
        let created = Self::fetch_in_transaction(&mut transaction, &id).await?;
        transaction.commit().await?;
        Ok(created)
    }

    async fn list_for_document(
        &self,
        document_id: &str,
    ) -> Result<Vec<AgentProposal>, ProposalRepositoryError> {
        let rows = sqlx::query_as::<_, AgentProposalRow>(
            r#"
            SELECT p.*
            FROM agent_proposals p
            JOIN deliveries d ON d.id = p.source_delivery_id
            WHERE d.document_id = ?
            ORDER BY p.created_at ASC, p.id ASC
            "#,
        )
        .bind(document_id)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    async fn get(&self, id: &str) -> Result<Option<AgentProposal>, ProposalRepositoryError> {
        sqlx::query_as::<_, AgentProposalRow>("SELECT * FROM agent_proposals WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .map(TryInto::try_into)
            .transpose()
    }

    async fn accept(
        &self,
        id: &str,
        action: ProposalAcceptance,
    ) -> Result<AcceptProposalResult, ProposalRepositoryError> {
        let mut transaction = self.pool.begin().await?;
        let current = Self::fetch_in_transaction(&mut transaction, id).await?;
        if current.status != AgentProposalStatus::Pending {
            return Err(ProposalRepositoryError::AlreadyResolved);
        }
        let (card, reading_session) = match action {
            ProposalAcceptance::Sticky(card) => {
                let created =
                    SqliteStickyRepository::create_in_transaction(&mut transaction, &card)
                        .await
                        .map_err(|error| ProposalRepositoryError::Storage(error.to_string()))?;
                (Some(created), None)
            }
            ProposalAcceptance::Reading {
                source_delivery_id,
                content,
                estimated_minutes,
            } => {
                let source_document_id: String =
                    sqlx::query_scalar("SELECT document_id FROM deliveries WHERE id = ?")
                        .bind(source_delivery_id)
                        .fetch_optional(&mut *transaction)
                        .await?
                        .ok_or(ProposalRepositoryError::SourceNotFound)?;
                let created = SqliteReadingRepository::create_session_in_transaction(
                    &mut transaction,
                    &source_document_id,
                    &content,
                    estimated_minutes,
                )
                .await
                .map_err(|error| ProposalRepositoryError::Storage(error.to_string()))?;
                (None, Some(created))
            }
        };
        let proposal = Self::resolve(&mut transaction, id, AgentProposalStatus::Accepted).await?;
        transaction.commit().await?;
        Ok(AcceptProposalResult {
            proposal,
            card,
            reading_session,
        })
    }

    async fn reject(&self, id: &str) -> Result<AgentProposal, ProposalRepositoryError> {
        let mut transaction = self.pool.begin().await?;
        let proposal = Self::resolve(&mut transaction, id, AgentProposalStatus::Rejected).await?;
        transaction.commit().await?;
        Ok(proposal)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use serde_json::json;

    use super::*;
    use crate::{
        application::{
            delivery_service::DeliveryService, ports::sticky_repository::StickyRepository,
            proposal_service::ProposalService,
        },
        domain::{
            delivery::CreateDeliveryInput,
            proposal::{AgentProposalStatus, AgentProposalType, CreateAgentProposalInput},
            reader::{ReaderDocumentType, ReaderSourceType},
        },
        persistence::{delivery_repository::SqliteDeliveryRepository, sqlite},
    };

    async fn source_delivery(pool: &SqlitePool) -> crate::domain::delivery::IngestDeliveryResult {
        let repository = SqliteDeliveryRepository::new(pool.clone());
        DeliveryService::new(Arc::new(repository))
            .ingest(CreateDeliveryInput {
                idempotency_key: "proposal-test-source".to_owned(),
                document_type: ReaderDocumentType::Brief,
                title: "会议总结".to_owned(),
                subtitle: None,
                content_markdown: "会议决定了三项下一步行动。".to_owned(),
                source_type: ReaderSourceType::Agent,
                source_label: Some("Reading Agent".to_owned()),
                delivered_at: None,
            })
            .await
            .unwrap()
    }

    fn input(
        proposal_type: AgentProposalType,
        payload: serde_json::Value,
        delivery_id: &str,
    ) -> CreateAgentProposalInput {
        CreateAgentProposalInput {
            proposal_type,
            title: "建议".to_owned(),
            description: "请确认是否加入工作区。".to_owned(),
            payload,
            source_delivery_id: Some(delivery_id.to_owned()),
        }
    }

    #[tokio::test]
    async fn accepts_each_action_rejects_without_mutation_and_persists_after_restart() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("proposal.sqlite3");
        let database = sqlite::connect(&path).await.unwrap();
        let source = source_delivery(&database.0).await;
        let document_id = source.item.document.id.clone();
        let delivery_id = source.item.delivery.id.clone();
        let repository = SqliteProposalRepository::new(database.0.clone());
        let service = ProposalService::new(Arc::new(repository));

        let todo = service
            .create(input(
                AgentProposalType::Todo,
                json!({ "content": "整理会议纪要" }),
                &delivery_id,
            ))
            .await
            .unwrap();
        let record = service
            .create(input(
                AgentProposalType::Record,
                json!({ "content": "关键结论需要在周五前确认。" }),
                &delivery_id,
            ))
            .await
            .unwrap();
        let reading = service
            .create(input(
                AgentProposalType::Reading,
                json!({ "content": "延伸阅读正文", "estimatedMinutes": 3 }),
                &delivery_id,
            ))
            .await
            .unwrap();
        let rejected = service
            .create(input(
                AgentProposalType::Todo,
                json!({ "content": "不应创建的待办" }),
                &delivery_id,
            ))
            .await
            .unwrap();

        let accepted_todo = service.accept(&todo.id).await.unwrap();
        let accepted_record = service.accept(&record.id).await.unwrap();
        let accepted_reading = service.accept(&reading.id).await.unwrap();
        let card_count_before_reject = SqliteStickyRepository::new(database.0.clone())
            .list()
            .await
            .unwrap()
            .len();
        let rejected = service.reject(&rejected.id).await.unwrap();
        let card_count_after_reject = SqliteStickyRepository::new(database.0.clone())
            .list()
            .await
            .unwrap()
            .len();

        assert_eq!(accepted_todo.card.unwrap().text, "整理会议纪要");
        assert_eq!(
            accepted_record.card.unwrap().text,
            "关键结论需要在周五前确认。"
        );
        assert_eq!(
            accepted_reading
                .reading_session
                .unwrap()
                .session
                .estimated_minutes,
            3
        );
        assert_eq!(rejected.status, AgentProposalStatus::Rejected);
        assert_eq!(card_count_before_reject, card_count_after_reject);
        assert!(service.accept(&todo.id).await.is_err());
        assert_eq!(
            SqliteStickyRepository::new(database.0.clone())
                .list()
                .await
                .unwrap()
                .len(),
            card_count_after_reject
        );
        database.0.close().await;

        let reopened = sqlite::connect(&path).await.unwrap();
        let persisted = SqliteProposalRepository::new(reopened.0)
            .list_for_document(&document_id)
            .await
            .unwrap();
        assert_eq!(persisted.len(), 4);
        assert_eq!(
            persisted
                .iter()
                .filter(|proposal| proposal.status == AgentProposalStatus::Accepted)
                .count(),
            3
        );
        assert_eq!(
            persisted
                .iter()
                .filter(|proposal| proposal.status == AgentProposalStatus::Rejected)
                .count(),
            1
        );
    }
}
