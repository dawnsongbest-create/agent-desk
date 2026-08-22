use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    application::{
        delivery_service::{DeliveryService, DeliveryServiceError},
        ports::delivery_repository::DeliveryRepositoryError,
        proposal_service::{ProposalService, ProposalServiceError},
        reading_service::{ReadingService, ReadingServiceError},
    },
    domain::{
        agent_connection::AgentCapabilityAction,
        delivery::{CreateDeliveryInput, IngestDeliveryResult},
        proposal::{AgentProposal, AgentProposalType, CreateAgentProposalInput},
        reader::{ReaderDocumentType, ReaderSourceType},
        reading::{CreateReadingPlanInput, ReadingDifficulty, ReadingPlan},
    },
};

pub const OPENCLAW_ADAPTER_ID: &str = "openclaw";
pub const OPENCLAW_SOURCE_LABEL: &str = "OpenClaw";

const OPENCLAW_ACTIONS: &[AgentCapabilityAction] = &[
    AgentCapabilityAction::CreateDelivery,
    AgentCapabilityAction::CreateProposal,
    AgentCapabilityAction::CreateReadingPlan,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpenClawDeliveryType {
    Research,
    Brief,
    Report,
}

impl OpenClawDeliveryType {
    const fn document_type(self) -> ReaderDocumentType {
        match self {
            Self::Research | Self::Report => ReaderDocumentType::Report,
            Self::Brief => ReaderDocumentType::Brief,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenClawDeliveryRequest {
    pub title: String,
    pub content: String,
    #[serde(rename = "type")]
    pub delivery_type: OpenClawDeliveryType,
    #[serde(default)]
    pub subtitle: Option<String>,
    #[serde(default)]
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum OpenClawProposalType {
    #[serde(rename = "TODO", alias = "todo")]
    Todo,
    #[serde(rename = "RECORD", alias = "record")]
    Record,
    #[serde(rename = "READING", alias = "reading")]
    Reading,
}

impl OpenClawProposalType {
    const fn domain_type(self) -> AgentProposalType {
        match self {
            Self::Todo => AgentProposalType::Todo,
            Self::Record => AgentProposalType::Record,
            Self::Reading => AgentProposalType::Reading,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct OpenClawProposalPayload {
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub estimated_minutes: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenClawProposalRequest {
    #[serde(rename = "type")]
    pub proposal_type: OpenClawProposalType,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub payload: OpenClawProposalPayload,
    #[serde(default)]
    pub source_delivery_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenClawReadingPlanRequest {
    pub title: String,
    pub daily_minutes: i64,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default = "default_schedule_time")]
    pub schedule_time: String,
    #[serde(default)]
    pub difficulty: OpenClawReadingDifficulty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum OpenClawReadingDifficulty {
    #[default]
    Normal,
    Technical,
}

impl OpenClawReadingDifficulty {
    const fn domain(self) -> ReadingDifficulty {
        match self {
            Self::Normal => ReadingDifficulty::Normal,
            Self::Technical => ReadingDifficulty::Technical,
        }
    }
}

fn default_schedule_time() -> String {
    "08:00".to_owned()
}

#[derive(Debug, Error)]
pub enum OpenClawAdapterError {
    #[error("OpenClaw request payload is invalid.")]
    InvalidPayload,
    #[error("OpenClaw request conflicts with an existing idempotency key.")]
    Conflict,
    #[error("OpenClaw application service is unavailable.")]
    Unavailable,
}

impl From<DeliveryServiceError> for OpenClawAdapterError {
    fn from(error: DeliveryServiceError) -> Self {
        match error {
            DeliveryServiceError::Validation(_) => Self::InvalidPayload,
            DeliveryServiceError::Repository(DeliveryRepositoryError::IdempotencyConflict) => {
                Self::Conflict
            }
            DeliveryServiceError::Repository(_) => Self::Unavailable,
        }
    }
}

impl From<ProposalServiceError> for OpenClawAdapterError {
    fn from(error: ProposalServiceError) -> Self {
        match error {
            ProposalServiceError::Validation(_)
            | ProposalServiceError::Sticky(_)
            | ProposalServiceError::Reading(_)
            | ProposalServiceError::InvalidStoredPayload => Self::InvalidPayload,
            ProposalServiceError::Repository(_) => Self::Unavailable,
        }
    }
}

impl From<ReadingServiceError> for OpenClawAdapterError {
    fn from(error: ReadingServiceError) -> Self {
        match error {
            ReadingServiceError::Validation(_) | ReadingServiceError::PlanNotActive => {
                Self::InvalidPayload
            }
            ReadingServiceError::Repository(_) | ReadingServiceError::Delivery(_) => {
                Self::Unavailable
            }
        }
    }
}

/// OpenClaw-specific input mapping. Persistence remains behind application
/// services; this adapter never receives a database connection or repository.
#[derive(Clone)]
pub struct OpenClawAdapter {
    deliveries: DeliveryService,
    proposals: ProposalService,
    reading: ReadingService,
}

impl OpenClawAdapter {
    pub fn new(
        deliveries: DeliveryService,
        proposals: ProposalService,
        reading: ReadingService,
    ) -> Self {
        Self {
            deliveries,
            proposals,
            reading,
        }
    }

    pub const fn adapter_id(&self) -> &'static str {
        OPENCLAW_ADAPTER_ID
    }

    pub const fn declared_actions(&self) -> &'static [AgentCapabilityAction] {
        OPENCLAW_ACTIONS
    }

    pub async fn create_delivery(
        &self,
        request: OpenClawDeliveryRequest,
    ) -> Result<IngestDeliveryResult, OpenClawAdapterError> {
        let idempotency_key = request
            .idempotency_key
            .unwrap_or_else(|| format!("openclaw:delivery:{}", Uuid::new_v4().simple()));
        Ok(self
            .deliveries
            .ingest(CreateDeliveryInput {
                idempotency_key,
                document_type: request.delivery_type.document_type(),
                title: request.title,
                subtitle: request.subtitle,
                content_markdown: request.content,
                source_type: ReaderSourceType::Agent,
                source_label: Some(OPENCLAW_SOURCE_LABEL.to_owned()),
                delivered_at: None,
            })
            .await?)
    }

    pub async fn create_proposal(
        &self,
        request: OpenClawProposalRequest,
    ) -> Result<AgentProposal, OpenClawAdapterError> {
        let content = request
            .payload
            .content
            .unwrap_or_else(|| request.description.clone());
        let payload: Value = match request.proposal_type {
            OpenClawProposalType::Todo | OpenClawProposalType::Record => {
                json!({ "content": content })
            }
            OpenClawProposalType::Reading => json!({
                "content": content,
                "estimatedMinutes": request.payload.estimated_minutes.unwrap_or(15),
            }),
        };
        Ok(self
            .proposals
            .create(CreateAgentProposalInput {
                proposal_type: request.proposal_type.domain_type(),
                title: request.title,
                description: request.description,
                payload,
                source_delivery_id: request.source_delivery_id,
            })
            .await?)
    }

    pub async fn create_reading_plan(
        &self,
        request: OpenClawReadingPlanRequest,
    ) -> Result<ReadingPlan, OpenClawAdapterError> {
        let content_markdown = request.content.unwrap_or_else(|| {
            format!("# {}\n\n由 OpenClaw 创建的阅读计划。", request.title.trim())
        });
        Ok(self
            .reading
            .create_plan(CreateReadingPlanInput {
                title: request.title,
                source_name: Some(OPENCLAW_SOURCE_LABEL.to_owned()),
                content_markdown,
                daily_minutes: request.daily_minutes,
                schedule_time: request.schedule_time,
                difficulty: request.difficulty.domain(),
            })
            .await?)
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenClawResponse<T> {
    pub version: &'static str,
    pub adapter: &'static str,
    pub agent_connection_id: String,
    pub data: T,
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::{
        application::ports::{
            delivery_repository::DeliveryRepository, proposal_repository::ProposalRepository,
            reading_repository::ReadingRepository,
        },
        domain::proposal::AgentProposalStatus,
        persistence::{
            delivery_repository::SqliteDeliveryRepository,
            proposal_repository::SqliteProposalRepository,
            reading_repository::SqliteReadingRepository, sqlite,
        },
    };

    #[tokio::test]
    async fn first_agent_workflow_uses_services_and_persists_after_restart() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("openclaw.sqlite3");
        let database = sqlite::connect(&path).await.unwrap();
        let deliveries = SqliteDeliveryRepository::new(database.0.clone());
        let proposals = SqliteProposalRepository::new(database.0.clone());
        let reading = SqliteReadingRepository::new(database.0.clone());
        let proposal_service = ProposalService::new(Arc::new(proposals.clone()));
        let adapter = OpenClawAdapter::new(
            DeliveryService::new(Arc::new(deliveries.clone())),
            proposal_service.clone(),
            ReadingService::new(Arc::new(reading.clone()), Arc::new(deliveries.clone())),
        );

        let delivery = adapter
            .create_delivery(OpenClawDeliveryRequest {
                title: "AI Daily Research".to_owned(),
                content: "今日 AI 动态：Transformer 架构仍值得深入学习。".to_owned(),
                delivery_type: OpenClawDeliveryType::Research,
                subtitle: Some("OpenClaw 每日研究".to_owned()),
                idempotency_key: Some("openclaw:first-demo".to_owned()),
            })
            .await
            .unwrap();
        assert!(delivery.created);
        assert_eq!(
            delivery.item.document.source_label.as_deref(),
            Some(OPENCLAW_SOURCE_LABEL)
        );

        let proposal = adapter
            .create_proposal(OpenClawProposalRequest {
                proposal_type: OpenClawProposalType::Reading,
                title: "深入阅读 Transformer Architecture".to_owned(),
                description: "建议阅读核心架构并形成学习记录。".to_owned(),
                payload: OpenClawProposalPayload {
                    content: Some("Transformer Architecture 核心概念".to_owned()),
                    estimated_minutes: Some(15),
                },
                source_delivery_id: Some(delivery.item.delivery.id.clone()),
            })
            .await
            .unwrap();
        let plan = adapter
            .create_reading_plan(OpenClawReadingPlanRequest {
                title: "Transformer Study".to_owned(),
                daily_minutes: 10,
                content: Some("# Transformer Study\n\n每天学习一个核心模块。".to_owned()),
                schedule_time: "08:00".to_owned(),
                difficulty: OpenClawReadingDifficulty::Technical,
            })
            .await
            .unwrap();
        let accepted = proposal_service.accept(&proposal.id).await.unwrap();
        let session = accepted.reading_session.unwrap().session;
        assert_eq!(accepted.proposal.status, AgentProposalStatus::Accepted);
        assert_eq!(session.estimated_minutes, 15);
        assert_eq!(plan.daily_minutes, 10);

        database.0.close().await;
        let reopened = sqlite::connect(&path).await.unwrap();
        let recovered_deliveries = SqliteDeliveryRepository::new(reopened.0.clone())
            .list_inbox()
            .await
            .unwrap();
        let recovered_proposals = SqliteProposalRepository::new(reopened.0.clone())
            .list_for_document(&delivery.item.document.id)
            .await
            .unwrap();
        let recovered_plans = SqliteReadingRepository::new(reopened.0.clone())
            .list_plans()
            .await
            .unwrap();
        let recovered_session = SqliteReadingRepository::new(reopened.0)
            .get_session(&session.id)
            .await
            .unwrap();
        assert_eq!(recovered_deliveries.len(), 1);
        assert_eq!(recovered_proposals.len(), 1);
        assert_eq!(recovered_proposals[0].status, AgentProposalStatus::Accepted);
        assert_eq!(recovered_plans.len(), 1);
        assert!(recovered_session.is_some());
    }
}
