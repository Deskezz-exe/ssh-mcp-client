use std::sync::Arc;

use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, Json, ServerHandler,
};
use serde::Deserialize;

use crate::core;
use crate::state::AppState;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ServerIdParams {
    pub server_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RunCommandParams {
    pub server_id: String,
    pub command: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ConfirmParams {
    pub token: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListDirectoryParams {
    pub server_id: String,
    #[serde(default = "default_dir_path")]
    pub path: String,
}

fn default_dir_path() -> String {
    ".".to_string()
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UploadFileParams {
    pub server_id: String,
    #[schemars(description = "Absolute path to the file on this PC (where the servertool app runs), not the remote server")]
    pub local_path: String,
    #[schemars(description = "Destination path on the remote server")]
    pub remote_path: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DownloadFileParams {
    pub server_id: String,
    #[schemars(description = "Path to the file on the remote server")]
    pub remote_path: String,
    #[schemars(description = "Absolute destination path on this PC (where the servertool app runs), not the remote server")]
    pub local_path: String,
}

#[derive(Clone)]
pub struct ServertoolMcp {
    state: Arc<AppState>,
    tool_router: ToolRouter<Self>,
}

impl ServertoolMcp {
    pub fn new(state: Arc<AppState>) -> Self {
        Self {
            state,
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl ServertoolMcp {
    #[tool(
        description = "List saved server profiles (host/port/username, no passwords) and whether each currently has an active SSH session."
    )]
    async fn list_servers(&self) -> Result<Json<Vec<core::ServerSummary>>, String> {
        core::list_servers(&self.state).map(Json).map_err(|e| e.to_string())
    }

    #[tool(
        description = "Connect to a saved server profile over SSH. Reuses an existing connection if one is already open for this server (including one opened from the desktop GUI) instead of opening a new one."
    )]
    async fn connect_server(
        &self,
        Parameters(ServerIdParams { server_id }): Parameters<ServerIdParams>,
    ) -> Result<Json<bool>, String> {
        core::ensure_connected(&self.state, &server_id)
            .await
            .map(|_| Json(true))
            .map_err(|e| e.to_string())
    }

    #[tool(
        description = "Run a shell command on a connected server, over its existing SSH connection (opens a fresh exec channel; does not interfere with anything the user is doing in the GUI terminal). Connects first if the server isn't already connected. Destructive-looking commands are NOT executed — the result comes back with requires_confirmation=true and a confirmation_token; show the reason to the user, and only if they explicitly agree, call confirm_dangerous_command with that token to actually run it."
    )]
    async fn run_command(
        &self,
        Parameters(RunCommandParams { server_id, command }): Parameters<RunCommandParams>,
    ) -> Result<Json<core::CommandOutcome>, String> {
        core::run_command(&self.state, &server_id, &command)
            .await
            .map(Json)
            .map_err(|e| e.to_string())
    }

    #[tool(
        description = "Execute a command that run_command previously blocked as dangerous, after the user has explicitly confirmed it in chat. The token expires 5 minutes after run_command returned it."
    )]
    async fn confirm_dangerous_command(
        &self,
        Parameters(ConfirmParams { token }): Parameters<ConfirmParams>,
    ) -> Result<Json<core::CommandOutcome>, String> {
        core::confirm_dangerous_command(&self.state, &token)
            .await
            .map(Json)
            .map_err(|e| e.to_string())
    }

    #[tool(
        description = "List the contents of a directory on a connected server over SFTP. Path defaults to the login directory (\".\") if omitted."
    )]
    async fn list_directory(
        &self,
        Parameters(ListDirectoryParams { server_id, path }): Parameters<ListDirectoryParams>,
    ) -> Result<Json<core::RemoteListing>, String> {
        core::list_remote_directory(&self.state, &server_id, &path)
            .await
            .map(Json)
            .map_err(|e| e.to_string())
    }

    #[tool(
        description = "Upload a file from this PC (where the servertool desktop app runs) to a connected server over SFTP. local_path must be a path on this PC, not the remote server. Returns the number of bytes sent."
    )]
    async fn upload_file(
        &self,
        Parameters(UploadFileParams {
            server_id,
            local_path,
            remote_path,
        }): Parameters<UploadFileParams>,
    ) -> Result<Json<u64>, String> {
        core::upload_file(&self.state, &server_id, &local_path, &remote_path)
            .await
            .map(Json)
            .map_err(|e| e.to_string())
    }

    #[tool(
        description = "Download a file from a connected server to this PC (where the servertool desktop app runs) over SFTP. local_path is where it will be saved on this PC. Returns the number of bytes received."
    )]
    async fn download_file(
        &self,
        Parameters(DownloadFileParams {
            server_id,
            remote_path,
            local_path,
        }): Parameters<DownloadFileParams>,
    ) -> Result<Json<u64>, String> {
        core::download_file(&self.state, &server_id, &remote_path, &local_path)
            .await
            .map(Json)
            .map_err(|e| e.to_string())
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for ServertoolMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build()).with_instructions(
            "Controls SSH sessions to the user's own VPS servers via the ssh-mcp-client desktop app. \
             Sessions are shared with whatever the user has open in the app's GUI. \
             Destructive commands are blocked by run_command until confirmed via confirm_dangerous_command. \
             File tools (list_directory, upload_file, download_file) use SFTP and never delete anything; \
             local_path in upload_file/download_file refers to this PC, not the remote server.",
        )
    }
}
