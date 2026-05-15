import { invoke } from "@tauri-apps/api/core";
import type { BackupResult, KeyRecord, SwitchResult, ToolStatus } from "./types";

export const listKeys = () => invoke<KeyRecord[]>("list_keys");
export const saveKey = (payload: KeyRecord) => invoke<KeyRecord>("save_key", { payload });
export const deleteKey = (id: string) => invoke<boolean>("delete_key", { id });
export const switchKey = (id: string) => invoke<SwitchResult>("switch_key", { id });
export const detectTools = () => invoke<ToolStatus[]>("detect_tools");
export const backupConfig = (tool: KeyRecord["tool"]) => invoke<BackupResult>("backup_config", { tool });
export const restartTool = (tool: KeyRecord["tool"]) => invoke<string>("restart_tool", { tool });
export const installTool = (tool: KeyRecord["tool"]) => invoke<string>("install_tool", { tool });
