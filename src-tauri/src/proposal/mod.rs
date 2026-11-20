use std::sync::Arc;

use serde::Deserialize;
use tauri::{AppHandle, Emitter, State};

use crate::{
    application::{
        ports::proposal_repository::{AcceptProposalResult, ProposalRepositoryError},
        proposal_service::{ProposalService, ProposalServiceError},
    },
    codex_delivery::{todo_suggestions_enabled, TODO_SUGGESTIONS_CHANGED},
    domain::proposal::{
        todo_batch::{AcceptedTodoBatch, TodoSuggestionBatch},
        AgentProposal, CreateAgentProposalInput,
    },
    persistence::proposal_repository::SqliteProposalRepository,
};

pub struct ProposalState(ProposalService);
const TODO_CARDS_CHANGED: &str = "agent-desk://todo-cards-changed";

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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResolveTodoBatchInput {
    batch_id: String,
}

fn todo_error(error: ProposalServiceError) -> String {
    match error {
        ProposalServiceError::Repository(ProposalRepositoryError::NotFound) => "该批建议已不存在。",
        ProposalServiceError::Repository(ProposalRepositoryError::AlreadyResolved) => {
            "该批建议已处理，请刷新列表。"
        }
        _ => "无法确认操作结果，请刷新后重试。重试同一批次不会重复创建待办。",
    }
    .to_owned()
}

#[tauri::command]
pub async fn list_pending_todo_batches(
    state: State<'_, ProposalState>,
) -> Result<Vec<TodoSuggestionBatch>, String> {
    if !todo_suggestions_enabled() {
        return Ok(Vec::new());
    }
    state
        .0
        .list_pending_todo_batches()
        .await
        .map_err(todo_error)
}

#[tauri::command]
pub async fn accept_todo_batch(
    app: AppHandle,
    state: State<'_, ProposalState>,
    input: ResolveTodoBatchInput,
) -> Result<AcceptedTodoBatch, String> {
    if !todo_suggestions_enabled() {
        return Err("待办建议已禁用。".to_owned());
    }
    let result = state
        .0
        .accept_todo_batch(&input.batch_id)
        .await
        .map_err(todo_error)?;
    let _ = app.emit(TODO_SUGGESTIONS_CHANGED, ());
    let _ = app.emit(TODO_CARDS_CHANGED, ());
    Ok(result)
}

#[tauri::command]
pub async fn reject_todo_batch(
    app: AppHandle,
    state: State<'_, ProposalState>,
    input: ResolveTodoBatchInput,
) -> Result<(), String> {
    if !todo_suggestions_enabled() {
        return Err("待办建议已禁用。".to_owned());
    }
    state
        .0
        .reject_todo_batch(&input.batch_id)
        .await
        .map_err(todo_error)?;
    let _ = app.emit(TODO_SUGGESTIONS_CHANGED, ());
    Ok(())
}
