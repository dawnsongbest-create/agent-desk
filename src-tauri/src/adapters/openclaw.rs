use crate::domain::agent_connection::AgentCapabilityAction;

pub const OPENCLAW_ADAPTER_ID: &str = "openclaw";

/// Contract placeholder for the first external adapter. M3-C1 deliberately
/// provides no transport or workspace mutation implementation.
pub trait OpenClawAdapter: Send + Sync {
    fn adapter_id(&self) -> &'static str {
        OPENCLAW_ADAPTER_ID
    }

    fn declared_actions(&self) -> &'static [AgentCapabilityAction];
}
