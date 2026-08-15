use std::sync::Arc;

use serde::Deserialize;
use tauri::State;

use crate::{
    application::delivery_service::{DeliveryService, DeliveryServiceError},
    domain::delivery::{CreateDeliveryInput, InboxDelivery, IngestDeliveryResult},
    persistence::delivery_repository::SqliteDeliveryRepository,
};

pub struct DeliveryState(DeliveryService);

impl DeliveryState {
    pub fn new(repository: SqliteDeliveryRepository) -> Self {
        Self(DeliveryService::new(Arc::new(repository)))
    }
}

#[derive(Debug, Deserialize)]
pub struct OpenDeliveryInput {
    id: String,
}

fn command_error(error: DeliveryServiceError) -> String {
    error.to_string()
}

#[tauri::command]
pub async fn ingest_delivery(
    state: State<'_, DeliveryState>,
    input: CreateDeliveryInput,
) -> Result<IngestDeliveryResult, String> {
    state.0.ingest(input).await.map_err(command_error)
}

#[tauri::command]
pub async fn list_inbox(state: State<'_, DeliveryState>) -> Result<Vec<InboxDelivery>, String> {
    state.0.list_inbox().await.map_err(command_error)
}

#[tauri::command]
pub async fn get_inbox_unread_count(state: State<'_, DeliveryState>) -> Result<i64, String> {
    state.0.get_unread_count().await.map_err(command_error)
}

#[tauri::command]
pub async fn open_delivery(
    state: State<'_, DeliveryState>,
    input: OpenDeliveryInput,
) -> Result<InboxDelivery, String> {
    state.0.open(&input.id).await.map_err(command_error)
}
