use std::{net::SocketAddr, sync::Arc, time::Duration};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::oneshot,
    task::JoinHandle,
    time::timeout,
};

use crate::application::{
    agent_connection_service::AgentConnectionService, capability_service::CapabilityService,
};

const MAX_HEADER_BYTES: usize = 8 * 1024;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(3);

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
                        tokio::spawn(async move {
                            let _ = handle_connection(stream, connections, capabilities).await;
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

async fn handle_connection(
    mut stream: TcpStream,
    connections: Arc<AgentConnectionService>,
    capabilities: Arc<CapabilityService>,
) -> Result<(), std::io::Error> {
    let request = match timeout(REQUEST_TIMEOUT, read_request(&mut stream)).await {
        Ok(Ok(request)) => request,
        _ => {
            write_response(&mut stream, 400, r#"{"error":"invalid_request"}"#).await?;
            return Ok(());
        }
    };
    let Some(request_line) = request.lines().next() else {
        write_response(&mut stream, 400, r#"{"error":"invalid_request"}"#).await?;
        return Ok(());
    };
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts.next().unwrap_or_default();
    let path = request_parts.next().unwrap_or_default();
    if method != "GET" {
        write_response(&mut stream, 405, r#"{"error":"method_not_allowed"}"#).await?;
        return Ok(());
    }
    if path != "/api/v1/health" && path != "/api/v1/capabilities" {
        write_response(&mut stream, 404, r#"{"error":"not_found"}"#).await?;
        return Ok(());
    }

    let token = bearer_token(&request).unwrap_or_default();
    match connections.authenticate(token).await {
        Ok(true) => {}
        Ok(false) => {
            write_response(&mut stream, 401, r#"{"error":"unauthorized"}"#).await?;
            return Ok(());
        }
        Err(_) => {
            write_response(&mut stream, 503, r#"{"error":"bridge_unavailable"}"#).await?;
            return Ok(());
        }
    }

    let body = if path == "/api/v1/health" {
        r#"{"version":"v1","status":"ok"}"#.to_owned()
    } else {
        serde_json::to_string(&capabilities.discover())
            .unwrap_or_else(|_| r#"{"error":"bridge_unavailable"}"#.to_owned())
    };
    write_response(&mut stream, 200, &body).await
}

async fn read_request(stream: &mut TcpStream) -> Result<String, std::io::Error> {
    let mut bytes = Vec::with_capacity(1024);
    let mut buffer = [0_u8; 1024];
    loop {
        let read = stream.read(&mut buffer).await?;
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..read]);
        if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
        if bytes.len() >= MAX_HEADER_BYTES {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "request headers are too large",
            ));
        }
    }
    String::from_utf8(bytes).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "request headers are not UTF-8",
        )
    })
}

fn bearer_token(request: &str) -> Option<&str> {
    request.lines().skip(1).find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if !name.eq_ignore_ascii_case("authorization") {
            return None;
        }
        value.trim().strip_prefix("Bearer ")
    })
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
        404 => "Not Found",
        405 => "Method Not Allowed",
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
        application::agent_connection_service::AgentConnectionService,
        domain::agent_connection::AgentConnectionStatus,
        persistence::{agent_connection_repository::SqliteAgentConnectionRepository, sqlite},
    };

    async fn request(address: SocketAddr, path: &str, token: Option<&str>) -> String {
        let mut stream = TcpStream::connect(address).await.unwrap();
        let authorization = token
            .map(|token| format!("Authorization: Bearer {token}\r\n"))
            .unwrap_or_default();
        let request = format!(
            "GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\n{authorization}Connection: close\r\n\r\n"
        );
        stream.write_all(request.as_bytes()).await.unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).await.unwrap();
        response
    }

    #[tokio::test]
    async fn starts_stops_authenticates_and_discovers_capabilities_on_loopback_only() {
        let temp = tempfile::tempdir().unwrap();
        let database = sqlite::connect(&temp.path().join("bridge.sqlite3"))
            .await
            .unwrap();
        let connections = Arc::new(AgentConnectionService::new(Arc::new(
            SqliteAgentConnectionRepository::new(database.0),
        )));
        connections
            .ensure(AgentConnectionStatus::Active)
            .await
            .unwrap();
        let issued = connections.generate_token().await.unwrap();
        let bridge = RunningAgentBridge::start(0, connections, Arc::new(CapabilityService))
            .await
            .unwrap();
        let address = bridge.local_addr();
        assert_eq!(address.ip(), std::net::Ipv4Addr::LOCALHOST);

        let missing = request(address, "/api/v1/health", None).await;
        let invalid = request(address, "/api/v1/capabilities", Some("wrong")).await;
        let health = request(address, "/api/v1/health", Some(&issued.token)).await;
        let capabilities = request(address, "/api/v1/capabilities", Some(&issued.token)).await;
        assert!(missing.starts_with("HTTP/1.1 401"));
        assert!(invalid.starts_with("HTTP/1.1 401"));
        assert!(health.starts_with("HTTP/1.1 200"));
        assert!(health.contains(r#"{"version":"v1","status":"ok"}"#));
        assert!(capabilities.starts_with("HTTP/1.1 200"));
        assert!(capabilities.contains(r#""delivery":true"#));
        assert!(capabilities.contains(r#""proposal":true"#));
        assert!(capabilities.contains(r#""reading":true"#));

        bridge.stop().await;
        assert!(TcpStream::connect(address).await.is_err());
    }
}
