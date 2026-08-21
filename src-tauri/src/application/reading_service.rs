use std::sync::Arc;

use thiserror::Error;

use crate::{
    application::{
        delivery_service::{DeliveryService, DeliveryServiceError},
        ports::{
            delivery_repository::DeliveryRepository,
            reading_repository::{ReadingRepository, ReadingRepositoryError},
        },
    },
    domain::{
        delivery::CreateDeliveryInput,
        reader::{ReaderDocumentType, ReaderSourceType},
        reading::{
            estimate_reading_minutes, infer_difficulty, next_reading_section, validate_plan,
            validate_session_content, CreateReadingPlanInput, GenerateReadingDeliveryResult,
            ReadingPlan, ReadingPlanStatus, ReadingSessionResult, ReadingValidationError,
        },
    },
};

#[derive(Debug, Error)]
pub enum ReadingServiceError {
    #[error(transparent)]
    Validation(#[from] ReadingValidationError),
    #[error(transparent)]
    Repository(#[from] ReadingRepositoryError),
    #[error(transparent)]
    Delivery(#[from] DeliveryServiceError),
    #[error("Reading plan must be active before generating a delivery.")]
    PlanNotActive,
}

#[derive(Clone)]
pub struct ReadingService {
    repository: Arc<dyn ReadingRepository>,
    deliveries: DeliveryService,
}

impl ReadingService {
    pub fn new(
        repository: Arc<dyn ReadingRepository>,
        delivery_repository: Arc<dyn DeliveryRepository>,
    ) -> Self {
        Self {
            repository,
            deliveries: DeliveryService::new(delivery_repository),
        }
    }

    pub async fn create_plan(
        &self,
        input: CreateReadingPlanInput,
    ) -> Result<ReadingPlan, ReadingServiceError> {
        Ok(self.repository.create_plan(&validate_plan(input)?).await?)
    }

    pub async fn list_plans(&self) -> Result<Vec<ReadingPlan>, ReadingServiceError> {
        Ok(self.repository.list_plans().await?)
    }

    pub async fn set_plan_status(
        &self,
        id: &str,
        status: ReadingPlanStatus,
    ) -> Result<ReadingPlan, ReadingServiceError> {
        Ok(self.repository.set_plan_status(id, status).await?)
    }

    pub async fn generate_today(
        &self,
        plan_id: &str,
    ) -> Result<GenerateReadingDeliveryResult, ReadingServiceError> {
        let plan = self
            .repository
            .get_plan(plan_id)
            .await?
            .ok_or(ReadingRepositoryError::NotFound)?;
        if plan.status != ReadingPlanStatus::Active {
            return Err(ReadingServiceError::PlanNotActive);
        }
        let day = plan.current_day + 1;
        let section = next_reading_section(&plan)?;
        let subtitle = format!(
            "今日阅读 · Day {day} · 预计 {} 分钟",
            section.estimated_minutes
        );
        let delivery = self
            .deliveries
            .ingest(CreateDeliveryInput {
                idempotency_key: format!("reading-plan:{}:day:{day}", plan.id),
                document_type: ReaderDocumentType::Reading,
                title: plan.title.clone(),
                subtitle: Some(subtitle),
                content_markdown: section.content,
                source_type: ReaderSourceType::Agent,
                source_label: plan
                    .source_name
                    .clone()
                    .or_else(|| Some("Reading Agent".to_owned())),
                delivered_at: None,
            })
            .await?;
        let (plan, generation) = self
            .repository
            .record_generation(
                &plan.id,
                day,
                &delivery.item.delivery.id,
                &delivery.item.document.id,
                section.start,
                section.end,
                section.estimated_minutes,
            )
            .await?;
        Ok(GenerateReadingDeliveryResult {
            plan,
            delivery,
            generation,
        })
    }

    pub async fn create_session(
        &self,
        source_document_id: &str,
        content: String,
    ) -> Result<ReadingSessionResult, ReadingServiceError> {
        let (content, estimated_minutes) = Self::prepare_session(content, None)?;
        Ok(self
            .repository
            .create_session(source_document_id, &content, estimated_minutes)
            .await?)
    }

    pub fn prepare_session(
        content: String,
        requested_minutes: Option<i64>,
    ) -> Result<(String, i64), ReadingServiceError> {
        let content = validate_session_content(content)?;
        let estimated_minutes = requested_minutes.unwrap_or_else(|| {
            let difficulty = infer_difficulty(&content);
            estimate_reading_minutes(&content, difficulty)
        });
        if !(1..=240).contains(&estimated_minutes) {
            return Err(ReadingValidationError::InvalidDailyMinutes.into());
        }
        Ok((content, estimated_minutes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::ports::{
            delivery_repository::DeliveryRepository, reading_repository::ReadingRepository,
        },
        domain::reading::ReadingDifficulty,
        persistence::{
            delivery_repository::SqliteDeliveryRepository,
            reading_repository::SqliteReadingRepository, sqlite,
        },
    };

    #[tokio::test]
    async fn plan_generates_through_delivery_service_and_survives_restart() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("reading.sqlite3");
        let database = sqlite::connect(&path).await.unwrap();
        let reading_repository = SqliteReadingRepository::new(database.0.clone());
        let delivery_repository = SqliteDeliveryRepository::new(database.0.clone());
        let service = ReadingService::new(
            Arc::new(reading_repository.clone()),
            Arc::new(delivery_repository.clone()),
        );
        let plan = service
            .create_plan(CreateReadingPlanInput {
                title: "Rust 阅读".to_owned(),
                source_name: Some("本地 Markdown".to_owned()),
                content_markdown: format!(
                    "# 第一章\n\n{}\n\n# 第二章\n\n{}",
                    "甲".repeat(900),
                    "乙".repeat(900)
                ),
                daily_minutes: 2,
                schedule_time: "08:00".to_owned(),
                difficulty: ReadingDifficulty::Technical,
            })
            .await
            .unwrap();
        let generated = service.generate_today(&plan.id).await.unwrap();
        assert!(generated.delivery.created);
        assert_eq!(generated.generation.day, 1);
        assert_eq!(
            generated.delivery.item.document.document_type,
            ReaderDocumentType::Reading
        );
        assert!(generated
            .delivery
            .item
            .document
            .subtitle
            .as_deref()
            .unwrap()
            .contains("预计"));
        assert_eq!(delivery_repository.get_unread_count().await.unwrap(), 1);
        database.0.close().await;

        let reopened = sqlite::connect(&path).await.unwrap();
        let plans = SqliteReadingRepository::new(reopened.0.clone())
            .list_plans()
            .await
            .unwrap();
        let inbox = SqliteDeliveryRepository::new(reopened.0)
            .list_inbox()
            .await
            .unwrap();
        assert_eq!(plans[0].current_day, 1);
        assert_eq!(inbox.len(), 1);
    }
}
