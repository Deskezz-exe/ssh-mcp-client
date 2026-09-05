mod tools;

use std::net::SocketAddr;
use std::sync::Arc;

use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
};
use tokio_util::sync::CancellationToken;

use crate::state::AppState;

pub struct McpServerHandle {
    pub port: u16,
    cancellation_token: CancellationToken,
}

impl McpServerHandle {
    pub fn shutdown(&self) {
        self.cancellation_token.cancel();
    }
}

/// Starts the embedded MCP server on 127.0.0.1:{port} with the Streamable
/// HTTP transport, mounted at `/mcp`. Runs until `McpServerHandle::shutdown`
/// is called (wired to the Tauri app's exit event) or the process ends.
pub async fn start(state: Arc<AppState>, port: u16) -> std::io::Result<McpServerHandle> {
    let ct = CancellationToken::new();

    let service: StreamableHttpService<tools::ServertoolMcp, LocalSessionManager> = StreamableHttpService::new(
        move || Ok(tools::ServertoolMcp::new(state.clone())),
        Default::default(),
        StreamableHttpServerConfig::default().with_cancellation_token(ct.child_token()),
    );

    let router = axum::Router::new().nest_service("/mcp", service);
    let addr: SocketAddr = ([127, 0, 0, 1], port).into();
    let listener = tokio::net::TcpListener::bind(addr).await?;

    let ct_server = ct.clone();
    tokio::spawn(async move {
        let _ = axum::serve(listener, router)
            .with_graceful_shutdown(async move { ct_server.cancelled_owned().await })
            .await;
    });

    Ok(McpServerHandle {
        port,
        cancellation_token: ct,
    })
}
