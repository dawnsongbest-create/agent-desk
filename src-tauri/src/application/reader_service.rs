use std::sync::Arc;

use thiserror::Error;

use crate::{
    application::ports::reader_document_repository::{
        ReaderDocumentRepository, ReaderDocumentRepositoryError,
    },
    domain::reader::{
        validate_document, validate_selection, NewReaderDocument, ReaderDocument,
        ReaderDocumentType, ReaderSourceType, ReaderValidationError, SelectionCaptureResult,
    },
};

pub const BUILTIN_READER_DOCUMENT_ID: &str = "reader_builtin_foundation_v1";
const BUILTIN_CONTENT: &str = include_str!("../../fixtures/reader_foundation.md");

#[derive(Debug, Error)]
pub enum ReaderServiceError {
    #[error(transparent)]
    Validation(#[from] ReaderValidationError),
    #[error(transparent)]
    Repository(#[from] ReaderDocumentRepositoryError),
}

#[derive(Clone)]
pub struct ReaderService {
    repository: Arc<dyn ReaderDocumentRepository>,
}

impl ReaderService {
    pub fn new(repository: Arc<dyn ReaderDocumentRepository>) -> Self {
        Self { repository }
    }

    pub async fn open_current(
        &self,
        current_document_id: Option<&str>,
    ) -> Result<ReaderDocument, ReaderServiceError> {
        if let Some(id) = current_document_id {
            if let Some(document) = self.repository.get_by_id(id).await? {
                return Ok(document);
            }
        }

        if let Some(document) = self.repository.list().await?.into_iter().next() {
            return Ok(document);
        }

        self.repository
            .create(&validate_document(NewReaderDocument {
                id: Some(BUILTIN_READER_DOCUMENT_ID.to_owned()),
                document_type: ReaderDocumentType::Article,
                title: "给长文章留一张安静的纸".to_owned(),
                subtitle: Some("在连续阅读与随手记录之间，保留一点呼吸。".to_owned()),
                content_markdown: BUILTIN_CONTENT.to_owned(),
                source_type: ReaderSourceType::Builtin,
                source_label: Some("Agent Desk 内置阅读示例".to_owned()),
            })?)
            .await
            .map_err(Into::into)
    }

    pub async fn get(&self, id: &str) -> Result<ReaderDocument, ReaderServiceError> {
        self.repository
            .get_by_id(id)
            .await?
            .ok_or(ReaderDocumentRepositoryError::NotFound.into())
    }

    pub async fn list(&self) -> Result<Vec<ReaderDocument>, ReaderServiceError> {
        Ok(self.repository.list().await?)
    }

    pub async fn create(
        &self,
        document_type: ReaderDocumentType,
        title: String,
        subtitle: Option<String>,
        content_markdown: String,
        source_type: ReaderSourceType,
        source_label: Option<String>,
    ) -> Result<ReaderDocument, ReaderServiceError> {
        let document = validate_document(NewReaderDocument {
            id: None,
            document_type,
            title,
            subtitle,
            content_markdown,
            source_type,
            source_label,
        })?;
        Ok(self.repository.create(&document).await?)
    }

    pub async fn capture_selection(
        &self,
        document_id: &str,
        selected_text: String,
    ) -> Result<SelectionCaptureResult, ReaderServiceError> {
        let selected_text = validate_selection(selected_text)?;
        Ok(self
            .repository
            .capture_selection(document_id, &selected_text)
            .await?)
    }
}
