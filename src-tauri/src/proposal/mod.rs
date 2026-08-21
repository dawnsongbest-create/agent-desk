use std::sync::Arc;

use serde::Deserialize;
use tauri::State;

use crate::{
    application::{
        ports::proposal_repository::AcceptProposalResult,
        proposal_service::{ProposalService, ProposalServiceError},
    },
    domain::proposal::{AgentProposal, CreateAgentProposalInput},
    persistence::proposal_repository::SqliteProposalRepository,
};

pub struct ProposalState(ProposalService);

impl ProposalState {
    pub fn new(repository: SqliteProposalRepository) -> Self {
        Self(ProposalService::new(Arc::new(repository)))
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentProposalInput {
    document_id: String,
}

#[derive(Debug, Deserialize)]
pub struct ResolveProposalInput {
    id: String,
}

fn command_error(error: ProposalServiceError) -> String {
    error.to_string()
}

#[tauri::command]
pub async fn create_agent_proposal(
    state: State<'_, ProposalState>,
    input: CreateAgentProposalInput,
) -> Result<AgentProposal, String> {
    state.0.create(input).await.map_err(command_error)
}

#[tauri::command]
pub async fn list_agent_proposals(
    state: State<'_, ProposalState>,
    input: DocumentProposalInput,
) -> Result<Vec<AgentProposal>, String> {
    state
        .0
        .list_for_document(&input.document_id)
        .await
        .map_err(command_error)
}

#[tauri::command]
pub async fn accept_agent_proposal(
    state: State<'_, ProposalState>,
    input: ResolveProposalInput,
) -> Result<AcceptProposalResult, String> {
    state.0.accept(&input.id).await.map_err(command_error)
}

#[tauri::command]
pub async fn reject_agent_proposal(
    state: State<'_, ProposalState>,
    input: ResolveProposalInput,
) -> Result<AgentProposal, String> {
    state.0.reject(&input.id).await.map_err(command_error)
}
