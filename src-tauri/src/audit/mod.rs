pub mod dangerous;

use std::path::Path;

use rusqlite::{params, Connection};

use crate::error::AppError;

pub fn open(app_data_dir: &Path) -> Result<Connection, AppError> {
    std::fs::create_dir_all(app_data_dir)?;
    let conn = Connection::open(app_data_dir.join("audit.db"))?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS command_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            ts TEXT NOT NULL,
            server_id TEXT NOT NULL,
            command TEXT NOT NULL,
            source TEXT NOT NULL,
            dangerous INTEGER NOT NULL,
            confirmed INTEGER NOT NULL,
            exit_code INTEGER,
            stdout_excerpt TEXT
        );",
    )?;
    Ok(conn)
}

#[allow(clippy::too_many_arguments)]
pub fn log_command(
    conn: &Connection,
    server_id: &str,
    command: &str,
    source: &str,
    dangerous: bool,
    confirmed: bool,
    exit_code: Option<i32>,
    stdout_excerpt: &str,
) -> Result<(), AppError> {
    let excerpt: String = stdout_excerpt.chars().take(2000).collect();
    conn.execute(
        "INSERT INTO command_log (ts, server_id, command, source, dangerous, confirmed, exit_code, stdout_excerpt)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            chrono::Utc::now().to_rfc3339(),
            server_id,
            command,
            source,
            dangerous as i32,
            confirmed as i32,
            exit_code,
            excerpt,
        ],
    )?;
    Ok(())
}
