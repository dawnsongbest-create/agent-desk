use std::sync::Arc;

use thiserror::Error;

use crate::{
    application::ports::delivery_repository::{DeliveryRepository, DeliveryRepositoryError},
    domain::delivery::{
        validate_delivery, CreateDeliveryInput, DeliveryValidationError, InboxDelivery,
        IngestDeliveryResult,
    },
};

#[derive(Debug, Error)]
pub enum DeliveryServiceError {
    #[error(transparent)]
    Validation(#[from] DeliveryValidationError),
    #[error(transparent)]
    Repository(#[from] DeliveryRepositoryError),
}

#[derive(Clone)]
pub struct DeliveryService {
    repository: Arc<dyn DeliveryRepository>,
}

impl DeliveryService {
    pub fn new(repository: Arc<dyn DeliveryRepository>) -> Self {
        Self { repository }
    }

    pub async fn ingest(
        &self,
        input: CreateDeliveryInput,
    ) -> Result<IngestDeliveryResult, DeliveryServiceError> {
        Ok(self.repository.ingest(&validate_delivery(input)?).await?)
    }

    pub async fn list_inbox(&self) -> Result<Vec<InboxDelivery>, DeliveryServiceError> {
        Ok(self.repository.list_inbox().await?)
    }

    pub async fn get_unread_count(&self) -> Result<i64, DeliveryServiceError> {
        Ok(self.repository.get_unread_count().await?)
    }

    pub async fn open(&self, id: &str) -> Result<InboxDelivery, DeliveryServiceError> {
        Ok(self.repository.mark_opened(id).await?)
    }
}
