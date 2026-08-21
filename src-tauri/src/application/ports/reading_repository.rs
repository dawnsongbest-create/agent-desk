use async_trait::async_trait;
use thiserror::Error;

use crate::domain::reading::{
    CreateReadingPlanInput, ReadingPlan, ReadingPlanDelivery, ReadingPlanStatus, ReadingSession,
    ReadingSessionResult,
};

#[derive(Debug, Error)]
pub enum ReadingRepositoryError {
    #[error("Reading plan or source document was not found.")]
    NotFound,
    #[error("Reading plan progress changed while creating today's delivery.")]
    ProgressConflict,
    #[error("Reading storage failed: {0}")]
    Storage(String),
}

impl From<sqlx::Error> for ReadingRepositoryError {
    fn from(error: sqlx::Error) -> Self {
        Self::Storage(error.to_string())
    }
}

#[async_trait]
pub trait ReadingRepository: Send + Sync {
    async fn create_plan(
        &self,
        input: &CreateReadingPlanInput,
    ) -> Result<ReadingPlan, ReadingRepositoryError>;
    async fn list_plans(&self) -> Result<Vec<ReadingPlan>, ReadingRepositoryError>;
    async fn get_plan(&self, id: &str) -> Result<Option<ReadingPlan>, ReadingRepositoryError>;
    async fn set_plan_status(
        &self,
        id: &str,
        status: ReadingPlanStatus,
    ) -> Result<ReadingPlan, ReadingRepositoryError>;
    #[allow(clippy::too_many_arguments)]
    async fn record_generation(
        &self,
        plan_id: &str,
        day: i64,
        delivery_id: &str,
        document_id: &str,
        content_start: i64,
        content_end: i64,
        estimated_minutes: i64,
    ) -> Result<(ReadingPlan, ReadingPlanDelivery), ReadingRepositoryError>;
    async fn create_session(
        &self,
        source_document_id: &str,
        content: &str,
        estimated_minutes: i64,
    ) -> Result<ReadingSessionResult, ReadingRepositoryError>;
    async fn get_session(&self, id: &str)
        -> Result<Option<ReadingSession>, ReadingRepositoryError>;
}
