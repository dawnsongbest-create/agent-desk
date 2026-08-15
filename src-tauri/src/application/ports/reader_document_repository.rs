use async_trait::async_trait;
use thiserror::Error;

use crate::domain::reader::{
    NewReaderDocument, ReaderDocument, RecordSourceRef, SelectionCaptureResult,
};

#[derive(Debug, Error)]
pub enum ReaderDocumentRepositoryError {
    #[error("Reader document was not found.")]
    NotFound,
    #[error("Reader document storage failed: {0}")]
    Storage(String),
}

impl From<sqlx::Error> for ReaderDocumentRepositoryError {
    fn from(error: sqlx::Error) -> Self {
        Self::Storage(error.to_string())
    }
}

#[async_trait]
pub trait ReaderDocumentRepository: Send + Sync {
    async fn create(
        &self,
        document: &NewReaderDocument,
    ) -> Result<ReaderDocument, ReaderDocumentRepositoryError>;
    async fn get_by_id(
        &self,
        id: &str,
    ) -> Result<Option<ReaderDocument>, ReaderDocumentRepositoryError>;
    async fn list(&self) -> Result<Vec<ReaderDocument>, ReaderDocumentRepositoryError>;
    async fn capture_selection(
        &self,
        document_id: &str,
        selected_text: &str,
    ) -> Result<SelectionCaptureResult, ReaderDocumentRepositoryError>;
    async fn get_record_source_ref(
        &self,
        record_id: &str,
    ) -> Result<Option<RecordSourceRef>, ReaderDocumentRepositoryError>;
}
