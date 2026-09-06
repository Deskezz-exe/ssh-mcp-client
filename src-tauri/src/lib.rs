mod audit;
mod commands;
mod core;
mod error;
mod mcp;
mod ssh;
mod state;
mod storage;

use std::sync::{Arc, Mutex};

use tauri::Manager;

use state::AppState;

const DEFAULT_MCP_PORT: u16 = 47821;

/// A `data` folder next to the running exe, not the OS's per-user app data
/// directory — this app ships as a single portable exe (no installer), so
/// its profiles/settings/audit log travel with it (e.g. on a USB stick)
/// instead of landing in %APPDATA% on the system drive.
fn portable_data_dir() -> std::path::PathBuf {
    let exe = std::env::current_exe().expect("resolve current exe path");
    exe.parent()
        .expect("exe path has a parent directory")
        .join("data")
}

fn load_mcp_port(app_data_dir: &std::path::Path) -> u16 {
    let path = app_data_dir.join("settings.json");
    let Ok(data) = std::fs::read_to_string(path) else {
        return DEFAULT_MCP_PORT;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&data) else {
        return DEFAULT_MCP_PORT;
    };
    value
        .get("mcp_port")
        .and_then(|v| v.as_u64())
        .map(|v| v as u16)
        .unwrap_or(DEFAULT_MCP_PORT)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt::init();

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_data_dir = portable_data_dir();
            std::fs::create_dir_all(&app_data_dir).expect("create app data dir");

            let db = audit::open(&app_data_dir).expect("open audit database");
            let mcp_port = load_mcp_port(&app_data_dir);

            let state = Arc::new(AppState {
                sessions: Mutex::new(Default::default()),
                pending: Mutex::new(Default::default()),
                db: Mutex::new(db),
                app_data_dir,
                mcp_port,
                mcp_handle: Mutex::new(None),
            });

            let mcp_handle = tauri::async_runtime::block_on(mcp::start(state.clone(), mcp_port))
                .expect("failed to start embedded MCP server");
            tracing::info!("MCP server listening on http://127.0.0.1:{}/mcp", mcp_handle.port);
            *state.mcp_handle.lock().unwrap() = Some(mcp_handle);

            app.manage(state);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_servers,
            commands::save_profile,
            commands::delete_profile,
            commands::set_favorite,
            commands::open_terminal,
            commands::write_pty,
            commands::resize_pty,
            commands::disconnect_server,
            commands::mcp_server_info,
            commands::list_remote_directory,
            commands::delete_remote_file,
            commands::remote_file_exists,
            commands::local_file_exists,
            commands::upload_to_server,
            commands::download_from_server,
            commands::get_home_dir,
            commands::list_local_directory,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| {
        if let tauri::RunEvent::ExitRequested { .. } = event {
            if let Some(state) = app_handle.try_state::<Arc<AppState>>() {
                if let Some(handle) = state.mcp_handle.lock().unwrap().take() {
                    handle.shutdown();
                }
            }
        }
    });
}
