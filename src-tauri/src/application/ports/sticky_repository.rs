use async_trait::async_trait;
use thiserror::Error;

use crate::domain::sticky::{NewStickyCard, StickyCard, StickyProfile};

#[derive(Debug, Error)]
pub enum StickyRepositoryError {
    #[error("Sticky card was not found.")]
    NotFound,
    #[error("The requested operation is not valid for this card.")]
    InvalidOperation,
    #[error("The requested order does not match the current Sticky surface.")]
    InvalidOrder,
    #[error("Sticky storage failed: {0}")]
    Storage(String),
}

impl From<sqlx::Error> for StickyRepositoryError {
    fn from(error: sqlx::Error) -> Self {
        Self::Storage(error.to_string())
    }
}

#[async_trait]
pub trait StickyRepository: Send + Sync {
    async fn list(&self) -> Result<Vec<StickyCard>, StickyRepositoryError>;
    async fn create(&self, card: &NewStickyCard) -> Result<StickyCard, StickyRepositoryError>;
    async fn update_text(&self, id: &str, text: &str) -> Result<StickyCard, StickyRepositoryError>;
    async fn set_task_completed(
        &self,
        id: &str,
        completed: bool,
    ) -> Result<StickyCard, StickyRepositoryError>;
    async fn set_task_due_date(
        &self,
        id: &str,
        due_date: Option<&str>,
    ) -> Result<StickyCard, StickyRepositoryError>;
    async fn delete(&self, id: &str) -> Result<(), StickyRepositoryError>;
    async fn reorder(
        &self,
        ordered_ids: &[String],
    ) -> Result<Vec<StickyCard>, StickyRepositoryError>;
    async fn get_profile(&self) -> Result<StickyProfile, StickyRepositoryError>;
    async fn update_quote(&self, quote_text: &str) -> Result<StickyProfile, StickyRepositoryError>;
    async fn get_record_text(&self, id: &str) -> Result<String, StickyRepositoryError>;
}
