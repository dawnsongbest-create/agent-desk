use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, SqlitePool};
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
    codex_delivery::{
        credentials::{credential_reference, OsCredentialStore},
        CodexError, ReadingRequest, MAX_CODEX_BODY,
    },
    domain::proposal::todo_batch::{TodoSuggestionRequest, MAX_TODO_BODY},
};

pub(super) const TOKEN_PREFIX: &str = "adk_dumate_";
const TTL_SECONDS: i64 = 30 * 60;
const SOURCE_LABEL: &str = "百度搭子";
const PREVIEW_ENV: &str = "AGENT_DESK_ENABLE_DUMATE_PREVIEW";
const UNAVAILABLE: &str = "百度搭子本地授权暂不可用。";
const STORE_ERROR: &str = "系统凭据存储操作失败；请检查后重试。";

type Changed = Arc<dyn Fn() + Send + Sync>;
type Clock = fn() -> i64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Scopes {
    pub reading: bool,
    pub todo: bool,
}

impl Scopes {
    pub fn allows(self, name: &str) -> bool {
        match name {
            "send_reading" => self.reading,
            "suggest_todos" => self.todo,
            _ => false,
        }
    }
}

#[derive(FromRow)]
pub(super) struct Grant {
    reading_namespace: String,
    todo_namespace: String,
    reading_enabled: bool,
    todo_enabled: bool,
    token_hash: Option<Vec<u8>>,
    issued_at: Option<i64>,
    expires_at: Option<i64>,
}

impl Grant {
    pub fn scopes(&self) -> Scopes {
        Scopes {
            reading: self.reading_enabled,
            todo: self.todo_enabled,
        }
    }

    fn usable(&self, now: i64) -> bool {
        matches!((self.issued_at, self.expires_at), (Some(from), Some(until))
            if from <= now && now < until)
    }

    fn matches(&self, token: &str) -> bool {
        valid_token(token)
            && self.token_hash.as_ref().is_some_and(|hash| {
                let digest: [u8; 32] = Sha256::digest(token.as_bytes()).into();
                bool::from(hash.ct_eq(&digest))
            })
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DuMateStatus {
    preview_available: bool,
    bridge_running: bool,
    endpoint: String,
    reading: bool,
    todo: bool,
    expires_at: Option<i64>,
}

pub(crate) struct DuMate {
    pool: SqlitePool,
    pub(super) gate: Mutex<()>,
    deliveries: DeliveryService,
    proposals: ProposalService,
    inbox_changed: Changed,
    todo_changed: Changed,
    credentials: Arc<dyn CredentialPort>,
    credential_ref: String,
    pub(super) preview_available: bool,
    clock: Clock,
}

impl DuMate {
    pub(crate) fn new(
        pool: SqlitePool,
        deliveries: DeliveryService,
        proposals: ProposalService,
        inbox_changed: Changed,
        todo_changed: Changed,
    ) -> Self {
        Self {
            pool,
            gate: Mutex::new(()),
            deliveries,
            proposals,
            inbox_changed,
            todo_changed,
            credentials: Arc::new(OsCredentialStore),
            credential_ref: format!("{}-dumate-v1", credential_reference()),
            preview_available: crate::audit_data_dir().is_some()
                && std::env::var(PREVIEW_ENV).as_deref() == Ok("1"),
            clock: unix_time,
        }
    }

    #[cfg(test)]
    pub(super) fn isolated(
        pool: SqlitePool,
        deliveries: DeliveryService,
        proposals: ProposalService,
        credentials: Arc<dyn CredentialPort>,
        clock: Clock,
    ) -> Self {
        let mut service = Self::new(
            pool,
            deliveries,
            proposals,
            Arc::new(|| {}),
            Arc::new(|| {}),
        );
        service.credentials = credentials;
        service.credential_ref = format!("test-{}", Uuid::new_v4().simple());
        service.preview_available = true;
        service.clock = clock;
        service
    }

    async fn grant(&self) -> Result<Option<Grant>, sqlx::Error> {
        sqlx::query_as(
            "SELECT c.reading_namespace, s.dedupe_namespace AS todo_namespace, c.reading_enabled,
                    c.todo_enabled, c.token_hash, c.issued_at, c.expires_at
             FROM local_mcp_connections c JOIN todo_suggestion_sources s USING (provider)
             WHERE c.provider='dumate'",
        )
        .fetch_optional(&self.pool)
        .await
    }

    // Callers hold gate across current authentication AND the business commit.
    pub(super) async fn authenticate(&self, token: &str) -> Result<Grant, CodexError> {
        if !self.preview_available || !valid_token(token) {
            return Err(CodexError::Unauthorized);
        }
        self.grant()
            .await
            .map_err(|_| CodexError::Unavailable)?
            .filter(|grant| grant.usable((self.clock)()) && grant.matches(token))
            .ok_or(CodexError::Unauthorized)
    }

    pub(super) async fn reserve_legacy_key(&self, key: Option<&str>) -> Result<(), CodexError> {
        let Some(key) = key
            .map(str::trim)
            .filter(|key| key.starts_with("dumate-reading:"))
        else {
            return Ok(());
        };
        let namespace: Option<String> = sqlx::query_scalar(
            "SELECT reading_namespace FROM local_mcp_connections WHERE provider='dumate'",
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| CodexError::Unavailable)?;
        if namespace
            .is_some_and(|namespace| key.starts_with(&format!("dumate-reading:{namespace}:")))
        {
            return Err(CodexError::Forbidden);
        }
        Ok(())
    }

    pub(super) async fn status(&self, port: u16, running: bool) -> Result<DuMateStatus, String> {
        let _guard = self.gate.lock().await;
        self.status_locked(port, running).await
    }

    async fn status_locked(&self, port: u16, running: bool) -> Result<DuMateStatus, String> {
        let mut status = DuMateStatus {
            preview_available: self.preview_available,
            bridge_running: running,
            endpoint: format!("http://127.0.0.1:{port}/mcp"),
            reading: false,
            todo: false,
            expires_at: None,
        };
        if self.preview_available {
            if let Some(grant) = self.grant().await.map_err(|_| UNAVAILABLE)? {
                status.expires_at = grant.expires_at;
                if grant.usable((self.clock)()) {
                    status.reading = grant.reading_enabled;
                    status.todo = grant.todo_enabled;
                }
            }
        }
        Ok(status)
    }

    pub(super) async fn configure(
        &self,
        port: u16,
        running: bool,
        scopes: Scopes,
        rotate: bool,
    ) -> Result<DuMateStatus, String> {
        if !self.preview_available {
            return Err("百度搭子功能仅在已启用的隔离预览中可用。".into());
        }
        let _guard = self.gate.lock().await;
        if !scopes.reading && !scopes.todo {
            self.disable_locked().await?;
            self.delete_credential().await?;
            return self.status_locked(port, running).await;
        }
        if !running {
            return Err("请先启用本地 Bridge。".into());
        }
        if !rotate
            && self
                .grant()
                .await
                .map_err(|_| UNAVAILABLE)?
                .is_some_and(|grant| {
                    grant.usable((self.clock)())
                        && (!scopes.reading || grant.reading_enabled)
                        && (!scopes.todo || grant.todo_enabled)
                })
        {
            // Narrowing does not grant any new powers or extend the lifetime.
            sqlx::query("UPDATE local_mcp_connections SET reading_enabled=?, todo_enabled=? WHERE provider='dumate'")
                .bind(scopes.reading)
                .bind(scopes.todo)
                .execute(&self.pool)
                .await
                .map_err(|_| UNAVAILABLE)?;
            return self.status_locked(port, running).await;
        }

        // Invalidate the prior hash first: OS credential failures must never
        // leave a wider grant paired with an older client credential.
        self.disable_locked().await?;
        let token = format!(
            "{TOKEN_PREFIX}{}{}{}",
            Uuid::new_v4().simple(),
            Uuid::new_v4().simple(),
            Uuid::new_v4().simple()
        );
        let hash: Vec<u8> = Sha256::digest(token.as_bytes()).to_vec();
        let credentials = self.credentials.clone();
        let reference = self.credential_ref.clone();
        let saved = tokio::task::spawn_blocking(move || {
            credentials.store(&reference, &token)?;
            Ok::<_, crate::application::ports::credentials::CredentialError>(
                credentials.read(&reference)?.as_deref() == Some(token.as_str()),
            )
        })
        .await;
        if !matches!(saved, Ok(Ok(true))) {
            let _ = self.delete_credential().await;
            return Err(STORE_ERROR.into());
        }
        if self.activate_locked(scopes, hash).await.is_err() {
            let _ = self.delete_credential().await;
            return Err(UNAVAILABLE.into());
        }
        self.status_locked(port, running).await
    }

    async fn activate_locked(&self, scopes: Scopes, hash: Vec<u8>) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        sqlx::query(
            "INSERT INTO todo_suggestion_sources (dedupe_namespace, provider) VALUES (?, 'dumate')
             ON CONFLICT(provider) DO NOTHING",
        )
        .bind(Uuid::new_v4().simple().to_string())
        .execute(&mut *tx)
        .await?;
        let issued = (self.clock)();
        sqlx::query(
            "INSERT INTO local_mcp_connections
             (provider,reading_namespace,reading_enabled,todo_enabled,token_hash,issued_at,expires_at,generation)
             VALUES ('dumate',?,?,?,?,?,?,1)
             ON CONFLICT(provider) DO UPDATE SET reading_enabled=excluded.reading_enabled,
                 todo_enabled=excluded.todo_enabled, token_hash=excluded.token_hash,
                 issued_at=excluded.issued_at, expires_at=excluded.expires_at, generation=generation+1",
        )
        .bind(Uuid::new_v4().simple().to_string())
        .bind(scopes.reading)
        .bind(scopes.todo)
        .bind(hash)
        .bind(issued)
        .bind(issued + TTL_SECONDS)
        .execute(&mut *tx)
        .await?;
        tx.commit().await
    }

    async fn disable_locked(&self) -> Result<(), String> {
        sqlx::query(
            "UPDATE local_mcp_connections SET reading_enabled=0, todo_enabled=0,
             token_hash=NULL, issued_at=NULL, expires_at=NULL, generation=generation+1
             WHERE provider='dumate'",
        )
        .execute(&self.pool)
        .await
        .map_err(|_| UNAVAILABLE)?;
        Ok(())
    }

    async fn delete_credential(&self) -> Result<(), String> {
        let credentials = self.credentials.clone();
        let reference = self.credential_ref.clone();
        tokio::task::spawn_blocking(move || credentials.delete(&reference))
            .await
            .map_err(|_| STORE_ERROR)?
            .map_err(|_| STORE_ERROR.to_owned())
    }

    pub(super) async fn authorization_header(&self) -> Result<String, String> {
        let _guard = self.gate.lock().await;
        if !self.preview_available {
            return Err(UNAVAILABLE.into());
        }
        let credentials = self.credentials.clone();
        let reference = self.credential_ref.clone();
        let token = tokio::task::spawn_blocking(move || credentials.read(&reference))
            .await
            .map_err(|_| STORE_ERROR)?
            .map_err(|_| STORE_ERROR)?
            .ok_or(STORE_ERROR)?;
        self.authenticate(&token)
            .await
            .map_err(|_| "授权已到期或撤销，请重新授权。")?;
        Ok(format!("Bearer {token}"))
    }

    pub(super) async fn invoke(
        &self,
        grant: &Grant,
        name: &str,
        arguments: &serde_json::value::RawValue,
    ) -> Result<Value, CodexError> {
        if !grant.usable((self.clock)()) {
            return Err(CodexError::Unauthorized);
        }
        if !grant.scopes().allows(name) {
            return Err(CodexError::Forbidden);
        }
        match name {
            "send_reading" => {
                if arguments.get().len() > MAX_CODEX_BODY {
                    return Err(CodexError::TooLarge);
                }
                let request: ReadingRequest = serde_json::from_str(arguments.get())
                    .map_err(|_| CodexError::InvalidPayload)?;
                request.validate()?;
                let key = request
                    .idempotency_key
                    .as_deref()
                    .ok_or(CodexError::InvalidPayload)?
                    .trim()
                    .to_owned();
                let digest = Sha256::digest(key.as_bytes());
                let namespace = &grant.reading_namespace;
                let saved = self
                    .deliveries
                    .ingest(request.into_delivery(
                        format!("dumate-reading:{namespace}:{digest:x}"),
                        SOURCE_LABEL,
                    ))
                    .await
                    .map_err(|error| match error {
                        DeliveryServiceError::Validation(_) => CodexError::InvalidPayload,
                        DeliveryServiceError::Repository(
                            DeliveryRepositoryError::IdempotencyConflict,
                        ) => CodexError::Conflict,
                        DeliveryServiceError::Repository(_) => CodexError::Unavailable,
                    })?;
                if saved.created {
                    (self.inbox_changed)();
                }
                Ok(json!({
                    "version": "v1", "operation": name, "request_id": key,
                    "delivery_id": saved.item.delivery.id, "document_id": saved.item.document.id,
                    "created": saved.created,
                }))
            }
            "suggest_todos" => {
                if arguments.get().len() > MAX_TODO_BODY {
                    return Err(CodexError::TooLarge);
                }
                let request: TodoSuggestionRequest = serde_json::from_str(arguments.get())
                    .map_err(|_| CodexError::InvalidPayload)?;
                let key = request.idempotency_key.ok_or(CodexError::InvalidPayload)?;
                let (batch, created) = self
                    .proposals
                    .submit_todo_batch(&grant.todo_namespace, &key, request.items)
                    .await
                    .map_err(|error| match error {
                        ProposalServiceError::Repository(
                            ProposalRepositoryError::TodoBatchConflict,
                        ) => CodexError::Conflict,
                        ProposalServiceError::Repository(_) => CodexError::Unavailable,
                        _ => CodexError::InvalidPayload,
                    })?;
                if created {
                    (self.todo_changed)();
                }
                Ok(json!({
                    "version": "v1", "operation": name, "request_id": key,
                    "batch_id": batch.id, "status": batch.status, "created": created,
                }))
            }
            _ => Err(CodexError::Forbidden),
        }
    }
}

fn unix_time() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn valid_token(token: &str) -> bool {
    token
        .strip_prefix(TOKEN_PREFIX)
        .is_some_and(|part| part.len() == 96 && part.bytes().all(|byte| byte.is_ascii_hexdigit()))
}
