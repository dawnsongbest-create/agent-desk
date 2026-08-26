use tauri::{AppHandle, Emitter, LogicalSize, Manager, Size};
use tauri_plugin_store::StoreExt;

use crate::domain::preferences::{
    DesktopStickyKind, DesktopStickyPosition, EdgeDockPosition, PlacementMode, Preferences,
    StickyMode, SurfaceMode, WindowPreset,
};
use std::path::PathBuf;

const PREFERENCES_FILE: &str = "preferences.json";
const PREFERENCES_KEY: &str = "preferences";

fn preferences_file() -> PathBuf {
    crate::audit_data_dir()
        .map(|data_dir| data_dir.join(PREFERENCES_FILE))
        .unwrap_or_else(|| PathBuf::from(PREFERENCES_FILE))
}

pub(crate) fn read(app: &AppHandle) -> Result<Preferences, String> {
    let store = app
        .store(preferences_file())
        .map_err(|error| error.to_string())?;
    let Some(value) = store.get(PREFERENCES_KEY) else {
        return Ok(Preferences::default());
    };

    Ok(serde_json::from_value::<Preferences>(value)
        .unwrap_or_default()
        .normalize_legacy())
}

pub(crate) fn write(app: &AppHandle, preferences: &Preferences) -> Result<(), String> {
    let store = app
        .store(preferences_file())
        .map_err(|error| error.to_string())?;
    let value = serde_json::to_value(preferences).map_err(|error| error.to_string())?;
    store.set(PREFERENCES_KEY.to_string(), value);
    store.save().map_err(|error| error.to_string())
}

fn publish(app: &AppHandle, preferences: &Preferences) {
    let _ = app.emit("agent-desk://preferences-changed", preferences);
}

pub(crate) fn set_surface_and_placement(
    app: &AppHandle,
    surface_mode: SurfaceMode,
    placement_mode: PlacementMode,
) -> Result<Preferences, String> {
    let mut preferences = read(app)?;
    preferences.surface_mode = surface_mode;
    preferences.placement_mode = placement_mode;
    write(app, &preferences)?;
    publish(app, &preferences);
    Ok(preferences)
}

pub(crate) fn set_display_state(
    app: &AppHandle,
    surface_mode: SurfaceMode,
    sticky_mode: StickyMode,
    placement_mode: PlacementMode,
) -> Result<Preferences, String> {
    let mut preferences = read(app)?;
    preferences.surface_mode = surface_mode;
    preferences.sticky_mode = sticky_mode;
    preferences.placement_mode = placement_mode;
    write(app, &preferences)?;
    publish(app, &preferences);
    Ok(preferences)
}

pub(crate) fn set_desktop_sticky_kind(
    app: &AppHandle,
    desktop_sticky_kind: DesktopStickyKind,
) -> Result<Preferences, String> {
    let mut preferences = read(app)?;
    preferences.desktop_sticky_kind = desktop_sticky_kind;
    write(app, &preferences)?;
    publish(app, &preferences);
    Ok(preferences)
}

pub(crate) fn save_desktop_sticky_position(
    app: &AppHandle,
    position: DesktopStickyPosition,
) -> Result<(), String> {
    let mut preferences = read(app)?;
    if preferences.surface_mode != SurfaceMode::Desktop
        || preferences.placement_mode != PlacementMode::Free
    {
        return Ok(());
    }
    preferences.desktop_sticky_position = Some(position);
    write(app, &preferences)
}

pub(crate) fn save_edge_dock_position(
    app: &AppHandle,
    position: EdgeDockPosition,
) -> Result<(), String> {
    let mut preferences = read(app)?;
    if preferences.placement_mode == PlacementMode::Free {
        return Ok(());
    }
    preferences.edge_dock_position = Some(position);
    write(app, &preferences)
}

pub fn initialize(app: &AppHandle) -> Result<Preferences, String> {
    let mut preferences = read(app)?.apply_startup_policy();
    preferences.always_on_top = true;
    write(app, &preferences)?;
    apply_window_preferences(app, &preferences)?;
    Ok(preferences)
}

fn apply_window_preferences(app: &AppHandle, _preferences: &Preferences) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "main window is unavailable".to_string())?;
    window
        .set_always_on_top(true)
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_preferences(app: AppHandle) -> Result<Preferences, String> {
    read(&app)
}

#[tauri::command]
pub fn update_preferences(
    app: AppHandle,
    mut preferences: Preferences,
) -> Result<Preferences, String> {
    if preferences.schema_version != 1 {
        return Err("unsupported preferences schema version".to_string());
    }

    preferences.always_on_top = true;
    apply_window_preferences(&app, &preferences)?;
    write(&app, &preferences)?;
    publish(&app, &preferences);
    Ok(preferences)
}

#[tauri::command]
pub fn apply_window_preset(app: AppHandle, preset: WindowPreset) -> Result<(), String> {
    let (width, height) = preset
        .logical_size()
        .ok_or_else(|| "Custom is not an applicable window preset".to_string())?;
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "main window is unavailable".to_string())?;
    window
        .set_size(Size::Logical(LogicalSize::new(width, height)))
        .map_err(|error| error.to_string())?;
    crate::shell::clamp_window_to_monitor(&window).map_err(|error| error.to_string())
}
