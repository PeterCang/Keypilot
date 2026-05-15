use crate::error::AppError;
use crate::models::KeyRecord;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AppState {
  pub version: u32,
  pub keys: Vec<KeyRecord>,
}

impl Default for AppState {
  fn default() -> Self {
    Self {
      version: 1,
      keys: Vec::new(),
    }
  }
}

pub fn data_file_path() -> Result<PathBuf, AppError> {
  let home = std::env::var("HOME")
    .or_else(|_| std::env::var("USERPROFILE"))
    .map_err(|_| AppError::InvalidState("cannot resolve home directory".to_string()))?;

  let dir = if cfg!(target_os = "windows") {
    Path::new(&home).join("AppData").join("Roaming").join("KeyPilot")
  } else {
    Path::new(&home)
      .join("Library")
      .join("Application Support")
      .join("KeyPilot")
  };

  fs::create_dir_all(&dir)?;
  Ok(dir.join("data.json"))
}

pub fn load_state() -> Result<AppState, AppError> {
  let path = data_file_path()?;
  if !path.exists() {
    let state = AppState::default();
    save_state(&state)?;
    return Ok(state);
  }

  let content = fs::read_to_string(path)?;
  Ok(serde_json::from_str(&content)?)
}

pub fn save_state(state: &AppState) -> Result<(), AppError> {
  let path = data_file_path()?;
  let temp_path = path.with_extension("json.tmp");
  let backup_path = path.with_extension("json.bak");

  if path.exists() {
    fs::copy(&path, &backup_path)?;
  }

  let body = serde_json::to_string_pretty(state)?;
  fs::write(&temp_path, body)?;
  fs::rename(&temp_path, &path)?;

  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::models::{KeyRecord, ToolType};
  use std::sync::{Mutex, OnceLock};
  use std::time::{SystemTime, UNIX_EPOCH};

  fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
  }

  fn unique_temp_home() -> PathBuf {
    let nanos = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .expect("time went backwards")
      .as_nanos();
    std::env::temp_dir().join(format!("keypilot-storage-test-{nanos}"))
  }

  #[test]
  fn load_and_save_state_roundtrip() {
    let _guard = env_lock().lock().expect("lock poisoned");
    let test_home = unique_temp_home();
    fs::create_dir_all(&test_home).expect("failed to create temp dir");
    std::env::set_var("HOME", &test_home);
    std::env::set_var("USERPROFILE", &test_home);

    let mut state = AppState::default();
    state.keys.push(KeyRecord {
      id: "k1".to_string(),
      name: "primary".to_string(),
      tool: ToolType::Codex,
      api_key: "sk-test".to_string(),
      base_url: None,
      model: Some("gpt-5".to_string()),
      is_active: true,
      created_at: "2026-01-01T00:00:00Z".to_string(),
      updated_at: Some("2026-01-01T00:00:00Z".to_string()),
      note: Some("note".to_string()),
    });

    save_state(&state).expect("save_state failed");
    let loaded = load_state().expect("load_state failed");
    assert_eq!(loaded.keys.len(), 1);
    assert_eq!(loaded.keys[0].name, "primary");
    assert!(data_file_path().expect("data path missing").exists());
  }
}
