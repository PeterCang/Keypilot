use crate::error::AppError;
use crate::models::{
  AuthMethodType, BackupResult, KeyRecord, SwitchResult, ToolAuthSnapshot, ToolCurrentConfig,
  ToolType,
};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn user_home() -> Result<PathBuf, AppError> {
  let home = std::env::var("HOME")
    .or_else(|_| std::env::var("USERPROFILE"))
    .map_err(|_| AppError::InvalidState("cannot resolve home".to_string()))?;
  Ok(PathBuf::from(home))
}

fn copy_if_exists(tool: ToolType, source: PathBuf) -> Result<BackupResult, AppError> {
  if !source.exists() {
    return Ok(BackupResult {
      tool,
      backup_path: None,
      success: true,
      message: "target config does not exist yet".to_string(),
    });
  }

  let backup = rotate_backups_and_copy_current(&source)?;
  Ok(BackupResult {
    tool,
    backup_path: Some(backup.display().to_string()),
    success: true,
    message: "backup created".to_string(),
  })
}

fn backup_path_for_index(source: &PathBuf, index: usize) -> PathBuf {
  let file_name = source
    .file_name()
    .map(|name| name.to_string_lossy().to_string())
    .unwrap_or_else(|| "config".to_string());
  source.with_file_name(format!("{file_name}.bak{index}"))
}

fn rotate_backups_and_copy_current(source: &PathBuf) -> Result<PathBuf, AppError> {
  if !source.exists() {
    return Err(AppError::InvalidState(format!(
      "cannot back up missing file: {}",
      source.display()
    )));
  }

  let oldest = backup_path_for_index(source, 5);
  if oldest.exists() {
    fs::remove_file(&oldest)?;
  }

  for index in (1..=4).rev() {
    let current = backup_path_for_index(source, index);
    if current.exists() {
      let next = backup_path_for_index(source, index + 1);
      fs::rename(current, next)?;
    }
  }

  let latest = backup_path_for_index(source, 1);
  fs::copy(source, &latest)?;
  Ok(latest)
}

fn run_command(program: &str, args: &[&str]) -> Result<(), AppError> {
  let status = Command::new(program).args(args).status()?;
  if !status.success() {
    return Err(AppError::InvalidState(format!(
      "command failed: {} {}",
      program,
      args.join(" ")
    )));
  }
  Ok(())
}

#[cfg(target_os = "windows")]
#[derive(Clone, Copy)]
enum WindowsEnvScope {
  User,
  Machine,
}

#[cfg(target_os = "windows")]
fn verify_scope_env_var(
  scope: WindowsEnvScope,
  key: &str,
  value: Option<&str>,
) -> Result<(), AppError> {
  let actual = match scope {
    WindowsEnvScope::User => read_registry_env_var("HKCU\\Environment", key)?,
    WindowsEnvScope::Machine => read_registry_env_var(
      "HKLM\\SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment",
      key,
    )?,
  };
  let expected = value.map(|v| v.to_string());
  if actual != expected {
    return Err(AppError::InvalidState(format!(
      "env verify failed for {key} in {:?} scope: expected {:?}, got {:?}",
      match scope {
        WindowsEnvScope::User => "user",
        WindowsEnvScope::Machine => "machine",
      },
      expected,
      actual
    )));
  }
  Ok(())
}

#[cfg(target_os = "windows")]
fn persist_user_env_var(key: &str, value: Option<&str>) -> Result<(), AppError> {
  match value {
    Some(v) => run_command(
      "reg",
      &[
        "add",
        "HKCU\\Environment",
        "/v",
        key,
        "/t",
        "REG_SZ",
        "/d",
        v,
        "/f",
      ],
    ),
    None => run_command("reg", &["delete", "HKCU\\Environment", "/v", key, "/f"]),
  }
}

#[cfg(target_os = "windows")]
fn persist_machine_env_var(key: &str, value: Option<&str>) -> Result<(), AppError> {
  match value {
    Some(v) => run_command(
      "reg",
      &[
        "add",
        "HKLM\\SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment",
        "/v",
        key,
        "/t",
        "REG_SZ",
        "/d",
        v,
        "/f",
      ],
    ),
    None => run_command(
      "reg",
      &[
        "delete",
        "HKLM\\SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment",
        "/v",
        key,
        "/f",
      ],
    ),
  }
}

#[cfg(target_os = "windows")]
fn read_registry_env_var(scope_key: &str, key: &str) -> Result<Option<String>, AppError> {
  let output = Command::new("reg")
    .args(["query", scope_key, "/v", key])
    .output()?;
  if !output.status.success() {
    return Ok(None);
  }
  let stdout = String::from_utf8_lossy(&output.stdout);
  for line in stdout.lines() {
    let trimmed = line.trim();
    if trimmed.starts_with(key) && trimmed.contains("REG_") {
      if let Some(reg_index) = trimmed.find("REG_") {
        let rest = trimmed[(reg_index + 4)..].trim_start();
        if let Some(space_index) = rest.find(char::is_whitespace) {
          let value = rest[(space_index + 1)..].trim();
          if !value.is_empty() {
            return Ok(Some(value.to_string()));
          }
        }
      }
    }
  }
  Ok(None)
}

#[cfg(target_os = "windows")]
#[allow(dead_code)]
fn read_user_env_var(key: &str) -> Result<Option<String>, AppError> {
  read_registry_env_var("HKCU\\Environment", key)
}

#[cfg(target_os = "windows")]
fn read_effective_env_var(key: &str) -> Result<Option<String>, AppError> {
  if let Ok(v) = std::env::var(key) {
    if !v.trim().is_empty() {
      return Ok(Some(v));
    }
  }
  if let Some(v) = read_registry_env_var("HKCU\\Environment", key)? {
    return Ok(Some(v));
  }
  read_registry_env_var(
    "HKLM\\SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment",
    key,
  )
}

#[cfg(target_os = "macos")]
fn persist_user_env_var(key: &str, value: Option<&str>) -> Result<(), AppError> {
  match value {
    Some(v) => run_command("launchctl", &["setenv", key, v])?,
    None => run_command("launchctl", &["unsetenv", key])?,
  }
  Ok(())
}

#[cfg(target_os = "macos")]
fn read_user_env_var(key: &str) -> Result<Option<String>, AppError> {
  let output = Command::new("launchctl").args(["getenv", key]).output()?;
  if !output.status.success() {
    return Ok(None);
  }
  let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
  if value.is_empty() {
    Ok(None)
  } else {
    Ok(Some(value))
  }
}

#[cfg(target_os = "macos")]
fn read_effective_env_var(key: &str) -> Result<Option<String>, AppError> {
  read_user_env_var(key)
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn persist_user_env_var(_key: &str, _value: Option<&str>) -> Result<(), AppError> {
  Err(AppError::InvalidState(
    "persistent env var is not supported on this platform".to_string(),
  ))
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn read_user_env_var(_key: &str) -> Result<Option<String>, AppError> {
  Ok(None)
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn read_effective_env_var(_key: &str) -> Result<Option<String>, AppError> {
  Ok(None)
}

fn set_and_verify_env_var(key: &str, value: Option<&str>) -> Result<(), AppError> {
  #[cfg(target_os = "windows")]
  {
    let scope = if read_registry_env_var("HKCU\\Environment", key)?.is_some() {
      WindowsEnvScope::User
    } else if read_registry_env_var(
      "HKLM\\SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment",
      key,
    )?
    .is_some()
    {
      WindowsEnvScope::Machine
    } else {
      WindowsEnvScope::User
    };

    match scope {
      WindowsEnvScope::User => {
        persist_user_env_var(key, value)?;
        verify_scope_env_var(WindowsEnvScope::User, key, value)?;
      }
      WindowsEnvScope::Machine => {
        if persist_machine_env_var(key, value).is_ok() {
          verify_scope_env_var(WindowsEnvScope::Machine, key, value)?;
        } else {
          persist_user_env_var(key, value)?;
          verify_scope_env_var(WindowsEnvScope::User, key, value)?;
        }
      }
    }

    match value {
      Some(v) => std::env::set_var(key, v),
      None => std::env::remove_var(key),
    }
    return Ok(());
  }

  #[cfg(not(target_os = "windows"))]
  {
    persist_user_env_var(key, value)?;
    let actual = read_user_env_var(key)?;
    let expected = value.map(|v| v.to_string());
    if actual != expected {
      return Err(AppError::InvalidState(format!(
        "env verify failed for {key}: expected {:?}, got {:?}",
        expected, actual
      )));
    }

    match value {
      Some(v) => std::env::set_var(key, v),
      None => std::env::remove_var(key),
    }
    Ok(())
  }
}

fn switch_env_with_rollback(changes: &[(&str, Option<&str>)]) -> Result<(), AppError> {
  #[cfg(target_os = "windows")]
  let mut applied: Vec<(&str, Option<String>, Option<String>)> = Vec::new();
  #[cfg(not(target_os = "windows"))]
  let mut applied: Vec<(&str, Option<String>)> = Vec::new();

  for (key, value) in changes {
    #[cfg(target_os = "windows")]
    let before_user = read_registry_env_var("HKCU\\Environment", key)?;
    #[cfg(target_os = "windows")]
    let before_machine = read_registry_env_var(
      "HKLM\\SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment",
      key,
    )?;
    #[cfg(not(target_os = "windows"))]
    let before = read_user_env_var(key)?;

    if let Err(e) = set_and_verify_env_var(key, *value) {
      #[cfg(target_os = "windows")]
      {
        for (rollback_key, user_before, machine_before) in applied.iter().rev() {
          let _ = persist_user_env_var(rollback_key, user_before.as_deref());
          let _ = persist_machine_env_var(rollback_key, machine_before.as_deref());
        }
      }
      #[cfg(not(target_os = "windows"))]
      {
        for (rollback_key, rollback_value) in applied.iter().rev() {
          let _ = set_and_verify_env_var(rollback_key, rollback_value.as_deref());
        }
      }
      return Err(e);
    }
    #[cfg(target_os = "windows")]
    applied.push((key, before_user, before_machine));
    #[cfg(not(target_os = "windows"))]
    applied.push((key, before));
  }
  Ok(())
}

fn toml_escape(value: &str) -> String {
  value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn codex_provider_name(record: &KeyRecord) -> String {
  record
    .note
    .as_deref()
    .map(str::trim)
    .filter(|x| !x.is_empty())
    .or_else(|| {
      let name = record.name.trim();
      if name.is_empty() {
        None
      } else {
        Some(name)
      }
    })
    .unwrap_or("openai")
    .to_string()
}

fn is_toml_key_line(line: &str, key: &str) -> bool {
  let trimmed = line.trim_start();
  if !trimmed.starts_with(key) {
    return false;
  }
  trimmed[key.len()..].trim_start().starts_with('=')
}

fn toml_assignment_line(key: &str, value: &str) -> String {
  format!("{key} = \"{}\"", toml_escape(value))
}

fn replace_or_insert_top_level_string(content: &str, key: &str, value: &str) -> String {
  let mut lines: Vec<String> = content.lines().map(|x| x.to_string()).collect();
  let had_trailing_newline = content.ends_with('\n');
  let first_section_index = lines
    .iter()
    .position(|line| {
      let trimmed = line.trim();
      trimmed.starts_with('[') && trimmed.ends_with(']')
    })
    .unwrap_or(lines.len());

  for line in lines.iter_mut().take(first_section_index) {
    if is_toml_key_line(line, key) {
      let indent_len = line.len() - line.trim_start().len();
      let indent = &line[..indent_len];
      *line = format!("{indent}{}", toml_assignment_line(key, value));
      return join_toml_lines(lines, had_trailing_newline);
    }
  }

  let insert_at = first_section_index;
  lines.insert(insert_at, toml_assignment_line(key, value));
  join_toml_lines(lines, true)
}

fn find_toml_section(lines: &[String], section: &str) -> Option<(usize, usize)> {
  let header = format!("[{section}]");
  let start = lines.iter().position(|line| line.trim() == header)?;
  let end = lines[(start + 1)..]
    .iter()
    .position(|line| {
      let trimmed = line.trim();
      trimmed.starts_with('[') && trimmed.ends_with(']')
    })
    .map(|offset| start + 1 + offset)
    .unwrap_or(lines.len());
  Some((start, end))
}

fn replace_or_insert_section_string(
  content: &str,
  section: &str,
  key: &str,
  value: &str,
) -> String {
  let mut lines: Vec<String> = content.lines().map(|x| x.to_string()).collect();
  let had_trailing_newline = content.ends_with('\n');

  if let Some((start, end)) = find_toml_section(&lines, section) {
    for line in lines.iter_mut().take(end).skip(start + 1) {
      if is_toml_key_line(line, key) {
        let indent_len = line.len() - line.trim_start().len();
        let indent = &line[..indent_len];
        *line = format!("{indent}{}", toml_assignment_line(key, value));
        return join_toml_lines(lines, had_trailing_newline);
      }
    }
    lines.insert(end, toml_assignment_line(key, value));
    return join_toml_lines(lines, true);
  }

  if !lines.is_empty() && lines.last().is_some_and(|line| !line.trim().is_empty()) {
    lines.push(String::new());
  }
  lines.push(format!("[{section}]"));
  lines.push(toml_assignment_line(key, value));
  join_toml_lines(lines, true)
}

fn join_toml_lines(lines: Vec<String>, trailing_newline: bool) -> String {
  let mut body = lines.join("\n");
  if trailing_newline && !body.ends_with('\n') {
    body.push('\n');
  }
  body
}

fn update_codex_config_toml(existing: Option<&str>, record: &KeyRecord) -> String {
  let provider = codex_provider_name(record);
  let mut content = existing.unwrap_or("").to_string();

  content = replace_or_insert_top_level_string(&content, "model_provider", &provider);

  if let Some(model) = record
    .model
    .as_deref()
    .map(str::trim)
    .filter(|x| !x.is_empty())
  {
    content = replace_or_insert_top_level_string(&content, "model", model);
  }

  let section = format!("model_providers.{provider}");
  content = replace_or_insert_section_string(&content, &section, "name", &provider);
  if let Some(base_url) = record
    .base_url
    .as_deref()
    .map(str::trim)
    .filter(|x| !x.is_empty())
  {
    content = replace_or_insert_section_string(&content, &section, "base_url", base_url);
  }
  replace_or_insert_section_string(&content, &section, "wire_api", "responses")
}

fn update_codex_auth_json(existing: Option<&str>, record: &KeyRecord) -> Result<Vec<u8>, AppError> {
  let mut root = if let Some(content) = existing {
    serde_json::from_str::<serde_json::Value>(content)?
  } else {
    serde_json::json!({})
  };

  if !root.is_object() {
    root = serde_json::json!({});
  }
  let obj = root
    .as_object_mut()
    .ok_or_else(|| AppError::InvalidState("invalid codex auth root object".to_string()))?;
  obj.insert(
    "OPENAI_API_KEY".to_string(),
    serde_json::Value::String(record.api_key.clone()),
  );
  Ok(serde_json::to_vec_pretty(&root)?)
}

fn write_atomic(path: &PathBuf, bytes: &[u8]) -> Result<(), AppError> {
  let tmp_path = path.with_extension("tmp");
  fs::write(&tmp_path, bytes)?;
  fs::rename(&tmp_path, path)?;
  Ok(())
}

fn restore_file(path: &PathBuf, backup: &Option<Vec<u8>>) -> Result<(), AppError> {
  match backup {
    Some(content) => fs::write(path, content)?,
    None => {
      if path.exists() {
        fs::remove_file(path)?;
      }
    }
  }
  Ok(())
}

fn parse_toml_string_value(content: &str, key: &str) -> Option<String> {
  let needle = format!("{key} =");
  for raw_line in content.lines() {
    let line = raw_line.trim();
    if !line.starts_with(&needle) {
      continue;
    }
    let eq_index = line.find('=')?;
    let value = line[(eq_index + 1)..].trim();
    if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
      return Some(value[1..(value.len() - 1)].to_string());
    }
  }
  None
}

fn parse_toml_string_value_in_section(content: &str, section: &str, key: &str) -> Option<String> {
  let section_header = format!("[{section}]");
  let key_needle = format!("{key} =");
  let mut in_target_section = false;
  for raw_line in content.lines() {
    let line = raw_line.trim();
    if line.starts_with('[') && line.ends_with(']') {
      in_target_section = line == section_header;
      continue;
    }
    if !in_target_section {
      continue;
    }
    if !line.starts_with(&key_needle) {
      continue;
    }
    let eq_index = line.find('=')?;
    let value = line[(eq_index + 1)..].trim();
    if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
      return Some(value[1..(value.len() - 1)].to_string());
    }
  }
  None
}

fn read_codex_config_values(
  config_path: &PathBuf,
) -> Result<(Option<String>, Option<String>), AppError> {
  if !config_path.exists() {
    return Ok((None, None));
  }
  let content = fs::read_to_string(config_path)?;
  let model = parse_toml_string_value(&content, "model");
  let model_provider = parse_toml_string_value(&content, "model_provider");

  let base_url = if let Some(provider) = model_provider {
    parse_toml_string_value_in_section(&content, &format!("model_providers.{provider}"), "base_url")
      .or_else(|| parse_toml_string_value(&content, "base_url"))
  } else {
    parse_toml_string_value(&content, "base_url")
  };

  Ok((base_url, model))
}

fn resolve_effective_snapshot(mut snapshots: Vec<ToolAuthSnapshot>) -> Vec<ToolAuthSnapshot> {
  snapshots.sort_by_key(|x| x.priority);
  let mut marked = false;
  for item in &mut snapshots {
    let has_value = item.api_key.is_some() || item.base_url.is_some() || item.model.is_some();
    item.is_effective = !marked && has_value;
    if item.is_effective {
      marked = true;
    }
  }
  snapshots
}

fn to_current_config(tool: ToolType, snapshots: &[ToolAuthSnapshot]) -> ToolCurrentConfig {
  if let Some(effective) = snapshots.iter().find(|x| x.is_effective) {
    return ToolCurrentConfig {
      tool,
      api_key: effective.api_key.clone(),
      base_url: effective.base_url.clone(),
      model: effective.model.clone(),
      source: effective.source.clone(),
    };
  }
  ToolCurrentConfig {
    tool,
    api_key: None,
    base_url: None,
    model: None,
    source: "none".to_string(),
  }
}

fn read_claude_settings_json(home: &PathBuf) -> Result<(Option<String>, Option<String>), AppError> {
  let settings_path = home.join(".claude").join("settings.json");
  if !settings_path.exists() {
    return Ok((None, None));
  }
  let content = fs::read_to_string(settings_path)?;
  let json: serde_json::Value = serde_json::from_str(&content)?;
  let env = json.get("env").and_then(|v| v.as_object());
  let api_key = env
    .and_then(|m| m.get("ANTHROPIC_AUTH_TOKEN"))
    .and_then(|v| v.as_str())
    .map(|v| v.to_string())
    .or_else(|| {
      env
        .and_then(|m| m.get("ANTHROPIC_API_KEY"))
        .and_then(|v| v.as_str())
        .map(|v| v.to_string())
    });
  let base_url = env
    .and_then(|m| m.get("ANTHROPIC_BASE_URL"))
    .and_then(|v| v.as_str())
    .map(|v| v.to_string());
  Ok((api_key, base_url))
}

fn write_claude_settings_json(record: &KeyRecord) -> Result<(), AppError> {
  let home = user_home()?;
  let claude_dir = home.join(".claude");
  fs::create_dir_all(&claude_dir)?;
  let settings_path = claude_dir.join("settings.json");
  let settings_before = if settings_path.exists() {
    Some(fs::read(&settings_path)?)
  } else {
    None
  };
  if settings_path.exists() {
    rotate_backups_and_copy_current(&settings_path)?;
  }

  let mut root = if settings_path.exists() {
    let content = fs::read_to_string(&settings_path)?;
    serde_json::from_str::<serde_json::Value>(&content)?
  } else {
    serde_json::json!({})
  };

  if !root.is_object() {
    root = serde_json::json!({});
  }
  let obj = root
    .as_object_mut()
    .ok_or_else(|| AppError::InvalidState("invalid claude settings root object".to_string()))?;

  let env_value = obj.entry("env").or_insert_with(|| serde_json::json!({}));
  if !env_value.is_object() {
    *env_value = serde_json::json!({});
  }

  let env_obj = env_value
    .as_object_mut()
    .ok_or_else(|| AppError::InvalidState("invalid claude settings env object".to_string()))?;
  env_obj.insert(
    "ANTHROPIC_AUTH_TOKEN".to_string(),
    serde_json::Value::String(record.api_key.clone()),
  );
  env_obj.remove("ANTHROPIC_API_KEY");
  match record.base_url.as_deref() {
    Some(v) if !v.trim().is_empty() => {
      env_obj.insert(
        "ANTHROPIC_BASE_URL".to_string(),
        serde_json::Value::String(v.to_string()),
      );
    }
    _ => {
      env_obj.remove("ANTHROPIC_BASE_URL");
    }
  }

  let body = serde_json::to_vec_pretty(&root)?;
  if let Err(err) = write_atomic(&settings_path, &body) {
    let _ = restore_file(&settings_path, &settings_before);
    return Err(err);
  }

  let (api_key, base_url) = read_claude_settings_json(&home)?;
  if api_key.as_deref() != Some(record.api_key.as_str()) || base_url != record.base_url {
    let _ = restore_file(&settings_path, &settings_before);
    return Err(AppError::InvalidState(
      "claude settings.json verification failed".to_string(),
    ));
  }
  Ok(())
}

pub fn backup_config_for_tool(tool: ToolType) -> Result<BackupResult, AppError> {
  let home = user_home()?;
  match tool {
    ToolType::Codex | ToolType::CodexApp => {
      copy_if_exists(tool, home.join(".codex").join("auth.json"))
    }
    ToolType::ClaudeCode => Ok(BackupResult {
      tool,
      backup_path: None,
      success: true,
      message: "claude-code uses env vars, no config file backup".to_string(),
    }),
    ToolType::GeminiCli => Ok(BackupResult {
      tool,
      backup_path: None,
      success: true,
      message: "gemini-cli uses env vars, no config file backup".to_string(),
    }),
  }
}

#[allow(dead_code)]
pub fn switch_key_for_record(record: &KeyRecord) -> Result<SwitchResult, AppError> {
  switch_key_for_record_with_source(record, "env")
}

pub fn switch_key_for_record_with_source(
  record: &KeyRecord,
  current_source: &str,
) -> Result<SwitchResult, AppError> {
  match record.tool {
    ToolType::ClaudeCode => {
      let source = current_source.to_ascii_lowercase();
      if source.ends_with(".claude\\settings.json") || source.ends_with(".claude/settings.json") {
        write_claude_settings_json(record)?;
      } else {
        switch_env_with_rollback(&[
          ("ANTHROPIC_AUTH_TOKEN", Some(record.api_key.as_str())),
          ("ANTHROPIC_BASE_URL", record.base_url.as_deref()),
        ])?;
      }
      Ok(SwitchResult {
        success: true,
        warning: None,
        requires_restart: true,
        message: "claude-code key switched".to_string(),
      })
    }
    ToolType::GeminiCli => {
      switch_env_with_rollback(&[
        ("GEMINI_API_KEY", Some(record.api_key.as_str())),
        ("GOOGLE_GEMINI_BASE_URL", record.base_url.as_deref()),
        ("GEMINI_MODEL", record.model.as_deref()),
      ])?;
      Ok(SwitchResult {
        success: true,
        warning: None,
        requires_restart: true,
        message: "gemini-cli key switched".to_string(),
      })
    }
    ToolType::Codex | ToolType::CodexApp => {
      let home = user_home()?;
      let codex_dir = home.join(".codex");
      fs::create_dir_all(&codex_dir)?;

      let auth_path = codex_dir.join("auth.json");
      let config_path = codex_dir.join("config.toml");
      let auth_before = if auth_path.exists() {
        Some(fs::read(&auth_path)?)
      } else {
        None
      };
      let config_before = if config_path.exists() {
        Some(fs::read(&config_path)?)
      } else {
        None
      };
      if auth_path.exists() {
        rotate_backups_and_copy_current(&auth_path)?;
      }
      if config_path.exists() {
        rotate_backups_and_copy_current(&config_path)?;
      }

      let auth_existing = auth_before
        .as_deref()
        .map(std::str::from_utf8)
        .transpose()
        .map_err(|e| AppError::InvalidState(format!("codex auth.json is not utf-8: {e}")))?;
      let config_existing = config_before
        .as_deref()
        .map(std::str::from_utf8)
        .transpose()
        .map_err(|e| AppError::InvalidState(format!("codex config.toml is not utf-8: {e}")))?;
      let auth_body = update_codex_auth_json(auth_existing, record)?;
      let config_body = update_codex_config_toml(config_existing, record);

      let write_result = (|| -> Result<(), AppError> {
        write_atomic(&auth_path, &auth_body)?;
        write_atomic(&config_path, config_body.as_bytes())?;
        Ok(())
      })();

      if let Err(err) = write_result {
        let _ = restore_file(&config_path, &config_before);
        let _ = restore_file(&auth_path, &auth_before);
        return Err(err);
      }

      let auth_actual = fs::read_to_string(&auth_path)?;
      let config_actual = fs::read_to_string(&config_path)?;
      let auth_ok = auth_actual.contains("OPENAI_API_KEY") && auth_actual.contains(&record.api_key);
      let provider = codex_provider_name(record);
      let config_ok = parse_toml_string_value(&config_actual, "model_provider").as_deref()
        == Some(provider.as_str())
        && parse_toml_string_value_in_section(
          &config_actual,
          &format!("model_providers.{provider}"),
          "wire_api",
        )
        .as_deref()
          == Some("responses");
      if !auth_ok || !config_ok {
        let _ = restore_file(&config_path, &config_before);
        let _ = restore_file(&auth_path, &auth_before);
        return Err(AppError::InvalidState(
          "codex config verification failed".to_string(),
        ));
      }

      Ok(SwitchResult {
        success: true,
        warning: None,
        requires_restart: true,
        message: "codex key switched".to_string(),
      })
    }
  }
}

pub fn read_current_tool_config(tool: ToolType) -> Result<ToolCurrentConfig, AppError> {
  let snapshots = detect_tool_auth_methods(tool)?;
  Ok(to_current_config(tool, &snapshots))
}

pub fn detect_tool_auth_methods(tool: ToolType) -> Result<Vec<ToolAuthSnapshot>, AppError> {
  let home = user_home()?;
  match tool {
    ToolType::ClaudeCode => {
      let process_api_key = std::env::var("ANTHROPIC_AUTH_TOKEN")
        .ok()
        .filter(|x| !x.trim().is_empty())
        .or_else(|| {
          std::env::var("ANTHROPIC_API_KEY")
            .ok()
            .filter(|x| !x.trim().is_empty())
        });
      let process_base_url = std::env::var("ANTHROPIC_BASE_URL")
        .ok()
        .filter(|x| !x.trim().is_empty());
      let (settings_api_key, settings_base_url) = read_claude_settings_json(&home)?;
      #[cfg(target_os = "windows")]
      let user_api_key = read_registry_env_var("HKCU\\Environment", "ANTHROPIC_AUTH_TOKEN")?.or(
        read_registry_env_var("HKCU\\Environment", "ANTHROPIC_API_KEY")?,
      );
      #[cfg(target_os = "windows")]
      let user_base_url = read_registry_env_var("HKCU\\Environment", "ANTHROPIC_BASE_URL")?;
      #[cfg(not(target_os = "windows"))]
      let user_api_key = read_user_env_var("ANTHROPIC_AUTH_TOKEN")?;
      #[cfg(not(target_os = "windows"))]
      let user_base_url = read_user_env_var("ANTHROPIC_BASE_URL")?;

      #[cfg(target_os = "windows")]
      let machine_api_key = read_registry_env_var(
        "HKLM\\SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment",
        "ANTHROPIC_AUTH_TOKEN",
      )?
      .or(read_registry_env_var(
        "HKLM\\SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment",
        "ANTHROPIC_API_KEY",
      )?);
      #[cfg(target_os = "windows")]
      let machine_base_url = read_registry_env_var(
        "HKLM\\SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment",
        "ANTHROPIC_BASE_URL",
      )?;
      #[cfg(not(target_os = "windows"))]
      let machine_api_key = None;
      #[cfg(not(target_os = "windows"))]
      let machine_base_url = None;

      let snapshots = vec![
        ToolAuthSnapshot {
          tool,
          method: AuthMethodType::EnvProcess,
          source: "env".to_string(),
          api_key: process_api_key,
          base_url: process_base_url,
          model: None,
          writable: false,
          is_effective: false,
          priority: 1,
        },
        ToolAuthSnapshot {
          tool,
          method: AuthMethodType::SettingsJson,
          source: home
            .join(".claude")
            .join("settings.json")
            .display()
            .to_string(),
          api_key: settings_api_key,
          base_url: settings_base_url,
          model: None,
          writable: true,
          is_effective: false,
          priority: 2,
        },
        ToolAuthSnapshot {
          tool,
          method: AuthMethodType::EnvUser,
          source: if cfg!(target_os = "windows") {
            "HKCU\\Environment".to_string()
          } else {
            "user_env".to_string()
          },
          api_key: user_api_key,
          base_url: user_base_url,
          model: None,
          writable: true,
          is_effective: false,
          priority: 3,
        },
        ToolAuthSnapshot {
          tool,
          method: AuthMethodType::EnvMachine,
          source: if cfg!(target_os = "windows") {
            "HKLM\\SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment".to_string()
          } else {
            "machine_env".to_string()
          },
          api_key: machine_api_key,
          base_url: machine_base_url,
          model: None,
          writable: false,
          is_effective: false,
          priority: 4,
        },
      ];
      Ok(resolve_effective_snapshot(snapshots))
    }
    ToolType::GeminiCli => Ok(ToolCurrentConfig {
      tool,
      api_key: read_effective_env_var("GEMINI_API_KEY")?,
      base_url: read_effective_env_var("GOOGLE_GEMINI_BASE_URL")?,
      model: read_effective_env_var("GEMINI_MODEL")?,
      source: "env".to_string(),
    })
    .map(|cfg| {
      vec![ToolAuthSnapshot {
        tool,
        method: AuthMethodType::EnvUser,
        source: cfg.source.clone(),
        api_key: cfg.api_key,
        base_url: cfg.base_url,
        model: cfg.model,
        writable: true,
        is_effective: true,
        priority: 1,
      }]
    }),
    ToolType::Codex | ToolType::CodexApp => {
      let codex_dir = home.join(".codex");
      let auth_path = codex_dir.join("auth.json");
      let config_path = codex_dir.join("config.toml");

      let api_key = if auth_path.exists() {
        let content = fs::read_to_string(&auth_path)?;
        let json: serde_json::Value = serde_json::from_str(&content)?;
        json
          .get("OPENAI_API_KEY")
          .and_then(|v| v.as_str())
          .map(|v| v.to_string())
      } else {
        None
      };

      let (base_url, model) = read_codex_config_values(&config_path)?;

      Ok(vec![ToolAuthSnapshot {
        tool,
        method: AuthMethodType::AuthJson,
        source: format!("{} + {}", auth_path.display(), config_path.display()),
        api_key,
        base_url,
        model,
        writable: true,
        is_effective: true,
        priority: 1,
      }])
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
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
    std::env::temp_dir().join(format!("keypilot-adapter-test-{nanos}"))
  }

  #[test]
  fn switch_codex_writes_auth_json() {
    let _guard = env_lock().lock().expect("lock poisoned");
    let test_home = unique_temp_home();
    fs::create_dir_all(&test_home).expect("failed to create temp dir");
    std::env::set_var("HOME", &test_home);
    std::env::set_var("USERPROFILE", &test_home);

    let record = KeyRecord {
      id: "c1".to_string(),
      name: "codex-main".to_string(),
      tool: ToolType::Codex,
      api_key: "sk-codex-test".to_string(),
      base_url: None,
      model: None,
      is_active: true,
      created_at: "2026-01-01T00:00:00Z".to_string(),
      updated_at: None,
      note: Some("gac".to_string()),
    };

    let result = switch_key_for_record(&record).expect("switch failed");
    assert!(result.success);
    assert!(result.requires_restart);

    let auth_path = test_home.join(".codex").join("auth.json");
    let config_path = test_home.join(".codex").join("config.toml");
    assert!(auth_path.exists(), "auth.json should exist");
    assert!(config_path.exists(), "config.toml should exist");
    let content = fs::read_to_string(auth_path).expect("read auth.json failed");
    let config_content = fs::read_to_string(config_path).expect("read config.toml failed");
    assert!(content.contains("OPENAI_API_KEY"));
    assert!(content.contains("sk-codex-test"));
    assert!(config_content.contains("model_provider = \"gac\""));
    assert!(config_content.contains("[model_providers.gac]"));
    assert!(config_content.contains("wire_api = \"responses\""));
  }

  #[test]
  fn switch_codex_creates_backup_files_when_existing() {
    let _guard = env_lock().lock().expect("lock poisoned");
    let test_home = unique_temp_home();
    fs::create_dir_all(test_home.join(".codex")).expect("failed to create codex dir");
    std::env::set_var("HOME", &test_home);
    std::env::set_var("USERPROFILE", &test_home);

    let auth_path = test_home.join(".codex").join("auth.json");
    let config_path = test_home.join(".codex").join("config.toml");
    fs::write(&auth_path, r#"{"OPENAI_API_KEY":"old"}"#).expect("seed auth failed");
    fs::write(
      &config_path,
      "model_provider = \"old\"\nmodel = \"old-model\"\n",
    )
    .expect("seed config failed");

    let record = KeyRecord {
      id: "c3".to_string(),
      name: "codex-next".to_string(),
      tool: ToolType::Codex,
      api_key: "sk-codex-new".to_string(),
      base_url: Some("https://api.openai.com/v1".to_string()),
      model: Some("gpt-5".to_string()),
      is_active: true,
      created_at: "2026-01-01T00:00:00Z".to_string(),
      updated_at: None,
      note: Some("gac".to_string()),
    };

    let result = switch_key_for_record(&record).expect("switch failed");
    assert!(result.success);

    let auth_bak = test_home.join(".codex").join("auth.json.bak1");
    let config_bak = test_home.join(".codex").join("config.toml.bak1");
    assert!(auth_bak.exists(), "auth backup should exist");
    assert!(config_bak.exists(), "config backup should exist");

    let auth_new = fs::read_to_string(auth_path).expect("read new auth failed");
    let config_new = fs::read_to_string(config_path).expect("read new config failed");
    assert!(auth_new.contains("sk-codex-new"));
    assert!(config_new.contains("gpt-5"));
    assert!(config_new.contains("model_provider = \"gac\""));
    assert!(config_new.contains("[model_providers.gac]"));
  }

  #[test]
  fn switch_codex_preserves_unrelated_auth_and_config_values() {
    let _guard = env_lock().lock().expect("lock poisoned");
    let test_home = unique_temp_home();
    fs::create_dir_all(test_home.join(".codex")).expect("failed to create codex dir");
    std::env::set_var("HOME", &test_home);
    std::env::set_var("USERPROFILE", &test_home);

    let auth_path = test_home.join(".codex").join("auth.json");
    let config_path = test_home.join(".codex").join("config.toml");
    fs::write(
      &auth_path,
      r#"{
  "OPENAI_API_KEY": "old",
  "tokens": {
    "refresh": "keep-me"
  }
}"#,
    )
    .expect("seed auth failed");
    fs::write(
      &config_path,
      r#"model_provider = "old"
model = "old-model"
model_reasoning_effort = "medium"
disable_response_storage = true

[marketplaces.cc-speak]
source_type = "git"
source = "https://github.com/PeterCang/cc-speak.git"

[plugins."browser-use@openai-bundled"]
enabled = true

[model_providers.old]
name = "old"
base_url = "https://old.example/v1"
wire_api = "responses"
"#,
    )
    .expect("seed config failed");

    let record = KeyRecord {
      id: "c4".to_string(),
      name: "codex-next".to_string(),
      tool: ToolType::Codex,
      api_key: "sk-codex-new".to_string(),
      base_url: Some("https://gaccode.com/codex/v1".to_string()),
      model: Some("gpt-5.3-codex".to_string()),
      is_active: true,
      created_at: "2026-01-01T00:00:00Z".to_string(),
      updated_at: None,
      note: Some("gac".to_string()),
    };

    let result = switch_key_for_record(&record).expect("switch failed");
    assert!(result.success);

    let auth_new: serde_json::Value =
      serde_json::from_str(&fs::read_to_string(auth_path).expect("read auth failed"))
        .expect("parse auth failed");
    assert_eq!(auth_new["OPENAI_API_KEY"].as_str(), Some("sk-codex-new"));
    assert_eq!(auth_new["tokens"]["refresh"].as_str(), Some("keep-me"));

    let config_new = fs::read_to_string(config_path).expect("read config failed");
    assert!(config_new.contains("model_provider = \"gac\""));
    assert!(config_new.contains("model = \"gpt-5.3-codex\""));
    assert!(config_new.contains("model_reasoning_effort = \"medium\""));
    assert!(config_new.contains("disable_response_storage = true"));
    assert!(config_new.contains("[marketplaces.cc-speak]"));
    assert!(config_new.contains("[plugins.\"browser-use@openai-bundled\"]"));
    assert!(config_new.contains("[model_providers.old]"));
    assert!(config_new.contains("[model_providers.gac]"));
    assert!(config_new.contains("base_url = \"https://gaccode.com/codex/v1\""));
    assert!(config_new.contains("wire_api = \"responses\""));
  }

  #[test]
  fn switch_claude_settings_json_creates_backup_and_preserves_other_values() {
    let _guard = env_lock().lock().expect("lock poisoned");
    let test_home = unique_temp_home();
    let claude_dir = test_home.join(".claude");
    fs::create_dir_all(&claude_dir).expect("failed to create claude dir");
    std::env::set_var("HOME", &test_home);
    std::env::set_var("USERPROFILE", &test_home);

    let settings_path = claude_dir.join("settings.json");
    fs::write(
      &settings_path,
      r#"{
  "env": {
    "ANTHROPIC_AUTH_TOKEN": "old",
    "ANTHROPIC_BASE_URL": "https://old.example"
  },
  "permissions": {
    "allow": ["Bash(ls *)"]
  }
}"#,
    )
    .expect("seed settings failed");

    let record = KeyRecord {
      id: "c5".to_string(),
      name: "claude-next".to_string(),
      tool: ToolType::ClaudeCode,
      api_key: "sk-claude-new".to_string(),
      base_url: Some("https://api.anthropic.com".to_string()),
      model: None,
      is_active: true,
      created_at: "2026-01-01T00:00:00Z".to_string(),
      updated_at: None,
      note: None,
    };

    let source = settings_path.display().to_string();
    let result = switch_key_for_record_with_source(&record, &source).expect("switch failed");
    assert!(result.success);

    let backup_path = claude_dir.join("settings.json.bak1");
    assert!(backup_path.exists(), "settings backup should exist");
    let backup = fs::read_to_string(backup_path).expect("read backup failed");
    assert!(backup.contains("\"ANTHROPIC_AUTH_TOKEN\": \"old\""));
    assert!(backup.contains("\"permissions\""));

    let next: serde_json::Value =
      serde_json::from_str(&fs::read_to_string(settings_path).expect("read settings failed"))
        .expect("parse settings failed");
    assert_eq!(
      next["env"]["ANTHROPIC_AUTH_TOKEN"].as_str(),
      Some("sk-claude-new")
    );
    assert_eq!(
      next["env"]["ANTHROPIC_BASE_URL"].as_str(),
      Some("https://api.anthropic.com")
    );
    assert_eq!(next["permissions"]["allow"][0].as_str(), Some("Bash(ls *)"));
  }

  #[test]
  fn rotated_backups_keep_latest_five_versions() {
    let test_home = unique_temp_home();
    fs::create_dir_all(&test_home).expect("failed to create temp dir");
    let target = test_home.join("config.toml");

    for index in 1..=6 {
      fs::write(&target, format!("version-{index}")).expect("write target failed");
      rotate_backups_and_copy_current(&target).expect("backup failed");
    }

    assert_eq!(
      fs::read_to_string(test_home.join("config.toml.bak1")).expect("read bak1 failed"),
      "version-6"
    );
    assert_eq!(
      fs::read_to_string(test_home.join("config.toml.bak2")).expect("read bak2 failed"),
      "version-5"
    );
    assert_eq!(
      fs::read_to_string(test_home.join("config.toml.bak3")).expect("read bak3 failed"),
      "version-4"
    );
    assert_eq!(
      fs::read_to_string(test_home.join("config.toml.bak4")).expect("read bak4 failed"),
      "version-3"
    );
    assert_eq!(
      fs::read_to_string(test_home.join("config.toml.bak5")).expect("read bak5 failed"),
      "version-2"
    );
    assert!(
      !test_home.join("config.toml.bak6").exists(),
      "only five backups should be retained"
    );
  }

  #[test]
  fn detect_codex_reads_model_provider_base_url() {
    let _guard = env_lock().lock().expect("lock poisoned");
    let test_home = unique_temp_home();
    fs::create_dir_all(test_home.join(".codex")).expect("failed to create codex dir");
    std::env::set_var("HOME", &test_home);
    std::env::set_var("USERPROFILE", &test_home);

    let auth_path = test_home.join(".codex").join("auth.json");
    let config_path = test_home.join(".codex").join("config.toml");
    fs::write(&auth_path, r#"{"OPENAI_API_KEY":"sk-codex-provider"}"#).expect("seed auth failed");
    fs::write(
      &config_path,
      r#"
model_provider = "aipor"
model = "gpt-5"
model_reasoning_effort = "high"
disable_response_storage = true

[model_providers.aipor]
name = "aipor"
base_url = "https://code.aipor.cc"
wire_api = "responses"
requires_openai_auth = true
"#,
    )
    .expect("seed config failed");

    let cfg = read_current_tool_config(ToolType::Codex).expect("read codex config failed");
    assert_eq!(cfg.api_key.as_deref(), Some("sk-codex-provider"));
    assert_eq!(cfg.model.as_deref(), Some("gpt-5"));
    assert_eq!(cfg.base_url.as_deref(), Some("https://code.aipor.cc"));
  }

  #[test]
  #[cfg(not(any(target_os = "windows", target_os = "macos")))]
  fn claude_switch_fails_on_unsupported_platform() {
    let _guard = env_lock().lock().expect("lock poisoned");
    let record = KeyRecord {
      id: "c2".to_string(),
      name: "claude-main".to_string(),
      tool: ToolType::ClaudeCode,
      api_key: "sk-claude-test".to_string(),
      base_url: Some("https://api.anthropic.com".to_string()),
      model: None,
      is_active: true,
      created_at: "2026-01-01T00:00:00Z".to_string(),
      updated_at: None,
      note: None,
    };
    let err = switch_key_for_record(&record).expect_err("should fail");
    assert!(err.to_string().contains("not supported"));
  }
}
