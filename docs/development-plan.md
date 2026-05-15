# Keypilot 开发计划与实施进度

## 文档说明
本文件将 `agents.md` 中的任务级 WBS 计划映射为当前仓库实施状态，便于按里程碑推进与派工。

## WBS 进度总览（截至 2026-05-16）
- WBS-1 项目基线与脚手架：进行中（可编译可运行，待基线文件与忽略规则收敛）
- WBS-2 本地数据层：进行中（核心读写已完成，已补关键单测）
- WBS-3 适配器层：进行中（切换链路可用，持久化与回滚增强待补）
- WBS-4 进程检测与重启引导：未开始（仅检测占位）
- WBS-5 前端主界面：进行中（已满足中英文国际化硬约束）
- WBS-6 系统托盘：未开始
- WBS-7 安装器能力：进行中（已从占位升级为可执行安装）
- WBS-8 测试与发布：进行中（已补最小单测集，发布流程未完成）

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
  - `install_tool`（已实现执行链路，Gemini 仍为占位提示）
- 存储能力：`data.json` 初始化、读写、临时文件写入、备份文件生成
- 适配器能力：
  - Codex: `~/.codex/auth.json` 写入与备份
  - Claude/Gemini: 环境变量切换（当前进程级，系统持久化待实现）
- CI：GitHub Actions 基础流水线（前端 typecheck/build + Rust check）
- 国际化：前端 UI 支持 `zh-CN` 与 `en-US` 两种语言切换
- 单元测试：
  - `storage` 读写回环测试
  - `adapters` 的 Codex 切换写入测试
  - `installer` 安装计划测试

## 本会话完成记录（用于新会话续接）
- `ddaa23e`：项目规则更新，新增“任务完成即提交”与“UI 中英文强制支持”。
- `22ef4fa`：前端国际化落地，新增语言切换与双语文案资源。
- `a3f94fa`：新增后端关键单测（storage/adapters）。
- `e223918`：`install_tool` 从占位升级为可执行安装链路。

## 下一步实施顺序（新会话起点）
1. WBS-3：完成 Claude/Gemini 环境变量系统级持久化写入（Windows/macOS）与读取校验。
2. WBS-3：补 Codex `config.toml` 与 `auth.json` 一致性写入、失败回滚与错误分层。
3. WBS-4：实现“切换后提示重启 / 切换并重启”流程与运行态分支。
4. WBS-6：实现系统托盘菜单与主窗口状态双向同步。
5. WBS-7：补安装日志流式事件（当前为一次性返回），完善 Gemini 安装链路。
6. WBS-8：扩充回归矩阵（失败流/回滚流/跨平台流）并补发布工件流程。
