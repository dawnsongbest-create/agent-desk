use crate::domain::agent_connection::AgentCapabilities;

#[derive(Debug, Default)]
pub struct CapabilityService;

impl CapabilityService {
    pub fn discover(&self) -> AgentCapabilities {
        AgentCapabilities::default()
    }
}
