mod server;

use std::sync::Arc;

use serde::Serialize;
use tauri::State;
use tokio::sync::{Mutex, RwLock};

use crate::{
    adapters::openclaw::OpenClawAdapter,
    application::{
        agent_connection_service::{AgentConnectionService, IssuedAgentToken},
        capability_service::CapabilityService,
    },
    domain::agent_connection::{AgentConnection, AgentConnectionStatus, AGENT_API_VERSION},
    persistence::agent_connection_repository::SqliteAgentConnectionRepository,
};
use server::RunningAgentBridge;

const DEFAULT_BRIDGE_PORT: u16 = 47321;
const BRIDGE_PORT_ENV: &str = "AGENT_DESK_BRIDGE_PORT";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentBridgeStatus {
    version: &'static str,
    enabled: bool,
    running: bool,
    bind_address: &'static str,
    port: u16,
    endpoint: String,
    connection: Option<AgentConnection>,
    last_error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateAgentTokenResult {
    token: String,
    status: AgentBridgeStatus,
}

pub struct AgentBridgeState {
    connections: Arc<AgentConnectionService>,
    capabilities: Arc<CapabilityService>,
    openclaw: Arc<OpenClawAdapter>,
    configured_port: u16,
    runtime: Mutex<Option<RunningAgentBridge>>,
    last_error: RwLock<Option<String>>,
}

impl AgentBridgeState {
    pub fn new(repository: SqliteAgentConnectionRepository, openclaw: OpenClawAdapter) -> Self {
        let configured_port = std::env::var(BRIDGE_PORT_ENV)
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(DEFAULT_BRIDGE_PORT);
        Self {
            connections: Arc::new(AgentConnectionService::new(Arc::new(repository))),
            capabilities: Arc::new(CapabilityService),
            openclaw: Arc::new(openclaw),
            configured_port,
            runtime: Mutex::new(None),
            last_error: RwLock::new(None),
        }
    }

    pub async fn restore(&self) {
        match self.connections.get().await {
            Ok(Some(connection)) if connection.status == AgentConnectionStatus::Active => {
                if let Err(error) = self.start().await {
                    *self.last_error.write().await = Some(error);
                }
            }
            Ok(_) => {}
            Err(error) => *self.last_error.write().await = Some(error.to_string()),
        }
    }

    async fn start(&self) -> Result<AgentBridgeStatus, String> {
        let mut runtime = self.runtime.lock().await;
        if runtime.is_none() {
            let bridge = RunningAgentBridge::start(
                self.configured_port,
                self.connections.clone(),
                self.capabilities.clone(),
                self.openclaw.clone(),
            )
            .await
            .map_err(|error| format!("Local Agent Bridge could not bind to localhost: {error}"))?;
            if let Err(error) = self.connections.ensure(AgentConnectionStatus::Active).await {
                bridge.stop().await;
                return Err(error.to_string());
            }
            *runtime = Some(bridge);
        }
        *self.last_error.write().await = None;
        drop(runtime);
        self.status().await
    }

    async fn stop(&self) -> Result<AgentBridgeStatus, String> {
        if let Some(bridge) = self.runtime.lock().await.take() {
            bridge.stop().await;
        }
        self.connections
            .set_status(AgentConnectionStatus::Inactive)
            .await
            .map_err(|error| error.to_string())?;
        *self.last_error.write().await = None;
        self.status().await
    }

    async fn generate_token(&self) -> Result<GenerateAgentTokenResult, String> {
        let IssuedAgentToken { token, .. } = self
            .connections
            .generate_token()
            .await
            .map_err(|error| error.to_string())?;
        Ok(GenerateAgentTokenResult {
            token,
            status: self.status().await?,
        })
    }

    async fn status(&self) -> Result<AgentBridgeStatus, String> {
        let connection = self
            .connections
            .get()
            .await
            .map_err(|error| error.to_string())?;
        let runtime = self.runtime.lock().await;
        let (running, port) = runtime
            .as_ref()
            .map(|bridge| (true, bridge.local_addr().port()))
            .unwrap_or((false, self.configured_port));
        Ok(AgentBridgeStatus {
            version: AGENT_API_VERSION,
            enabled: connection
                .as_ref()
                .is_some_and(|item| item.status == AgentConnectionStatus::Active),
            running,
            bind_address: "127.0.0.1",
            port,
            endpoint: format!("http://127.0.0.1:{port}/api/v1"),
            connection: connection.as_ref().map(AgentConnection::from),
            last_error: self.last_error.read().await.clone(),
        })
    }
}

#[tauri::command]
pub async fn get_agent_bridge_status(
    state: State<'_, AgentBridgeState>,
) -> Result<AgentBridgeStatus, String> {
    state.status().await
}

#[tauri::command]
pub async fn start_agent_bridge(
    state: State<'_, AgentBridgeState>,
) -> Result<AgentBridgeStatus, String> {
    state.start().await
}

#[tauri::command]
pub async fn stop_agent_bridge(
    state: State<'_, AgentBridgeState>,
) -> Result<AgentBridgeStatus, String> {
    state.stop().await
}

#[tauri::command]
pub async fn generate_agent_token(
    state: State<'_, AgentBridgeState>,
) -> Result<GenerateAgentTokenResult, String> {
    state.generate_token().await
}
