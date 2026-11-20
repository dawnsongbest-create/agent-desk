use std::sync::{atomic::AtomicI64, Mutex};

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use uuid::Uuid;

use super::*;
use crate::{
    agent_bridge::dumate::Scopes,
    application::{
        delivery_service::DeliveryService,
        ports::credentials::{CredentialError, CredentialPort},
        proposal_service::ProposalService,
        reading_service::ReadingService,
    },
    domain::{agent_connection::AgentConnectionStatus, proposal::todo_batch::TodoSuggestionSource},
    persistence::{
        agent_connection_repository::SqliteAgentConnectionRepository,
        delivery_repository::SqliteDeliveryRepository,
        proposal_repository::SqliteProposalRepository, reading_repository::SqliteReadingRepository,
        sqlite,
    },
};

const T0: i64 = 1_800_000_000;
static CLOCK: AtomicI64 = AtomicI64::new(T0);
const READ: Scopes = Scopes {
    reading: true,
    todo: false,
};
const TODO: Scopes = Scopes {
    reading: false,
    todo: true,
};
const BOTH: Scopes = Scopes {
    reading: true,
    todo: true,
};
const OFF: Scopes = Scopes {
    reading: false,
    todo: false,
};
const ACCEPT: &str = "Accept: application/json, text/event-stream\r\n";
const VERSION: &str = "MCP-Protocol-Version: 2025-06-18\r\n";

#[derive(Default)]
struct MemoryCredential(Mutex<Option<String>>);

impl CredentialPort for MemoryCredential {
    fn store(&self, _: &str, secret: &str) -> Result<(), CredentialError> {
        *self.0.lock().unwrap() = Some(secret.into());
        Ok(())
    }
    fn read(&self, _: &str) -> Result<Option<String>, CredentialError> {
        Ok(self.0.lock().unwrap().clone())
    }
    fn delete(&self, _: &str) -> Result<(), CredentialError> {
        *self.0.lock().unwrap() = None;
        Ok(())
    }
}

struct Fixture {
    _dir: tempfile::TempDir,
    pool: SqlitePool,
    connections: Arc<AgentConnectionService>,
    openclaw_token: String,
    openclaw: Arc<OpenClawAdapter>,
    codex: Arc<CodexDelivery>,
    dumate: Arc<DuMate>,
    deliveries: DeliveryService,
    proposals: ProposalService,
}

impl Fixture {
    async fn new(clock: fn() -> i64) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let pool = sqlite::connect(&dir.path().join("isolated.sqlite3"))
            .await
            .unwrap()
            .0;
        let delivery = Arc::new(SqliteDeliveryRepository::new(pool.clone()));
        let deliveries = DeliveryService::new(delivery.clone());
        let proposals = ProposalService::new(Arc::new(SqliteProposalRepository::new(pool.clone())));
        let reading = ReadingService::new(
            Arc::new(SqliteReadingRepository::new(pool.clone())),
            delivery,
        );
        let connections = Arc::new(AgentConnectionService::new(Arc::new(
            SqliteAgentConnectionRepository::new(pool.clone()),
        )));
        connections
            .ensure(AgentConnectionStatus::Active)
            .await
            .unwrap();
        let openclaw_token = connections.generate_token().await.unwrap().token;
        let openclaw = Arc::new(OpenClawAdapter::new(
            deliveries.clone(),
            proposals.clone(),
            reading,
        ));
        let codex = Arc::new(CodexDelivery::new(
            pool.clone(),
            deliveries.clone(),
            proposals.clone(),
            Arc::new(|| {}),
            Arc::new(|| {}),
        ));
        let dumate = Arc::new(DuMate::isolated(
            pool.clone(),
            deliveries.clone(),
            proposals.clone(),
            Arc::new(MemoryCredential::default()),
            clock,
        ));
        Self {
            _dir: dir,
            pool,
            connections,
            openclaw_token,
            openclaw,
            codex,
            dumate,
            deliveries,
            proposals,
        }
    }

    async fn start(&self) -> RunningAgentBridge {
        RunningAgentBridge::start(
            0,
            self.connections.clone(),
            Arc::new(CapabilityService),
            self.openclaw.clone(),
            self.codex.clone(),
            self.dumate.clone(),
        )
        .await
        .unwrap()
    }

    async fn configure(&self, port: u16, scopes: Scopes, rotate: bool) -> String {
        self.dumate
            .configure(port, true, scopes, rotate)
            .await
            .unwrap();
        self.dumate
            .authorization_header()
            .await
            .unwrap()
            .strip_prefix("Bearer ")
            .unwrap()
            .into()
    }
}

async fn raw(address: SocketAddr, request: &str) -> String {
    let mut socket = TcpStream::connect(address).await.unwrap();
    socket.write_all(request.as_bytes()).await.unwrap();
    let mut response = String::new();
    socket.read_to_string(&mut response).await.unwrap();
    response
}

async fn request(
    address: SocketAddr,
    method: &str,
    path: &str,
    bearer: &str,
    extra: &str,
    body: &str,
) -> String {
    let auth = if bearer.is_empty() {
        String::new()
    } else {
        format!("Authorization: Bearer {bearer}\r\n")
    };
    raw(address, &format!("{method} {path} HTTP/1.1\r\nHost: {address}\r\n{auth}Content-Type: application/json\r\n{ACCEPT}{extra}Content-Length: {}\r\n\r\n{body}", body.len())).await
}

async fn post(address: SocketAddr, bearer: &str, payload: Value) -> String {
    request(
        address,
        "POST",
        "/mcp",
        bearer,
        VERSION,
        &payload.to_string(),
    )
    .await
}

fn body(response: &str) -> Value {
    assert!(response.starts_with("HTTP/1.1 200 "), "{response}");
    serde_json::from_str(response.split_once("\r\n\r\n").unwrap().1).unwrap()
}
fn rpc(id: i64, method: &str, params: Value) -> Value {
    json!({"jsonrpc":"2.0", "id":id, "method":method,"params":params})
}
fn call(name: &str, arguments: Value) -> Value {
    rpc(2, "tools/call", json!({"name":name,"arguments":arguments}))
}
fn reading(key: &str, markdown: &str) -> Value {
    json!({"title":"合成阅读报告", "content":markdown,"type":"report","idempotency_key":key})
}
fn todo(key: &str) -> Value {
    json!({"items":[{"text":"合成低优先级", "priority":"low"},{"text":"合成高优先级", "priority":"high"}],
        "idempotency_key":key})
}
fn structured(response: &str) -> Value {
    let value = body(response);
    let result = &value["result"]["structuredContent"];
    assert!(result.is_object(), "{value}");
    result.clone()
}
fn assert_status(response: &str, code: u16) {
    assert!(
        response.starts_with(&format!("HTTP/1.1 {code} ")),
        "{response}"
    );
}

#[tokio::test]
async fn streamable_http_scopes_origins_and_committed_receipts_through_the_single_bridge() {
    let f = Fixture::new(|| CLOCK.load(Ordering::SeqCst)).await;
    let bridge = f.start().await;
    let address = bridge.local_addr();
    assert_eq!(address.ip(), std::net::Ipv4Addr::LOCALHOST);
    let list = rpc(1, "tools/list", json!({}));
    for invalid in ["", "wrong", f.openclaw_token.as_str()] {
        assert_status(&post(address, invalid, list.clone()).await, 401);
    }
    let read_token = f.configure(address.port(), READ, false).await;
    let init = rpc(
        0,
        "initialize",
        json!({
            "protocolVersion":"2025-06-18", "clientInfo":{"name":"isolated test client","version":"1.0"},
            "capabilities":{}
        }),
    );
    let result = body(&post(address, &read_token, init.clone()).await);
    assert_eq!(result["result"]["protocolVersion"], "2025-06-18");
    let mut future_init = init;
    future_init["params"]["protocolVersion"] = json!("2099-01-01");
    let future = request(
        address,
        "POST",
        "/mcp",
        &read_token,
        "",
        &future_init.to_string(),
    )
    .await;
    assert_eq!(body(&future)["result"]["protocolVersion"], "2025-11-25");
    assert_status(
        &post(
            address,
            &read_token,
            json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        )
        .await,
        202,
    );
    let tools = body(&post(address, &read_token, list.clone()).await)["result"]["tools"].clone();
    assert_eq!(tools.as_array().unwrap().len(), 1);
    assert_eq!(tools[0]["name"], "send_reading");
    assert!(tools[0]["inputSchema"]["required"]
        .as_array()
        .unwrap()
        .contains(&json!("idempotency_key")));
    let get = request(address, "GET", "/mcp", &read_token, "", "").await;
    assert_status(&get, 405);
    assert!(get.contains("Allow: POST\r\n"));
    assert_eq!(
        body(&post(address, &read_token, call("suggest_todos", todo("same"))).await)["result"]
            ["isError"],
        true
    );
    assert_eq!(
        body(&post(address, &read_token, rpc(3, "resources/list", json!({}))).await)["error"]
            ["code"],
        -32601
    );
    assert_eq!(
        body(&post(address, &read_token, call("accept_todo", json!({}))).await)["error"]["code"],
        -32602
    );
    assert_eq!(
        body(
            &post(
                address,
                &read_token,
                call(
                    "send_reading",
                    json!({"type":"report","title":"x","content":"y"})
                )
            )
            .await
        )["result"]["isError"],
        true
    );
    let mut fake_source = reading("fake", "test");
    fake_source["source_label"] = json!("Codex");
    assert_eq!(
        body(&post(address, &read_token, call("send_reading", fake_source)).await)["result"]
            ["isError"],
        true
    );
    let mut no_id = call("send_reading", reading("no-id", "must not persist"));
    no_id.as_object_mut().unwrap().remove("id");
    assert_status(&post(address, &read_token, no_id).await, 202);
    assert!(f.deliveries.list_inbox().await.unwrap().is_empty());

    let both_token = f.configure(address.port(), BOTH, false).await;
    assert_ne!(both_token, read_token);
    assert_status(&post(address, &read_token, list.clone()).await, 401);
    let both_tools =
        body(&post(address, &both_token, list.clone()).await)["result"]["tools"].clone();
    assert_eq!(both_tools.as_array().unwrap().len(), 2);
    assert_eq!(both_tools[1]["name"], "suggest_todos");
    assert!(both_tools[1]["inputSchema"]["required"]
        .as_array()
        .unwrap()
        .contains(&json!("idempotency_key")));
    assert!(!mcp_tool_optional_changed());

    for headers in [
        "Origin: https://synthetic.invalid\r\n",
        "Host: 127.0.0.1\r\n",
        "Authorization: Bearer duplicate\r\n",
        "Content-Length: 1\r\n",
        "Transfer-Encoding: chunked\r\n",
        "MCP-Protocol-Version: unknown\r\n",
        "Forwarded: host=synthetic.invalid\r\n",
    ] {
        let response = request(
            address,
            "POST",
            "/mcp",
            &both_token,
            headers,
            &list.to_string(),
        )
        .await;
        assert_status(
            &response,
            if headers.starts_with("Origin") {
                403
            } else {
                400
            },
        );
    }
    let wrong_host = raw(
        address,
        &format!(
            "GET /mcp HTTP/1.1\r\nHost: synthetic.invalid:{}\r\n\r\n",
            address.port()
        ),
    )
    .await;
    assert_status(&wrong_host, 403);
    assert_status(
        &request(
            address,
            "POST",
            "/mcp",
            &both_token,
            "MCP-Protocol-Version: unknown\r\n",
            "{}",
        )
        .await,
        400,
    );
    let invalid_media = raw(address, &format!("POST /mcp HTTP/1.1\r\nHost: {address}\r\nAuthorization: Bearer {both_token}\r\nContent-Type: text/plain\r\n\r\n")).await;
    assert_status(&invalid_media, 415);
    let invalid_accept = raw(address, &format!("POST /mcp HTTP/1.1\r\nHost: {address}\r\nAuthorization: Bearer {both_token}\r\nContent-Type: application/json\r\nAccept: application/json, text/event-stream;q=0\r\n\r\n")).await;
    assert_status(&invalid_accept, 406);
    assert_eq!(
        body(&request(address, "POST", "/mcp", &both_token, VERSION, "{").await)["error"]["code"],
        -32700
    );
    assert_eq!(
        body(&post(address, &both_token, json!([])).await)["error"]["code"],
        -32600
    );
    let list_body = list.to_string();
    let chunked = raw(address, &format!("POST /mcp HTTP/1.1\r\nHost: {address}\r\nAuthorization: Bearer {both_token}\r\nContent-Type: application/json\r\n{ACCEPT}{VERSION}Transfer-Encoding: chunked\r\n\r\n{:x}\r\n{list_body}\r\n0\r\n\r\n", list_body.len())).await;
    assert_eq!(body(&chunked)["result"]["tools"], both_tools);

    for (name, input) in [
        (
            "send_reading",
            reading("big", &"a".repeat(mcp_http::MAX_BODY + 1)),
        ),
        (
            "suggest_todos",
            json!({"idempotency_key":"big","items":[{"priority":"low","text":"a".repeat(65_536)}]}),
        ),
    ] {
        assert_status(&post(address, &both_token, call(name, input)).await, 413);
    }
    let markdown = "# 合成报告\n\n- 保留 Markdown";
    let send = call("send_reading", reading("same", markdown));
    let (r1, r2) = tokio::join!(
        post(address, &both_token, send.clone()),
        post(address, &both_token, send)
    );
    let a = structured(&r1);
    let b = structured(&r2);
    assert_eq!(a["delivery_id"], b["delivery_id"]);
    assert_eq!(a["document_id"], b["document_id"]);
    assert_ne!(a["created"], b["created"]);
    let conflict = body(
        &post(
            address,
            &both_token,
            call("send_reading", reading("same", "changed")),
        )
        .await,
    );
    assert!(conflict.to_string().contains("idempotency_conflict"));
    let row = f
        .deliveries
        .open(a["delivery_id"].as_str().unwrap())
        .await
        .unwrap();
    assert_eq!(row.document.source_label.as_deref(), Some("百度搭子"));
    assert_eq!(row.document.content_markdown, markdown);
    assert_eq!(row.document.id, a["document_id"]);
    assert_eq!(f.deliveries.get_unread_count().await.unwrap(), 0);
    assert_status(
        &request(address, "POST", "/api/v1/delivery", &both_token, "", "{}").await,
        401,
    );
    assert_status(&request(address, "POST", "/api/v1/delivery", &f.openclaw_token, "", &json!({
            "title":"Synthetic", "content":"fixture", "type":"research", "idempotency_key":row.delivery.idempotency_key,
    }).to_string()).await, 403);

    let batch = structured(&post(address, &both_token, call("suggest_todos", todo("same"))).await);
    let dup = structured(&post(address, &both_token, call("suggest_todos", todo("same"))).await);
    assert_eq!(batch["batch_id"], dup["batch_id"]);
    assert_eq!(dup["created"], false);
    let conflict = body(
        &post(
            address,
            &both_token,
            call(
                "suggest_todos",
                json!({
                    "items": [{"priority":"high","text":"changed"}], "idempotency_key":"same"
                }),
            ),
        )
        .await,
    );
    assert!(conflict.to_string().contains("idempotency_conflict"));
    let for_reject =
        structured(&post(address, &both_token, call("suggest_todos", todo("reject"))).await);
    f.proposals
        .reject_todo_batch(for_reject["batch_id"].as_str().unwrap())
        .await
        .unwrap();
    let old_codex = format!(
        "{}{}{}",
        crate::codex_delivery::TODO_TOKEN_PREFIX,
        Uuid::new_v4().simple(),
        Uuid::new_v4().simple()
    );
    sqlx::query(
        "INSERT INTO codex_todo_suggestion_grant VALUES ('codex-todo-suggestions',?,
        'active','suggest_todos',?,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP)",
    )
    .bind(Sha256::digest(old_codex.as_bytes()).to_vec())
    .bind(Uuid::new_v4().simple().to_string())
    .execute(&f.pool)
    .await
    .unwrap();
    assert_status(&post(address, &old_codex, list.clone()).await, 401);
    let old_receipt = body(
        &request(
            address,
            "POST",
            crate::codex_delivery::TODO_PATH,
            &old_codex,
            "",
            &todo("same").to_string(),
        )
        .await,
    );
    assert_ne!(old_receipt["batch_id"], batch["batch_id"]);
    let pending = f.proposals.list_pending_todo_batches().await.unwrap();
    let dumate_batch = pending
        .iter()
        .find(|b| b.source == TodoSuggestionSource::Dumate)
        .unwrap();
    assert_eq!(dumate_batch.id, batch["batch_id"]);
    assert_eq!(dumate_batch.items[0].text, "合成高优先级");
    assert_eq!(
        pending
            .iter()
            .filter(|b| b.source == TodoSuggestionSource::Codex)
            .count(),
        1
    );
    assert_eq!(count_tasks(&f.pool).await, 0);
    let narrowed = f.configure(address.port(), TODO, false).await;
    assert_eq!(both_token, narrowed);
    let todo_tools = body(&post(address, &narrowed, list.clone()).await)["result"]["tools"].clone();
    assert_eq!(todo_tools.as_array().unwrap().len(), 1);
    assert_eq!(todo_tools[0]["name"], "suggest_todos");
    assert_eq!(
        body(
            &post(
                address,
                &narrowed,
                call("send_reading", reading("denied", "no"))
            )
            .await
        )["result"]["isError"],
        true
    );
    let accepted = f
        .proposals
        .accept_todo_batch(&dumate_batch.id)
        .await
        .unwrap();
    assert_eq!(accepted.todo_ids.len(), 2);

    bridge.stop().await;
    assert!(TcpStream::connect(address).await.is_err());
    let bridge = f.start().await;
    let address = bridge.local_addr();
    let accepted_replay =
        structured(&post(address, &narrowed, call("suggest_todos", todo("same"))).await);
    assert_eq!(accepted_replay["status"], "accepted");
    assert_eq!(count_tasks(&f.pool).await, 2);
    assert_eq!(f.deliveries.list_inbox().await.unwrap().len(), 1);
    CLOCK.store(T0 + 1800, Ordering::SeqCst);
    assert_status(&post(address, &narrowed, list.clone()).await, 401);
    assert!(f.dumate.authorization_header().await.is_err());
    let rotated = f.configure(address.port(), BOTH, true).await;
    assert_ne!(narrowed, rotated);
    assert_status(&post(address, &narrowed, list.clone()).await, 401);
    f.dumate
        .configure(address.port(), true, OFF, false)
        .await
        .unwrap();
    assert_status(&post(address, &rotated, list).await, 401);
    assert_eq!(
        f.proposals
            .accept_todo_batch(&dumate_batch.id)
            .await
            .unwrap(),
        accepted
    );
    assert_status(
        &request(address, "GET", "/api/v1/health", &f.openclaw_token, "", "").await,
        200,
    );
    bridge.stop().await;
}

fn mcp_tool_optional_changed() -> bool {
    use crate::codex_delivery::mcp;
    [mcp::tool(), mcp::todo_tool()].iter().any(|tool| {
        tool["inputSchema"]["required"]
            .as_array()
            .unwrap()
            .contains(&json!("idempotency_key"))
    })
}

async fn count_tasks(pool: &SqlitePool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM task_payloads")
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn await_active_request(dumate: &DuMate) {
    timeout(Duration::from_secs(2), async {
        loop {
            if dumate.gate.try_lock().is_err() {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn revoke_and_stop_wait_for_in_flight_commit_and_close_old_admission() {
    let f = Fixture::new(|| T0).await;
    let bridge = f.start().await;
    let address = bridge.local_addr();
    let token = f.configure(address.port(), TODO, false).await;
    let writer = f.pool.begin_with("BEGIN IMMEDIATE").await.unwrap();
    let started = tokio::spawn({
        let token = token.clone();
        async move { post(address, &token, call("suggest_todos", todo("revoke-race"))).await }
    });
    await_active_request(&f.dumate).await;
    let mut revoke = tokio::spawn({
        let dumate = f.dumate.clone();
        async move {
            dumate
                .configure(address.port(), true, OFF, false)
                .await
                .unwrap()
        }
    });
    assert!(timeout(Duration::from_millis(30), &mut revoke)
        .await
        .is_err());
    writer.rollback().await.unwrap();
    assert_eq!(structured(&started.await.unwrap())["created"], true);
    revoke.await.unwrap();
    assert_status(
        &post(address, &token, call("suggest_todos", todo("after-revoke"))).await,
        401,
    );

    let token = f.configure(address.port(), TODO, false).await;
    let writer = f.pool.begin_with("BEGIN IMMEDIATE").await.unwrap();
    let started = tokio::spawn({
        let token = token.clone();
        async move { post(address, &token, call("suggest_todos", todo("stop-race"))).await }
    });
    await_active_request(&f.dumate).await;
    let mut stop = tokio::spawn(bridge.stop());
    assert!(timeout(Duration::from_millis(30), &mut stop).await.is_err());
    writer.rollback().await.unwrap();
    assert_eq!(structured(&started.await.unwrap())["created"], true);
    stop.await.unwrap();
    assert!(TcpStream::connect(address).await.is_err());
    let old_admission = AtomicBool::new(false);
    let delayed = mcp_http::respond(
        &f.dumate,
        &old_admission,
        "POST",
        &[("authorization".into(), format!("Bearer {token}"))],
        &[],
    )
    .await;
    assert_eq!(delayed.0, 503);
    assert_eq!(count_tasks(&f.pool).await, 0);
    assert_eq!(
        f.proposals.list_pending_todo_batches().await.unwrap().len(),
        2
    );
}
