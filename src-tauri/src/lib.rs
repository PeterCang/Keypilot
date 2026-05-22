mod adapters;
mod error;
mod installer;
mod models;
mod process;
mod storage;

use adapters::{
  backup_config_for_tool, detect_tool_auth_methods, read_current_tool_config, switch_key_for_record_with_source,
};
use chrono::Utc;
use models::{BackupResult, KeyRecord, SwitchResult, SyncKeyResult, ToolAuthSnapshot, ToolCurrentConfig, ToolStatus, ToolType};
use storage::{load_state, save_state};
use tauri::Emitter;
use tauri::Manager;
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::TrayIconBuilder;
use uuid::Uuid;

#[tauri::command]
fn list_keys() -> Result<Vec<KeyRecord>, String> {
  Ok(load_state().map_err(|e| e.to_string())?.keys)
}

fn tool_label(tool: ToolType) -> &'static str {
  match tool {
    ToolType::ClaudeCode => "claude-code",
    ToolType::Codex => "codex",
    ToolType::CodexApp => "codex-app",
    ToolType::GeminiCli => "gemini-cli",
  }
}

fn tools_share_config(left: ToolType, right: ToolType) -> bool {
  matches!(
    (left, right),
    (ToolType::Codex, ToolType::Codex)
      | (ToolType::Codex, ToolType::CodexApp)
      | (ToolType::CodexApp, ToolType::Codex)
      | (ToolType::CodexApp, ToolType::CodexApp)
  ) || left == right
}

fn build_tray_menu(app: &tauri::AppHandle) -> Result<tauri::menu::Menu<tauri::Wry>, String> {
  let state = load_state().map_err(|e| e.to_string())?;
  let mut builder = MenuBuilder::new(app).text("open", "Open").separator();
  for key in state.keys.iter().filter(|k| k.is_active) {
    builder = builder.text(
      format!("active:{}", key.id),
      format!("* {} ({})", key.name, tool_label(key.tool)),
    );
  }
  builder = builder.separator();
  for key in state.keys {
    builder = builder.text(format!("switch:{}", key.id), format!("Switch {}", key.name));
  }
  builder = builder.separator().item(&MenuItemBuilder::with_id("quit", "Quit").build(app).map_err(|e| e.to_string())?);
  builder.build().map_err(|e| e.to_string())
}

fn refresh_tray_menu(app: &tauri::AppHandle) -> Result<(), String> {
  let menu = build_tray_menu(app)?;
  let tray = app.tray_by_id("main-tray").ok_or_else(|| "main tray not found".to_string())?;
  tray.set_menu(Some(menu)).map_err(|e| e.to_string())
}

fn import_current_key_if_missing(
  state: &mut storage::AppState,
  tool: ToolType,
  current: &ToolCurrentConfig,
  is_active: bool,
) -> bool {
  let Some(api_key) = current.api_key.as_ref().map(|v| v.trim()).filter(|v| !v.is_empty()) else {
    return false;
  };
  let provider_name = current
    .provider_name
    .clone()
    .filter(|value| !value.trim().is_empty())
    .unwrap_or_else(|| current.source.clone());

  if let Some(existing_index) = state
    .keys
    .iter()
    .position(|item| tools_share_config(item.tool, tool) && item.api_key.trim() == api_key)
  {
    let existing_id = state.keys[existing_index].id.clone();
    let mut changed = false;
    if state.keys[existing_index].base_url != current.base_url {
      state.keys[existing_index].base_url = current.base_url.clone();
      changed = true;
    }
    if state.keys[existing_index].model != current.model {
      state.keys[existing_index].model = current.model.clone();
      changed = true;
    }
    if state.keys[existing_index].note.as_deref().unwrap_or("").trim().is_empty() {
      state.keys[existing_index].note = Some(provider_name);
      changed = true;
    }
    if is_active {
      for item in &mut state.keys {
        if tools_share_config(item.tool, tool) {
          let next_active = item.id == existing_id;
          if item.is_active != next_active {
            item.is_active = next_active;
            changed = true;
          }
        }
      }
    }
    if changed {
      state.keys[existing_index].updated_at = Some(Utc::now().to_rfc3339());
    }
    return changed;
  }

  let now = Utc::now().to_rfc3339();
  state.keys.push(KeyRecord {
    id: Uuid::new_v4().to_string(),
    name: format!("imported-{}-{}", tool_label(tool), &now[0..19].replace('T', " ")),
    tool,
    api_key: api_key.to_string(),
    base_url: current.base_url.clone(),
    model: current.model.clone(),
    is_active,
    created_at: now.clone(),
    updated_at: Some(now),
    note: Some(provider_name),
  });
  true
}

fn has_key_for_tool_config_group(state: &storage::AppState, tool: ToolType) -> bool {
  state.keys.iter().any(|item| tools_share_config(item.tool, tool))
}

fn ensure_initial_key_for_tool_state(
  state: &mut storage::AppState,
  tool: ToolType,
  current: &ToolCurrentConfig,
) -> bool {
  if has_key_for_tool_config_group(state, tool) {
    return false;
  }

  import_current_key_if_missing(state, tool, current, true)
}

#[tauri::command]
fn ensure_initial_key_for_tool(app: tauri::AppHandle, tool: ToolType) -> Result<Vec<KeyRecord>, String> {
  let mut state = load_state().map_err(|e| e.to_string())?;
  if !has_key_for_tool_config_group(&state, tool) {
    let current = read_current_tool_config(tool).map_err(|e| e.to_string())?;
    if ensure_initial_key_for_tool_state(&mut state, tool, &current) {
      save_state(&state).map_err(|e| e.to_string())?;
      let _ = refresh_tray_menu(&app);
    }
  }
  Ok(state.keys)
}

// Reads the key currently active in the tool config, syncs is_active in data.json to match,
// and imports the key if it is not yet in the list. Returns the updated key list and the
// effective auth snapshot so the frontend can show the read source.
#[tauri::command]
fn sync_active_key_for_tool(app: tauri::AppHandle, tool: ToolType) -> Result<SyncKeyResult, String> {
  let snapshots = detect_tool_auth_methods(tool).map_err(|e| e.to_string())?;
  let effective = snapshots.into_iter().find(|s| s.is_effective);

  let mut state = load_state().map_err(|e| e.to_string())?;
  let mut changed = false;

  if let Some(ref snap) = effective {
    if let Some(ref api_key) = snap.api_key {
      let api_key = api_key.trim();
      if !api_key.is_empty() {
        let current = ToolCurrentConfig {
          tool,
          api_key: Some(api_key.to_string()),
          base_url: snap.base_url.clone(),
          model: snap.model.clone(),
          provider_name: None,
          source: snap.source.clone(),
        };
        // import_current_key_if_missing marks the matched/new key as active and clears others
        if import_current_key_if_missing(&mut state, tool, &current, true) {
          changed = true;
        }
      }
    } else {
      // Tool has no key set — ensure at least the list is populated on first run
      if !has_key_for_tool_config_group(&state, tool) {
        let current = read_current_tool_config(tool).map_err(|e| e.to_string())?;
        if ensure_initial_key_for_tool_state(&mut state, tool, &current) {
          changed = true;
        }
      }
    }
  } else if !has_key_for_tool_config_group(&state, tool) {
    let current = read_current_tool_config(tool).map_err(|e| e.to_string())?;
    if ensure_initial_key_for_tool_state(&mut state, tool, &current) {
      changed = true;
    }
  }

  if changed {
    save_state(&state).map_err(|e| e.to_string())?;
    let _ = refresh_tray_menu(&app);
  }

  Ok(SyncKeyResult {
    keys: state.keys,
    effective_snapshot: effective,
  })
}

#[cfg(test)]
mod tests {
  use super::*;
  use storage::AppState;

  fn key_record(id: &str, api_key: &str, is_active: bool) -> KeyRecord {
    KeyRecord {
      id: id.to_string(),
      name: id.to_string(),
      tool: ToolType::Codex,
      api_key: api_key.to_string(),
      base_url: None,
      model: None,
      is_active,
      created_at: "2026-01-01T00:00:00Z".to_string(),
      updated_at: None,
      note: None,
    }
  }

  #[test]
  fn ensure_initial_key_does_not_override_existing_active_key() {
    let mut state = AppState {
      version: 1,
      keys: vec![key_record("first", "sk-first", true), key_record("second", "sk-second", false)],
    };
    let current = ToolCurrentConfig {
      tool: ToolType::Codex,
      api_key: Some("sk-second".to_string()),
      base_url: None,
      model: None,
      provider_name: None,
      source: "test".to_string(),
    };

    let changed = ensure_initial_key_for_tool_state(&mut state, ToolType::Codex, &current);

    assert!(!changed);
    assert!(state.keys[0].is_active);
    assert!(!state.keys[1].is_active);
  }

  #[test]
  fn ensure_initial_key_imports_current_config_when_group_is_empty() {
    let mut state = AppState::default();
    let current = ToolCurrentConfig {
      tool: ToolType::Codex,
      api_key: Some("sk-current".to_string()),
      base_url: Some("https://example.test/v1".to_string()),
      model: Some("gpt-5".to_string()),
      provider_name: Some("example".to_string()),
      source: "test".to_string(),
    };

    let changed = ensure_initial_key_for_tool_state(&mut state, ToolType::CodexApp, &current);

    assert!(changed);
    assert_eq!(state.keys.len(), 1);
    assert_eq!(state.keys[0].tool, ToolType::CodexApp);
    assert_eq!(state.keys[0].api_key, "sk-current");
    assert!(state.keys[0].is_active);
  }
}

#[tauri::command]
fn save_key(app: tauri::AppHandle, payload: KeyRecord) -> Result<KeyRecord, String> {
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
  let _ = refresh_tray_menu(&app);
  Ok(payload)
}

#[tauri::command]
fn delete_key(app: tauri::AppHandle, id: String) -> Result<bool, String> {
  let mut state = load_state().map_err(|e| e.to_string())?;
  let before = state.keys.len();
  state.keys.retain(|x| x.id != id);
  save_state(&state).map_err(|e| e.to_string())?;
  let _ = refresh_tray_menu(&app);
  Ok(state.keys.len() < before)
}

#[tauri::command]
fn switch_key(app: tauri::AppHandle, id: String) -> Result<SwitchResult, String> {
  let mut state = load_state().map_err(|e| e.to_string())?;
  let target = state
    .keys
    .iter()
    .find(|x| x.id == id)
    .cloned()
    .ok_or_else(|| "key not found".to_string())?;

  let current = read_current_tool_config(target.tool).map_err(|e| e.to_string())?;

  let mut result =
    switch_key_for_record_with_source(&target, &current.source).map_err(|e| e.to_string())?;
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
    if tools_share_config(item.tool, target.tool) {
      item.is_active = item.id == target.id;
    }
  }

  save_state(&state).map_err(|e| e.to_string())?;
  let _ = refresh_tray_menu(&app);
  Ok(result)
}

#[tauri::command]
fn detect_tools() -> Result<Vec<ToolStatus>, String> {
  Ok(vec![
    process::detect_tool(ToolType::ClaudeCode),
    process::detect_tool(ToolType::Codex),
    process::detect_tool(ToolType::CodexApp),
    process::detect_tool(ToolType::GeminiCli),
  ])
}

#[tauri::command]
fn backup_config(tool: ToolType) -> Result<BackupResult, String> {
  backup_config_for_tool(tool).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_tool_current_config(tool: ToolType) -> Result<ToolCurrentConfig, String> {
  read_current_tool_config(tool).map_err(|e| e.to_string())
}

#[tauri::command]
fn detect_tool_auth(tool: ToolType) -> Result<Vec<ToolAuthSnapshot>, String> {
  detect_tool_auth_methods(tool).map_err(|e| e.to_string())
}

#[tauri::command]
fn install_tool(app: tauri::AppHandle, tool: ToolType, custom_cmd: Option<String>) -> Result<String, String> {
  installer::install_tool(&app, tool, custom_cmd.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
fn restart_tool(tool: ToolType) -> Result<String, String> {
  process::restart_tool(tool)
}

#[tauri::command]
fn uninstall_tool(app: tauri::AppHandle, tool: ToolType, custom_cmd: Option<String>) -> Result<String, String> {
  installer::uninstall_tool(&app, tool, custom_cmd.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
fn start_tool(tool: ToolType, args: Option<String>, project_dir: Option<String>) -> Result<String, String> {
  installer::start_tool(tool, args.as_deref().unwrap_or(""), project_dir.as_deref()).map_err(|e| e.to_string())
}

pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_dialog::init())
    .setup(|app| {
      let app_handle = app.handle().clone();
      let menu = build_tray_menu(&app_handle)?;
      let _tray = TrayIconBuilder::with_id("main-tray")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(move |app, event| {
          let id = event.id.as_ref();
          if id == "quit" {
            app.exit(0);
            return;
          }
          if id == "open" {
            if let Some(window) = app.get_webview_window("main") {
              let _ = window.show();
              let _ = window.set_focus();
            }
            return;
          }
          if let Some(key_id) = id.strip_prefix("switch:") {
            if let Ok(mut state) = load_state() {
              if let Some(target) = state.keys.iter().find(|k| k.id == key_id).cloned() {
                let current = read_current_tool_config(target.tool).unwrap_or(ToolCurrentConfig {
                  tool: target.tool,
                  api_key: None,
                  base_url: None,
                  model: None,
                  provider_name: None,
                  source: "none".to_string(),
                });
                if switch_key_for_record_with_source(&target, &current.source).is_ok() {
                  for item in &mut state.keys {
                    if tools_share_config(item.tool, target.tool) {
                      item.is_active = item.id == target.id;
                    }
                  }
                  let _ = save_state(&state);
                  let _ = app.emit("key-switched", &target.id);
                }
              }
            }
            let _ = refresh_tray_menu(app);
          }
        })
        .build(app)?;
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      list_keys,
      ensure_initial_key_for_tool,
      sync_active_key_for_tool,
      save_key,
      delete_key,
      switch_key,
      detect_tools,
      backup_config,
      get_tool_current_config,
      detect_tool_auth,
      install_tool,
      restart_tool,
      uninstall_tool,
      start_tool
    ])
    .run(tauri::generate_context!())
    .expect("error while running keypilot app");
}
