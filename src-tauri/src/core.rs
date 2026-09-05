//! Business logic shared between the Tauri GUI commands and the MCP tools,
//! so both surfaces go through the exact same SSH sessions and audit log.

use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::Serialize;
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
