use tauri::{AppHandle, Manager};
use tauri_plugin_store::StoreExt;

use crate::domain::preferences::Preferences;

const PREFERENCES_FILE: &str = "preferences.json";
const PREFERENCES_KEY: &str = "preferences";

fn read(app: &AppHandle) -> Result<Preferences, String> {
    let store = app
        .store(PREFERENCES_FILE)
        .map_err(|error| error.to_string())?;
    let Some(value) = store.get(PREFERENCES_KEY) else {
        return Ok(Preferences::default());
    };

    Ok(serde_json::from_value(value).unwrap_or_default())
}

fn write(app: &AppHandle, preferences: &Preferences) -> Result<(), String> {
    let store = app
        .store(PREFERENCES_FILE)
        .map_err(|error| error.to_string())?;
    let value = serde_json::to_value(preferences).map_err(|error| error.to_string())?;
    store.set(PREFERENCES_KEY.to_string(), value);
    store.save().map_err(|error| error.to_string())
}

pub fn initialize(app: &AppHandle) -> Result<Preferences, String> {
    let preferences = read(app)?;
    write(app, &preferences)?;
    apply_window_preferences(app, &preferences)?;
    Ok(preferences)
}

fn apply_window_preferences(app: &AppHandle, preferences: &Preferences) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "main window is unavailable".to_string())?;
    window
        .set_always_on_top(preferences.always_on_top)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_preferences(app: AppHandle) -> Result<Preferences, String> {
    read(&app)
}

#[tauri::command]
pub fn update_preferences(app: AppHandle, preferences: Preferences) -> Result<Preferences, String> {
    if preferences.schema_version != 1 {
        return Err("unsupported preferences schema version".to_string());
    }

    apply_window_preferences(&app, &preferences)?;
    write(&app, &preferences)?;
    Ok(preferences)
}
