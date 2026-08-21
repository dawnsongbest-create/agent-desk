use async_trait::async_trait;
use sqlx::{Sqlite, SqlitePool, Transaction};

use crate::{
    application::ports::reading_repository::{ReadingRepository, ReadingRepositoryError},
    domain::{
        reader::{ReaderDocument, ReaderDocumentType, ReaderSourceType},
        reading::{
            CreateReadingPlanInput, ReadingDifficulty, ReadingPlan, ReadingPlanDelivery,
            ReadingPlanStatus, ReadingSession, ReadingSessionResult,
        },
    },
};

#[derive(Debug, sqlx::FromRow)]
struct ReadingPlanRow {
    id: String,
    title: String,
    source_name: Option<String>,
    content_markdown: String,
    total_content_length: i64,
    daily_minutes: i64,
    schedule_time: String,
    difficulty: String,
    status: String,
    current_offset: i64,
    current_day: i64,
    created_at: String,
    updated_at: String,
}

impl TryFrom<ReadingPlanRow> for ReadingPlan {
    type Error = ReadingRepositoryError;

    fn try_from(row: ReadingPlanRow) -> Result<Self, Self::Error> {
        let difficulty = match row.difficulty.as_str() {
            "normal" => ReadingDifficulty::Normal,
            "technical" => ReadingDifficulty::Technical,
            _ => {
                return Err(Self::Error::Storage(
                    "unexpected reading difficulty".to_owned(),
                ))
            }
        };
        let status = match row.status.as_str() {
            "active" => ReadingPlanStatus::Active,
            "paused" => ReadingPlanStatus::Paused,
            "completed" => ReadingPlanStatus::Completed,
            _ => {
                return Err(Self::Error::Storage(
                    "unexpected reading plan status".to_owned(),
                ))
            }
        };
        Ok(Self {
            id: row.id,
            title: row.title,
            source_name: row.source_name,
            content_markdown: row.content_markdown,
            total_content_length: row.total_content_length,
            daily_minutes: row.daily_minutes,
            schedule_time: row.schedule_time,
            difficulty,
            status,
            current_offset: row.current_offset,
            current_day: row.current_day,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

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
    type Error = ReadingRepositoryError;

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
pub struct SqliteReadingRepository {
    pool: SqlitePool,
}

impl SqliteReadingRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    async fn timestamp(
        transaction: &mut Transaction<'_, Sqlite>,
    ) -> Result<String, ReadingRepositoryError> {
        Ok(
            sqlx::query_scalar("SELECT strftime('%Y-%m-%dT%H:%M:%fZ', 'now')")
                .fetch_one(&mut **transaction)
                .await?,
        )
    }

    async fn fetch_plan(
        transaction: &mut Transaction<'_, Sqlite>,
        id: &str,
    ) -> Result<ReadingPlan, ReadingRepositoryError> {
        sqlx::query_as::<_, ReadingPlanRow>("SELECT * FROM reading_plans WHERE id = ?")
            .bind(id)
            .fetch_optional(&mut **transaction)
            .await?
            .ok_or(ReadingRepositoryError::NotFound)?
            .try_into()
    }

    pub(crate) async fn create_session_in_transaction(
        transaction: &mut Transaction<'_, Sqlite>,
        source_document_id: &str,
        content: &str,
        estimated_minutes: i64,
    ) -> Result<ReadingSessionResult, ReadingRepositoryError> {
        let source_title: String =
            sqlx::query_scalar("SELECT title FROM reader_documents WHERE id = ?")
                .bind(source_document_id)
                .fetch_optional(&mut **transaction)
                .await?
                .ok_or(ReadingRepositoryError::NotFound)?;
        let session_id: String =
            sqlx::query_scalar("SELECT 'reading_session_' || lower(hex(randomblob(16)))")
                .fetch_one(&mut **transaction)
                .await?;
        let document_id: String =
            sqlx::query_scalar("SELECT 'reader_' || lower(hex(randomblob(16)))")
                .fetch_one(&mut **transaction)
                .await?;
        let timestamp = Self::timestamp(transaction).await?;
        let title = format!("今日阅读 · {source_title}");
        let subtitle = format!("今日阅读 · 预计 {estimated_minutes} 分钟");
        sqlx::query(
            r#"
            INSERT INTO reader_documents (
                id, document_type, title, subtitle, content_markdown,
                source_type, source_label, created_at, updated_at
            ) VALUES (?, 'reading', ?, ?, ?, 'local', ?, ?, ?)
            "#,
        )
        .bind(&document_id)
        .bind(&title)
        .bind(&subtitle)
        .bind(content)
        .bind(format!("选自《{source_title}》"))
        .bind(&timestamp)
        .bind(&timestamp)
        .execute(&mut **transaction)
        .await?;
        sqlx::query(
            r#"
            INSERT INTO reading_sessions (
                id, source_document_id, reader_document_id,
                content, estimated_minutes, created_at
            ) VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&session_id)
        .bind(source_document_id)
        .bind(&document_id)
        .bind(content)
        .bind(estimated_minutes)
        .bind(&timestamp)
        .execute(&mut **transaction)
        .await?;
        let session =
            sqlx::query_as::<_, ReadingSession>("SELECT * FROM reading_sessions WHERE id = ?")
                .bind(&session_id)
                .fetch_one(&mut **transaction)
                .await?;
        let document =
            sqlx::query_as::<_, ReaderDocumentRow>("SELECT * FROM reader_documents WHERE id = ?")
                .bind(&document_id)
                .fetch_one(&mut **transaction)
                .await?
                .try_into()?;
        Ok(ReadingSessionResult { session, document })
    }
}

#[async_trait]
impl ReadingRepository for SqliteReadingRepository {
    async fn create_plan(
        &self,
        input: &CreateReadingPlanInput,
    ) -> Result<ReadingPlan, ReadingRepositoryError> {
        let mut transaction = self.pool.begin().await?;
        let id: String = sqlx::query_scalar("SELECT 'reading_plan_' || lower(hex(randomblob(16)))")
            .fetch_one(&mut *transaction)
            .await?;
        let timestamp = Self::timestamp(&mut transaction).await?;
        sqlx::query(
            r#"
            INSERT INTO reading_plans (
                id, title, source_name, content_markdown, total_content_length,
                daily_minutes, schedule_time, difficulty, status,
                current_offset, current_day, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'active', 0, 0, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(&input.title)
        .bind(&input.source_name)
        .bind(&input.content_markdown)
        .bind(input.content_markdown.chars().count() as i64)
        .bind(input.daily_minutes)
        .bind(&input.schedule_time)
        .bind(input.difficulty.as_str())
        .bind(&timestamp)
        .bind(&timestamp)
        .execute(&mut *transaction)
        .await?;
        let plan = Self::fetch_plan(&mut transaction, &id).await?;
        transaction.commit().await?;
        Ok(plan)
    }

    async fn list_plans(&self) -> Result<Vec<ReadingPlan>, ReadingRepositoryError> {
        sqlx::query_as::<_, ReadingPlanRow>(
            "SELECT * FROM reading_plans ORDER BY updated_at DESC, created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(TryInto::try_into)
        .collect()
    }

    async fn get_plan(&self, id: &str) -> Result<Option<ReadingPlan>, ReadingRepositoryError> {
        sqlx::query_as::<_, ReadingPlanRow>("SELECT * FROM reading_plans WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .map(TryInto::try_into)
            .transpose()
    }

    async fn set_plan_status(
        &self,
        id: &str,
        status: ReadingPlanStatus,
    ) -> Result<ReadingPlan, ReadingRepositoryError> {
        let mut transaction = self.pool.begin().await?;
        let timestamp = Self::timestamp(&mut transaction).await?;
        let updated =
            sqlx::query("UPDATE reading_plans SET status = ?, updated_at = ? WHERE id = ?")
                .bind(status.as_str())
                .bind(timestamp)
                .bind(id)
                .execute(&mut *transaction)
                .await?;
        if updated.rows_affected() != 1 {
            return Err(ReadingRepositoryError::NotFound);
        }
        let plan = Self::fetch_plan(&mut transaction, id).await?;
        transaction.commit().await?;
        Ok(plan)
    }

    async fn record_generation(
        &self,
        plan_id: &str,
        day: i64,
        delivery_id: &str,
        document_id: &str,
        content_start: i64,
        content_end: i64,
        estimated_minutes: i64,
    ) -> Result<(ReadingPlan, ReadingPlanDelivery), ReadingRepositoryError> {
        let mut transaction = self.pool.begin().await?;
        if let Some(existing) = sqlx::query_as::<_, ReadingPlanDelivery>(
            "SELECT * FROM reading_plan_deliveries WHERE plan_id = ? AND day = ?",
        )
        .bind(plan_id)
        .bind(day)
        .fetch_optional(&mut *transaction)
        .await?
        {
            let plan = Self::fetch_plan(&mut transaction, plan_id).await?;
            transaction.commit().await?;
            return Ok((plan, existing));
        }

        let timestamp = Self::timestamp(&mut transaction).await?;
        let advanced = sqlx::query(
            r#"
            UPDATE reading_plans
            SET current_offset = ?, current_day = ?, updated_at = ?
            WHERE id = ? AND status = 'active'
              AND current_offset = ? AND current_day = ?
            "#,
        )
        .bind(content_end)
        .bind(day)
        .bind(&timestamp)
        .bind(plan_id)
        .bind(content_start)
        .bind(day - 1)
        .execute(&mut *transaction)
        .await?;
        if advanced.rows_affected() != 1 {
            return Err(ReadingRepositoryError::ProgressConflict);
        }
        sqlx::query(
            r#"
            INSERT INTO reading_plan_deliveries (
                plan_id, day, delivery_id, document_id, content_start,
                content_end, estimated_minutes, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(plan_id)
        .bind(day)
        .bind(delivery_id)
        .bind(document_id)
        .bind(content_start)
        .bind(content_end)
        .bind(estimated_minutes)
        .bind(&timestamp)
        .execute(&mut *transaction)
        .await?;
        let plan = Self::fetch_plan(&mut transaction, plan_id).await?;
        let generation = sqlx::query_as::<_, ReadingPlanDelivery>(
            "SELECT * FROM reading_plan_deliveries WHERE plan_id = ? AND day = ?",
        )
        .bind(plan_id)
        .bind(day)
        .fetch_one(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok((plan, generation))
    }

    async fn create_session(
        &self,
        source_document_id: &str,
        content: &str,
        estimated_minutes: i64,
    ) -> Result<ReadingSessionResult, ReadingRepositoryError> {
        let mut transaction = self.pool.begin().await?;
        let result = Self::create_session_in_transaction(
            &mut transaction,
            source_document_id,
            content,
            estimated_minutes,
        )
        .await?;
        transaction.commit().await?;
        Ok(result)
    }

    async fn get_session(
        &self,
        id: &str,
    ) -> Result<Option<ReadingSession>, ReadingRepositoryError> {
        Ok(
            sqlx::query_as::<_, ReadingSession>("SELECT * FROM reading_sessions WHERE id = ?")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::ports::{
            delivery_repository::DeliveryRepository, reading_repository::ReadingRepository,
        },
        domain::{
            delivery::CreateDeliveryInput,
            reader::{ReaderDocumentType, ReaderSourceType},
            reading::{validate_plan, CreateReadingPlanInput},
        },
        persistence::{delivery_repository::SqliteDeliveryRepository, sqlite},
    };

    #[tokio::test]
    async fn creates_persists_and_updates_plan_status() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("plans.sqlite3");
        let database = sqlite::connect(&path).await.unwrap();
        let repository = SqliteReadingRepository::new(database.0.clone());
        let input = validate_plan(CreateReadingPlanInput {
            title: "长文计划".to_owned(),
            source_name: Some("本地 Markdown".to_owned()),
            content_markdown: "# 第一章\n\n保留完整正文".repeat(40),
            daily_minutes: 8,
            schedule_time: "07:45".to_owned(),
            difficulty: ReadingDifficulty::Normal,
        })
        .unwrap();
        let plan = repository.create_plan(&input).await.unwrap();
        let paused = repository
            .set_plan_status(&plan.id, ReadingPlanStatus::Paused)
            .await
            .unwrap();
        assert_eq!(paused.status, ReadingPlanStatus::Paused);
        database.0.close().await;
        let reopened = sqlite::connect(&path).await.unwrap();
        let plans = SqliteReadingRepository::new(reopened.0)
            .list_plans()
            .await
            .unwrap();
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].content_markdown, input.content_markdown);
        assert_eq!(plans[0].status, ReadingPlanStatus::Paused);
    }

    #[tokio::test]
    async fn selection_session_is_atomic_and_keeps_source_relationship() {
        let temp = tempfile::tempdir().unwrap();
        let database = sqlite::connect(&temp.path().join("session.sqlite3"))
            .await
            .unwrap();
        let deliveries = SqliteDeliveryRepository::new(database.0.clone());
        let source = deliveries
            .ingest(&CreateDeliveryInput {
                idempotency_key: "session-source".to_owned(),
                document_type: ReaderDocumentType::Article,
                title: "来源文章".to_owned(),
                subtitle: None,
                content_markdown: "来源正文".to_owned(),
                source_type: ReaderSourceType::Local,
                source_label: None,
                delivered_at: None,
            })
            .await
            .unwrap();
        let repository = SqliteReadingRepository::new(database.0.clone());
        let result = repository
            .create_session(&source.item.document.id, "精确选中文字", 1)
            .await
            .unwrap();
        assert_eq!(result.session.source_document_id, source.item.document.id);
        assert_eq!(result.session.content, "精确选中文字");
        assert_eq!(result.document.content_markdown, "精确选中文字");
        assert_eq!(
            repository
                .get_session(&result.session.id)
                .await
                .unwrap()
                .unwrap(),
            result.session
        );
    }
}
