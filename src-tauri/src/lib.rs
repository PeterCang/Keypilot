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
use models::{BackupResult, KeyRecord, SwitchResult, ToolAuthSnapshot, ToolCurrentConfig, ToolStatus, ToolType};
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

#[tauri::command]
fn ensure_initial_key_for_tool(app: tauri::AppHandle, tool: ToolType) -> Result<Vec<KeyRecord>, String> {
  let mut state = load_state().map_err(|e| e.to_string())?;
  let current = read_current_tool_config(tool).map_err(|e| e.to_string())?;
  if import_current_key_if_missing(&mut state, tool, &current, true) {
    save_state(&state).map_err(|e| e.to_string())?;
    let _ = refresh_tray_menu(&app);
  }
  Ok(state.keys)
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
fn install_tool(app: tauri::AppHandle, tool: ToolType) -> Result<String, String> {
  installer::install_tool(&app, tool).map_err(|e| e.to_string())
}

#[tauri::command]
fn restart_tool(tool: ToolType) -> Result<String, String> {
  process::restart_tool(tool)
}

#[tauri::command]
fn uninstall_tool(app: tauri::AppHandle, tool: ToolType) -> Result<String, String> {
  installer::uninstall_tool(&app, tool).map_err(|e| e.to_string())
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
