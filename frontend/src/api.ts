import { invoke } from "@tauri-apps/api/core";
import type { BackupResult, KeyRecord, SwitchResult, SyncKeyResult, ToolAuthSnapshot, ToolCurrentConfig, ToolStatus } from "./types";

export const listKeys = () => invoke<KeyRecord[]>("list_keys");
export const ensureInitialKeyForTool = (tool: KeyRecord["tool"]) =>
  invoke<KeyRecord[]>("ensure_initial_key_for_tool", { tool });
export const syncActiveKeyForTool = (tool: KeyRecord["tool"]) =>
  invoke<SyncKeyResult>("sync_active_key_for_tool", { tool });
export const saveKey = (payload: KeyRecord) => invoke<KeyRecord>("save_key", { payload });
export const deleteKey = (id: string) => invoke<boolean>("delete_key", { id });
export const switchKey = (id: string) => invoke<SwitchResult>("switch_key", { id });
export const detectTools = () => invoke<ToolStatus[]>("detect_tools");
export const backupConfig = (tool: KeyRecord["tool"]) => invoke<BackupResult>("backup_config", { tool });
export const getToolCurrentConfig = (tool: KeyRecord["tool"]) =>
  invoke<ToolCurrentConfig>("get_tool_current_config", { tool });
export const detectToolAuth = (tool: KeyRecord["tool"]) =>
  invoke<ToolAuthSnapshot[]>("detect_tool_auth", { tool });
export const restartTool = (tool: KeyRecord["tool"]) => invoke<string>("restart_tool", { tool });
export const installTool = (tool: KeyRecord["tool"]) => invoke<string>("install_tool", { tool });
export const uninstallTool = (tool: KeyRecord["tool"]) => invoke<string>("uninstall_tool", { tool });
export const startTool = (tool: KeyRecord["tool"], args: string, projectDir?: string) =>
  invoke<string>("start_tool", { tool, args, projectDir });
