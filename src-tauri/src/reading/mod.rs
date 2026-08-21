use std::sync::Arc;

use serde::Deserialize;
use tauri::State;

use crate::{
    application::reading_service::{ReadingService, ReadingServiceError},
    domain::reading::{
        CreateReadingPlanInput, GenerateReadingDeliveryResult, ReadingPlan, ReadingPlanStatus,
        ReadingSessionResult,
    },
    persistence::{
        delivery_repository::SqliteDeliveryRepository, reading_repository::SqliteReadingRepository,
    },
};

pub struct ReadingState(ReadingService);

impl ReadingState {
    pub fn new(
        reading_repository: SqliteReadingRepository,
        delivery_repository: SqliteDeliveryRepository,
    ) -> Self {
        Self(ReadingService::new(
            Arc::new(reading_repository),
            Arc::new(delivery_repository),
        ))
    }
}

#[derive(Debug, Deserialize)]
pub struct ReadingPlanIdInput {
    id: String,
}

#[derive(Debug, Deserialize)]
pub struct SetReadingPlanStatusInput {
    id: String,
    status: ReadingPlanStatus,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateReadingSessionInput {
    source_document_id: String,
    content: String,
}

fn command_error(error: ReadingServiceError) -> String {
    error.to_string()
}

#[tauri::command]
pub async fn create_reading_plan(
    state: State<'_, ReadingState>,
    input: CreateReadingPlanInput,
) -> Result<ReadingPlan, String> {
    state.0.create_plan(input).await.map_err(command_error)
}

#[tauri::command]
pub async fn list_reading_plans(
    state: State<'_, ReadingState>,
) -> Result<Vec<ReadingPlan>, String> {
    state.0.list_plans().await.map_err(command_error)
}

#[tauri::command]
pub async fn set_reading_plan_status(
    state: State<'_, ReadingState>,
    input: SetReadingPlanStatusInput,
) -> Result<ReadingPlan, String> {
    state
        .0
        .set_plan_status(&input.id, input.status)
        .await
        .map_err(command_error)
}

#[tauri::command]
pub async fn generate_reading_delivery(
    state: State<'_, ReadingState>,
    input: ReadingPlanIdInput,
) -> Result<GenerateReadingDeliveryResult, String> {
    state
        .0
        .generate_today(&input.id)
        .await
        .map_err(command_error)
}

#[tauri::command]
pub async fn create_reading_session(
    state: State<'_, ReadingState>,
    input: CreateReadingSessionInput,
) -> Result<ReadingSessionResult, String> {
    state
        .0
        .create_session(&input.source_document_id, input.content)
        .await
        .map_err(command_error)
}
