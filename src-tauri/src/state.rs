use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::ssh::SshSession;

pub struct AppState {
    pub sessions: Mutex<HashMap<String, Arc<SshSession>>>,
    pub app_data_dir: PathBuf,
}
