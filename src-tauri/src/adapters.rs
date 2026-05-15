use crate::error::AppError;
use crate::models::{BackupResult, KeyRecord, SwitchResult, ToolType};
use std::fs;
use std::path::PathBuf;

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
      std::env::set_var("ANTHROPIC_AUTH_TOKEN", &record.api_key);
      if let Some(url) = &record.base_url {
        std::env::set_var("ANTHROPIC_BASE_URL", url);
      }
      Ok(SwitchResult {
        success: true,
        warning: Some("仅当前进程生效；MVP 下一步将落地系统级持久化写入".to_string()),
        requires_restart: true,
        message: "claude-code key switched".to_string(),
      })
    }
    ToolType::GeminiCli => {
      std::env::set_var("GEMINI_API_KEY", &record.api_key);
      if let Some(url) = &record.base_url {
        std::env::set_var("GOOGLE_GEMINI_BASE_URL", url);
      }
      if let Some(model) = &record.model {
        std::env::set_var("GEMINI_MODEL", model);
      }
      Ok(SwitchResult {
        success: true,
        warning: Some("仅当前进程生效；MVP 下一步将落地系统级持久化写入".to_string()),
        requires_restart: true,
        message: "gemini-cli key switched".to_string(),
      })
    }
    ToolType::Codex => {
      let home = user_home()?;
      let codex_dir = home.join(".codex");
      fs::create_dir_all(&codex_dir)?;

      let auth_path = codex_dir.join("auth.json");
      if auth_path.exists() {
        fs::copy(&auth_path, auth_path.with_extension("json.bak"))?;
      }

      let payload = serde_json::json!({ "OPENAI_API_KEY": record.api_key });
      fs::write(&auth_path, serde_json::to_vec_pretty(&payload)?)?;

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
    assert!(auth_path.exists(), "auth.json should exist");
    let content = fs::read_to_string(auth_path).expect("read auth.json failed");
    assert!(content.contains("OPENAI_API_KEY"));
    assert!(content.contains("sk-codex-test"));
  }
}
