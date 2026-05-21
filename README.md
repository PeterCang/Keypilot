# Keypilot

A cross-platform desktop app for managing and switching API keys used by AI coding agents — Claude Code, Codex CLI, Codex App, and Gemini CLI.

---

## What it does

AI coding agents read their API keys from environment variables or config files. Switching between providers, accounts, or custom endpoints means manually editing those files every time. Keypilot centralizes that into a single UI: store multiple keys per tool, switch the active one with one click, and launch the tool directly from the app.

Key capabilities:

- **Multi-key management** — store any number of keys per tool, each with a provider name, API key, base URL, and model
- **One-click switching** — writes the selected key to the correct location for each tool (env var, `settings.json`, or config files), with backup and rollback on failure
- **Auth source detection** — reads where the current key actually comes from (process env, `settings.json`, user registry, machine registry) and writes back to the same source
- **Tool lifecycle** — install, uninstall, start, and restart tools from within the app
- **System tray** — switch keys without opening the main window
- **Auto-import** — detects the key already configured in a tool on first launch and imports it automatically
- **Config backups** — keeps up to 5 rotating backups of every config file it touches
- **i18n** — Chinese (zh-CN) and English (en-US) UI

---

## Supported tools

| Tool | Config location |
|---|---|
| Claude Code | `~/.claude/settings.json` → `ANTHROPIC_API_KEY` / `ANTHROPIC_BASE_URL` env var |
| Codex CLI | `~/.codex/auth.json` + `~/.codex/config.toml` |
| Codex App | Same as Codex CLI (shared config group) |
| Gemini CLI | `GEMINI_API_KEY` / `GEMINI_BASE_URL` / `GEMINI_MODEL` env var |

---

## Tech stack

| Layer | Technology |
|---|---|
| Desktop shell | [Tauri 2](https://tauri.app/) |
| Backend | Rust (2021 edition) |
| Frontend | React 18 + TypeScript 5 + Vite 5 |
| Serialization | serde / serde_json |
| IPC | Tauri `invoke()` commands + Tauri events |
| Storage | `data.json` — atomic write (tmp → rename) with `.bak` copy |
| Tray | Tauri `tray-icon` feature |

---

## Architecture

```
frontend/src/
  api.ts          thin invoke() wrappers — no business logic
  App.tsx         single-page UI
  types.ts        shared TypeScript types
  i18n.ts         zh-CN / en-US dictionaries

src-tauri/src/
  models.rs       KeyRecord, ToolType, SwitchResult, ToolAuthSnapshot, …
  storage.rs      load/save data.json (atomic write + .bak)
  adapters.rs     switching logic, auth detection, backup rotation
  installer.rs    npm install/uninstall, tool start, streaming install-log events
  process.rs      detect installed tools, is_tool_running, restart_tool
  lib.rs          Tauri command registration, tray menu, tray event handler
```

All business logic lives in Rust. The frontend never writes config files directly.

### Auth source priority (Claude Code)

`detect_tool_auth_methods` returns snapshots sorted by priority. The first one with a value is "effective":

1. `env_process` — current process env (read-only)
2. `settings_json` — `~/.claude/settings.json` (writable)
3. `env_user` — HKCU registry on Windows / `launchctl` on macOS (writable)
4. `env_machine` — HKLM registry on Windows (read-only)

`switch_key` routes the write to whichever source is currently effective.

### Codex / Codex App shared config

`ToolType::Codex` and `ToolType::CodexApp` both read and write the same `~/.codex/auth.json` and `~/.codex/config.toml`. The frontend mirrors this with `toolConfigGroup()` (maps `codex-app` → `codex`).

### Backup rotation

Config files get up to 5 rotating backups: `.bak1` (newest) through `.bak5`. The 6th rotation drops `.bak5`.

---

## Getting started

### Prerequisites

- [Node.js](https://nodejs.org/) 18+
- [Rust](https://rustup.rs/) (stable)
- [Tauri CLI](https://tauri.app/start/prerequisites/)

### Development

```bash
# Install frontend dependencies
npm install --prefix frontend

# Run in dev mode (hot-reload frontend + Rust backend)
cargo tauri dev
```

### Type checking

```bash
npm run typecheck --prefix frontend
cargo check --manifest-path src-tauri/Cargo.toml
```

### Tests

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

### Build (Windows MSI)

```bash
npm run build --prefix frontend
cargo tauri build --manifest-path src-tauri/Cargo.toml --bundles msi
```

---

## Data storage

| Platform | Path |
|---|---|
| Windows | `%APPDATA%\KeyPilot\data.json` |
| macOS | `~/Library/Application Support/KeyPilot/data.json` |

The file is written atomically (write to `.tmp`, then rename). A `.bak` copy is kept alongside it.

---

## Contributing

- All user-visible strings must have both `zh-CN` and `en-US` entries in `frontend/src/i18n.ts`
- High-risk writes (config file changes) must use backup + verify + rollback
- TypeScript typecheck and `cargo check` must pass before merging
- Core unit tests must pass (`cargo test`)
- Commit after every logical change — do not batch unrelated changes
