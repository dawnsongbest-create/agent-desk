use serde::{Deserialize, Serialize};

pub const AGENT_API_VERSION: &str = "v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentConnectionStatus {
    Active,
    Inactive,
}

impl AgentConnectionStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Inactive => "inactive",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredAgentConnection {
    pub id: String,
    pub name: String,
    pub token_hash: Option<String>,
    pub status: AgentConnectionStatus,
    pub created_at: String,
    pub updated_at: String,
    pub last_used_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentConnection {
    pub id: String,
    pub name: String,
    pub status: String,
    pub has_token: bool,
    pub created_at: String,
    pub updated_at: String,
    pub last_used_at: Option<String>,
}

impl From<&StoredAgentConnection> for AgentConnection {
    fn from(connection: &StoredAgentConnection) -> Self {
        Self {
            id: connection.id.clone(),
            name: connection.name.clone(),
            status: connection.status.as_str().to_owned(),
            has_token: connection.token_hash.is_some(),
            created_at: connection.created_at.clone(),
            updated_at: connection.updated_at.clone(),
            last_used_at: connection.last_used_at.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AgentCapabilities {
    pub version: &'static str,
    pub delivery: bool,
    pub proposal: bool,
    pub reading: bool,
}

impl Default for AgentCapabilities {
    fn default() -> Self {
        Self {
            version: AGENT_API_VERSION,
            delivery: true,
            proposal: true,
            reading: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentCapabilityAction {
    CapabilityCheck,
    CreateDelivery,
    CreateProposal,
    CreateReadingPlan,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentRequest {
    pub version: String,
    pub action: AgentCapabilityAction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AgentResponse<T> {
    pub version: &'static str,
    pub data: T,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_contract_is_versioned_and_rejects_unknown_fields() {
        let request: AgentRequest = serde_json::from_value(serde_json::json!({
            "version": "v1",
            "action": "capability_check"
        }))
        .unwrap();
        assert_eq!(request.version, AGENT_API_VERSION);
        assert_eq!(request.action, AgentCapabilityAction::CapabilityCheck);
        assert!(serde_json::from_value::<AgentRequest>(serde_json::json!({
            "version": "v1",
            "agent_id": "openclaw",
            "action": "capability_check"
        }))
        .is_err());
    }
}
