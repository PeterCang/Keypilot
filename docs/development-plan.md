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
  - Codex CLI/App: 共享同一套 `~/.codex/` 配置与同一 Key List，选中 `codex` 或 `codex-app` 时列表和激活态保持一致
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
- `9fe8483`：Key List 在工具切换时按当前工具重载。
- `5639429`：Key List 增加“当前工具配置”展示（来源 + 配置值）。
- `dcf021b`：修复环境变量读取范围（进程 + HKCU + HKLM）并兼容 Claude 变量名。
- `2524768`：调整 `Current Tool Config` 展示顺序并显示完整 API Key。
- `f40f0af`：引入多认证方式模型与 `detect_tool_auth` 探测接口（架构基线）。

## 认证 Provider 架构（新增，2026-05-17）
### 目标
- 支持“一个工具多种认证方式”，避免在 `switch_key` / `get_tool_current_config` 中写死路径与变量名。
- 新增工具或新增认证方式时，仅新增适配实现，不改核心编排流程。
- 为 UI 提供“多来源可观测性”：不仅给出当前生效配置，还给出各来源快照与优先级。

### 分层设计
- `ToolAdapter`（工具级）：定义工具支持的认证方式、写入默认策略、优先级顺序。
- `AuthProvider`（认证方式级）：每种方式独立实现 `detect / apply / verify / backup / restore`。
- `SwitchOrchestrator`（编排级）：统一执行“备份 -> 写入 -> 校验 -> 失败回滚 -> 日志”。

### 已落地的数据模型
- Rust: `src-tauri/src/models.rs`
  - `AuthMethodType`
  - `ToolAuthSnapshot`
- 前端: `frontend/src/types.ts`
  - `AuthMethodType`
  - `ToolAuthSnapshot`

### 已落地的后端接口
- `get_tool_current_config(tool) -> ToolCurrentConfig`
  - 通过快照解析有效配置返回（兼容现有 UI）。
- `detect_tool_auth(tool) -> ToolAuthSnapshot[]`
  - 返回该工具所有支持来源的探测结果、优先级、可写性、是否生效。
- 命令注册位置：`src-tauri/src/lib.rs`
- 核心实现位置：`src-tauri/src/adapters.rs`

### Claude Code 样板（第一版）
- 支持探测的认证方式：
  - `env_process`：当前进程环境变量（只读）。
  - `settings_json`：`~/.claude/settings.json` 的 `env.ANTHROPIC_AUTH_TOKEN / ANTHROPIC_BASE_URL`。
  - `env_user`：Windows `HKCU\\Environment`（可写）。
  - `env_machine`：Windows `HKLM\\SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment`（只读）。
- 当前优先级（小数字优先）：
  1. `env_process`
  2. `settings_json`
  3. `env_user`
  4. `env_machine`
- 有效配置解析规则：
  - 按优先级排序后，首个包含 `apiKey/baseUrl/model` 任一值的快照记为 `isEffective=true`。

### 扩展约定（后续新增工具/方式）
- 新增工具：
  - 在 `ToolType` 增加枚举；
  - 在 `detect_tool_auth_methods(tool)` 中补该工具的 Provider 组合；
  - 复用统一快照与解析逻辑。
- 新增认证方式：
  - 在 `AuthMethodType` 增加类型；
  - 在适配器中补对应 Provider 的 `detect/apply/verify`；
  - 将其插入目标工具的优先级列表。
- 兼容要求：
  - 不得破坏现有 `switch_key`、`get_tool_current_config` API 契约；
  - 高风险写入必须保留备份与回滚能力。

## 下一步实施顺序（新会话起点）
1. WBS-8：继续扩充失败流/跨平台流集成测试脚手架。
2. WBS-8：为 `windows-package` 补工件上传与版本标签发布流程。
