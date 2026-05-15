use crate::models::{ToolStatus, ToolType};
use std::process::Command;

fn tool_bin(tool: ToolType) -> &'static str {
  match tool {
    ToolType::ClaudeCode => "claude",
    ToolType::Codex => "codex",
    ToolType::GeminiCli => "gemini",
  }
}

pub fn is_tool_running(tool: ToolType) -> bool {
  let name = tool_bin(tool);
  #[cfg(target_os = "windows")]
  {
    let filter = format!("IMAGENAME eq {name}.exe");
    if let Ok(output) = Command::new("tasklist").args(["/FI", &filter]).output() {
      let text = String::from_utf8_lossy(&output.stdout).to_lowercase();
      return text.contains(&format!("{name}.exe"));
    }
    false
  }
  #[cfg(not(target_os = "windows"))]
  {
    if let Ok(status) = Command::new("pgrep").args(["-f", name]).status() {
      return status.success();
    }
    false
  }
}

pub fn restart_tool(tool: ToolType) -> Result<String, String> {
  let name = tool_bin(tool);
  if !is_tool_running(tool) {
    return Ok(format!("{name} is not running"));
  }

  #[cfg(target_os = "windows")]
  {
    let image = format!("{name}.exe");
    let status = Command::new("taskkill")
      .args(["/IM", &image, "/F"])
      .status()
      .map_err(|e| format!("taskkill failed: {e}"))?;
    if !status.success() {
      return Err(format!("failed to stop {image}"));
    }
  }

  #[cfg(not(target_os = "windows"))]
  {
    let status = Command::new("pkill")
      .args(["-f", name])
      .status()
      .map_err(|e| format!("pkill failed: {e}"))?;
    if !status.success() {
      return Err(format!("failed to stop {name}"));
    }
  }

  Ok(format!("{name} stopped; reopen the tool to complete restart"))
}

pub fn detect_tool(tool: ToolType) -> ToolStatus {
  let bin = tool_bin(tool);

  match which::which(bin) {
    Ok(path) => ToolStatus {
      tool,
      installed: true,
      version: None,
      location: Some(path.display().to_string()),
    },
    Err(_) => ToolStatus {
      tool,
      installed: false,
      version: None,
      location: None,
    },
  }
}
