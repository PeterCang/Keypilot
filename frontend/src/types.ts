export type ToolType = "claude-code" | "codex" | "gemini-cli" | "codex-app";

export interface KeyRecord {
  id: string;
  name: string;
  tool: ToolType;
  apiKey: string;
  baseUrl?: string;
  model?: string;
  isActive: boolean;
  createdAt: string;
  updatedAt?: string;
  note?: string;
}

export interface SwitchResult {
  success: boolean;
  warning?: string;
  requiresRestart: boolean;
  message: string;
}

export interface ToolStatus {
  tool: ToolType;
  installed: boolean;
  version?: string;
  location?: string;
}

export interface BackupResult {
  tool: ToolType;
  backupPath?: string;
  success: boolean;
  message: string;
}

export interface ToolCurrentConfig {
  tool: ToolType;
  apiKey?: string;
  baseUrl?: string;
  model?: string;
  source: string;
}

export type AuthMethodType =
  | "settings_json"
  | "env_process"
  | "env_user"
  | "env_machine"
  | "config_toml"
  | "auth_json"
  | "custom";

export interface ToolAuthSnapshot {
  tool: ToolType;
  method: AuthMethodType;
  source: string;
  apiKey?: string;
  baseUrl?: string;
  model?: string;
  writable: boolean;
  isEffective: boolean;
  priority: number;
}
