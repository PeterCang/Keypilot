# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this project is

Keypilot is a cross-platform desktop app (Tauri + React + TypeScript + Rust) for managing and switching API keys used by AI coding agents: Claude Code, Codex CLI, Codex App, and Gemini CLI.

## Commands

### Frontend (React + TypeScript + Vite)
```
npm install --prefix frontend
npm run dev --prefix frontend
npm run typecheck --prefix frontend
npm run build --prefix frontend
```

### Backend (Tauri + Rust)
```
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml -- <test_name>
```

### Full app (dev mode)
```
cargo tauri dev
```

### Windows package
```
npm run build --prefix frontend
cargo tauri build --manifest-path src-tauri/Cargo.toml --bundles msi
```

## Architecture

### Data flow
The frontend calls Tauri commands via `frontend/src/api.ts` (thin wrappers around `invoke()`). All business logic lives in Rust. The frontend never writes config files directly.

### Rust modules (`src-tauri/src/`)
- `models.rs` — shared types: `KeyRecord`, `ToolType`, `SwitchResult`, `ToolStatus`, `ToolCurrentConfig`, `ToolAuthSnapshot`, `AuthMethodType`. All types use `camelCase` serde for JSON interop with the frontend.
- `storage.rs` — reads/writes `data.json` (Windows: `%APPDATA%\KeyPilot\data.json`, macOS: `~/Library/Application Support/KeyPilot/data.json`). Uses atomic write (tmp → rename) with a `.bak` copy.
- `adapters.rs` — the core switching logic. Implements `switch_key_for_record_with_source`, `read_current_tool_config`, `detect_tool_auth_methods`, and `backup_config_for_tool`. All high-risk writes use backup + verify + rollback.
- `installer.rs` — runs `npm install -g <package>` for each tool, streams output as `install-log` Tauri events.
- `process.rs` — detects whether a tool is installed/running, implements `restart_tool`.
- `lib.rs` — registers all Tauri commands, builds the system tray menu, handles tray events.

### Key invariant: codex / codex-app share config
`ToolType::Codex` and `ToolType::CodexApp` both read/write the same `~/.codex/auth.json` and `~/.codex/config.toml`. The frontend mirrors this with `toolConfigGroup()` (maps `codex-app` → `codex`). When filtering the key list, always use `toolsShareConfig()` / `tools_share_config()`, not direct equality.

### Auth source priority (Claude Code)
`detect_tool_auth_methods` returns snapshots sorted by priority:
1. `env_process` (current process env, read-only)
2. `settings_json` (`~/.claude/settings.json`, writable)
3. `env_user` (HKCU registry on Windows / launchctl on macOS, writable)
4. `env_machine` (HKLM registry on Windows, read-only)

`switch_key_for_record_with_source` routes to `settings.json` write vs. env-var write based on where the current active config was read from.

### Backup rotation
Config files get up to 5 rotating backups: `.bak1` (newest) through `.bak5`. The 6th rotation drops `.bak5`. Implemented in `rotate_backups_and_copy_current` in `adapters.rs`.

### i18n constraint
All user-visible strings must have both `zh-CN` and `en-US` entries in `frontend/src/i18n.ts`. No hardcoded UI text.

### Tray ↔ UI sync
After any tray-triggered key switch, the backend emits a `key-switched` Tauri event. The frontend listens in `App.tsx` and calls `reloadAll()` to stay in sync.

## Quality gates (must pass before merging)
- TypeScript typecheck passes (`npm run typecheck --prefix frontend`)
- Rust check passes (`cargo check`)
- Core unit tests pass (`cargo test`)
- New UI text has both `zh-CN` and `en-US` translations
- High-risk writes (config file changes) have backup + verify + rollback

## Workflow rules
- **Commit after every change**: after any code modification, immediately stage and commit. Do not batch unrelated changes into one commit.
