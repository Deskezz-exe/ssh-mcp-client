use std::sync::{Arc, Mutex as StdMutex};

use base64::Engine;
use russh::keys::*;
use russh::*;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;

use crate::error::AppError;

/// SSH client event handler. We accept every host key at the protocol level
/// and instead do TOFU (trust-on-first-use) fingerprint comparison ourselves
/// right after `connect()` returns, once we know the caller's expectation.
struct ClientHandler {
    seen_fingerprint: Arc<StdMutex<Option<String>>>,
}

impl client::Handler for ClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &PublicKeyOrCertificate,
    ) -> Result<bool, Self::Error> {
        if let PublicKeyOrCertificate::PublicKey { key, .. } = server_public_key {
            let fp = key.fingerprint(HashAlg::Sha256).to_string();
            *self.seen_fingerprint.lock().unwrap() = Some(fp);
        }
        Ok(true)
    }
}

enum PtyInput {
    Data(Vec<u8>),
    Resize { cols: u32, rows: u32 },
}

struct PtyHandle {
    input_tx: mpsc::UnboundedSender<PtyInput>,
}

pub struct SshSession {
    pub server_id: String,
    handle: client::Handle<ClientHandler>,
    pty: StdMutex<Option<PtyHandle>>,
}

impl SshSession {
    /// Opens the TCP+SSH connection and authenticates with a password.
    /// Returns the session (with an empty `server_id` — set it after) and
    /// the SHA256 fingerprint of the host key actually presented, so the
    /// caller can do TOFU comparison/storage.
    pub async fn connect(
        host: &str,
        port: u16,
        username: &str,
        password: &str,
        expected_fingerprint: Option<String>,
    ) -> Result<(Self, String), AppError> {
        let seen_fingerprint = Arc::new(StdMutex::new(None));
        let config = Arc::new(client::Config::default());
        let sh = ClientHandler {
            seen_fingerprint: seen_fingerprint.clone(),
        };

        let mut handle = client::connect(config, (host, port), sh).await?;

        let observed = seen_fingerprint
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| AppError::Other("server did not present a host key".into()))?;

        if let Some(expected) = &expected_fingerprint {
            if expected != &observed {
                return Err(AppError::HostKeyMismatch {
                    expected: expected.clone(),
                    actual: observed,
                });
            }
        }

        let auth = handle.authenticate_password(username, password).await?;
        if !auth.success() {
            return Err(AppError::AuthFailed);
        }

        Ok((
            SshSession {
                server_id: String::new(),
                handle,
                pty: StdMutex::new(None),
            },
            observed,
        ))
    }

    /// Opens an interactive PTY + shell on a fresh channel and starts a
    /// background task that streams output to the frontend as
    /// `pty-output:{server_id}` events (base64-encoded, since terminal
    /// output isn't guaranteed to be valid UTF-8) and emits
    /// `pty-closed:{server_id}` when the remote shell exits.
    pub async fn open_pty(&self, app: AppHandle, cols: u32, rows: u32) -> Result<(), AppError> {
        let channel = self.handle.channel_open_session().await?;
        channel
            .request_pty(false, "xterm-256color", cols, rows, 0, 0, &[])
            .await?;
        channel.request_shell(true).await?;

        let (tx, mut rx) = mpsc::unbounded_channel::<PtyInput>();
        {
            let mut guard = self.pty.lock().unwrap();
            *guard = Some(PtyHandle { input_tx: tx });
        }

        let server_id = self.server_id.clone();
        tokio::spawn(async move {
            let mut channel = channel;
            loop {
                tokio::select! {
                    input = rx.recv() => {
                        match input {
                            Some(PtyInput::Data(bytes)) => {
                                if channel.data_bytes(bytes).await.is_err() {
                                    break;
                                }
                            }
                            Some(PtyInput::Resize { cols, rows }) => {
                                let _ = channel.window_change(cols, rows, 0, 0).await;
                            }
                            None => {
                                let _ = channel.eof().await;
                                break;
                            }
                        }
                    }
                    msg = channel.wait() => {
                        match msg {
                            Some(ChannelMsg::Data { data }) | Some(ChannelMsg::ExtendedData { data, .. }) => {
                                let payload = base64::engine::general_purpose::STANDARD.encode(&data);
                                let _ = app.emit(&format!("pty-output:{server_id}"), payload);
                            }
                            Some(ChannelMsg::Eof) | Some(ChannelMsg::Close) | None => break,
                            _ => {}
                        }
                    }
                }
            }
            let _ = app.emit(&format!("pty-closed:{server_id}"), ());
        });

        Ok(())
    }

    pub fn write_pty(&self, data: Vec<u8>) -> Result<(), AppError> {
        let guard = self.pty.lock().unwrap();
        let pty = guard
            .as_ref()
            .ok_or_else(|| AppError::Other("no PTY open for this server".into()))?;
        pty.input_tx
            .send(PtyInput::Data(data))
            .map_err(|_| AppError::Other("PTY channel closed".into()))
    }

    pub fn resize_pty(&self, cols: u32, rows: u32) -> Result<(), AppError> {
        let guard = self.pty.lock().unwrap();
        let pty = guard
            .as_ref()
            .ok_or_else(|| AppError::Other("no PTY open for this server".into()))?;
        pty.input_tx
            .send(PtyInput::Resize { cols, rows })
            .map_err(|_| AppError::Other("PTY channel closed".into()))
    }
}
