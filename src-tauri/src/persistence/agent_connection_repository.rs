use async_trait::async_trait;
use sqlx::SqlitePool;

use crate::{
    application::ports::agent_connection_repository::{
        AgentConnectionRepository, AgentConnectionRepositoryError,
    },
    domain::agent_connection::{AgentConnectionStatus, StoredAgentConnection},
};

const LOCAL_CONNECTION_ID: &str = "local-agent-bridge";
const LOCAL_CONNECTION_NAME: &str = "Local Agent Bridge";

#[derive(Debug, sqlx::FromRow)]
struct AgentConnectionRow {
    id: String,
    name: String,
    token_hash: Option<String>,
    status: String,
    created_at: String,
    updated_at: String,
    last_used_at: Option<String>,
}

impl TryFrom<AgentConnectionRow> for StoredAgentConnection {
    type Error = AgentConnectionRepositoryError;

    fn try_from(row: AgentConnectionRow) -> Result<Self, Self::Error> {
        let status = match row.status.as_str() {
            "active" => AgentConnectionStatus::Active,
            "inactive" => AgentConnectionStatus::Inactive,
            _ => {
                return Err(Self::Error::Storage(
                    "unexpected agent connection status".to_owned(),
                ))
            }
        };
        Ok(Self {
            id: row.id,
            name: row.name,
            token_hash: row.token_hash,
            status,
            created_at: row.created_at,
            updated_at: row.updated_at,
            last_used_at: row.last_used_at,
        })
    }
}

#[derive(Clone)]
pub struct SqliteAgentConnectionRepository {
    pool: SqlitePool,
}

impl SqliteAgentConnectionRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    async fn fetch(&self) -> Result<Option<StoredAgentConnection>, AgentConnectionRepositoryError> {
        sqlx::query_as::<_, AgentConnectionRow>("SELECT * FROM agent_connections WHERE id = ?")
            .bind(LOCAL_CONNECTION_ID)
            .fetch_optional(&self.pool)
            .await?
            .map(TryInto::try_into)
            .transpose()
    }
}

#[async_trait]
impl AgentConnectionRepository for SqliteAgentConnectionRepository {
    async fn get(&self) -> Result<Option<StoredAgentConnection>, AgentConnectionRepositoryError> {
        self.fetch().await
    }

    async fn ensure(
        &self,
        status: AgentConnectionStatus,
    ) -> Result<StoredAgentConnection, AgentConnectionRepositoryError> {
        sqlx::query(
            r#"
            INSERT INTO agent_connections (
                id, name, token_hash, status, created_at, updated_at, last_used_at
            ) VALUES (
                ?, ?, NULL, ?,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), NULL
            )
            ON CONFLICT(id) DO UPDATE SET
                status = excluded.status,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(LOCAL_CONNECTION_ID)
        .bind(LOCAL_CONNECTION_NAME)
        .bind(status.as_str())
        .execute(&self.pool)
        .await?;
        self.fetch()
            .await?
            .ok_or_else(|| AgentConnectionRepositoryError::Storage("connection missing".to_owned()))
    }

    async fn rotate_token(
        &self,
        token_hash: &str,
    ) -> Result<StoredAgentConnection, AgentConnectionRepositoryError> {
        sqlx::query(
            r#"
            INSERT INTO agent_connections (
                id, name, token_hash, status, created_at, updated_at, last_used_at
            ) VALUES (
                ?, ?, ?, 'inactive',
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), NULL
            )
            ON CONFLICT(id) DO UPDATE SET
                token_hash = excluded.token_hash,
                updated_at = excluded.updated_at,
                last_used_at = NULL
            "#,
        )
        .bind(LOCAL_CONNECTION_ID)
        .bind(LOCAL_CONNECTION_NAME)
        .bind(token_hash)
        .execute(&self.pool)
        .await?;
        self.fetch()
            .await?
            .ok_or_else(|| AgentConnectionRepositoryError::Storage("connection missing".to_owned()))
    }

    async fn set_status(
        &self,
        status: AgentConnectionStatus,
    ) -> Result<StoredAgentConnection, AgentConnectionRepositoryError> {
        self.ensure(status).await
    }

    async fn touch_last_used(&self) -> Result<(), AgentConnectionRepositoryError> {
        sqlx::query(
            r#"
            UPDATE agent_connections
            SET last_used_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
            WHERE id = ?
            "#,
        )
        .bind(LOCAL_CONNECTION_ID)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::sqlite;

    #[tokio::test]
    async fn connection_status_and_hash_survive_restart() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("connections.sqlite3");
        let database = sqlite::connect(&path).await.unwrap();
        let repository = SqliteAgentConnectionRepository::new(database.0.clone());
        repository
            .ensure(AgentConnectionStatus::Active)
            .await
            .unwrap();
        repository.rotate_token(&"a".repeat(64)).await.unwrap();
        database.0.close().await;

        let reopened = sqlite::connect(&path).await.unwrap();
        let connection = SqliteAgentConnectionRepository::new(reopened.0)
            .get()
            .await
            .unwrap()
            .unwrap();
        assert_eq!(connection.status, AgentConnectionStatus::Active);
        assert_eq!(
            connection.token_hash.as_deref(),
            Some("a".repeat(64).as_str())
        );
    }
}
