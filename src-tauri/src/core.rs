//! Business logic shared between the Tauri GUI commands and the MCP tools,
//! so both surfaces go through the exact same SSH sessions and audit log.

use std::sync::Arc;
use std::time::{Duration, Instant};

use russh_sftp::protocol::OpenFlags;
use serde::Serialize;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use crate::audit::{self, dangerous};
use crate::error::AppError;
use crate::ssh::SshSession;
use crate::state::{AppState, PendingCommand};
use crate::storage::{keychain, profiles};

const PENDING_TTL: Duration = Duration::from_secs(300);

#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
pub struct ServerSummary {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub connected: bool,
    pub favorite: bool,
}

#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
pub struct CommandOutcome {
    pub executed: bool,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub exit_code: Option<i32>,
    pub requires_confirmation: bool,
    pub confirmation_token: Option<String>,
    pub reason: Option<String>,
}

pub fn list_servers(state: &AppState) -> Result<Vec<ServerSummary>, AppError> {
    let profiles = profiles::load(&state.app_data_dir)?;
    let sessions = state.sessions.lock().unwrap();
    let mut summaries: Vec<ServerSummary> = profiles
        .into_iter()
        .map(|p| ServerSummary {
            connected: sessions.contains_key(&p.id),
            favorite: p.favorite,
            id: p.id,
            name: p.name,
            host: p.host,
            port: p.port,
            username: p.username,
        })
        .collect();
    // Favorites first, otherwise keep the saved order.
    summaries.sort_by_key(|s| !s.favorite);
    Ok(summaries)
}

/// Returns the existing SSH session for `server_id` if one is open
/// (whether it was opened from the GUI or from a previous MCP call), or
/// opens a fresh one using the saved profile + keychain password.
pub async fn ensure_connected(state: &AppState, server_id: &str) -> Result<Arc<SshSession>, AppError> {
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

pub async fn run_command(state: &AppState, server_id: &str, command: &str) -> Result<CommandOutcome, AppError> {
    let session = ensure_connected(state, server_id).await?;

    if let Some(reason) = dangerous::is_dangerous(command) {
        let token = Uuid::new_v4().to_string();
        state.pending.lock().unwrap().insert(
            token.clone(),
            PendingCommand {
                server_id: server_id.to_string(),
                command: command.to_string(),
                created_at: Instant::now(),
            },
        );
        {
            let db = state.db.lock().unwrap();
            audit::log_command(&db, server_id, command, "mcp", true, false, None, "")?;
        }
        return Ok(CommandOutcome {
            executed: false,
            stdout: None,
            stderr: None,
            exit_code: None,
            requires_confirmation: true,
            confirmation_token: Some(token),
            reason: Some(format!(
                "Blocked: {reason}. Show this to the user in chat; if they agree, call confirm_dangerous_command with this token."
            )),
        });
    }

    let result = session.exec(command).await?;
    {
        let db = state.db.lock().unwrap();
        audit::log_command(&db, server_id, command, "mcp", false, false, result.exit_code, &result.stdout)?;
    }
    Ok(CommandOutcome {
        executed: true,
        stdout: Some(result.stdout),
        stderr: Some(result.stderr),
        exit_code: result.exit_code,
        requires_confirmation: false,
        confirmation_token: None,
        reason: None,
    })
}

pub async fn confirm_dangerous_command(state: &AppState, token: &str) -> Result<CommandOutcome, AppError> {
    let pending = {
        let mut pending_map = state.pending.lock().unwrap();
        pending_map.remove(token).ok_or(AppError::UnknownToken)?
    };
    if pending.created_at.elapsed() > PENDING_TTL {
        return Err(AppError::Other(
            "confirmation token expired — call run_command again to re-request it".into(),
        ));
    }

    let session = ensure_connected(state, &pending.server_id).await?;
    let result = session.exec(&pending.command).await?;
    {
        let db = state.db.lock().unwrap();
        audit::log_command(
            &db,
            &pending.server_id,
            &pending.command,
            "mcp",
            true,
            true,
            result.exit_code,
            &result.stdout,
        )?;
    }
    Ok(CommandOutcome {
        executed: true,
        stdout: Some(result.stdout),
        stderr: Some(result.stderr),
        exit_code: result.exit_code,
        requires_confirmation: false,
        confirmation_token: None,
        reason: None,
    })
}

#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
pub struct RemoteEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    /// Unix timestamp (seconds), if the server reported one.
    pub modified: Option<u64>,
}

#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
pub struct RemoteListing {
    /// The resolved absolute path that was actually listed — e.g. `path`
    /// of "." resolves to something like "/root", so the UI can show
    /// where it really is instead of the literal request string.
    pub current: String,
    pub entries: Vec<RemoteEntry>,
}

fn unix_secs(t: std::io::Result<std::time::SystemTime>) -> Option<u64> {
    t.ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
}

/// Lists a remote directory over a fresh SFTP subsystem channel (same SSH
/// connection, no extra TCP/SSH handshake). Directories sort first.
pub async fn list_remote_directory(state: &AppState, server_id: &str, path: &str) -> Result<RemoteListing, AppError> {
    let session = ensure_connected(state, server_id).await?;
    let sftp = session.open_sftp().await?;
    let current = sftp.canonicalize(path).await?;
    let dir = sftp.read_dir(&current).await?;

    let mut entries: Vec<RemoteEntry> = dir
        .map(|entry| {
            let metadata = entry.metadata();
            RemoteEntry {
                name: entry.file_name(),
                path: entry.path(),
                is_dir: entry.file_type().is_dir(),
                size: metadata.len(),
                modified: unix_secs(metadata.modified()),
            }
        })
        .collect();
    entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    Ok(RemoteListing { current, entries })
}

/// Deletes a single file on the remote server. Never used for directories —
/// this is a deliberately narrow, GUI-only, human-confirmed action.
pub async fn delete_remote_file(state: &AppState, server_id: &str, path: &str) -> Result<(), AppError> {
    let session = ensure_connected(state, server_id).await?;
    let sftp = session.open_sftp().await?;
    sftp.remove_file(path).await?;
    Ok(())
}

/// Checks whether a path already exists on the remote server, so the GUI
/// can ask before an upload/download silently overwrites something.
pub async fn remote_path_exists(state: &AppState, server_id: &str, path: &str) -> Result<bool, AppError> {
    let session = ensure_connected(state, server_id).await?;
    let sftp = session.open_sftp().await?;
    Ok(sftp.try_exists(path).await?)
}

/// Uploads a file from this machine (where the app runs) to the remote
/// server over SFTP. Returns the number of bytes sent.
pub async fn upload_file(state: &AppState, server_id: &str, local_path: &str, remote_path: &str) -> Result<u64, AppError> {
    let session = ensure_connected(state, server_id).await?;
    let data = tokio::fs::read(local_path).await?;
    let len = data.len() as u64;

    let sftp = session.open_sftp().await?;
    let mut file = sftp
        .open_with_flags(remote_path, OpenFlags::CREATE | OpenFlags::TRUNCATE | OpenFlags::WRITE)
        .await?;
    file.write_all(&data).await?;
    file.shutdown().await?;
    Ok(len)
}

/// Downloads a file from the remote server to this machine over SFTP.
/// Returns the number of bytes received.
pub async fn download_file(state: &AppState, server_id: &str, remote_path: &str, local_path: &str) -> Result<u64, AppError> {
    let session = ensure_connected(state, server_id).await?;
    let sftp = session.open_sftp().await?;
    let data = sftp.read(remote_path).await?;
    let len = data.len() as u64;

    if let Some(parent) = std::path::Path::new(local_path).parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(local_path, &data).await?;
    Ok(len)
}
