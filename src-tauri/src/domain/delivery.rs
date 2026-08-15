use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::domain::reader::{
    validate_document, NewReaderDocument, ReaderDocument, ReaderDocumentType, ReaderSourceType,
    ReaderValidationError,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDeliveryInput {
    pub idempotency_key: String,
    pub document_type: ReaderDocumentType,
    pub title: String,
    pub subtitle: Option<String>,
    pub content_markdown: String,
    pub source_type: ReaderSourceType,
    pub source_label: Option<String>,
    pub delivered_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Delivery {
    pub id: String,
    pub document_id: String,
    pub idempotency_key: String,
    pub delivered_at: String,
    pub opened_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InboxDelivery {
    pub delivery: Delivery,
    pub document: ReaderDocument,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IngestDeliveryResult {
    pub item: InboxDelivery,
    pub created: bool,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DeliveryValidationError {
    #[error("Delivery idempotency key cannot be empty.")]
    EmptyIdempotencyKey,
    #[error("Delivery idempotency key is too long.")]
    IdempotencyKeyTooLong,
    #[error("Delivery timestamp cannot be empty.")]
    EmptyDeliveredAt,
    #[error("Delivery timestamp is too long.")]
    DeliveredAtTooLong,
    #[error(transparent)]
    Reader(#[from] ReaderValidationError),
}

pub fn validate_delivery(
    mut input: CreateDeliveryInput,
) -> Result<CreateDeliveryInput, DeliveryValidationError> {
    input.idempotency_key = input.idempotency_key.trim().to_owned();
    if input.idempotency_key.is_empty() {
        return Err(DeliveryValidationError::EmptyIdempotencyKey);
    }
    if input.idempotency_key.chars().count() > 500 {
        return Err(DeliveryValidationError::IdempotencyKeyTooLong);
    }
    if let Some(delivered_at) = input.delivered_at.take() {
        let delivered_at = delivered_at.trim().to_owned();
        if delivered_at.is_empty() {
            return Err(DeliveryValidationError::EmptyDeliveredAt);
        }
        if delivered_at.chars().count() > 100 {
            return Err(DeliveryValidationError::DeliveredAtTooLong);
        }
        input.delivered_at = Some(delivered_at);
    }
    let document = validate_document(NewReaderDocument {
        id: None,
        document_type: input.document_type,
        title: input.title,
        subtitle: input.subtitle,
        content_markdown: input.content_markdown,
        source_type: input.source_type,
        source_label: input.source_label,
    })?;
    input.title = document.title;
    input.subtitle = document.subtitle;
    input.content_markdown = document.content_markdown;
    input.source_label = document.source_label;
    Ok(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_normalizes_envelope_without_truncating_content() {
        let content = "长文".repeat(3_100);
        let validated = validate_delivery(CreateDeliveryInput {
            idempotency_key: "  delivery-a  ".to_owned(),
            document_type: ReaderDocumentType::Brief,
            title: "  每日简报  ".to_owned(),
            subtitle: None,
            content_markdown: content.clone(),
            source_type: ReaderSourceType::Agent,
            source_label: Some(" Agent Desk ".to_owned()),
            delivered_at: None,
        })
        .unwrap();
        assert_eq!(validated.idempotency_key, "delivery-a");
        assert_eq!(validated.title, "每日简报");
        assert_eq!(validated.content_markdown, content);
        assert_eq!(validated.source_label.as_deref(), Some("Agent Desk"));
    }
}
