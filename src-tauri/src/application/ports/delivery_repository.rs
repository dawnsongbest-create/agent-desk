use async_trait::async_trait;
use thiserror::Error;

use crate::domain::delivery::{CreateDeliveryInput, InboxDelivery, IngestDeliveryResult};

#[derive(Debug, Error)]
pub enum DeliveryRepositoryError {
    #[error("Delivery was not found.")]
    NotFound,
    #[error("IDEMPOTENCY_CONFLICT")]
    IdempotencyConflict,
    #[error("Delivery storage failed: {0}")]
    Storage(String),
}

impl From<sqlx::Error> for DeliveryRepositoryError {
    fn from(error: sqlx::Error) -> Self {
        Self::Storage(error.to_string())
    }
}

#[async_trait]
pub trait DeliveryRepository: Send + Sync {
    async fn ingest(
        &self,
        input: &CreateDeliveryInput,
    ) -> Result<IngestDeliveryResult, DeliveryRepositoryError>;
    async fn list_inbox(&self) -> Result<Vec<InboxDelivery>, DeliveryRepositoryError>;
    async fn get(&self, id: &str) -> Result<Option<InboxDelivery>, DeliveryRepositoryError>;
    async fn get_unread_count(&self) -> Result<i64, DeliveryRepositoryError>;
    async fn mark_opened(&self, id: &str) -> Result<InboxDelivery, DeliveryRepositoryError>;
}
