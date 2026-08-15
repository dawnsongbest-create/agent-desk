use std::sync::Arc;

use serde::Deserialize;
use tauri::State;
use tauri_plugin_dialog::DialogExt;

use crate::{
    application::sticky_service::{StickyService, StickyServiceError},
    domain::sticky::{StickyCard, StickyCardKind, StickyProfile},
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStickyQuoteInput {
    quote_text: String,
}

#[derive(Debug, Deserialize)]
pub struct ExportStickyRecordInput {
    id: String,
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

#[tauri::command]
pub async fn get_sticky_profile(state: State<'_, StickyState>) -> Result<StickyProfile, String> {
    state.0.get_profile().await.map_err(command_error)
}

#[tauri::command]
pub async fn update_sticky_quote(
    state: State<'_, StickyState>,
    input: UpdateStickyQuoteInput,
) -> Result<StickyProfile, String> {
    state
        .0
        .update_quote(input.quote_text)
        .await
        .map_err(command_error)
}

fn export_file_name(text: &str) -> String {
    let first_line = text
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("记录");
    let safe = first_line
        .chars()
        .filter(|character| {
            !matches!(
                character,
                '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
            )
        })
        .take(48)
        .collect::<String>();
    format!("{}.md", safe.trim().trim_end_matches('.'))
}

fn write_record_export(path: &std::path::Path, text: &str) -> std::io::Result<()> {
    std::fs::write(path, text.as_bytes())
}

#[tauri::command]
pub async fn export_sticky_record(
    app: tauri::AppHandle,
    state: State<'_, StickyState>,
    input: ExportStickyRecordInput,
) -> Result<bool, String> {
    let text = state
        .0
        .record_text(&input.id)
        .await
        .map_err(command_error)?;
    let file = app
        .dialog()
        .file()
        .set_title("导出记录")
        .set_file_name(export_file_name(&text))
        .add_filter("Markdown", &["md"])
        .blocking_save_file();
    let Some(file) = file else {
        return Ok(false);
    };
    let path = file.into_path().map_err(|error| error.to_string())?;
    write_record_export(&path, &text).map_err(|error| error.to_string())?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_a_safe_markdown_file_name_from_the_first_nonempty_line() {
        assert_eq!(
            export_file_name("\n  Agent Desk: Reader / idea?\nbody"),
            "Agent Desk Reader  idea.md"
        );
    }

    #[test]
    fn exported_markdown_reopens_with_exact_saved_content() {
        let temp = tempfile::tempdir().expect("temp directory");
        let path = temp.path().join("记录.md");
        let source = format!(
            "第一行\n{}\n- Markdown-friendly 原文",
            "中文内容".repeat(1_300)
        );
        write_record_export(&path, &source).expect("export should write");
        assert_eq!(std::fs::read_to_string(path).unwrap(), source);
    }
}
