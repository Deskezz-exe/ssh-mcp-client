use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerProfile {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    #[serde(default)]
    pub host_key_fingerprint: Option<String>,
    #[serde(default)]
    pub favorite: bool,
}

fn profiles_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("servers.json")
}

pub fn load(app_data_dir: &Path) -> Result<Vec<ServerProfile>, AppError> {
    let path = profiles_path(app_data_dir);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let data = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&data)?)
}

pub fn save(app_data_dir: &Path, profiles: &[ServerProfile]) -> Result<(), AppError> {
    std::fs::create_dir_all(app_data_dir)?;
    let data = serde_json::to_string_pretty(profiles)?;
    std::fs::write(profiles_path(app_data_dir), data)?;
    Ok(())
}

pub fn upsert(app_data_dir: &Path, profile: ServerProfile) -> Result<(), AppError> {
    let mut profiles = load(app_data_dir)?;
    if let Some(existing) = profiles.iter_mut().find(|p| p.id == profile.id) {
        *existing = profile;
    } else {
        profiles.push(profile);
    }
    save(app_data_dir, &profiles)
}

pub fn delete(app_data_dir: &Path, id: &str) -> Result<(), AppError> {
    let mut profiles = load(app_data_dir)?;
    profiles.retain(|p| p.id != id);
    save(app_data_dir, &profiles)
}

pub fn find(app_data_dir: &Path, id: &str) -> Result<Option<ServerProfile>, AppError> {
    Ok(load(app_data_dir)?.into_iter().find(|p| p.id == id))
}

pub fn set_host_key_fingerprint(app_data_dir: &Path, id: &str, fingerprint: &str) -> Result<(), AppError> {
    let mut profiles = load(app_data_dir)?;
    if let Some(p) = profiles.iter_mut().find(|p| p.id == id) {
        p.host_key_fingerprint = Some(fingerprint.to_string());
    }
    save(app_data_dir, &profiles)
}

pub fn set_favorite(app_data_dir: &Path, id: &str, favorite: bool) -> Result<(), AppError> {
    let mut profiles = load(app_data_dir)?;
    if let Some(p) = profiles.iter_mut().find(|p| p.id == id) {
        p.favorite = favorite;
    }
    save(app_data_dir, &profiles)
}
