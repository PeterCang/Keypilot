use crate::error::AppError;
use crate::models::ToolType;
use crate::process::detect_tool;
use shlex;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use tauri::AppHandle;
use tauri::Emitter;

fn install_plan(tool: ToolType) -> Option<(&'static str, &'static [&'static str])> {
  match tool {
    ToolType::Codex => Some(("npm", &["install", "-g", "@openai/codex"])),
    ToolType::CodexApp => Some(("npm", &["install", "-g", "@openai/codex"])),
    ToolType::ClaudeCode => Some(("npm", &["install", "-g", "@anthropic-ai/claude-code"])),
    ToolType::GeminiCli => Some(("npm", &["install", "-g", "@google/gemini-cli"])),
  }
}

fn uninstall_plan(tool: ToolType) -> (&'static str, &'static [&'static str]) {
  match tool {
    ToolType::Codex => ("npm", &["uninstall", "-g", "@openai/codex"]),
    ToolType::CodexApp => ("npm", &["uninstall", "-g", "@openai/codex"]),
    ToolType::ClaudeCode => ("npm", &["uninstall", "-g", "@anthropic-ai/claude-code"]),
    ToolType::GeminiCli => ("npm", &["uninstall", "-g", "@google/gemini-cli"]),
  }
}

fn bin_name(tool: ToolType) -> &'static str {
  match tool {
    ToolType::ClaudeCode => "claude",
    ToolType::Codex => "codex",
    ToolType::CodexApp => "codex",
    ToolType::GeminiCli => "gemini",
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

pub fn uninstall_tool(app: &AppHandle, tool: ToolType) -> Result<String, AppError> {
  let (command, args) = uninstall_plan(tool);
  emit_log(app, format!("start: {} {}", command, args.join(" ")));
  let output = Command::new(command).args(args).output()?;
  if output.status.success() {
    emit_log(app, "uninstall succeeded");
    Ok(format!("uninstall succeeded: {} {}", command, args.join(" ")))
  } else {
    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(AppError::InvalidState(format!(
      "uninstall failed: {} {} | {}",
      command,
      args.join(" "),
      stderr.trim()
    )))
  }
}

pub fn start_tool(tool: ToolType, args: &str) -> Result<String, AppError> {
  let status = detect_tool(tool);
  let target = status
    .location
    .map(|path| path.trim().trim_matches('"').to_string())
    .filter(|path| !path.trim().is_empty())
    .filter(|path| std::path::Path::new(path).is_file())
    .unwrap_or_else(|| bin_name(tool).to_string());

  let mut parsed_args = shlex::split(args)
    .ok_or_else(|| AppError::InvalidState("launch args parse failed".to_string()))?;
  if matches!(tool, ToolType::CodexApp) {
    let has_app_subcommand = parsed_args.first().map(|x| x == "app").unwrap_or(false);
    if !has_app_subcommand {
      parsed_args.insert(0, "app".to_string());
    }
  }

  #[cfg(target_os = "windows")]
  {
    if !matches!(tool, ToolType::CodexApp) {
      let run_line = if args.trim().is_empty() {
        target.clone()
      } else {
        format!("{target} {args}")
      };
      Command::new("cmd")
        .args(["/C", "start", "", "cmd", "/K", &run_line])
        .spawn()?;
      return Ok(format!("{target} started in a new terminal window"));
    }
  }

  let mut cmd = Command::new(&target);
  if !parsed_args.is_empty() {
    cmd.args(parsed_args);
  }
  cmd.spawn()?;
  Ok(format!("{target} started"))
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
