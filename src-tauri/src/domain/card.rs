use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardType {
    Note,
    Task,
    Reading,
    AgentMessage,
}

impl CardType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Note => "note",
            Self::Task => "task",
            Self::Reading => "reading",
            Self::AgentMessage => "agent_message",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardLifecycle {
    Active,
    Completed,
    Archived,
    Deleted,
}

impl CardLifecycle {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Completed => "completed",
            Self::Archived => "archived",
            Self::Deleted => "deleted",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewBaseCard {
    pub id: String,
    pub card_type: CardType,
    pub title: Option<String>,
    pub source_kind: String,
    pub source_agent_id: Option<String>,
    pub source_delivery_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct CardRecord {
    pub id: String,
    pub card_type: String,
    pub title: Option<String>,
    pub lifecycle: String,
    pub attention: Option<String>,
    pub source_kind: String,
    pub source_agent_id: Option<String>,
    pub source_delivery_id: Option<String>,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
    pub archived_at: Option<String>,
    pub deleted_at: Option<String>,
}
