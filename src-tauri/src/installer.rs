use crate::error::AppError;
use crate::models::ToolType;
use crate::process::detect_tool;
use std::process::Command;

fn install_plan(tool: ToolType) -> Option<(&'static str, &'static [&'static str])> {
  match tool {
    ToolType::Codex => Some(("npm", &["install", "-g", "@openai/codex"])),
    ToolType::ClaudeCode => Some(("npm", &["install", "-g", "@anthropic-ai/claude-code"])),
    ToolType::GeminiCli => None,
  }
}

pub fn install_tool(tool: ToolType) -> Result<String, AppError> {
  let status = detect_tool(tool);
  if status.installed {
    return Ok(format!(
      "tool already installed at {}",
      status.location.unwrap_or_else(|| "unknown location".to_string())
    ));
  }

  let Some((command, args)) = install_plan(tool) else {
    return Ok("installer placeholder: gemini-cli install workflow will be added in next iteration".to_string());
  };

  let output = Command::new(command).args(args).output()?;
  if output.status.success() {
    return Ok(format!("install succeeded: {} {}", command, args.join(" ")));
  }

  let stderr = String::from_utf8_lossy(&output.stderr);
  let stdout = String::from_utf8_lossy(&output.stdout);
  Err(AppError::InvalidState(format!(
    "install failed: {} {} | stdout: {} | stderr: {}",
    command,
    args.join(" "),
    stdout.trim(),
    stderr.trim()
  )))
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
  fn gemini_plan_is_placeholder() {
    let plan = install_plan(ToolType::GeminiCli);
    assert!(plan.is_none());
  }
}
