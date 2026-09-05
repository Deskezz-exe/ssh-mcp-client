use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::mcp::McpServerHandle;
use crate::ssh::SshSession;

pub struct PendingCommand {
    pub server_id: String,
    pub command: String,
    pub created_at: Instant,
}

pub struct AppState {
    pub sessions: Mutex<HashMap<String, Arc<SshSession>>>,
    pub pending: Mutex<HashMap<String, PendingCommand>>,
    pub db: Mutex<rusqlite::Connection>,
    pub app_data_dir: PathBuf,
    pub mcp_port: u16,
    pub mcp_handle: Mutex<Option<McpServerHandle>>,
}
