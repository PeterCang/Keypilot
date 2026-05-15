# Keypilot

Keypilot is a cross-platform desktop tool for managing and switching API keys used by AI Coding Agents.

## Current Workspace Layout
- `agents.md`: 开发协作 Agent 规范 + 任务级 WBS。
- `docs/development-plan.md`: WBS 实施进度与下一步计划。
- `frontend/`: React + TypeScript + Vite 前端。
- `src-tauri/`: Tauri + Rust 后端。

## Quick Start (Scaffold Stage)
1. Frontend
   - `npm install --prefix frontend`
   - `npm run dev --prefix frontend`
2. Tauri
   - `cargo check --manifest-path src-tauri/Cargo.toml`
3. Windows 打包（MVP）
   - `npm run build --prefix frontend`
   - `cargo tauri build --manifest-path src-tauri/Cargo.toml`

## Documentation
See [调研报告.md](./调研报告.md) for research context and [agents.md](./agents.md) for collaboration and WBS.
Release notes template: [docs/release-notes-template.md](./docs/release-notes-template.md).
