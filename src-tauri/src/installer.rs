use crate::error::AppError;
use crate::models::ToolType;
use crate::process::detect_tool;
use shlex;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::thread;
use tauri::AppHandle;
use tauri::Emitter;

const GEMINI_MIN_NODE_MAJOR: u32 = 20;

fn install_plan(tool: ToolType) -> Option<(&'static str, &'static [&'static str])> {
  match tool {
    ToolType::Codex => Some((npm_command(), &["install", "-g", "@openai/codex"])),
    ToolType::CodexApp => Some((npm_command(), &["install", "-g", "@openai/codex"])),
    ToolType::ClaudeCode => Some((npm_command(), &["install", "-g", "@anthropic-ai/claude-code"])),
    ToolType::GeminiCli => Some((npm_command(), &["install", "-g", "@google/gemini-cli"])),
  }
}

fn uninstall_plan(tool: ToolType) -> (&'static str, &'static [&'static str]) {
  match tool {
    ToolType::Codex => (npm_command(), &["uninstall", "-g", "@openai/codex"]),
    ToolType::CodexApp => (npm_command(), &["uninstall", "-g", "@openai/codex"]),
    ToolType::ClaudeCode => (npm_command(), &["uninstall", "-g", "@anthropic-ai/claude-code"]),
    ToolType::GeminiCli => (npm_command(), &["uninstall", "-g", "@google/gemini-cli"]),
  }
}

#[cfg(target_os = "windows")]
fn npm_command() -> &'static str {
  "npm.cmd"
}

#[cfg(not(target_os = "windows"))]
fn npm_command() -> &'static str {
  "npm"
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

fn command_output(command: &str, args: &[&str]) -> Result<String, AppError> {
  let output = Command::new(command).args(args).output()?;
  if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let suffix = if stderr.is_empty() { String::new() } else { format!(": {stderr}") };
    return Err(AppError::InvalidState(format!(
      "{} {} failed{}",
      command,
      args.join(" "),
      suffix
    )));
  }
  Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn parse_node_major(version: &str) -> Option<u32> {
  version.trim().trim_start_matches('v').split('.').next()?.parse::<u32>().ok()
}

fn ensure_gemini_prerequisites(app: &AppHandle) -> Result<(), AppError> {
  emit_log(app, "checking Node.js version");
  let node_version = command_output("node", &["--version"]).map_err(|e| {
    AppError::InvalidState(format!(
      "Gemini CLI requires Node.js {GEMINI_MIN_NODE_MAJOR}+ and npm. Node.js check failed: {e}"
    ))
  })?;
  let major = parse_node_major(&node_version).ok_or_else(|| {
    AppError::InvalidState(format!("unable to parse Node.js version: {node_version}"))
  })?;
  emit_log(app, format!("node: {node_version}"));
  if major < GEMINI_MIN_NODE_MAJOR {
    return Err(AppError::InvalidState(format!(
      "Gemini CLI requires Node.js {GEMINI_MIN_NODE_MAJOR}+, current version is {node_version}"
    )));
  }

  emit_log(app, "checking npm version");
  let npm_version = command_output(npm_command(), &["--version"]).map_err(|e| {
    AppError::InvalidState(format!("Gemini CLI requires npm. npm check failed: {e}"))
  })?;
  emit_log(app, format!("npm: {npm_version}"));
  Ok(())
}

fn ensure_install_prerequisites(app: &AppHandle, tool: ToolType) -> Result<(), AppError> {
  match tool {
    ToolType::GeminiCli => ensure_gemini_prerequisites(app),
    ToolType::ClaudeCode | ToolType::Codex | ToolType::CodexApp => {
      let version = command_output(npm_command(), &["--version"])?;
      emit_log(app, format!("npm: {version}"));
      Ok(())
    }
  }
}

fn pipe_reader(app: AppHandle, label: &'static str, stream: impl std::io::Read + Send + 'static) -> thread::JoinHandle<()> {
  thread::spawn(move || {
    let reader = BufReader::new(stream);
    for line in reader.lines() {
      emit_log(&app, format!("{label}: {}", line.unwrap_or_default()));
    }
  })
}

fn run_install_command(app: &AppHandle, command: &str, args: &[&str]) -> Result<(), AppError> {
  emit_log(app, format!("start: {} {}", command, args.join(" ")));
  let mut child = Command::new(command)
    .args(args)
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()?;

  let stdout_handle = child.stdout.take().map(|stdout| pipe_reader(app.clone(), "stdout", stdout));
  let stderr_handle = child.stderr.take().map(|stderr| pipe_reader(app.clone(), "stderr", stderr));
  let status = child.wait()?;

  if let Some(handle) = stdout_handle {
    let _ = handle.join();
  }
  if let Some(handle) = stderr_handle {
    let _ = handle.join();
  }

  if status.success() {
    emit_log(app, "install command succeeded");
    Ok(())
  } else {
    emit_log(app, format!("install command failed with status: {status}"));
    Err(AppError::InvalidState(format!(
      "install failed: {} {}",
      command,
      args.join(" ")
    )))
  }
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

  ensure_install_prerequisites(app, tool)?;
  run_install_command(app, command, args)?;

  let refreshed = detect_tool(tool);
  if !refreshed.installed {
    return Err(AppError::InvalidState(format!(
      "install command completed, but {} was not found on PATH",
      bin_name(tool)
    )));
  }

  let location = refreshed.location.unwrap_or_else(|| "unknown location".to_string());
  let version = refreshed.version.unwrap_or_else(|| "unknown version".to_string());
  emit_log(app, format!("install verified: {version} at {location}"));
  Ok(format!("install succeeded: {version} at {location}"))
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

pub fn start_tool(tool: ToolType, args: &str, project_dir: Option<&str>) -> Result<String, AppError> {
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

  let launch_dir = {
    let dir = project_dir
      .map(str::trim)
      .filter(|x| !x.is_empty())
      .ok_or_else(|| AppError::InvalidState("project directory is required".to_string()))?;
    let path = std::path::PathBuf::from(dir);
    if !path.is_dir() {
      return Err(AppError::InvalidState(format!("project directory not found: {dir}")));
    }
    path
  };

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
        .current_dir(&launch_dir)
        .env("PROJECT_DIRECTORY", &launch_dir)
        .env("PROJECT_DIRECTOR", &launch_dir)
        .spawn()?;
      return Ok(format!("{target} started in a new terminal window"));
    }
  }

  let mut cmd = Command::new(&target);
  cmd.current_dir(&launch_dir);
  cmd.env("PROJECT_DIRECTORY", &launch_dir);
  cmd.env("PROJECT_DIRECTOR", &launch_dir);
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

  #[test]
  fn parses_node_major() {
    assert_eq!(parse_node_major("v20.11.1"), Some(20));
    assert_eq!(parse_node_major("22.0.0"), Some(22));
    assert_eq!(parse_node_major("not-a-version"), None);
  }
}
