# Keypilot 发布说明模板（MVP）

## 版本信息
- 版本号：`vX.Y.Z`
- 发布日期：`YYYY-MM-DD`
- 发布负责人：`Release Agent`

## 新增功能
- 

## 修复问题
- 

## 已知限制
- 

## 升级与回滚
- 升级：下载安装包后覆盖安装。
- 回滚：卸载当前版本并安装上一稳定版本；必要时恢复配置备份：
  - `~/.codex/auth.json.bak`
  - `~/.codex/config.toml.bak`
  - `AppData/Roaming/KeyPilot/data.json.bak`（Windows）
  - `~/Library/Application Support/KeyPilot/data.json.bak`（macOS）

## 验证清单
- [ ] 前端构建通过（`npm run build --prefix frontend`）
- [ ] Rust 测试通过（`cargo test --manifest-path src-tauri/Cargo.toml`）
- [ ] 核心切换链路手测通过（UI 与托盘）
- [ ] 中英文文案校验通过

