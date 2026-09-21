mod commands;
mod db;
mod load_test;

use commands::AppState;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_data_dir)?;
            let db_path = app_data_dir.join("poster.db");
            let db = db::Db::new(db_path.to_str().unwrap())
                .map_err(|e| format!("failed to open database: {e}"))?;
            app.manage(AppState {
                db,
                // Arc-wrapped so async commands can hand the same map to spawned tasks.
                load_tests: Arc::new(Mutex::new(HashMap::new())),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_projects,
            commands::create_project,
            commands::rename_project,
            commands::delete_project,
            commands::list_requests,
            commands::create_request,
            commands::save_request,
            commands::delete_request,
            commands::list_variables,
            commands::upsert_variable,
            commands::delete_variable,
            commands::list_extracts,
            commands::create_extract,
            commands::delete_extract,
            commands::send_request,
            commands::export_curl,
            commands::parse_curl,
            commands::export_project,
            commands::import_project,
            commands::start_load_test,
            commands::stop_load_test,
            commands::cleanup_load_test,
            commands::load_test_status,
            commands::save_file,
            commands::read_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
