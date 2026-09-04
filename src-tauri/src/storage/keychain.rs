use crate::error::AppError;

const SERVICE: &str = "ssh-mcp-client";

pub fn set_password(server_id: &str, password: &str) -> Result<(), AppError> {
    let entry = keyring::Entry::new(SERVICE, server_id).map_err(|e| AppError::Keychain(e.to_string()))?;
    entry.set_password(password).map_err(|e| AppError::Keychain(e.to_string()))
}

pub fn get_password(server_id: &str) -> Result<String, AppError> {
    let entry = keyring::Entry::new(SERVICE, server_id).map_err(|e| AppError::Keychain(e.to_string()))?;
    entry.get_password().map_err(|e| AppError::Keychain(e.to_string()))
}

pub fn delete_password(server_id: &str) -> Result<(), AppError> {
    let entry = keyring::Entry::new(SERVICE, server_id).map_err(|e| AppError::Keychain(e.to_string()))?;
    entry.delete_credential().map_err(|e| AppError::Keychain(e.to_string()))
}
