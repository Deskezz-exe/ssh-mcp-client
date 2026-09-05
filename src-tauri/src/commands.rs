use std::path::Path;
use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Manager, State};
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

#[tauri::command]
pub async fn list_remote_directory(
    state: State<'_, Arc<AppState>>,
    server_id: String,
    path: String,
) -> CmdResult<Vec<core::RemoteEntry>> {
    let state = state.inner().clone();
    core::list_remote_directory(&state, &server_id, &path)
        .await
        .map_err(to_str_err)
}

#[tauri::command]
pub async fn upload_to_server(
    state: State<'_, Arc<AppState>>,
    server_id: String,
    local_path: String,
    remote_path: String,
) -> CmdResult<u64> {
    let state = state.inner().clone();
    core::upload_file(&state, &server_id, &local_path, &remote_path)
        .await
        .map_err(to_str_err)
}

#[tauri::command]
pub async fn download_from_server(
    state: State<'_, Arc<AppState>>,
    server_id: String,
    remote_path: String,
    local_path: String,
) -> CmdResult<u64> {
    let state = state.inner().clone();
    core::download_file(&state, &server_id, &remote_path, &local_path)
        .await
        .map_err(to_str_err)
}

#[tauri::command]
pub fn get_home_dir(app: AppHandle) -> CmdResult<String> {
    app.path()
        .home_dir()
        .map(|p| p.to_string_lossy().into_owned())
        .map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalListing {
    pub parent: Option<String>,
    pub entries: Vec<LocalEntry>,
}

#[tauri::command]
pub fn list_local_directory(path: String) -> CmdResult<LocalListing> {
    let dir = Path::new(&path);
    let read_dir = std::fs::read_dir(dir).map_err(|e| e.to_string())?;

    let mut entries = Vec::new();
    for entry in read_dir {
        let entry = entry.map_err(|e| e.to_string())?;
        let metadata = entry.metadata().map_err(|e| e.to_string())?;
        entries.push(LocalEntry {
            name: entry.file_name().to_string_lossy().into_owned(),
            path: entry.path().to_string_lossy().into_owned(),
            is_dir: metadata.is_dir(),
            size: metadata.len(),
        });
    }
    entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.to_lowercase().cmp(&b.name.to_lowercase())));

    let parent = dir.parent().map(|p| p.to_string_lossy().into_owned());
    Ok(LocalListing { parent, entries })
}
