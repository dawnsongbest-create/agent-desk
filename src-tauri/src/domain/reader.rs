use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::domain::sticky::StickyCard;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReaderDocumentType {
    Article,
    Brief,
    Reading,
    Report,
}

impl ReaderDocumentType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Article => "article",
            Self::Brief => "brief",
            Self::Reading => "reading",
            Self::Report => "report",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReaderSourceType {
    Local,
    Builtin,
    Agent,
}

impl ReaderSourceType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Builtin => "builtin",
            Self::Agent => "agent",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReaderDocument {
    pub id: String,
    pub document_type: ReaderDocumentType,
    pub title: String,
    pub subtitle: Option<String>,
    pub content_markdown: String,
    pub source_type: ReaderSourceType,
    pub source_label: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewReaderDocument {
    pub id: Option<String>,
    pub document_type: ReaderDocumentType,
    pub title: String,
    pub subtitle: Option<String>,
    pub content_markdown: String,
    pub source_type: ReaderSourceType,
    pub source_label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct RecordSourceRef {
    pub record_id: String,
    pub document_id: String,
    pub source_type: String,
    pub selected_text: String,
    pub document_title_snapshot: String,
    pub captured_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectionCaptureResult {
    pub record: StickyCard,
    pub source_ref: RecordSourceRef,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ReaderValidationError {
    #[error("Reader document title cannot be empty.")]
    EmptyTitle,
    #[error("Reader document content cannot be empty.")]
    EmptyContent,
    #[error("Reader document content exceeds the supported size.")]
    ContentTooLong,
    #[error("Reader selection cannot be empty.")]
    EmptySelection,
    #[error("Reader selection is too long to save as a record.")]
    SelectionTooLong,
}

pub fn validate_document(
    mut input: NewReaderDocument,
) -> Result<NewReaderDocument, ReaderValidationError> {
    input.title = input.title.trim().to_owned();
    if input.title.is_empty() {
        return Err(ReaderValidationError::EmptyTitle);
    }
    if input.content_markdown.trim().is_empty() {
        return Err(ReaderValidationError::EmptyContent);
    }
    if input.content_markdown.chars().count() > 1_000_000 {
        return Err(ReaderValidationError::ContentTooLong);
    }
    input.subtitle = input.subtitle.and_then(|value| {
        let trimmed = value.trim().to_owned();
        (!trimmed.is_empty()).then_some(trimmed)
    });
    input.source_label = input.source_label.and_then(|value| {
        let trimmed = value.trim().to_owned();
        (!trimmed.is_empty()).then_some(trimmed)
    });
    Ok(input)
}

pub fn validate_selection(text: String) -> Result<String, ReaderValidationError> {
    if text.trim().is_empty() {
        return Err(ReaderValidationError::EmptySelection);
    }
    if text.chars().count() > 100_000 {
        return Err(ReaderValidationError::SelectionTooLong);
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_validation_preserves_exact_text() {
        let source = "  第一行\nSecond `code` line  ".to_string();
        assert_eq!(validate_selection(source.clone()).unwrap(), source);
    }

    #[test]
    fn selection_rejects_only_whitespace() {
        assert_eq!(
            validate_selection(" \n\t ".to_string()).unwrap_err(),
            ReaderValidationError::EmptySelection
        );
    }
}
