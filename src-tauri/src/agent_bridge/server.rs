use std::{net::SocketAddr, sync::Arc, time::Duration};

use serde::de::DeserializeOwned;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::oneshot,
    task::JoinHandle,
    time::timeout,
};

use crate::{
    adapters::openclaw::{
        OpenClawAdapter, OpenClawAdapterError, OpenClawDeliveryRequest, OpenClawProposalRequest,
        OpenClawReadingPlanRequest, OpenClawResponse,
    },
    application::{
        agent_connection_service::{AgentConnectionService, AuthenticatedAgentConnection},
        capability_service::CapabilityService,
    },
    domain::agent_connection::AGENT_API_VERSION,
};

const MAX_HEADER_BYTES: usize = 8 * 1024;
const MAX_BODY_BYTES: usize = 4 * 1024 * 1024;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

pub struct RunningAgentBridge {
    local_addr: SocketAddr,
    shutdown: oneshot::Sender<()>,
    task: JoinHandle<()>,
}

impl RunningAgentBridge {
    pub async fn start(
        port: u16,
        connections: Arc<AgentConnectionService>,
        capabilities: Arc<CapabilityService>,
        openclaw: Arc<OpenClawAdapter>,
    ) -> Result<Self, std::io::Error> {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await?;
        let local_addr = listener.local_addr()?;
        let (shutdown, mut shutdown_receiver) = oneshot::channel();
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
                        tokio::spawn(async move {
                            let _ = handle_connection(stream, connections, capabilities, openclaw).await;
                        });
                    }
                }
            }
        });
        Ok(Self {
            local_addr,
            shutdown,
            task,
        })
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    pub async fn stop(self) {
        let _ = self.shutdown.send(());
        let _ = self.task.await;
    }
}

#[derive(Debug)]
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

async fn handle_connection(
    mut stream: TcpStream,
    connections: Arc<AgentConnectionService>,
    capabilities: Arc<CapabilityService>,
    openclaw: Arc<OpenClawAdapter>,
) -> Result<(), std::io::Error> {
    let request = match timeout(REQUEST_TIMEOUT, read_request(&mut stream)).await {
        Ok(Ok(request)) => request,
        _ => {
            write_response(&mut stream, 400, r#"{"error":"invalid_request"}"#).await?;
            return Ok(());
        }
    };

    let token = request.bearer_token().unwrap_or_default();
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

    let (status, body) = route_request(&request, &identity, &capabilities, &openclaw).await;
    write_response(&mut stream, status, &body).await
}

async fn route_request(
    request: &HttpRequest,
    identity: &AuthenticatedAgentConnection,
    capabilities: &CapabilityService,
    openclaw: &OpenClawAdapter,
) -> (u16, String) {
    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/api/v1/health") => (200, r#"{"version":"v1","status":"ok"}"#.to_owned()),
        ("GET", "/api/v1/capabilities") => serialize_ok(&capabilities.discover()),
        ("POST", "/api/v1/delivery") => {
            let Ok(input) = request.json::<OpenClawDeliveryRequest>() else {
                return invalid_payload();
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

async fn read_request(stream: &mut TcpStream) -> Result<HttpRequest, std::io::Error> {
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
    for line in lines {
        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| invalid_data("request header is invalid"))?;
        headers.push((name.trim().to_owned(), value.trim().to_owned()));
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
    if content_length > MAX_BODY_BYTES {
        return Err(invalid_data("request body is too large"));
    }

    let body_start = header_end + 4;
    while bytes.len() < body_start + content_length {
        let read = stream.read(&mut buffer).await?;
        if read == 0 {
            return Err(invalid_data("request body is incomplete"));
        }
        bytes.extend_from_slice(&buffer[..read]);
    }
    let body = bytes[body_start..body_start + content_length].to_vec();
    Ok(HttpRequest {
        method,
        path,
        headers,
        body,
    })
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
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        409 => "Conflict",
        503 => "Service Unavailable",
        _ => "Error",
    };
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{body}",
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
            ProposalService::new(Arc::new(proposal_repository)),
            ReadingService::new(Arc::new(reading_repository), Arc::new(delivery_repository)),
        ));
        let bridge =
            RunningAgentBridge::start(0, connections, Arc::new(CapabilityService), openclaw)
                .await
                .unwrap();
        let address = bridge.local_addr();
        assert_eq!(address.ip(), std::net::Ipv4Addr::LOCALHOST);

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
