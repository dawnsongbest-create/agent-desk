use std::sync::Arc;

use serde::Deserialize;
use tauri::State;

use crate::{
    application::sticky_service::{StickyService, StickyServiceError},
    domain::sticky::{StickyCard, StickyCardKind},
    persistence::sticky_repository::SqliteStickyRepository,
};

pub struct StickyState(StickyService);

impl StickyState {
    pub fn new(repository: SqliteStickyRepository) -> Self {
        Self(StickyService::new(Arc::new(repository)))
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateStickyCardInput {
    kind: StickyCardKind,
    text: String,
    due_date: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStickyTextInput {
    id: String,
    text: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetTaskCompletedInput {
    id: String,
    completed: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetTaskDueDateInput {
    id: String,
    due_date: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DeleteStickyCardInput {
    id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReorderStickyCardsInput {
    ordered_ids: Vec<String>,
}

fn command_error(error: StickyServiceError) -> String {
    error.to_string()
}

#[tauri::command]
pub async fn list_sticky_cards(state: State<'_, StickyState>) -> Result<Vec<StickyCard>, String> {
    state.0.list().await.map_err(command_error)
}

#[tauri::command]
pub async fn create_sticky_card(
    state: State<'_, StickyState>,
    input: CreateStickyCardInput,
) -> Result<StickyCard, String> {
    state
        .0
        .create(input.kind, input.text, input.due_date)
        .await
        .map_err(command_error)
}

#[tauri::command]
pub async fn update_sticky_text(
    state: State<'_, StickyState>,
    input: UpdateStickyTextInput,
) -> Result<StickyCard, String> {
    state
        .0
        .update_text(&input.id, input.text)
        .await
        .map_err(command_error)
}

#[tauri::command]
pub async fn set_task_completed(
    state: State<'_, StickyState>,
    input: SetTaskCompletedInput,
) -> Result<StickyCard, String> {
    state
        .0
        .set_task_completed(&input.id, input.completed)
        .await
        .map_err(command_error)
}

#[tauri::command]
pub async fn set_task_due_date(
    state: State<'_, StickyState>,
    input: SetTaskDueDateInput,
) -> Result<StickyCard, String> {
    state
        .0
        .set_task_due_date(&input.id, input.due_date)
        .await
        .map_err(command_error)
}

#[tauri::command]
pub async fn delete_sticky_card(
    state: State<'_, StickyState>,
    input: DeleteStickyCardInput,
) -> Result<(), String> {
    state.0.delete(&input.id).await.map_err(command_error)
}

#[tauri::command]
pub async fn reorder_sticky_cards(
    state: State<'_, StickyState>,
    input: ReorderStickyCardsInput,
) -> Result<Vec<StickyCard>, String> {
    state
        .0
        .reorder(input.ordered_ids)
        .await
        .map_err(command_error)
}
