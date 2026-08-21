use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentProposalType {
    Todo,
    Record,
    Reading,
}

impl AgentProposalType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Todo => "todo",
            Self::Record => "record",
            Self::Reading => "reading",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentProposalStatus {
    Pending,
    Accepted,
    Rejected,
}

impl AgentProposalStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentProposal {
    pub id: String,
    #[serde(rename = "type")]
    pub proposal_type: AgentProposalType,
    pub title: String,
    pub description: String,
    pub payload_json: String,
    pub source_delivery_id: Option<String>,
    pub status: AgentProposalStatus,
    pub created_at: String,
    pub resolved_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAgentProposalInput {
    #[serde(rename = "type")]
    pub proposal_type: AgentProposalType,
    pub title: String,
    pub description: String,
    pub payload: Value,
    pub source_delivery_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct TodoProposalPayload {
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct RecordProposalPayload {
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct ReadingProposalPayload {
    pub content: String,
    pub estimated_minutes: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewAgentProposal {
    pub proposal_type: AgentProposalType,
    pub title: String,
    pub description: String,
    pub payload_json: String,
    pub source_delivery_id: Option<String>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ProposalValidationError {
    #[error("Proposal title cannot be empty.")]
    EmptyTitle,
    #[error("Proposal title is too long.")]
    TitleTooLong,
    #[error("Proposal description cannot be empty.")]
    EmptyDescription,
    #[error("Proposal description is too long.")]
    DescriptionTooLong,
    #[error("Proposal payload does not match its type.")]
    InvalidPayload,
    #[error("Reading proposals require a source delivery.")]
    MissingReadingSource,
    #[error("Estimated reading minutes must be between 1 and 240.")]
    InvalidEstimatedMinutes,
}

pub fn validate_proposal(
    mut input: CreateAgentProposalInput,
) -> Result<NewAgentProposal, ProposalValidationError> {
    input.title = input.title.trim().to_owned();
    input.description = input.description.trim().to_owned();
    if input.title.is_empty() {
        return Err(ProposalValidationError::EmptyTitle);
    }
    if input.title.chars().count() > 500 {
        return Err(ProposalValidationError::TitleTooLong);
    }
    if input.description.is_empty() {
        return Err(ProposalValidationError::EmptyDescription);
    }
    if input.description.chars().count() > 4_000 {
        return Err(ProposalValidationError::DescriptionTooLong);
    }
    if input.proposal_type == AgentProposalType::Reading && input.source_delivery_id.is_none() {
        return Err(ProposalValidationError::MissingReadingSource);
    }
    let payload_json = match input.proposal_type {
        AgentProposalType::Todo => serde_json::to_string(
            &serde_json::from_value::<TodoProposalPayload>(input.payload)
                .map_err(|_| ProposalValidationError::InvalidPayload)?,
        )
        .map_err(|_| ProposalValidationError::InvalidPayload)?,
        AgentProposalType::Record => serde_json::to_string(
            &serde_json::from_value::<RecordProposalPayload>(input.payload)
                .map_err(|_| ProposalValidationError::InvalidPayload)?,
        )
        .map_err(|_| ProposalValidationError::InvalidPayload)?,
        AgentProposalType::Reading => {
            let payload = serde_json::from_value::<ReadingProposalPayload>(input.payload)
                .map_err(|_| ProposalValidationError::InvalidPayload)?;
            if !(1..=240).contains(&payload.estimated_minutes) {
                return Err(ProposalValidationError::InvalidEstimatedMinutes);
            }
            serde_json::to_string(&payload).map_err(|_| ProposalValidationError::InvalidPayload)?
        }
    };

    Ok(NewAgentProposal {
        proposal_type: input.proposal_type,
        title: input.title,
        description: input.description,
        payload_json,
        source_delivery_id: input.source_delivery_id,
    })
}
