use sqlx::{Sqlite, SqliteConnection};
use uuid::Uuid;

use super::{
    credentials::todo_credential_reference, hashes_match, unavailable, CodexDelivery, CodexError,
    TODO_TOKEN_PREFIX,
};

const TODO_ID: &str = "codex-todo-suggestions";
const DISABLE_ENV: &str = "AGENT_DESK_DISABLE_TODO_SUGGESTIONS";
type GrantRow = (Option<Vec<u8>>, String, String);

#[derive(serde::Serialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TodoGrantStatus {
    Disabled,
    Disconnected,
    Connected,
    NeedsAuthorization,
}

pub(crate) fn todo_suggestions_enabled() -> bool {
    std::env::var_os(DISABLE_ENV).as_deref() != Some("1".as_ref())
}

impl CodexDelivery {
    pub(super) fn todo_reference(&self) -> String {
        todo_credential_reference(&self.credential_ref)
    }

    async fn todo_row(&self) -> Result<Option<GrantRow>, sqlx::Error> {
        sqlx::query_as::<Sqlite, GrantRow>(
            "SELECT token_hash, state, dedupe_namespace FROM codex_todo_suggestion_grant
             WHERE id=? AND scope='suggest_todos'",
        )
        .bind(TODO_ID)
        .fetch_optional(&self.pool)
        .await
    }

    async fn todo_status_locked(&self) -> Result<TodoGrantStatus, String> {
        if !self.enabled || !todo_suggestions_enabled() {
            return Ok(TodoGrantStatus::Disabled);
        }
        let Some((Some(hash), state, _)) = self.todo_row().await.map_err(|_| unavailable())? else {
            return Ok(TodoGrantStatus::Disconnected);
        };
        if state != "active" {
            return Ok(TodoGrantStatus::Disconnected);
        }
        Ok(match self.read_credential(self.todo_reference()).await {
            Ok(Some(secret)) if hashes_match(&hash, &secret) => TodoGrantStatus::Connected,
            _ => TodoGrantStatus::NeedsAuthorization,
        })
    }

    pub(crate) async fn todo_status(&self) -> Result<TodoGrantStatus, String> {
        let _guard = self.gate.lock().await;
        self.todo_status_locked().await
    }

    pub(crate) async fn pair_todo(&self) -> Result<TodoGrantStatus, String> {
        if !self.enabled || !todo_suggestions_enabled() {
            return Err("待办建议已禁用。".to_owned());
        }
        let _guard = self.gate.lock().await;
        {
            let mut connection = self.pool.acquire().await.map_err(|_| unavailable())?;
            Self::revoke_todo_db(&mut connection)
                .await
                .map_err(|_| unavailable())?;
        }
        let reference = self.todo_reference();
        let hash = self
            .issue_credential(TODO_TOKEN_PREFIX, reference.clone())
            .await?;
        let updated = sqlx::query(
            "INSERT INTO codex_todo_suggestion_grant
             (id, token_hash, state, scope, dedupe_namespace, created_at, updated_at)
             VALUES (?, ?, 'active', 'suggest_todos', ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
             ON CONFLICT(id) DO UPDATE SET token_hash=excluded.token_hash,
             state='active', updated_at=CURRENT_TIMESTAMP",
        )
        .bind(TODO_ID)
        .bind(hash)
        .bind(Uuid::new_v4().simple().to_string())
        .execute(&self.pool)
        .await;
        if updated.is_err() {
            let _ = self.delete_credential(reference).await;
            return Err(unavailable());
        }
        self.todo_status_locked().await
    }

    pub(super) async fn revoke_todo_db(
        connection: &mut SqliteConnection,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE codex_todo_suggestion_grant
             SET token_hash=NULL, state='revoked', updated_at=CURRENT_TIMESTAMP WHERE id=?",
        )
        .bind(TODO_ID)
        .execute(connection)
        .await?;
        Ok(())
    }

    pub(crate) async fn revoke_todo(&self) -> Result<TodoGrantStatus, String> {
        let _guard = self.gate.lock().await;
        {
            let mut connection = self.pool.acquire().await.map_err(|_| unavailable())?;
            Self::revoke_todo_db(&mut connection)
                .await
                .map_err(|_| unavailable())?;
        }
        self.delete_credential(self.todo_reference())
            .await
            .map_err(|_| {
                "待办建议的服务端授权已撤销，但系统凭据清理失败；请重试撤销。".to_owned()
            })?;
        self.todo_status_locked().await
    }

    // Caller holds the shared Codex gate through the write, not only this lookup.
    pub(super) async fn authenticate_todo(&self, token: &str) -> Result<String, CodexError> {
        if !token.starts_with(TODO_TOKEN_PREFIX) {
            return Err(CodexError::Unauthorized);
        }
        let Some((Some(hash), state, namespace)) =
            self.todo_row().await.map_err(|_| CodexError::Unavailable)?
        else {
            return Err(CodexError::Unauthorized);
        };
        if state != "active" || !hashes_match(&hash, token) {
            return Err(CodexError::Unauthorized);
        }
        if !self.enabled || !todo_suggestions_enabled() {
            return Err(CodexError::Forbidden);
        }
        Ok(namespace)
    }
}
