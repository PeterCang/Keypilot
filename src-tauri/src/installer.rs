use crate::error::AppError;
use crate::models::ToolType;
use crate::process::detect_tool;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use tauri::AppHandle;
use tauri::Emitter;

fn install_plan(tool: ToolType) -> Option<(&'static str, &'static [&'static str])> {
  match tool {
    ToolType::Codex => Some(("npm", &["install", "-g", "@openai/codex"])),
    ToolType::ClaudeCode => Some(("npm", &["install", "-g", "@anthropic-ai/claude-code"])),
    ToolType::GeminiCli => Some(("npm", &["install", "-g", "@google/gemini-cli"])),
  }
}

fn emit_log(app: &AppHandle, line: impl Into<String>) {
  let _ = app.emit("install-log", line.into());
}

pub fn install_tool(app: &AppHandle, tool: ToolType) -> Result<String, AppError> {
  let status = detect_tool(tool);
  if status.installed {
    emit_log(
      app,
      format!(
        "tool already installed at {}",
        status.location.clone().unwrap_or_else(|| "unknown location".to_string())
      ),
    );
    return Ok(format!(
      "tool already installed at {}",
      status.location.unwrap_or_else(|| "unknown location".to_string())
    ));
  }

  let Some((command, args)) = install_plan(tool) else {
    return Err(AppError::InvalidState("missing install plan".to_string()));
  };
  emit_log(app, format!("start: {} {}", command, args.join(" ")));

  let mut child = Command::new(command)
    .args(args)
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()?;

  if let Some(stdout) = child.stdout.take() {
    let reader = BufReader::new(stdout);
    for line in reader.lines() {
      emit_log(app, format!("stdout: {}", line.unwrap_or_default()));
    }
  }
  if let Some(stderr) = child.stderr.take() {
    let reader = BufReader::new(stderr);
    for line in reader.lines() {
      emit_log(app, format!("stderr: {}", line.unwrap_or_default()));
    }
  }
  let status = child.wait()?;
  if status.success() {
    emit_log(app, "install succeeded");
    Ok(format!("install succeeded: {} {}", command, args.join(" ")))
  } else {
    emit_log(app, "install failed");
    Err(AppError::InvalidState(format!(
      "install failed: {} {}",
      command,
      args.join(" ")
    )))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn codex_install_plan_exists() {
    let plan = install_plan(ToolType::Codex);
    assert!(plan.is_some());
  }

  #[test]
  fn gemini_plan_exists() {
    let plan = install_plan(ToolType::GeminiCli);
    assert!(plan.is_some());
  }
}
