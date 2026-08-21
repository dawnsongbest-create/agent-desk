use async_trait::async_trait;
use thiserror::Error;

use crate::domain::{
    proposal::{AgentProposal, NewAgentProposal},
    reading::ReadingSessionResult,
    sticky::{NewStickyCard, StickyCard},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProposalAcceptance {
    Sticky(NewStickyCard),
    Reading {
        source_delivery_id: String,
        content: String,
        estimated_minutes: i64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptProposalResult {
    pub proposal: AgentProposal,
    pub card: Option<StickyCard>,
    pub reading_session: Option<ReadingSessionResult>,
}

#[derive(Debug, Error)]
pub enum ProposalRepositoryError {
    #[error("Agent proposal was not found.")]
    NotFound,
    #[error("Agent proposal has already been resolved.")]
    AlreadyResolved,
    #[error("The proposal source delivery was not found.")]
    SourceNotFound,
    #[error("Agent proposal storage failed: {0}")]
    Storage(String),
}

impl From<sqlx::Error> for ProposalRepositoryError {
    fn from(error: sqlx::Error) -> Self {
        Self::Storage(error.to_string())
    }
}

#[async_trait]
pub trait ProposalRepository: Send + Sync {
    async fn create(
        &self,
        proposal: &NewAgentProposal,
    ) -> Result<AgentProposal, ProposalRepositoryError>;
    async fn list_for_document(
        &self,
        document_id: &str,
    ) -> Result<Vec<AgentProposal>, ProposalRepositoryError>;
    async fn get(&self, id: &str) -> Result<Option<AgentProposal>, ProposalRepositoryError>;
    async fn accept(
        &self,
        id: &str,
        action: ProposalAcceptance,
    ) -> Result<AcceptProposalResult, ProposalRepositoryError>;
    async fn reject(&self, id: &str) -> Result<AgentProposal, ProposalRepositoryError>;
}
