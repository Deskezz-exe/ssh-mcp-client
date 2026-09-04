use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, State};
use uuid::Uuid;

use crate::error::AppError;
use crate::ssh::SshSession;
use crate::state::AppState;
use crate::storage::profiles::ServerProfile;
use crate::storage::{keychain, profiles};

type CmdResult<T> = Result<T, String>;

fn to_str_err(e: AppError) -> String {
    e.to_string()
}

#[derive(Debug, Clone, Serialize)]
pub struct ServerSummary {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub connected: bool,
}

/// Returns the existing SSH session for `server_id` if one is already open,
/// or connects a fresh one using the saved profile + keychain password.
async fn ensure_connected(state: &AppState, server_id: &str) -> Result<Arc<SshSession>, AppError> {
    {
        let sessions = state.sessions.lock().unwrap();
        if let Some(session) = sessions.get(server_id) {
            return Ok(session.clone());
        }
    }

    let profile = profiles::find(&state.app_data_dir, server_id)?
        .ok_or_else(|| AppError::ServerNotFound(server_id.to_string()))?;
    let password = keychain::get_password(server_id)?;

    let (mut session, observed_fingerprint) = SshSession::connect(
        &profile.host,
        profile.port,
        &profile.username,
        &password,
        profile.host_key_fingerprint.clone(),
    )
    .await?;
    session.server_id = server_id.to_string();

    if profile.host_key_fingerprint.is_none() {
        profiles::set_host_key_fingerprint(&state.app_data_dir, server_id, &observed_fingerprint)?;
    }

    let session = Arc::new(session);
    state
        .sessions
        .lock()
        .unwrap()
        .insert(server_id.to_string(), session.clone());
    Ok(session)
}

#[tauri::command]
pub async fn list_servers(state: State<'_, Arc<AppState>>) -> CmdResult<Vec<ServerSummary>> {
    let profiles = profiles::load(&state.app_data_dir).map_err(to_str_err)?;
    let sessions = state.sessions.lock().unwrap();
    Ok(profiles
        .into_iter()
        .map(|p| ServerSummary {
            connected: sessions.contains_key(&p.id),
            id: p.id,
            name: p.name,
            host: p.host,
            port: p.port,
            username: p.username,
        })
        .collect())
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
pub async fn open_terminal(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    server_id: String,
    cols: u32,
    rows: u32,
) -> CmdResult<()> {
    let state = state.inner().clone();
    let session = ensure_connected(&state, &server_id).await.map_err(to_str_err)?;
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
