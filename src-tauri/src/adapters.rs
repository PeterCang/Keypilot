use crate::error::AppError;
use crate::models::{BackupResult, KeyRecord, SwitchResult, ToolType};
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

  let backup = source.with_extension("bak");
  fs::copy(&source, &backup)?;
  Ok(BackupResult {
    tool,
    backup_path: Some(backup.display().to_string()),
    success: true,
    message: "backup created".to_string(),
  })
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
    None => run_command(
      "reg",
      &["delete", "HKCU\\Environment", "/v", key, "/f"],
    ),
  }
}

#[cfg(target_os = "windows")]
fn read_user_env_var(key: &str) -> Result<Option<String>, AppError> {
  let output = Command::new("reg")
    .args(["query", "HKCU\\Environment", "/v", key])
    .output()?;
  if !output.status.success() {
    return Ok(None);
  }
  let stdout = String::from_utf8_lossy(&output.stdout);
  for line in stdout.lines() {
    if line.contains("REG_") && line.contains(key) {
      let parts: Vec<&str> = line.split_whitespace().collect();
      if let Some(value) = parts.last() {
        return Ok(Some((*value).to_string()));
      }
    }
  }
  Ok(None)
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

fn set_and_verify_env_var(key: &str, value: Option<&str>) -> Result<(), AppError> {
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

fn switch_env_with_rollback(changes: &[(&str, Option<&str>)]) -> Result<(), AppError> {
  let mut applied: Vec<(&str, Option<String>)> = Vec::new();
  for (key, value) in changes {
    let before = read_user_env_var(key)?;
    if let Err(e) = set_and_verify_env_var(key, *value) {
      for (rollback_key, rollback_value) in applied.iter().rev() {
        let _ = set_and_verify_env_var(rollback_key, rollback_value.as_deref());
      }
      return Err(e);
    }
    applied.push((key, before));
  }
  Ok(())
}

fn toml_escape(value: &str) -> String {
  value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn render_codex_config(record: &KeyRecord) -> String {
  let mut lines = Vec::new();
  lines.push("[api]".to_string());
  lines.push(format!("key = \"{}\"", toml_escape(&record.api_key)));
  if let Some(url) = &record.base_url {
    lines.push(format!("base_url = \"{}\"", toml_escape(url)));
  }
  if let Some(model) = &record.model {
    lines.push(format!("model = \"{}\"", toml_escape(model)));
  }
  lines.push(String::new());
  lines.join("\n")
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

pub fn backup_config_for_tool(tool: ToolType) -> Result<BackupResult, AppError> {
  let home = user_home()?;
  match tool {
    ToolType::Codex => copy_if_exists(tool, home.join(".codex").join("auth.json")),
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

pub fn switch_key_for_record(record: &KeyRecord) -> Result<SwitchResult, AppError> {
  match record.tool {
    ToolType::ClaudeCode => {
      switch_env_with_rollback(&[
        ("ANTHROPIC_AUTH_TOKEN", Some(record.api_key.as_str())),
        ("ANTHROPIC_BASE_URL", record.base_url.as_deref()),
      ])?;
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
    ToolType::Codex => {
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
        fs::copy(&auth_path, auth_path.with_extension("json.bak"))?;
      }
      if config_path.exists() {
        fs::copy(&config_path, config_path.with_extension("toml.bak"))?;
      }

      let payload = serde_json::json!({ "OPENAI_API_KEY": record.api_key });
      let auth_body = serde_json::to_vec_pretty(&payload)?;
      let config_body = render_codex_config(record);

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
      let config_ok = config_actual.contains("[api]") && config_actual.contains(&record.api_key);
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

#[cfg(test)]
mod tests {
  use super::*;
  use std::time::{SystemTime, UNIX_EPOCH};

  fn unique_temp_home() -> PathBuf {
    let nanos = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .expect("time went backwards")
      .as_nanos();
    std::env::temp_dir().join(format!("keypilot-adapter-test-{nanos}"))
  }

  #[test]
  fn switch_codex_writes_auth_json() {
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
      note: None,
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
    assert!(config_content.contains("[api]"));
    assert!(config_content.contains("sk-codex-test"));
  }

  #[test]
  fn switch_codex_creates_backup_files_when_existing() {
    let test_home = unique_temp_home();
    fs::create_dir_all(test_home.join(".codex")).expect("failed to create codex dir");
    std::env::set_var("HOME", &test_home);
    std::env::set_var("USERPROFILE", &test_home);

    let auth_path = test_home.join(".codex").join("auth.json");
    let config_path = test_home.join(".codex").join("config.toml");
    fs::write(&auth_path, r#"{"OPENAI_API_KEY":"old"}"#).expect("seed auth failed");
    fs::write(&config_path, "[api]\nkey = \"old\"\n").expect("seed config failed");

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
      note: None,
    };

    let result = switch_key_for_record(&record).expect("switch failed");
    assert!(result.success);

    let auth_bak = test_home.join(".codex").join("auth.json.bak");
    let config_bak = test_home.join(".codex").join("config.toml.bak");
    assert!(auth_bak.exists(), "auth backup should exist");
    assert!(config_bak.exists(), "config backup should exist");

    let auth_new = fs::read_to_string(auth_path).expect("read new auth failed");
    let config_new = fs::read_to_string(config_path).expect("read new config failed");
    assert!(auth_new.contains("sk-codex-new"));
    assert!(config_new.contains("gpt-5"));
  }

  #[test]
  #[cfg(not(any(target_os = "windows", target_os = "macos")))]
  fn claude_switch_fails_on_unsupported_platform() {
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
