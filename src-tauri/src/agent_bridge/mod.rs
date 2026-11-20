#[cfg(target_os = "windows")]
mod clipboard;
pub(crate) mod dumate;
mod mcp_http;
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
    chatgpt::{self, ChatGpt},
    codex_delivery::{CodexDelivery, PairingStatus, TodoGrantStatus},
    domain::agent_connection::{AgentConnection, AgentConnectionStatus, AGENT_API_VERSION},
    persistence::agent_connection_repository::SqliteAgentConnectionRepository,
};
use dumate::{DuMate, DuMateStatus, Scopes};
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
    codex: Arc<CodexDelivery>,
    dumate: Arc<DuMate>,
    chatgpt: Arc<ChatGpt>,
    configured_port: u16,
    runtime: Mutex<Option<RunningAgentBridge>>,
    last_error: RwLock<Option<String>>,
}

impl AgentBridgeState {
    pub(crate) fn new(
        repository: SqliteAgentConnectionRepository,
        openclaw: OpenClawAdapter,
        codex: CodexDelivery,
        dumate: DuMate,
        chatgpt: ChatGpt,
    ) -> Self {
        let configured_port = std::env::var(BRIDGE_PORT_ENV)
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(DEFAULT_BRIDGE_PORT);
        Self {
            connections: Arc::new(AgentConnectionService::new(Arc::new(repository))),
            capabilities: Arc::new(CapabilityService),
            openclaw: Arc::new(openclaw),
            codex: Arc::new(codex),
            dumate: Arc::new(dumate),
            chatgpt: Arc::new(chatgpt),
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
            Ok(_) if self.chatgpt.active().await.unwrap_or(false) => match self.bind(false).await {
                Ok(bridge) => *self.runtime.lock().await = Some(bridge),
                Err(error) => *self.last_error.write().await = Some(error),
            },
            Ok(_) => {}
            Err(error) => *self.last_error.write().await = Some(error.to_string()),
        }
    }

    async fn bind(&self, legacy: bool) -> Result<RunningAgentBridge, String> {
        RunningAgentBridge::start(
            self.configured_port,
            self.connections.clone(),
            self.capabilities.clone(),
            self.openclaw.clone(),
            self.codex.clone(),
            self.dumate.clone(),
            Some((self.chatgpt.clone(), legacy)),
        )
        .await
        .map_err(|error| format!("Local Agent Bridge could not bind to localhost: {error}"))
    }

    async fn start(&self) -> Result<AgentBridgeStatus, String> {
        let mut runtime = self.runtime.lock().await;
        if runtime.is_none() {
            *runtime = Some(self.bind(false).await?);
        }
        if let Err(error) = self.connections.ensure(AgentConnectionStatus::Active).await {
            if !self.chatgpt.active().await.unwrap_or(false) {
                if let Some(bridge) = runtime.take() {
                    bridge.stop().await;
                }
            }
            return Err(error.to_string());
        }
        if let Some(bridge) = runtime.as_ref() {
            bridge.enable_legacy();
        }
        *self.last_error.write().await = None;
        drop(runtime);
        self.status().await
    }

    async fn stop(&self) -> Result<AgentBridgeStatus, String> {
        let mut runtime = self.runtime.lock().await;
        if self.chatgpt.active().await.unwrap_or(false) {
            if let Some(bridge) = runtime.as_ref() {
                bridge.disable_legacy().await;
            }
        } else if let Some(bridge) = runtime.take() {
            bridge.stop().await;
        }
        self.connections
            .set_status(AgentConnectionStatus::Inactive)
            .await
            .map_err(|error| error.to_string())?;
        *self.last_error.write().await = None;
        drop(runtime);
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
            .map(|bridge| (bridge.legacy_running(), bridge.local_addr().port()))
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
pub(crate) async fn get_chatgpt_status(
    state: State<'_, AgentBridgeState>,
) -> Result<chatgpt::Status, String> {
    let runtime = state.runtime.lock().await;
    state.chatgpt.status(runtime.is_some()).await
}

#[tauri::command]
pub(crate) async fn configure_chatgpt(
    window: tauri::WebviewWindow,
    state: State<'_, AgentBridgeState>,
    reading: bool,
    todo: bool,
    rotate: bool,
) -> Result<chatgpt::Status, String> {
    if window.label() != "main" {
        return Err("请在主窗口设置中管理 ChatGPT。".into());
    }
    let scopes = chatgpt::Scopes { reading, todo };
    let mut runtime = state.runtime.lock().await;
    if scopes.any() && runtime.is_none() {
        *runtime = Some(state.bind(false).await?);
    }
    let result = state.chatgpt.configure(scopes, rotate).await;
    if runtime
        .as_ref()
        .is_some_and(|bridge| !bridge.legacy_running())
        && matches!(state.chatgpt.active().await, Ok(false))
    {
        if let Some(bridge) = runtime.take() {
            bridge.stop().await;
        }
    }
    result?;
    state.chatgpt.status(runtime.is_some()).await
}

#[tauri::command]
pub(crate) async fn export_chatgpt_package(
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
    state: State<'_, AgentBridgeState>,
) -> Result<bool, String> {
    if window.label() != "main" {
        return Err("请在主窗口设置中导出安装包。".into());
    }
    let port = {
        let runtime = state.runtime.lock().await;
        if !matches!(state.chatgpt.active().await, Ok(true)) {
            return Err("请先为 ChatGPT 开启至少一项权限。".into());
        }
        runtime
            .as_ref()
            .ok_or("本地连接未启动。")?
            .local_addr()
            .port()
    };
    chatgpt::export_package(app, port, chatgpt::profile()).await
}

#[tauri::command]
pub(crate) async fn get_dumate_status(
    state: State<'_, AgentBridgeState>,
) -> Result<DuMateStatus, String> {
    let runtime = state.runtime.lock().await;
    let port = runtime
        .as_ref()
        .map_or(state.configured_port, |bridge| bridge.local_addr().port());
    state
        .dumate
        .status(
            port,
            runtime
                .as_ref()
                .is_some_and(RunningAgentBridge::legacy_running),
        )
        .await
}

#[tauri::command]
pub(crate) async fn configure_dumate(
    state: State<'_, AgentBridgeState>,
    reading: bool,
    todo: bool,
    rotate: bool,
) -> Result<DuMateStatus, String> {
    let runtime = state.runtime.lock().await;
    let port = runtime
        .as_ref()
        .map_or(state.configured_port, |bridge| bridge.local_addr().port());
    state
        .dumate
        .configure(
            port,
            runtime
                .as_ref()
                .is_some_and(RunningAgentBridge::legacy_running),
            Scopes { reading, todo },
            rotate,
        )
        .await
}

#[tauri::command]
pub(crate) async fn copy_dumate_authorization(
    window: tauri::WebviewWindow,
    state: State<'_, AgentBridgeState>,
) -> Result<Option<String>, String> {
    if window.label() != "main" {
        return Err("请从主窗口设置复制凭据。".into());
    }
    let runtime = state.runtime.lock().await;
    if !runtime
        .as_ref()
        .is_some_and(RunningAgentBridge::legacy_running)
    {
        return Err("请先启用本地 Bridge。".into());
    }
    let header = state.dumate.authorization_header().await?;
    #[cfg(target_os = "windows")]
    {
        clipboard::copy_authorization(&window, &header)?;
        Ok(None)
    }
    // Fallback for the WKWebView: only this explicit action can receive
    // the header, never the status IPC. It is not rendered or cached.
    #[cfg(not(target_os = "windows"))]
    Ok(Some(header))
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

#[tauri::command]
pub(crate) async fn get_codex_pairing(
    state: State<'_, AgentBridgeState>,
) -> Result<PairingStatus, String> {
    let port = state.status().await?.port;
    state.codex.status(port).await
}

#[tauri::command]
pub(crate) async fn pair_codex(
    state: State<'_, AgentBridgeState>,
) -> Result<PairingStatus, String> {
    let bridge = state.status().await?;
    if !bridge.running {
        return Err("请先启用本地 Bridge，再授权 Codex。".to_owned());
    }
    state.codex.pair(bridge.port).await
}

#[tauri::command]
pub(crate) async fn revoke_codex(
    state: State<'_, AgentBridgeState>,
) -> Result<PairingStatus, String> {
    let port = state.status().await?.port;
    state.codex.revoke(port).await
}

#[tauri::command]
pub(crate) async fn get_codex_todo_grant(
    state: State<'_, AgentBridgeState>,
) -> Result<TodoGrantStatus, String> {
    state.codex.todo_status().await
}

#[tauri::command]
pub(crate) async fn pair_codex_todo(
    state: State<'_, AgentBridgeState>,
) -> Result<TodoGrantStatus, String> {
    if !state.status().await?.running {
        return Err("请先启用本地 Bridge，再授权待办建议。".to_owned());
    }
    state.codex.pair_todo().await
}

#[tauri::command]
pub(crate) async fn revoke_codex_todo(
    state: State<'_, AgentBridgeState>,
) -> Result<TodoGrantStatus, String> {
    state.codex.revoke_todo().await
}
