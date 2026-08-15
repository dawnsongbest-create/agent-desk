pub mod application;
pub mod domain;
pub mod persistence;
mod preferences;
mod reader;
mod shell;
mod sticky;

use tauri::{Manager, WindowEvent};
use tauri_plugin_window_state::{AppHandleExt, StateFlags};

const AUDIT_DATA_DIR_ENV: &str = "AGENT_DESK_AUDIT_DATA_DIR";

pub(crate) fn audit_data_dir() -> Option<std::path::PathBuf> {
    std::env::var_os(AUDIT_DATA_DIR_ENV)
        .map(std::path::PathBuf::from)
        .filter(|path| path.is_absolute())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut window_state = tauri_plugin_window_state::Builder::default()
        .with_state_flags(StateFlags::POSITION | StateFlags::SIZE)
        .skip_initial_state("main");
    if let Some(data_dir) = audit_data_dir() {
        window_state = window_state.with_filename(
            data_dir
                .join(".window-state.json")
                .to_string_lossy()
                .into_owned(),
        );
    }

    tauri::Builder::default()
        // Tech Lead invariant: single-instance must be registered before every other plugin.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            shell::surface_main_window(app);
        }))
        .plugin(window_state.build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .setup(|app| {
            let app_data_dir = if let Some(data_dir) = audit_data_dir() {
                data_dir
            } else {
                app.path().app_data_dir()?
            };
            let database_path = app_data_dir.join("agent-desk.sqlite3");
            let database =
                tauri::async_runtime::block_on(persistence::sqlite::connect(&database_path))?;
            let sticky_repository =
                persistence::sticky_repository::SqliteStickyRepository::new(database.0.clone());
            let reader_repository =
                persistence::reader_document_repository::SqliteReaderDocumentRepository::new(
                    database.0.clone(),
                );

            app.manage(database);
            app.manage(sticky::StickyState::new(sticky_repository));
            app.manage(reader::ReaderState::new(reader_repository));
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
            preferences::update_preferences,
            preferences::apply_window_preset,
            sticky::list_sticky_cards,
            sticky::create_sticky_card,
            sticky::update_sticky_text,
            sticky::set_task_completed,
            sticky::set_task_due_date,
            sticky::delete_sticky_card,
            sticky::reorder_sticky_cards,
            sticky::get_sticky_profile,
            sticky::update_sticky_quote,
            sticky::export_sticky_record,
            reader::open_reader_document,
            reader::get_reader_document,
            reader::list_reader_documents,
            reader::create_reader_document,
            reader::capture_reader_selection
        ])
        .run(tauri::generate_context!())
        .expect("error while running Agent Desk");
}
