use async_trait::async_trait;

use crate::domain::card::{CardRecord, NewBaseCard};

#[async_trait]
pub trait CardRepository: Send + Sync {
    async fn create_base(&self, card: &NewBaseCard) -> Result<CardRecord, sqlx::Error>;
    async fn get_by_id(&self, id: &str) -> Result<Option<CardRecord>, sqlx::Error>;
}
