pub mod application;
pub mod domain;
pub mod persistence;
mod preferences;
mod shell;

use tauri::{Manager, WindowEvent};
use tauri_plugin_window_state::{AppHandleExt, StateFlags};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Tech Lead invariant: single-instance must be registered before every other plugin.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            shell::surface_main_window(app);
        }))
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(StateFlags::POSITION | StateFlags::SIZE)
                .skip_initial_state("main")
                .build(),
        )
        .plugin(tauri_plugin_store::Builder::default().build())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;
            let database_path = app_data_dir.join("agent-desk.sqlite3");
            let database =
                tauri::async_runtime::block_on(persistence::sqlite::connect(&database_path))?;

            app.manage(database);
            preferences::initialize(app.handle()).map_err(std::io::Error::other)?;
            shell::setup_tray(app)?;
            shell::restore_main_window(app.handle())?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main" {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window
                        .app_handle()
                        .save_window_state(StateFlags::POSITION | StateFlags::SIZE);
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            preferences::get_preferences,
            preferences::update_preferences
        ])
        .run(tauri::generate_context!())
        .expect("error while running Agent Desk");
}
