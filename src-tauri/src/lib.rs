mod adapters;
mod error;
mod installer;
mod models;
mod process;
mod storage;

use adapters::{backup_config_for_tool, switch_key_for_record};
use models::{BackupResult, KeyRecord, SwitchResult, ToolStatus, ToolType};
use storage::{load_state, save_state};

#[tauri::command]
fn list_keys() -> Result<Vec<KeyRecord>, String> {
  Ok(load_state().map_err(|e| e.to_string())?.keys)
}

#[tauri::command]
fn save_key(payload: KeyRecord) -> Result<KeyRecord, String> {
  let mut state = load_state().map_err(|e| e.to_string())?;
  let mut replaced = false;

  for item in &mut state.keys {
    if item.id == payload.id {
      *item = payload.clone();
      replaced = true;
      break;
    }
  }

  if !replaced {
    state.keys.push(payload.clone());
  }

  save_state(&state).map_err(|e| e.to_string())?;
  Ok(payload)
}

#[tauri::command]
fn delete_key(id: String) -> Result<bool, String> {
  let mut state = load_state().map_err(|e| e.to_string())?;
  let before = state.keys.len();
  state.keys.retain(|x| x.id != id);
  save_state(&state).map_err(|e| e.to_string())?;
  Ok(state.keys.len() < before)
}

#[tauri::command]
fn switch_key(id: String) -> Result<SwitchResult, String> {
  let mut state = load_state().map_err(|e| e.to_string())?;
  let target = state
    .keys
    .iter()
    .find(|x| x.id == id)
    .cloned()
    .ok_or_else(|| "key not found".to_string())?;

  let mut result = switch_key_for_record(&target).map_err(|e| e.to_string())?;
  let running = process::is_tool_running(target.tool);
  result.requires_restart = running;
  result.warning = if running {
    Some("target tool is running, restart recommended".to_string())
  } else {
    None
  };
  if running {
    result.message = format!("{}; tool restart is recommended", result.message);
  }

  for item in &mut state.keys {
    if item.tool == target.tool {
      item.is_active = item.id == target.id;
    }
  }

  save_state(&state).map_err(|e| e.to_string())?;
  Ok(result)
}

#[tauri::command]
fn detect_tools() -> Result<Vec<ToolStatus>, String> {
  Ok(vec![
    process::detect_tool(ToolType::ClaudeCode),
    process::detect_tool(ToolType::Codex),
    process::detect_tool(ToolType::GeminiCli),
  ])
}

#[tauri::command]
fn backup_config(tool: ToolType) -> Result<BackupResult, String> {
  backup_config_for_tool(tool).map_err(|e| e.to_string())
}

#[tauri::command]
fn install_tool(tool: ToolType) -> Result<String, String> {
  installer::install_tool(tool).map_err(|e| e.to_string())
}

#[tauri::command]
fn restart_tool(tool: ToolType) -> Result<String, String> {
  process::restart_tool(tool)
}

pub fn run() {
  tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![
      list_keys,
      save_key,
      delete_key,
      switch_key,
      detect_tools,
      backup_config,
      install_tool,
      restart_tool
    ])
    .run(tauri::generate_context!())
    .expect("error while running keypilot app");
}
