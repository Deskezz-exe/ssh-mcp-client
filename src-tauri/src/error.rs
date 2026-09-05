use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("ssh error: {0}")]
    Ssh(#[from] russh::Error),
    #[error("sftp error: {0}")]
    Sftp(#[from] russh_sftp::client::error::Error),
    #[error("server not found: {0}")]
    ServerNotFound(String),
    #[error("authentication failed")]
    AuthFailed,
    #[error("host key changed for this server: expected {expected}, got {actual} (possible MITM or reinstalled server — delete and re-add the profile if this is expected)")]
    HostKeyMismatch { expected: String, actual: String },
    #[error("keychain error: {0}")]
    Keychain(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("unknown or expired confirmation token")]
    UnknownToken,
    #[error("{0}")]
    Other(String),
}
