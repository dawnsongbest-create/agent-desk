use async_trait::async_trait;
use sqlx::{Sqlite, SqlitePool, Transaction};

use crate::{
    application::ports::delivery_repository::{DeliveryRepository, DeliveryRepositoryError},
    domain::{
        delivery::{CreateDeliveryInput, Delivery, InboxDelivery, IngestDeliveryResult},
        reader::{ReaderDocument, ReaderDocumentType, ReaderSourceType},
    },
};

#[derive(Debug, sqlx::FromRow)]
struct InboxDeliveryRow {
    delivery_id: String,
    document_id: String,
    idempotency_key: String,
    delivered_at: String,
    opened_at: Option<String>,
    document_type: String,
    title: String,
    subtitle: Option<String>,
    content_markdown: String,
    source_type: String,
    source_label: Option<String>,
    created_at: String,
    updated_at: String,
}

impl TryFrom<InboxDeliveryRow> for InboxDelivery {
    type Error = DeliveryRepositoryError;

    fn try_from(row: InboxDeliveryRow) -> Result<Self, Self::Error> {
        let document_type = match row.document_type.as_str() {
            "article" => ReaderDocumentType::Article,
            "brief" => ReaderDocumentType::Brief,
            "reading" => ReaderDocumentType::Reading,
            "report" => ReaderDocumentType::Report,
            _ => {
                return Err(Self::Error::Storage(
                    "unexpected Reader document type".to_owned(),
                ))
            }
        };
        let source_type = match row.source_type.as_str() {
            "local" => ReaderSourceType::Local,
            "builtin" => ReaderSourceType::Builtin,
            "agent" => ReaderSourceType::Agent,
            _ => {
                return Err(Self::Error::Storage(
                    "unexpected Reader source type".to_owned(),
                ))
            }
        };
        Ok(Self {
            delivery: Delivery {
                id: row.delivery_id,
                document_id: row.document_id.clone(),
                idempotency_key: row.idempotency_key,
                delivered_at: row.delivered_at,
                opened_at: row.opened_at,
            },
            document: ReaderDocument {
                id: row.document_id,
                document_type,
                title: row.title,
                subtitle: row.subtitle,
                content_markdown: row.content_markdown,
                source_type,
                source_label: row.source_label,
                created_at: row.created_at,
                updated_at: row.updated_at,
            },
        })
    }
}

const INBOX_BY_KEY: &str = r#"
    SELECT
        d.id AS delivery_id, d.document_id, d.idempotency_key,
        d.delivered_at, d.opened_at,
        r.document_type, r.title, r.subtitle, r.content_markdown,
        r.source_type, r.source_label, r.created_at, r.updated_at
    FROM deliveries d
    JOIN reader_documents r ON r.id = d.document_id
    WHERE d.idempotency_key = ?
"#;

const INBOX_BY_ID: &str = r#"
    SELECT
        d.id AS delivery_id, d.document_id, d.idempotency_key,
        d.delivered_at, d.opened_at,
        r.document_type, r.title, r.subtitle, r.content_markdown,
        r.source_type, r.source_label, r.created_at, r.updated_at
    FROM deliveries d
    JOIN reader_documents r ON r.id = d.document_id
    WHERE d.id = ?
"#;

const INBOX_LIST: &str = r#"
    SELECT
        d.id AS delivery_id, d.document_id, d.idempotency_key,
        d.delivered_at, d.opened_at,
        r.document_type, r.title, r.subtitle, r.content_markdown,
        r.source_type, r.source_label, r.created_at, r.updated_at
    FROM deliveries d
    JOIN reader_documents r ON r.id = d.document_id
    ORDER BY d.delivered_at DESC, d.id DESC
"#;

#[derive(Clone)]
pub struct SqliteDeliveryRepository {
    pool: SqlitePool,
}

impl SqliteDeliveryRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    async fn timestamp(
        transaction: &mut Transaction<'_, Sqlite>,
    ) -> Result<String, DeliveryRepositoryError> {
        Ok(
            sqlx::query_scalar("SELECT strftime('%Y-%m-%dT%H:%M:%fZ', 'now')")
                .fetch_one(&mut **transaction)
                .await?,
        )
    }

    async fn fetch_by_key_from_pool(
        &self,
        key: &str,
    ) -> Result<Option<InboxDelivery>, DeliveryRepositoryError> {
        sqlx::query_as::<_, InboxDeliveryRow>(INBOX_BY_KEY)
            .bind(key)
            .fetch_optional(&self.pool)
            .await?
            .map(TryInto::try_into)
            .transpose()
    }

    async fn fetch_by_key_in_transaction(
        transaction: &mut Transaction<'_, Sqlite>,
        key: &str,
    ) -> Result<Option<InboxDelivery>, DeliveryRepositoryError> {
        sqlx::query_as::<_, InboxDeliveryRow>(INBOX_BY_KEY)
            .bind(key)
            .fetch_optional(&mut **transaction)
            .await?
            .map(TryInto::try_into)
            .transpose()
    }

    fn payload_matches(item: &InboxDelivery, input: &CreateDeliveryInput) -> bool {
        item.document.document_type == input.document_type
            && item.document.title == input.title
            && item.document.subtitle == input.subtitle
            && item.document.content_markdown == input.content_markdown
            && item.document.source_type == input.source_type
            && item.document.source_label == input.source_label
            && input
                .delivered_at
                .as_ref()
                .is_none_or(|delivered_at| item.delivery.delivered_at == *delivered_at)
    }

    fn existing_result(
        item: InboxDelivery,
        input: &CreateDeliveryInput,
    ) -> Result<IngestDeliveryResult, DeliveryRepositoryError> {
        if Self::payload_matches(&item, input) {
            Ok(IngestDeliveryResult {
                item,
                created: false,
            })
        } else {
            Err(DeliveryRepositoryError::IdempotencyConflict)
        }
    }
}

#[async_trait]
impl DeliveryRepository for SqliteDeliveryRepository {
    async fn ingest(
        &self,
        input: &CreateDeliveryInput,
    ) -> Result<IngestDeliveryResult, DeliveryRepositoryError> {
        let mut transaction = self.pool.begin().await?;
        if let Some(existing) =
            Self::fetch_by_key_in_transaction(&mut transaction, &input.idempotency_key).await?
        {
            transaction.commit().await?;
            return Self::existing_result(existing, input);
        }

        let document_id: String =
            sqlx::query_scalar("SELECT 'reader_' || lower(hex(randomblob(16)))")
                .fetch_one(&mut *transaction)
                .await?;
        let delivery_id: String =
            sqlx::query_scalar("SELECT 'delivery_' || lower(hex(randomblob(16)))")
                .fetch_one(&mut *transaction)
                .await?;
        let created_at = Self::timestamp(&mut transaction).await?;
        let delivered_at = input
            .delivered_at
            .clone()
            .unwrap_or_else(|| created_at.clone());

        sqlx::query(
            r#"
            INSERT INTO reader_documents (
                id, document_type, title, subtitle, content_markdown,
                source_type, source_label, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&document_id)
        .bind(input.document_type.as_str())
        .bind(&input.title)
        .bind(&input.subtitle)
        .bind(&input.content_markdown)
        .bind(input.source_type.as_str())
        .bind(&input.source_label)
        .bind(&created_at)
        .bind(&created_at)
        .execute(&mut *transaction)
        .await?;

        let insert_delivery = sqlx::query(
            r#"
            INSERT INTO deliveries (
                id, document_id, idempotency_key, delivered_at, opened_at
            ) VALUES (?, ?, ?, ?, NULL)
            "#,
        )
        .bind(&delivery_id)
        .bind(&document_id)
        .bind(&input.idempotency_key)
        .bind(&delivered_at)
        .execute(&mut *transaction)
        .await;

        if let Err(error) = insert_delivery {
            transaction.rollback().await?;
            if let Some(existing) = self.fetch_by_key_from_pool(&input.idempotency_key).await? {
                return Self::existing_result(existing, input);
            }
            return Err(error.into());
        }

        let created = Self::fetch_by_key_in_transaction(&mut transaction, &input.idempotency_key)
            .await?
            .ok_or(DeliveryRepositoryError::NotFound)?;
        transaction.commit().await?;
        Ok(IngestDeliveryResult {
            item: created,
            created: true,
        })
    }

    async fn list_inbox(&self) -> Result<Vec<InboxDelivery>, DeliveryRepositoryError> {
        let rows = sqlx::query_as::<_, InboxDeliveryRow>(INBOX_LIST)
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    async fn get(&self, id: &str) -> Result<Option<InboxDelivery>, DeliveryRepositoryError> {
        sqlx::query_as::<_, InboxDeliveryRow>(INBOX_BY_ID)
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .map(TryInto::try_into)
            .transpose()
    }

    async fn get_unread_count(&self) -> Result<i64, DeliveryRepositoryError> {
        Ok(
            sqlx::query_scalar("SELECT COUNT(*) FROM deliveries WHERE opened_at IS NULL")
                .fetch_one(&self.pool)
                .await?,
        )
    }

    async fn mark_opened(&self, id: &str) -> Result<InboxDelivery, DeliveryRepositoryError> {
        let mut transaction = self.pool.begin().await?;
        sqlx::query_as::<_, InboxDeliveryRow>(INBOX_BY_ID)
            .bind(id)
            .fetch_optional(&mut *transaction)
            .await?
            .ok_or(DeliveryRepositoryError::NotFound)?;
        let opened_at = Self::timestamp(&mut transaction).await?;
        sqlx::query("UPDATE deliveries SET opened_at = COALESCE(opened_at, ?) WHERE id = ?")
            .bind(opened_at)
            .bind(id)
            .execute(&mut *transaction)
            .await?;
        let opened = sqlx::query_as::<_, InboxDeliveryRow>(INBOX_BY_ID)
            .bind(id)
            .fetch_one(&mut *transaction)
            .await?
            .try_into()?;
        transaction.commit().await?;
        Ok(opened)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{domain::delivery::validate_delivery, persistence::sqlite};

    fn input(key: &str, title: &str, delivered_at: &str) -> CreateDeliveryInput {
        validate_delivery(CreateDeliveryInput {
            idempotency_key: key.to_owned(),
            document_type: ReaderDocumentType::Brief,
            title: title.to_owned(),
            subtitle: Some("今日重点".to_owned()),
            content_markdown: format!("# {title}\n\n正文与 `code`。"),
            source_type: ReaderSourceType::Agent,
            source_label: Some("Daily Brief".to_owned()),
            delivered_at: Some(delivered_at.to_owned()),
        })
        .unwrap()
    }

    #[tokio::test]
    async fn ingests_lists_newest_first_and_survives_restart() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("delivery.sqlite3");
        let database = sqlite::connect(&path).await.unwrap();
        let repository = SqliteDeliveryRepository::new(database.0.clone());
        let a = repository
            .ingest(&input("a", "第一份", "2026-08-15T08:00:00.000Z"))
            .await
            .unwrap();
        let b = repository
            .ingest(&input("b", "第二份", "2026-08-15T09:00:00.000Z"))
            .await
            .unwrap();
        assert!(a.created && b.created);
        assert_eq!(
            repository.get(&a.item.delivery.id).await.unwrap(),
            Some(a.item.clone())
        );
        assert_eq!(repository.get_unread_count().await.unwrap(), 2);
        assert_eq!(
            repository
                .list_inbox()
                .await
                .unwrap()
                .into_iter()
                .map(|item| item.document.title)
                .collect::<Vec<_>>(),
            vec!["第二份", "第一份"]
        );
        database.0.close().await;

        let reopened = sqlite::connect(&path).await.unwrap();
        let repository = SqliteDeliveryRepository::new(reopened.0);
        assert_eq!(repository.list_inbox().await.unwrap().len(), 2);
        assert_eq!(repository.get_unread_count().await.unwrap(), 2);
        let duplicate = repository
            .ingest(&input("a", "第一份", "2026-08-15T08:00:00.000Z"))
            .await
            .unwrap();
        assert!(!duplicate.created);
        assert_eq!(repository.list_inbox().await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn opening_sets_only_the_first_opened_timestamp() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("opened.sqlite3");
        let database = sqlite::connect(&path).await.unwrap();
        let repository = SqliteDeliveryRepository::new(database.0.clone());
        let created = repository
            .ingest(&input("open", "待打开", "2026-08-15T08:00:00.000Z"))
            .await
            .unwrap();
        let first = repository
            .mark_opened(&created.item.delivery.id)
            .await
            .unwrap();
        let first_opened_at = first.delivery.opened_at.clone().unwrap();
        let second = repository
            .mark_opened(&created.item.delivery.id)
            .await
            .unwrap();
        assert_eq!(
            second.delivery.opened_at.as_deref(),
            Some(first_opened_at.as_str())
        );
        assert_eq!(repository.get_unread_count().await.unwrap(), 0);
        database.0.close().await;

        let reopened = sqlite::connect(&path).await.unwrap();
        let repository = SqliteDeliveryRepository::new(reopened.0);
        assert_eq!(repository.get_unread_count().await.unwrap(), 0);
        assert_eq!(
            repository.list_inbox().await.unwrap()[0]
                .delivery
                .opened_at
                .as_deref(),
            Some(first_opened_at.as_str())
        );
    }

    #[tokio::test]
    async fn duplicate_is_idempotent_but_changed_payload_conflicts() {
        let temp = tempfile::tempdir().unwrap();
        let database = sqlite::connect(&temp.path().join("idempotency.sqlite3"))
            .await
            .unwrap();
        let repository = SqliteDeliveryRepository::new(database.0.clone());
        let original = input("same-key", "原始标题", "2026-08-15T08:00:00.000Z");
        let first = repository.ingest(&original).await.unwrap();
        let duplicate = repository.ingest(&original).await.unwrap();
        assert!(first.created);
        assert!(!duplicate.created);
        assert_eq!(first.item, duplicate.item);

        let error = repository
            .ingest(&input("same-key", "冲突标题", "2026-08-15T08:00:00.000Z"))
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            DeliveryRepositoryError::IdempotencyConflict
        ));
        assert_eq!(repository.list_inbox().await.unwrap().len(), 1);
        let document_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM reader_documents")
            .fetch_one(&database.0)
            .await
            .unwrap();
        assert_eq!(document_count, 1);
        assert_eq!(
            repository.list_inbox().await.unwrap()[0].document.title,
            "原始标题"
        );
    }

    #[tokio::test]
    async fn delivery_insert_failure_rolls_back_the_reader_document() {
        let temp = tempfile::tempdir().unwrap();
        let database = sqlite::connect(&temp.path().join("rollback.sqlite3"))
            .await
            .unwrap();
        sqlx::raw_sql(
            r#"
            CREATE TRIGGER reject_delivery_fixture
            BEFORE INSERT ON deliveries
            BEGIN
                SELECT RAISE(ABORT, 'fixture rejects delivery');
            END;
            "#,
        )
        .execute(&database.0)
        .await
        .unwrap();
        let repository = SqliteDeliveryRepository::new(database.0.clone());
        assert!(repository
            .ingest(&input("rollback", "不会残留", "2026-08-15T08:00:00.000Z"))
            .await
            .is_err());
        let document_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM reader_documents")
            .fetch_one(&database.0)
            .await
            .unwrap();
        assert_eq!(document_count, 0);
    }

    #[tokio::test]
    async fn schema_rejects_an_orphan_delivery() {
        let temp = tempfile::tempdir().unwrap();
        let database = sqlite::connect(&temp.path().join("foreign-key.sqlite3"))
            .await
            .unwrap();
        let result = sqlx::query(
            r#"
            INSERT INTO deliveries (id, document_id, idempotency_key, delivered_at)
            VALUES ('delivery_orphan', 'missing', 'orphan', '2026-08-15T08:00:00.000Z')
            "#,
        )
        .execute(&database.0)
        .await;
        assert!(result.is_err());
    }
}
