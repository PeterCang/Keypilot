use crate::models::{ToolStatus, ToolType};
use std::process::Command;

fn tool_bin(tool: ToolType) -> &'static str {
  match tool {
    ToolType::ClaudeCode => "claude",
    ToolType::Codex => "codex",
    ToolType::CodexApp => "codex",
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
  if matches!(tool, ToolType::CodexApp) {
    return detect_codex_app();
  }

  let bin = tool_bin(tool);

  match which::which(bin) {
    Ok(path) => ToolStatus {
      tool,
      installed: true,
      version: detect_tool_version(tool),
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

#[cfg(target_os = "windows")]
fn detect_codex_app() -> ToolStatus {
  if let Some((location, version)) = detect_codex_app_from_appx() {
    return ToolStatus {
      tool: ToolType::CodexApp,
      installed: true,
      version,
      location: Some(location),
    };
  }

  if let Some((location, version)) = detect_codex_app_from_registry() {
    return ToolStatus {
      tool: ToolType::CodexApp,
      installed: true,
      version,
      location: Some(location),
    };
  }

  let candidates = codex_app_windows_candidates();
  for exe_path in candidates {
    if exe_path.is_file() {
      let location = exe_path.display().to_string();
      let version = detect_windows_file_version(&location).or_else(|| detect_windows_file_file_version(&location));
      return ToolStatus {
        tool: ToolType::CodexApp,
        installed: true,
        version,
        location: Some(location),
      };
    }
  }

  ToolStatus {
    tool: ToolType::CodexApp,
    installed: false,
    version: None,
    location: None,
  }
}

#[cfg(target_os = "windows")]
fn detect_codex_app_from_appx() -> Option<(String, Option<String>)> {
  let script = r#"
$pkg = Get-AppxPackage -Name 'OpenAI.Codex*' -ErrorAction SilentlyContinue | Select-Object -First 1
if ($pkg -and $pkg.InstallLocation) {
  $exe = Join-Path $pkg.InstallLocation 'Codex.exe'
  if (-not (Test-Path $exe)) {
    $exe = Join-Path $pkg.InstallLocation 'codex.exe'
  }
  if (Test-Path $exe) {
    "$exe|$($pkg.Version)"
  } else {
    "$($pkg.InstallLocation)|$($pkg.Version)"
  }
}
"#;
  let output = Command::new("powershell")
    .args(["-NoProfile", "-Command", script])
    .output()
    .ok()?;
  if !output.status.success() {
    return None;
  }
  let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
  if raw.is_empty() {
    return None;
  }
  let mut parts = raw.splitn(2, '|');
  let location = parts.next()?.trim().trim_matches('"').to_string();
  if location.is_empty() {
    return None;
  }
  let version = parts
    .next()
    .map(|s| s.trim().to_string())
    .filter(|s| !s.is_empty())
    .map(|s| normalize_version_output(&s));
  Some((location, version))
}

#[cfg(target_os = "windows")]
fn detect_codex_app_from_registry() -> Option<(String, Option<String>)> {
  let script = r#"
$paths = @(
  'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*',
  'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*',
  'HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*'
)
$hit = Get-ItemProperty $paths -ErrorAction SilentlyContinue |
  Where-Object { $_.DisplayName -match 'Codex' -and $_.DisplayIcon } |
  Select-Object -First 1
if ($hit) {
  $exe = $hit.DisplayIcon -replace ',\d+$',''
  $ver = $hit.DisplayVersion
  "$exe|$ver"
}
"#;
  let output = Command::new("powershell")
    .args(["-NoProfile", "-Command", script])
    .output()
    .ok()?;
  if !output.status.success() {
    return None;
  }
  let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
  if raw.is_empty() {
    return None;
  }
  let mut parts = raw.splitn(2, '|');
  let location = parts.next()?.trim().trim_matches('"').to_string();
  if location.is_empty() {
    return None;
  }
  let version = parts
    .next()
    .map(|s| s.trim().to_string())
    .filter(|s| !s.is_empty())
    .map(|s| normalize_version_output(&s));
  Some((location, version))
}

#[cfg(not(target_os = "windows"))]
fn detect_codex_app() -> ToolStatus {
  ToolStatus {
    tool: ToolType::CodexApp,
    installed: false,
    version: None,
    location: None,
  }
}

#[cfg(target_os = "windows")]
fn codex_app_windows_candidates() -> Vec<std::path::PathBuf> {
  let mut paths = Vec::new();
  if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
    paths.push(
      std::path::PathBuf::from(&local_app_data)
        .join("Programs")
        .join("Codex")
        .join("Codex.exe"),
    );
    paths.push(
      std::path::PathBuf::from(&local_app_data)
        .join("codex")
        .join("Codex.exe"),
    );
  }
  if let Ok(program_files) = std::env::var("ProgramFiles") {
    paths.push(
      std::path::PathBuf::from(&program_files)
        .join("Codex")
        .join("Codex.exe"),
    );
  }
  if let Ok(program_files_x86) = std::env::var("ProgramFiles(x86)") {
    paths.push(
      std::path::PathBuf::from(&program_files_x86)
        .join("Codex")
        .join("Codex.exe"),
    );
  }
  paths
}

#[cfg(target_os = "windows")]
fn detect_windows_file_version(exe_path: &str) -> Option<String> {
  let script = format!("(Get-Item -LiteralPath '{}').VersionInfo.ProductVersion", exe_path.replace('\'', "''"));
  let output = Command::new("powershell")
    .args(["-NoProfile", "-Command", &script])
    .output()
    .ok()?;
  if !output.status.success() {
    return None;
  }
  let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
  if raw.is_empty() {
    return None;
  }
  Some(normalize_version_output(&raw))
}

#[cfg(target_os = "windows")]
fn detect_windows_file_file_version(exe_path: &str) -> Option<String> {
  let script = format!("(Get-Item -LiteralPath '{}').VersionInfo.FileVersion", exe_path.replace('\'', "''"));
  let output = Command::new("powershell")
    .args(["-NoProfile", "-Command", &script])
    .output()
    .ok()?;
  if !output.status.success() {
    return None;
  }
  let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
  if raw.is_empty() {
    return None;
  }
  Some(normalize_version_output(&raw))
}

fn detect_tool_version(tool: ToolType) -> Option<String> {
  if matches!(tool, ToolType::Codex) {
    if let Some(version) = detect_npm_global_pkg_version("@openai/codex") {
      return Some(version);
    }
  }

  if matches!(tool, ToolType::ClaudeCode) {
    if let Some(version) = detect_npm_global_pkg_version("@anthropic-ai/claude-code") {
      return Some(version);
    }
  }

  if matches!(tool, ToolType::GeminiCli) {
    if let Some(version) = detect_npm_global_pkg_version("@google/gemini-cli") {
      return Some(version);
    }
  }

  let bin = tool_bin(tool);
  let args_candidates: &[&[&str]] = match tool {
    ToolType::ClaudeCode => &[&["--version"], &["-v"], &["version"]],
    ToolType::Codex | ToolType::CodexApp => &[&["--version"], &["-v"], &["version"]],
    ToolType::GeminiCli => &[&["--version"], &["-v"], &["version"]],
  };

  for args in args_candidates {
    if let Ok(output) = run_tool_command(bin, args) {
      if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !stdout.is_empty() {
          return Some(normalize_version_output(&stdout));
        }
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if !stderr.is_empty() {
          return Some(normalize_version_output(&stderr));
        }
      }
    }
  }

  None
}

fn run_tool_command(bin: &str, args: &[&str]) -> std::io::Result<std::process::Output> {
  #[cfg(target_os = "windows")]
  {
    let mut cmdline = String::from(bin);
    for arg in args {
      cmdline.push(' ');
      cmdline.push_str(arg);
    }
    Command::new("cmd").args(["/C", &cmdline]).output()
  }
  #[cfg(not(target_os = "windows"))]
  {
    Command::new(bin).args(args).output()
  }
}

fn detect_npm_global_pkg_version(pkg: &str) -> Option<String> {
  let output = Command::new("npm")
    .args(["list", "-g", pkg, "--depth=0", "--json"])
    .output()
    .ok()?;

  if !output.status.success() {
    return None;
  }

  let stdout = String::from_utf8_lossy(&output.stdout);
  let pattern = format!("\"{pkg}\"");
  let pkg_pos = stdout.find(&pattern)?;
  let version_key_pos = stdout[pkg_pos..].find("\"version\"")? + pkg_pos;
  let after_key = &stdout[version_key_pos..];
  let colon_pos = after_key.find(':')?;
  let after_colon = after_key[(colon_pos + 1)..].trim_start();
  let first_quote = after_colon.find('"')?;
  let after_first_quote = &after_colon[(first_quote + 1)..];
  let second_quote = after_first_quote.find('"')?;
  let version = &after_first_quote[..second_quote];
  if version.is_empty() {
    return None;
  }
  Some(version.to_string())
}

fn normalize_version_output(raw: &str) -> String {
  let cleaned = raw.replace(['\r', '\n'], " ");
  for token in cleaned.split_whitespace() {
    let v = token.trim_matches(|c: char| c == '(' || c == ')' || c == ',' || c == ';');
    if looks_like_version(v) {
      return v.to_string();
    }
  }
  cleaned.trim().to_string()
}

fn looks_like_version(token: &str) -> bool {
  let trimmed = token.strip_prefix('v').unwrap_or(token);
  let mut dot_count = 0usize;
  let mut digit_count = 0usize;
  for ch in trimmed.chars() {
    if ch.is_ascii_digit() {
      digit_count += 1;
      continue;
    }
    if ch == '.' {
      dot_count += 1;
      continue;
    }
    return false;
  }
  digit_count > 0 && dot_count >= 1
}
