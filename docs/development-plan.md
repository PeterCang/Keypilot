# Keypilot 开发计划与实施进度

## 文档说明
本文件将 `agents.md` 中的任务级 WBS 计划映射为当前仓库实施状态，便于按里程碑推进与派工。

## WBS 进度总览（截至 2026-05-16）
- WBS-1 项目基线与脚手架：进行中（可编译可运行，待基线文件与忽略规则收敛）
- WBS-2 本地数据层：进行中（核心读写已完成，已补关键单测）
- WBS-3 适配器层：进行中（Claude/Gemini 系统级持久化、Codex 双配置一致性与回滚已完成）
- WBS-4 进程检测与重启引导：进行中（运行态分支 + 切换并重启框架已完成）
- WBS-5 前端主界面：进行中（已满足中英文国际化硬约束）
- WBS-6 系统托盘：进行中（托盘快捷切换与主窗口状态同步已完成）
- WBS-7 安装器能力：进行中（安装日志事件流已完成，Gemini 安装链路已补）
- WBS-8 测试与发布：进行中（回归测试已扩充，发布模板与 Windows 打包 CI 已落地）

## 已落地内容
- 前端骨架：`frontend/`（React + TypeScript + Vite）
- 后端骨架：`src-tauri/`（Tauri + Rust）
- 命令接口已实现：
  - `list_keys`
  - `save_key`
  - `delete_key`
  - `switch_key`
  - `detect_tools`
  - `backup_config`
  - `install_tool`（日志事件流 + Claude/Codex/Gemini 安装计划）
  - `restart_tool`（切换后重启流程框架）
- 存储能力：`data.json` 初始化、读写、临时文件写入、备份文件生成
- 适配器能力：
  - Codex: `~/.codex/auth.json + config.toml` 一致性写入、校验、备份与失败回滚
  - Claude/Gemini: 系统级环境变量持久化写入与读取校验（含失败回滚）
- 托盘能力：托盘快捷切换、主窗口状态同步、托盘事件回传
- CI：GitHub Actions（前端 typecheck/build + Rust check + Windows MSI 打包）
- 国际化：前端 UI 支持 `zh-CN` 与 `en-US` 两种语言切换
- 单元测试：
  - `storage` 读写回环测试
  - `adapters` 的 Codex 切换写入/备份测试
  - `installer` 安装计划测试（含 Gemini）

## 本轮新增记录（自动续推）
- `0161761`：Claude/Gemini 环境变量系统级持久化写入与校验回滚。
- `3a84e84`：Codex `auth.json + config.toml` 一致性写入与失败回滚。
- `15778ad`：运行态检测、切换后重启提示与“切换并重启”流程框架。
- `09f2951`：系统托盘快捷切换与 UI 状态同步事件。
- `f636273`：安装日志事件流 + Gemini 安装链路补齐。
- `f9cb463`：补充 Codex 备份回归测试并更新进度文档。
- `2e874ed`：补发布说明模板与 Windows 打包流程文档。
- `46287d8`：新增 Windows 自动打包 CI 作业。

## 本会话完成记录（用于新会话续接）
- `ddaa23e`：项目规则更新，新增“任务完成即提交”与“UI 中英文强制支持”。
- `22ef4fa`：前端国际化落地，新增语言切换与双语文案资源。
- `a3f94fa`：新增后端关键单测（storage/adapters）。
- `e223918`：`install_tool` 从占位升级为可执行安装链路。

## 下一步实施顺序（新会话起点）
1. WBS-8：继续扩充失败流/跨平台流集成测试脚手架。
2. WBS-8：为 `windows-package` 补工件上传与版本标签发布流程。
