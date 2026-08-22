use std::sync::Arc;

use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    application::ports::agent_connection_repository::{
        AgentConnectionRepository, AgentConnectionRepositoryError,
    },
    domain::agent_connection::{AgentConnection, AgentConnectionStatus, StoredAgentConnection},
};

#[derive(Debug, Error)]
pub enum AgentConnectionServiceError {
    #[error(transparent)]
    Repository(#[from] AgentConnectionRepositoryError),
    #[error("Agent token is not configured.")]
    TokenMissing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssuedAgentToken {
    pub token: String,
    pub connection: AgentConnection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedAgentConnection {
    pub id: String,
    pub name: String,
}

#[derive(Clone)]
pub struct AgentConnectionService {
    repository: Arc<dyn AgentConnectionRepository>,
}

impl AgentConnectionService {
    pub fn new(repository: Arc<dyn AgentConnectionRepository>) -> Self {
        Self { repository }
    }

    pub async fn get(&self) -> Result<Option<StoredAgentConnection>, AgentConnectionServiceError> {
        Ok(self.repository.get().await?)
    }

    pub async fn ensure(
        &self,
        status: AgentConnectionStatus,
    ) -> Result<StoredAgentConnection, AgentConnectionServiceError> {
        Ok(self.repository.ensure(status).await?)
    }

    pub async fn set_status(
        &self,
        status: AgentConnectionStatus,
    ) -> Result<StoredAgentConnection, AgentConnectionServiceError> {
        Ok(self.repository.set_status(status).await?)
    }

    pub async fn generate_token(&self) -> Result<IssuedAgentToken, AgentConnectionServiceError> {
        let token = format!("adk_{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
        let stored = self.repository.rotate_token(&hash_token(&token)).await?;
        Ok(IssuedAgentToken {
            token,
            connection: AgentConnection::from(&stored),
        })
    }

    pub async fn authenticate_identity(
        &self,
        token: &str,
    ) -> Result<Option<AuthenticatedAgentConnection>, AgentConnectionServiceError> {
        let Some(connection) = self.repository.get().await? else {
            return Ok(None);
        };
        if connection.status != AgentConnectionStatus::Active {
            return Ok(None);
        }
        let Some(expected_hash) = connection.token_hash else {
            return Ok(None);
        };
        let supplied_hash = hash_token(token);
        if !constant_time_equal(expected_hash.as_bytes(), supplied_hash.as_bytes()) {
            return Ok(None);
        }
        self.repository.touch_last_used().await?;
        Ok(Some(AuthenticatedAgentConnection {
            id: connection.id,
            name: connection.name,
        }))
    }

    pub async fn authenticate(&self, token: &str) -> Result<bool, AgentConnectionServiceError> {
        Ok(self.authenticate_identity(token).await?.is_some())
    }
}

fn hash_token(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn constant_time_equal(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::persistence::{
        agent_connection_repository::SqliteAgentConnectionRepository, sqlite,
    };

    #[tokio::test]
    async fn token_is_random_hashed_and_persists_without_plaintext() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("agent-connection.sqlite3");
        let database = sqlite::connect(&path).await.unwrap();
        let service = AgentConnectionService::new(Arc::new(SqliteAgentConnectionRepository::new(
            database.0.clone(),
        )));

        service.ensure(AgentConnectionStatus::Active).await.unwrap();
        let first = service.generate_token().await.unwrap();
        let stored = service.get().await.unwrap().unwrap();
        assert!(first.token.starts_with("adk_"));
        assert_eq!(first.token.len(), 68);
        assert_eq!(stored.token_hash.as_deref().unwrap().len(), 64);
        assert_ne!(stored.token_hash.as_deref(), Some(first.token.as_str()));
        assert!(service.authenticate(&first.token).await.unwrap());
        assert!(!service.authenticate("invalid-token").await.unwrap());
        let second = service.generate_token().await.unwrap();
        assert_ne!(first.token, second.token);
        assert!(!service.authenticate(&first.token).await.unwrap());
        assert!(service.authenticate(&second.token).await.unwrap());
        database.0.close().await;

        let reopened = sqlite::connect(&path).await.unwrap();
        let recovered =
            AgentConnectionService::new(Arc::new(SqliteAgentConnectionRepository::new(reopened.0)));
        assert!(recovered.authenticate(&second.token).await.unwrap());
        assert!(recovered
            .get()
            .await
            .unwrap()
            .unwrap()
            .last_used_at
            .is_some());
    }
}
