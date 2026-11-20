mod package;

pub(crate) use package::export_package;

use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use hmac::{Hmac, KeyInit, Mac};
use serde::{Deserialize, Serialize};
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
    codex_delivery::{credentials::OsCredentialStore, CodexError, ReadingRequest, MAX_CODEX_BODY},
    domain::proposal::todo_batch::{TodoSuggestionRequest, MAX_TODO_BODY},
};

pub(crate) const CHALLENGE_PATH: &str = "/api/v1/chatgpt/challenge";
pub(crate) const READING_PATH: &str = "/api/v1/chatgpt/reading";
pub(crate) const TODO_PATH: &str = "/api/v1/chatgpt/todos";
pub(crate) const PREFIX: &str = "adk_chatgpt_proof_";
const CHALLENGE_LIFETIME: Duration = Duration::from_secs(15);
const MAX_CHALLENGES: usize = 256;
type Signal = Arc<dyn Fn() + Send + Sync>;
type HmacSha256 = Hmac<sha2_next::Sha256>;

#[derive(Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Scope {
    SendReading,
    SuggestTodos,
}

impl Scope {
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::SendReading => "send_reading",
            Self::SuggestTodos => "suggest_todos",
        }
    }

    pub(crate) const fn path(self) -> &'static str {
        match self {
            Self::SendReading => READING_PATH,
            Self::SuggestTodos => TODO_PATH,
        }
    }

    fn from_path(path: &str) -> Option<Self> {
        match path {
            READING_PATH => Some(Self::SendReading),
            TODO_PATH => Some(Self::SuggestTodos),
            _ => None,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Challenge {
    pub(crate) profile: String,
    pub(crate) port: u16,
    pub(crate) scope: Scope,
    pub(crate) nonce: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ServerProof {
    pub(crate) server_proof: String,
}

#[derive(Clone, Copy)]
pub(crate) struct Scopes {
    pub(crate) reading: bool,
    pub(crate) todo: bool,
}

impl Scopes {
    pub(crate) fn any(self) -> bool {
        self.reading || self.todo
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Status {
    phase: &'static str,
    reading: bool,
    todo: bool,
    last_used_at: Option<String>,
}

#[derive(FromRow)]
struct Grant {
    dedupe_namespace: String,
    token_hash: Option<Vec<u8>>,
    last_used_at: Option<String>,
}

pub(crate) struct ChatGpt {
    pool: SqlitePool,
    deliveries: DeliveryService,
    proposals: ProposalService,
    inbox_changed: Signal,
    todo_changed: Signal,
    credentials: Arc<dyn CredentialPort>,
    profile: String,
    pub(crate) gate: Mutex<()>,
    tickets: Mutex<HashMap<(Scope, String), Instant>>,
}

pub(crate) fn digest(secret: &str) -> Vec<u8> {
    Sha256::digest(secret.as_bytes()).to_vec()
}

pub(crate) fn profile() -> String {
    crate::audit_data_dir()
        .map(|dir| {
            format!(
                "audit-{:x}",
                Sha256::digest(dir.to_string_lossy().as_bytes())
            )
        })
        .unwrap_or_else(|| "desktop-v1".to_owned())
}

pub(crate) fn valid_profile(value: &str) -> bool {
    value == "desktop-v1"
        || value
            .strip_prefix("audit-")
            .is_some_and(|part| hex_of(part, 64))
}

pub(crate) fn reference(profile: &str, scope: Scope) -> String {
    format!("chatgpt-local-v1-{profile}-{}", scope.name())
}

fn hex_of(value: &str, len: usize) -> bool {
    value.len() == len && value.bytes().all(|b| b.is_ascii_hexdigit())
}

pub(crate) fn nonce() -> String {
    Uuid::new_v4().simple().to_string()
}

fn proof(hash: &[u8], mode: &str, challenge: &Challenge, payload: &[u8]) -> String {
    let mut mac = HmacSha256::new_from_slice(hash).expect("HMAC accepts arbitrary key lengths");
    mac.update(b"agent-desk/chatgpt-local/v1\n");
    for field in [
        mode,
        &challenge.profile,
        &challenge.port.to_string(),
        challenge.scope.name(),
        &challenge.nonce,
    ] {
        mac.update(field.as_bytes());
        mac.update(b"\n");
    }
    mac.update(&Sha256::digest(payload));
    mac.finalize()
        .into_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub(crate) fn matches_server(hash: &[u8], challenge: &Challenge, response: &ServerProof) -> bool {
    matches_proof(
        &proof(hash, "server", challenge, &[]),
        &response.server_proof,
    )
}

pub(crate) fn request_proof(hash: &[u8], challenge: &Challenge, payload: &[u8]) -> String {
    format!(
        "{PREFIX}{}_{}",
        challenge.nonce,
        proof(hash, "request", challenge, payload)
    )
}

fn matches_proof(expected: &str, supplied: &str) -> bool {
    hex_of(supplied, 64) && bool::from(expected.as_bytes().ct_eq(supplied.as_bytes()))
}

pub(crate) fn handles(path: &str) -> bool {
    path == CHALLENGE_PATH || Scope::from_path(path).is_some()
}

pub(crate) fn body_limit(path: &str) -> usize {
    match path {
        CHALLENGE_PATH => 1024,
        TODO_PATH => MAX_TODO_BODY,
        _ => MAX_CODEX_BODY,
    }
}

impl ChatGpt {
    pub(crate) fn new(
        pool: SqlitePool,
        deliveries: DeliveryService,
        proposals: ProposalService,
        inbox_changed: Signal,
        todo_changed: Signal,
    ) -> Self {
        Self {
            pool,
            deliveries,
            proposals,
            inbox_changed,
            todo_changed,
            credentials: Arc::new(OsCredentialStore),
            profile: profile(),
            gate: Mutex::new(()),
            tickets: Mutex::new(HashMap::new()),
        }
    }

    async fn grant(&self, scope: Scope) -> Result<Option<Grant>, CodexError> {
        sqlx::query_as("SELECT dedupe_namespace, token_hash, last_used_at FROM chatgpt_desktop_grants WHERE scope=?")
            .bind(scope.name()).fetch_optional(&self.pool).await.map_err(|_| CodexError::Unavailable)
    }

    pub(crate) async fn active(&self) -> Result<bool, CodexError> {
        sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM chatgpt_desktop_grants WHERE token_hash IS NOT NULL)",
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|_| CodexError::Unavailable)
    }

    async fn os_call<T: Send + 'static>(
        &self,
        scope: Scope,
        call: impl FnOnce(
                &dyn CredentialPort,
                &str,
            )
                -> Result<T, crate::application::ports::credentials::CredentialError>
            + Send
            + 'static,
    ) -> Result<T, String> {
        let store = self.credentials.clone();
        let name = reference(&self.profile, scope);
        tokio::task::spawn_blocking(move || call(store.as_ref(), &name))
            .await
            .map_err(|_| credential_error())?
            .map_err(|_| credential_error())
    }

    pub(crate) async fn status(&self, bridge_running: bool) -> Result<Status, String> {
        let _lock = self.gate.lock().await;
        let reading = self
            .grant(Scope::SendReading)
            .await
            .map_err(|_| store_error())?;
        let todo = self
            .grant(Scope::SuggestTodos)
            .await
            .map_err(|_| store_error())?;
        let enabled_read = reading.as_ref().is_some_and(|g| g.token_hash.is_some());
        let enabled_todo = todo.as_ref().is_some_and(|g| g.token_hash.is_some());
        let mut needs_repair = false;
        for (scope, grant) in [(Scope::SendReading, &reading), (Scope::SuggestTodos, &todo)] {
            if let Some(hash) = grant.as_ref().and_then(|g| g.token_hash.as_ref()) {
                needs_repair |= !self
                    .os_call(scope, |store, name| store.read(name))
                    .await
                    .is_ok_and(|secret| {
                        secret.is_some_and(|value| bool::from(hash.ct_eq(&digest(&value))))
                    });
            }
        }
        let last_used_at = [reading.as_ref(), todo.as_ref()]
            .into_iter()
            .flatten()
            .filter_map(|grant| grant.last_used_at.as_ref())
            .max()
            .cloned();
        let phase = match (enabled_read, enabled_todo) {
            (false, false) if reading.is_none() && todo.is_none() => "disconnected",
            (false, false) => "revoked",
            _ if !bridge_running || needs_repair => "error",
            _ if last_used_at.is_none() => "awaiting_first_call",
            (true, true) => "connected",
            _ => "partial",
        };
        Ok(Status {
            phase,
            reading: enabled_read,
            todo: enabled_todo,
            last_used_at,
        })
    }

    pub(crate) async fn configure(&self, scopes: Scopes, rotate: bool) -> Result<(), String> {
        let _lock = self.gate.lock().await;
        self.clear_tickets().await;
        // Disable all selected old grants before touching the OS store. A
        // failure deleting one credential must not leave the other authorized.
        sqlx::query(
            "UPDATE chatgpt_desktop_grants
            SET token_hash=NULL,last_used_at=NULL,updated_at=CURRENT_TIMESTAMP
            WHERE token_hash IS NOT NULL AND
                (? OR (scope='send_reading' AND NOT ?) OR (scope='suggest_todos' AND NOT ?))",
        )
        .bind(rotate)
        .bind(scopes.reading)
        .bind(scopes.todo)
        .execute(&self.pool)
        .await
        .map_err(|_| store_error())?;
        let mut cleanup_failed = false;
        for (scope, desired) in [
            (Scope::SendReading, scopes.reading),
            (Scope::SuggestTodos, scopes.todo),
        ] {
            let previous = self.grant(scope).await.map_err(|_| store_error())?;
            let was_enabled = previous
                .as_ref()
                .is_some_and(|row| row.token_hash.is_some());
            if desired && was_enabled && !rotate {
                if let Ok(Some(existing)) =
                    self.os_call(scope, |store, name| store.read(name)).await
                {
                    if previous
                        .as_ref()
                        .and_then(|g| g.token_hash.as_ref())
                        .is_some_and(|hash| bool::from(hash.ct_eq(&digest(&existing))))
                    {
                        continue;
                    }
                }
            }
            if previous.is_some() {
                sqlx::query("UPDATE chatgpt_desktop_grants SET token_hash=NULL,last_used_at=NULL,updated_at=CURRENT_TIMESTAMP WHERE scope=?")
                    .bind(scope.name()).execute(&self.pool).await.map_err(|_| store_error())?;
            }
            if !desired {
                if previous.is_some() {
                    cleanup_failed |= self
                        .os_call(scope, |store, name| store.delete(name))
                        .await
                        .is_err();
                }
                continue;
            }
            let secret = format!("{}{}", nonce(), nonce());
            let hash = digest(&secret);
            let result = self.os_call(scope, move |store, name| {
                store.store(name, &secret)?;
                if store.read(name)?.as_deref() != Some(secret.as_str()) {
                    return Err(crate::application::ports::credentials::CredentialError::OperationFailed);
                }
                Ok(())
            }).await;
            if let Err(error) = result {
                let _ = self.os_call(scope, |store, name| store.delete(name)).await;
                return Err(error);
            }
            let saved = sqlx::query(
                "INSERT INTO chatgpt_desktop_grants (scope,dedupe_namespace,token_hash,updated_at)
                 VALUES(?,?,?,CURRENT_TIMESTAMP) ON CONFLICT(scope) DO UPDATE
                 SET token_hash=excluded.token_hash,updated_at=excluded.updated_at,last_used_at=NULL",
            ).bind(scope.name()).bind(nonce()).bind(hash).execute(&self.pool).await;
            if saved.is_err() {
                let _ = self.os_call(scope, |store, name| store.delete(name)).await;
                return Err(store_error());
            }
        }
        if cleanup_failed {
            Err(credential_error())
        } else {
            Ok(())
        }
    }

    #[cfg(test)]
    pub(crate) fn with_test_credentials(mut self, credentials: Arc<dyn CredentialPort>) -> Self {
        self.credentials = credentials;
        self.profile = "desktop-v1".to_owned();
        self
    }

    #[cfg(all(test, target_os = "windows"))]
    pub(crate) fn with_test_profile(mut self, isolated: String) -> Self {
        assert!(isolated.starts_with("audit-") && valid_profile(&isolated));
        self.profile = isolated;
        self
    }

    pub(crate) async fn clear_tickets(&self) {
        self.tickets.lock().await.clear();
    }

    async fn challenge(&self, port: u16, body: &[u8]) -> Result<Value, CodexError> {
        let challenge: Challenge =
            serde_json::from_slice(body).map_err(|_| CodexError::InvalidPayload)?;
        if challenge.profile != self.profile
            || challenge.port != port
            || !hex_of(&challenge.nonce, 32)
        {
            return Err(CodexError::Unauthorized);
        }
        let hash = self
            .grant(challenge.scope)
            .await?
            .and_then(|row| row.token_hash)
            .ok_or(CodexError::Forbidden)?;
        let mut tickets = self.tickets.lock().await;
        tickets.retain(|_, created| created.elapsed() < CHALLENGE_LIFETIME);
        let index = (challenge.scope, challenge.nonce.clone());
        if tickets.len() >= MAX_CHALLENGES || tickets.contains_key(&index) {
            return Err(CodexError::Unavailable);
        }
        tickets.insert(index, Instant::now());
        Ok(json!({ "server_proof": proof(&hash, "server", &challenge, &[]) }))
    }

    // The listener holds gate through authentication and commit. No Bearer
    // secret is transmitted; a proof is bound to a one-time ticket and body.
    pub(crate) async fn handle(
        &self,
        port: u16,
        method: &str,
        path: &str,
        authorization: Option<&str>,
        body: &[u8],
    ) -> Result<Value, CodexError> {
        if method != "POST" {
            return Err(CodexError::Forbidden);
        }
        if body.len() > body_limit(path) {
            return Err(CodexError::TooLarge);
        }
        if path == CHALLENGE_PATH {
            return self.challenge(port, body).await;
        }
        let scope = Scope::from_path(path).ok_or(CodexError::Forbidden)?;
        let row = self.grant(scope).await?.ok_or(CodexError::Forbidden)?;
        let hash = row.token_hash.ok_or(CodexError::Forbidden)?;
        let (n, tag) = authorization
            .and_then(|value| value.strip_prefix(PREFIX))
            .and_then(|value| value.split_once('_'))
            .ok_or(CodexError::Unauthorized)?;
        if !hex_of(n, 32) || !hex_of(tag, 64) {
            return Err(CodexError::Unauthorized);
        }
        let challenge = Challenge {
            profile: self.profile.clone(),
            port,
            scope,
            nonce: n.to_owned(),
        };
        let mut tickets = self.tickets.lock().await;
        if !tickets
            .get(&(scope, n.to_owned()))
            .is_some_and(|t| t.elapsed() < CHALLENGE_LIFETIME)
            || !matches_proof(&proof(&hash, "request", &challenge, body), tag)
        {
            return Err(CodexError::Unauthorized);
        }
        tickets.remove(&(scope, n.to_owned()));
        drop(tickets);
        let (receipt, created) = self.submit(scope, &row.dedupe_namespace, body).await?;
        let _ = sqlx::query(
            "UPDATE chatgpt_desktop_grants SET last_used_at=CURRENT_TIMESTAMP WHERE scope=?",
        )
        .bind(scope.name())
        .execute(&self.pool)
        .await;
        if created {
            match scope {
                Scope::SendReading => (self.inbox_changed)(),
                Scope::SuggestTodos => (self.todo_changed)(),
            }
        }
        Ok(receipt)
    }

    pub(crate) async fn reserve_legacy_key(&self, key: &str) -> Result<(), CodexError> {
        if let Some(row) = self.grant(Scope::SendReading).await? {
            if key
                .trim()
                .starts_with(&format!("chatgpt-reading:{}:", row.dedupe_namespace))
            {
                return Err(CodexError::Forbidden);
            }
        }
        Ok(())
    }

    async fn submit(
        &self,
        scope: Scope,
        space: &str,
        body: &[u8],
    ) -> Result<(Value, bool), CodexError> {
        match scope {
            Scope::SendReading => {
                let request: ReadingRequest =
                    serde_json::from_slice(body).map_err(|_| CodexError::InvalidPayload)?;
                request.validate()?;
                let key = request
                    .idempotency_key
                    .as_deref()
                    .ok_or(CodexError::InvalidPayload)?
                    .trim()
                    .to_owned();
                let key_digest = Sha256::digest(key.as_bytes());
                let saved = self
                    .deliveries
                    .ingest(request.into_delivery(
                        format!("chatgpt-reading:{space}:{key_digest:x}"),
                        "ChatGPT",
                    ))
                    .await
                    .map_err(|error| match error {
                        DeliveryServiceError::Validation(_) => CodexError::InvalidPayload,
                        DeliveryServiceError::Repository(
                            DeliveryRepositoryError::IdempotencyConflict,
                        ) => CodexError::Conflict,
                        _ => CodexError::Unavailable,
                    })?;
                Ok((
                    json!({"version":"v1","operation":scope.name(),"request_id":key,
                    "delivery_id":saved.item.delivery.id,"document_id":saved.item.document.id,"created":saved.created}),
                    saved.created,
                ))
            }
            Scope::SuggestTodos => {
                let request: TodoSuggestionRequest =
                    serde_json::from_slice(body).map_err(|_| CodexError::InvalidPayload)?;
                let key = request.idempotency_key.ok_or(CodexError::InvalidPayload)?;
                let (batch, created) = self
                    .proposals
                    .submit_todo_batch(space, &key, request.items)
                    .await
                    .map_err(|error| match error {
                        ProposalServiceError::Repository(
                            ProposalRepositoryError::TodoBatchConflict,
                        ) => CodexError::Conflict,
                        ProposalServiceError::Repository(_) => CodexError::Unavailable,
                        _ => CodexError::InvalidPayload,
                    })?;
                Ok((
                    json!({"version":"v1","operation":scope.name(),"request_id":key,
                    "batch_id":batch.id,"status":batch.status,"created":created}),
                    created,
                ))
            }
        }
    }
}

fn store_error() -> String {
    "无法保存 ChatGPT 授权，请重试。".to_owned()
}
fn credential_error() -> String {
    "系统凭据库不可用。请解锁系统凭据库，再管理 ChatGPT 授权。".to_owned()
}
