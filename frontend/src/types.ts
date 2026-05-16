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
