use std::sync::Arc;

use serde::Deserialize;
use tauri::State;

use crate::{
    application::reader_service::{ReaderService, ReaderServiceError},
    domain::reader::{
        ReaderDocument, ReaderDocumentType, ReaderSourceType, SelectionCaptureResult,
    },
    persistence::reader_document_repository::SqliteReaderDocumentRepository,
};

pub struct ReaderState(ReaderService);

impl ReaderState {
    pub fn new(repository: SqliteReaderDocumentRepository) -> Self {
        Self(ReaderService::new(Arc::new(repository)))
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenReaderDocumentInput {
    current_document_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GetReaderDocumentInput {
    id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateReaderDocumentInput {
    document_type: ReaderDocumentType,
    title: String,
    subtitle: Option<String>,
    content_markdown: String,
    source_type: ReaderSourceType,
    source_label: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureReaderSelectionInput {
    document_id: String,
    selected_text: String,
}

fn command_error(error: ReaderServiceError) -> String {
    error.to_string()
}

#[tauri::command]
pub async fn open_reader_document(
    state: State<'_, ReaderState>,
    input: OpenReaderDocumentInput,
) -> Result<ReaderDocument, String> {
    state
        .0
        .open_current(input.current_document_id.as_deref())
        .await
        .map_err(command_error)
}

#[tauri::command]
pub async fn get_reader_document(
    state: State<'_, ReaderState>,
    input: GetReaderDocumentInput,
) -> Result<ReaderDocument, String> {
    state.0.get(&input.id).await.map_err(command_error)
}

#[tauri::command]
pub async fn list_reader_documents(
    state: State<'_, ReaderState>,
) -> Result<Vec<ReaderDocument>, String> {
    state.0.list().await.map_err(command_error)
}

#[tauri::command]
pub async fn create_reader_document(
    state: State<'_, ReaderState>,
    input: CreateReaderDocumentInput,
) -> Result<ReaderDocument, String> {
    state
        .0
        .create(
            input.document_type,
            input.title,
            input.subtitle,
            input.content_markdown,
            input.source_type,
            input.source_label,
        )
        .await
        .map_err(command_error)
}

#[tauri::command]
pub async fn capture_reader_selection(
    state: State<'_, ReaderState>,
    input: CaptureReaderSelectionInput,
) -> Result<SelectionCaptureResult, String> {
    state
        .0
        .capture_selection(&input.document_id, input.selected_text)
        .await
        .map_err(command_error)
}
