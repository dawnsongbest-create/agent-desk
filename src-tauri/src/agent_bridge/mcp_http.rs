use std::{
    net::SocketAddr,
    sync::atomic::{AtomicBool, Ordering},
};

use serde::Deserialize;
use serde_json::{json, value::RawValue, Value};

use super::dumate::{DuMate, Scopes};
use crate::codex_delivery::{mcp, CodexError, MAX_CODEX_BODY};

pub(super) const PATH: &str = "/mcp";
pub(super) const MAX_BODY: usize = MAX_CODEX_BODY + 8 * 1024;
const VERSION: &str = "2025-11-25";
const FALLBACK: &str = "2025-03-26";

type Response = (u16, String);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Rpc<'a> {
    jsonrpc: &'a str,
    method: &'a str,
    #[serde(default, borrow)]
    id: RpcId<'a>,
    #[serde(default, borrow)]
    params: Option<&'a RawValue>,
}

#[derive(Default)]
struct RpcId<'a>(Option<&'a RawValue>);

impl<'de: 'a, 'a> Deserialize<'de> for RpcId<'a> {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        // Distinguish a notification (no ID) from an explicitly invalid null ID.
        <&'de RawValue>::deserialize(d).map(|raw| Self(Some(raw)))
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Initialize<'a> {
    protocol_version: &'a str,
    capabilities: serde_json::Map<String, Value>,
    client_info: ClientInfo<'a>,
}

#[derive(Deserialize)]
struct ClientInfo<'a> {
    name: &'a str,
    version: &'a str,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ToolCall<'a> {
    name: &'a str,
    #[serde(borrow)]
    arguments: &'a RawValue,
    #[serde(default, rename = "_meta")]
    _meta: Option<serde_json::Map<String, Value>>,
}

fn supported(version: &str) -> bool {
    matches!(version, VERSION | "2025-06-18" | FALLBACK)
}

// Shared HTTP parser leaves legacy routes untouched. These checks are specific
// to MCP: no proxy forwarding, origin relaxation, DNS hostnames or CORS.
pub(super) fn validate_headers(
    headers: &[(String, String)],
    address: SocketAddr,
) -> Result<(), u16> {
    for name in [
        "host",
        "origin",
        "authorization",
        "accept",
        "content-type",
        "content-length",
        "transfer-encoding",
        "mcp-protocol-version",
        "mcp-session-id",
        "expect",
        "upgrade",
    ] {
        if headers
            .iter()
            .filter(|(key, _)| key.eq_ignore_ascii_case(name))
            .count()
            > 1
        {
            return Err(400);
        }
    }
    let authority = address.to_string();
    if header(headers, "host") != Some(authority.as_str()) {
        return Err(403);
    }
    if header(headers, "origin").is_some_and(|origin| origin != format!("http://{authority}")) {
        return Err(403);
    }
    if header(headers, "mcp-session-id").is_some()
        || header(headers, "upgrade").is_some()
        || header(headers, "expect").is_some()
        || headers.iter().any(|(name, _)| {
            let name = name.to_ascii_lowercase();
            name == "forwarded" || name.starts_with("x-forwarded-")
        })
    {
        return Err(400);
    }
    Ok(())
}

pub(super) fn header<'a>(headers: &'a [(String, String)], target: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(target))
        .map(|(_, value)| value.as_str())
}

pub(super) fn http_error(status: u16) -> Response {
    let error = match status {
        401 => "unauthorized",
        403 => "forbidden",
        405 => "method_not_allowed",
        406 => "not_acceptable",
        413 => "payload_too_large",
        415 => "unsupported_media_type",
        503 => "bridge_unavailable",
        _ => "invalid_request",
    };
    (status, json!({ "error": error }).to_string())
}

fn media_type(value: &str, expected: &str) -> bool {
    value
        .split(';')
        .next()
        .is_some_and(|kind| kind.trim().eq_ignore_ascii_case(expected))
}

fn accepts(value: &str, expected: &str) -> bool {
    value.split(',').any(|item| {
        media_type(item, expected)
            && !item.split(';').skip(1).any(|parameter| {
                parameter.split_once('=').is_some_and(|(name, value)| {
                    name.trim().eq_ignore_ascii_case("q")
                        && !value
                            .trim()
                            .parse::<f64>()
                            .is_ok_and(|quality| quality > 0.0 && quality <= 1.0)
                })
            })
    })
}

pub(super) async fn respond(
    dumate: &DuMate,
    accepting: &AtomicBool,
    method: &str,
    headers: &[(String, String)],
    body: &[u8],
) -> Response {
    let Some(token) = header(headers, "authorization")
        .and_then(|value| value.split_once(' '))
        .filter(|(scheme, _)| scheme.eq_ignore_ascii_case("bearer"))
        .map(|(_, token)| token)
    else {
        return http_error(401);
    };
    // Hold this lock through dispatch, not network output. Stop closes this
    // listener's admission flag and drains any already-admitted commit.
    let _guard = dumate.gate.lock().await;
    if !accepting.load(Ordering::Acquire) {
        return http_error(503);
    }
    let grant = match dumate.authenticate(token).await {
        Ok(grant) => grant,
        Err(error) => return http_error(error.http().0),
    };
    if method != "POST" {
        return http_error(405);
    }
    if !header(headers, "content-type").is_some_and(|value| media_type(value, "application/json")) {
        return http_error(415);
    }
    if !header(headers, "accept").is_some_and(|value| {
        accepts(value, "application/json") && accepts(value, "text/event-stream")
    }) {
        return http_error(406);
    }
    let header_version = header(headers, "mcp-protocol-version");
    if !supported(header_version.unwrap_or(FALLBACK)) {
        return http_error(400);
    }
    // Check JSON syntax independently so a valid but malformed JSON-RPC
    // envelope is reported as Invalid Request, not Parse Error.
    if serde_json::from_slice::<&RawValue>(body).is_err() {
        return rpc_error(Value::Null, -32700, "Parse error");
    }
    let Ok(rpc) = serde_json::from_slice::<Rpc<'_>>(body) else {
        return rpc_error(Value::Null, -32600, "Invalid Request");
    };
    let id = match rpc.id.0 {
        None => None,
        Some(raw) => match serde_json::from_str::<Value>(raw.get()) {
            Ok(id)
                if id.is_i64() || id.is_u64() || id.as_str().is_some_and(|id| id.len() <= 256) =>
            {
                Some(id)
            }
            _ => return rpc_error(Value::Null, -32600, "Invalid Request"),
        },
    };
    if rpc.jsonrpc != "2.0" || rpc.method.is_empty() {
        return rpc_error(id.unwrap_or(Value::Null), -32600, "Invalid Request");
    }
    let Some(id) = id else {
        // This stateless server never calls clients. No ID-less message can
        // execute a tool, regardless of type or content.
        return (202, String::new());
    };
    let params = rpc.params.map(RawValue::get).unwrap_or("{}");
    let result = match rpc.method {
        "initialize" => {
            let Ok(init) = serde_json::from_str::<Initialize<'_>>(params) else {
                return rpc_error(id, -32602, "Invalid initialize parameters");
            };
            if init.client_info.name.trim().is_empty()
                || init.client_info.version.trim().is_empty()
                || init.protocol_version.trim().is_empty()
                || init.capabilities.len() > 64
                || header_version.is_some_and(|version| version != init.protocol_version)
            {
                return rpc_error(id, -32602, "Invalid initialize parameters");
            }
            let negotiated = if supported(init.protocol_version) {
                init.protocol_version
            } else {
                VERSION
            };
            json!({
                "protocolVersion": negotiated, "capabilities": { "tools": {} },
                "serverInfo": { "name": "agent-desk-dumate", "version": env!("CARGO_PKG_VERSION") },
            })
        }
        "tools/list" => {
            let Ok(params) = serde_json::from_str::<serde_json::Map<String, Value>>(params) else {
                return rpc_error(id, -32602, "Invalid tool list parameters");
            };
            if params.contains_key("cursor") {
                return rpc_error(id, -32602, "Unknown cursor");
            }
            json!({ "tools": tools(grant.scopes()) })
        }
        "tools/call" => {
            let Ok(call) = serde_json::from_str::<ToolCall<'_>>(params) else {
                return rpc_error(id, -32602, "Invalid tool call parameters");
            };
            if !matches!(call.name, "send_reading" | "suggest_todos") {
                return rpc_error(id, -32602, "Unknown tool");
            }
            match dumate.invoke(&grant, call.name, call.arguments).await {
                Ok(receipt) => {
                    let text = receipt.to_string();
                    let mut result = json!({ "content": [{ "type": "text", "text": text }] });
                    if header_version != Some(FALLBACK) && header_version.is_some() {
                        result["structuredContent"] = receipt;
                    }
                    result
                }
                Err(CodexError::TooLarge) => return http_error(413),
                Err(CodexError::Unauthorized) => return http_error(401),
                Err(error) => {
                    let code = match error {
                        CodexError::Forbidden => "forbidden_capability",
                        CodexError::Conflict => "idempotency_conflict",
                        CodexError::InvalidPayload => "invalid_payload",
                        _ => "bridge_unavailable",
                    };
                    json!({ "isError": true, "content": [ {
                        "type": "text", "text": json!({ "error":code }).to_string(),
                    }] })
                }
            }
        }
        "ping" => json!({}),
        _ => return rpc_error(id, -32601, "Method not found"),
    };
    (
        200,
        json!({ "jsonrpc": "2.0", "id": id, "result": result }).to_string(),
    )
}

fn rpc_error(id: Value, code: i32, message: &str) -> Response {
    (
        200,
        json!({
            "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message },
        })
        .to_string(),
    )
}

fn tools(scopes: Scopes) -> Vec<Value> {
    [(scopes.reading, mcp::tool()), (scopes.todo, mcp::todo_tool())]
        .into_iter()
        .filter(|(allowed, _)| *allowed)
        .map(|(_, mut tool)| {
            tool["inputSchema"]["required"].as_array_mut()
                .expect("static MCP tool schema").push(json!("idempotency_key"));
            tool["annotations"]["idempotentHint"] = json!(true);
            tool["description"] = if tool["name"] == "send_reading" {
                json!("Send a Markdown document to Agent Desk Inbox only when asked. Supply a new idempotency_key for a new send; reuse it only for a retry.")
            } else {
                json!("Submit 1–20 Todo suggestions for user review. No Todo is created until the user confirms. Supply a new idempotency_key for a new batch; reuse it only for a retry.")
            };
            tool
        })
        .collect()
}
