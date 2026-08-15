use async_trait::async_trait;
use sqlx::{Sqlite, SqlitePool, Transaction};

use crate::{
    application::ports::reader_document_repository::{
        ReaderDocumentRepository, ReaderDocumentRepositoryError,
    },
    domain::{
        reader::{
            NewReaderDocument, ReaderDocument, ReaderDocumentType, ReaderSourceType,
            RecordSourceRef, SelectionCaptureResult,
        },
        sticky::{StickyCard, StickyCardKind},
    },
};

#[derive(Debug, sqlx::FromRow)]
struct ReaderDocumentRow {
    id: String,
    document_type: String,
    title: String,
    subtitle: Option<String>,
    content_markdown: String,
    source_type: String,
    source_label: Option<String>,
    created_at: String,
    updated_at: String,
}

impl TryFrom<ReaderDocumentRow> for ReaderDocument {
    type Error = ReaderDocumentRepositoryError;

    fn try_from(row: ReaderDocumentRow) -> Result<Self, Self::Error> {
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
            id: row.id,
            document_type,
            title: row.title,
            subtitle: row.subtitle,
            content_markdown: row.content_markdown,
            source_type,
            source_label: row.source_label,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

#[derive(Clone)]
pub struct SqliteReaderDocumentRepository {
    pool: SqlitePool,
}

impl SqliteReaderDocumentRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    async fn timestamp(
        transaction: &mut Transaction<'_, Sqlite>,
    ) -> Result<String, ReaderDocumentRepositoryError> {
        Ok(
            sqlx::query_scalar("SELECT strftime('%Y-%m-%dT%H:%M:%fZ', 'now')")
                .fetch_one(&mut **transaction)
                .await?,
        )
    }

    async fn fetch_in_transaction(
        transaction: &mut Transaction<'_, Sqlite>,
        id: &str,
    ) -> Result<ReaderDocument, ReaderDocumentRepositoryError> {
        let row =
            sqlx::query_as::<_, ReaderDocumentRow>("SELECT * FROM reader_documents WHERE id = ?")
                .bind(id)
                .fetch_optional(&mut **transaction)
                .await?
                .ok_or(ReaderDocumentRepositoryError::NotFound)?;
        row.try_into()
    }
}

#[async_trait]
impl ReaderDocumentRepository for SqliteReaderDocumentRepository {
    async fn create(
        &self,
        document: &NewReaderDocument,
    ) -> Result<ReaderDocument, ReaderDocumentRepositoryError> {
        let mut transaction = self.pool.begin().await?;
        let id = match &document.id {
            Some(id) => id.clone(),
            None => {
                sqlx::query_scalar("SELECT 'reader_' || lower(hex(randomblob(16)))")
                    .fetch_one(&mut *transaction)
                    .await?
            }
        };
        let timestamp = Self::timestamp(&mut transaction).await?;
        sqlx::query(
            r#"
            INSERT INTO reader_documents (
                id, document_type, title, subtitle, content_markdown,
                source_type, source_label, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(document.document_type.as_str())
        .bind(&document.title)
        .bind(&document.subtitle)
        .bind(&document.content_markdown)
        .bind(document.source_type.as_str())
        .bind(&document.source_label)
        .bind(&timestamp)
        .bind(&timestamp)
        .execute(&mut *transaction)
        .await?;
        let created = Self::fetch_in_transaction(&mut transaction, &id).await?;
        transaction.commit().await?;
        Ok(created)
    }

    async fn get_by_id(
        &self,
        id: &str,
    ) -> Result<Option<ReaderDocument>, ReaderDocumentRepositoryError> {
        sqlx::query_as::<_, ReaderDocumentRow>("SELECT * FROM reader_documents WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .map(TryInto::try_into)
            .transpose()
    }

    async fn list(&self) -> Result<Vec<ReaderDocument>, ReaderDocumentRepositoryError> {
        let rows = sqlx::query_as::<_, ReaderDocumentRow>(
            "SELECT * FROM reader_documents ORDER BY updated_at DESC, created_at DESC, id ASC",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    async fn capture_selection(
        &self,
        document_id: &str,
        selected_text: &str,
    ) -> Result<SelectionCaptureResult, ReaderDocumentRepositoryError> {
        let mut transaction = self.pool.begin().await?;
        let document_title: String =
            sqlx::query_scalar("SELECT title FROM reader_documents WHERE id = ?")
                .bind(document_id)
                .fetch_optional(&mut *transaction)
                .await?
                .ok_or(ReaderDocumentRepositoryError::NotFound)?;
        let record_id: String = sqlx::query_scalar("SELECT 'card_' || lower(hex(randomblob(16)))")
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
            ) VALUES (?, 'note', NULL, 'active', NULL, 'user', NULL, NULL, '{}', ?, ?)
            "#,
        )
        .bind(&record_id)
        .bind(&timestamp)
        .bind(&timestamp)
        .execute(&mut *transaction)
        .await?;
        sqlx::query("INSERT INTO note_payloads (card_id, body) VALUES (?, ?)")
            .bind(&record_id)
            .bind(selected_text)
            .execute(&mut *transaction)
            .await?;
        sqlx::query(
            r#"
            INSERT INTO card_placements (card_id, surface, position, created_at, updated_at)
            VALUES (?, 'sticky', ?, ?, ?)
            "#,
        )
        .bind(&record_id)
        .bind(position)
        .bind(&timestamp)
        .bind(&timestamp)
        .execute(&mut *transaction)
        .await?;
        sqlx::query(
            r#"
            INSERT INTO record_source_refs (
                record_id, document_id, source_type, selected_text,
                document_title_snapshot, captured_at
            ) VALUES (?, ?, 'reader_selection', ?, ?, ?)
            "#,
        )
        .bind(&record_id)
        .bind(document_id)
        .bind(selected_text)
        .bind(&document_title)
        .bind(&timestamp)
        .execute(&mut *transaction)
        .await?;

        let source_ref = sqlx::query_as::<_, RecordSourceRef>(
            "SELECT * FROM record_source_refs WHERE record_id = ?",
        )
        .bind(&record_id)
        .fetch_one(&mut *transaction)
        .await?;
        let record = StickyCard {
            id: record_id,
            kind: StickyCardKind::Note,
            text: selected_text.to_owned(),
            completed: false,
            due_date: None,
            position,
            created_at: timestamp.clone(),
            updated_at: timestamp,
        };
        transaction.commit().await?;
        Ok(SelectionCaptureResult { record, source_ref })
    }

    async fn get_record_source_ref(
        &self,
        record_id: &str,
    ) -> Result<Option<RecordSourceRef>, ReaderDocumentRepositoryError> {
        Ok(sqlx::query_as::<_, RecordSourceRef>(
            "SELECT * FROM record_source_refs WHERE record_id = ?",
        )
        .bind(record_id)
        .fetch_optional(&self.pool)
        .await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::ports::sticky_repository::StickyRepository,
        domain::reader::{validate_document, NewReaderDocument},
        persistence::{sqlite, sticky_repository::SqliteStickyRepository},
    };

    fn sample_document() -> NewReaderDocument {
        validate_document(NewReaderDocument {
            id: None,
            document_type: ReaderDocumentType::Article,
            title: "中英代码文档".to_owned(),
            subtitle: Some("Reader persistence".to_owned()),
            content_markdown: "中文段落\n\nEnglish text\n\n```ts\nconst exact = true;\n```"
                .to_owned(),
            source_type: ReaderSourceType::Local,
            source_label: Some("本地测试".to_owned()),
        })
        .unwrap()
    }

    #[tokio::test]
    async fn creates_lists_and_reopens_exact_markdown() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("reader.sqlite3");
        let database = sqlite::connect(&path).await.unwrap();
        let repository = SqliteReaderDocumentRepository::new(database.0.clone());
        let created = repository.create(&sample_document()).await.unwrap();
        assert_eq!(
            repository.get_by_id(&created.id).await.unwrap(),
            Some(created.clone())
        );
        assert_eq!(repository.list().await.unwrap(), vec![created.clone()]);
        database.0.close().await;

        let reopened = sqlite::connect(&path).await.unwrap();
        let repository = SqliteReaderDocumentRepository::new(reopened.0);
        assert_eq!(
            repository
                .get_by_id(&created.id)
                .await
                .unwrap()
                .unwrap()
                .content_markdown,
            created.content_markdown
        );
    }

    #[tokio::test]
    async fn captures_exact_selection_and_source_in_one_transaction() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("capture.sqlite3");
        let database = sqlite::connect(&path).await.unwrap();
        let repository = SqliteReaderDocumentRepository::new(database.0.clone());
        let document = repository.create(&sample_document()).await.unwrap();
        let selections = [
            "一句中文".to_owned(),
            "第一段中文\n\n第二段中文".to_owned(),
            "English selection".to_owned(),
            "const exact = true;".to_owned(),
            "长摘录内容".repeat(110),
        ];
        let mut record_ids = Vec::new();
        for selection in selections {
            let captured = repository
                .capture_selection(&document.id, &selection)
                .await
                .unwrap();
            assert_eq!(captured.record.text, selection);
            assert_eq!(captured.source_ref.selected_text, selection);
            assert_eq!(captured.source_ref.document_id, document.id);
            assert_eq!(captured.source_ref.document_title_snapshot, document.title);
            record_ids.push(captured.record.id.clone());
            assert_eq!(
                repository
                    .get_record_source_ref(&captured.record.id)
                    .await
                    .unwrap(),
                Some(captured.source_ref)
            );
        }
        drop(repository);
        database.0.close().await;

        let reopened = sqlite::connect(&path).await.unwrap();
        let repository = SqliteReaderDocumentRepository::new(reopened.0.clone());
        for record_id in record_ids {
            let source_ref = repository
                .get_record_source_ref(&record_id)
                .await
                .unwrap()
                .expect("source ref should survive restart");
            assert_eq!(source_ref.document_id, document.id);
        }
        let sticky = SqliteStickyRepository::new(reopened.0);
        assert_eq!(sticky.list().await.unwrap().len(), 5);
    }

    #[tokio::test]
    async fn missing_document_rolls_back_the_record() {
        let temp = tempfile::tempdir().unwrap();
        let database = sqlite::connect(&temp.path().join("rollback.sqlite3"))
            .await
            .unwrap();
        let repository = SqliteReaderDocumentRepository::new(database.0.clone());
        assert!(repository
            .capture_selection("missing", "原文")
            .await
            .is_err());
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM cards")
            .fetch_one(&database.0)
            .await
            .unwrap();
        assert_eq!(count, 0);
    }
}
