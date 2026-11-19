use std::{
    io::{self, BufRead, Write},
    net::{Ipv4Addr, SocketAddrV4},
    time::Duration,
};

use serde_json::{json, Value};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    runtime::{Builder, Runtime},
    time::timeout,
};
use uuid::Uuid;

use crate::application::ports::credentials::CredentialPort;

use super::{
    credentials::{credential_reference, OsCredentialStore},
    ReadingReceipt, ReadingRequest, CODEX_TOKEN_PREFIX, MAX_CODEX_BODY, READING_PATH,
};

// The transport limit is separate so a modestly oversize tool input can
// still be decoded and rejected with its proper JSON-RPC request ID.
const MAX_FRAME: usize = 8 * MAX_CODEX_BODY;
const MAX_RESPONSE: usize = 8 * 1024;
const PROTOCOLS: &[&str] = &["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];

pub(super) fn run(args: &[String]) -> i32 {
    let [port] = args else { return 2 };
    let Ok(port) = port.parse::<u16>() else {
        return 2;
    };
    if port == 0 {
        return 2;
    }
    let Ok(runtime) = Builder::new_current_thread().enable_all().build() else {
        return 1;
    };
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut input = stdin.lock();
    let mut output = stdout.lock();
    let mut protocol = Protocol::default();
    let mut buffer = Vec::new();
    loop {
        buffer.clear();
        match read_frame(&mut input, &mut buffer) {
            Ok(false) => return 0,
            Err(error) => {
                if error.kind() == io::ErrorKind::InvalidData {
                    let _ = serde_json::to_writer(
                        &mut output,
                        &rpc_error(Value::Null, -32600, "MCP frame exceeds the size limit"),
                    );
                    let _ = output.write_all(b"\n").and_then(|()| output.flush());
                }
                return 1;
            }
            Ok(true) => {}
        }
        let response = match serde_json::from_slice::<Value>(&buffer) {
            Ok(message) => protocol.handle(&message, |args| send(&runtime, port, args)),
            Err(_) => Some(rpc_error(Value::Null, -32700, "Parse error")),
        };
        if let Some(response) = response {
            if serde_json::to_writer(&mut output, &response).is_err()
                || output
                    .write_all(b"\n")
                    .and_then(|()| output.flush())
                    .is_err()
            {
                return 1;
            }
        }
    }
}

fn read_frame(input: &mut impl BufRead, frame: &mut Vec<u8>) -> io::Result<bool> {
    loop {
        let pending = input.fill_buf()?;
        if pending.is_empty() {
            return Ok(!frame.is_empty());
        }
        let newline = pending.iter().position(|byte| *byte == b'\n');
        let count = newline.map_or(pending.len(), |index| index + 1);
        if frame.len() + count > MAX_FRAME {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "STDIO frame too large",
            ));
        }
        frame.extend_from_slice(&pending[..count]);
        input.consume(count);
        if newline.is_some() {
            return Ok(true);
        }
    }
}

#[derive(Default)]
struct Protocol {
    negotiated: bool,
    ready: bool,
}

impl Protocol {
    fn handle(&mut self, message: &Value, send: impl FnOnce(Value) -> Value) -> Option<Value> {
        let id = message.get("id").cloned();
        let method = message.get("method").and_then(Value::as_str);
        if message.get("jsonrpc").and_then(Value::as_str) != Some("2.0") || method.is_none() {
            return Some(rpc_error(
                id.unwrap_or(Value::Null),
                -32600,
                "Invalid Request",
            ));
        }
        let method = method?;
        let Some(id) = id else {
            if method == "notifications/initialized" && self.negotiated {
                self.ready = true;
            }
            return None;
        };
        if !id.is_string() && id.as_i64().is_none() && id.as_u64().is_none() {
            return Some(rpc_error(Value::Null, -32600, "Invalid Request id"));
        }
        let params = message.get("params").unwrap_or(&Value::Null);
        let result = match method {
            "ping" => json!({}),
            "initialize" if !self.negotiated => {
                let Some(requested) = params.get("protocolVersion").and_then(Value::as_str) else {
                    return Some(rpc_error(id, -32602, "Invalid initialize parameters"));
                };
                if !params.get("capabilities").is_some_and(Value::is_object)
                    || !params.get("clientInfo").is_some_and(Value::is_object)
                {
                    return Some(rpc_error(id, -32602, "Invalid initialize parameters"));
                }
                self.negotiated = true;
                let selected = if PROTOCOLS.contains(&requested) {
                    requested
                } else {
                    PROTOCOLS[0]
                };
                json!({
                    "protocolVersion": selected,
                    "capabilities": { "tools": { "listChanged": false } },
                    "serverInfo": { "name": "agent-desk", "version": env!("CARGO_PKG_VERSION") },
                    "instructions": "Send a report to Agent Desk Inbox only when asked. Only state success after a committed receipt. For uncertain delivery check Inbox or retry using the returned request ID; never guess a new ID for that same attempt."
                })
            }
            _ if !self.ready => return Some(rpc_error(id, -32002, "MCP initialization required")),
            "tools/list" => json!({ "tools": [tool()] }),
            "tools/call" => {
                if params.get("name").and_then(Value::as_str) != Some("send_reading") {
                    return Some(rpc_error(id, -32602, "Unknown tool"));
                }
                send(params.get("arguments").cloned().unwrap_or(Value::Null))
            }
            _ => return Some(rpc_error(id, -32601, "Method not found")),
        };
        Some(json!({ "jsonrpc": "2.0", "id": id, "result": result }))
    }
}

fn rpc_error(id: Value, code: i32, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

fn tool() -> Value {
    json!({
        "name": "send_reading",
        "title": "Send reading to Agent Desk",
        "description": "Send this Markdown document to Agent Desk Inbox when the user asks. Does not create tasks or move focus. Use the same idempotency_key only when retrying that one send; omit it for a new send.",
        "annotations": { "readOnlyHint": false, "destructiveHint": false, "idempotentHint": false, "openWorldHint": false },
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "required": ["title", "content", "type"],
            "properties": {
                "title": { "type": "string", "maxLength": 500 },
                "content": { "type": "string", "description": "Full Markdown, not a file path. The encoded JSON body must fit 1 MiB." },
                "type": { "type": "string", "enum": ["article", "report", "brief"] },
                "subtitle": { "type": "string", "maxLength": 500 },
                "idempotency_key": { "type": "string", "maxLength": 500 }
            }
        }
    })
}

fn tool_error(code: &str, message: &str, request_id: Option<&str>) -> Value {
    json!({
        "isError": true,
        "content": [{ "type": "text", "text": message }],
        "structuredContent": { "code": code, "request_id": request_id }
    })
}

fn send(runtime: &Runtime, port: u16, args: Value) -> Value {
    let Ok(mut request) = serde_json::from_value::<ReadingRequest>(args) else {
        return tool_error(
            "invalid_payload",
            "Provide title, full Markdown and a supported type.",
            None,
        );
    };
    if let Err(error) = request.validate() {
        if error == super::CodexError::TooLarge {
            return tool_error(
                "payload_too_large",
                "The Markdown exceeds the size limit; nothing was truncated or sent.",
                None,
            );
        }
        return tool_error(
            "invalid_payload",
            "Check the title, subtitle, Markdown and request key.",
            None,
        );
    }
    let key = request
        .idempotency_key
        .get_or_insert_with(|| Uuid::new_v4().simple().to_string())
        .trim()
        .to_owned();
    request.idempotency_key = Some(key.clone());
    let Ok(body) = serde_json::to_vec(&request) else {
        return tool_error("invalid_payload", "Unable to encode the document.", None);
    };
    if body.len() > MAX_CODEX_BODY {
        return tool_error(
            "payload_too_large",
            "The UTF-8 JSON body exceeds 1 MiB; nothing was truncated or sent.",
            None,
        );
    }
    let Ok(Some(token)) = OsCredentialStore.read(&credential_reference()) else {
        return tool_error(
            "credential_unavailable",
            "Cannot read the system credential. Connect or reauthorize Codex in Agent Desk.",
            None,
        );
    };
    if !valid_token(&token) {
        return tool_error(
            "credential_unavailable",
            "Reauthorize Codex in Agent Desk.",
            None,
        );
    }
    // Reuse the exact same key/body across the one bounded, in-process retry.
    // No persistent queue: on lost process state, never auto-send as a new request.
    let outcome = runtime.block_on(async {
        match call_once(port, &token, &body, &key).await {
            Err(CallFailure::Uncertain) => match call_once(port, &token, &body, &key).await {
                Ok(receipt) => Ok(receipt),
                Err(_) => Err(CallFailure::Uncertain),
            },
            outcome => outcome,
        }
    });
    match outcome {
        Ok(receipt) => json!({
            "content": [{ "type": "text", "text": if receipt.created { "已提交到 Agent Desk Inbox。" } else { "该请求已提交过，未重复创建。" } }],
            "structuredContent": receipt
        }),
        Err(CallFailure::Unavailable) => tool_error("bridge_unavailable", "无法连接 Agent Desk Bridge。请确认已启用 Bridge；本次未发送。", Some(&key)),
        Err(CallFailure::Rejected(401)) => tool_error("unauthorized", "Codex 授权已失效。请在 Agent Desk 中重新连接。", Some(&key)),
        Err(CallFailure::Rejected(403)) => tool_error("forbidden", "此连接没有阅读投递权限，或 Codex 投递已禁用。", Some(&key)),
        Err(CallFailure::Rejected(409)) => tool_error("idempotency_conflict", "同一请求 ID 已用于不同内容；未覆盖原投递。", Some(&key)),
        Err(CallFailure::Rejected(413)) => tool_error("payload_too_large", "请求超过 1 MiB，未截断或保存。", Some(&key)),
        Err(CallFailure::Rejected(_)) => tool_error("invalid_payload", "Agent Desk 拒绝了投递参数；未保存。", Some(&key)),
        Err(CallFailure::Uncertain) => tool_error("delivery_uncertain", "结果不确定，请检查 Inbox。如需重试同一次发送，请复用返回的 request_id。若连接器重启且原 ID 丢失，不要重新生成 ID 自动重试。", Some(&key)),
    }
}

fn valid_token(token: &str) -> bool {
    token
        .strip_prefix(CODEX_TOKEN_PREFIX)
        .is_some_and(|rest| rest.len() == 64 && rest.bytes().all(|b| b.is_ascii_hexdigit()))
}

#[derive(Debug)]
enum CallFailure {
    Unavailable,
    Rejected(u16),
    Uncertain,
}

async fn call_once(
    port: u16,
    token: &str,
    body: &[u8],
    key: &str,
) -> Result<ReadingReceipt, CallFailure> {
    let address = SocketAddrV4::new(Ipv4Addr::LOCALHOST, port);
    let mut stream = timeout(Duration::from_secs(2), TcpStream::connect(address))
        .await
        .map_err(|_| CallFailure::Unavailable)?
        .map_err(|_| CallFailure::Unavailable)?;
    let response = timeout(Duration::from_secs(6), async {
        let headers = format!(
            "POST {READING_PATH} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAuthorization: Bearer {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        stream.write_all(headers.as_bytes()).await?;
        stream.write_all(body).await?;
        let mut response = Vec::new();
        (&mut stream).take((MAX_RESPONSE + 1) as u64).read_to_end(&mut response).await?;
        Ok::<_, io::Error>(response)
    }).await.map_err(|_| CallFailure::Uncertain)?.map_err(|_| CallFailure::Uncertain)?;
    parse_receipt(&response, key)
}

fn parse_receipt(response: &[u8], key: &str) -> Result<ReadingReceipt, CallFailure> {
    if response.len() > MAX_RESPONSE {
        return Err(CallFailure::Uncertain);
    }
    let response = std::str::from_utf8(response).map_err(|_| CallFailure::Uncertain)?;
    let (headers, body) = response
        .split_once("\r\n\r\n")
        .ok_or(CallFailure::Uncertain)?;
    let mut lines = headers.split("\r\n");
    let mut status = lines
        .next()
        .ok_or(CallFailure::Uncertain)?
        .split_whitespace();
    if status.next() != Some("HTTP/1.1") {
        return Err(CallFailure::Uncertain);
    }
    match status.next() {
        Some("200") => {}
        Some(code @ ("400" | "401" | "403" | "409" | "413")) => {
            return Err(CallFailure::Rejected(
                code.parse().map_err(|_| CallFailure::Uncertain)?,
            ));
        }
        _ => return Err(CallFailure::Uncertain), // includes redirect and unknown server
    }
    let mut length = None;
    for line in lines {
        let (name, value) = line.split_once(':').ok_or(CallFailure::Uncertain)?;
        if name.eq_ignore_ascii_case("transfer-encoding") {
            return Err(CallFailure::Uncertain);
        }
        if name.eq_ignore_ascii_case("content-length")
            && length
                .replace(
                    value
                        .trim()
                        .parse::<usize>()
                        .map_err(|_| CallFailure::Uncertain)?,
                )
                .is_some()
        {
            return Err(CallFailure::Uncertain);
        }
    }
    if length != Some(body.len()) {
        return Err(CallFailure::Uncertain);
    }
    let receipt: ReadingReceipt = serde_json::from_str(body).map_err(|_| CallFailure::Uncertain)?;
    if receipt.version != "v1"
        || receipt.operation != "send_reading"
        || receipt.request_id != key
        || !valid_receipt_id(&receipt.delivery_id, "delivery_")
        || !valid_receipt_id(&receipt.document_id, "reader_")
    {
        return Err(CallFailure::Uncertain);
    }
    Ok(receipt)
}

fn valid_receipt_id(value: &str, prefix: &str) -> bool {
    value
        .strip_prefix(prefix)
        .is_some_and(|id| id.len() == 32 && id.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_discovers_only_reading_and_never_executes_notifications() {
        let mut server = Protocol::default();
        let no_send = |_| panic!("unexpected tool invocation");
        let denied = server
            .handle(
                &json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}),
                no_send,
            )
            .unwrap();
        assert_eq!(denied["error"]["code"], -32002);
        let init = server.handle(&json!({
            "jsonrpc":"2.0","id":2,"method":"initialize",
            "params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1"}}
        }), no_send).unwrap();
        assert_eq!(init["result"]["protocolVersion"], "2025-06-18");
        assert!(server
            .handle(
                &json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
                no_send
            )
            .is_none());
        let found = server
            .handle(
                &json!({"jsonrpc":"2.0","id":"list","method":"tools/list"}),
                no_send,
            )
            .unwrap();
        assert_eq!(found["id"], "list");
        let tools = found["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["name"], "send_reading");
        assert_eq!(tools[0]["inputSchema"]["additionalProperties"], false);
        assert!(server
            .handle(
                &json!({"jsonrpc":"2.0","method":"tools/call","params":{"name":"send_reading"}}),
                no_send
            )
            .is_none());
    }

    #[test]
    fn receipt_requires_operation_matching_id_and_both_committed_identifiers() {
        let body = json!({"version":"v1","operation":"send_reading","request_id":"known",
            "delivery_id":format!("delivery_{}", "a".repeat(32)),
            "document_id":format!("reader_{}", "b".repeat(32)), "created":true})
        .to_string();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        );
        assert!(parse_receipt(response.as_bytes(), "known").unwrap().created);
        assert!(matches!(
            parse_receipt(response.as_bytes(), "foreign"),
            Err(CallFailure::Uncertain)
        ));
        assert!(matches!(
            parse_receipt(
                b"HTTP/1.1 200 OK\r\nContent-Length: 15\r\n\r\n{\"status\":\"ok\"}",
                "known"
            ),
            Err(CallFailure::Uncertain)
        ));
        assert!(matches!(
            parse_receipt(b"HTTP/1.1 302 Found\r\nLocation: /\r\n\r\n", "known"),
            Err(CallFailure::Uncertain)
        ));
    }
}
