# Keypilot 开发协作 Agents 规范与任务级 WBS 计划

## 1. 文档目标与适用范围
本文件用于规范 Keypilot 项目中的开发协作智能体（Agent）分工、交接、质量门禁与迭代节奏，确保任务可直接拆分并派工执行。

本文件定义的是“项目研发协作角色 Agent”，不涉及产品内 AI Agent 运行时协议。

## 2. 固定角色定义（7 个）

### 2.1 Product Agent
- 职责
  - 需求澄清与范围冻结。
  - 维护验收标准与优先级。
  - 管控 MVP 范围，避免需求蔓延。
- 不负责
  - 具体技术实现方案。
  - 代码级缺陷修复。
- 输入
  - 用户目标、调研结论、里程碑约束。
  - 来自 Architecture/QA 的风险反馈。
- 输出
  - 冻结后的需求说明（含非目标）。
  - 任务验收标准与优先级列表。
  - 版本范围变更记录。
- 完成定义（DoD）
  - 每个里程碑均有可执行验收条目。
  - MVP 范围与延期项边界清晰。
- 关键协作对象
  - Architecture Agent、QA Agent、Release Agent。

### 2.2 Architecture Agent
- 职责
  - 设计模块边界与接口约束。
  - 把关非功能需求（稳定性、可回滚、可观测）。
  - 裁决技术冲突并锁定实现约束。
- 不负责
  - 业务优先级排序。
  - 发布说明与安装包交付。
- 输入
  - 产品冻结需求。
  - 现有技术栈约束（Tauri + React + TypeScript + Rust + 系统托盘）。
- 输出
  - 模块划分与接口契约文档。
  - 关键类型与错误模型约束。
  - 技术冲突裁决记录。
- 完成定义（DoD）
  - Public API 与关键数据结构已冻结。
  - 高风险链路（配置写入/安装器）有回滚策略。
- 关键协作对象
  - Frontend Agent、Backend Agent、Integration Agent。

### 2.3 Frontend Agent
- 职责
  - 实现 React + TypeScript 主界面。
  - 管理前端状态流与后端命令联动。
  - 实现托盘触发后的 UI 状态同步。
- 不负责
  - 系统级配置写入。
  - 安装器命令执行与系统权限处理。
- 输入
  - 冻结的前后端接口签名。
  - 交互流程与验收标准。
- 输出
  - UI 页面、组件与状态管理实现。
  - 前端错误展示与操作日志视图。
- 完成定义（DoD）
  - 关键用户流在 3 步内完成切换。
  - 切换结果、错误、工具状态可视化完整。
- 关键协作对象
  - Backend Agent、Integration Agent、QA Agent。

### 2.4 Backend Agent
- 职责
  - 实现 Rust/Tauri commands。
  - 实现数据存储、配置读写、备份回滚。
  - 实现进程检测、安装器执行与事件日志。
- 不负责
  - 前端视觉与交互布局。
  - 需求优先级决策。
- 输入
  - 冻结接口与类型约束。
  - 工具适配器规则（Claude/Codex/Gemini）。
- 输出
  - 可调用命令实现与统一错误模型。
  - 存储/适配器/安装器核心逻辑。
- 完成定义（DoD）
  - 核心命令具备可测性与失败回滚。
  - 高风险操作有备份与恢复证据。
- 关键协作对象
  - Architecture Agent、Integration Agent、QA Agent。

### 2.5 Integration Agent
- 职责
  - 负责 Claude/Codex/Gemini 适配器集成。
  - 打通前后端与托盘端到端链路。
  - 统一 `switch_key` 结果模型。
- 不负责
  - 产品范围取舍。
  - 发布版本策略。
- 输入
  - Backend 命令与 Frontend 状态接口。
  - 三类工具配置读写规则。
- 输出
  - 适配器集成代码与验证清单。
  - 端到端联调报告。
- 完成定义（DoD）
  - 三类目标均完成“写入 + 验证 + 失败回滚”。
  - 切换主链路在 UI 与托盘入口行为一致。
- 关键协作对象
  - Frontend Agent、Backend Agent、QA Agent。

### 2.6 QA Agent
- 职责
  - 设计单元/集成/回归测试。
  - 维护回归矩阵与缺陷分级标准。
  - 验证高风险改动的回滚有效性。
- 不负责
  - 生产代码主实现。
  - 需求范围变更审批。
- 输入
  - 验收标准、接口约束、实现产物。
- 输出
  - 测试用例与执行报告。
  - 缺陷清单（含严重度与复现步骤）。
  - 回归结论与发布质量建议。
- 完成定义（DoD）
  - MVP 必测场景执行完毕并有证据。
  - 阻断级缺陷清零或经 Product 明确延期。
- 关键协作对象
  - Frontend Agent、Backend Agent、Release Agent。

### 2.7 Release Agent
- 职责
  - 负责打包、版本号、发布说明与交付清单。
  - 校验发布前门禁与工件完整性。
- 不负责
  - 跨模块技术方案设计。
  - 测试用例编写主责。
- 输入
  - QA 放行结论。
  - 版本范围与变更记录。
- 输出
  - 可安装包、版本标签、发布说明模板实例。
  - 最终交付清单与回滚指引。
- 完成定义（DoD）
  - Windows 包可生成并可安装验证。
  - 发布说明覆盖新增、修复、限制与回滚。
- 关键协作对象
  - Product Agent、QA Agent、Architecture Agent。

## 3. 协作协议
- 单一任务单一 Owner：每个任务只能有一个主责任 Agent。
- 跨角色交接只通过“已定义产物”完成（接口文档、代码 PR、测试报告、发布清单）。
- 任务提交纪律：每次完成一个任务后，必须立即提交该任务产生的代码变更（`git commit`），禁止累计多个已完成任务后再一次性提交。
- 强制 PR Checklist（缺一不可）
  - 接口变更说明。
  - 回滚路径。
  - 测试证据（日志、截图、报告或 CI 记录）。
- 决策优先级
  - 已确认文档 > 已冻结接口 > 临时讨论。
- 冲突处理
  - 技术冲突由 Architecture Agent 仲裁。
  - 范围冲突由 Product Agent 仲裁。

## 4. 质量门禁
合并前必须满足以下全部条件：
- TypeScript 类型检查通过。
- 核心单元测试通过。
- 关键链路手动验证通过。
- 文档同步更新。
- UI 国际化门禁通过：用户界面必须同时支持中文与英文，新增或变更文案默认提供 `zh-CN` 与 `en-US` 两套翻译，禁止仅单语言上线。

高风险改动（配置写入、安装器执行）额外要求：
- 提供失败回滚验证记录。
- 提供异常路径日志或复现证据。

## 5. 迭代节奏与流程
- 日节奏
  - 每日站会同步：昨日完成、今日计划、阻塞项。
  - 阻塞超过 24 小时必须升级到 Product/Architecture 协同处理。
- 周节奏
  - 每周里程碑验收。
  - 每周风险复盘（范围、质量、进度、回滚有效性）。
- Issue/PR 命名建议
  - Issue：`[WBS-x] 模块-任务名`
  - PR：`[WBS-x][Agent] 变更摘要`
- 状态流转
  - `todo -> in-progress -> review -> done`

## 6. Public APIs / Interfaces（冻结契约）
Tauri commands（统一签名与错误模型）：
- `list_keys() -> KeyRecord[]`
- `save_key(payload: KeyRecord) -> KeyRecord`
- `delete_key(id: string) -> boolean`
- `switch_key(id: string) -> SwitchResult`
- `detect_tools() -> ToolStatus[]`
- `install_tool(tool: ToolType) -> Stream<EventLog>`
- `backup_config(tool: ToolType) -> BackupResult`

关键类型约束：
- `ToolType = "claude-code" | "codex" | "codex-app" | "gemini-cli"`
- `codex` 与 `codex-app` 必须共享同一配置组：两者读写同一套 `~/.codex/config.toml` 与 `~/.codex/auth.json`，在 UI 中选中任一入口时 Key List 必须一致，切换激活态也必须在两者之间同步。
- `KeyRecord`、`SwitchResult`、`ToolStatus`、`BackupResult` 必须前后端共享定义，或通过生成/镜像机制保持一致。

## 7. 任务级 WBS 开发计划（MVP）

### WBS-1 项目基线与脚手架
- 建立 Tauri + React + TypeScript + Rust 工程骨架。
- 建立目录约定：`frontend/`、`src-tauri/`、`docs/`（若采用 Tauri 默认结构，需建立映射说明）。
- 建立基础 CI：类型检查、前端构建、Rust 构建。
- 验收标准
  - 本地可启动桌面壳。
  - 空白页面可加载。
  - CI 首次通过。

### WBS-2 本地数据层（Storage）
- 定义 `KeyRecord`、`ToolType`、`AppState`。
- 实现 `data.json` 初始化、读写、原子写入、备份。
- 实现命令：`list_keys` / `save_key` / `delete_key`。
- 验收标准
  - CRUD 正常。
  - 异常中断不损坏主文件。
  - 备份可恢复。

### WBS-3 适配器层（Claude/Codex/Gemini）
- Claude：用户级环境变量写入与读取校验。
- Codex：`config.toml` / `auth.json` 更新策略与备份回滚。
- Gemini：环境变量写入流程与校验。
- 统一 `switch_key` 返回模型：成功、警告、需重启、失败原因。
- 验收标准
  - 三类目标均完成写入、验证、失败回滚。

### WBS-4 进程检测与重启引导
- 实现目标工具运行态检测（Windows/macOS 分支）。
- 实现“切换后提示重启”与“切换并重启”流程框架。
- 验收标准
  - 运行中与未运行两条路径行为一致且可预期。

### WBS-5 前端主界面
- Key 列表、编辑弹窗、激活态标识、切换按钮。
- 工具状态面板（安装状态、版本、最近切换结果）。
- 错误与日志展示面板（后端事件回传）。
- UI 国际化：主界面文案、提示、错误信息、托盘相关可见文本必须支持中文与英文切换。
- 验收标准
  - 关键用户流可在 3 步内完成切换。
  - 中文与英文两种语言下核心流程文案完整、语义一致且无回退到硬编码文本。

### WBS-6 系统托盘
- 托盘菜单展示当前激活 Key 与快捷切换项。
- 托盘切换触发与 UI 状态同步。
- 验收标准
  - 不打开主窗口即可完成切换。
  - 状态即时同步。

### WBS-7 安装器能力
- `detect_tools`：Node/npm/工具安装状态检测。
- `install_tool`：安装命令执行、日志流式回传、错误分层提示。
- 验收标准
  - 至少 1 个工具安装链路全流程通过。
  - 其他工具完成检测 + 占位流程。

### WBS-8 测试与发布
- 单元测试：storage、adapter、`switch_key` 核心逻辑。
- 集成测试：切换主链路、回滚链路、异常链路。
- 发布：Windows 打包、版本号策略、发布说明模板。
- 验收标准
  - MVP 回归矩阵通过。
  - 可生成可安装包。

## 8. MVP 必测场景（Test Plan）
- 正常流
  - 新增 Key -> 激活切换 -> 状态刷新 -> 托盘同步。
- 失败流
  - 无权限写入。
  - 目标配置文件损坏。
  - 工具运行中冲突。
- 回滚流
  - 写入中断后自动恢复备份，数据文件不损坏。
- 跨平台流
  - Windows 与 macOS 的环境变量写入、进程检测行为一致性。
- 安装流
  - 依赖缺失、安装失败、重复安装、版本检测提示。

## 9. 假设与边界
- 文档输出语言为中文。
- 本文 Agent 指开发协作角色，不是产品功能 Agent。
- 开发计划粒度为任务级 WBS，可直接拆成 Issue 派工。
- 当前阶段以 MVP 落地优先，余额监控、自动切换等能力延后到后续迭代。
