# M5-C — ChatGPT Product Integration Gate

**候选实现；未获准合并或发布。** 工作从 M5-B1 RC `e0c4881` 分叉（原 Draft PR #2 保持不变）。本报告区分自动化、Windows 原生和真实 ChatGPT Work 验收。

| 检查 | 证据与状态 |
| --- | --- |
| 应用实现 | ChatGPT 独立身份、双权限默认关闭、轮换/撤销、有效调用状态、设置向导、自动 ZIP。原功能使用原凭据和路由。 |
| SQLite | 新建和 0010/0011 升级自动化通过。0011 夹具逐字段匹配 Codex/DuMate 三种批次状态、收据和来源；`PRAGMA foreign_key_check` 为空。0001–0011 未改。 |
| Windows 自动化 | 已有一次 69 项 Rust 测试通过（另有系统凭据用例按需运行），105 项前端测试通过；严格 Clippy、TypeScript、ESLint 已通过实现层基线。发布构建与最终复验另记。 |
| 独立客户端 | 通过 HTTP 集成用例核对本机专用路由、权限、请求签名、过时证明、来源标签、幂等、仅 pending、重启和 OpenClaw 隔离。此类证据不等于真实 Work 调用。 |
| Windows Truth Build / 原生窗口 | 待记录最终 SHA、独立 OS 凭据 smoke 和导出结果。 |
| GitHub Actions | 等待独立 Draft PR 的 Windows 和 macOS 实际结果。M5-B1 的 CI 不用于 M5-C。 |
| ChatGPT Work → Local | 待用新产品包实调两个工具，查 Inbox/Reader、待确认区、拒绝和确认以及权限操作。在此之前不宣布可交付。GPT-C0 是独立探针，通过记录不能替代。 |
| macOS 真实客户端 | 尚未运行。Hosted CI 不覆盖插件实际导入、执行权限或用户 Keychain 提示。 |
| 首次连接 | 设计为选择并确认权限、保存/安装 ZIP、首次调用三段。所需操作数和用时需由真实使用者测量；未声称一分钟。 |
| 正式环境 | 未启动或覆盖正式程序，未更改正式数据和授权。正式 exe SHA-256：`0C9E13094F57C54445FCC7D43D79A764D6A99BC325F577C10AA21DA18B946792`。正式 AppData 前后哈希未采集：不读取真实用户内容参与隔离验收。 |

## 文件范围

- Rust 和 SQLite：`src-tauri/src/chatgpt.rs`、`src-tauri/src/chatgpt/package.rs`、Bridge、STDIO、命令注册、来源枚举和测试，以及新增 `src-tauri/migrations/0012_chatgpt_desktop_connection.sql`。
- 前端：`src/features/agentBridge/` 的独立 ChatGPT 卡、回归测试、统一连接入口，以及 Todo 建议来源映射。
- 发布和说明：`scripts/tauri.mjs` 清理构建路径、`package.json` 使用该构建入口，以及本报告、[技术说明](M5_C_CHATGPT_ARCHITECTURE.md)和[用户指南](M5_C_CHATGPT_INSTALL_GUIDE.md)。

真实验收必须使用合成内容、隔离 AppData/WebView2、隔离凭据和独立端口。记录实际回执、持久行和 UI；不保存 Token、用户绝对路径或原始私人日志。断开后的历史数据不会删除。升级前应停机备份；回退旧程序用迁移前对应备份，不能用旧程序继续打开已迁移的库。

```text
M5_C_CHATGPT_INTEGRATION_STATUS: IMPLEMENTED_PENDING_NATIVE_GATE
CHATGPT_INDEPENDENT_IDENTITY: IMPLEMENTED
CHATGPT_ONBOARDING: IMPLEMENTED_PENDING_REAL_WORK_ACCEPTANCE
MAIN_MERGE_STATUS: NOT_AUTHORIZED
CANONICAL_REPLACEMENT_STATUS: NOT_AUTHORIZED
```
