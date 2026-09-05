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
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for ServertoolMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build()).with_instructions(
            "Controls SSH sessions to the user's own VPS servers via the ssh-mcp-client desktop app. \
             Sessions are shared with whatever the user has open in the app's GUI. \
             Destructive commands are blocked by run_command until confirmed via confirm_dangerous_command.",
        )
    }
}
