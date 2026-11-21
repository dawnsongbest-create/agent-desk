use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use serde::de::DeserializeOwned;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    sync::oneshot,
    task::JoinHandle,
    time::timeout,
};

use super::{
    dumate::{DuMate, TOKEN_PREFIX as DUMATE_PREFIX},
    mcp_http,
};

#[cfg(test)]
#[path = "dumate_http_tests.rs"]
mod dumate_http_tests;

use crate::{
    adapters::openclaw::{
        OpenClawAdapter, OpenClawAdapterError, OpenClawDeliveryRequest, OpenClawProposalRequest,
        OpenClawReadingPlanRequest, OpenClawResponse,
    },
    application::{
        agent_connection_service::{AgentConnectionService, AuthenticatedAgentConnection},
        capability_service::CapabilityService,
    },
    chatgpt::{self, ChatGpt},
    codex_delivery::{CodexDelivery, CodexError, CODEX_TOKEN_PREFIX, READING_PATH},
    domain::agent_connection::AGENT_API_VERSION,
};

const MAX_HEADER_BYTES: usize = 8 * 1024;
const MAX_BODY_BYTES: usize = 4 * 1024 * 1024;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
const REJECTED_BODY_DRAIN_TIMEOUT: Duration = Duration::from_millis(500);

pub struct RunningAgentBridge {
    local_addr: SocketAddr,
    shutdown: oneshot::Sender<()>,
    task: JoinHandle<()>,
    accepting: Arc<AtomicBool>,
    dumate: Arc<DuMate>,
    chatgpt: Option<Arc<ChatGpt>>,
    legacy_enabled: Arc<AtomicBool>,
}

impl RunningAgentBridge {
    pub async fn start(
        port: u16,
        connections: Arc<AgentConnectionService>,
        capabilities: Arc<CapabilityService>,
        openclaw: Arc<OpenClawAdapter>,
        codex: Arc<CodexDelivery>,
        dumate: Arc<DuMate>,
        chatgpt: Option<(Arc<ChatGpt>, bool)>,
    ) -> Result<Self, std::io::Error> {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await?;
        let local_addr = listener.local_addr()?;
        let (shutdown, mut shutdown_receiver) = oneshot::channel();
        let accepting = Arc::new(AtomicBool::new(true));
        let listener_admission = accepting.clone();
        let listener_dumate = dumate.clone();
        let legacy_enabled = Arc::new(AtomicBool::new(chatgpt.as_ref().is_none_or(|(_, on)| *on)));
        let chatgpt = chatgpt.map(|(service, _)| service);
        let listener_chatgpt = chatgpt.clone();
        let listener_legacy = legacy_enabled.clone();
        let task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut shutdown_receiver => break,
                    accepted = listener.accept() => {
                        let Ok((stream, peer)) = accepted else { break };
                        if !peer.ip().is_loopback() {
                            continue;
                        }
                        let connections = connections.clone();
                        let capabilities = capabilities.clone();
                        let openclaw = openclaw.clone();
                        let codex = codex.clone();
                        let dumate = listener_dumate.clone();
                        let accepting = listener_admission.clone();
                        let chatgpt = listener_chatgpt.clone();
                        let legacy_enabled = listener_legacy.clone();
                        tokio::spawn(async move {
                            let _ = handle_connection(stream, connections, capabilities, openclaw, codex, dumate, accepting, chatgpt, legacy_enabled).await;
                        });
                    }
                }
            }
        });
        Ok(Self {
            local_addr,
            shutdown,
            task,
            accepting,
            dumate,
            chatgpt,
            legacy_enabled,
        })
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    pub fn legacy_running(&self) -> bool {
        self.legacy_enabled.load(Ordering::Acquire)
    }

    pub fn enable_legacy(&self) {
        self.legacy_enabled.store(true, Ordering::Release);
    }

    pub async fn disable_legacy(&self) {
        self.legacy_enabled.store(false, Ordering::Release);
        drop(self.dumate.gate.lock().await);
    }

    pub async fn stop(self) {
        self.accepting.store(false, Ordering::Release);
        self.legacy_enabled.store(false, Ordering::Release);
        let _ = self.shutdown.send(());
        let _ = self.task.await;
        drop(self.dumate.gate.lock().await);
        if let Some(chatgpt) = &self.chatgpt {
            let _gate = chatgpt.gate.lock().await;
            chatgpt.clear_tickets().await;
        }
    }
}

struct HttpRequest {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl HttpRequest {
    fn header(&self, expected: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(expected))
            .map(|(_, value)| value.as_str())
    }

    fn bearer_token(&self) -> Option<&str> {
        let value = self.header("authorization")?.trim();
        let (scheme, token) = value.split_once(' ')?;
        (scheme.eq_ignore_ascii_case("bearer") && !token.trim().is_empty()).then_some(token.trim())
    }

    fn json<T: DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_slice(&self.body)
    }
}

#[allow(clippy::too_many_arguments)]
async fn handle_connection(
    mut stream: TcpStream,
    connections: Arc<AgentConnectionService>,
    capabilities: Arc<CapabilityService>,
    openclaw: Arc<OpenClawAdapter>,
    codex: Arc<CodexDelivery>,
    dumate: Arc<DuMate>,
    accepting: Arc<AtomicBool>,
    chatgpt: Option<Arc<ChatGpt>>,
    legacy_enabled: Arc<AtomicBool>,
) -> Result<(), std::io::Error> {
    let request = match timeout(REQUEST_TIMEOUT, read_request(&mut stream)).await {
        Ok(Ok(RequestRead::Complete(request))) => request,
        Ok(Ok(RequestRead::Rejected {
            status,
            remaining,
            mcp,
        })) => {
            let (_, body) = mcp_http::http_error(status);
            write_http_response(&mut stream, status, &body, mcp).await?;
            // The response has been sent and the write half closed. Drain a
            // bounded amount so macOS doesn't reset a socket with unread data.
            // Never parse or retain the over-limit body, or wait on a slow peer.
            let mut discarded = (&mut stream).take(remaining as u64);
            let mut sink = tokio::io::sink();
            let _ = timeout(
                REJECTED_BODY_DRAIN_TIMEOUT,
                tokio::io::copy(&mut discarded, &mut sink),
            )
            .await;
            return Ok(());
        }
        _ => {
            write_response(&mut stream, 400, r#"{"error":"invalid_request"}"#).await?;
            return Ok(());
        }
    };

    if chatgpt::handles(&request.path) {
        let (status, body) = if let Some(service) = &chatgpt {
            let _gate = service.gate.lock().await;
            if !accepting.load(Ordering::Acquire) {
                codex_error(CodexError::Unavailable)
            } else if request.header("content-type") != Some("application/json") {
                invalid_payload()
            } else {
                match service
                    .handle(
                        stream.local_addr()?.port(),
                        &request.method,
                        &request.path,
                        request.bearer_token(),
                        &request.body,
                    )
                    .await
                {
                    Ok(value) => serialize_ok(&value),
                    Err(error) => codex_error(error),
                }
            }
        } else {
            (404, r#"{"error":"not_found"}"#.to_owned())
        };
        return write_response(&mut stream, status, &body).await;
    }
    if !legacy_enabled.load(Ordering::Acquire) {
        return write_response(&mut stream, 403, r#"{"error":"forbidden_capability"}"#).await;
    }
    if request.path == mcp_http::PATH {
        let (status, body) = if dumate.preview_available {
            mcp_http::respond(
                &dumate,
                &legacy_enabled,
                &request.method,
                &request.headers,
                &request.body,
            )
            .await
        } else {
            (404, r#"{"error":"not_found"}"#.to_owned())
        };
        return write_http_response(&mut stream, status, &body, true).await;
    }
    let token = request.bearer_token().unwrap_or_default();
    if token.starts_with(DUMATE_PREFIX) || token.starts_with(chatgpt::PREFIX) {
        write_response(&mut stream, 401, r#"{"error":"unauthorized"}"#).await?;
        return Ok(());
    }
    if token.starts_with(CODEX_TOKEN_PREFIX) {
        let (status, body) = match codex
            .dispatch(token, &request.method, &request.path, &request.body)
            .await
        {
            Ok(receipt) => (200, receipt),
            Err(error) => codex_error(error),
        };
        return write_response(&mut stream, status, &body).await;
    }
    let identity = match connections.authenticate_identity(token).await {
        Ok(Some(identity)) => identity,
        Ok(None) => {
            write_response(&mut stream, 401, r#"{"error":"unauthorized"}"#).await?;
            return Ok(());
        }
        Err(_) => {
            write_response(&mut stream, 503, r#"{"error":"bridge_unavailable"}"#).await?;
            return Ok(());
        }
    };

    let (status, body) = route_request(
        &request,
        &identity,
        &capabilities,
        &openclaw,
        &codex,
        &dumate,
        chatgpt.as_deref(),
    )
    .await;
    write_response(&mut stream, status, &body).await
}

async fn route_request(
    request: &HttpRequest,
    identity: &AuthenticatedAgentConnection,
    capabilities: &CapabilityService,
    openclaw: &OpenClawAdapter,
    codex: &CodexDelivery,
    dumate: &DuMate,
    chatgpt: Option<&ChatGpt>,
) -> (u16, String) {
    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/api/v1/health") => (200, r#"{"version":"v1","status":"ok"}"#.to_owned()),
        ("GET", "/api/v1/capabilities") => serialize_ok(&capabilities.discover()),
        (_, READING_PATH | crate::codex_delivery::TODO_PATH) => codex_error(CodexError::Forbidden),
        ("POST", "/api/v1/delivery") => {
            let Ok(input) = request.json::<OpenClawDeliveryRequest>() else {
                return invalid_payload();
            };
            if let Err(error) = codex
                .reserve_legacy_key(input.idempotency_key.as_deref())
                .await
            {
                return codex_error(error);
            }
            // Serialize lookup of the reserved DuMate namespace with first
            // pairing. Ordinary legacy sends never acquire the DuMate lock.
            let _reserved_key_guard = if input
                .idempotency_key
                .as_deref()
                .is_some_and(|key| key.trim_start().starts_with("dumate-reading:"))
            {
                let guard = dumate.gate.lock().await;
                if let Err(error) = dumate
                    .reserve_legacy_key(input.idempotency_key.as_deref())
                    .await
                {
                    return codex_error(error);
                }
                Some(guard)
            } else {
                None
            };
            let _chatgpt_key_guard =
                if let (Some(service), Some(key)) = (chatgpt, input.idempotency_key.as_deref()) {
                    if key.trim_start().starts_with("chatgpt-reading:") {
                        let guard = service.gate.lock().await;
                        if let Err(error) = service.reserve_legacy_key(key).await {
                            return codex_error(error);
                        }
                        Some(guard)
                    } else {
                        None
                    }
                } else {
                    None
                };
            adapter_result(identity, openclaw, openclaw.create_delivery(input).await)
        }
        ("POST", "/api/v1/proposal") => {
            let Ok(input) = request.json::<OpenClawProposalRequest>() else {
                return invalid_payload();
            };
            adapter_result(identity, openclaw, openclaw.create_proposal(input).await)
        }
        ("POST", "/api/v1/reading-plan") => {
            let Ok(input) = request.json::<OpenClawReadingPlanRequest>() else {
                return invalid_payload();
            };
            adapter_result(
                identity,
                openclaw,
                openclaw.create_reading_plan(input).await,
            )
        }
        ("POST", "/api/v1/todo" | "/api/v1/record" | "/api/v1/sticky") => {
            (403, r#"{"error":"forbidden_capability"}"#.to_owned())
        }
        ("GET" | "POST", path) if path.starts_with("/api/v1/") => {
            (404, r#"{"error":"not_found"}"#.to_owned())
        }
        (
            _,
            "/api/v1/health"
            | "/api/v1/capabilities"
            | "/api/v1/delivery"
            | "/api/v1/proposal"
            | "/api/v1/reading-plan",
        ) => (405, r#"{"error":"method_not_allowed"}"#.to_owned()),
        _ => (404, r#"{"error":"not_found"}"#.to_owned()),
    }
}

fn codex_error(error: CodexError) -> (u16, String) {
    let (status, body) = error.http();
    (status, body.to_owned())
}

fn adapter_result<T: serde::Serialize>(
    identity: &AuthenticatedAgentConnection,
    openclaw: &OpenClawAdapter,
    result: Result<T, OpenClawAdapterError>,
) -> (u16, String) {
    match result {
        Ok(data) => serialize_ok(&OpenClawResponse {
            version: AGENT_API_VERSION,
            adapter: openclaw.adapter_id(),
            agent_connection_id: identity.id.clone(),
            data,
        }),
        Err(OpenClawAdapterError::InvalidPayload) => invalid_payload(),
        Err(OpenClawAdapterError::Conflict) => {
            (409, r#"{"error":"idempotency_conflict"}"#.to_owned())
        }
        Err(OpenClawAdapterError::Unavailable) => {
            (503, r#"{"error":"bridge_unavailable"}"#.to_owned())
        }
    }
}

fn serialize_ok<T: serde::Serialize>(value: &T) -> (u16, String) {
    match serde_json::to_string(value) {
        Ok(body) => (200, body),
        Err(_) => (503, r#"{"error":"bridge_unavailable"}"#.to_owned()),
    }
}

fn invalid_payload() -> (u16, String) {
    (400, r#"{"error":"invalid_payload"}"#.to_owned())
}

enum RequestRead {
    Complete(HttpRequest),
    Rejected {
        status: u16,
        remaining: usize,
        mcp: bool,
    },
}

async fn read_request(stream: &mut TcpStream) -> Result<RequestRead, std::io::Error> {
    let mut bytes = Vec::with_capacity(1024);
    let mut buffer = [0_u8; 4096];
    let header_end = loop {
        let read = stream.read(&mut buffer).await?;
        if read == 0 {
            return Err(invalid_data("request ended before headers"));
        }
        bytes.extend_from_slice(&buffer[..read]);
        if let Some(position) = find_header_end(&bytes) {
            break position;
        }
        if bytes.len() >= MAX_HEADER_BYTES {
            return Err(invalid_data("request headers are too large"));
        }
    };
    if header_end > MAX_HEADER_BYTES {
        return Err(invalid_data("request headers are too large"));
    }

    let header_text = std::str::from_utf8(&bytes[..header_end])
        .map_err(|_| invalid_data("request headers are not UTF-8"))?;
    let mut lines = header_text.split("\r\n");
    let mut request_line = lines
        .next()
        .ok_or_else(|| invalid_data("request line is missing"))?
        .split_whitespace();
    let method = request_line.next().unwrap_or_default().to_owned();
    let path = request_line.next().unwrap_or_default().to_owned();
    let version = request_line.next().unwrap_or_default();
    if method.is_empty()
        || path.is_empty()
        || version != "HTTP/1.1"
        || request_line.next().is_some()
    {
        return Err(invalid_data("request line is invalid"));
    }

    let mut headers = Vec::new();
    let mcp = path == mcp_http::PATH;
    let chat = chatgpt::handles(&path);
    for line in lines {
        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| invalid_data("request header is invalid"))?;
        if (mcp || chat)
            && (name.is_empty()
                || !name
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte))
                || value
                    .bytes()
                    .any(|byte| byte.is_ascii_control() && byte != b'\t'))
        {
            return Ok(RequestRead::Rejected {
                status: 400,
                remaining: 0,
                mcp,
            });
        }
        headers.push((name.trim().to_owned(), value.trim().to_owned()));
    }
    let body_start = header_end + 4;
    if mcp || chat {
        if let Err(status) = mcp_http::validate_headers(&headers, stream.local_addr()?) {
            return Ok(RequestRead::Rejected {
                status,
                remaining: 0,
                mcp,
            });
        }
        if let Some(transfer) = mcp_http::header(&headers, "transfer-encoding") {
            if !mcp
                || !transfer.eq_ignore_ascii_case("chunked")
                || mcp_http::header(&headers, "content-length").is_some()
                || method != "POST"
            {
                return Ok(RequestRead::Rejected {
                    status: 400,
                    remaining: 0,
                    mcp,
                });
            }
            let read = std::io::Cursor::new(&bytes[body_start..]).chain(&mut *stream);
            let body = match read_mcp_chunks(BufReader::new(read)).await? {
                Some(body) => body,
                None => {
                    return Ok(RequestRead::Rejected {
                        status: 413,
                        remaining: MAX_BODY_BYTES,
                        mcp,
                    })
                }
            };
            return Ok(RequestRead::Complete(HttpRequest {
                method,
                path,
                headers,
                body,
            }));
        }
        if mcp_http::header(&headers, "content-length")
            .is_some_and(|len| len.is_empty() || !len.bytes().all(|byte| byte.is_ascii_digit()))
        {
            return Ok(RequestRead::Rejected {
                status: 400,
                remaining: 0,
                mcp,
            });
        }
    }
    if headers
        .iter()
        .any(|(name, _)| name.eq_ignore_ascii_case("transfer-encoding"))
    {
        return Err(invalid_data("transfer encoding is not supported"));
    }
    let content_length = headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .map(|(_, value)| value.parse::<usize>())
        .transpose()
        .map_err(|_| invalid_data("content length is invalid"))?
        .unwrap_or(0);
    if (mcp && content_length > mcp_http::MAX_BODY)
        || (chat && content_length > chatgpt::body_limit(&path))
        || (path == crate::codex_delivery::TODO_PATH
            && content_length > crate::domain::proposal::todo_batch::MAX_TODO_BODY)
    {
        return Ok(RequestRead::Rejected {
            status: 413,
            remaining: content_length
                .saturating_sub(bytes.len() - body_start)
                .min(MAX_BODY_BYTES),
            mcp,
        });
    }
    if mcp && method != "POST" && content_length > 0 {
        return Ok(RequestRead::Rejected {
            status: 400,
            remaining: content_length,
            mcp,
        });
    }
    if content_length > MAX_BODY_BYTES {
        return Err(invalid_data("request body is too large"));
    }

    while bytes.len() < body_start + content_length {
        let read = stream.read(&mut buffer).await?;
        if read == 0 {
            return Err(invalid_data("request body is incomplete"));
        }
        bytes.extend_from_slice(&buffer[..read]);
    }
    let body = bytes[body_start..body_start + content_length].to_vec();
    Ok(RequestRead::Complete(HttpRequest {
        method,
        path,
        headers,
        body,
    }))
}

async fn read_mcp_chunks(
    mut reader: impl tokio::io::AsyncRead + Unpin,
) -> Result<Option<Vec<u8>>, std::io::Error> {
    let mut body = Vec::new();
    // Bound framing overhead as well as decoded bytes. No trailer fields or
    // framing extensions are needed for this origin-only JSON endpoint.
    for _ in 0..4096 {
        let mut length = String::with_capacity(16);
        loop {
            let next = reader.read_u8().await?;
            if next == b'\r' && reader.read_u8().await? == b'\n' {
                break;
            }
            if length.len() == 16 || !next.is_ascii_hexdigit() {
                return Err(invalid_data("invalid MCP chunk size"));
            }
            length.push(char::from(next));
        }
        let length = usize::from_str_radix(&length, 16)
            .map_err(|_| invalid_data("invalid MCP chunk size"))?;
        if length > mcp_http::MAX_BODY - body.len() {
            return Ok(None);
        }
        let start = body.len();
        body.resize(start + length, 0);
        reader.read_exact(&mut body[start..]).await?;
        if reader.read_u8().await? != b'\r' || reader.read_u8().await? != b'\n' {
            return Err(invalid_data("invalid MCP chunk ending"));
        }
        if length == 0 {
            return Ok(Some(body));
        }
    }
    Err(invalid_data("too many MCP chunks"))
}

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
}

fn invalid_data(message: &'static str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, message)
}

async fn write_response(
    stream: &mut TcpStream,
    status: u16,
    body: &str,
) -> Result<(), std::io::Error> {
    write_http_response(stream, status, body, false).await
}

async fn write_http_response(
    stream: &mut TcpStream,
    status: u16,
    body: &str,
    mcp: bool,
) -> Result<(), std::io::Error> {
    let reason = match status {
        200 => "OK",
        202 => "Accepted",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        406 => "Not Acceptable",
        409 => "Conflict",
        413 => "Payload Too Large",
        415 => "Unsupported Media Type",
        503 => "Service Unavailable",
        _ => "Error",
    };
    let extra = match (mcp, status) {
        (true, 405) => "Allow: POST\r\n",
        (true, 401) => "WWW-Authenticate: Bearer\r\n",
        _ => "",
    };
    let content_type = if status == 202 {
        ""
    } else {
        "Content-Type: application/json; charset=utf-8\r\n"
    };
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\n{extra}{content_type}Content-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes()).await?;
    stream.shutdown().await
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::{
        adapters::openclaw::OpenClawAdapter,
        application::{
            agent_connection_service::AgentConnectionService, delivery_service::DeliveryService,
            proposal_service::ProposalService, reading_service::ReadingService,
        },
        domain::agent_connection::AgentConnectionStatus,
        persistence::{
            agent_connection_repository::SqliteAgentConnectionRepository,
            delivery_repository::SqliteDeliveryRepository,
            proposal_repository::SqliteProposalRepository,
            reading_repository::SqliteReadingRepository, sqlite,
        },
    };

    async fn request(
        address: SocketAddr,
        method: &str,
        path: &str,
        token: Option<&str>,
        body: Option<&str>,
    ) -> String {
        let mut stream = TcpStream::connect(address).await.unwrap();
        let authorization = token
            .map(|token| format!("Authorization: Bearer {token}\r\n"))
            .unwrap_or_default();
        let body = body.unwrap_or_default();
        let request = format!(
            "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\n{authorization}Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(request.as_bytes()).await.unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).await.unwrap();
        response
    }

    fn response_json(response: &str) -> serde_json::Value {
        serde_json::from_str(response.split_once("\r\n\r\n").unwrap().1).unwrap()
    }

    #[tokio::test]
    async fn authenticates_routes_openclaw_and_blocks_direct_workspace_mutation() {
        let temp = tempfile::tempdir().unwrap();
        let database = sqlite::connect(&temp.path().join("bridge.sqlite3"))
            .await
            .unwrap();
        let connections = Arc::new(AgentConnectionService::new(Arc::new(
            SqliteAgentConnectionRepository::new(database.0.clone()),
        )));
        connections
            .ensure(AgentConnectionStatus::Active)
            .await
            .unwrap();
        let issued = connections.generate_token().await.unwrap();
        let delivery_repository = SqliteDeliveryRepository::new(database.0.clone());
        let proposal_repository = SqliteProposalRepository::new(database.0.clone());
        let reading_repository = SqliteReadingRepository::new(database.0.clone());
        let openclaw = Arc::new(OpenClawAdapter::new(
            DeliveryService::new(Arc::new(delivery_repository.clone())),
            ProposalService::new(Arc::new(proposal_repository.clone())),
            ReadingService::new(Arc::new(reading_repository), Arc::new(delivery_repository)),
        ));
        let dumate = Arc::new(DuMate::new(
            database.0.clone(),
            DeliveryService::new(Arc::new(SqliteDeliveryRepository::new(database.0.clone()))),
            ProposalService::new(Arc::new(proposal_repository.clone())),
            Arc::new(|| {}),
            Arc::new(|| {}),
        ));
        let codex = Arc::new(CodexDelivery::new(
            database.0.clone(),
            DeliveryService::new(Arc::new(SqliteDeliveryRepository::new(database.0.clone()))),
            ProposalService::new(Arc::new(proposal_repository)),
            Arc::new(|| {}),
            Arc::new(|| {}),
        ));
        let bridge = RunningAgentBridge::start(
            0,
            connections,
            Arc::new(CapabilityService),
            openclaw,
            codex,
            dumate,
            None,
        )
        .await
        .unwrap();
        let address = bridge.local_addr();
        assert_eq!(address.ip(), std::net::Ipv4Addr::LOCALHOST);
        let codex_secret = format!(
            "{CODEX_TOKEN_PREFIX}{}{}",
            uuid::Uuid::new_v4().simple(),
            uuid::Uuid::new_v4().simple()
        );
        let codex_hash = {
            use sha2::{Digest, Sha256};
            Sha256::digest(codex_secret.as_bytes()).to_vec()
        };
        sqlx::query(
            "INSERT INTO codex_connection
            (id, token_hash, state, scope, dedupe_namespace, created_at, updated_at)
            VALUES ('codex-reading', ?, 'active', 'send_reading', ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        )
        .bind(codex_hash)
        .bind(uuid::Uuid::new_v4().simple().to_string())
        .execute(&database.0)
        .await
        .unwrap();
        let reading = request(
            address, "POST", READING_PATH, Some(&codex_secret),
            Some(r##"{"title":"Codex HTTP fixture","content":"# 完整正文","type":"report","idempotency_key":"fixture-http"}"##),
        ).await;
        assert!(reading.starts_with("HTTP/1.1 200"));
        assert_eq!(response_json(&reading)["operation"], "send_reading");
        let todo_secret = format!(
            "{}{}{}",
            crate::codex_delivery::TODO_TOKEN_PREFIX,
            uuid::Uuid::new_v4().simple(),
            uuid::Uuid::new_v4().simple()
        );
        let todo_hash = {
            use sha2::{Digest, Sha256};
            Sha256::digest(todo_secret.as_bytes()).to_vec()
        };
        sqlx::query("INSERT INTO codex_todo_suggestion_grant VALUES
            ('codex-todo-suggestions', ?, 'active', 'suggest_todos', ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)")
            .bind(todo_hash).bind(uuid::Uuid::new_v4().simple().to_string())
            .execute(&database.0).await.unwrap();
        let todo_path = crate::codex_delivery::TODO_PATH;
        let todo_body = r#"{"items":[{"text":"HTTP fixture","priority":"low"}],"idempotency_key":"fixture-http"}"#;
        let boundary = format!(
            "{todo_body}{}",
            " ".repeat(crate::domain::proposal::todo_batch::MAX_TODO_BODY - todo_body.len())
        );
        let todo = request(
            address,
            "POST",
            todo_path,
            Some(&todo_secret),
            Some(&boundary),
        )
        .await;
        assert!(todo.starts_with("HTTP/1.1 200"));
        assert_eq!(response_json(&todo)["operation"], "suggest_todos");
        assert_eq!(response_json(&todo)["status"], "pending");
        let replay = request(
            address,
            "POST",
            todo_path,
            Some(&todo_secret),
            Some(todo_body),
        )
        .await;
        assert_eq!(
            response_json(&todo)["batch_id"],
            response_json(&replay)["batch_id"]
        );
        assert_eq!(response_json(&replay)["created"], false);
        assert!(request(
            address,
            "POST",
            todo_path,
            Some(&todo_secret),
            Some(&(boundary + " "))
        )
        .await
        .starts_with("HTTP/1.1 413"));
        // A sender can wait for the status before sending the declared body.
        // The server must not wait for the oversized body before replying.
        let mut waiting = TcpStream::connect(address).await.unwrap();
        let headers = format!(
            "POST {todo_path} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {}\r\n\r\n",
            MAX_BODY_BYTES + 1
        );
        waiting.write_all(headers.as_bytes()).await.unwrap();
        let mut refusal = String::new();
        timeout(Duration::from_secs(2), waiting.read_to_string(&mut refusal))
            .await
            .unwrap()
            .unwrap();
        assert!(refusal.starts_with("HTTP/1.1 413"));
        drop(waiting);
        assert!(request(address, "POST", todo_path, Some(&todo_secret),
            Some(r#"{"items":[{"text":"valid","priority":"low"},{"text":"invalid","priority":"urgent"}],"idempotency_key":"rejected-input"}"#)
        ).await.starts_with("HTTP/1.1 400"));
        let counts: (i64, i64) = sqlx::query_as(
            "SELECT
            (SELECT COUNT(*) FROM todo_suggestion_batches), (SELECT COUNT(*) FROM task_payloads)",
        )
        .fetch_one(&database.0)
        .await
        .unwrap();
        assert_eq!(counts, (1, 0));
        for bearer in [&issued.token, &codex_secret] {
            assert!(request(
                address,
                "POST",
                crate::codex_delivery::TODO_PATH,
                Some(bearer),
                Some("{}")
            )
            .await
            .starts_with("HTTP/1.1 403"));
        }
        assert!(request(
            address,
            "POST",
            crate::codex_delivery::TODO_PATH,
            None,
            Some("{}")
        )
        .await
        .starts_with("HTTP/1.1 401"));
        assert!(request(
            address,
            "POST",
            READING_PATH,
            Some(&issued.token),
            Some("{}")
        )
        .await
        .starts_with("HTTP/1.1 403"));
        assert!(request(
            address,
            "POST",
            READING_PATH,
            Some("adk_codex_invalid"),
            Some("{}")
        )
        .await
        .starts_with("HTTP/1.1 401"));
        for endpoint in [
            "/api/v1/delivery",
            "/api/v1/proposal",
            "/api/v1/reading-plan",
            "/api/v1/todo",
            "/api/v1/record",
            "/api/v1/sticky",
        ] {
            assert!(
                request(address, "POST", endpoint, Some(&codex_secret), Some("{}"))
                    .await
                    .starts_with("HTTP/1.1 403")
            );
        }

        let missing = request(address, "GET", "/api/v1/health", None, None).await;
        let invalid = request(address, "GET", "/api/v1/capabilities", Some("wrong"), None).await;
        let health = request(address, "GET", "/api/v1/health", Some(&issued.token), None).await;
        let capabilities = request(
            address,
            "GET",
            "/api/v1/capabilities",
            Some(&issued.token),
            None,
        )
        .await;
        let delivery = request(
            address,
            "POST",
            "/api/v1/delivery",
            Some(&issued.token),
            Some(r#"{"title":"AI Daily Research","content":"今日 AI 动态","type":"research","idempotency_key":"openclaw-demo"}"#),
        )
        .await;
        let delivery_id = response_json(&delivery)["data"]["item"]["delivery"]["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let proposal_body = serde_json::json!({
            "type": "READING",
            "title": "深入阅读 Transformer Architecture",
            "description": "建议阅读核心架构并形成学习记录。",
            "payload": { "estimated_minutes": 15 },
            "source_delivery_id": delivery_id,
        })
        .to_string();
        let proposal = request(
            address,
            "POST",
            "/api/v1/proposal",
            Some(&issued.token),
            Some(&proposal_body),
        )
        .await;
        let reading = request(
            address,
            "POST",
            "/api/v1/reading-plan",
            Some(&issued.token),
            Some(r#"{"title":"Transformer Study","daily_minutes":10}"#),
        )
        .await;
        let invalid_identity = request(
            address,
            "POST",
            "/api/v1/reading-plan",
            Some(&issued.token),
            Some(r#"{"title":"Transformer Study","daily_minutes":10,"agent_id":"spoofed"}"#),
        )
        .await;
        let forbidden = request(
            address,
            "POST",
            "/api/v1/todo",
            Some(&issued.token),
            Some(r#"{"content":"direct mutation"}"#),
        )
        .await;

        assert!(missing.starts_with("HTTP/1.1 401"));
        assert!(invalid.starts_with("HTTP/1.1 401"));
        assert!(health.starts_with("HTTP/1.1 200"));
        assert!(capabilities.starts_with("HTTP/1.1 200"));
        assert!(capabilities.contains(r#""delivery":true"#));
        assert!(capabilities.contains(r#""proposal":true"#));
        assert!(capabilities.contains(r#""reading":true"#));
        assert!(delivery.starts_with("HTTP/1.1 200"));
        assert!(delivery.contains(r#""adapter":"openclaw""#));
        assert!(delivery.contains(r#""title":"AI Daily Research""#));
        assert!(proposal.starts_with("HTTP/1.1 200"));
        assert!(proposal.contains(r#""type":"reading""#));
        assert!(reading.starts_with("HTTP/1.1 200"));
        assert!(reading.contains(r#""title":"Transformer Study""#));
        assert!(invalid_identity.starts_with("HTTP/1.1 400"));
        assert!(forbidden.starts_with("HTTP/1.1 403"));

        bridge.stop().await;
        assert!(TcpStream::connect(address).await.is_err());
    }
}
