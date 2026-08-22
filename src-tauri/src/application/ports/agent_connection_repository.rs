use async_trait::async_trait;
use thiserror::Error;

use crate::domain::agent_connection::{AgentConnectionStatus, StoredAgentConnection};

#[derive(Debug, Error)]
pub enum AgentConnectionRepositoryError {
    #[error("Agent connection storage failed: {0}")]
    Storage(String),
}

impl From<sqlx::Error> for AgentConnectionRepositoryError {
    fn from(error: sqlx::Error) -> Self {
        Self::Storage(error.to_string())
    }
}

#[async_trait]
pub trait AgentConnectionRepository: Send + Sync {
    async fn get(&self) -> Result<Option<StoredAgentConnection>, AgentConnectionRepositoryError>;
    async fn ensure(
        &self,
        status: AgentConnectionStatus,
    ) -> Result<StoredAgentConnection, AgentConnectionRepositoryError>;
    async fn rotate_token(
        &self,
        token_hash: &str,
    ) -> Result<StoredAgentConnection, AgentConnectionRepositoryError>;
    async fn set_status(
        &self,
        status: AgentConnectionStatus,
    ) -> Result<StoredAgentConnection, AgentConnectionRepositoryError>;
    async fn touch_last_used(&self) -> Result<(), AgentConnectionRepositoryError>;
}
