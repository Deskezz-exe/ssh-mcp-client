use std::sync::Arc;

use tauri::{AppHandle, State};
use uuid::Uuid;

use crate::core;
use crate::error::AppError;
use crate::state::AppState;
use crate::storage::profiles::ServerProfile;
use crate::storage::{keychain, profiles};

type CmdResult<T> = Result<T, String>;

fn to_str_err(e: AppError) -> String {
    e.to_string()
}

#[tauri::command]
pub async fn list_servers(state: State<'_, Arc<AppState>>) -> CmdResult<Vec<core::ServerSummary>> {
    let state = state.inner().clone();
    core::list_servers(&state).map_err(to_str_err)
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn save_profile(
    state: State<'_, Arc<AppState>>,
    name: String,
    host: String,
    port: u16,
    username: String,
    password: String,
) -> CmdResult<ServerProfile> {
    let id = Uuid::new_v4().to_string();
    let profile = ServerProfile {
        id: id.clone(),
        name,
        host,
        port,
        username,
        host_key_fingerprint: None,
        favorite: false,
    };
    profiles::upsert(&state.app_data_dir, profile.clone()).map_err(to_str_err)?;
    keychain::set_password(&id, &password).map_err(to_str_err)?;
    Ok(profile)
}

#[tauri::command]
pub async fn delete_profile(state: State<'_, Arc<AppState>>, server_id: String) -> CmdResult<()> {
    state.sessions.lock().unwrap().remove(&server_id);
    let _ = keychain::delete_password(&server_id);
    profiles::delete(&state.app_data_dir, &server_id).map_err(to_str_err)
}

#[tauri::command]
pub async fn set_favorite(state: State<'_, Arc<AppState>>, server_id: String, favorite: bool) -> CmdResult<()> {
    profiles::set_favorite(&state.app_data_dir, &server_id, favorite).map_err(to_str_err)
}

#[tauri::command]
pub async fn open_terminal(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    server_id: String,
    cols: u32,
    rows: u32,
) -> CmdResult<()> {
    let state = state.inner().clone();
    let session = core::ensure_connected(&state, &server_id).await.map_err(to_str_err)?;
    session.open_pty(app, cols, rows).await.map_err(to_str_err)
}

#[tauri::command]
pub async fn write_pty(state: State<'_, Arc<AppState>>, server_id: String, data: String) -> CmdResult<()> {
    let sessions = state.sessions.lock().unwrap();
    let session = sessions
        .get(&server_id)
        .ok_or_else(|| "no active session for this server".to_string())?;
    session.write_pty(data.into_bytes()).map_err(to_str_err)
}

#[tauri::command]
pub async fn resize_pty(state: State<'_, Arc<AppState>>, server_id: String, cols: u32, rows: u32) -> CmdResult<()> {
    let sessions = state.sessions.lock().unwrap();
    let session = sessions
        .get(&server_id)
        .ok_or_else(|| "no active session for this server".to_string())?;
    session.resize_pty(cols, rows).map_err(to_str_err)
}

#[tauri::command]
pub async fn disconnect_server(state: State<'_, Arc<AppState>>, server_id: String) -> CmdResult<()> {
    state.sessions.lock().unwrap().remove(&server_id);
    Ok(())
}

#[tauri::command]
pub fn mcp_server_info(state: State<'_, Arc<AppState>>) -> CmdResult<u16> {
    Ok(state.mcp_port)
}
