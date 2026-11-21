pub(crate) mod credentials;
pub(crate) mod mcp;
mod todo_grant;

pub(crate) use todo_grant::{todo_suggestions_enabled, TodoGrantStatus};

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use subtle::ConstantTimeEq;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::{
    application::{
        delivery_service::{DeliveryService, DeliveryServiceError},
        ports::{
            credentials::CredentialPort, delivery_repository::DeliveryRepositoryError,
            proposal_repository::ProposalRepositoryError,
        },
        proposal_service::{ProposalService, ProposalServiceError},
    },
    domain::{
        delivery::CreateDeliveryInput,
        proposal::{
            todo_batch::{TodoSuggestionRequest, MAX_TODO_BODY},
            AgentProposalStatus,
        },
        reader::{ReaderDocumentType, ReaderSourceType},
    },
};

pub(crate) const MAX_CODEX_BODY: usize = 1024 * 1024;
pub(crate) const READING_PATH: &str = "/api/v1/reading-deliveries";
pub(crate) const TODO_PATH: &str = "/api/v1/todo-suggestions";
pub(crate) const CODEX_TOKEN_PREFIX: &str = "adk_codex_";
pub(crate) const TODO_TOKEN_PREFIX: &str = "adk_codex_todo_";
pub(crate) const INBOX_CHANGED: &str = "agent-desk://inbox-changed";
pub(crate) const TODO_SUGGESTIONS_CHANGED: &str = "agent-desk://todo-suggestions-changed";

const CONNECTION_ID: &str = "codex-reading";
const DISABLE_ENV: &str = "AGENT_DESK_DISABLE_CODEX";

type PairingRow = (Option<Vec<u8>>, String, String, Option<String>);
type Changed = Arc<dyn Fn() + Send + Sync>;

pub fn start_stdio_connector(args: &[String]) -> i32 {
    mcp::run(args)
}

pub fn start_chatgpt_connector(args: &[String]) -> i32 {
    mcp::run_chatgpt(args)
}

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ReadingType {
    Article,
    Report,
    Brief,
}

impl From<ReadingType> for ReaderDocumentType {
    fn from(value: ReadingType) -> Self {
        match value {
            ReadingType::Article => Self::Article,
            ReadingType::Report => Self::Report,
            ReadingType::Brief => Self::Brief,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReadingRequest {
    title: String,
    content: String,
    #[serde(rename = "type")]
    kind: ReadingType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    subtitle: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) idempotency_key: Option<String>,
}

impl ReadingRequest {
    pub(crate) fn validate(&self) -> Result<(), CodexError> {
        let title = self.title.trim();
        if title.is_empty()
            || title.chars().count() > 500
            || self.subtitle.as_ref().is_some_and(|subtitle| {
                subtitle.trim().chars().count() > 500 || subtitle.contains('\0')
            })
            || title.contains('\0')
            || self.content.trim().is_empty()
            || self.content.contains('\0')
        {
            return Err(CodexError::InvalidPayload);
        }
        if self.content.chars().count() > 1_000_000 {
            return Err(CodexError::TooLarge);
        }
        if self.idempotency_key.as_ref().is_some_and(|key| {
            let key = key.trim();
            key.is_empty() || key.chars().count() > 500 || key.chars().any(char::is_control)
        }) {
            return Err(CodexError::InvalidPayload);
        }
        Ok(())
    }

    pub(crate) fn into_delivery(self, key: String, source: &str) -> CreateDeliveryInput {
        CreateDeliveryInput {
            idempotency_key: key,
            document_type: self.kind.into(),
            title: self.title,
            subtitle: self.subtitle,
            content_markdown: self.content,
            source_type: ReaderSourceType::Agent,
            source_label: Some(source.to_owned()),
            delivered_at: None,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadingReceipt {
    version: String,
    operation: String,
    request_id: String,
    delivery_id: String,
    document_id: String,
    created: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TodoReceipt {
    version: String,
    operation: String,
    request_id: String,
    batch_id: String,
    status: AgentProposalStatus,
    created: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CodexError {
    Unauthorized,
    Forbidden,
    TooLarge,
    InvalidPayload,
    Conflict,
    Unavailable,
}

impl CodexError {
    pub(crate) const fn http(self) -> (u16, &'static str) {
        match self {
            Self::Unauthorized => (401, r#"{"error":"unauthorized"}"#),
            Self::Forbidden => (403, r#"{"error":"forbidden_capability"}"#),
            Self::InvalidPayload => (400, r#"{"error":"invalid_payload"}"#),
            Self::TooLarge => (413, r#"{"error":"payload_too_large"}"#),
            Self::Conflict => (409, r#"{"error":"idempotency_conflict"}"#),
            Self::Unavailable => (503, r#"{"error":"bridge_unavailable"}"#),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum PairingPhase {
    Disabled,
    Disconnected,
    AwaitingTest,
    Connected,
    NeedsAuthorization,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PairingStatus {
    phase: PairingPhase,
    executable: Option<String>,
    arguments: Vec<String>,
}

pub(crate) struct CodexDelivery {
    pool: SqlitePool,
    deliveries: DeliveryService,
    proposals: ProposalService,
    credential_store: Arc<dyn CredentialPort>,
    credential_ref: String,
    enabled: bool,
    // Serialize Codex requests and configuration changes. Revocation returning
    // success means no in-flight authorized Codex write can still commit.
    gate: Mutex<()>,
    inbox_changed: Changed,
    todo_suggestions_changed: Changed,
}

impl CodexDelivery {
    pub(crate) fn new(
        pool: SqlitePool,
        deliveries: DeliveryService,
        proposals: ProposalService,
        inbox_changed: Changed,
        todo_suggestions_changed: Changed,
    ) -> Self {
        Self {
            pool,
            deliveries,
            proposals,
            credential_store: Arc::new(credentials::OsCredentialStore),
            credential_ref: credentials::credential_reference(),
            enabled: std::env::var_os(DISABLE_ENV).as_deref() != Some("1".as_ref()),
            gate: Mutex::new(()),
            inbox_changed,
            todo_suggestions_changed,
        }
    }

    async fn row(&self) -> Result<Option<PairingRow>, sqlx::Error> {
        sqlx::query_as(
            "SELECT token_hash, state, dedupe_namespace, delivered_at
             FROM codex_connection WHERE id = ? AND scope = 'send_reading'",
        )
        .bind(CONNECTION_ID)
        .fetch_optional(&self.pool)
        .await
    }

    async fn read_credential(&self, reference: String) -> Result<Option<String>, String> {
        let store = Arc::clone(&self.credential_store);
        tokio::task::spawn_blocking(move || store.read(&reference))
            .await
            .map_err(|_| credential_error())?
            .map_err(|_| credential_error())
    }

    pub(crate) async fn status(&self, port: u16) -> Result<PairingStatus, String> {
        let _guard = self.gate.lock().await;
        self.status_locked(port).await
    }

    async fn status_locked(&self, port: u16) -> Result<PairingStatus, String> {
        let phase = if !self.enabled {
            PairingPhase::Disabled
        } else if let Some((hash, state, _, delivered_at)) =
            self.row().await.map_err(|_| unavailable())?
        {
            if state != "active" {
                PairingPhase::Disconnected
            } else if match self.read_credential(self.credential_ref.clone()).await {
                Ok(Some(secret)) => hash.as_deref().is_some_and(|h| hashes_match(h, &secret)),
                _ => false,
            } {
                if delivered_at.is_some() {
                    PairingPhase::Connected
                } else {
                    PairingPhase::AwaitingTest
                }
            } else {
                PairingPhase::NeedsAuthorization
            }
        } else {
            PairingPhase::Disconnected
        };
        let executable = std::env::current_exe()
            .ok()
            .and_then(|path| path.into_os_string().into_string().ok());
        Ok(PairingStatus {
            phase,
            executable,
            arguments: vec!["--codex-mcp".to_owned(), port.to_string()],
        })
    }

    pub(crate) async fn pair(&self, port: u16) -> Result<PairingStatus, String> {
        if !self.enabled {
            return Err("Codex 投递已禁用。".to_owned());
        }
        if std::env::current_exe()
            .ok()
            .is_none_or(|p| !p.is_file() || p.to_str().is_none())
        {
            return Err("无法定位本地 Codex 连接器。请从安装后的 Agent Desk 重试。".to_owned());
        }
        let _guard = self.gate.lock().await;
        // Fail closed on partial pairing, without changing the OpenClaw row.
        self.revoke_db().await?;
        let hash = self
            .issue_credential(CODEX_TOKEN_PREFIX, self.credential_ref.clone())
            .await?;
        let updated = sqlx::query(
            "INSERT INTO codex_connection
             (id, token_hash, state, scope, dedupe_namespace, created_at, updated_at, delivered_at)
             VALUES (?, ?, 'active', 'send_reading', ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, NULL)
             ON CONFLICT(id) DO UPDATE SET token_hash=excluded.token_hash,
             state='active', updated_at=CURRENT_TIMESTAMP, delivered_at=NULL",
        )
        .bind(CONNECTION_ID)
        .bind(hash)
        .bind(Uuid::new_v4().simple().to_string())
        .execute(&self.pool)
        .await;
        if updated.is_err() {
            let _ = self.delete_credential(self.credential_ref.clone()).await;
            return Err(unavailable());
        }
        self.status_locked(port).await
    }

    async fn issue_credential(&self, prefix: &str, reference: String) -> Result<Vec<u8>, String> {
        let token = format!(
            "{prefix}{}{}",
            Uuid::new_v4().simple(),
            Uuid::new_v4().simple()
        );
        let hash = Sha256::digest(token.as_bytes()).to_vec();
        let store = Arc::clone(&self.credential_store);
        tokio::task::spawn_blocking(move || {
            if let Err(error) = store.store(&reference, &token) {
                let _ = store.delete(&reference);
                return Err(error);
            }
            let expected: [u8; 32] = Sha256::digest(token.as_bytes()).into();
            match store.read(&reference) {
                Ok(Some(recovered)) if hashes_match(&expected, &recovered) => Ok(()),
                _ => {
                    let _ = store.delete(&reference);
                    Err(crate::application::ports::credentials::CredentialError::OperationFailed)
                }
            }
        })
        .await
        .map_err(|_| credential_error())?
        .map_err(|_| credential_error())?;
        Ok(hash)
    }

    async fn revoke_db(&self) -> Result<(), String> {
        sqlx::query(
            "UPDATE codex_connection SET token_hash=NULL, state='revoked',
                     updated_at=CURRENT_TIMESTAMP, delivered_at=NULL WHERE id=?",
        )
        .bind(CONNECTION_ID)
        .execute(&self.pool)
        .await
        .map_err(|_| unavailable())?;
        Ok(())
    }

    async fn delete_credential(&self, reference: String) -> Result<(), String> {
        let store = Arc::clone(&self.credential_store);
        tokio::task::spawn_blocking(move || store.delete(&reference))
            .await
            .map_err(|_| credential_error())?
            .map_err(|_| credential_error())
    }

    pub(crate) async fn revoke(&self, port: u16) -> Result<PairingStatus, String> {
        let _guard = self.gate.lock().await;
        let mut tx = self.pool.begin().await.map_err(|_| unavailable())?;
        sqlx::query(
            "UPDATE codex_connection SET token_hash=NULL, state='revoked',
             updated_at=CURRENT_TIMESTAMP, delivered_at=NULL WHERE id=?",
        )
        .bind(CONNECTION_ID)
        .execute(&mut *tx)
        .await
        .map_err(|_| unavailable())?;
        Self::revoke_todo_db(&mut tx)
            .await
            .map_err(|_| unavailable())?;
        tx.commit().await.map_err(|_| unavailable())?;
        let reading = self.delete_credential(self.credential_ref.clone()).await;
        let todo = self.delete_credential(self.todo_reference()).await;
        if reading.is_err() || todo.is_err() {
            return Err("Codex 服务端授权已撤销，但系统凭据清理失败；请重试断开。".to_owned());
        }
        self.status_locked(port).await
    }

    // Protect the exact private namespace, including after revocation. Normal
    // OpenClaw keys and historical v1 idempotency behavior remain unchanged.
    pub(crate) async fn reserve_legacy_key(&self, key: Option<&str>) -> Result<(), CodexError> {
        let Some(key) = key.filter(|key| key.trim_start().starts_with("codex-reading:")) else {
            return Ok(());
        };
        if let Some((_, _, namespace, _)) = self.row().await.map_err(|_| CodexError::Unavailable)? {
            if key
                .trim()
                .starts_with(&format!("codex-reading:{namespace}:"))
            {
                return Err(CodexError::Forbidden);
            }
        }
        Ok(())
    }

    pub(crate) async fn dispatch(
        &self,
        token: &str,
        method: &str,
        path: &str,
        body: &[u8],
    ) -> Result<String, CodexError> {
        let _guard = self.gate.lock().await;
        if token.starts_with(TODO_TOKEN_PREFIX) {
            let namespace = self.authenticate_todo(token).await?;
            if method != "POST" || path != TODO_PATH {
                return Err(CodexError::Forbidden);
            }
            return self.suggest_todos(&namespace, body).await;
        }
        let Some((Some(hash), state, namespace, _)) =
            self.row().await.map_err(|_| CodexError::Unavailable)?
        else {
            return Err(CodexError::Unauthorized);
        };
        if state != "active" || !hashes_match(&hash, token) {
            return Err(CodexError::Unauthorized);
        }
        if !self.enabled || method != "POST" || path != READING_PATH {
            return Err(CodexError::Forbidden);
        }
        if body.len() > MAX_CODEX_BODY {
            return Err(CodexError::TooLarge);
        }
        let request: ReadingRequest =
            serde_json::from_slice(body).map_err(|_| CodexError::InvalidPayload)?;
        request.validate()?;
        let key = request
            .idempotency_key
            .as_deref()
            .ok_or(CodexError::InvalidPayload)?
            .trim();
        let key = key.to_owned();
        let key_digest = format!("{:x}", Sha256::digest(key.as_bytes()));
        let stored = self
            .deliveries
            .ingest(
                request.into_delivery(format!("codex-reading:{namespace}:{key_digest}"), "Codex"),
            )
            .await
            .map_err(|error| match error {
                DeliveryServiceError::Validation(_) => CodexError::InvalidPayload,
                DeliveryServiceError::Repository(DeliveryRepositoryError::IdempotencyConflict) => {
                    CodexError::Conflict
                }
                DeliveryServiceError::Repository(_) => CodexError::Unavailable,
            })?;
        let _ = sqlx::query(
            "UPDATE codex_connection SET delivered_at=CURRENT_TIMESTAMP
             WHERE id=? AND delivered_at IS NULL",
        )
        .bind(CONNECTION_ID)
        .execute(&self.pool)
        .await;
        if stored.created {
            (self.inbox_changed)();
        }
        serde_json::to_string(&ReadingReceipt {
            version: "v1".into(),
            operation: "send_reading".into(),
            request_id: key,
            delivery_id: stored.item.delivery.id,
            document_id: stored.item.document.id,
            created: stored.created,
        })
        .map_err(|_| CodexError::Unavailable)
    }

    async fn suggest_todos(&self, namespace: &str, body: &[u8]) -> Result<String, CodexError> {
        if body.len() > MAX_TODO_BODY {
            return Err(CodexError::TooLarge);
        }
        let request: TodoSuggestionRequest =
            serde_json::from_slice(body).map_err(|_| CodexError::InvalidPayload)?;
        let key = request.idempotency_key.ok_or(CodexError::InvalidPayload)?;
        let (batch, created) = self
            .proposals
            .submit_todo_batch(namespace, &key, request.items)
            .await
            .map_err(|error| match error {
                ProposalServiceError::Repository(ProposalRepositoryError::TodoBatchConflict) => {
                    CodexError::Conflict
                }
                ProposalServiceError::Repository(_) => CodexError::Unavailable,
                _ => CodexError::InvalidPayload,
            })?;
        if created {
            (self.todo_suggestions_changed)();
        }
        serde_json::to_string(&TodoReceipt {
            version: "v1".into(),
            operation: "suggest_todos".into(),
            request_id: key,
            batch_id: batch.id,
            status: batch.status,
            created,
        })
        .map_err(|_| CodexError::Unavailable)
    }
}

fn hashes_match(hash: &[u8], token: &str) -> bool {
    let candidate: [u8; 32] = Sha256::digest(token.as_bytes()).into();
    bool::from(hash.ct_eq(&candidate))
}

fn credential_error() -> String {
    "系统凭据库不可访问。请检查系统授权后重试，不会使用明文文件。".to_owned()
}

fn unavailable() -> String {
    "无法更新 Codex 连接状态，请重试。".to_owned()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Mutex as SyncMutex,
    };

    use super::*;
    use crate::{
        application::agent_connection_service::AgentConnectionService,
        application::ports::credentials::CredentialError,
        domain::agent_connection::AgentConnectionStatus,
        persistence::{
            agent_connection_repository::SqliteAgentConnectionRepository,
            delivery_repository::SqliteDeliveryRepository,
            proposal_repository::SqliteProposalRepository, sqlite,
        },
    };

    #[derive(Default)]
    struct TestStore {
        secrets: SyncMutex<HashMap<String, String>>,
        fail: AtomicBool,
        fail_read: AtomicBool,
        fail_delete: AtomicBool,
    }

    impl CredentialPort for TestStore {
        fn store(&self, reference: &str, token: &str) -> Result<(), CredentialError> {
            if self.fail.load(Ordering::SeqCst) {
                return Err(CredentialError::Unavailable);
            }
            self.secrets
                .lock()
                .unwrap()
                .insert(reference.to_owned(), token.to_owned());
            Ok(())
        }
        fn read(&self, reference: &str) -> Result<Option<String>, CredentialError> {
            if self.fail_read.load(Ordering::SeqCst) {
                return Err(CredentialError::Unavailable);
            }
            Ok(self.secrets.lock().unwrap().get(reference).cloned())
        }
        fn delete(&self, reference: &str) -> Result<(), CredentialError> {
            if self.fail_delete.load(Ordering::SeqCst) {
                return Err(CredentialError::Unavailable);
            }
            self.secrets.lock().unwrap().remove(reference);
            Ok(())
        }
    }

    fn body(key: &str, content: &str, kind: &str) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "title": "Synthetic reading", "content": content, "type": kind,
            "subtitle": "阶段测试", "idempotency_key": key
        }))
        .unwrap()
    }

    fn proposals(pool: &SqlitePool) -> ProposalService {
        ProposalService::new(Arc::new(SqliteProposalRepository::new(pool.clone())))
    }

    #[tokio::test]
    async fn pairing_delivery_concurrent_replay_and_revocation_leave_openclaw_unchanged() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("isolated.sqlite3");
        let db = sqlite::connect(&path).await.unwrap().0;
        let legacy =
            AgentConnectionService::new(Arc::new(SqliteAgentConnectionRepository::new(db.clone())));
        legacy.ensure(AgentConnectionStatus::Active).await.unwrap();
        let old = legacy.generate_token().await.unwrap().token;
        let events = Arc::new(AtomicUsize::new(0));
        let count = events.clone();
        let deliveries = DeliveryService::new(Arc::new(SqliteDeliveryRepository::new(db.clone())));
        let mut codex = CodexDelivery::new(
            db.clone(),
            deliveries.clone(),
            proposals(&db),
            Arc::new(move || {
                count.fetch_add(1, Ordering::SeqCst);
            }),
            Arc::new(|| {}),
        );
        let store = Arc::new(TestStore::default());
        codex.credential_store = store.clone();
        codex.enabled = true;
        let paired = codex.pair(47321).await.unwrap();
        assert!(matches!(paired.phase, PairingPhase::AwaitingTest));
        let token = store.read(&codex.credential_ref).unwrap().unwrap();
        let content = format!("# 标题\n\n{}\n", "**Unicode** — 中文🙂\n".repeat(2500));
        let payload = body("one-send", &content, "article");
        let ingest =
            async |payload: &[u8]| codex.dispatch(&token, "POST", READING_PATH, payload).await;
        let (one, two) = tokio::join!(ingest(&payload), ingest(&payload));
        let one: ReadingReceipt = serde_json::from_str(&one.unwrap()).unwrap();
        let two: ReadingReceipt = serde_json::from_str(&two.unwrap()).unwrap();
        assert_ne!(one.created, two.created);
        assert_eq!(
            (one.delivery_id, one.document_id),
            (two.delivery_id, two.document_id)
        );
        assert_eq!(events.load(Ordering::SeqCst), 1);
        assert_eq!(deliveries.get_unread_count().await.unwrap(), 1);
        let row = deliveries.list_inbox().await.unwrap().remove(0);
        assert_eq!(row.document.content_markdown, content);
        assert_eq!(row.document.source_label.as_deref(), Some("Codex"));
        assert_eq!(row.document.document_type, ReaderDocumentType::Article);
        assert_eq!(
            codex
                .reserve_legacy_key(Some(&row.delivery.idempotency_key))
                .await,
            Err(CodexError::Forbidden)
        );
        assert_eq!(
            ingest(&body("one-send", "changed", "article")).await,
            Err(CodexError::Conflict)
        );
        assert!(ingest(&body("independent-user-send", &content, "article"))
            .await
            .is_ok());
        assert!(ingest(&body("report", "a report", "report")).await.is_ok());
        assert!(ingest(&body("brief", "a brief", "brief")).await.is_ok());
        let documents = deliveries.list_inbox().await.unwrap();
        assert!(documents
            .iter()
            .any(|row| row.document.document_type == ReaderDocumentType::Brief));
        assert!(documents
            .iter()
            .any(|row| row.document.document_type == ReaderDocumentType::Report));
        assert_eq!(ingest(b"{").await, Err(CodexError::InvalidPayload));
        assert_eq!(
            ingest(&vec![b' '; MAX_CODEX_BODY + 1]).await,
            Err(CodexError::TooLarge)
        );
        assert_eq!(ingest(br#"{"title":"x","content":"y","type":"article","idempotency_key":"x","agent_id":"openclaw"}"#).await, Err(CodexError::InvalidPayload));
        assert!(matches!(
            codex.status(47321).await.unwrap().phase,
            PairingPhase::Connected
        ));
        assert_eq!(
            codex
                .dispatch(&token, "POST", "/api/v1/proposal", &[])
                .await,
            Err(CodexError::Forbidden)
        );
        assert_eq!(
            codex.dispatch(&old, "POST", READING_PATH, &payload).await,
            Err(CodexError::Unauthorized)
        );
        assert_eq!(events.load(Ordering::SeqCst), 4);
        assert!(legacy.authenticate(&old).await.unwrap());
        assert!(matches!(
            codex.revoke(47321).await.unwrap().phase,
            PairingPhase::Disconnected
        ));
        assert_eq!(ingest(&payload).await, Err(CodexError::Unauthorized));
        assert!(store.read(&codex.credential_ref).unwrap().is_none());
        assert!(legacy.authenticate(&old).await.unwrap());
        drop(codex);
        db.close().await;
        let reopened = sqlite::connect(&path).await.unwrap().0;
        let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM deliveries")
            .fetch_one(&reopened)
            .await
            .unwrap();
        assert_eq!(rows, 4);
    }

    #[tokio::test]
    async fn credential_write_failure_cannot_leave_authorized_codex() {
        let temp = tempfile::tempdir().unwrap();
        let db = sqlite::connect(&temp.path().join("failure.sqlite3"))
            .await
            .unwrap()
            .0;
        let mut codex = CodexDelivery::new(
            db.clone(),
            DeliveryService::new(Arc::new(SqliteDeliveryRepository::new(db.clone()))),
            proposals(&db),
            Arc::new(|| {}),
            Arc::new(|| {}),
        );
        let store = Arc::new(TestStore::default());
        codex.enabled = true;
        codex.credential_store = store.clone();
        codex.pair(47321).await.unwrap();
        let old = store.read(&codex.credential_ref).unwrap().unwrap();
        store.fail.store(true, Ordering::SeqCst);
        assert!(codex.pair(47321).await.is_err());
        assert_eq!(
            codex.dispatch(&old, "POST", READING_PATH, &[]).await,
            Err(CodexError::Unauthorized)
        );
        assert!(matches!(
            codex.status(47321).await.unwrap().phase,
            PairingPhase::Disconnected
        ));
    }

    #[tokio::test]
    async fn todo_grant_is_independent_fail_closed_and_stable_after_reauthorization() {
        let temp = tempfile::tempdir().unwrap();
        let pool = sqlite::connect(&temp.path().join("grant.sqlite3"))
            .await
            .unwrap()
            .0;
        let mut codex = CodexDelivery::new(
            pool.clone(),
            DeliveryService::new(Arc::new(SqliteDeliveryRepository::new(pool.clone()))),
            proposals(&pool),
            Arc::new(|| {}),
            Arc::new(|| {}),
        );
        let store = Arc::new(TestStore::default());
        codex.credential_store = store.clone();
        codex.enabled = true;
        let reference = codex.todo_reference();
        assert_ne!(reference, codex.credential_ref);
        assert_eq!(
            codex.todo_status().await.unwrap(),
            TodoGrantStatus::Disconnected
        );
        assert_eq!(
            codex.authenticate_todo(TODO_TOKEN_PREFIX).await,
            Err(CodexError::Unauthorized)
        );
        codex.pair(47321).await.unwrap();
        let reading = store.read(&codex.credential_ref).unwrap().unwrap();
        assert_eq!(codex.pair_todo().await.unwrap(), TodoGrantStatus::Connected);
        let todo = store.read(&reference).unwrap().unwrap();
        let namespace = codex.authenticate_todo(&todo).await.unwrap();
        assert_eq!(
            codex.dispatch(&reading, "POST", TODO_PATH, b"{}").await,
            Err(CodexError::Forbidden)
        );
        for path in [READING_PATH, "/api/v1/proposal", "/api/v1/reading-plan"] {
            assert_eq!(
                codex.dispatch(&todo, "POST", path, b"{}").await,
                Err(CodexError::Forbidden)
            );
        }
        store.fail_delete.store(true, Ordering::SeqCst);
        assert!(codex.revoke_todo().await.is_err());
        assert!(store.read(&reference).unwrap().is_some());
        assert_eq!(
            codex.authenticate_todo(&todo).await,
            Err(CodexError::Unauthorized)
        );
        assert!(codex
            .dispatch(
                &reading,
                "POST",
                READING_PATH,
                &body("after-revoke", "reading", "brief")
            )
            .await
            .is_ok());
        store.fail_delete.store(false, Ordering::SeqCst);
        store.fail_read.store(true, Ordering::SeqCst);
        assert!(codex.pair_todo().await.is_err());
        store.fail_read.store(false, Ordering::SeqCst);
        assert!(store.read(&reference).unwrap().is_none());
        store.fail.store(true, Ordering::SeqCst);
        assert!(codex.pair_todo().await.is_err());
        store.fail.store(false, Ordering::SeqCst);
        codex.pair_todo().await.unwrap();
        let new_todo = store.read(&reference).unwrap().unwrap();
        assert!(todo != new_todo);
        assert_eq!(codex.authenticate_todo(&new_todo).await.unwrap(), namespace);
        assert!(store
            .read(&codex.credential_ref)
            .unwrap()
            .is_some_and(|token| token == reading));
        codex.revoke(47321).await.unwrap();
        assert!(store.read(&reference).unwrap().is_none());
        assert_eq!(
            codex.authenticate_todo(&new_todo).await,
            Err(CodexError::Unauthorized)
        );
        assert_eq!(
            codex.dispatch(&reading, "POST", READING_PATH, b"{}").await,
            Err(CodexError::Unauthorized)
        );
    }

    #[tokio::test]
    async fn todo_revocation_waits_for_in_flight_batch_commit() {
        let temp = tempfile::tempdir().unwrap();
        let pool = sqlite::connect(&temp.path().join("revoke-race.sqlite3"))
            .await
            .unwrap()
            .0;
        let mut codex = CodexDelivery::new(
            pool.clone(),
            DeliveryService::new(Arc::new(SqliteDeliveryRepository::new(pool.clone()))),
            proposals(&pool),
            Arc::new(|| {}),
            Arc::new(|| {}),
        );
        let store = Arc::new(TestStore::default());
        codex.credential_store = store.clone();
        codex.enabled = true;
        codex.pair_todo().await.unwrap();
        let token = store.read(&codex.todo_reference()).unwrap().unwrap();
        let request = br#"{"items":[{"text":"Fixture before revoke","priority":"high"}],"idempotency_key":"race"}"#;
        let write_lock = pool.begin_with("BEGIN IMMEDIATE").await.unwrap();
        let (submitted, revoked) =
            tokio::join!(codex.dispatch(&token, "POST", TODO_PATH, request), async {
                tokio::time::timeout(std::time::Duration::from_secs(2), async {
                    while codex.gate.try_lock().is_ok() {
                        tokio::task::yield_now().await;
                    }
                })
                .await
                .unwrap();
                assert!(tokio::time::timeout(
                    std::time::Duration::from_millis(25),
                    codex.revoke_todo()
                )
                .await
                .is_err());
                write_lock.commit().await.unwrap();
                codex.revoke_todo().await
            });
        assert!(submitted.is_ok());
        assert_eq!(revoked.unwrap(), TodoGrantStatus::Disconnected);
        assert_eq!(
            codex.dispatch(&token, "POST", TODO_PATH, request).await,
            Err(CodexError::Unauthorized)
        );
        assert_eq!(
            codex
                .proposals
                .list_pending_todo_batches()
                .await
                .unwrap()
                .len(),
            1
        );
    }
}
