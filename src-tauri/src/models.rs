use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ToolType {
  ClaudeCode,
  Codex,
  CodexApp,
  GeminiCli,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct KeyRecord {
  pub id: String,
  pub name: String,
  pub tool: ToolType,
  pub api_key: String,
  pub base_url: Option<String>,
  pub model: Option<String>,
  pub is_active: bool,
  pub created_at: String,
  pub updated_at: Option<String>,
  pub note: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SwitchResult {
  pub success: bool,
  pub warning: Option<String>,
  pub requires_restart: bool,
  pub message: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ToolStatus {
  pub tool: ToolType,
  pub installed: bool,
  pub version: Option<String>,
  pub location: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BackupResult {
  pub tool: ToolType,
  pub backup_path: Option<String>,
  pub success: bool,
  pub message: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ToolCurrentConfig {
  pub tool: ToolType,
  pub api_key: Option<String>,
  pub base_url: Option<String>,
  pub model: Option<String>,
  pub provider_name: Option<String>,
  pub source: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuthMethodType {
  SettingsJson,
  EnvProcess,
  EnvUser,
  EnvMachine,
  ConfigToml,
  AuthJson,
  Custom,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ToolAuthSnapshot {
  pub tool: ToolType,
  pub method: AuthMethodType,
  pub source: String,
  pub api_key: Option<String>,
  pub base_url: Option<String>,
  pub model: Option<String>,
  pub writable: bool,
  pub is_effective: bool,
  pub priority: u32,
}
