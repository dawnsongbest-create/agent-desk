use std::sync::Arc;

use serde_json::from_str;
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    application::{
        ports::proposal_repository::{
            AcceptProposalResult, ProposalAcceptance, ProposalRepository, ProposalRepositoryError,
        },
        reading_service::{ReadingService, ReadingServiceError},
        sticky_service::{StickyService, StickyServiceError},
    },
    domain::{
        proposal::{
            todo_batch::{
                valid_request_key, AcceptedTodoBatch, TodoSuggestionBatch, TodoSuggestionItem,
                MAX_REASON_CHARS, MAX_TODO_ITEMS,
            },
            validate_proposal, AgentProposal, AgentProposalType, CreateAgentProposalInput,
            ProposalValidationError, ReadingProposalPayload, RecordProposalPayload,
            TodoProposalPayload,
        },
        sticky::StickyCardKind,
    },
};

#[derive(Debug, Error)]
pub enum ProposalServiceError {
    #[error(transparent)]
    Validation(#[from] ProposalValidationError),
    #[error(transparent)]
    Sticky(#[from] StickyServiceError),
    #[error(transparent)]
    Reading(#[from] ReadingServiceError),
    #[error(transparent)]
    Repository(#[from] ProposalRepositoryError),
    #[error("Proposal payload could not be read.")]
    InvalidStoredPayload,
}

#[derive(Clone)]
pub struct ProposalService {
    repository: Arc<dyn ProposalRepository>,
}

impl ProposalService {
    pub fn new(repository: Arc<dyn ProposalRepository>) -> Self {
        Self { repository }
    }

    pub async fn create(
        &self,
        input: CreateAgentProposalInput,
    ) -> Result<AgentProposal, ProposalServiceError> {
        let mut proposal = validate_proposal(input)?;
        proposal.payload_json = match proposal.proposal_type {
            AgentProposalType::Todo => {
                let payload: TodoProposalPayload = from_str(&proposal.payload_json)
                    .map_err(|_| ProposalServiceError::InvalidStoredPayload)?;
                let card =
                    StickyService::prepare_create(StickyCardKind::Task, payload.content, None)?;
                serde_json::to_string(&TodoProposalPayload { content: card.text })
                    .map_err(|_| ProposalServiceError::InvalidStoredPayload)?
            }
            AgentProposalType::Record => {
                let payload: RecordProposalPayload = from_str(&proposal.payload_json)
                    .map_err(|_| ProposalServiceError::InvalidStoredPayload)?;
                let card =
                    StickyService::prepare_create(StickyCardKind::Note, payload.content, None)?;
                serde_json::to_string(&RecordProposalPayload { content: card.text })
                    .map_err(|_| ProposalServiceError::InvalidStoredPayload)?
            }
            AgentProposalType::Reading => {
                let payload: ReadingProposalPayload = from_str(&proposal.payload_json)
                    .map_err(|_| ProposalServiceError::InvalidStoredPayload)?;
                let (content, estimated_minutes) = ReadingService::prepare_session(
                    payload.content,
                    Some(payload.estimated_minutes),
                )?;
                serde_json::to_string(&ReadingProposalPayload {
                    content,
                    estimated_minutes,
                })
                .map_err(|_| ProposalServiceError::InvalidStoredPayload)?
            }
        };
        Ok(self.repository.create(&proposal).await?)
    }

    pub async fn list_for_document(
        &self,
        document_id: &str,
    ) -> Result<Vec<AgentProposal>, ProposalServiceError> {
        Ok(self.repository.list_for_document(document_id).await?)
    }

    pub async fn accept(&self, id: &str) -> Result<AcceptProposalResult, ProposalServiceError> {
        let proposal = self
            .repository
            .get(id)
            .await?
            .ok_or(ProposalRepositoryError::NotFound)?;
        let action = match proposal.proposal_type {
            AgentProposalType::Todo => {
                let payload: TodoProposalPayload = from_str(&proposal.payload_json)
                    .map_err(|_| ProposalServiceError::InvalidStoredPayload)?;
                ProposalAcceptance::Sticky(StickyService::prepare_create(
                    StickyCardKind::Task,
                    payload.content,
                    None,
                )?)
            }
            AgentProposalType::Record => {
                let payload: RecordProposalPayload = from_str(&proposal.payload_json)
                    .map_err(|_| ProposalServiceError::InvalidStoredPayload)?;
                ProposalAcceptance::Sticky(StickyService::prepare_create(
                    StickyCardKind::Note,
                    payload.content,
                    None,
                )?)
            }
            AgentProposalType::Reading => {
                let payload: ReadingProposalPayload = from_str(&proposal.payload_json)
                    .map_err(|_| ProposalServiceError::InvalidStoredPayload)?;
                let source_delivery_id = proposal
                    .source_delivery_id
                    .ok_or(ProposalValidationError::MissingReadingSource)?;
                let (content, estimated_minutes) = ReadingService::prepare_session(
                    payload.content,
                    Some(payload.estimated_minutes),
                )?;
                ProposalAcceptance::Reading {
                    source_delivery_id,
                    content,
                    estimated_minutes,
                }
            }
        };
        Ok(self.repository.accept(id, action).await?)
    }

    pub async fn reject(&self, id: &str) -> Result<AgentProposal, ProposalServiceError> {
        Ok(self.repository.reject(id).await?)
    }

    pub fn prepare_todo_items(
        mut items: Vec<TodoSuggestionItem>,
    ) -> Result<Vec<TodoSuggestionItem>, ProposalServiceError> {
        if !(1..=MAX_TODO_ITEMS).contains(&items.len()) {
            return Err(ProposalValidationError::InvalidPayload.into());
        }
        for item in &mut items {
            if item.text.contains('\0')
                || item.reason.as_ref().is_some_and(|reason| {
                    reason.contains('\0') || reason.chars().count() > MAX_REASON_CHARS
                })
            {
                return Err(ProposalValidationError::InvalidPayload.into());
            }
            let card = StickyService::prepare_create(
                StickyCardKind::Task,
                std::mem::take(&mut item.text),
                item.due_date.take(),
            )?;
            item.text = card.text;
            item.due_date = card.due_date;
            item.reason = item
                .reason
                .take()
                .map(|reason| reason.trim().to_owned())
                .filter(|reason| !reason.is_empty());
        }
        items.sort_by_key(|item| item.priority);
        Ok(items)
    }

    pub async fn submit_todo_batch(
        &self,
        namespace: &str,
        key: &str,
        items: Vec<TodoSuggestionItem>,
    ) -> Result<(TodoSuggestionBatch, bool), ProposalServiceError> {
        if !valid_request_key(key) {
            return Err(ProposalValidationError::InvalidPayload.into());
        }
        let items = Self::prepare_todo_items(items)?;
        let digest = format!("{:x}", Sha256::digest(key.as_bytes()));
        Ok(self
            .repository
            .submit_todo_batch(namespace, &digest, &items)
            .await?)
    }

    pub async fn list_pending_todo_batches(
        &self,
    ) -> Result<Vec<TodoSuggestionBatch>, ProposalServiceError> {
        Ok(self.repository.list_pending_todo_batches().await?)
    }

    pub async fn accept_todo_batch(
        &self,
        id: &str,
    ) -> Result<AcceptedTodoBatch, ProposalServiceError> {
        Ok(self.repository.accept_todo_batch(id).await?)
    }

    pub async fn reject_todo_batch(&self, id: &str) -> Result<(), ProposalServiceError> {
        Ok(self.repository.reject_todo_batch(id).await?)
    }
}
