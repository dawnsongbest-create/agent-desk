use serde::{Deserialize, Serialize};

use super::AgentProposalStatus;

pub const MAX_TODO_BODY: usize = 64 * 1024;
pub const MAX_TODO_ITEMS: usize = 20;
pub const MAX_REASON_CHARS: usize = 200;
pub const MAX_REQUEST_KEY_CHARS: usize = 128;

// Variant order is the approved rank; sorting must remain stable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TodoPriority {
    High,
    Medium,
    Low,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TodoSuggestionItem {
    pub text: String,
    pub priority: TodoPriority,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due_date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TodoSuggestionRequest {
    pub items: Vec<TodoSuggestionItem>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
}

pub fn valid_request_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= MAX_REQUEST_KEY_CHARS
        && key
            .bytes()
            .all(|ch| ch.is_ascii_alphanumeric() || b"-_:.~".contains(&ch))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TodoSuggestionSource {
    Codex,
    Dumate,
    Chatgpt,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TodoSuggestionBatch {
    pub id: String,
    pub source: TodoSuggestionSource,
    pub items: Vec<TodoSuggestionItem>,
    pub status: AgentProposalStatus,
    pub created_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptedTodoBatch {
    pub todo_ids: Vec<String>,
}
