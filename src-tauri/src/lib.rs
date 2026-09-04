mod commands;
mod error;
mod ssh;
mod state;
mod storage;

use std::sync::{Arc, Mutex};

use tauri::Manager;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir().expect("resolve app data dir");
            std::fs::create_dir_all(&app_data_dir).expect("create app data dir");

            let state = Arc::new(AppState {
                sessions: Mutex::new(Default::default()),
                app_data_dir,
            });
            app.manage(state);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_servers,
            commands::save_profile,
            commands::delete_profile,
            commands::open_terminal,
            commands::write_pty,
            commands::resize_pty,
            commands::disconnect_server,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
